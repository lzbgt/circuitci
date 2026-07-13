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
- resolves `.include` and `.lib` dependencies relative to the source deck and
  emits SHA-256 pins for every imported model file,
- tolerates closed ngspice `.control` / `.endc` blocks, ignoring simulator
  commands while importing `tran` timing and reviewed raw `.meas`/`meas`
  statements when present,
- emits voltage probes for discovered non-ground SPICE nodes,
- creates a file-backed `analog_dc` scenario when the deck contains `.op`,
- creates one file-backed `analog_transient` scenario using the original deck
  when the deck contains transient timing, or by default when no `.op` was
  requested,
- creates an additional file-backed `analog_measure` scenario when the deck
  contains ngspice `.meas`/`meas` cards. Raw measure statements remain
  ngspice-specific text; explicit Xyce backends fail closed through the normal
  measure-analysis adapter boundary. Transient measure imports use deck
  `.tran` timing when present, otherwise the CLI/default import timing. AC
  measure imports require a valid `.ac dec` sweep.

## Element Mapping

The importer understands common SPICE element prefixes:

| Prefix | Pins | Board IR model |
| --- | --- | --- |
| `R` | `A`, `B` | `generic.analog.resistor` |
| `C` | `A`, `B` | `generic.analog.capacitor` |
| `I` | `P`, `N` | `generic.analog.imported_spice_device` |
| `V` | `P`, `N` | `generic.analog.imported_spice_device` |
| `E` | `P`, `N`, `CP`, `CN` | `generic.analog.imported_spice_device` |
| `G` | `P`, `N`, `CP`, `CN` | `generic.analog.imported_spice_device` |
| `F` | `P`, `N` | `generic.analog.imported_spice_device` |
| `H` | `P`, `N` | `generic.analog.imported_spice_device` |
| `B` | `P`, `N` | `generic.analog.imported_spice_device` |
| `D` | `A`, `K` | `generic.analog.imported_spice_device` |
| `Q` | `C`, `B`, `E`, optional `S` | `generic.analog.imported_spice_device` |
| `M` | `D`, `G`, `S`, `B` | `generic.analog.imported_spice_device` |
| `X` | `P1..PN` | `generic.analog.imported_spice_device` |
| other two-terminal sources/passives | `A`, `B` | `generic.analog.imported_spice_device` |

Imported independent voltage and current sources preserve DC and `PULSE(...)`
primitive metadata in Board IR. Common small-signal source forms such as
`V1 in 0 AC 1` and `V1 in 0 DC 0 AC 1` import successfully; AC magnitude and
phase remain in the original file-backed deck rather than being projected into
Board IR primitive metadata. Common transient waveform forms such as
`SIN(...)`, `SINE(...)`, `PWL(...)`, `EXP(...)`, `SFFM(...)`, and `AM(...)`
also import as file-backed source devices without synthesized primitive
metadata. Imported elements keep their simulator behavior in the original deck.
CircuitCI does not invent datasheet-backed device metadata for them during
import. Primitive values are preserved in Board IR where they can be represented
losslessly, but `netlist_source: file` still makes the deck the solver source of
truth.

Numeric `R`/`C`/`L` values are projected into Board IR primitive metadata.
Parameterized or expression-valued passives such as `R1 in out {RVAL}` remain
file-backed and import without synthesized primitive metadata; `.param` cards
and expression evaluation stay in the original deck.

Dependent and behavioral sources also keep their simulator behavior in the
source deck. Voltage-controlled `E`/`G` sources expose output pins (`P`, `N`) and
control-node pins (`CP`, `CN`) so imported topology remains reviewable.
Current-controlled `F`/`H` sources expose only their output pins because their
control source is a source-name reference, not a circuit node. Behavioral `B`
sources expose their two output pins and are treated as voltage-like only when
the expression begins with `V=`.

The importer derives voltage probes for every non-ground deck node and current
probes for imported independent voltage sources using SPICE branch expressions
such as `I(V1)`, including voltage sources whose waveform stays file-backed
instead of becoming Board IR primitive metadata. Voltage-output dependent
sources (`E`, `H`, and `B` with `V=`) get the same branch probes. Those current
probes support GUI oscilloscope inspection of supply or stimulus source current
without requiring generated-from-Board branch instrumentation.

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

Decks that request `.op` import as `analog_dc` scenarios with `analysis:
{type: op}` and `SPICE_DC_ANALYSIS`. If the same deck also requests transient
timing, the importer emits both operating-point and transient scenarios against
the same source deck.

## Fail-Closed Rules

The importer rejects malformed element lines instead of guessing:

- too few tokens for the element prefix,
- malformed `.include` or `.lib` path,
- `.include` or `.lib` path that does not resolve to a local file,
- unmatched or unclosed `.control` / `.endc` blocks,
- malformed `.meas` cards, duplicate `.meas` result names, mixed `.meas` modes,
  or `.meas ac` cards without a valid `.ac dec` sweep,
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
