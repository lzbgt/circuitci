# KiCad XML Netlist Importer

CircuitCI can ingest KiCad's generic XML netlist export and create a Board IR
connectivity project:

```sh
circuitci import-kicad-netlist board.net --output board.project.yaml
```

This importer is a schematic connectivity bridge, not a full `.kicad_sch`
parser and not physical sign-off. KiCad XML contains component references,
values, fields, and net nodes. It does not by itself prove datasheet-backed
device models, transient stimuli, solver tolerances, or pass/fail assertions.

## Imported Data

The importer reads:

- `components/comp@ref`
- `comp/value`
- `comp/libsource@lib` and `comp/libsource@part`
- `comp/fields/field@name`
- `nets/net@code`, `nets/net@name`
- `net/node@ref`, `net/node@pin`

Board IR output preserves KiCad component references and pin numbers. Pins are
not remapped into package-specific aliases during import; later model-mapping
work can attach exact symbol or datasheet models.

## Model Mapping

Each imported component uses the first available model source:

1. `CircuitCI_Model` field in the KiCad component.
2. The `--default-model` CLI value.
3. `generic.schematic.imported_component`.

The generic imported schematic model is a traceability placeholder with passive
numeric pins. It is useful for connectivity validation and agent repair loops,
but it is intentionally low-confidence and does not provide datasheet operating
limits or analog equations.

## Net Classification

Net kind is inferred conservatively:

- `0`, `gnd`, `ground`, and names containing `gnd` become `ground`.
- all other nets become `digital_or_analog`.

The importer does not infer `power`, nominal voltage, or powered state from
names such as `+3V3`, `VDD`, or `VBUS`. Those semantics require an explicit
user or design-rule mapping before checks that depend on power-domain behavior.

## Scenario Contract

The importer emits no scenarios by default. A scenario must be added later to
run checks such as `SPICE_TRANSIENT_ANALYSIS`, `GPIO_BACKDRIVE`, or boot/reset
rules. This avoids producing fake pass/fail results from connectivity data that
does not include physics models or quantitative assertions.

Generated projects include `project.import_source: kicad_xml_netlist`. Runtime
reports include a `SCHEMATIC_IMPORT_ONLY` limitation for that source, even when
the connectivity project otherwise validates cleanly.

## Fail-Closed Rules

Import fails instead of guessing when it sees:

- duplicate component references,
- duplicate component pin assignments across nets,
- net nodes that reference unknown components,
- missing component refs or node pins,
- XML parse errors,
- a file with no importable components.

## References

- KiCad generic XML netlist and customized BOM/netlist documentation:
  `https://github.com/KiCad/kicad-doc/blob/master/src/eeschema/eeschema_creating_customized_netlists_and_bom_files.adoc`
- KiCad XML netlist exporter source:
  `https://docs.kicad.org/doxygen/netlist__exporter__xml_8cpp_source.html`
