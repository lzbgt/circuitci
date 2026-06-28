# Manufacturing Metadata Importer

`circuitci import-manufacturing-metadata` applies reviewed board/order
manufacturing metadata from a CSV file to an existing Board IR project and
writes a schema-backed JSON evidence manifest.

Input CSV columns:

- `field`: required. Supported values are Board IR manufacturing keys such as
  `stencil_thickness_mm`, `min_drill_edge_clearance_mm`,
  `min_slot_edge_clearance_mm`, `min_paste_area_ratio`,
  `max_paste_area_ratio`, `min_solder_paste_spacing_mm`, and
  `max_stitch_via_distance_mm`. A few plain-label aliases such as
  `stencil thickness`, `hole to board edge clearance`, and
  `stitch via distance` are accepted.
- `value`: required for supported fields.
- `unit`: optional. Length fields must be `mm` when a unit is supplied. Ratio
  fields may be unitless fractions or `%`, which is normalized to a fraction.
- `source`: optional row-level provenance kept in the manifest.
- `notes`: optional row-level provenance kept in the manifest.

Example:

```bash
circuitci import-manufacturing-metadata \
  --project out/imported_with_drills.project.yaml \
  --metadata order_metadata.csv \
  --output out/imported_with_order_metadata.project.yaml \
  --source jlc_order_metadata
```

By default, unsupported `field` values fail closed. For larger fabrication or
stencil-order exports that contain unrelated rows, pass
`--allow-unknown-fields`. Unsupported rows are then preserved in the manifest as
`skipped_unknown_field` evidence and are not written into Board IR.

The JSON manifest conforms to
`schemas/manufacturing_metadata_import.schema.json` and records:

- source project path, size, and SHA-256,
- metadata CSV path, size, SHA-256, columns, and row count,
- applied field count, skipped row count, and final `board.manufacturing.source`
  label,
- every CSV row with raw columns, normalized Board IR field/value when applied,
  row-level source/notes, and skip reason for unsupported rows.

This importer updates only `board.manufacturing` and preserves existing design,
schematic, layout, Gerber, drill, and assembly evidence. It does not infer
schematic connectivity, component pin behavior, or global JLCPCB defaults.
