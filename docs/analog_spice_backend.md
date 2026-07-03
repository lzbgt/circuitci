# SPICE-Class Analog Backend

CircuitCI must not claim physical proof for analog circuits unless it runs a
physics solver with explicit device models, netlists, stimuli, convergence
diagnostics, and measured waveform assertions. Behavioral checks such as
`CONTROL_LINE_RELEASE_SEQUENCE` are useful for early triage, but they do not
prove circuits where transistor saturation, diode clamps, parasitic
capacitance, base-charge storage, or RC discharge paths determine the result.

This document defines the first physical analog simulation contract. It is
generic by design: the UM STM32L4 USB downloader circuit is an acceptance
driver, not a hard-coded engine path.

## Problem Statement

The UM STM32 USB downloader reset/boot circuit has an analog failure mode: Q2
and Q3 can be driven into saturation, so host-side modem-control automation
cannot reliably release reset and boot straps at the expected time. A digital
or declared-delay rule can only say what the circuit designer intended. It
cannot calculate whether Q2/Q3, D13, pull resistors, capacitances, and host
drive levels produce valid BOOT0/NRST waveforms at the MCU sampling instant.

CircuitCI therefore needs a SPICE-class transient simulation path before it can
answer "does this USB downloader circuit physically reboot into the intended
mode?"

## Design Principles

- Use Rust for orchestration, validation, parsing, report generation, and the
  analog/digital bridge.
- Use mature SPICE-class backends for nonlinear analog solving instead of
  inventing a toy solver.
- Treat backend absence, missing model cards, non-convergence, or missing
  waveform assertions as critical for physical analog acceptance scenarios.
- Keep device coverage generic: passives, independent/dependent sources,
  switches, diodes, BJTs, MOSFETs, op amp/regulator macromodels, and vendor
  subcircuits must be supported through backend model libraries.
- Reusable generic macro-models must enter through the same component-library
  and `simulation.spice` subcircuit path as vendor models. Generic op-amp,
  comparator, 3.3 V regulator, buck regulator, buck-boost regulator, boost
  regulator, and selected-source power-mux models are allowed for preliminary
  topology, waveform, and GUI workflow checks, but their low-confidence model
  metadata must keep them out of stability, noise, current-limit, thermal,
  switching-ripple, switchover, reverse-current, EMI, or final sign-off claims.
- Keep chip behavior out of the analog solver. MCU pins become electrical
  thresholds, leakage/current limits, capacitance, clamps, and stimulus/load
  models. Firmware and boot ROM behavior stays in digital/protocol validators.
- Preserve artifacts: generated/provided netlists, included model files, raw or
  CSV waveforms, solver logs, and assertion measurements must be referenced in
  reports.

## Backend Contract

An `analog_transient` scenario owns a SPICE deck and waveform assertions.
An `analog_ac` scenario owns a small-signal SPICE deck, Bode response exports,
and AC assertions for frequency-domain design observation. An `analog_dc`
scenario owns a SPICE deck, `.op` operating-point export, and DC bias
assertions. An `analog_sparameter` scenario owns a frequency sweep plus
single-port or multi-port reference-impedance definitions for Xyce
S-parameter exports. An `analog_transfer_function` scenario owns a small-signal
output/input source contract for future `.TF` gain and resistance exports.

Required fields:

- `backend`: `ngspice`, `xyce`, `embedded_ngspice`, or `auto`.
- `netlist`: path to a SPICE-compatible deck.
- `model_files`: SPICE model-card or subcircuit files used by the deck.
- `node_bindings`: mapping from SPICE nodes to Board IR nets.
- `pin_bindings`: mapping from Board IR component pins to SPICE nodes.
- `analysis`: transient settings with stop time and maximum step, AC/noise/
  S-parameter settings with start/stop frequency and points per decade,
  `type: op` for DC operating-point analysis, `s_parameter_ports` for
  S-parameter port contracts, and `type: tf` with
  `transfer_output_expression` plus `transfer_input_source` for transfer
  function contracts.
- `stimuli`: named host, power, or load events when the deck is generated from
  board IR. For hand-authored decks this can be empty.
- `probes`: named voltages/currents to export.
- `assertions`: threshold checks over transient waveform samples, AC Bode
  response values, or DC operating-point values.

The first Rust implementation may support hand-authored SPICE decks before full
netlist generation. That is acceptable only if the deck is explicitly bound
back to Board IR nets/pins and the report clearly records the deck and model
artifacts used. It is not acceptable to replace a SPICE deck with a declared
delay and call it physical simulation.

## Acceptance Semantics

For a scenario with check `SPICE_TRANSIENT_ANALYSIS`:

1. Resolve the scenario netlist path relative to the project file.
2. Select a backend:
   - `ngspice` means `ngspice` must be executable.
   - `xyce` means `Xyce` or `xyce` must be executable.
   - `embedded_ngspice` means a mature ngspice-derived solver must be
     dynamically loaded, compiled, or linked into CircuitCI behind the analog
     adapter. It must not resolve to a partial in-house SPICE subset.
   - `auto` chooses the first available backend that this CircuitCI runtime can
     actually execute and normalize for the requested analysis. In the current
     slice that means external ngspice for transient, AC, DC, and noise, plus
     embedded ngspice for transient only. Xyce is intentionally not selected by
     `auto` until coverage is complete enough to avoid surprising backend
     changes.
   - Explicit `backend: xyce` supports transient, AC, DC operating-point, and
     noise runs through dedicated Xyce wrappers that export CSV-like solver
     data, normalize it to the `transient_waveform`, `ac_bode`,
     `operating_point`, `noise_spectrum`, and `noise_total` contracts, and
     write the same `solver_manifest.json` provenance as ngspice runs.
3. If no required backend is available, emit a critical
   `ANALOG_BACKEND_UNAVAILABLE` finding.
4. If the netlist or included model files are missing, emit a critical
   `ANALOG_NETLIST_UNAVAILABLE` finding.
5. Validate that node and pin bindings map to real Board IR nets and component
   pins.
6. Expand any declared analog parameter sweeps into bounded input corners, run
   transient analysis for each corner, and export machine-readable waveform
   data.
7. If the backend exits nonzero or reports non-convergence, emit a critical
   `SPICE_TRANSIENT_ANALYSIS` finding.
8. Evaluate waveform assertions. Failed assertions emit critical
   `SPICE_TRANSIENT_ANALYSIS` findings with measured and limit data.
9. Successful solver runs write `solver_manifest.json` next to the run outputs.
   The manifest records a versioned schema id, scenario and analysis kind,
   requested and selected backend, solver command/status/stdout/stderr byte
   counts, source netlist, wrapper deck, model files, sweep overrides, solver
   log, raw outputs, and normalized outputs such as `waveform.csv`, `bode.csv`,
   `operating_point.csv`, `noise_spectrum.csv`, and `noise_total.csv`. The
   manifest is a backend-neutral contract for future Xyce and RF/HB adapters;
   report consumers should use it for provenance and output discovery instead
   of inferring run state from backend-specific filenames alone.
   Opt-in real-Xyce conformance coverage is available through
   `CIRCUITCI_RUN_REAL_XYCE=1 cargo test --test analog_spice_xyce_cli` for
   transient/AC/DC/noise and
   `CIRCUITCI_RUN_REAL_XYCE=1 cargo test --test analog_sparameter_cli` for
   S-parameter; these tests are skipped by default unless the variable is set
   and `Xyce` or `xyce` is on `PATH`.
10. For generated Board IR decks, append datasheet operating-limit probes for
   MOSFET, BJT, and diode voltage/current/power ratings. Exceeding a rating
   emits a critical `SPICE_OPERATING_LIMIT` finding with measured maximum
   transient stress and the effective datasheet limit. Scenario ambient
   temperature derates power limits only when model metadata declares the
   derating slope. Pulse current ratings are considered only when model
   metadata declares pulse width and duty cycle. Missing usable
   absolute-maximum, derating, or pulse metadata for these generated
   semiconductor models also emits `SPICE_OPERATING_LIMIT` before solver
   execution.
11. Passing physical analog acceptance requires no critical findings, no
   blocking analog limitations, and suite-required waveform/artifact evidence.

For a scenario with check `SPICE_AC_ANALYSIS`:

1. Resolve and bind the scenario netlist and model files the same way as
   transient validation.
2. Require `analysis.type: ac`, finite positive `start_frequency_hz`, finite
   `stop_frequency_hz` greater than the start frequency, and
   `points_per_decade` in `1..=1000` when provided.
3. Select external `ngspice` or explicit `backend: xyce`; embedded ngspice
   still fails closed until an equivalent AC export path is implemented.
   `backend: auto` does not select Xyce for this analysis until real-Xyce
   conformance coverage is enabled.
4. Expand bounded run-input sweeps exactly like transient validation,
   including raw `.param`, generated component-value parameters, model-library
   sections, and `.temp` corners.
5. Run `ac dec`/`.AC DEC`, export complex probe data, and convert it to
   `bode.csv` with `frequency_hz`, per-probe magnitude in dB, phase in degrees,
   and linear magnitude columns.
6. Evaluate AC/Bode assertions over `bode.csv`, including gain and phase at a
   frequency, rising or falling gain-crossing frequency checks, phase margin at
   the falling 0 dB gain crossing, and gain margin at the falling -180 degree
   phase crossing. Failed assertions emit critical `SPICE_AC_ANALYSIS` findings
   with measured and limit data.
7. Preserve `bode.csv` paths in the report `waveforms` list so the GUI Scopes
   view can load them as frequency-axis traces with magnitude/phase lanes and
   sweep-corner comparison support.

For a scenario with check `SPICE_DC_ANALYSIS`:

1. Resolve and bind the scenario netlist and model files the same way as
   transient and AC validation.
2. Require `analysis.type: op` and at least one operating-point probe.
3. Select external `ngspice` or explicit `backend: xyce`; embedded ngspice
   still fails closed until an equivalent operating-point export path is
   implemented. `backend: auto` does not select Xyce for this analysis until
   real-Xyce conformance coverage is enabled.
4. Expand bounded run-input sweeps exactly like transient and AC validation.
5. Run `.op`/`.OP`, export operating-point probe data, and normalize it to
   `operating_point.csv` with one column per declared probe.
6. Evaluate `operating_point` assertions over the normalized CSV. Failed
   assertions emit critical `SPICE_DC_ANALYSIS` findings with measured value,
   limit, relation, and artifact path.
7. Emit sweep worst-corner margin summaries for DC assertions, so bias and
   quiescent operating margins can be reviewed across tolerance, temperature,
   load, and model-section corners.

For a scenario with check `SPICE_NOISE_ANALYSIS`:

1. Resolve and bind the scenario netlist and model files the same way as
   transient, AC, and DC validation.
2. Require `analysis.type: noise`, finite positive `start_frequency_hz`, finite
   `stop_frequency_hz` greater than the start frequency, `points_per_decade`
   in `1..=1000`, `noise_output_node`, and `noise_input_source`. Optional
   `noise_reference_node` creates a differential output expression.
3. Select external `ngspice` or explicit `backend: xyce`; embedded ngspice
   still fails closed until an equivalent noise export path is implemented.
   `backend: auto` does not select Xyce for this analysis until real-Xyce
   conformance coverage is enabled.
4. Expand bounded run-input sweeps exactly like transient, AC, and DC
   validation. Temperature corners still emit `.temp`; model-section and
   generated component-value corners still produce isolated run directories.
5. Run `.noise`/`.NOISE`, export the spectral-density plot, and normalize it to
   `noise_spectrum.csv` with `frequency_hz`,
   `onoise_v_per_sqrt_hz`, and `inoise_v_per_sqrt_hz`.
6. Export the integrated-total plot to `noise_total.csv` with
   `onoise_total_v` and `inoise_total_v`.
7. Evaluate noise assertions over those artifacts. Supported checks are output
   or input-referred noise density at a frequency using
   `threshold_v_per_sqrt_hz`, and integrated output or input-referred RMS noise
   using `threshold_v`.
8. Preserve `noise_spectrum.csv` paths in the report `waveforms` list and
   preserve `noise_total.csv` as normal artifacts.

For a scenario with check `SPICE_S_PARAMETER_ANALYSIS`:

1. Resolve and bind the scenario netlist and model files the same way as other
   analog validations.
2. Require `analysis.type: sparam`, finite positive `start_frequency_hz`,
   `stop_frequency_hz` greater than the start frequency, `points_per_decade`
   in `1..=1000`, and at least one `s_parameter_ports[]` entry.
3. Each S-parameter port declares `name`, `positive_node`, `negative_node`,
   and positive finite `reference_impedance_ohm`. Both nodes must be bound to
   Board IR nets through `node_bindings`.
4. Explicit `backend: xyce` generates Xyce port devices, runs `.AC` plus
   `.LIN SPARCALC=1`, captures Touchstone RI output, and normalizes it to
   `s_parameters.csv` with magnitude, phase, and linear magnitude columns.
   The raw Touchstone file and normalized CSV are recorded in
   `solver_manifest.json`.
5. `backend: auto`, `ngspice`, and `embedded_ngspice` remain conservative for
   this check and fail closed with backend-planning evidence until those
   adapters emit the same normalized `s_parameters` contract.

For a scenario with check `SPICE_TRANSFER_FUNCTION_ANALYSIS`:

1. Resolve and bind the scenario netlist and model files the same way as other
   analog validations.
2. Require `analysis.type: tf`, non-empty `transfer_output_expression`, and
   non-empty `transfer_input_source`.
3. For generated Board IR decks, `transfer_input_source` must name a generated
   board component so `.TF` provenance is tied to an actual source instance.
4. External `backend: ngspice` writes `circuitci_ngspice_tf.cir`,
   `ngspice_tf.log`, `transfer_function_raw.txt`,
   `transfer_function_summary.csv`, and `solver_manifest.json`. The normalized
   summary contains the small-signal gain, input resistance, and output
   resistance parsed from ngspice `.TF` output. Opt-in real-ngspice
   conformance coverage is available through
   `CIRCUITCI_RUN_REAL_NGSPICE=1 cargo test --test analog_transfer_function_cli`;
   the test is skipped by default unless the variable is set and `ngspice` is
   on `PATH`.
5. Xyce and embedded ngspice remain fail-closed with backend-planning evidence
   until those adapters emit the same normalized `transfer_function_summary`
   contract.

Until the real-backend transient and AC contracts above are satisfied for the
target circuit, CircuitCI must not present the UM USB downloader physical
acceptance as passing.

## UM Downloader Physical Acceptance Target

The UM fixture must model the USB downloader analog network with common
devices:

- S8050/S8050-like NPN transistor model for Q2.
- SS8550/SS8550-like PNP transistor model for Q3.
- 1N4148/1N4148WS diode model for D13.
- Board pull resistors and any known base/gate resistors.
- Host control-line voltage sources or USB-UART output macromodels.
- MCU BOOT0/NRST input load, clamp/leakage, capacitance, and threshold.
- Optional measured parasitic capacitances when bench data is available.

The physical assertion should check waveforms such as:

- `V(BOOT0)` is below the target low threshold before the MCU boot-sampling
  instant for application boot.
- `V(NRST)` crosses the reset-release threshold with sufficient margin and does
  not remain clamped by the saturated transistor network.
- Release timing is robust across declared process/model corners when those
  corners are available.

Executable assertions now support single-point samples, min/max/mean/RMS
windows, signed integration/energy windows, rising/falling crossing-time checks,
minimum high/low pulse-width checks, duty-cycle checks, and threshold
crossing-count checks for no-recross or ringing budgets, plus settling-time and
overshoot-percent checks against voltage/current/power targets and tolerances,
phase-delay checks between two probes, and setup/hold timing checks that measure
checked-signal stability around reference probe edges.
Analog run setups can declare bounded run-input sweeps; each corner records the
sweep name, corner name, raw parameter values, generated component value inputs,
and selected model sections on generated findings. Raw `parameters` still emit
ordinary ngspice `.param` cards. `component_values` entries target generated
primitive fields such as `RLOAD.value_ohm`, `CLOAD.value_f`, `VSUPPLY.dc_v`, or
`ILOAD.dc_a`; CircuitCI emits a nominal generated `.param` for that field and
uses the generated parameter in the primitive line, so load/source corners can
be driven from Board IR component names instead of hand-written SPICE parameter
names. `monte_carlo` sweep blocks deterministically sample generated component
fields around a nominal value and tolerance using the declared seed; sampled
values are expanded into the same component-value override path, so artifacts
and margin summaries remain reproducible. Monte Carlo component entries support
`distribution: uniform` and `distribution: normal`; normal sampling treats the
declared tolerance as +/-3 sigma and clamps generated z-scores to that span.
Swept assertions emit
`ANALOG_SWEEP_MARGIN_SUMMARY` info findings that identify the worst evaluated
corner, parameter values, component value inputs, selected model-library
sections, measured value, limit, relation, and numeric margin for each
assertion. Monte Carlo sweeps also emit `ANALOG_MONTE_CARLO_YIELD_SUMMARY`
findings with pass/fail sample counts, yield percent, mean margin, standard
deviation, min/max margin, p1/p5/p50/p95 sampled-margin percentiles, and the
limiting sample. Optional `monte_carlo.criteria` limits can require minimum
yield percent or minimum p1/p5/p50/p95 margin; failed criteria promote the
Monte Carlo summary finding to critical. With criteria declared, per-sample
assertion failures become tagged evidence rows so the declared yield target
controls pass/fail, while backend and solver failures remain critical. Sweeps
can select vendor model-library sections through
`model_sections`; CircuitCI emits section-specific ngspice `.lib "path" section`
cards for each corner. When a sweep declares `TEMP_C` or `TEMPERATURE_C`,
CircuitCI emits both the matching `.param` and an ngspice `.temp` card for that
corner.

Quantitative correctness depends on model quality. For saturation-dominated BJT
release timing, model inputs must cover transistor storage/recovery, diode
capacitance/recovery, host output impedance, resistor tolerances, supply range,
temperature, MCU input leakage/clamps/capacitance, and relevant board
parasitics. Missing model provenance or corner coverage must remain visible as
blocking physical limitations.

Discrete transistor and MOSFET models must carry datasheet-derived device
parameters, not only package/pin metadata. Examples:

- MOSFET: `Qg`, `Qgd`, `Qgs`, `Ciss`, `Coss`, `Crss`, `Rds(on)` at stated
  `Vgs`/`Id`/temperature, `Vgs(th)` range, body-diode `Qrr`/`trr`, SOA and
  thermal resistance.
- BJT: current gain ranges versus collector current, `VCE(sat)` at forced beta,
  transition frequency, input/output capacitance, delay/storage/fall times when
  provided, breakdown voltages, leakage, and thermal limits.
- Diode: forward voltage/current curve points, junction capacitance, reverse
  recovery, leakage, and breakdown.

These values either parameterize the SPICE model directly, constrain model-fit
quality, or become sweep/corner inputs. Missing values that affect the claimed
analysis must be explicit model-quality limitations.

## Report Honesty Rules

- A behavioral pass may be reported only as behavioral.
- A physical analog pass requires an analog backend artifact trail.
- Hand-authored decks must be bound to Board IR nets and pins so the simulated
  circuit can be audited against the schematic.
- A missing backend is a critical finding for `analog_transient` acceptance, not
  a non-blocking limitation.
- The suite name and description must distinguish behavioral acceptance from
  physical analog acceptance.

## Implementation Plan

1. Add typed `analog` scenario metadata to the Board IR and JSON schema. Done.
2. Add `SPICE_TRANSIENT_ANALYSIS` dispatch for `analog_transient` scenarios. Done.
3. Add a Rust analog backend module that detects `ngspice`/`Xyce`, resolves
   deck paths, and fails critically when physical prerequisites are missing. Done.
4. Add a UM physical acceptance fixture and suite. On hosts without a mature
   SPICE backend it fails closed with `ANALOG_BACKEND_UNAVAILABLE`; on this host
   with ngspice 46 installed it runs the transient deck and fails the bad
   circuit with quantitative `SPICE_TRANSIENT_ANALYSIS` findings. Done.
5. Add explicit `embedded_ngspice` backend selection that fails unless a mature
   ngspice-derived engine is actually dynamically loaded, linked, or vendored.
   Do not implement a toy partial solver. Done for system `libngspice`.
6. Add the real external ngspice runner and waveform parser. Done.
7. Add the real embedded ngspice shared-library runner. Done for dynamic
   `libngspice` loading through `ngSpice_Circ` and `ngSpice_Command`, using the
   same waveform CSV parser and report evidence contract as the external
   backend.
8. Add generic model-library support for device/subcircuit model packs and
   board-to-SPICE netlist generation.
9. Replace UM physical acceptance failure with measured waveform assertions
   from the real SPICE run. Done for the hand-authored UM Q2/Q3 fixture.
