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
  static current budgets, and explicit regulator
  dropout/output-current/startup/capacitance metadata plus reset-supervisor
  threshold metadata. It does not infer a
  whole-board analog power tree or solve regulator ramp waveform shape,
  load-dependent dropout, inrush, charger/power-mux behavior, reset-output
  waveform shape, thermal behavior, or load-transient stability.
- `RESET_RELEASE_AFTER_POWER_VALID` can consume target rail `power_valid_at_us`
  and reset-supervisor delay metadata. It does not derive reset release from an
  analog RC/supervisor waveform unless an explicit `analog_transient` scenario
  is provided.
- `GPIO_BACKDRIVE` uses a simple diode/source-resistance approximation.
- `INTERFACE_PROTECTION_REVIEW` checks declared signal-conditioning channel
  metadata, unpowered-isolation claims, observed disabled-state evidence,
  declared static supply-order constraints, and clamp-only protection metadata
  such as reference net kind, standoff voltage, and line capacitance. It does
  not prove analog leakage, dynamic clamp current, ESD pulse performance,
  propagation delay, edge rate, USB eye margin, or signal integrity.
- `USB_CONNECTOR_PROTECTION_VALID` checks that declared USB connector D+/D- and,
  when requested, VBUS nets have connected clamp-only protection with compatible
  reference wiring and optional standoff-voltage evidence. When requested, it
  can also require the connector shield pin to connect to a declared ground net.
  It does not prove ESD pulse robustness, connector placement, RC/ferrite/chassis
  shield-bonding strategy, differential routing, return-path quality, USB eye
  margin, or layout-level protection effectiveness.
- `USB_PROTECTION_PLACEMENT_VALID` checks explicit component placement
  coordinates and center-to-center connector-to-protection distance for USB
  clamp coverage. It does not prove trace order, trace length, via count,
  parasitic inductance, shield strategy, return-path continuity, differential
  impedance, ESD pulse survival, or USB signal integrity.
- `USB_ROUTE_GEOMETRY_VALID` and `USB_VBUS_ROUTE_VALID` check imported static
  route geometry for USB data nets and VBUS respectively. VBUS route checks are
  limited to route length, via count, optional minimum segment width, and
  connector-to-protection route distance. They do not prove VBUS current
  capacity, fuse trip behavior, inrush current, voltage drop under load,
  temperature rise, or ESD pulse survival.
- `USB_RETURN_PATH_VALID` checks whether USB D+/D- route segment midpoints are
  inside same-layer ground-zone outlines. It does not prove filled-zone
  continuity, adjacent-plane coupling, impedance, eye margin, stitching-via
  quality, common-mode radiation, or return-current behavior under signal
  transitions.
- `CLOCK_SOURCE_VALID` checks declared external crystal support-network
  connectivity and load capacitance. It does not prove oscillator startup,
  negative resistance, ESR margin, drive level, ppm accuracy, temperature
  drift, or layout parasitics.
- Quantitative waveform proof is available only through `analog_transient`
  scenarios with a SPICE-class backend and explicit assertions.
- Imported SPICE decks can produce solver and waveform evidence, but an
  assertion-free imported deck reports `ANALOG_ASSERTIONS_ABSENT`; waveform
  evidence alone is not design sign-off.
- KiCad XML, native `.kicad_sch`, and `.kicad_pcb` layout-evidence import are
  conservative. Unsupported or ambiguous constructs fail closed instead of being
  guessed. PCB import currently extracts component center placements plus
  segment/via route geometry, copper-zone outlines, and a bounded subset of
  net-class/custom-rule route constraints for mapped nets, not full pad
  geometry, arbitrary DRC rule semantics, filled-copper connectivity, thermal
  relief behavior, return paths, or signal-integrity constraints.
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
