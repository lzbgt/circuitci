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
  `stitch via distance` are accepted. Repeated `thermal_copper` rows are
  supported for reviewed thermal layout policy, and repeated
  `thermal_measurement` rows are supported for reviewed measured-temperature
  evidence.
- `value`: required for supported fields.
- `unit`: optional. Length fields must be `mm` when a unit is supplied. Ratio
  fields may be unitless fractions or `%`, which is normalized to a fraction.
  `thermal_copper` rows use `mm2`/square millimeters for minimum copper area.
  `thermal_measurement` rows use `C`/`celsius` for measured temperature.
- `source`: optional row-level provenance kept in the manifest.
- `notes`: optional row-level provenance kept in the manifest.

`thermal_copper` rows use `value` as `min_copper_area_mm2` and require extra
columns:

- `name`: stable thermal policy identifier. If an existing
  `board.manufacturing.thermal_copper[]` entry has the same name, the importer
  replaces that entry so repeated imports stay deterministic. Duplicate CSV
  names fail closed.
- `component`: Board IR component reference.
- `power_loss_w`: reviewed positive dissipation assumption.

Optional `thermal_copper` columns map directly to Board IR policy fields:

- `min_thermal_via_count`
- `min_plated_thermal_via_count`
- `min_thermal_via_drill_mm`
- `min_thermal_via_plating_thickness_um`
- `min_total_thermal_via_barrel_cross_section_mm2`
- `min_copper_thickness_um`
- `rated_ambient_temperature_C`
- `min_airflow_lfm`
- `enclosure_profile`
- `nets`
- `layers`

`nets` and `layers` accept comma-, semicolon-, or pipe-separated lists. These
rows are reviewed policy evidence only; the importer does not infer thermal
nets, layers, vias, package limits, or heat-flow behavior.

`thermal_measurement` rows require extra columns:

- `name`: stable measurement identifier.
- `component`: Board IR component reference.
- `ambient_temperature_C`: optional ambient measurement context.
- `measurement_uncertainty_C`: optional reviewed non-negative measurement
  uncertainty in C.
- `power_loss_w`: optional reviewed dissipation context.
- `measurement_point`: optional probe/IR-camera point label.

The importer appends measurement rows to
`board.manufacturing.thermal_measurements[]` and preserves the raw columns in
the manifest. It does not infer pass/fail limits from the measured temperature.

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
