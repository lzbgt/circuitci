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
  `stitch via distance` are accepted. Repeated `controlled_impedance_net` and
  `controlled_impedance_pair` rows are supported for reviewed impedance target
  evidence. Repeated `thermal_copper` rows are supported for reviewed thermal
  layout policy, and repeated
  `thermal_measurement` rows are supported for reviewed measured-temperature
  evidence. Repeated `thermal_environment` rows are supported for reviewed
  operating environment evidence. Repeated `stackup_layer` rows are supported
  for reviewed stackup layer evidence. Repeated `rf_antenna_keepout` and
  `rf_antenna_feed_path` rows are supported for reviewed RF antenna layout
  constraints.
- `value`: required for supported fields.
- `unit`: optional. Length fields must be `mm` when a unit is supplied. Ratio
  fields may be unitless fractions or `%`, which is normalized to a fraction.
  `controlled_impedance_net` and `controlled_impedance_pair` rows use
  `ohm`/`ohms` for the target impedance value.
  `thermal_copper` rows use `mm2`/square millimeters for minimum copper area.
  `thermal_measurement` rows use `C`/`celsius` for measured temperature.
  `thermal_package` rows use `C/W` for junction-to-ambient package thermal
  resistance.
  `thermal_environment` rows use `C`/`celsius` for ambient temperature.
  `stackup_layer` rows use `value` as the layer kind and ignore `unit`.
  `rf_antenna_keepout` and `rf_antenna_feed_path` rows use `mm` for the
  distance value when a unit is supplied.
- `source`: optional row-level provenance kept in the manifest.
- `notes`: optional row-level provenance kept in the manifest.

`controlled_impedance_net` rows use `value` as `target_impedance_ohm` and
require extra columns:

- `net`: Board IR net name.
- `expected_width_mm`: reviewed route width target.
- `max_width_error_mm`: reviewed non-negative width tolerance.

`controlled_impedance_pair` rows use `value` as
`target_differential_impedance_ohm` and require extra columns:

- `first_net`: first Board IR net name.
- `second_net`: second Board IR net name. It must be distinct from
  `first_net`.
- `expected_width_mm`: reviewed route width target for each member.
- `expected_gap_mm`: reviewed pair gap target.
- `max_width_error_mm`: reviewed non-negative width tolerance.
- `max_gap_error_mm`: reviewed non-negative gap tolerance.

The importer replaces existing controlled-impedance net targets by `net`, and
existing differential-pair targets by unordered net pair, so repeated imports
stay deterministic. Duplicate CSV targets fail closed. These rows are reviewed
target evidence only; the importer does not calculate impedance, derive targets
from stackup, or infer high-speed nets.

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

`thermal_package` rows use `value` as
`thermal_resistance_junction_to_ambient_C_per_W` and require extra columns:

- `component`: Board IR component reference.
- `max_junction_temperature_C`: reviewed package junction limit.

`source` or `package_source` must name the reviewed package-table source. Rows
create or replace entries under `board.manufacturing.thermal_packages[]` by
`component`. Duplicate CSV components fail closed. These rows are reviewed
package thermal evidence only; the importer does not infer package metadata
from model names, designators, or power-loss text.

`thermal_environment` rows use `value` as `ambient_temperature_C` and require
extra columns:

- `name`: stable reviewed environment identifier.

Optional `thermal_environment` columns:

- `airflow_lfm`: reviewed non-negative airflow.
- `enclosure_profile`: reviewed enclosure/product configuration label.

`source` or `environment_source` must name the reviewed environment source.
Rows create or replace entries under
`board.manufacturing.thermal_environments[]` by `name`. Duplicate CSV names
fail closed. These rows are reviewed operating environment evidence only; the
importer does not infer airflow, enclosure, or acceptable thermal limits.

`stackup_layer` rows use `value` as the layer `kind`. Accepted values are
`signal`, `plane`, `dielectric`, and `other`, with conservative aliases such as
`core`/`prepreg` for dielectric layers. They require extra columns:

- `name`: stable stackup layer name matching imported route, zone, and
  scenario layer references.

Optional `stackup_layer` columns map directly to
`board.layout.stackup.layers[]` fields:

- `reference_net`
- `thickness_mm`
- `copper_thickness_um`
- `dielectric_constant`
- `material`

The importer replaces an existing stackup layer with the same `name` so
repeated imports stay deterministic. Duplicate CSV layer names fail closed.
These rows are reviewed stackup evidence only; the importer does not calculate
impedance, infer copper weights, or infer layer roles from names.

`rf_antenna_keepout` rows use `value` as `min_copper_clearance_mm` and require
extra columns:

- `name`: stable RF keepout identifier.
- `layer`: Board IR copper layer name.
- `polygon`: reviewed keepout polygon as `x:y; x:y; x:y` points in board
  millimeters.

Optional `rf_antenna_keepout` columns:

- `antenna_net`: antenna/feed net whose own copper is excluded from intrusion
  checks.

`rf_antenna_feed_path` rows use `value` as `max_feed_route_length_mm` and
require extra columns:

- `name`: stable RF feed-path identifier.
- `antenna_net`: Board IR antenna/feed net.
- `feed_component`: component reference for the antenna feed start.
- `feed_pin`: pin on `feed_component` connected to `antenna_net`.
- `matching_components`: comma-, semicolon-, or pipe-separated matching-network
  component references.
- `max_matching_component_distance_mm`: reviewed non-negative placement
  distance limit.

RF rows create or replace entries under
`board.layout.constraints.rf_antenna.keepouts[]` and
`board.layout.constraints.rf_antenna.feed_paths[]` by `name`. Duplicate CSV
names fail closed. These rows are reviewed RF layout evidence only; the
importer does not infer antenna topology, RF roles, matching components, or
keepout geometry from net names or designators.

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

This importer updates only explicit reviewed `board.manufacturing` fields,
`board.layout.stackup.layers[]` entries, and reviewed RF antenna layout
constraints while preserving existing design, schematic, Gerber, drill, and
assembly evidence. It does not infer schematic connectivity, component pin
behavior, stackup properties, RF topology, or global JLCPCB defaults.
