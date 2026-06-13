# Excellon/NC Drill Importer

`circuitci import-excellon-drill` enriches an existing Board IR project with
fabrication drill-hit evidence from an Excellon/NC drill file.

```bash
circuitci import-excellon-drill fabrication/Drill_PTH_Through.DRL \
  --project out/imported_with_outline.project.yaml \
  --output out/imported_with_pth_drills.project.yaml
```

Run the command again with the previous output as `--project` to append another
drill file such as `Drill_PTH_Through_Via.DRL` or `Drill_NPTH_Through.DRL`.

## Imported Evidence

- `board.layout.drills[]`
- drill center `at.x_mm` / `at.y_mm`,
- `drill_mm`,
- `plating`: `plated`, `non_plated`, or `unknown`,
- optional `owner_kind`, `net`, `component`, `pin`, and `via_index` when
  existing Board IR layout evidence gives exactly one pad or via owner; via
  drill layers can also inherit `owner_kind: via` plus `net` from unique
  co-located Gerber copper flash evidence,
- source drill layer comment,
- selected tool such as `T01`,
- source hit index within the imported drill file.

## Supported Drill Subset

The first implementation intentionally supports the narrow, observed
JLC/EasyEDA Pro drill subset:

- `M48` header,
- `METRIC,LZ,...` units/format declaration,
- tool definitions such as `T01C0.30500`,
- `G90` absolute coordinates,
- selected-tool hits such as `X29.3Y-8.64001`,
- plating comments `;TYPE=PLATED` and `;TYPE=NON_PLATED`,
- layer comments such as `;Layer: PTH_Through`.

It fails closed for inches, incremental coordinates, undefined tools, missing
tool selections, missing units, non-positive tool diameters, and unsupported
commands.

## Limits

Drill import is fabrication evidence only. It can annotate drill hits with
existing pad or via ownership when the input Board IR already contains a unique
matching drilled pad or route via at the same center and diameter. For
via-labeled drill layers, it can also conservatively annotate a drill as a via
when previously imported Gerber copper flash evidence at the same coordinate has
exactly one net owner. It does not infer pad copper, annular rings, plated
barrel connectivity, slots, routed cutouts, or electrical scenarios from drill
files alone. It is intended to be
combined with schematic import, PCB import, JLC/EasyEDA assembly import, Gerber
outline import, or explicit Board IR before electrical validation.

The importer appends drill hits as evidence and does not deduplicate overlapping
fabrication outputs. Some JLC/EasyEDA releases include both aggregate PTH drill
files and via-only PTH drill files; import the file set that matches the
fabricator's intended drill package semantics.

## Related Validation

`DRILL_TO_BOARD_EDGE_CLEARANCE_VALID` can consume imported `board.layout.drills`
plus `board.layout.outline.segments` to screen each drill edge against the
nearest external, cutout, or unknown board-edge segment.

`DRILL_ANNULAR_RING_VALID` reports optional drill ownership fields when present,
which makes undersized annular-ring findings traceable to the affected pad or
via instead of only a raw drill hit. When imported Gerber copper flashes also
carry ownership, the rule rejects conflicting drill/copper owner nets; scenarios
can also require matching annular-ring flash evidence on listed copper layers.
