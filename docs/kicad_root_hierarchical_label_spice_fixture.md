# KiCad Root Hierarchical Label SPICE Fixture

## Purpose

`examples/import_kicad_root_hier_label_spice/` proves root-sheet
`hierarchical_label` support reaches generated SPICE validation.

The fixture is a simple RC charge path:

- `V1` drives `R1`,
- `R1` feeds the RC node,
- `C1` returns the RC node to ground,
- the RC node is named by a root `hierarchical_label` rather than a local or
  global label.

The generated Board IR must place `R1.B` and `C1.A` on
`net_root_rc`. The mapping then generates a SPICE transient and asserts:

```yaml
expression: V(net_root_rc)
relation: above
threshold_v: 2.5
at_us: 2000.0
```

If root hierarchical labels stop flowing through the normal label pipeline, the
generated scenario cannot probe `net_root_rc` successfully.

## Boundary

This fixture does not make a root hierarchical label behave like a parent-sheet
interface. It only proves that an attached root hierarchical label can name a
root net for Board IR and generated SPICE. Child hierarchical labels still use
the stricter sheet-pin matching contract.
