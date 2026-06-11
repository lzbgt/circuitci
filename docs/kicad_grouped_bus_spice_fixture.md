# KiCad Grouped Bus SPICE Fixture

## Purpose

`examples/import_kicad_grouped_bus_spice/` proves grouped KiCad bus aliases
reach physical validation, not only schematic connectivity import.

The fixture models a simple RC charge path:

- `V1` drives `R1`,
- `R1` feeds the RC node,
- `C1` returns the node to ground,
- the RC node is named through a bus entry and a grouped `bus_alias` member.

The schematic declares:

```scheme
(bus_alias "PORTBUS" (members "PORT{RESET_RC SPARE}"))
```

The bus graphic carries scalar label `PORT.RESET_RC`. The wire stub entering
the bus entry is intentionally unlabeled, so the importer must resolve the
scalar member from the grouped alias and place the RC node on
`net_port_reset_rc`.

## Contract

The mapping file generates a normal Board-IR SPICE transient scenario from the
imported schematic. The scenario asserts:

```yaml
expression: V(net_port_reset_rc)
relation: above
threshold_v: 2.5
at_us: 2000.0
```

If grouped alias resolution regresses, `R1.B` and `C1.A` will not land on
`net_port_reset_rc`, and generated SPICE will either fail to generate or fail
the quantitative assertion.

The fixture does not infer any bus-entry net from graphics alone. The attached
bus segment has exactly one resolvable scalar member, so it stays inside the
conservative importer contract.
