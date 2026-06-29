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
  operating environment evidence. Repeated `thermal_limit` rows are supported
  for reviewed measured/package temperature limits. Repeated `stackup_layer`
  rows are supported for reviewed stackup layer evidence. Repeated
  `rf_antenna_keepout`, `rf_antenna_feed_path`,
  `rf_antenna_matching_network`, `rf_antenna_measurement`, and
  `rf_antenna_performance_limit` rows are supported for reviewed RF antenna
  layout/topology, measured return-loss evidence, and reviewed RF performance
  limits.
- `value`: required for supported fields.
- `unit`: optional. Length fields must be `mm` when a unit is supplied. Ratio
  fields may be unitless fractions or `%`, which is normalized to a fraction.
  `controlled_impedance_net` and `controlled_impedance_pair` rows use
  `ohm`/`ohms` for the target impedance value.
  `controlled_impedance_coupon` rows use `ohm`/`ohms` for the measured coupon
  impedance value.
  `thermal_copper` rows use `mm2`/square millimeters for minimum copper area.
  `thermal_measurement` rows use `C`/`celsius` for measured temperature.
  `thermal_package` rows use `C/W` for junction-to-ambient package thermal
  resistance.
  `thermal_environment` rows use `C`/`celsius` for ambient temperature.
  `thermal_limit` rows use `C`/`celsius` for max measured temperature.
  `stackup_layer` rows use `value` as the layer kind and ignore `unit`.
  `rf_antenna_keepout` and `rf_antenna_feed_path` rows use `mm` for the
  distance value when a unit is supplied.
  `rf_antenna_matching_network` rows use `value` as the reviewed topology and
  ignore `unit`.
  `rf_antenna_measurement` and `rf_antenna_performance_limit` rows use
  `dB`/`decibel` for measured or required return-loss magnitude.
- `source`: optional row-level provenance kept in the manifest.
- `notes`: optional row-level provenance kept in the manifest.

`controlled_impedance_net` rows use `value` as `target_impedance_ohm` and
require extra columns:

- `net`: Board IR net name.
- `expected_width_mm`: reviewed route width target.
- `max_width_error_mm`: reviewed non-negative width tolerance.
- Optional `solder_mask_state`: reviewed `covered` or `opened` route loading
  target. Aliases `masked`, `maskcovered`, `open`, and `exposed` are
  normalized.
- Optional `solder_mask_layer`: imported Board IR solder-mask layer name such
  as `F.Mask`.
- Optional `solder_mask_source`: reviewed source for the solder-mask loading
  target. When omitted, scenario suggestions can fall back to the row `source`.

`controlled_impedance_pair` rows use `value` as
`target_differential_impedance_ohm` and require extra columns:

- `first_net`: first Board IR net name.
- `second_net`: second Board IR net name. It must be distinct from
  `first_net`.
- `expected_width_mm`: reviewed route width target for each member.
- `expected_gap_mm`: reviewed pair gap target.
- `max_width_error_mm`: reviewed non-negative width tolerance.
- `max_gap_error_mm`: reviewed non-negative gap tolerance.
- Optional `solder_mask_state`: reviewed `covered` or `opened` route loading
  target for both pair members.
- Optional `solder_mask_layer`: imported Board IR solder-mask layer name such
  as `F.Mask`.
- Optional `solder_mask_source`: reviewed source for the solder-mask loading
  target. When omitted, scenario suggestions can fall back to the row `source`.

The importer replaces existing controlled-impedance net targets by `net`, and
existing differential-pair targets by unordered net pair, so repeated imports
stay deterministic. Duplicate CSV targets fail closed. These rows are reviewed
target evidence only; the importer does not calculate impedance, derive targets
from stackup, or infer high-speed nets.

`controlled_impedance_coupon` rows use `value` as
`measured_impedance_ohm` and require extra columns:

- `name`: stable coupon identifier. Existing coupon rows with the same name
  are replaced; duplicate CSV names fail closed.
- `coupon_type`: `single_ended` or `differential`.
- `target_impedance_ohm`: reviewed coupon target impedance.
- `max_impedance_error_ohm`: reviewed non-negative coupon tolerance.
- Optional `min_batch_sample_count`,
  `max_batch_mean_impedance_error_ohm`,
  `max_batch_sample_impedance_error_ohm`, and `max_batch_stddev_ohm` columns
  declare reviewed coupon batch acceptance limits.
- Optional `process_lot`, `panel_id`, `stackup_revision`,
  `coupon_trace_layer`, `coupon_trace_width_mm`, and
  `max_trace_width_delta_mm` columns declare reviewed coupon-to-board trace
  correlation evidence. Differential coupons may also declare
  `coupon_trace_gap_mm` and `max_trace_gap_delta_mm`.
- For `single_ended`, `net` is required and `first_net`/`second_net` must be
  blank.
- For `differential`, `first_net` and `second_net` are required and `net` must
  be blank.

`controlled_impedance_coupon_sample` rows use `value` as a sampled
`measured_impedance_ohm` and require `coupon_name`, `name`, and `source`.
Rows attach under the named imported or pre-existing coupon; duplicate
`coupon_name`/sample-name pairs fail closed.

`controlled_impedance_solver_result` rows use `value` as
`solved_impedance_ohm` and require extra columns:

- `name`: stable reviewed solver-result identifier. Existing solver-result
  rows with the same name are replaced; duplicate CSV names fail closed.
- `result_type`: `single_ended` or `differential`.
- `target_impedance_ohm`: reviewed board target impedance used by the solver
  setup.
- `max_impedance_error_ohm`: reviewed non-negative solver-result tolerance.
- `solver`: reviewed solver/tool name.
- `solver_artifact_uri`: reviewed URI or repository-relative path for the
  solver output package/report used as evidence.
- `solver_artifact_sha256`: 64-character SHA-256 digest for the solver output
  artifact referenced by `solver_artifact_uri`.
- `stackup_revision`, `route_layer`, `reference_layer`, and
  `dielectric_layer`: reviewed solver setup and stackup references.
- `solved_width_mm` and `max_route_width_delta_mm`: reviewed modeled trace
  width and allowed imported-route width delta.
- For `single_ended`, `net` is required and `first_net`/`second_net` must be
  blank.
- For `differential`, `first_net`, `second_net`, `solved_gap_mm`, and
  `max_route_gap_delta_mm` are required and `net` must be blank.

Optional `controlled_impedance_solver_result` columns are `solver_version`,
`frequency_mhz`, and `solver_source` when the ordinary `source` column is not
used. These rows are reviewed solver-result evidence only; the importer
preserves artifact provenance but does not run a field solver, fetch artifacts,
or infer stackup parameters.

Coupon rows are reviewed measurement evidence only; the importer does not
decide whether a coupon statistically represents the routed board. Validation
requires each imported coupon to map to exactly one reviewed
`controlled_impedance_net` or `controlled_impedance_pair` target with matching
target impedance before the coupon tolerance, batch statistics, or trace
correlation evidence are evaluated.

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

`thermal_limit` rows use `value` as `max_measured_temperature_C` and require
extra columns:

- `name`: stable reviewed limit identifier.

Optional `thermal_limit` columns:

- `component`: component reference the limit applies to. When omitted, the
  limit is treated as a reviewed board/global limit by scenario suggestion.
- `max_temperature_rise_C`: positive reviewed package or measurement rise
  limit.
- `max_junction_temperature_margin_C`: reviewed non-negative margin below the
  source-backed package max-junction temperature.

`source` or `limit_source` must name the reviewed limit source. Rows create or
replace entries under `board.manufacturing.thermal_limits[]` by `name`.
Duplicate CSV names fail closed. These rows are reviewed limit evidence for
scenario suggestion only; validators still consume explicit scenario
parameters and the importer does not infer acceptable temperatures from
measurements, package ratings, environments, or component names.

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

`rf_antenna_matching_network` rows use `value` as reviewed topology
(`series`, `l`, `pi`, `t`, or `custom`) and require extra columns:

- `name`: stable RF matching-network identifier.
- `antenna_net`: Board IR antenna/feed net.
- `elements`: semicolon- or pipe-separated reviewed topology elements. Series
  elements use `series:COMPONENT:INPUT_NET:OUTPUT_NET`; shunt elements use
  `shunt:COMPONENT:SIGNAL_NET` or
  `shunt:COMPONENT:SIGNAL_NET:REFERENCE_NET`.

Optional `rf_antenna_matching_network` columns:

- `reference_net`: default reference net for shunt elements without an
  element-local reference net.
- `matching_source`: reviewed RF topology source when the ordinary `source`
  column is not used.

`rf_antenna_measurement` rows use `value` as positive `return_loss_db` and
require extra columns:

- `name`: stable RF measurement identifier.
- `antenna_net`: Board IR antenna/feed net.
- `frequency_mhz`: positive measurement frequency in MHz.

Optional `rf_antenna_measurement` columns:

- `measurement_method`: reviewed measurement method such as `vna_s11`.
- `measurement_condition`: reviewed condition name from
  `board.layout.constraints.rf_antenna.measurement_conditions[]`.

`rf_antenna_performance_limit` rows use `value` as positive
`min_return_loss_db` and require extra columns:

- `name`: stable RF performance-limit identifier.
- `antenna_net`: Board IR antenna/feed net.

Optional `rf_antenna_performance_limit` columns:

- `frequency_min_mhz`: positive lower edge of the reviewed operating band.
- `frequency_max_mhz`: positive upper edge of the reviewed operating band.
- `min_measurement_count`: positive minimum number of unique in-band
  measurement frequencies required when treating the selected rows as a
  reviewed S-parameter sweep.
- `max_frequency_step_mhz`: positive maximum allowed frequency gap between
  adjacent selected sweep points, including reviewed frequency-band edges when
  present.
- `required_measurement_condition`: reviewed condition name that selected
  measurements must explicitly reference.
- `limit_source`: reviewed limit source when the ordinary `source` column is
  not used.

`rf_antenna_measurement_condition` rows create reviewed RF measurement
condition metadata. They require `name` and `source` or `condition_source`.
Optional columns are `fixture`, `cable_setup`, `enclosure_profile`, and
`notes`. Validation and suggestions treat a condition as usable evidence only
when at least one of `fixture`, `cable_setup`, or `enclosure_profile` is
present.

RF rows create or replace entries under
`board.layout.constraints.rf_antenna.keepouts[]`,
`board.layout.constraints.rf_antenna.feed_paths[]`, and
`board.layout.constraints.rf_antenna.matching_networks[]`, and
`board.layout.constraints.rf_antenna.measurements[]`, and
`board.layout.constraints.rf_antenna.performance_limits[]`, and
`board.layout.constraints.rf_antenna.measurement_conditions[]` by `name`.
Duplicate CSV names fail closed. These rows are reviewed RF
layout/topology/measurement/limit evidence only; the importer does not infer
antenna topology, RF roles, matching components, keepout geometry, acceptable
return loss, measurement conditions, or S-parameter sweep behavior from net
names, component values, or designators.

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
`board.layout.stackup.layers[]` entries, and reviewed RF antenna layout and
measurement constraints while preserving existing design, schematic, Gerber,
drill, and assembly evidence. It does not infer schematic connectivity,
component pin behavior, stackup properties, RF topology, acceptable RF
performance, or global JLCPCB defaults.
