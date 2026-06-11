# Board IR to SPICE Generation

CircuitCI must not depend on hand-written fixture decks for every board issue.
The analog backend still delegates nonlinear device physics to mature SPICE
engines such as ngspice, but CircuitCI should be able to generate the SPICE deck
from Board IR and component model metadata.

## Scope

This slice adds generated transient decks for board-local analog subcircuits.
It is not a new simulator and must not implement SPICE numerics in Rust. Rust
only translates audited Board IR into a solver deck, records artifacts, invokes
the mature backend, and evaluates waveform assertions.

Initial primitive coverage is intentionally small in resource usage, not a toy
scope:

- resistor,
- capacitor,
- independent DC voltage source,
- independent pulse voltage source,
- diode backed by `simulation.spice`,
- BJT NPN/PNP backed by `simulation.spice`.

Unsupported components in a generated deck are critical validation-input
failures. They must not be silently omitted.

## Project Contract

An `analog_transient` scenario can use either a hand-authored deck or generated
Board IR source:

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

Discrete semiconductors should derive their SPICE model name/type/path from the
component model's `simulation.spice` metadata. The scenario still declares
`model_files` with SHA-256 pins so a physical result is tied to exact model
artifacts.

## Generation Rules

1. Map Board IR nets to SPICE nodes using `node_bindings`.
2. Map the declared `ground_net` to node `0`; reject missing or conflicting
   ground bindings.
3. Emit exactly the components listed in `generated.components`, in that order.
4. Reject unknown components, unknown pins, missing pin nets, and nets without
   node bindings.
5. Reject unsupported primitives and missing required primitive parameters.
6. Include declared model files with absolute paths in the generated deck.
7. Emit generated deck, wrapper, solver log, and waveform as report artifacts.
8. Keep all solver execution, convergence checks, waveform parsing, and
   assertion evaluation in the existing ngspice runner path.

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
