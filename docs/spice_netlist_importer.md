# SPICE Netlist Importer

CircuitCI can ingest an existing SPICE-compatible deck and create a Board IR
project that runs the same deck through the analog validation pipeline.

## Scope

This importer targets simulator decks exported from schematic tools or written
by hand. Native KiCad and KiCad XML importers now also produce Board IR, while
EasyEDA, Altium, and other schematic importers remain future adapter layers.
The SPICE deck path gives agents a direct way to run physical waveforms from
real design artifacts when a board region already has a solver deck.

Command:

```sh
circuitci import-spice board_region.cir --output imported.project.yaml
```

The generated project:

- declares discovered SPICE elements as Board IR components,
- declares discovered SPICE nodes as Board IR nets,
- binds SPICE nodes back to Board IR nets through `analog.node_bindings`,
- binds element terminals back to Board IR endpoints through
  `analog.pin_bindings`,
- preserves `.include` and `.lib` dependencies as `analog.model_files`,
- computes SHA-256 pins for included model files that resolve locally,
- emits voltage probes for discovered non-ground SPICE nodes,
- creates one file-backed `analog_transient` scenario using the original deck.

## Element Mapping

The importer understands common SPICE element prefixes:

| Prefix | Pins | Board IR model |
| --- | --- | --- |
| `R` | `A`, `B` | `generic.analog.resistor` |
| `C` | `A`, `B` | `generic.analog.capacitor` |
| `V` | `P`, `N` | `generic.analog.imported_spice_device` |
| `D` | `A`, `K` | `generic.analog.imported_spice_device` |
| `Q` | `C`, `B`, `E`, optional `S` | `generic.analog.imported_spice_device` |
| `M` | `D`, `G`, `S`, `B` | `generic.analog.imported_spice_device` |
| `X` | `P1..PN` | `generic.analog.imported_spice_device` |
| other two-terminal sources/passives | `A`, `B` | `generic.analog.imported_spice_device` |

Imported elements keep their simulator behavior in the original deck.
CircuitCI does not invent datasheet-backed device metadata for them during
import. Primitive values are preserved in Board IR where they can be represented
losslessly, but `netlist_source: file` still makes the deck the solver source
of truth.

## File-Backed Scenario Contract

Imported projects use `analog.netlist_source: file`, so the original deck
remains the simulator source of truth. The importer emits an empty
`assertions: []` list by default. This is deliberate: importing proves that the
deck can be represented and simulated, but engineering pass/fail thresholds
must come from a board-specific review or later agent repair task.

When such a scenario solves without assertions, the report remains `pass` if no
critical solver or model issue occurs, but it includes
`ANALOG_ASSERTIONS_ABSENT` as an informational finding. Agents must treat that
as waveform evidence only, not as design sign-off.

File-backed scenarios may also have `model_files: []` when the deck uses only
built-in SPICE primitives such as R, C, and independent sources.

## Fail-Closed Rules

The importer rejects malformed element lines instead of guessing:

- too few tokens for the element prefix,
- malformed `.include` or `.lib` path,
- unsupported control blocks embedded in the source deck,
- orphan continuation lines,
- element names that cannot be represented as Board IR component IDs.

The validation pipeline still owns solver correctness. Importing a deck does
not prove that the deck has accurate vendor models, parasitics, tolerances, or
thermal/SOA metadata.

## Review Notes

- File-backed imported scenarios may have an empty `model_files` list for decks
  containing only built-in SPICE primitives.
- Imported external devices use a generic placeholder library model only to
  keep Board IR binding explicit. The actual device equations come from the
  source SPICE deck and included model files.
- Datasheet operating-limit automation remains available for
  `generated_from_board` scenarios. Imported file-backed decks initially get
  waveform assertions and solver evidence; later slices can map imported
  elements to datasheet-backed library models.
