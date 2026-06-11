# SPICE-Class Analog Transient Backend

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
- Keep chip behavior out of the analog solver. MCU pins become electrical
  thresholds, leakage/current limits, capacitance, clamps, and stimulus/load
  models. Firmware and boot ROM behavior stays in digital/protocol validators.
- Preserve artifacts: generated/provided netlists, included model files, raw or
  CSV waveforms, solver logs, and assertion measurements must be referenced in
  reports.

## Backend Contract

An `analog_transient` scenario owns a SPICE deck and waveform assertions.

Required fields:

- `backend`: `ngspice`, `xyce`, `embedded_ngspice`, or `auto`.
- `netlist`: path to a SPICE-compatible transient deck.
- `model_files`: SPICE model-card or subcircuit files used by the deck.
- `node_bindings`: mapping from SPICE nodes to Board IR nets.
- `pin_bindings`: mapping from Board IR component pins to SPICE nodes.
- `analysis`: transient settings, including stop time and maximum step.
- `stimuli`: named host, power, or load events when the deck is generated from
  board IR. For hand-authored decks this can be empty.
- `probes`: named voltages/currents to export.
- `assertions`: threshold checks over waveform samples.

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
   - `embedded_ngspice` means a mature ngspice-derived solver must be compiled
     or linked into CircuitCI behind the analog adapter. It must not resolve to
     a partial in-house SPICE subset.
   - `auto` chooses the first available configured backend.
3. If no required backend is available, emit a critical
   `ANALOG_BACKEND_UNAVAILABLE` finding.
4. If the netlist or included model files are missing, emit a critical
   `ANALOG_NETLIST_UNAVAILABLE` finding.
5. Validate that node and pin bindings map to real Board IR nets and component
   pins.
6. Run transient analysis and export machine-readable waveform data.
7. If the backend exits nonzero or reports non-convergence, emit a critical
   `SPICE_TRANSIENT_ANALYSIS` finding.
8. Evaluate waveform assertions. Failed assertions emit critical
   `SPICE_TRANSIENT_ANALYSIS` findings with measured and limit data.
9. For generated Board IR decks, append datasheet operating-limit probes for
   MOSFET/BJT voltage, current, and power ratings. Exceeding a rating emits a
   critical `SPICE_OPERATING_LIMIT` finding with measured maximum absolute
   transient stress and the datasheet limit. Missing usable MOSFET/BJT
   absolute-maximum rating metadata also emits `SPICE_OPERATING_LIMIT` before
   solver execution.
10. Passing physical analog acceptance requires no critical findings, no
   blocking analog limitations, and suite-required waveform/artifact evidence.

Until steps 5-7 are implemented for a real backend, CircuitCI must not present
the UM USB downloader physical acceptance as passing.

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

Executable assertions now support single-point samples, min/max windows, and
voltage/current/power thresholds. A complete physical acceptance language also
needs crossing-time, setup/hold, minimum pulse width, ringing/no-recross,
integration/energy, and corner-sweep assertions.

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
   ngspice-derived engine is actually linked or vendored. Do not implement a
   toy partial solver.
6. Add the real external ngspice runner and waveform parser. Done.
7. Add generic model-library support for device/subcircuit model packs and
   board-to-SPICE netlist generation.
8. Replace UM physical acceptance failure with measured waveform assertions
   from the real SPICE run. Done for the hand-authored UM Q2/Q3 fixture.
