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
assertions. An `analog_dc_sweep` scenario owns a swept independent source,
normalized sweep-curve export, and min/max/mean/sample assertions over the
curve. An `analog_sparameter` scenario owns a frequency sweep plus single-port
or multi-port reference-impedance definitions for Xyce S-parameter exports. An
`analog_transfer_function` scenario owns a small-signal output/input source
contract for `.TF` gain and resistance exports. An `analog_pole_zero` scenario
owns a small-signal output/reference/input-source contract for `.PZ` pole and
zero extraction. An `analog_sensitivity` scenario owns a DC or AC
output-variable sensitivity contract with optional ngspice parameter filters
for `.SENS` exports. An `analog_distortion` scenario owns a small-signal
distortion contract for ngspice `.DISTO`-style harmonic and intermodulation
products. An `analog_fourier` scenario owns a transient-backed
`.FOUR` harmonic extraction contract for a bound output expression and
fundamental frequency. An `analog_harmonic_balance` scenario owns a periodic
steady-state harmonic-balance contract for drive sources, a fundamental
frequency, a bound output expression, and normalized spectrum evidence. An
`analog_pss` scenario owns periodic steady-state or oscillator evidence
requirements: mode, frequency guess, stabilization interval, output expression,
drive-source provenance when driven, and convergence metadata. An
`analog_phase_noise` scenario owns oscillator phase-noise evidence
requirements: carrier frequency, offset sweep bounds, output expression,
optional integration window, drive-source provenance when driven, and a hard
dependency on trusted PSS/PNOISE convergence artifacts. An
`analog_periodic_ac` scenario owns PAC/PXF-style periodic small-signal
evidence requirements: carrier frequency, sideband count, small-signal
frequency sweep, output expression, input source, large-signal drive-source
provenance, and a hard dependency on trusted PSS/HB linearization convergence
artifacts. An
`analog_measure` scenario owns reviewed ngspice `.MEASURE` scalar extraction
statements or portable structured measure templates for transient or AC
results.

Required fields:

- `backend`: `ngspice`, `xyce`, `embedded_ngspice`, or `auto`.
- `netlist`: path to a SPICE-compatible deck.
- `model_files`: SPICE model-card, subcircuit, or compiled OSDI files used by
  the deck. Ordinary SPICE includes should be SHA-pinned. OpenVAF/OSDI compact
  model artifacts additionally declare `artifact_format: osdi_shared_object`,
  `source_path`, `source_sha256`, `compiler: openvaf`, `compiler_version`, and
  `compiler_command` so the compiled shared object is tied to auditable
  Verilog-A source and build provenance. The compiler command must invoke
  `openvaf` and reference the declared Verilog-A source and OSDI output path;
  missing or hash-stale compiled artifacts fail closed with a rebuild plan
  unless CI opts into direct OpenVAF execution with
  `CIRCUITCI_RUN_OPENVAF_BUILDS=1`, after which the produced artifact hash must
  match the declared pin before the backend can run. External ngspice wrappers
  load OSDI artifacts with `pre_osdi` inside `.control`; generated netlists do
  not `.include` the binary artifact. A runtime that rejects `pre_osdi` fails
  closed and preserves the wrapper/log artifacts. Opt-in real ngspice/OpenVAF
  conformance uses
  `CIRCUITCI_RUN_REAL_NGSPICE_OSDI=1 cargo test --test analog_model_compiler_cli`;
  it skips unless `ngspice`, `openvaf`, and ngspice `pre_osdi` command support
  are available on the host. Explicit `backend: xyce` with OSDI model artifacts
  fails closed: primary Xyce documentation describes Verilog-A support through
  Xyce/ADMS-generated C++ linked into Xyce or loaded as a Xyce `-plugin`, not
  through OpenVAF `*.osdi` runtime loading. The evidence boundary is documented
  in
  `docs/research/circuit_simulation_full_featured/xyce_openvaf_osdi_compatibility.md`.
  Solver manifests include
  `inputs.model_file_provenance[]` for OpenVAF/OSDI entries, recording declared
  and actual source/artifact hashes, compiler command/version, compiler
  availability, whether the OpenVAF build env was enabled, and `rebuild_mode`
  such as `prebuilt_verified`, `rebuilt_missing_artifact`, or
  `rebuilt_hash_stale_artifact`. Report JSON, Markdown, and GUI artifact views
  also expose these records as `model_file_provenance[]` with scenario,
  analysis, backend, and manifest-path context. Xyce/ADMS model plugins use a
  separate fail-closed contract, `artifact_format: xyce_adms_plugin`, with
  source/plugin/conformance SHA-256 pins, `compiler: xyce_adms`, a
  `buildxyceplugin` command, `plugin_load_command` containing Xyce `-plugin`,
  Xyce version, Xyce/ADMS template revision, and configure options including
  `--enable-shared` and `--enable-xyce-shareable`. CircuitCI validates those
  fields and artifacts but emits `ANALOG_MODEL_COMPILER_XYCE_PLUGIN_UNSUPPORTED`
  until a real-Xyce plugin loader is enabled. The opt-in qualification hook
  `CIRCUITCI_RUN_REAL_XYCE_ADMS_PLUGIN=1 cargo test --test analog_model_compiler_cli`
  skips unless `Xyce`/`xyce` and `buildxyceplugin` are on `PATH`; when runnable
  it builds the Xyce tutorial RLC Verilog-A plugin, loads it with `Xyce
  -plugin`, pins the retained conformance artifact, and verifies CircuitCI's
  fail-closed contract against those real artifacts.
  Reusable compact-model packages can additionally declare
  `model_package_name`, `model_package_version`, `model_package_artifact_id`,
  `model_package_lock_path`, and `model_package_lock_sha256`. The lock file may
  be JSON or YAML and is schema-backed by
  `schemas/model_package_lock.schema.json`; it must contain matching package
  identity plus an artifact row with matching id, path, SHA-256, artifact
  format, and compiler. Scenarios may inline those lock fields or reference a
  pinned package registry entry with `model_package_registry_path`,
  `model_package_registry_sha256`, and `model_package_registry_entry`; registry
  entries import the package identity and lock pointer and must not disagree
  with any explicit scenario metadata. Valid package locks and registries are
  retained as report artifacts and projected into `solver_manifest.json` plus
  top-level `model_file_provenance[]` so reports can distinguish
  package-qualified compact models from one-off scenario files. Use
  `circuitci export-model-package` to generate deterministic JSON locks and
  optional registries from real artifact hashes; repeat
  `--package-artifact id=...,path=...,artifact_format=...[,compiler=...]` when
  one package must pin paired Verilog-A source, OpenVAF/OSDI, Xyce/ADMS plugin,
  and conformance-report artifacts, and use `--registry-artifact-id` to select
  the runtime artifact imported by registry-backed scenarios. Then use
  `circuitci verify-model-package` to validate the lock and optional registry
  before adding the package to a scenario. Use
  `circuitci merge-model-package-registry` to aggregate exported package
  registries into a shared deterministic registry while rewriting lock paths
  relative to that shared registry; verification output follows
  `schemas/model_package_verification_report.schema.json`.
- Generated scenarios that use the generic analog behavioral library preserve
  the built-in registry-qualified package entry
  `generic_analog_behavioral_spice` from
  `models/packages/compact_model_registry.json`, so report
  `model_file_provenance[]` shows the reusable package source rather than only
  a copied include-file hash.
- `node_bindings`: mapping from SPICE nodes to Board IR nets.
- `pin_bindings`: mapping from Board IR component pins to SPICE nodes.
- `analysis`: transient settings with stop time and maximum step, AC/noise/
  S-parameter settings with start/stop frequency and points per decade,
  `type: op` for DC operating-point analysis, `type: dc_sweep` with
  `dc_sweep_source`, `dc_sweep_start`, `dc_sweep_stop`, `dc_sweep_step`, and
  optional `dc_sweep_assertions[]`, `s_parameter_ports` for S-parameter port
  contracts, and `type: tf` with
  `transfer_output_expression` plus `transfer_input_source` for transfer
  function contracts. Pole-zero contracts use `type: pz`,
  `pole_zero_output_node`, `pole_zero_reference_node`,
  `pole_zero_input_source`, and `pole_zero_mode`. Sensitivity contracts use
  `type: sens`, `sensitivity_output_expression`, `sensitivity_mode`, optional
  `sensitivity_filters`, and AC frequency fields when `sensitivity_mode: ac`.
  Distortion contracts use `type: disto`,
  `distortion_mode: harmonic|intermodulation`, frequency sweep fields
  `distortion_start_frequency_hz`, `distortion_stop_frequency_hz`, and
  `distortion_points_per_decade`, a bound `distortion_output_expression`,
  `distortion_f1_sources[]`, optional `distortion_f2_sources[]`, and
  `distortion_f2_over_f1` for intermodulation. Fourier contracts use `type:
  fourier`, transient `stop_time_us` and
  `max_step_us`, `fourier_fundamental_frequency_hz`,
  `fourier_output_expression`, and optional `fourier_harmonics`. PSS contracts
  use `type: pss`, `pss_frequency_guess_hz`,
  `pss_stabilization_time_us`, `pss_output_expression`, optional
  `pss_mode: driven|autonomous`, optional `pss_periods`,
  `pss_drive_sources[]` for driven generated-source provenance, and optional
  `pss_residual_tolerance`, `pss_state_error_tolerance`, and
  `pss_max_iterations`. Phase-noise contracts use `type: phase_noise`,
  `phase_noise_carrier_frequency_hz`, offset sweep fields
  `phase_noise_offset_start_hz`, `phase_noise_offset_stop_hz`,
  `phase_noise_points_per_decade`, `phase_noise_output_expression`, optional
  `phase_noise_mode: driven|autonomous`, optional
  `phase_noise_drive_sources[]`, and optional integration window fields
  `phase_noise_integration_start_hz` / `phase_noise_integration_stop_hz`.
  Measurement
  contracts use `type: measure`, `measure_mode: tran|ac`, and either reviewed
  raw `measure_statements[]` or portable `measure_templates[]`; transient
  measure mode uses `stop_time_us` and `max_step_us`, while AC measure mode
  uses start/stop frequency and points per decade.
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
   manifest also records OpenVAF/OSDI source/artifact hash verification and
   rebuild mode under `inputs.model_file_provenance[]`. It is a backend-neutral
   contract for future Xyce and RF/HB adapters. Report consumers can inspect
   top-level `model_file_provenance[]` first and open the referenced manifest
   only when they need full solver inputs or output discovery instead of
   inferring run state from backend-specific filenames alone.
   OpenVAF `*.osdi` artifacts are currently external-ngspice-only in CircuitCI;
   Xyce compact-model support uses the separate fail-closed
   `xyce_adms_plugin` contract and still requires a real-Xyce plugin loader
   before execution is enabled. The real plugin-build/load qualification hook is
   `CIRCUITCI_RUN_REAL_XYCE_ADMS_PLUGIN=1 cargo test --test analog_model_compiler_cli`.
   Opt-in real-Xyce conformance coverage is available through
   `CIRCUITCI_RUN_REAL_XYCE=1 cargo test --test analog_spice_xyce_cli` for
   transient/AC/DC/noise,
   `CIRCUITCI_RUN_REAL_XYCE=1 cargo test --test analog_dc_sweep_cli` for DC
   sweep,
   `CIRCUITCI_RUN_REAL_XYCE=1 cargo test --test analog_sparameter_cli` for
   S-parameter, and
   `CIRCUITCI_RUN_REAL_XYCE=1 cargo test --test analog_measure_cli` for
   structured measure templates. These tests are skipped by default unless the
   variable is set and `Xyce` or `xyce` is on `PATH`.
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

For a scenario with check `SPICE_DC_SWEEP_ANALYSIS`:

1. Resolve and bind the scenario netlist and model files the same way as other
   analog validations.
2. Require `analysis.type: dc_sweep`, a finite source sweep range, at least one
   probe, and a generated-board source component when the deck is generated
   from Board IR.
3. Select external `ngspice` or explicit `backend: xyce`; embedded ngspice
   fails closed with planning evidence until equivalent output normalization
   is implemented. `backend: auto` remains conservative and selects ngspice
   for this path.
4. Run `.dc`/`.DC`, export raw sweep probe data, and normalize it to
   `dc_sweep.csv` rows with `sweep_source`, `sweep_value`, `probe`, and
   `value`.
5. Optional `dc_sweep_assertions[]` evaluate `min`, `max`, `mean`, or `sample`
   values from the normalized curve. Failed assertions emit critical
   `SPICE_DC_SWEEP_ANALYSIS` findings with measured value, sweep point, limit,
   relation, and artifact path.
6. Swept input-corner and Monte Carlo summaries reuse the same worst-corner and
   yield report machinery used by waveform, AC, operating-point, noise, and
   measure assertions.

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

For a scenario with check `SPICE_POLE_ZERO_ANALYSIS`:

1. Resolve and bind the scenario netlist and model files the same way as other
   analog validations.
2. Require `analysis.type: pz`, non-empty `pole_zero_output_node`,
   `pole_zero_reference_node`, `pole_zero_input_source`, and `pole_zero_mode`.
3. Require output/reference nodes to be bound through `node_bindings`; for
   generated Board IR decks, `pole_zero_input_source` must name a generated
   board component.
4. `pole_zero_mode` is one of `poles`, `zeros`, or `poles_and_zeros`.
5. External `backend: ngspice` writes `circuitci_ngspice_pz.cir`,
   `ngspice_pz.log`, `pole_zero_raw.txt`, `pole_zero_summary.csv`, and
   `solver_manifest.json`. The wrapper uses the ngspice control-command form
   `pz node1 node2 node3 node4 cur|vol pol|zer|pz`, omits the declared input
   source element from the small-signal PZ deck, and normalizes printed roots
   into complex rad/s plus derived frequency. Opt-in real-ngspice conformance
   coverage is available through
   `CIRCUITCI_RUN_REAL_NGSPICE=1 cargo test --test analog_pole_zero_cli`; the
   test is skipped by default unless the variable is set and `ngspice` is on
   `PATH`.
6. Xyce and embedded ngspice remain fail-closed with backend-planning evidence
   until those adapters emit the same normalized `pole_zero_summary` contract.

For a scenario with check `SPICE_SENSITIVITY_ANALYSIS`:

1. Resolve and bind the scenario netlist and model files the same way as other
   analog validations.
2. Require `analysis.type: sens`, non-empty `sensitivity_output_expression`,
   and `sensitivity_mode` set to `dc` or `ac`.
3. `sensitivity_output_expression` must be a bound `V(node)`,
   `V(node,reference)`, or `I(source)` expression. Optional
   `sensitivity_filters[]` map to ngspice SENS filters.
4. AC mode requires `start_frequency_hz`, `stop_frequency_hz`, and
   `points_per_decade`, matching the AC sweep fields used elsewhere.
5. External `ngspice` writes `sensitivity_raw.txt`,
   `sensitivity_summary.csv`, and `solver_manifest.json`. DC sensitivity rows
   normalize scalar parameter derivatives; AC sensitivity rows normalize
   per-frequency complex sensitivity values plus magnitude.
6. Opt-in real-ngspice conformance coverage is available through
   `CIRCUITCI_RUN_REAL_NGSPICE=1 cargo test --test analog_sensitivity_cli`;
   the test is skipped by default unless the variable is set and `ngspice` is
   on `PATH`.
7. Xyce and embedded ngspice remain fail-closed with backend-planning evidence
   until those adapters emit the same normalized `sensitivity_summary`
   contract.

For a scenario with check `SPICE_DISTORTION_ANALYSIS`:

1. Resolve and bind the scenario netlist and model files the same way as other
   analog validations.
2. Require `analysis.type: disto`, positive finite
   `distortion_start_frequency_hz`, `distortion_stop_frequency_hz` greater
   than the start frequency, `distortion_points_per_decade` in `1..=1000`, a
   bound `distortion_output_expression`, and at least one
   `distortion_f1_sources[]` entry.
3. `distortion_mode` defaults to `harmonic` and may be `harmonic` or
   `intermodulation`. Harmonic mode must omit `distortion_f2_over_f1`;
   intermodulation mode requires finite `distortion_f2_over_f1` in `0..1` and
   at least one `distortion_f2_sources[]` entry. For generated Board IR decks,
   all declared distortion source names must resolve to generated components.
4. `distortion_output_expression` must be a bound `V(node)`,
   `V(node,reference)`, or `I(source)` expression.
5. For `backend: ngspice`, CircuitCI writes a wrapper deck that annotates the
   declared source lines with explicit `DISTOF1 1.0 0.0` and `DISTOF2 1.0 0.0`
   defaults, runs `.disto dec`, selects the ngspice distortion plots, and
   prints the requested output expression.
6. Successful ngspice runs retain the wrapper, solver log/raw text,
   `distortion_spectrum.csv`, `distortion_summary.csv`,
   `distortion_convergence.json`, and
   `solver_manifest.json`. The spectrum rows include component label,
   frequency, complex value, magnitude, and phase. The summary records each
   component's row count and maximum magnitude. The convergence JSON records
   backend, mode, row count, selected components, output expression, and the
   shared non-convergence detector result.
7. Validation reports project `distortion_summary.csv` rows into top-level
   `distortion_summaries[]`, Markdown reports include a "Distortion Summary"
   section, and GUI Scopes loads `distortion_spectrum.csv` as frequency-axis
   magnitude, phase, real, and imaginary traces per distortion component.
8. Real-solver conformance is opt-in through
   `CIRCUITCI_RUN_REAL_NGSPICE_DISTO=1 cargo test --test analog_distortion_cli`;
   it skips unless `ngspice` is on `PATH`.
9. Xyce and embedded ngspice remain fail-closed with backend-planning evidence.
   The saved Xyce 7.8 reference text does not document a matching distortion
   command; QUCS-S and SPICE OPUS references are not current CircuitCI adapter
   contracts.

For a scenario with check `SPICE_FOURIER_ANALYSIS`:

1. Resolve and bind the scenario netlist and model files the same way as other
   analog validations.
2. Require `analysis.type: fourier`, positive finite transient `stop_time_us`
   and `max_step_us`, positive finite `fourier_fundamental_frequency_hz`, and a
   non-empty `fourier_output_expression`.
3. `fourier_output_expression` must be a bound `V(node)`,
   `V(node,reference)`, or `I(source)` expression.
4. The stop time must cover at least one fundamental period. Optional
   `fourier_harmonics` must be in `1..=1024`; omitted harmonic count defaults
   to ten non-DC harmonics.
5. External `ngspice` writes `circuitci_ngspice_fourier.cir`,
   `ngspice_fourier.log`, `fourier_raw.txt`, `fourier_summary.csv`, and
   `solver_manifest.json`. The normalized summary records the DC row,
   harmonic number, frequency, magnitude, phase, normalized magnitude/phase,
   and solver-reported THD/grid metadata.
6. Opt-in real-ngspice conformance coverage is available through
   `CIRCUITCI_RUN_REAL_NGSPICE=1 cargo test --test analog_fourier_cli`; the
   test is skipped by default unless the variable is set and `ngspice` is on
   `PATH`.
7. Xyce and embedded ngspice remain fail-closed with backend-planning evidence
   until those adapters emit the same normalized `fourier_summary` contract.

For a scenario with check `SPICE_HARMONIC_BALANCE_ANALYSIS`:

1. Resolve and bind the scenario netlist and model files the same way as other
   analog validations.
2. Require `analysis.type: hb`, positive finite
   `hb_fundamental_frequency_hz`, non-empty `hb_output_expression`, and at
   least one `hb_drive_sources[]` entry.
3. `hb_output_expression` must be a bound `V(node)`, `V(node,reference)`, or
   `I(source)` expression. For generated Board IR decks, every drive source
   must name a generated board component so the planned large-signal periodic
   excitation is tied to source provenance.
4. Optional `hb_harmonics` must be in `1..=1024`; omitted harmonic count
   defaults to ten harmonics in planning evidence.
5. Explicit `backend: xyce` writes `circuitci_xyce_hb.cir`, `xyce_hb.log`,
   `hb_spectrum_raw.csv`, `hb_spectrum.csv`, and `solver_manifest.json`. The
   wrapper uses `.HB <fundamental>`, `.OPTIONS HBINT NUMFREQ=<harmonics>`, and
   `.PRINT HB_FD FORMAT=CSV` based on the Xyce 7.8 Reference Guide. The
   normalized spectrum records signed harmonic index, frequency, complex
   output value, magnitude, and phase.
6. The GUI Scopes loader recognizes `hb_spectrum.csv` artifacts and converts
   non-negative harmonic rows into frequency-axis traces for magnitude, phase,
   real, and imaginary output values.
7. Opt-in real-solver conformance is available with
   `CIRCUITCI_RUN_REAL_XYCE=1 cargo test --test analog_harmonic_balance_cli`;
   it skips unless `Xyce` or `xyce` is on `PATH`.
8. `backend: auto`, `ngspice`, and `embedded_ngspice` remain fail-closed with
   backend-planning evidence until those adapters emit the same normalized
   `hb_spectrum` contract.

For a scenario with check `SPICE_PSS_ANALYSIS`:

1. Resolve and bind the scenario netlist and model files the same way as other
   analog validations.
2. Require `analysis.type: pss`, a positive finite
   `pss_frequency_guess_hz`, positive finite `pss_stabilization_time_us`, and a
   bound `pss_output_expression`.
3. `pss_mode` defaults to `driven` and may be `driven` or `autonomous`.
   Driven mode requires at least one `pss_drive_sources[]` entry. For generated
   Board IR decks, each drive source must name a generated board component so
   the periodic excitation has source provenance. Autonomous mode may omit
   drive sources for oscillator evidence planning.
4. `pss_output_expression` must be a bound `V(node)`, `V(node,reference)`, or
   `I(source)` expression. Optional `pss_periods` must be in `1..=4096`;
   optional `pss_residual_tolerance` and `pss_state_error_tolerance` must be
   positive finite values; optional `pss_max_iterations` must be in
   `1..=10000`.
5. The current implementation intentionally fails closed after contract
   validation. It emits backend-planning evidence with planned normalized
   outputs `pss_waveform`, `pss_spectrum`, and `pss_convergence`, and records
   the manifest schema expected from a future solver adapter.
6. The backend-planning finding records the current primary-source assessment:
   Xyce 7.8 has HB but no separate documented `.PSS` command; ngspice PSS is
   experimental, autonomous-focused, build-gated by `--enable-pss`, and lacks a
   trusted normalized output contract in this runtime; SPICE OPUS `ssse`
   shooting is documented but has no CircuitCI backend adapter or conformance
   suite.
7. CircuitCI must not present PSS or oscillator sign-off as passing until a
   trusted backend emits the normalized waveform, spectrum, convergence, raw
   solver output, and solver-manifest artifacts.

For a scenario with check `SPICE_PHASE_NOISE_ANALYSIS`:

1. Resolve and bind the scenario netlist and model files the same way as other
   analog validations.
2. Require `analysis.type: phase_noise`, a positive finite
   `phase_noise_carrier_frequency_hz`, positive finite
   `phase_noise_offset_start_hz`, `phase_noise_offset_stop_hz` greater than
   the start offset, `phase_noise_points_per_decade` in `1..=1000`, and a
   bound `phase_noise_output_expression`.
3. `phase_noise_mode` defaults to `autonomous` and may be `driven` or
   `autonomous`. Driven mode requires at least one
   `phase_noise_drive_sources[]` entry. For generated Board IR decks, each
   drive source must name a generated board component so the large-signal
   periodic excitation has source provenance.
4. `phase_noise_output_expression` must be a bound `V(node)`,
   `V(node,reference)`, or `I(source)` expression. If integration limits are
   provided, both `phase_noise_integration_start_hz` and
   `phase_noise_integration_stop_hz` must be present, ordered, positive, and
   contained inside the offset sweep.
5. The current implementation intentionally fails closed after contract
   validation. It emits backend-planning evidence with planned normalized
   outputs `phase_noise_spectrum`, `phase_noise_integrated_jitter`,
   `phase_noise_convergence`, and `pss_convergence`.
6. The backend-planning finding records the current source-backed boundary:
   QUCS-COPEN papers document a `pnsolver` after `psssolver`, but no public
   source repository or adapter contract was found; Xyce HB is not treated as
   PSS/PNOISE; ngspice PSS/PNOISE remains experimental or build/runtime
   dependent without a trusted normalized contract here; SPICE OPUS `ssse`
   has no CircuitCI phase-noise adapter or conformance suite.
7. CircuitCI must not present oscillator phase-noise sign-off as passing until
   a trusted backend emits normalized phase-noise spectrum, integrated jitter,
   PSS convergence, phase-noise convergence, raw solver output, and
   solver-manifest artifacts.

For a scenario with check `SPICE_PERIODIC_AC_ANALYSIS`:

1. Resolve and bind the scenario netlist and model files the same way as other
   analog validations.
2. Require `analysis.type: pac`, positive finite
   `pac_carrier_frequency_hz`, `pac_start_frequency_hz`,
   `pac_stop_frequency_hz` greater than the start frequency,
   `pac_points_per_decade` in `1..=1000`, a bound `pac_output_expression`,
   and `pac_input_source`.
3. `pac_mode` defaults to `pac` and may be `pac` or `pxf`. `pac_sidebands`
   defaults to `1` and may be `0..=1024`. For generated Board IR decks,
   `pac_input_source` and each optional `pac_drive_sources[]` entry must name
   a generated board component so the small-signal injection and large-signal
   periodic drive have explicit provenance.
4. `pac_output_expression` must be a bound `V(node)`,
   `V(node,reference)`, or `I(source)` expression.
5. The current implementation intentionally fails closed after contract
   validation. It emits backend-planning evidence with planned normalized
   outputs `pac_response`, `pac_sidebands`, `pac_convergence`, and
   `pss_convergence`.
6. The backend-planning finding records the current source-backed boundary:
   Xyce HB is available but the saved Xyce primary docs do not document PAC/PXF
   command/output artifacts; ngspice documents experimental autonomous PSS and
   names PAC/PNoise as downstream analyses but does not provide a stable
   PAC/PXF command/output contract; QUCS-COPEN remains a theory source until
   public source, build, adapter, and conformance artifacts are found.
7. CircuitCI must not present periodic small-signal sign-off as passing until a
   trusted backend emits normalized PAC/PXF response, sideband, convergence,
   raw solver output, and solver-manifest artifacts with real-solver
   conformance coverage.

For a scenario with check `SPICE_MEASURE_ANALYSIS`:

1. Resolve and bind the scenario netlist and model files the same way as other
   analog validations.
2. Require `analysis.type: measure`, `measure_mode: tran|ac`, and at least one
   named raw `measure_statements[]` item or structured `measure_templates[]`
   item.
3. Each raw statement must be a single-line ngspice `meas`/`.meas` command
   whose mode and result name match the declared metadata.
4. Each template declares `name`, `operation`, `expression`, and operation
   fields. Simple operations are `avg`, `max`, `min`, `rms`, and `find` with
   optional time/frequency windows. Transient timing operations include
   `delay`, `slew`, and `threshold_time`: `delay` compares trigger and target
   expressions, `slew` compares two thresholds on the same expression, and
   `threshold_time` records one crossing time. They render to SPICE
   `TRIG`/`TARG` or `WHEN` measurements with optional edge selectors and
   crossing counts. CircuitCI renders templates into backend-specific measure
   commands: ngspice receives generated `meas` commands, while explicit Xyce
   receives generated `.MEASURE` commands.
5. Voltage/current references in raw statements or templates must bind to
   declared scenario nodes/components. Transient mode requires positive finite
   `stop_time_us` and `max_step_us`. AC mode requires valid start/stop
   frequency and points per decade.
6. External `ngspice` writes `circuitci_ngspice_measure.cir`,
   `ngspice_measure.log`, `measure_raw.txt`, `measure_summary.csv`, and
   `solver_manifest.json`. The normalized summary records measurement name,
   mode, scalar value, and raw solver line.
7. Optional `measure_assertions[]` bind normalized `measure_summary.csv` rows to
   design limits. Each assertion names one measurement, declares `relation:
   above|below`, a finite `threshold`, and an optional display `unit`. A failed
   assertion emits a normal `SPICE_MEASURE_ANALYSIS` finding with the measured
   value, threshold, margin, summary artifact, and suggested fixes. Declared
   run-input sweeps record measure assertion margins in the shared
   `ANALOG_SWEEP_MARGIN_SUMMARY` and `ANALOG_MONTE_CARLO_YIELD_SUMMARY`
   reports, including Monte Carlo criteria handling and per-sample assertion
   evidence demotion when criteria are declared.
8. Explicit `backend: xyce` supports structured `measure_templates[]` and writes
   `circuitci_xyce_measure.cir`, `xyce_measure.log`, `measure_raw.txt`,
   `measure_summary.csv`, and `solver_manifest.json` using the same normalized
   `measure_summary` contract. The adapter records Xyce measure result files
   such as `.mt0` when the solver emits them, with stdout as a fallback for
   compatible outputs. Raw `measure_statements[]` remain ngspice-only and fail
   closed on Xyce because their syntax is backend-specific.
9. Opt-in real-ngspice conformance coverage is available through
   `CIRCUITCI_RUN_REAL_NGSPICE=1 cargo test --test analog_measure_cli`; the
   test is skipped by default unless the variable is set and `ngspice` is on
   `PATH`.
10. Embedded ngspice remains fail-closed with backend-planning evidence until it
   emits the same normalized `measure_summary` contract. `backend: auto` remains
   conservative and does not select Xyce for measure runs.

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
