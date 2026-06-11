# KiCad Symbol Instance Metadata

## Source Facts

The saved KiCad schematic reference under
`docs/research/kicad/sexpr-schematic.html` defines:

- symbol-level `(in_bom yes|no)`, which controls bill-of-material output,
- symbol-level `(instances ...)`, grouped by project and sheet path,
- each instance path carrying a `reference` and `unit`.

These tokens are not electrical connectivity, but they are useful provenance for
agents that import a real schematic and later compare Board IR against KiCad
production data.

## CircuitCI Contract

Native `.kicad_sch` import preserves the following component source metadata:

- `source.in_bom`: parsed from `(in_bom yes|no)`, defaulting to `true`,
- `source.unit`: the selected schematic symbol unit, defaulting to `1`,
- `source.instances`: project/path/reference/unit records from KiCad
  `(instances ...)` when present.

The importer validates instance records instead of treating them as authority
over connectivity:

- malformed `in_bom` values fail closed,
- malformed `instances`, `project`, or `path` records fail closed,
- instance `reference` must match the schematic symbol `Reference`,
- instance `unit` must match the schematic symbol unit,
- an `instances` block must contain at least one path record.

`on_board no` still controls physical import. `in_bom no` is preserved as
metadata but does not remove a component from Board IR because a component may
be physically on the board while intentionally omitted from the BOM.
