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
   is a validation-input failure.
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
- `examples/good_ideal_opamp_buffer` proves reusable generic behavioral
  macro-model packs can drive generated Board IR decks through the same
  subcircuit, model-file, and SHA-pinned artifact path used by vendor models.
  The same fixture is registered as a direct-open GUI scope example with routed
  schematic metadata for op-amp buffer observation.
- `examples/comparator_threshold_scope` proves the generic comparator
  macro-model in a direct-open GUI example with routed schematic metadata,
  named scope probes, and executable threshold/output-state waveform checks.
- `examples/good_tps22918_load_switch_observation` proves the TI TPS22918
  datasheet-backed load-switch model can use a reduced generic generated-SPICE
  face in a direct-open GUI example with routed schematic metadata, switched
  rail voltage probes, branch-current probes, and executable load-path checks.
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
