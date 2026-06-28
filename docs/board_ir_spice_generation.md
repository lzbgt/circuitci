# Board IR to SPICE Generation

CircuitCI must not depend on hand-written fixture decks for every board issue.
The analog backend still delegates nonlinear device physics to mature SPICE
engines such as ngspice, but CircuitCI should be able to generate the SPICE deck
from Board IR and component model metadata.

## Scope

This slice adds generated transient and AC/Bode decks for board-local analog subcircuits.
It is not a new simulator and must not implement SPICE numerics in Rust. Rust
only translates audited Board IR into a solver deck, records artifacts, invokes
the mature backend, and evaluates waveform assertions.

Initial primitive coverage is intentionally small in resource usage, not a toy
scope:

- resistor,
- capacitor,
- inductor,
- independent DC voltage source,
- independent pulse voltage source,
- independent DC current source,
- independent pulse current source,
- diode backed by `simulation.spice`,
- BJT NPN/PNP backed by `simulation.spice`,
- N-channel and P-channel MOSFETs backed by `simulation.spice`,
- subcircuits backed by `simulation.spice` with explicit `pin_order`.

Unsupported components in a generated deck are critical validation-input
failures. They must not be silently omitted.

## Project Contract

An `analog_transient`, `analog_ac`, or `analog_dc` scenario can use either a
hand-authored deck or generated Board IR source:

```yaml
analog:
  backend: auto
  netlist_source: generated_from_board
  generated:
    components: [VDTR, VRTS, R1, R26, R27, R8, D13, Q2, Q3, CBOOT, CNRST]
    ground_net: gnd
  model_files:
    - path: ../../models/spice/onsemi/ss8050_ss8550.lib
      sha256: ...
  node_bindings:
    - node: "0"
      net: gnd
    - node: nrst
      net: nrst
```

`netlist_source` defaults to `file` for compatibility with existing projects.
For `file`, `netlist` remains required and points to a SPICE-compatible deck.
For `generated_from_board`, `generated.components` is required and every listed
component must resolve through Board IR and component models.

Board components may include a `spice` object for primitive parameters:

```yaml
R8:
  model: generic.analog.resistor
  pins: {A: nrst, B: vdd_3v3}
  spice: {primitive: resistor, value_ohm: 10000}
```

Current-source primitives use SPICE's positive-current convention from `P` to
`N`. Use `dc_current_source` with `dc_a` for static loads and
`pulse_current_source` with `current_pulse` for pulsed loads or stress
stimuli:

```yaml
ILOAD:
  model: generic.analog.dc_current_source
  pins: {P: rail_3v3, N: gnd}
  spice: {primitive: dc_current_source, dc_a: 0.1}
```

For generated `analog_ac` scenarios, independent voltage and current source
primitives emit a unity small-signal source suffix (`AC 1`) in addition to
their declared DC or pulse operating point. This makes GUI-created Bode
observations executable without requiring users to hand-author source cards.

Capacitors may optionally declare `initial_v`. Generated ngspice wrappers add
`UIC` automatically when any selected generated capacitor has an initial
condition, so precharged storage-capacitor pulse circuits can be represented
without a hand-authored raw netlist.

Discrete semiconductors should derive their SPICE model name/type/path from the
component model's `simulation.spice` metadata. The scenario still declares
`model_files` with SHA-256 pins so a physical result is tied to exact model
artifacts. GUI generated run-setup creation and generated component inclusion
infer missing `model_files` entries from active component-library
`simulation.spice.model_path` metadata and write portable relative paths plus
SHA-256 pins when the files can be resolved from the project directory or an
ancestor. Hand-authored YAML and CLI validation still fail closed if required
model files are missing or unpinned.

Generated analog scenarios may also declare `operating_conditions`. An ambient
temperature enables datasheet power derating when the model provides linear
derating metadata. `allow_pulse_ratings` only permits pulse-current waivers
when the pulse rating declares both pulse width and duty cycle.

The first qualified pulse-current example is
`examples/good_mosfet_qualified_pulse_current`, which uses onsemi FDMC86184
metadata. Its companion `examples/bad_mosfet_pulse_duty` proves that current
below the pulsed-current scalar still fails when pulse width or duty exceeds
the encoded datasheet limits.

Digitized MOSFET SOA curve checks are documented in
`docs/soa_operating_limits.md`; `examples/bad_mosfet_soa_violation` exercises
paired `VDS`/`ID` envelope checking against hand-digitized screening points.

## Generation Rules

1. Map Board IR nets to SPICE nodes using `node_bindings`.
2. Map the declared `ground_net` to node `0`; reject missing or conflicting
   ground bindings.
3. Emit exactly the components listed in `generated.components`, in that order.
4. Reject unknown components, unknown pins, missing pin nets, and nets without
   node bindings.
5. Reject unsupported primitives and missing required primitive parameters.
6. Include declared model files with absolute paths in the generated deck.
7. Emit MOSFETs as SPICE `M` devices with required `D`, `G`, and `S` pins.
   If a body `B` pin is declared on the board component, bind it explicitly.
   If no `B` pin is declared, tie body to source only when the component model
   declares `simulation.spice.body_pin_policy: tie_to_source_when_absent`;
   otherwise fail before solver execution.
8. Emit subcircuits as SPICE `X` devices only when the component model declares
   `simulation.spice.pin_order`; a `.subckt` without deterministic pin mapping
   is a validation-input failure. Subcircuit models may also declare
   `simulation.spice.instance_parameters` to map numeric Board IR component
   parameters into SPICE instance assignments such as `ICHG_A=2`; a mapping may
   declare `default_value` for observation defaults that are still visible in
   component-model metadata.
9. Require every generated semiconductor or subcircuit model file to appear in
   `analog.model_files` with a SHA-256 pin.
10. Resolve model metadata paths from the Board IR project directory and its
    ancestors so CLI launch location does not change the physical model.
11. Prepare generated source decks before solver backend selection so Board IR,
    body-pin, subcircuit pin-order, and model-provenance contract errors are
    visible even on hosts without `ngspice` or `Xyce` installed.
12. Emit generated deck, wrapper, solver log, and waveform as report artifacts.
13. Keep all solver execution, convergence checks, waveform parsing, and
   assertion evaluation in the existing ngspice runner path.
14. Evaluate generated semiconductor operating limits with any declared
   scenario `operating_conditions`; fail closed when temperature or pulse
   metadata is incomplete.

## Review Notes

- Schema compatibility: `netlist_source` must be additive and default to `file`.
  Existing projects that declare `netlist` continue to work.
- Schema enforcement: file-backed scenarios require `netlist`; generated
  scenarios require `generated`. Runtime validation repeats this and fails
  closed so malformed projects cannot reach the solver as partial decks.
- Rust model access: component-library loading must deserialize
  `simulation.spice`; the generator must not reparse model YAML or hardcode
  semiconductor model names.
- Board topology: generated physical decks require explicit Board IR components
  and per-instance values for passives, sources, and device pins. Missing R/C/D
  or stimulus components are validation failures, not inferred shortcuts.
- Evidence quality: generated netlists are artifacts, not temporary invisible
  implementation details. A report must be reproducible from the emitted deck
  and model files.
- Model provenance: generation must not pass if a semiconductor component lacks
  `simulation.spice` metadata or a declared model file hash fails.
- Physical honesty: if a component model is low confidence or estimated, the
  existing limitation mechanism remains visible in the report.

## Contract Fixtures

- `examples/good_mosfet_low_side_switch` proves generated N-channel MOSFET `M`
  device emission with a SHA-pinned datasheet-fit NDS7002A model.
- `examples/good_csd17484f4_low_side_switch` proves generated N-channel MOSFET
  `M` device emission with the SHA-pinned TI CSD17484F4 datasheet-fit model
  under a TOF-style `21.8 V`, `30 ns`, `30 kHz` trigger condition.
- `examples/good_csd17484f4_vcsel_capacitor_discharge` proves generated
  capacitor `IC=` emission and `tran ... uic` execution for a precharged
  C27-style VCSEL pulse-discharge path through the same Q2 model.
- `examples/good_pmos_high_side_switch` proves generated P-channel MOSFET `M`
  device emission with a SHA-pinned datasheet-fit BSS84 model.
- `examples/good_subckt_rc_delay` proves generated subcircuit `X` device
  emission from explicit `simulation.spice.pin_order` metadata.
- `examples/good_bq25798_nvdc_observation` proves generated subcircuit `X`
  device emission can append model-declared instance parameters from Board IR
  component parameters for a reduced BQ25798 NVDC charger observation.
- `examples/good_ideal_opamp_buffer` proves reusable generic behavioral
  macro-model packs can drive generated Board IR decks through the same
  subcircuit, model-file, and SHA-pinned artifact path used by vendor models.
  The same fixture is registered as a direct-open GUI scope example with routed
  schematic metadata for op-amp buffer observation.
- `examples/good_wch_ch340c_usb_uart_observation` proves the WCH CH340C
  datasheet-backed USB-UART bridge model can use a reduced generated-SPICE face
  for VCC, TXD, DTR#, and RTS# output-state observations. The model-state
  inputs are explicit Board IR component parameters, not inferred USB protocol
  behavior.
- `examples/good_silabs_cp2102n_usb_uart_observation` proves the Silicon Labs
  CP2102N datasheet-backed USB-UART bridge model can use a reduced
  generated-SPICE face for VREGIN, generated VDD/VIO, TXD, RTS, and DTR
  output-state observations. The model-state inputs are explicit Board IR
  component parameters, not inferred USB protocol behavior.
- `examples/good_ftdi_ft232r_usb_uart_observation` proves the FTDI FT232R
  source-backed USB-UART bridge model can use a reduced generated-SPICE face
  for VCC, generated 3V3OUT/VCCIO, TXD, RTS#, and DTR# output-state
  observations. The model-state inputs are explicit Board IR component
  parameters, not inferred USB protocol or EEPROM/CBUS configuration behavior.
- `examples/good_wch_ch347_usb_jtag_observation` proves the WCH CH347
  source-backed USB-JTAG/debug bridge model can use a reduced generated-SPICE
  face for VCC, UART1 TXD, and JTAG TMS/TCK/TDI/TRST line-state observations.
  The model-state inputs are explicit Board IR component parameters, not
  inferred USB enumeration, JTAG TAP state, or driver-mode behavior.
- `examples/good_cmsis_dap_swd_probe_observation` proves the generic
  CMSIS-DAP SWD probe model can use a reduced generated-SPICE face for
  VTREF-referenced SWCLK, SWDIO, nRESET, and SWO line-state observations. The
  model-state inputs are explicit Board IR component parameters, not inferred
  USB transport, SWD protocol transfer, or probe-vendor electrical behavior.
- `examples/good_ti_txs0108e_level_shifter_observation` proves the TI TXS0108E
  datasheet-backed level-shifter model can use a reduced generated-SPICE face
  for an enabled A1-to-B1 mixed-voltage observation with rail, OE, input, and
  translated-output checks.
- `examples/good_tpd2eusb30_usb_esd_observation` proves the TI TPD2EUSB30
  datasheet-backed USB ESD model can use a reduced generated-SPICE face for
  normal-operation D+/D- standoff checks with the source-backed 0.7 pF
  line-capacitance load.
- `examples/good_nexperia_prtr5v0u2x_usb_esd_observation` proves the Nexperia
  PRTR5V0U2X datasheet-backed rail-to-rail USB ESD model can use a reduced
  generated-SPICE face for normal-operation VBUS, IO1, and IO2 standoff checks
  with source-backed IO/VCC capacitance loads.
- `examples/good_ti_esd2can24_q1_can_esd_observation` proves the TI
  ESD2CAN24-Q1 datasheet-backed CAN ESD model can use a reduced generated-SPICE
  face for normal-operation CANH/CANL standoff checks with the source-backed
  3 pF line-capacitance load.
- `examples/good_ti_esds552_rs485_esd_observation` proves the TI ESDS552
  datasheet-backed RS-485/RS-422 ESD/surge model can use a reduced
  generated-SPICE face for normal-operation A/B standoff checks with the
  source-backed 11 pF maximum line-capacitance load.
- `examples/good_ti_thvd1450_rs485_transceiver_observation` proves the TI
  THVD1450 datasheet-backed RS-485 transceiver model can use a reduced
  generated-SPICE face for VCC, DI, DE, RE_N, RO, and A/B line-state checks.
  The model-state inputs are explicit Board IR component parameters, not
  inferred RS-485 protocol, termination, or cable behavior.
- `examples/good_tps54331_5v_buck_observation` proves the TI TPS54331
  datasheet-backed buck-regulator model can use a reduced generated-SPICE face
  in a direct-open GUI example with routed schematic metadata, VIN/EN/VSENSE
  voltage probes, load-current probes, and executable preliminary rail checks.
- `examples/good_tps62162_3v3_buck_observation` proves the TI TPS62162
  datasheet-backed fixed 3.3 V buck-regulator model can use the same reduced
  generated-SPICE pattern with VIN/EN/VOS probes, load-current probes, and
  executable preliminary rail checks.
- `examples/good_tps63802_3v3_buck_boost_observation` proves the TI TPS63802
  datasheet-backed 3.3 V buck-boost model can use a reduced generated-SPICE
  face with VIN/EN/VOUT probes, load-current probes, and executable preliminary
  rail checks.
- `examples/good_tps61023_5v_boost_observation` proves the TI TPS61023
  datasheet-backed 5 V boost model can use a reduced generated-SPICE face with
  VIN/EN/VOUT probes, load-current probes, and executable preliminary rail
  checks.
- `examples/good_tps2121_power_mux_observation` proves the TI TPS2121
  datasheet-backed power-mux model can use a reduced generated-SPICE face with
  IN1/IN2/OUT probes, load-current probes, and executable preliminary
  selected-source rail checks.
- `examples/comparator_threshold_scope` proves the generic comparator
  macro-model in a direct-open GUI example with routed schematic metadata,
  named scope probes, and executable threshold/output-state waveform checks.
- `examples/good_tps22918_load_switch_observation` proves the TI TPS22918
  datasheet-backed load-switch model can use a reduced generic generated-SPICE
  face in a direct-open GUI example with routed schematic metadata, switched
  rail voltage probes, branch-current probes, and executable load-path checks.
- `examples/good_mcp73831_charger_observation` proves the Microchip MCP73831-2
  datasheet-backed charger model can use a reduced generic generated-SPICE face
  in a direct-open GUI example with routed schematic metadata, PROG resistor,
  battery-node voltage probes, charge-current probes, and executable charger
  checks.
- `examples/good_bq24075_power_path_observation` proves the TI BQ24075
  datasheet-backed power-path charger model can use a reduced generic
  generated-SPICE face in a direct-open GUI example with routed schematic
  metadata, ISET resistor, OUT/BAT voltage probes, charge-current probes, and
  executable power-path charger checks.
- `examples/good_bq25798_nvdc_observation` proves the TI BQ25798
  datasheet-backed buck-boost/NVDC charger model can map Board IR component
  parameters into a reduced generated-SPICE face in a direct-open GUI example
  with routed schematic metadata, SYS/BAT voltage probes, charge-current
  probes, and executable preliminary charger observation checks.
- `examples/loop_stability_bode_scope` proves file-backed AC/Bode loop-gain
  observation in a direct-open GUI example with routed schematic metadata,
  Bode artifact export, and executable phase/gain margin checks.
- `examples/bad_mosfet_missing_body_policy` proves a three-pin MOSFET fails
  closed when the model does not explicitly allow body-to-source tying.
- `examples/bad_mosfet_model_missing_sha` proves generated device models must
  be SHA-pinned in `analog.model_files`.
- `examples/bad_mosfet_missing_operating_ratings` proves generated MOSFET/BJT
  semiconductor models must carry usable absolute-maximum ratings before their
  simulations can be accepted as physical evidence.
- `examples/bad_subckt_wrong_pin_order` proves wrong subcircuit pin ordering can
  be detected by quantitative waveform assertions.
- `examples/bad_mosfet_overcurrent` proves generated MOSFET drain current and
  power can be checked automatically against datasheet absolute maximum ratings
  without a hand-authored current-limit assertion.
- `examples/bad_pmos_overcurrent` proves signed negative P-channel datasheet
  current ratings are preserved in the report while evaluated by absolute
  magnitude.
- `examples/bad_bjt_overcurrent` proves generated BJT collector current can be
  checked automatically against datasheet absolute maximum ratings without a
  hand-authored transistor-limit assertion.
- `examples/bad_diode_overcurrent` and `examples/bad_diode_reverse_voltage`
  prove generated diode forward-current, reverse-voltage, and power stress can
  be checked automatically against datasheet absolute maximum ratings.

## Datasheet Operating Limits

For generated Board IR decks, CircuitCI augments the ngspice waveform export
with automatic probes derived from component-model
`datasheet.absolute_maximum_ratings`:

- MOSFET `VDSS`, `VGSS`/`VGSS_continuous`, `ID`/`ID_continuous`, and `PD`.
- BJT `VCEO`, `VCBO`, `VEBO`, `IC`, and `PD`.
- Diode `VRRM`/`VR`, `IF`/`IF_AV`, and `PD`/`Ptot`.

Generated MOSFET/BJT/diode models fail closed if these rating groups are absent
or use the wrong unit, because a missing datasheet limit is not pass evidence.
The operating-limit probes are evaluated over the full transient using maximum
stress magnitude. Exceeding a rating emits `SPICE_OPERATING_LIMIT` with the
component id, datasheet rating key, expression, measured maximum, time of
maximum, unit, signed datasheet rating value, and absolute comparison limit.
These checks are supplemental to scenario assertions: a circuit can pass its
functional voltage/current assertions and still fail because the selected part
is overstressed.
