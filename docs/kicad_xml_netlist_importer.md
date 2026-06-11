# KiCad XML Netlist Importer

CircuitCI can ingest KiCad's generic XML netlist export and create a Board IR
connectivity project:

```sh
circuitci import-kicad-netlist board.net --output board.project.yaml
```

With an explicit mapping file:

```sh
circuitci import-kicad-netlist board.net \
  --mapping circuitci.kicad-map.yaml \
  --output board.project.yaml
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

1. `components.<ref>` entry in the mapping file.
2. first matching `libsource_rules` entry in the mapping file.
3. `CircuitCI_Model` or `CircuitCIModel` field in the KiCad component.
4. The `--default-model` CLI value.
5. `generic.schematic.imported_component`.

The generic imported schematic model is a traceability placeholder with passive
numeric pins. It is useful for connectivity validation and agent repair loops,
but it is intentionally low-confidence and does not provide datasheet operating
limits or analog equations.

When a mapping file changes a component away from the default placeholder, every
connected KiCad pin on that component must appear in `pin_map`. This is a
fail-closed rule: CircuitCI will not guess that KiCad pin `1` is model pin `A`,
or that MCU package pin `7` is `NRST`.

Mapping file shape:

```yaml
libraries:
  - ../../libs/generic
components:
  R1:
    model: generic.analog.resistor
    pin_map: { "1": A, "2": B }
  C1:
    model: generic.analog.capacitor
    pin_map: { "1": A, "2": B }
libsource_rules:
  - lib: Device
    part: R
    model: generic.analog.resistor
    pin_map: { "1": A, "2": B }
nets:
  +3V3:
    kind: power
    nominal_voltage: 3.3
    powered: true
  GND:
    kind: ground
```

Mapping files are strictly parsed. Unknown keys, invalid net kinds, unknown
component refs, unknown net names, unconnected source pins, duplicate target
model pins, unresolved model IDs, and target pins not declared by the selected
model all fail import before a project file is written.

## Generated Analog Scenarios

Mapping files may also declare `analog_scenarios`. This is the only KiCad import
path that emits a validation scenario:

```yaml
components:
  V1:
    model: generic.analog.dc_voltage_source
    pin_map: { "1": P, "2": N }
    spice: { primitive: dc_voltage_source, dc_v: 3.3 }
  D1:
    model: vendor.onsemi.1n4148ws
    pin_map: { "1": A, "2": K }
analog_scenarios:
  - name: rc_transient
    components: [V1, R1, D1, C1]
    ground_net: GND
    model_files:
      - path: ../../models/spice/onsemi/1n4148ws.lib
        sha256: dee84e9189e05a9af600a0224a63cb6d01ebec4df27ff4ed12baeddd34869504
    analysis: { type: tran, stop_time_us: 2000.0, max_step_us: 10.0 }
    stimuli:
      - { name: mapped_source, description: V1 is an explicit 3.3 V source. }
    probes:
      - { name: rc, expression: V(net_reset_rc), quantity: voltage }
    assertions:
      - { name: rc_charges, probe: rc, at_us: 2000.0, relation: above, threshold_v: 2.5 }
```

Scenario generation remains fail-closed:

- `components` is explicit; CircuitCI does not auto-include analog-looking parts.
- each generated component must have explicit mapping-file `spice` primitive
  metadata or selected model `simulation.spice` metadata,
- every generated component whose selected model uses `simulation.spice` must
  have a matching SHA-pinned `model_files` entry,
- `operating_conditions`, such as `allow_pulse_ratings`, are copied only when
  explicitly declared in the mapping file,
- `stimuli`, `probes`, and `assertions` must be non-empty,
- node bindings and pin bindings are derived completely from the mapped Board
  IR endpoints,
- the declared `ground_net` must map to a Board IR ground net and is bound to
  SPICE node `0`.

KiCad component values such as `10k` or `100n` are converted into SPICE
primitive values only when the mapping file explicitly requests strict passive
value parsing with `value_ohm_from: schematic_value` or
`value_f_from: schematic_value`. The resolved Board IR still contains numeric
`value_ohm` or `value_f`, not the source selector.

MOSFET pulse/SOA scenarios can explicitly enable qualified pulse ratings:

```yaml
operating_conditions:
  allow_pulse_ratings: true
```

## Net Classification

Net kind is inferred conservatively:

- `0`, `gnd`, `ground`, and names containing `gnd` become `ground`.
- all other nets become `digital_or_analog`.

The importer does not infer `power`, nominal voltage, or powered state from
names such as `+3V3`, `VDD`, or `VBUS`. Those semantics require an explicit
user or design-rule mapping before checks that depend on power-domain behavior.
Mapping-file net entries can set `kind`, `nominal_voltage`, and `powered`.

## Scenario Contract

The importer emits no scenarios by default. A scenario is emitted only when an
explicit mapping-file `analog_scenarios` entry supplies components, analysis,
stimuli, probes, and assertions. This avoids producing fake pass/fail results
from connectivity data that does not include physics models or quantitative
assertions.

Generated projects include `project.import_source: kicad_xml_netlist`. Runtime
reports include a `SCHEMATIC_IMPORT_ONLY` limitation for that source, even when
the connectivity project otherwise validates cleanly.

See also:

- `examples/import_kicad_xml/board.net`
- `examples/import_kicad_xml/circuitci.kicad-map.yaml`
- `examples/import_kicad_mosfet/board.net`
- `examples/import_kicad_mosfet/circuitci.kicad-map.yaml`

## Fail-Closed Rules

Import fails instead of guessing when it sees:

- duplicate component references,
- duplicate component pin assignments across nets,
- net nodes that reference unknown components,
- mapping entries for unknown component refs,
- mapping pin names that do not exist on the imported component,
- mapped components that change model without mapping every connected pin,
- duplicate mapped model pin names on a component,
- generated components whose selected model uses `simulation.spice` but whose
  scenario omits a matching SHA-pinned `model_files` entry,
- model files that are missing, unpinned, or whose SHA-256 does not match,
- missing component refs or node pins,
- XML parse errors,
- a file with no importable components.

## References

- KiCad generic XML netlist and customized BOM/netlist documentation:
  `https://github.com/KiCad/kicad-doc/blob/master/src/eeschema/eeschema_creating_customized_netlists_and_bom_files.adoc`
- KiCad XML netlist exporter source:
  `https://docs.kicad.org/doxygen/netlist__exporter__xml_8cpp_source.html`
