# KiCad Repeated Sheet Instances

Native KiCad hierarchy import supports repeated one-level child sheets without
guessing schematic intent.

The importer preserves component references when they are globally unique after
flattening. When a child reference collides with a root component or another
child instance, every component in that child sheet instance is renamed with the
sanitized sheet name:

```text
sheet "Filter A", child ref "R1" -> filter_a__R1
sheet "Filter B", child ref "R1" -> filter_b__R1
```

The same rewrite is applied to all nodes in the child netlist before merging,
so Board IR component pin bindings remain consistent. Existing explicit KiCad
mapping files should refer to the generated flattened component IDs when they
need per-instance overrides.

Repeated sheet-pin names are allowed because aliases are keyed by `(sheet,
pin)`. Root connectivity still controls whether those interfaces are shared or
separate:

- if two sheet pins share one root net, that root net must have an explicit
  canonical label,
- if two disconnected root groups would resolve to the same non-ground alias,
  import fails closed,
- ground aliases such as `GND` continue to collapse to ground.

Nested sheets and buses remain unsupported by this importer slice.
