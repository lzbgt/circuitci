# KiCad Non-Cardinal Rotation SPICE Fixture

## Purpose

`examples/import_kicad_non_cardinal_rotation_spice/` proves non-cardinal
symbol rotation reaches physical validation, not only schematic connectivity
import.

The fixture rotates `R1` by 45 degrees. Its library pin offsets are transformed
onto the importer's one-nanometer grid, then normal horizontal/vertical wires
connect those transformed pin coordinates into an RC charge path:

- `V1` drives `R1`,
- `R1` feeds `net_reset_rc`,
- `C1` returns the RC node to ground.

## Contract

The mapping file generates a Board-IR SPICE transient scenario and asserts:

```yaml
expression: V(net_reset_rc)
relation: above
threshold_v: 2.5
at_us: 2000.0
```

If arbitrary-angle pin transforms regress, the wires no longer attach to `R1`,
the imported Board IR no longer places `R1.A` on `net_3v3` and `R1.B` on
`net_reset_rc`, and generated SPICE either fails to generate or fails the
quantitative assertion.
