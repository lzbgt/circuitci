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
`frequency_mhz`, `solver_artifact_signature_uri`,
`solver_artifact_signature_sha256`, `solver_artifact_signer`,
`solver_input_deck_uri`, `solver_input_deck_sha256`, `input_stackup_revision`,
`input_route_layer`, `input_reference_layer`, `input_dielectric_layer`,
`input_width_mm`, `input_gap_mm`, `input_frequency_mhz`,
`copper_roughness_model`, `copper_roughness_um`,
`input_copper_roughness_model`, `input_copper_roughness_um`,
`etch_compensation_model`, `etch_compensation_um`,
`input_etch_compensation_model`, `input_etch_compensation_um`,
`solver_material_library`, `solver_material_library_revision`,
`solver_material_library_artifact_uri`,
`solver_material_library_artifact_sha256`, `input_material_library`,
`input_material_library_revision`, `stackup_signoff_source`,
`fabricator_stackup_revision`, `stackup_signoff_artifact_uri`,
`stackup_signoff_artifact_sha256`,
`min_solver_sample_count`,
`max_solver_frequency_step_mhz`, `required_solver_corners`, and
`solver_entitlement`, `solver_entitlement_features`,
`solver_execution_environment`, `solver_environment_fingerprint`,
`solver_environment_components`, `solver_run_log`, `solver_run_id`,
`solver_random_seed`, `solver_numeric_tolerance_policy`,
`solver_residual_error`, `solver_iterations`, and
`solver_source` when the ordinary `source` column is not used. When any
solver-artifact signature column is present, validation requires a non-empty
signature URI, 64-character signature SHA-256 digest, and signer name. When any
solver output-schema column is present, validation requires non-empty schema
name, schema version, schema artifact URI, and 64-character schema artifact
SHA-256 digest. When any solver configuration-lock column is present,
validation requires non-empty config-lock artifact URI, 64-character
config-lock SHA-256 digest, tool name, and revision, and checks that the tool
name matches the solver result. When any solver runtime column is present,
validation requires non-empty `solver_runtime_allowlist`,
`solver_runtime_profile`, and `solver_runtime_options`, plus a matching
reviewed `controlled_impedance_solver_runtime_allowlist` row for the solver and
configuration-lock revision. When any solver entitlement column is present, validation requires
non-empty `solver_entitlement`, `solver_version`, and
`solver_entitlement_features`, plus a matching reviewed
`controlled_impedance_solver_entitlement` row for the solver and version.
When any solver execution-environment column is present, validation requires
non-empty `solver_execution_environment`, `solver_version`,
`solver_environment_fingerprint`, and `solver_environment_components`, plus a
matching reviewed `controlled_impedance_solver_execution_environment` row for
the solver, version, and fingerprint.
When any solver run-log column is present, validation requires non-empty
`solver_run_log`, `solver_version`, `solver_run_id`, `solver_random_seed`, and
`solver_numeric_tolerance_policy`, plus non-negative `solver_residual_error`,
positive `solver_iterations`, and a matching reviewed
`controlled_impedance_solver_run_log` row for the solver, version, run id,
seed, and tolerance policy.
If that reviewed run-log row declares `min_rerun_count` and
`max_rerun_impedance_delta_ohm`, validation also requires matching
deterministic rerun samples whose seed, solved impedance, residual error, and
iteration count remain inside the reviewed run-log limits.
If that reviewed run-log row declares `min_convergence_sample_count`,
`max_convergence_impedance_delta_ohm`, and `required_stopping_criteria`,
validation also requires matching convergence samples whose stopping criteria,
solved impedance, residual error, and iteration remain inside the reviewed
run-log limits.
When any input-deck column is present, validation requires complete input-deck
provenance and checks that the reviewed input setup matches the solver-result
setup. Declaring any copper roughness column requires complete reviewed
result/input-deck roughness model and positive roughness values; CircuitCI
compares them for consistency but does not calculate roughness-adjusted
impedance. Declaring any etch compensation column requires complete reviewed
result/input-deck etch compensation model and positive compensation values;
CircuitCI compares them for consistency but does not infer finished trace
geometry. These rows are reviewed solver-result evidence only; the importer
preserves artifact provenance but does not run a field solver, fetch artifacts,
verify signatures, parse solver output schemas, parse tool configuration locks
or runtime allowlist/entitlement/environment/run-log/rerun/convergence
artifacts, parse
input decks, replay solver runs, or infer stackup parameters.
Repeated `field=controlled_impedance_solver_runtime_allowlist` rows require
`name`, `source`, `solver`, `solver_config_lock_revision`, `runtime_profile`,
`allowlist_revision`, `artifact_uri`, `artifact_sha256`, and
`allowed_options`.
Repeated `field=controlled_impedance_solver_entitlement` rows require `name`,
`source`, `solver`, `solver_version`, `entitlement_id`,
`entitlement_revision`, `artifact_uri`, `artifact_sha256`, and
`licensed_features`.
Repeated `field=controlled_impedance_solver_execution_environment` rows require
`name`, `source`, `solver`, `solver_version`, `environment_id`,
`environment_revision`, `artifact_uri`, `artifact_sha256`,
`reproducibility_fingerprint`, and `locked_components`.
Repeated `field=controlled_impedance_solver_run_log` rows require `name`,
`source`, `solver`, `solver_version`, `run_id`, `artifact_uri`,
`artifact_sha256`, `random_seed`, `numeric_tolerance_policy`,
`max_residual_error`, and `max_iterations`. Optional `min_rerun_count` and
`max_rerun_impedance_delta_ohm` columns declare a deterministic rerun policy.
Optional `min_convergence_sample_count`,
`max_convergence_impedance_delta_ohm`, and `required_stopping_criteria`
columns declare a convergence-window policy.
Repeated `field=controlled_impedance_solver_rerun` rows require
`solver_run_log`, `name`, `source`, `run_id`, `artifact_uri`,
`artifact_sha256`, `random_seed`, `solved_impedance_ohm`, `residual_error`,
and `iterations`; rows attach under the named imported or pre-existing solver
run log.
Repeated `field=controlled_impedance_solver_convergence_sample` rows require
`solver_run_log`, `name`, `source`, `artifact_uri`, `artifact_sha256`,
`iteration`, `solved_impedance_ohm`, `residual_error`, and
`stopping_criteria`; rows attach under the named imported or pre-existing
solver run log.
Declaring material-library columns requires a matching reviewed
`controlled_impedance_solver_material_library` artifact-content row before
validation can accept the solver result. If reviewed
`controlled_impedance_solver_material_acceptance` rows are present, validation
also requires a matching fabricator acceptance row for the solver result
material library/revision and fabricator stackup revision. If reviewed
`controlled_impedance_solver_material_process` rows are present, validation
also requires a matching lot/process row for the solver result material
library/revision, fabricator stackup revision, dielectric layer, and reviewed
material, then checks explicit Dk/thickness drift against reviewed limits.
Declaring any
stackup signoff column requires complete
signoff source/revision/artifact provenance, a 64-character signoff artifact
SHA-256 digest, and a fabricator stackup revision matching the solver result
`stackup_revision`.

`controlled_impedance_solver_sample` rows use `value` as sampled
`solved_impedance_ohm` and require `solver_result_name`, `name`, `source`,
`corner`, and `frequency_mhz`. Rows attach under the named imported or
pre-existing solver result; duplicate `solver_result_name`/sample-name pairs
fail closed.

`controlled_impedance_solver_material_corner` rows use `value` as reviewed
corner `dielectric_constant` and require `solver_result_name`, `name`,
`source`, `corner`, `dielectric_layer`, `material`,
`nominal_dielectric_constant`, `material_library`, and
`material_library_revision`. Rows attach under the named imported or
pre-existing solver result; duplicate `solver_result_name`/corner-name pairs
fail closed. Validation uses these rows to prove that declared solver corners
map back to reviewed stackup material evidence; it does not parse or execute
the material library artifact.

`controlled_impedance_solver_material_library` rows declare reviewed solver
material-library artifact content and require `name`, `source`,
`material_library`, `material_library_revision`, `artifact_uri`, a
64-character `artifact_sha256`, plus non-empty `corners`, `dielectric_layers`,
`materials`, and `content_fields` lists. `content_fields` must include
`corner`, `dielectric_layer`, `material`, `dielectric_constant`, and
`nominal_dielectric_constant`, matching the reviewed artifact fields used by
material-corner validation. Rows create or replace
`controlled_impedance.solver_material_libraries[]` entries by `name`;
duplicate row names fail closed. Validation matches these rows to solver
results by library, revision, artifact URI, and artifact SHA-256, then checks
that required solver corners, material-corner rows, and consumed artifact
fields are backed by declared artifact content.

`controlled_impedance_solver_material_acceptance` rows declare reviewed
fabricator material-acceptance evidence and require `name`, `source`,
`material_library`, `material_library_revision`,
`fabricator_stackup_revision`, `acceptance_artifact_uri`, a 64-character
`acceptance_artifact_sha256`, plus non-empty `accepted_corners`,
`accepted_dielectric_layers`, and `accepted_materials` lists. Optional
`accepted_by` records the reviewer or approval channel. Rows create or replace
`controlled_impedance.solver_material_acceptances[]` entries by `name`;
duplicate row names fail closed. Validation matches these rows to solver
results by material library, material-library revision, and fabricator stackup
revision, then checks that required solver corners and material-corner
layers/materials are accepted.

`controlled_impedance_solver_material_process` rows declare reviewed
fabricator material lot/process drift evidence and require `name`, `source`,
`material_library`, `material_library_revision`,
`fabricator_stackup_revision`, `dielectric_layer`, `material`, `process_lot`,
`material_lot`, `process_revision`, `drift_artifact_uri`, a 64-character
`drift_artifact_sha256`, positive `accepted_dielectric_constant`,
`measured_dielectric_constant`, `accepted_thickness_mm`, and
`measured_thickness_mm`, plus non-negative `max_dielectric_constant_delta`
and `max_thickness_delta_mm`. Rows create or replace
`controlled_impedance.solver_material_processes[]` entries by `name`;
duplicate row names fail closed. Validation matches these rows to solver
results by material library, material-library revision, fabricator stackup
revision, dielectric layer, and reviewed material, then checks measured-vs-
accepted drift values against the reviewed limits.

`controlled_impedance_solver_qualification` rows declare reviewed solver
tool/version qualification evidence and require `name`, `source`, `solver`,
`solver_version`, `qualification_artifact_uri`, and a 64-character
`qualification_artifact_sha256`. Rows create or replace
`board.manufacturing.controlled_impedance.solver_qualifications[]` entries by
stable `name`; duplicate CSV names fail closed.

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
