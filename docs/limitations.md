# Limitations

CircuitCI is a validation runtime, not a full EDA suite.

The runtime backbone is Rust. Python is not part of the production engine path.

## Not Implemented

- schematic editing
- PCB layout editing
- GHz RF or antenna solving
- DDR or high-speed signal integrity solving
- full USB PHY simulation
- full SMPS compensation design
- automatic datasheet-to-perfect-model generation
- broad firmware-in-loop MCU/peripheral machine coverage beyond the explicit
  QEMU pin-observation path

## Current Technical Limits

- Full transistor-level MCU simulation is intentionally not a CircuitCI goal.
  MCU models should be functional black boxes at the board boundary: firmware
  execution, reset/boot behavior, peripheral state, pin modes, electrical pin
  limits, thresholds, clamps, leakage, and timing that matter to the surrounding
  circuit.
- `firmware_in_loop` supports QEMU functional execution when the scenario
  declares a machine, firmware image, and expected board-facing pin states, and
  when the QEMU run emits explicit `CIRCUITCI_PIN` observations. It does not
  infer pins from MCU internals. Renode remains fail-closed until a Renode
  adapter is integrated.
- `POWER_TREE_VALID` checks declared rail power state, nominal voltage ranges,
  static current budgets, and explicit regulator dropout/output-current/startup
  metadata. It does not infer a whole-board analog power tree or solve
  regulator ramp waveform shape, load-dependent dropout, inrush,
  charger/power-mux behavior, thermal behavior, or load-transient stability.
- `GPIO_BACKDRIVE` uses a simple diode/source-resistance approximation.
- Quantitative waveform proof is available only through `analog_transient`
  scenarios with a SPICE-class backend and explicit assertions.
- Imported SPICE decks can produce solver and waveform evidence, but an
  assertion-free imported deck reports `ANALOG_ASSERTIONS_ABSENT`; waveform
  evidence alone is not design sign-off.
- KiCad XML and native `.kicad_sch` import are conservative. Unsupported or
  ambiguous constructs fail closed instead of being guessed.
- Component models are low-confidence generic behavioral models unless a vendor
  or datasheet-backed pack says otherwise.
- Reports include `LOW_CONFIDENCE_MODEL` limitations for `generic`, `estimated`, or `low` confidence models used by a project.
- `RESIDENT_BOOTLOADER_UPDATE_SEQUENCE` validates declared transaction traces and does not execute firmware, decode raw serial frames, recompute CRCs, emulate flash, or prove HIL behavior.
- `CONTROL_LINE_RELEASE_SEQUENCE` validates declared line effects and release delays and does not solve transistor storage, hidden RC networks, or physical modem-pin voltage truth tables.
- `analog_transient` scenarios are the only path intended for quantitative
  voltage/current waveform proof. If no SPICE-class backend is available, or if
  the solver cannot produce parseable waveform data, these scenarios fail with
  critical analog findings rather than producing fake passes.

Reports must include these limitations so automated agents and human users know when a pass does not imply full physical coverage.

For the broader gap list between the current tool and "verify any common IoT
board" readiness, see [common_iot_board_readiness_gaps.md](common_iot_board_readiness_gaps.md).
