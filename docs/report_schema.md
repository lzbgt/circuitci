# Report Schema

CircuitCI reports are built for both AI agents and engineers.

## JSON Report

```json
{
  "schema_version": "0.1.0",
  "project": "bad_backdrive_board",
  "profile": "iot_basic_v0",
  "result": "fail",
  "summary": {
    "critical": 1,
    "warning": 0,
    "info": 1
  },
  "failures": [],
  "warnings": [],
  "infos": [],
  "waveforms": [],
  "artifacts": [],
  "model_file_provenance": [],
  "limitations": [],
  "suggested_next_actions": [],
  "reproduction": {
    "command": "circuitci validate examples/bad_backdrive_board/project.yaml --output out/"
  }
}
```

## Finding Object

```json
{
  "id": "GPIO_BACKDRIVE",
  "severity": "critical",
  "scenario": "usb_hot_plug_mcu_unpowered",
  "message": "Powered component U2.TXD drives unpowered component U1.RX on net uart_rx.",
  "component": "U1",
  "net": "uart_rx",
  "endpoints": {
    "driver": { "component": "U2", "pin": "TXD" },
    "victim": { "component": "U1", "pin": "RX" }
  },
  "measured": {
    "injection_current_A": 0.0012
  },
  "limit": {
    "injection_current_A": 0.0001
  },
  "suggested_fixes": [
    "Add a series resistor sized to keep injection current below the receiving pin limit.",
    "Add a bus switch or isolation device.",
    "Ensure both components are in the same powered domain before driving the net."
  ]
}
```

## Result Semantics

- `fail`: at least one critical finding.
- `pass`: no critical finding.

For schema version `0.1.0`, the result is exactly:

```text
fail iff summary.critical > 0, otherwise pass
```

Warnings and limitations remain visible in the report but do not change `result` in schema version `0.1.0`.

## Limitation Object

```json
{
  "id": "UNSUPPORTED_SCENARIO",
  "scope": "scenario:thermal_map",
  "confidence": "low",
  "blocking": true,
  "message": "Scenario type thermal_map is documented but not implemented in this runtime."
}
```

Unsupported `iot_basic_v0` checks are blocking for fabrication readiness even when the executable subset has no critical finding.

`iot_basic_v0` reports must include a non-blocking
`PROFILE_COVERAGE_PARTIAL` limitation when declared project scenarios do not
cover the core executable profile checks. This limitation does not change
`result`; it prevents partial scenario declarations from being mistaken for
full-profile sign-off. Use `suggest-scenarios --profile iot_basic_v0` after
importing available evidence to generate missing-core-check remediation
templates.

Projects using `generic`, `estimated`, or `low` confidence component models must include non-blocking `LOW_CONFIDENCE_MODEL` limitations scoped to the component and model. Use `MODEL_QUALITY_REQUIRED` when a specific component's model quality must block fabrication sign-off.

## Additional Rule Findings

### `ANALOG_SWEEP_MARGIN_SUMMARY`

Analog transient, AC/Bode, DC operating-point, and noise scenarios with
`analog.sweeps` emit one informational summary per evaluated swept assertion.
These findings do not change pass/fail semantics; they identify the limiting
sweep corner so designers can see which input values control margin. Monte
Carlo sweeps use the same fields because sampled tolerances are expanded into
ordinary component-value corners before assertion evaluation.

Typical fields:

- `measured.analog_sweep`: sweep name.
- `measured.analog_corner`: corner label such as `corner_003`.
- `measured.analog_parameters`: parameter/value map used for that corner.
- `measured.analog_component_values`: generated component field/value map such
  as `RLOAD.value_ohm` or `VSUPPLY.dc_v` used for that corner.
- `measured.analog_model_sections`: model-file path to selected `.lib` section
  map used for that corner.
- `measured.assertion`, `measured.probe`, `measured.measured_value`,
  `measured.measured_unit`, `measured.margin`, `measured.passed`, and
  `measured.evaluated_corners`.
- `measured.measured_unit` is the compared assertion unit. Analog waveform
  checks may therefore report instantaneous units (`V`, `A`, `W`), timing/count
  units (`us`, `%`, `crossings`), or integrated units (`V*s`, `C`, `J`).
- `limit.relation`, `limit.limit_value`, `limit.limit_unit`, and
  `limit.minimum_margin`.

### `ANALOG_MONTE_CARLO_YIELD_SUMMARY`

Monte Carlo sweeps additionally emit one summary per evaluated assertion. These
findings preserve the same `measured.analog_sweep`,
`measured.analog_corner`, assertion, probe, quantity, component-value, parameter,
and model-section fields as `ANALOG_SWEEP_MARGIN_SUMMARY`, but the limiting
corner is the worst sampled margin and the aggregate fields describe the sampled
distribution. The summary is informational by default; if
`analog.sweeps[].monte_carlo.criteria` is declared, it becomes critical when any
declared yield or sampled-margin percentile limit fails. In criteria mode,
per-sample assertion failures are retained as informational evidence rows with
`measured.monte_carlo_sample_assertion_evidence: true`; backend, solver, and
non-assertion validation failures remain critical.

- `measured.evaluated_samples`: number of sampled corners with assertion
  measurements.
- `measured.passed_samples` and `measured.failed_samples`: assertion pass/fail
  counts across the sampled corners.
- `measured.yield_percent`: `passed_samples / evaluated_samples * 100`.
- `measured.mean_margin`, `measured.stddev_margin`, `measured.min_margin`, and
  `measured.max_margin`: margin distribution in the assertion unit.
- `measured.p1_margin`, `measured.p5_margin`, `measured.p50_margin`, and
  `measured.p95_margin`: linearly interpolated sampled-margin percentiles in
  the assertion unit.
- `measured.criteria_passed`: present only when criteria are declared; `true`
  when every declared yield/percentile criterion passes.
- `limit.minimum_margin`: always `0.0`; sample failures are already represented
  by the underlying assertion findings and by `failed_samples`.
- `limit.minimum_yield_percent`, `limit.minimum_p1_margin`,
  `limit.minimum_p5_margin`, `limit.minimum_p50_margin`, and
  `limit.minimum_p95_margin`: present only for declared Monte Carlo criteria.

Reset/boot/download rules use the same finding object. Required IDs:

- `RESET_RELEASE_AFTER_POWER_VALID`
- `BOOT_STRAP_DEFINED`
- `BOOT_STRAP_BIAS_VALID`
- `UART_BOOTLOADER_SYNC`
- `RESIDENT_BOOTLOADER_UPDATE_SEQUENCE`
- `CONTROL_LINE_RELEASE_SEQUENCE`
- `FUNCTIONAL_MCU_FIRMWARE`
- `INTERFACE_PROTECTION_REVIEW`
- `BUS_TERMINATION_VALID`
- `BUS_PROTECTION_PLACEMENT_VALID`
- `USB_CONNECTOR_PROTECTION_VALID`
- `USB_PROTECTION_PLACEMENT_VALID`
- `USB_CONNECTOR_ORIENTATION_VALID`
- `USB_CONNECTOR_EDGE_PROXIMITY_VALID`
- `USB_CONNECTOR_BODY_OVERHANG_VALID`
- `USB_ROUTE_GEOMETRY_VALID`
- `USB_VBUS_ROUTE_VALID`
- `USB_RETURN_PATH_VALID`
- `CONTROLLED_IMPEDANCE_GEOMETRY_VALID`
- `CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID`
- `CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID`
- `CONTROLLED_IMPEDANCE_COUPON_VALID`
- `CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID`
- `CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID`
- `CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID`
- `ADJACENT_PLANE_RETURN_PATH_VALID`
- `REFERENCE_PLANE_SLOT_CROSSING_VALID`
- `RETURN_PATH_STITCHING_VIA_VALID`
- `RF_ANTENNA_KEEPOUT_VALID`
- `RF_ANTENNA_FEED_PATH_VALID`
- `RF_ANTENNA_MATCHING_TOPOLOGY_VALID`
- `RF_ANTENNA_MEASURED_PERFORMANCE_VALID`
- `THERMAL_COPPER_AREA_VALID`
- `THERMAL_VIA_STACKUP_VALID`
- `THERMAL_VIA_PLATING_VALID`
- `THERMAL_VIA_BARREL_CROSS_SECTION_VALID`
- `THERMAL_PACKAGE_TEMPERATURE_VALID`
- `THERMAL_MEASURED_TEMPERATURE_VALID`
- `THERMAL_DERATING_ENVIRONMENT_VALID`
- `CLOCK_SOURCE_VALID`
- `POWER_TREE_VALID`
- `IO_VOLTAGE_COMPATIBLE`
- `MOTOR_BRIDGE_BUDGET_VALID`
- `MOTOR_LOAD_SUPPLY_VALID`
- `MOTOR_BRIDGE_LOSS_THERMAL_VALID`
- `MOTOR_BRIDGE_SWITCHING_VALID`
- `MOTOR_BRIDGE_SOA_VALID`
- `MOTOR_REGEN_CLAMP_VALID`
- `MOTOR_ROUTE_CURRENT_VALID`
- `MOTOR_CURRENT_SENSE_ACCURACY_VALID`
- `MOTOR_CURRENT_SENSE_PLACEMENT_VALID`
- `MODEL_QUALITY_REQUIRED`
- `POWER_SWITCH_BUDGET_VALID`
- `POWER_SWITCH_REVERSE_CURRENT_VALID`
- `POWER_SWITCH_INRUSH_VALID`
- `LOAD_CABLE_CURRENT_VALID`
- `LOAD_CABLE_THERMAL_DERATING_VALID`
- `LOAD_CABLE_VOLTAGE_DROP_VALID`
- `SPICE_TRANSIENT_ANALYSIS`
- `SPICE_AC_ANALYSIS`
- `SPICE_DC_ANALYSIS`
- `SPICE_DC_SWEEP_ANALYSIS`
- `SPICE_NOISE_ANALYSIS`
- `SPICE_S_PARAMETER_ANALYSIS`
- `SPICE_TRANSFER_FUNCTION_ANALYSIS`
- `SPICE_POLE_ZERO_ANALYSIS`
- `SPICE_SENSITIVITY_ANALYSIS`
- `SPICE_FOURIER_ANALYSIS`
- `SPICE_HARMONIC_BALANCE_ANALYSIS`
- `SPICE_PSS_ANALYSIS`
- `SPICE_PHASE_NOISE_ANALYSIS`
- `SPICE_MEASURE_ANALYSIS`
- `SPICE_OPERATING_LIMIT`

Reports must include `scenario`, `component` when applicable, measured timing values in `measured`, limits or expected states in `limit`, and concrete suggested fixes.

Stable rule detail keys:

- `RESET_RELEASE_AFTER_POWER_VALID.measured`: `power_valid_at_us`,
  `target_rail_power_valid_at_us`, `scenario_power_valid_at_us`,
  `reset_release_delay_us`, `reset_release_at_us`, `margin_us`.
- `RESET_RELEASE_AFTER_POWER_VALID.limit`:
  `reset_release_not_before_power_valid: true`,
  `required_reset_release_at_us`,
  `scenario_power_valid_matches_target_rail: true`.
- `BOOT_STRAP_DEFINED.measured`: `required_boot_mode`, `observed_<pin>`.
- `BOOT_STRAP_DEFINED.limit`: `required_<pin>`.
- `BOOT_STRAP_BIAS_VALID.measured`: `required_boot_mode`,
  `strap_voltage_V`, optional `strap_bias_current_A`, and optional
  `strap_bias_sources`.
- `BOOT_STRAP_BIAS_VALID.limit`: `required_<pin>`, `vih_min_V`,
  `vil_max_V`, and optional `max_strap_bias_current_A`.
- `UART_BOOTLOADER_SYNC.measured`: `interface`, `sync_event_found`, `event_at_us`.
- `UART_BOOTLOADER_SYNC.limit`: `sync_byte`, `expected_response`, `rx_pin`, `required_boot_mode`.
- `CLOCK_SOURCE_VALID.measured`: `clock_source`, `clock_input_net`,
  `clock_output_net`, `crystal_component`, `frequency_Hz`,
  `input_load_capacitance_F`, `output_load_capacitance_F`,
  `stray_capacitance_F`, `effective_load_capacitance_F`.
- `CLOCK_SOURCE_VALID.limit`: `required_crystal_between_clock_nets`,
  `required_load_capacitors_to_ground`, `crystal_load_capacitance_min_F`,
  `crystal_load_capacitance_max_F`, `clock_source_field`.

Interface-protection findings may include supply constraint detail:

- `INTERFACE_PROTECTION_REVIEW.measured`: `lower_supply_pin`,
  `lower_supply_net`, `lower_nominal_voltage_V`, `upper_supply_pin`,
  `upper_supply_net`, `upper_nominal_voltage_V`, plus side pin/supply/powered
  fields for unpowered-isolation failures. Clamp-only findings may include
  `reference_net_kind`, `protected_net_nominal_voltage_V`, or
  `line_capacitance_F`.
- `INTERFACE_PROTECTION_REVIEW.limit`: `supply_constraint`, `relation`,
  `lower_supply_pin`, `upper_supply_pin`, `required_unpowered_isolation`,
  `enable_pin`, and `required_disabled_state` when applicable. Clamp-only
  findings may include `protection_clamp`, `reference_pin`,
  `required_reference`, `working_voltage_max_V`, or
  `max_line_capacitance_F`.

`BUS_TERMINATION_VALID` reports are emitted by `interface_protection` scenarios
that explicitly declare bus endpoint topology and termination policy. Stable
measured keys include `line_a_net`, `line_b_net`, and `termination_ohm`.
Stable limit keys include `expected_termination_ohm` and
`termination_tolerance_percent`. The rule is intentionally topology-scoped: it
does not infer that all CAN/RS485 nodes should be terminated.

`BUS_PROTECTION_PLACEMENT_VALID` reports are emitted by
`interface_protection` scenarios that explicitly declare a differential bus
pair, a reference component, a checked protection or termination component, and
ordered `board.layout.routes` evidence for both lines. Stable measured keys
include `line_a_net`, `line_b_net`, `reference_component`,
`line_a_route_distance_mm`, and `line_b_route_distance_mm`; off-route findings
include `net` and `reference_component`. Stable limit keys include
`max_reference_to_checked_route_distance_mm` and
`max_component_to_route_distance_mm`.

`USB_CONNECTOR_PROTECTION_VALID` reports are emitted by `interface_protection`
scenarios that target a component model with `usb_connector` metadata. Stable
measured keys include `connector_component`, `connector_model`, `connector_pin`,
`protected_net`, `protection_component`, `protection_clamp`, `reference_pin`,
`reference_net`, `reference_net_kind`, `working_voltage_max_V`,
`shield_pin`, `shield_net`, and `shield_net_kind` when applicable. Stable limit
keys include `required_protection`, `required_reference`,
`required_data_working_voltage_min_V`,
`required_vbus_working_voltage_min_V`, and `required_shield_net_kind`.

`USB_PROTECTION_PLACEMENT_VALID` reports are emitted by
`interface_protection` scenarios that combine `usb_connector` metadata,
clamp-only protection metadata, and `board.layout.placements`. Stable measured
keys include `connector_component`, `connector_pin`, `protected_net`,
`connector_x_mm`, `connector_y_mm`, `connector_side`,
`protection_component`, `protection_clamp`, `protection_x_mm`,
`protection_y_mm`, `protection_side`, and `distance_mm` when placement
evidence is available. Stable limit keys include
`max_connector_to_protection_distance_mm`, `required_placement`, and
`required_protection`.

`USB_ROUTE_GEOMETRY_VALID` reports are emitted by `interface_protection`
scenarios that combine `usb_connector` metadata, clamp-only protection
metadata, `board.layout.placements`, and `board.layout.routes`. Stable measured
keys include `connector_signal`, `route_length_mm`, `via_count`,
`protection_component`, `connector_to_protection_route_distance_mm`,
`protection_components_without_placement`, and
`protection_components_off_route`. When pad-contact evidence is required,
route-order findings also report `connector_pad`, `protection_pad`,
`pad_component`, `pad_pin`, `protection_pads_missing`, and
`protection_pads_off_route` as applicable. Width findings also report
`segment_index`, `route_segment_width_mm`, and `route_width_delta_mm`.
Differential-pair findings also report `dp_net`, `dm_net`,
`dp_route_length_mm`, `dm_route_length_mm`, `data_pair_length_mismatch_mm`,
`dp_via_count`, `dm_via_count`, `data_pair_via_count_delta`,
`data_pair_centerline_distance_mm`, `data_pair_gap_mm`, and
`data_pair_gap_delta_mm`. Stable limit keys include
`max_data_line_route_length_mm`, `max_data_line_via_count`,
`expected_data_line_width_mm`, `max_data_line_width_delta_mm`,
`max_connector_to_protection_route_distance_mm`,
`max_component_to_route_distance_mm`, `max_data_pair_length_mismatch_mm`,
`max_data_pair_via_count_delta`, `expected_data_pair_gap_mm`, and
`max_data_pair_gap_delta_mm`. Pad-contact route findings additionally include
`route_pad_contact_policy`.

`USB_CONNECTOR_ORIENTATION_VALID` reports are emitted by
`interface_protection` scenarios that combine `usb_connector` metadata with
`board.layout.placements.<connector>.rotation_deg`. Stable measured keys
include `connector_rotation_deg`, `connector_rotation_error_deg`,
`connector_x_mm`, `connector_y_mm`, and optional `connector_side`. Stable limit
keys include `expected_connector_rotation_deg` and
`max_connector_rotation_error_deg`.

`USB_CONNECTOR_EDGE_PROXIMITY_VALID` reports are emitted by
`interface_protection` scenarios that combine `usb_connector` metadata,
`board.layout.placements`, and straight `board.layout.outline.segments`
evidence. Stable measured keys include
`connector_to_board_edge_distance_mm`, `connector_x_mm`, `connector_y_mm`,
`connector_edge_reference`, optional `connector_side`, optional
`footprint_graphic_layer`, optional `footprint_graphic_kind`,
`board_edge_start_x_mm`, `board_edge_start_y_mm`, `board_edge_end_x_mm`,
`board_edge_end_y_mm`, optional `board_edge_layer`, optional
`board_edge_source_primitive`, optional `board_edge_source_primitive_index`,
optional `board_edge_sample_index`, optional `board_edge_sample_count`,
optional `board_edge_contour_index`, and optional `board_edge_boundary_role`.
Stable limit keys include `max_connector_to_board_edge_distance_mm`.

`USB_CONNECTOR_BODY_OVERHANG_VALID` reports are emitted by
`interface_protection` scenarios that combine `usb_connector` metadata,
`board.layout.placements`, straight `board.layout.outline.segments`, and
connector `fabrication` or `courtyard` footprint graphics. Stable measured keys
include `connector_body_overhang_mm`, `connector_edge_reference`, optional
`footprint_graphic_layer`, optional `footprint_graphic_kind`,
`board_edge_start_x_mm`, `board_edge_start_y_mm`, `board_edge_end_x_mm`,
`board_edge_end_y_mm`, optional `board_edge_layer`, optional
`board_edge_source_primitive`, optional `board_edge_source_primitive_index`,
optional `board_edge_sample_index`, optional `board_edge_sample_count`,
optional `board_edge_contour_index`, optional `board_edge_boundary_role`,
`edge_angle_deg`, and `outward_normal_deg`. Stable limit keys include
`max_connector_body_overhang_mm`.

`USB_CONNECTOR_COMPONENT_CLEARANCE_VALID` reports are emitted by
`interface_protection` scenarios that combine `usb_connector` metadata with
connector `fabrication` or `courtyard` footprint graphics and other component
placement or footprint evidence. Stable measured keys include
`nearby_component`, `connector_to_component_clearance_mm`,
`connector_clearance_reference`, `nearby_component_clearance_reference`,
optional `connector_footprint_graphic_layer`, optional
`connector_footprint_graphic_kind`, optional
`nearby_component_footprint_graphic_layer`, and optional
`nearby_component_footprint_graphic_kind`. Stable limit keys include
`min_connector_to_component_clearance_mm`.

Scenario suggestion reports may also include
`scenario.usb_connectors[].nearest_component_clearance` for
`USB_CONNECTOR_COMPONENT_CLEARANCE_VALID` templates. That suggestion evidence
uses the same stable key names except `component` identifies the nearby
component and `clearance_mm` carries the measured connector-to-component
distance.

`USB_CONNECTOR_ENTRY_CLEARANCE_VALID` reports are emitted by
`interface_protection` scenarios that combine `usb_connector` metadata,
connector placement rotation, connector `fabrication` or `courtyard` footprint
graphics, and nearby component placement or footprint evidence. Stable measured
keys include `obstructing_component`, `entry_obstruction_depth_mm`,
`entry_obstruction_lateral_offset_mm`, `entry_direction_deg`,
`entry_direction_source`, optional `entry_direction_offset_deg`,
`entry_aperture_source`, `connector_front_projection_mm`,
`entry_aperture_front_projection_mm`,
`entry_aperture_center_lateral_projection_mm`, optional
`entry_aperture_front_offset_mm`, optional
`entry_aperture_lateral_offset_mm`, optional `entry_aperture_width_mm`,
optional `aperture_min_effective_clearance_width_mm`,
`effective_cable_entry_clearance_width_mm`,
`obstruction_reference`, optional `obstruction_footprint_graphic_layer`, and
optional `obstruction_footprint_graphic_kind`. Stable limit keys include
`min_cable_entry_clearance_depth_mm` and `cable_entry_clearance_width_mm`.
`entry_direction_source` is `scenario_parameter`, `placement_rotation`,
`component_model_offset`, `kicad_mapping_offset`, or
`footprint_property_offset`.
`entry_aperture_source` is `footprint_front`, `component_model_aperture`,
`kicad_mapping_aperture`, or `footprint_property_aperture`.

`DRILL_DIAMETER_VALID` reports are emitted by `manufacturing` scenarios that
check `board.layout.drills` circular drill evidence against selected process
diameter limits. Stable measured keys include `drill_index`, `drill_x_mm`,
`drill_y_mm`, `drill_mm`, `drill_radius_mm`, `drill_plating`, optional
`drill_castellated`, optional `drill_layer`, optional `drill_tool`, optional
`source_hit_index`, optional `drill_owner_kind`, optional `drill_net`,
optional `drill_component`, optional `drill_pin`, and optional
`drill_via_index`. Stable limit keys include `min_drill_diameter_mm` and
`max_drill_diameter_mm`.

`DRILL_TO_BOARD_EDGE_CLEARANCE_VALID` reports are emitted by `manufacturing`
scenarios that combine `board.layout.drills` evidence with
`board.layout.outline.segments`. Stable measured keys include `drill_index`,
`drill_x_mm`, `drill_y_mm`, `drill_mm`, `drill_radius_mm`, `clearance_mm`,
`center_to_board_edge_distance_mm`, `drill_plating`, optional `drill_layer`,
optional `drill_castellated`, optional `drill_tool`, optional
`source_hit_index`, optional `drill_owner_kind`, optional `drill_net`,
optional `drill_component`, optional `drill_pin`, optional `drill_via_index`,
`board_edge_start`, `board_edge_end`, optional `board_edge_layer`, optional
`board_edge_source_primitive`, optional `board_edge_source_primitive_index`,
optional `board_edge_contour_index`, and optional
`board_edge_boundary_role`. Stable limit keys include
`min_drill_edge_clearance_mm`.

`SLOT_TO_BOARD_EDGE_CLEARANCE_VALID` reports are emitted by `manufacturing`
scenarios that combine `board.layout.slots` evidence with
`board.layout.outline.segments`. Stable measured keys include `slot_index`,
`slot_start`, `slot_end`, `slot_width_mm`, `slot_radius_mm`, `clearance_mm`,
`slot_centerline_to_board_edge_distance_mm`, `slot_plating`, optional
`slot_layer`, optional `slot_tool`, optional `source_slot_index`,
`board_edge_start`, `board_edge_end`, optional `board_edge_layer`, optional
`board_edge_source_primitive`, optional `board_edge_source_primitive_index`,
optional `board_edge_contour_index`, and optional `board_edge_boundary_role`.
Stable limit keys include `min_slot_edge_clearance_mm`.

`SLOT_WIDTH_VALID` reports are emitted by `manufacturing` scenarios that check
`board.layout.slots` routed-slot process width. Stable measured keys include
`slot_index`, `slot_start`, `slot_end`, `slot_width_mm`, `slot_radius_mm`,
`slot_plating`, `slot_process`, optional `slot_layer`, optional `slot_tool`,
and optional `source_slot_index`. Stable limit keys include
`min_slot_width_mm`.

`SLOT_ASPECT_RATIO_VALID` reports are emitted by `manufacturing` scenarios that
check `board.layout.slots` routed-slot length-to-width process evidence. Stable
measured keys include `slot_index`, `slot_start`, `slot_end`, `slot_width_mm`,
`slot_radius_mm`, `slot_plating`, `slot_length_mm`, `slot_aspect_ratio`,
optional `slot_layer`, optional `slot_tool`, and optional `source_slot_index`.
Stable limit keys include `min_slot_aspect_ratio`.

`CASTELLATED_HOLE_VALID` reports are emitted by `manufacturing` scenarios that
compare explicitly marked `board.layout.drills[].castellated` evidence with a
castellated-hole process rule. Stable measured keys reuse drill evidence fields
from `DRILL_DIAMETER_VALID` and `DRILL_TO_BOARD_EDGE_CLEARANCE_VALID`,
including `drill_index`, `drill_x_mm`, `drill_y_mm`, `drill_mm`,
`drill_radius_mm`, `drill_plating`, optional `drill_castellated`,
`clearance_mm`, `center_to_board_edge_distance_mm`, and board-edge provenance
keys when an edge-clearance finding is emitted. Hole-pair spacing findings
report `first_drill_index`, `first_drill_at`, `first_drill_mm`,
`second_drill_index`, `second_drill_at`, `second_drill_mm`, and
`castellated_hole_to_hole_spacing_mm`. Stable limit keys include
`min_castellated_hole_diameter_mm`,
`min_castellated_hole_edge_clearance_mm`, and
`min_castellated_hole_to_hole_spacing_mm`.

`DRILL_ANNULAR_RING_VALID` reports are emitted by `manufacturing` scenarios
that combine `board.layout.drills` evidence with
`board.layout.copper.features` Gerber flash evidence. Stable measured keys
include `drill_index`, `drill_x_mm`, `drill_y_mm`, `drill_mm`,
`drill_radius_mm`, `drill_plating`, optional `drill_layer`, optional
`drill_tool`, optional `source_hit_index`, optional `drill_owner_kind`,
optional `drill_net`, optional `drill_component`, optional `drill_pin`,
optional `drill_via_index`, optional `required_copper_layer`,
`annular_ring_mm`,
`drill_to_copper_center_offset_mm`, `copper_feature_index`,
`copper_feature_x_mm`, `copper_feature_y_mm`, `copper_feature_layer`,
`copper_feature_aperture`, `copper_feature_shape`,
`copper_feature_size_x_mm`, `copper_feature_size_y_mm`,
optional `copper_feature_net`, optional `copper_feature_island_id`,
optional `copper_feature_owner_kind`, optional `copper_feature_component`,
optional `copper_feature_pin`, optional `copper_feature_via_index`, optional
`drill_copper_owner_mismatch`,
`copper_feature_source_primitive`, and
`copper_feature_source_primitive_index` when a matching flash exists. Stable
limit keys include `min_annular_ring_mm` and
`max_drill_to_copper_center_offset_mm`.

`COPPER_TO_BOARD_EDGE_CLEARANCE_VALID` reports are emitted by
`manufacturing` scenarios that combine `board.layout.copper.features`,
`board.layout.copper.segments`, or `board.layout.copper.regions` evidence with
`board.layout.outline.segments`.
Stable measured keys include `copper_kind`, `clearance_mm`,
`board_edge_start`, `board_edge_end`, optional `board_edge_layer`, optional
`board_edge_source_primitive`, optional `board_edge_source_primitive_index`,
optional `board_edge_contour_index`, and optional `board_edge_boundary_role`.
Feature findings also report `copper_feature_index`, `copper_feature_x_mm`,
`copper_feature_y_mm`, `copper_feature_layer`, optional
`copper_feature_net`, optional `copper_feature_island_id`,
optional `copper_feature_owner_kind`, optional `copper_feature_component`,
optional `copper_feature_pin`, optional `copper_feature_via_index`,
`copper_feature_aperture`, `copper_feature_shape`, `copper_feature_size_x_mm`,
`copper_feature_size_y_mm`, `copper_feature_source_primitive`, and
`copper_feature_source_primitive_index`. Segment findings report
`copper_segment_index`, `copper_segment_start`, `copper_segment_end`,
`copper_segment_layer`, optional `copper_segment_net`, optional
`copper_segment_island_id`, `copper_segment_aperture`,
`copper_segment_width_mm`, `copper_segment_source_primitive`,
`copper_segment_source_primitive_index`, and
`trace_centerline_to_board_edge_distance_mm`. Region findings report
`copper_region_index`, `copper_region_layer`, optional `copper_region_net`,
optional `copper_region_island_id`, `copper_region_polarity`,
`copper_region_source_primitive`,
`copper_region_source_primitive_index`, and
`copper_region_point_count`. Stable limit keys include
`min_copper_edge_clearance_mm`.

`COPPER_SPACING_VALID` reports are emitted by `manufacturing` scenarios that
compare same-layer `board.layout.copper.features`,
`board.layout.copper.segments`, and `board.layout.copper.regions` evidence.
Stable measured keys include
`clearance_mm`, `copper_layer`, `first_copper_kind`, and
`second_copper_kind`. Feature operands report prefixed keys such as
`first_copper_feature_index`, `first_copper_feature_x_mm`,
`first_copper_feature_y_mm`, `first_copper_feature_layer`,
optional `first_copper_feature_net`, optional
`first_copper_feature_island_id`, optional
`first_copper_feature_owner_kind`, optional `first_copper_feature_component`,
optional `first_copper_feature_pin`, optional `first_copper_feature_via_index`,
`first_copper_feature_aperture`,
`first_copper_feature_shape`, `first_copper_feature_size_x_mm`,
`first_copper_feature_size_y_mm`,
`first_copper_feature_source_primitive`, and
`first_copper_feature_source_primitive_index`; the same keys may appear with
the `second_` prefix. Segment operands report prefixed keys such as
`first_copper_segment_index`, `first_copper_segment_start`,
`first_copper_segment_end`, `first_copper_segment_layer`,
optional `first_copper_segment_net`, optional
`first_copper_segment_island_id`, `first_copper_segment_aperture`,
`first_copper_segment_width_mm`, `first_copper_segment_source_primitive`, and
`first_copper_segment_source_primitive_index`; the same keys may appear with
the `second_` prefix. Region operands report prefixed keys such as
`first_copper_region_index`, `first_copper_region_layer`,
optional `first_copper_region_net`, optional
`first_copper_region_island_id`, `first_copper_region_polarity`,
`first_copper_region_source_primitive`,
`first_copper_region_source_primitive_index`, and
`first_copper_region_point_count`; the same keys may appear with the
`second_` prefix. Stable limit keys include `min_copper_spacing_mm`.

`CONDUCTOR_CREEPAGE_CLEARANCE_VALID` reports are emitted by `manufacturing`
scenarios that compare explicitly declared net pairs against same-layer
imported copper geometry. Stable measured keys include `first_net`,
`second_net`, `copper_layer`, `planar_conductor_spacing_mm`,
`clearance_distance_mm`, `creepage_distance_mm`, `clearance_violation`, and
`creepage_violation`, plus the same prefixed copper operand keys used by
`COPPER_SPACING_VALID`. Stable limit keys include `min_clearance_mm` and
`min_creepage_mm`.

`CONTROLLED_IMPEDANCE_GEOMETRY_VALID` reports are emitted by `manufacturing`
scenarios that compare imported `board.layout.routes` evidence against
explicit reviewed impedance-geometry targets. Single-ended findings use stable
measured keys `net`, `target_source`, `target_impedance_ohm`, `route_net`,
`route_segment_index`, `route_layer`, `route_measured_width_mm`,
`route_width_error_mm`, `route_segment_start`, and `route_segment_end`.
Differential-pair findings use stable measured keys `first_net`, `second_net`,
`target_source`, `target_differential_impedance_ohm`, `worst_width_net`,
`worst_width_segment_index`, `worst_width_layer`,
`worst_width_measured_width_mm`, `worst_width_width_error_mm`, `gap_layer`,
`first_gap_route_segment_index`, `second_gap_route_segment_index`,
`measured_gap_mm`, `gap_error_mm`, `width_violation`, and `gap_violation`.
Stable limit keys include `expected_width_mm`, `expected_gap_mm`,
`max_width_error_mm`, and `max_gap_error_mm` when applicable.

`CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID` reports are emitted by
`manufacturing` scenarios that verify reviewed stackup metadata for explicit
route/reference/dielectric layer triples. Topology findings use stable measured
keys `net`, `route_layer`, `reference_layer`, `dielectric_layer`,
`route_layer_index`, `reference_layer_index`, `dielectric_layer_index`,
`route_copper_thickness_um`, `reference_copper_thickness_um`,
`dielectric_thickness_mm`, `dielectric_constant`, `dielectric_material`,
`route_layer_source`, `reference_layer_source`, `dielectric_layer_source`, and
`reference_net`. Stable limit keys include
`dielectric_layer_must_be_between_route_and_reference`.

`CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID` reports are emitted by
`manufacturing` scenarios that compare imported route evidence and imported
Gerber solder-mask opening evidence against reviewed solder-mask loading
targets. Stable measured keys include `net`, `route_layer`,
`solder_mask_layer`, `target_source`, `route_segment_index`, `sample_index`,
`sample_x_mm`, `sample_y_mm`, and `measured_solder_mask_state`. Stable limit
keys include `expected_solder_mask_state`.

`CONTROLLED_IMPEDANCE_COUPON_VALID` reports are emitted by `manufacturing`
scenarios that compare reviewed fabricator coupon measurements against
reviewed impedance tolerance windows after the coupon has matched exactly one
reviewed board controlled-impedance target for the same net or differential
pair. Stable measured keys include
`coupon_name`, `coupon_type`, `source`, `net`, `first_net`, `second_net`,
`target_impedance_ohm`, `measured_impedance_ohm`, and
`impedance_error_ohm`. Stable limit keys include
`max_impedance_error_ohm`.

`CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID` reports are emitted by
`manufacturing` scenarios that compute reviewed coupon sample statistics after
the coupon has matched exactly one reviewed board controlled-impedance target.
Stable measured keys include `coupon_name`, `coupon_type`, `source`, `net`,
`first_net`, `second_net`, `target_impedance_ohm`, `sample_count`,
`mean_impedance_ohm`, `mean_impedance_error_ohm`,
`max_sample_impedance_error_ohm`, and `stddev_impedance_ohm`. Stable limit keys
include `min_batch_sample_count`, `max_batch_mean_impedance_error_ohm`,
`max_batch_sample_impedance_error_ohm`, and `max_batch_stddev_ohm`.

`CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID` reports are emitted by
`manufacturing` scenarios that compare reviewed coupon process/trace metadata
against imported board route layer, width, and differential-pair gap evidence.
Stable measured keys include `coupon_name`, `coupon_type`, `source`, `net`,
`first_net`, `second_net`, `process_lot`, `panel_id`, `stackup_revision`,
`coupon_trace_layer`, `observed_route_layers`, `layer_mismatch`,
`width_segment_net`, `width_segment_index`, `measured_width_mm`,
`max_width_delta_mm`, and, for differential coupons, `measured_gap_mm`,
`max_gap_delta_mm`, `gap_first_segment_index`, and
`gap_second_segment_index`. Stable limit keys include `coupon_trace_width_mm`,
`max_trace_width_delta_mm`, `coupon_trace_gap_mm`, and
`max_trace_gap_delta_mm`.

`CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID` reports are emitted by
`manufacturing` scenarios that compare reviewed solver-result evidence against
matching board controlled-impedance targets plus imported route and stackup
evidence. Stable measured keys include `result`, `source`, `solver`,
`solver_version`, `solver_artifact_uri`, `solver_artifact_sha256`,
`stackup_revision`, `route_layer`, `solved_impedance_ohm`,
`impedance_error_ohm`, `max_route_width_delta_mm`, and, for differential
results, `max_route_gap_delta_mm`. When reviewed solver sweep/corner policy is
present, stable measured keys also include `sample_count`, `sample_names`,
`missing_solver_corners`, `max_sample_impedance_error_ohm`, `worst_sample`,
`max_solver_frequency_gap_mhz`, `frequency_gap_start_mhz`, and
`frequency_gap_end_mhz`. When reviewed input-deck setup evidence is present,
stable measured keys also include `solver_input_deck_uri`,
`solver_input_deck_sha256`, `input_deck_mismatches`,
`input_stackup_revision`, `input_route_layer`, `input_reference_layer`,
`input_dielectric_layer`, `input_width_mm`, `input_gap_mm`, and
`input_frequency_mhz`; with reviewed roughness evidence, stable measured keys
also include `input_copper_roughness_model` and
`input_copper_roughness_um`; with reviewed etch compensation evidence, stable
measured keys also include `input_etch_compensation_model` and
`input_etch_compensation_um`. If signed-artifact metadata is declared but
incomplete, the check fails closed as `VALIDATION_INPUT_MISSING`; signature
metadata is provenance input and is not repeated in pass/fail measured maps.
If solver output-schema metadata is declared but incomplete or malformed, the
check fails closed as `VALIDATION_INPUT_MISSING`; schema metadata is provenance
input and is not repeated in pass/fail measured maps.
If solver configuration-lock metadata is declared but incomplete, malformed,
or tied to a different tool than the solver result, the check fails closed as
`VALIDATION_INPUT_MISSING`; config-lock metadata is provenance input and is
not repeated in pass/fail measured maps.
If solver runtime allowlist metadata is declared but missing, incomplete,
stale, or does not cover every declared runtime option, the check fails closed
as `VALIDATION_INPUT_MISSING`; runtime allowlist metadata is provenance input
and is not repeated in pass/fail measured maps.
If solver entitlement metadata is declared but missing, incomplete, stale, or
does not cover every declared licensed feature, the check fails closed as
`VALIDATION_INPUT_MISSING`; entitlement metadata is provenance input and is
not repeated in pass/fail measured maps.
If solver execution-environment metadata is declared but missing, incomplete,
stale, or does not cover every declared locked component, the check fails
closed as `VALIDATION_INPUT_MISSING`; environment metadata is provenance input
and is not repeated in pass/fail measured maps.
If solver run-log metadata is declared but missing, incomplete, stale, has
residual/iteration evidence outside reviewed limits, or declares deterministic
rerun policy without enough matching rerun samples inside the reviewed
impedance/residual/iteration windows, or declares convergence-window policy
without enough matching convergence samples inside the reviewed
stopping-criteria/impedance/residual/iteration windows, or requires monotonic
residual decrease but has convergence samples whose residual error increases
with iteration, or declares numerical precision policy evidence whose roundoff
budget exceeds the solver result/convergence error budgets, the check fails closed as
`VALIDATION_INPUT_MISSING`; run-log metadata is provenance input and is not
repeated in pass/fail measured maps.
With reviewed material-library evidence, stable input-deck mismatch measured
keys also include `solver_material_library`,
`solver_material_library_revision`, `solver_material_library_artifact_uri`,
`solver_material_library_artifact_sha256`, `input_material_library`, and
`input_material_library_revision`.
If reviewed solver material-library artifact content evidence is missing,
ambiguous, incomplete, stale, or does not cover required corners and declared
material-corner layers/materials/fields, the check fails closed as
`VALIDATION_INPUT_MISSING`. If reviewed solver material-acceptance evidence is
present but missing, ambiguous, incomplete, stale, or does not accept required
corners, dielectric layers, or materials for the solver result, the check fails
closed as `VALIDATION_INPUT_MISSING`. If reviewed solver material-process
evidence is present but missing, ambiguous, incomplete, stale, or has measured
dielectric constant or thickness drift beyond reviewed limits, the check fails
closed as `VALIDATION_INPUT_MISSING`. If reviewed solver material-corner evidence is
declared but incomplete, ambiguous, or inconsistent with
`required_solver_corners`, solver material library metadata, or reviewed
stackup layer material/Dk evidence, the check fails closed as
`VALIDATION_INPUT_MISSING`.
If reviewed fabricator stackup signoff evidence is declared but incomplete or
its `fabricator_stackup_revision` does not match the solver result
`stackup_revision`, the check fails closed as `VALIDATION_INPUT_MISSING`
before solver-result limit findings are emitted.
Stable limit keys include `max_impedance_error_ohm`,
`max_route_width_delta_mm`, `max_route_gap_delta_mm`,
`min_solver_sample_count`, `max_solver_frequency_step_mhz`,
`required_solver_corners`, `stackup_revision`, `route_layer`,
`reference_layer`, `dielectric_layer`, `solved_width_mm`, `solved_gap_mm`, and
`frequency_mhz`; with reviewed roughness evidence, stable limit keys also
include `copper_roughness_model` and `copper_roughness_um`; with reviewed
etch compensation evidence, stable limit keys also include
`etch_compensation_model` and `etch_compensation_um`. When reviewed
`controlled_impedance.solver_qualifications[]` evidence exists, missing,
ambiguous, or malformed solver/version qualification evidence is reported as
`VALIDATION_INPUT_MISSING` before solver-result limit findings are emitted.

`ADJACENT_PLANE_RETURN_PATH_VALID` reports are emitted by `manufacturing`
scenarios that compare imported `board.layout.routes` evidence against
explicit `board.layout.stackup.layers` and `board.layout.zones` reference-plane
coverage evidence. Stable measured keys include `net`, `reference_net`,
`total_route_length_mm`, `unreferenced_route_length_mm`,
`unreferenced_segment_count`, `reference_layers`, and, for the first uncovered
segment, `first_unreferenced_route_segment_index`,
`first_unreferenced_route_layer`, and `first_unreferenced_reference_layer`.
Stable limit keys include `max_unreferenced_length_mm`.

`REFERENCE_PLANE_SLOT_CROSSING_VALID` reports are emitted by `manufacturing`
scenarios that compare imported `board.layout.routes` evidence against
explicit `board.layout.stackup.layers` and `board.layout.zones` reference-plane
coverage intervals. Stable measured keys include `net`, `reference_net`,
`total_route_length_mm`, `slot_crossing_count`, `reference_layers`, and, for
the first crossing, `first_slot_route_segment_index`,
`first_slot_route_layer`, `first_slot_reference_layer`, `first_slot_start_mm`,
and `first_slot_end_mm`. Stable limit keys include `max_slot_crossings`.

`RETURN_PATH_STITCHING_VIA_VALID` reports are emitted by `manufacturing`
scenarios that compare imported `board.layout.routes.<net>.vias[]` evidence
against a declared reference-net route via set. Stable measured keys include
`net`, `reference_net`, `signal_via_count`, `reference_via_count`,
`signal_via_index`, `signal_via_x_mm`, `signal_via_y_mm`,
`signal_via_layers`, and, when a matching reference via exists outside the
limit, `nearest_reference_via_index`, `nearest_reference_via_distance_mm`,
`nearest_reference_via_x_mm`, `nearest_reference_via_y_mm`, and
`nearest_reference_via_layers`. Stable limit keys include
`max_stitch_via_distance_mm` and `matching_layer_policy`.

`RF_ANTENNA_KEEPOUT_VALID` reports are emitted by `manufacturing` scenarios
that compare reviewed RF antenna keepout polygons against explicit same-layer
imported copper evidence. Stable measured keys include `keepout_name`,
`keepout_source`, `keepout_layer`, optional `antenna_net`,
`keepout_polygon_point_count`, `copper_kind`, `copper_index`,
optional `copper_net`, optional `copper_component`, and `clearance_mm`.
Feature findings also report `copper_feature_shape`, `copper_feature_at`, and
`copper_feature_size`. Segment findings also report `copper_segment_start`,
`copper_segment_end`, and `copper_segment_width_mm`. Region findings also
report `copper_region_point_count` and `copper_region_intrudes_keepout`.
Stable limit keys include `min_copper_clearance_mm`.

`RF_ANTENNA_FEED_PATH_VALID` reports are emitted by `manufacturing` scenarios
that compare reviewed RF antenna feed-path metadata against imported route,
pad, placement, and component-pin evidence. Stable measured keys include
`feed_path_name`, `feed_path_source`, `antenna_net`, `feed_component`,
`feed_pin`, and `matching_component_count`. Route-length findings also report
`feed_route_length_mm` and `feed_route_segment_count`; stable limit keys
include `max_feed_route_length_mm`. Matching-component distance findings also
report `matching_component` and `matching_component_distance_mm`; stable limit
keys include `max_matching_component_distance_mm`.

`RF_ANTENNA_MATCHING_TOPOLOGY_VALID` reports are emitted by `manufacturing`
scenarios that compare reviewed RF matching-network topology metadata against
explicit component pin and layout pad evidence. Stable measured keys include
`matching_network_name`, `matching_network_source`, `antenna_net`, `topology`,
`series_element_count`, `shunt_element_count`, and `element_count`. Stable
limit keys include `required_topology`.

`RF_ANTENNA_MEASURED_PERFORMANCE_VALID` reports are emitted by `manufacturing`
scenarios that compare reviewed RF measurement rows against explicit
return-loss, optional frequency-band, and optional sampled sweep-coverage
limits. Stable single-point measured keys include
`measurement_name`, `measurement_source`, `antenna_net`, `frequency_mhz`,
`return_loss_db`, `frequency_in_band`, and optional `measurement_method`.
Condition findings report stable measured key `measurement_condition` and
stable limit key `measurement_condition`.
Sweep-coverage findings report stable measured keys `measurement_names`,
`measurement_frequencies_mhz`, `unique_in_band_measurement_count`,
`max_frequency_gap_mhz`, `frequency_gap_start_mhz`, and
`frequency_gap_end_mhz` as applicable.
Return-loss findings report stable limit key `min_return_loss_db`. Frequency
band findings report stable limit keys `frequency_min_mhz` and/or
`frequency_max_mhz` when those limits are provided. Sweep-coverage findings
report stable limit keys `min_measurement_count` and
`max_frequency_step_mhz`.

`THERMAL_COPPER_AREA_VALID` reports are emitted by `manufacturing` scenarios
that compare reviewed component thermal-copper area rules against explicit
imported copper feature, segment, and region evidence. Stable measured keys
include `thermal_copper_name`, `thermal_copper_source`, `component`,
`power_loss_w`, `nets`, `layers`, `copper_area_mm2`,
`copper_feature_area_mm2`, `copper_segment_area_mm2`,
`copper_region_area_mm2`, and `copper_object_count`. Stable limit keys include
`min_copper_area_mm2`.

`THERMAL_VIA_STACKUP_VALID` reports are emitted by `manufacturing` scenarios
that compare reviewed thermal via-count and copper-thickness policy against
explicit route-via and stackup-layer evidence. Stable measured keys include
`thermal_copper_name`, `thermal_copper_source`, `component`, `power_loss_w`,
`nets`, `layers`, `route_via_count`, `thermal_via_count`, and
`observed_min_copper_thickness_um` for via-count findings. Copper-thickness
findings report `stackup_layer`, `stackup_layer_kind`, `stackup_layer_source`,
and `layer_copper_thickness_um`. Stable limit keys include
`min_thermal_via_count` and `min_copper_thickness_um`.

`THERMAL_VIA_PLATING_VALID` reports are emitted by `manufacturing` scenarios
that compare reviewed plated thermal-via count and drill-diameter policy
against explicit route-via and drill plating evidence. Stable measured keys
include `thermal_copper_name`, `thermal_copper_source`, `component`,
`power_loss_w`, `nets`, `layers`, `route_via_count`, `thermal_via_count`,
`matched_drill_count`, `plated_thermal_via_count`,
`non_plated_or_unknown_drill_count`, `plating_thickness_evidence_count`,
`observed_min_thermal_via_drill_mm`, and
`observed_min_thermal_via_plating_thickness_um`. Stable limit keys include
`min_plated_thermal_via_count`, `min_thermal_via_drill_mm`, and optional
`min_thermal_via_plating_thickness_um`.

`THERMAL_VIA_BARREL_CROSS_SECTION_VALID` reports are emitted by
`manufacturing` scenarios that compare reviewed total thermal via barrel
cross-section policy against explicit route-via and plated drill
diameter/thickness evidence. Stable measured keys include
`thermal_copper_name`, `thermal_copper_source`, `component`, `power_loss_w`,
`nets`, `layers`, `route_via_count`, `thermal_via_count`,
`matched_drill_count`, `plated_thermal_via_count`,
`non_plated_or_unknown_drill_count`, `plating_thickness_evidence_count`,
`observed_min_thermal_via_drill_mm`,
`observed_min_thermal_via_plating_thickness_um`, and
`observed_total_thermal_via_barrel_cross_section_mm2`. Stable limit keys
include `min_total_thermal_via_barrel_cross_section_mm2`.

`THERMAL_PACKAGE_TEMPERATURE_VALID` reports are emitted by `manufacturing`
scenarios that compare reviewed component power-loss metadata and component
model package thermal metadata against reviewed ambient and temperature-rise
limits. Stable measured keys include `thermal_copper_name`,
`thermal_copper_source`, `component`, `model`, `thermal_package_source`,
`power_loss_w`, `thermal_resistance_junction_to_ambient_C_per_W`,
`ambient_temperature_C`, `estimated_temperature_rise_C`, and
`estimated_junction_temperature_C`. Stable limit keys include
`max_temperature_rise_C`, `allowed_junction_temperature_C`,
`max_junction_temperature_C`, and `max_junction_temperature_margin_C`.

`THERMAL_DERATING_ENVIRONMENT_VALID` reports are emitted by `manufacturing`
scenarios that compare reviewed thermal-copper environment assumptions against
reviewed scenario operating environment inputs. Stable measured keys include
`thermal_copper_name`, `thermal_copper_source`, `component`, `power_loss_w`,
and one of `ambient_temperature_C`, `airflow_lfm`, or `enclosure_profile`.
Stable limit keys include `rated_ambient_temperature_C`, `min_airflow_lfm`, or
`required_enclosure_profile`.

`THERMAL_MEASURED_TEMPERATURE_VALID` reports are emitted by `manufacturing`
scenarios that compare reviewed thermal measurement rows against reviewed
temperature limits. Stable measured keys include `thermal_measurement_name`,
`thermal_measurement_source`, `component`, `measured_temperature_C`, optional
`ambient_temperature_C`, optional `measured_temperature_rise_C`, optional
`measurement_uncertainty_C`, optional `worst_case_measured_temperature_C`,
optional `worst_case_measured_temperature_rise_C`, optional `power_loss_w`,
optional `measurement_point`, and optional `measurement_notes`. Stable limit
keys include `max_measured_temperature_C` and optional `max_temperature_rise_C`.

`SOLDER_MASK_OPENING_VALID` reports are emitted by `manufacturing` scenarios
that compare Gerber copper flash evidence under `board.layout.copper.features`
with Gerber solder-mask opening evidence under `board.layout.solder_mask`.
Supported mask openings include flash features, circular-aperture draw
segments, and single-contour regions. Stable measured keys include
`copper_feature_index`, `copper_feature_x_mm`, `copper_feature_y_mm`,
`copper_feature_layer`, optional `copper_feature_net`, optional
`copper_feature_island_id`, optional `copper_feature_owner_kind`, optional
`copper_feature_component`, optional `copper_feature_pin`, optional
`copper_feature_via_index`, `copper_feature_aperture`,
`copper_feature_shape`, `copper_feature_size_x_mm`,
`copper_feature_size_y_mm`, `copper_feature_source_primitive`, and
`copper_feature_source_primitive_index`. Missing-opening findings also report
`expected_solder_mask_layer`. Undersized-opening findings also report
`solder_mask_kind`. Feature-opening findings report
`solder_mask_feature_index`, `solder_mask_feature_x_mm`,
`solder_mask_feature_y_mm`, `solder_mask_feature_layer`,
optional `solder_mask_feature_net`, optional
`solder_mask_feature_owner_kind`, optional `solder_mask_feature_component`,
optional `solder_mask_feature_pin`, optional
`solder_mask_feature_via_index`, `solder_mask_feature_aperture`,
`solder_mask_feature_shape`,
`solder_mask_feature_size_x_mm`, `solder_mask_feature_size_y_mm`,
`solder_mask_feature_source_primitive`,
`solder_mask_feature_source_primitive_index`,
`measured_mask_expansion_x_mm`, and `measured_mask_expansion_y_mm`.
Segment-opening findings report `solder_mask_segment_index`,
`solder_mask_segment_start`, `solder_mask_segment_end`,
`solder_mask_segment_layer`, optional `solder_mask_segment_net`, optional
`solder_mask_segment_owner_kind`, optional `solder_mask_segment_component`,
optional `solder_mask_segment_pin`, optional
`solder_mask_segment_via_index`, `solder_mask_segment_aperture`,
`solder_mask_segment_width_mm`, `solder_mask_segment_source_primitive`, and
`solder_mask_segment_source_primitive_index`. Region-opening findings report
`solder_mask_region_index`, `solder_mask_region_layer`, optional
`solder_mask_region_net`, optional `solder_mask_region_owner_kind`, optional
`solder_mask_region_component`, optional `solder_mask_region_pin`, optional
`solder_mask_region_via_index`, `solder_mask_region_source_primitive`,
`solder_mask_region_source_primitive_index`, and
`solder_mask_region_point_count`. All undersized-opening findings report
`measured_min_mask_expansion_mm`, and `copper_to_mask_center_offset_mm`.
Stable limit keys include `min_mask_expansion_mm` and
`max_copper_to_mask_center_offset_mm`.

`SOLDER_MASK_DAM_VALID` reports are emitted by `manufacturing` scenarios that
compare same-layer Gerber solder-mask opening features, segments, and regions
under `board.layout.solder_mask`. Stable measured keys include
`solder_mask_layer`, `solder_mask_dam_width_mm`, `first_solder_mask_kind`, and
`second_solder_mask_kind`. Feature operands report prefixed fields such as
`first_solder_mask_feature_index`, `first_solder_mask_feature_x_mm`,
`first_solder_mask_feature_y_mm`, `first_solder_mask_feature_layer`,
optional `first_solder_mask_feature_net`, optional
`first_solder_mask_feature_owner_kind`, optional
`first_solder_mask_feature_component`, optional
`first_solder_mask_feature_pin`, optional
`first_solder_mask_feature_via_index`,
`first_solder_mask_feature_aperture`, `first_solder_mask_feature_shape`,
`first_solder_mask_feature_size_x_mm`,
`first_solder_mask_feature_size_y_mm`,
`first_solder_mask_feature_source_primitive`, and
`first_solder_mask_feature_source_primitive_index`; the same keys may appear
with the `second_` prefix. Segment operands report prefixed fields such as
`first_solder_mask_segment_index`, `first_solder_mask_segment_start`,
`first_solder_mask_segment_end`, `first_solder_mask_segment_layer`,
optional `first_solder_mask_segment_net`, optional
`first_solder_mask_segment_owner_kind`, optional
`first_solder_mask_segment_component`, optional
`first_solder_mask_segment_pin`, optional
`first_solder_mask_segment_via_index`,
`first_solder_mask_segment_aperture`, `first_solder_mask_segment_width_mm`,
`first_solder_mask_segment_source_primitive`, and
`first_solder_mask_segment_source_primitive_index`. Region operands report
prefixed fields such as `first_solder_mask_region_index`,
`first_solder_mask_region_layer`, optional `first_solder_mask_region_net`,
optional `first_solder_mask_region_owner_kind`, optional
`first_solder_mask_region_component`, optional `first_solder_mask_region_pin`,
optional `first_solder_mask_region_via_index`,
`first_solder_mask_region_source_primitive`,
`first_solder_mask_region_source_primitive_index`, and
`first_solder_mask_region_point_count`. Stable limit keys include
`min_solder_mask_dam_mm`.

`SOLDER_PASTE_OPENING_VALID` reports are emitted by `manufacturing` scenarios
that compare Gerber copper flash evidence under `board.layout.copper.features`
with Gerber solder-paste opening evidence under `board.layout.solder_paste`.
Stable measured keys include
`copper_feature_index`, `copper_feature_x_mm`, `copper_feature_y_mm`,
`copper_feature_layer`, optional `copper_feature_net`, optional
`copper_feature_island_id`, optional `copper_feature_owner_kind`, optional
`copper_feature_component`, optional `copper_feature_pin`, optional
`copper_feature_via_index`, `copper_feature_aperture`,
`copper_feature_shape`, `copper_feature_size_x_mm`,
`copper_feature_size_y_mm`, `copper_feature_source_primitive`, and
`copper_feature_source_primitive_index`. Missing-opening findings also report
`expected_solder_paste_layer`. Area-ratio findings aggregate all co-located
paste openings within `max_copper_to_paste_center_offset_mm` and also report
`solder_paste_kind`, `copper_feature_area_mm2`,
`solder_paste_opening_area_mm2`, `solder_paste_opening_count`,
`solder_paste_area_ratio`, and `copper_to_paste_center_offset_mm`.
`solder_paste_kind` and object-specific fields identify the representative
nearest opening from that aggregate. Feature-opening findings report
`solder_paste_feature_index`, `solder_paste_feature_x_mm`,
`solder_paste_feature_y_mm`, `solder_paste_feature_layer`,
optional `solder_paste_feature_net`, optional
`solder_paste_feature_owner_kind`, optional `solder_paste_feature_component`,
optional `solder_paste_feature_pin`, optional
`solder_paste_feature_via_index`, `solder_paste_feature_aperture`,
`solder_paste_feature_shape`,
`solder_paste_feature_size_x_mm`, `solder_paste_feature_size_y_mm`,
`solder_paste_feature_source_primitive`,
`solder_paste_feature_source_primitive_index`. Segment-opening findings report
`solder_paste_segment_index`, `solder_paste_segment_start`,
`solder_paste_segment_end`, `solder_paste_segment_layer`,
optional `solder_paste_segment_net`, optional
`solder_paste_segment_owner_kind`, optional `solder_paste_segment_component`,
optional `solder_paste_segment_pin`, optional
`solder_paste_segment_via_index`, `solder_paste_segment_aperture`,
`solder_paste_segment_width_mm`,
`solder_paste_segment_source_primitive`, and
`solder_paste_segment_source_primitive_index`. Region-opening findings report
`solder_paste_region_index`, `solder_paste_region_layer`, optional
`solder_paste_region_net`, optional `solder_paste_region_owner_kind`,
optional `solder_paste_region_component`, optional
`solder_paste_region_pin`, optional `solder_paste_region_via_index`,
`solder_paste_region_source_primitive`,
`solder_paste_region_source_primitive_index`, and
`solder_paste_region_point_count`. Stable limit keys include
`min_paste_area_ratio`, `max_paste_area_ratio`, and
`max_copper_to_paste_center_offset_mm`.

`SOLDER_PASTE_APERTURE_SIZE_VALID` reports are emitted by `manufacturing`
scenarios that compare supported Gerber solder-paste openings under
`board.layout.solder_paste` with a stencil process minimum aperture size.
Stable measured keys include `solder_paste_kind` and
`solder_paste_aperture_size_mm`. Feature-opening findings report
`solder_paste_feature_index`, `solder_paste_feature_x_mm`,
`solder_paste_feature_y_mm`, `solder_paste_feature_layer`, optional
`solder_paste_feature_net`, optional `solder_paste_feature_owner_kind`,
optional `solder_paste_feature_component`, optional
`solder_paste_feature_pin`, optional `solder_paste_feature_via_index`,
`solder_paste_feature_aperture`, `solder_paste_feature_shape`,
`solder_paste_feature_size_x_mm`, `solder_paste_feature_size_y_mm`,
`solder_paste_feature_source_primitive`, and
`solder_paste_feature_source_primitive_index`. Segment-opening findings report
`solder_paste_segment_index`, `solder_paste_segment_start`,
`solder_paste_segment_end`, `solder_paste_segment_layer`, optional
`solder_paste_segment_net`, optional `solder_paste_segment_owner_kind`,
optional `solder_paste_segment_component`, optional
`solder_paste_segment_pin`, optional `solder_paste_segment_via_index`,
`solder_paste_segment_aperture`, `solder_paste_segment_width_mm`,
`solder_paste_segment_source_primitive`, and
`solder_paste_segment_source_primitive_index`. Stable limit keys include
`min_solder_paste_aperture_size_mm`.

`SOLDER_PASTE_APERTURE_AREA_RATIO_VALID` reports are emitted by
`manufacturing` scenarios that compare supported Gerber solder-paste openings
with a stencil release area-ratio floor. `stencil_thickness_mm` may come from
the scenario parameters or from `board.manufacturing` metadata. Stable measured keys include
`solder_paste_kind`, `solder_paste_aperture_area_mm2`,
`solder_paste_aperture_perimeter_mm`, `stencil_thickness_mm`, and
`solder_paste_aperture_area_ratio`. Feature, segment, and region findings
reuse the same `solder_paste_feature_*`, `solder_paste_segment_*`, and
`solder_paste_region_*` keys documented for solder-paste opening reports.
Stable limit keys include `min_solder_paste_aperture_area_ratio`.

`SOLDER_PASTE_IC_PIN_APERTURE_VALID` reports are emitted by `manufacturing`
scenarios that compare pad-owned Gerber solder-paste opening evidence with the
saved JLCPCB IC pin pitch aperture-width table. Stable measured keys include
`solder_paste_kind`, `solder_paste_ic_pin_aperture_width_mm`,
optional `solder_paste_ic_pin_aperture_length_mm`,
optional `owner_matched_copper_pad_length_mm`, `pin_pitch_mm`, and
`source_condition`. Owner-matched copper extension failures also report
`owner_matched_copper_feature_*` evidence. Feature, segment, and region findings reuse the same
`solder_paste_feature_*`, `solder_paste_segment_*`, and `solder_paste_region_*`
keys documented for solder-paste opening reports. Stable limit keys include
`min_solder_paste_ic_pin_aperture_width_mm` and
`max_solder_paste_ic_pin_aperture_width_mm`, plus optional
`solder_paste_ic_pin_aperture_length_mm` for source rows with an explicit
length and optional
`min_solder_paste_ic_pin_aperture_length_mm`,
`max_owner_matched_copper_pad_length_for_extension_mm`, and
`solder_paste_ic_pin_aperture_extension_per_end_mm` for source rows with a
condition-scoped copper-pad-length extension.

`SOLDER_PASTE_BGA_APERTURE_VALID` reports are emitted by `manufacturing`
scenarios that compare pad-owned Gerber solder-paste flash evidence with the
saved JLCPCB BGA pitch aperture-size table. Stable measured keys include
`solder_paste_kind`, `solder_paste_bga_aperture_size_mm`, `pin_pitch_mm`, and
`source_condition`. Feature findings reuse the same `solder_paste_feature_*`
keys documented for solder-paste opening reports. Stable limit keys include
`solder_paste_bga_aperture_size_mm`. Pitch-grid failures also report
`solder_paste_bga_feature_count`,
`solder_paste_bga_horizontal_pitch_gap_count`, and
`solder_paste_bga_vertical_pitch_gap_count`; stable limit keys include
`min_solder_paste_bga_horizontal_pitch_gap_count` and
`min_solder_paste_bga_vertical_pitch_gap_count`.

`SOLDER_PASTE_SPACING_VALID` reports are emitted by `manufacturing` scenarios
that compare same-layer Gerber solder-paste opening evidence under
`board.layout.solder_paste`. Stable measured keys include
`solder_paste_layer`, `solder_paste_spacing_mm`,
`first_solder_paste_kind`, and `second_solder_paste_kind`. Feature-opening
findings report prefixed keys such as `first_solder_paste_feature_index`,
`first_solder_paste_feature_x_mm`, `first_solder_paste_feature_y_mm`,
`first_solder_paste_feature_layer`, `first_solder_paste_feature_aperture`,
optional `first_solder_paste_feature_net`, optional
`first_solder_paste_feature_owner_kind`, optional
`first_solder_paste_feature_component`, optional
`first_solder_paste_feature_pin`, optional
`first_solder_paste_feature_via_index`,
`first_solder_paste_feature_shape`, `first_solder_paste_feature_size_x_mm`,
`first_solder_paste_feature_size_y_mm`,
`first_solder_paste_feature_source_primitive`, and
`first_solder_paste_feature_source_primitive_index`, with corresponding
`second_...` keys for the other opening. Segment-opening findings report
prefixed keys such as `first_solder_paste_segment_index`,
`first_solder_paste_segment_start`, `first_solder_paste_segment_end`,
`first_solder_paste_segment_layer`, optional
`first_solder_paste_segment_net`, optional
`first_solder_paste_segment_owner_kind`, optional
`first_solder_paste_segment_component`, optional
`first_solder_paste_segment_pin`, optional
`first_solder_paste_segment_via_index`,
`first_solder_paste_segment_aperture`,
`first_solder_paste_segment_width_mm`,
`first_solder_paste_segment_source_primitive`, and
`first_solder_paste_segment_source_primitive_index`. Region-opening findings
report prefixed keys such as `first_solder_paste_region_index`,
`first_solder_paste_region_layer`, optional `first_solder_paste_region_net`,
optional `first_solder_paste_region_owner_kind`, optional
`first_solder_paste_region_component`, optional
`first_solder_paste_region_pin`, optional
`first_solder_paste_region_via_index`,
`first_solder_paste_region_source_primitive`,
`first_solder_paste_region_source_primitive_index`, and
`first_solder_paste_region_point_count`. Stable limit keys include
`min_solder_paste_spacing_mm`.

`ASSEMBLY_FOOTPRINT_ALIGNMENT_VALID` reports are emitted by `manufacturing`
scenarios that compare JLC/EasyEDA BOM+CPL source metadata against imported
KiCad PCB footprint and placement evidence. Stable measured keys include
`reason`, `component`, and reason-specific source fields. `reason` is one of
`footprint_name_mismatch`, `part_property_mismatch`,
`placement_side_mismatch`, or `placement_rotation_mismatch`. Footprint-name
findings include `assembly_field`, `assembly_footprint`, and
`kicad_footprint_values`. Part-property findings include `assembly_field`,
`assembly_part`, `kicad_property_name`, and `kicad_property_value`. Side
findings include `assembly_side` and `layout_side`. Rotation findings include
`assembly_rotation_deg`, `layout_rotation_deg`, `rotation_delta_deg`, and limit
key `rotation_tolerance_deg`.

`PIN_1_ORIENTATION_VALID` reports are emitted by `manufacturing` scenarios that
compare an explicit expected pin-1 direction against imported KiCad footprint
`semantics.pin_1` and `semantics.body_bounds` evidence. Stable measured keys
include `component`, `pin_1_x_mm`, `pin_1_y_mm`, `pin_1_source`,
`body_center_x_mm`, `body_center_y_mm`, `body_bounds_source`,
`measured_pin_1_direction_deg`, `expected_pin_1_direction_deg`, and
`pin_1_direction_error_deg`. Stable limit keys include
`max_pin_1_direction_error_deg`.

`USB_VBUS_ROUTE_VALID` reports are emitted by `interface_protection` scenarios
that combine `usb_connector` metadata, VBUS clamp-only protection metadata,
`board.layout.placements`, and `board.layout.routes`. Stable measured keys
include `connector_signal`, `route_length_mm`, `via_count`,
`route_segment_width_mm`, `protection_component`,
`connector_to_vbus_protection_route_distance_mm`,
`protection_components_without_placement`, and
`protection_components_off_route`. When VBUS pad-contact evidence is required,
route-order findings also report `connector_pad`, `protection_pad`,
`pad_component`, `pad_pin`, `vbus_protection_pads_missing`, and
`vbus_protection_pads_off_route` as applicable. Stable limit keys include
`max_vbus_route_length_mm`, `max_vbus_via_count`,
`min_vbus_route_width_mm`,
`max_connector_to_vbus_protection_route_distance_mm`, and
`max_component_to_route_distance_mm`. Pad-contact VBUS findings additionally
include `vbus_route_pad_contact_policy`.

`USB_RETURN_PATH_VALID` reports are emitted by `interface_protection` scenarios
that combine `usb_connector` metadata, `board.layout.routes`, and same-layer
ground-zone outline evidence from `board.layout.zones`. Stable measured keys
include `connector_signal`, `unreferenced_route_length_mm`, and
`unreferenced_segments`. Each unreferenced segment entry includes
`segment_index`, `segment_length_mm`, `midpoint_x_mm`, `midpoint_y_mm`, and
`layer`. When `max_data_via_to_ground_stitch_distance_mm` is enabled, stable
measured keys also include `data_via_index`, `data_via_x_mm`, `data_via_y_mm`,
`data_via_layers`, and, when a candidate exists,
`nearest_ground_stitch_net`, `nearest_ground_stitch_via_index`, and
`nearest_ground_stitch_distance_mm`. When
`min_data_line_filled_zone_edge_clearance_mm` is enabled, stable measured keys
also include `segment_index`, `segment_length_mm`, `midpoint_x_mm`,
`midpoint_y_mm`, `layer`, and, when same-layer filled copper contains the
midpoint, `filled_zone_edge_clearance_mm`. Stable limit keys include
`max_data_line_unreferenced_length_mm`, `reference_net_kind`,
`reference_zone_geometry`, `reference_zone_layer_policy`, and
`reference_zone_contact_policy`; stitching findings additionally include
`max_data_via_to_ground_stitch_distance_mm` and
`required_ground_stitch_layer_policy`; filled-zone clearance findings
additionally include `min_data_line_filled_zone_edge_clearance_mm`.

`RESIDENT_BOOTLOADER_UPDATE_SEQUENCE` reports must include a non-blocking `ABSTRACT_PROTOCOL_TRACE` limitation because the rule validates declared transaction traces rather than raw firmware execution, raw-frame CRC recomputation, flash emulation, or HIL behavior.

`CONTROL_LINE_RELEASE_SEQUENCE` reports must include a non-blocking `ABSTRACT_CONTROL_LINE_MODEL` limitation because the rule validates declared line effects and release delays rather than transistor-level or RC waveform behavior.

`POWER_TREE_VALID` reports are emitted by `power_tree` scenarios. They fail
when active power pins are tied to non-power nets, rails are not declared
powered, nominal rail voltages are missing/invalid/outside component-model
operating ranges, declared rail current budgets are exceeded, or explicit
regulator conversion, load-switch, reset-supervisor, battery-charger, or
power-mux metadata is violated.
Stable measured keys include
`nominal_voltage_V`, `powered`, `declared_load_current_A`,
`declared_output_load_current_A`, `input_voltage_V`, `output_voltage_V`,
`dropout_margin_V`, `input_power_valid_at_us`, `output_power_valid_at_us`,
`startup_delay_us`, `declared_minimum_output_load_current_A`,
`support_capacitance_F`, `support_capacitors`, `input_inductance_H`,
`input_inductors`, `output_inductance_H`,
`output_inductors`, `switch_inductance_H`, `switch_inductors`,
`input_net`, `switch_net`, `output_net`, `switch_inductor_net_a`,
`switch_inductor_net_b`,
`input_powered`, `output_powered`, `control_state`,
`reset_supervisor_threshold_min_V`, `monitored_load_component`,
`monitored_load_pin`, `programmed_charge_current_A`,
`battery_nominal_voltage_V`, `selected_input`, `selected_input_powered`,
`inactive_input`, `inactive_input_powered`, and
`missing_load_current_metadata` depending on the
failure. Stable limit keys include `operating_voltage_minimum_V`,
`operating_voltage_maximum_V`, `powered`, `supply_current_limit_A`,
`dropout_voltage_V`, `regulator_min_output_current_A`,
`regulator_max_output_current_A`,
`earliest_output_power_valid_at_us`, `required_rail_timing_field`, and
`regulator_input_capacitance_min_F`, `regulator_output_capacitance_min_F`,
`regulator_input_inductance_min_H`, `regulator_input_inductance_max_H`,
`regulator_output_inductance_min_H`, `regulator_output_inductance_max_H`,
`regulator_switch_inductance_min_H`, `regulator_switch_inductance_max_H`,
`power_conversion_pin`, `input_pin`, `switch_pin`, `output_pin`,
`switch_inductor_pin_a`, `switch_inductor_pin_b`,
`power_conversion_field`, `control_pin`,
`required_enabled_state`,
`load_switch_max_output_current_A`, `power_switch_field`,
`reset_supervisor_threshold_max_V`, `load_operating_voltage_min_V`,
`reset_supervisor_field`,
`required_component_parameter`, `battery_charger_min_charge_current_A`,
`battery_charger_max_charge_current_A`, `input_supply_current_limit_A`,
`battery_charger_regulation_voltage_V`, `battery_charger_field`,
`programmed_charge_current_source`, `programming_resistor_component`,
`programming_resistor_ohm`, `programming_current_source`,
`selected_input_powered`, `required_reverse_blocking`, `allowed_inputs`, and
`power_mux_max_output_current_A`, `power_mux_field`.

`IO_VOLTAGE_COMPATIBLE` reports are emitted by `power_tree` scenarios that
declare the check. They compare same-net digital output/input pairs when model
metadata is present. Stable measured keys include `driver_high_voltage_V`,
`receiver_rail_voltage_V`, `source_impedance_ohm`, `diode_drop_V`, and
`injection_current_A`. Stable limit keys include `receiver_vih_min_V` and
`injection_current_A`.

`MOTOR_BRIDGE_BUDGET_VALID` reports are emitted by `motor_drive` scenarios that
declare the check. They compare explicit first-pass motor bridge budget values,
not simulated FOC behavior. Stable measured keys include
`motor_phase_peak_current_A`, `motor_phase_rms_current_A`,
`max_regen_current_A`, `motor_component`, `phase_shunt_power_W`, and
`phase_shunt_sense_voltage_V` when the corresponding comparison fails. Stable
limit keys include `bridge_reference_current_A`,
`bridge_device_current_class_A`, `motor_phase_peak_current_A`,
`motor_connector_current_rating_A`, `phase_shunt_power_rating_W`,
`min_shunt_power_margin_ratio`, and `max_shunt_sense_voltage_V`.

`MOTOR_LOAD_SUPPLY_VALID` reports are emitted by `motor_drive` scenarios that
compare a declared motor bus voltage window against selected motor supply
evidence. Stable measured keys include `bus_voltage_min_V` and
`bus_voltage_max_V`. Stable limit keys include
`motor_supply_voltage_min_V` and `motor_supply_voltage_max_V`.

`MOTOR_BRIDGE_LOSS_THERMAL_VALID` reports are emitted by `motor_drive`
scenarios that declare source-backed bridge loss/rating metadata and explicit
board thermal budget inputs. Stable measured keys include `bus_voltage_max_V`,
`motor_phase_peak_current_A`, `estimated_total_bridge_loss_W`,
`motor_phase_rms_current_A`, `reference_loss_W`, `reference_current_A`,
`loss_multiplier`, `motor_bridge_source`, and `motor_component`, depending on
which comparison fails. Stable limit keys include
`motor_bridge_voltage_rating_V`, `motor_bridge_current_rating_A`,
`max_total_bridge_loss_W`, and `min_loss_margin_ratio`.

`MOTOR_BRIDGE_SWITCHING_VALID` reports are emitted by `motor_drive` scenarios
that declare source-backed bridge gate-charge/rise/fall metadata and explicit
gate-drive/PWM budget inputs. Stable measured keys include
`estimated_total_switching_loss_W`, `bus_voltage_max_V`,
`motor_phase_peak_current_A`, `rise_time_s`, `fall_time_s`,
`pwm_frequency_Hz`, `switching_events_per_pwm_cycle`,
`average_gate_drive_current_A`, `gate_charge_total_C`,
`gate_charge_events_per_pwm_cycle`, `gate_drive_power_W`,
`gate_charge_voltage_V`, and `motor_component`, depending on which comparison
fails. Stable limit keys include `max_total_switching_loss_W`,
`min_switching_loss_margin_ratio`, and `max_average_gate_drive_current_A`.

`MOTOR_BRIDGE_SOA_VALID` reports are emitted by `motor_drive` scenarios that
declare a motor bridge and source-backed SOA metadata. System-SOA failures for
power blocks include measured keys `temperature_C`, `output_current_A`,
`current_source`, `system_soa_current_margin_ratio`,
`temperature_above_curve_range`, and `motor_component` when motor-load
evidence is used. Stable limit keys include `output_current_limit_A`,
`required_output_current_A`, `system_soa_curve`, `curve_temperature_node`,
`curve_current_kind`, `min_soa_current_margin_ratio`, `interpolation`,
`source_document`, `source_figure`, `test_conditions`,
`digitization_method`, `digitization_confidence`, and optional
`digitization_warning`. Missing or invalid SOA metadata is reported as a
critical fail-closed finding with stable measured keys `component`, `model`,
and `soa_metadata_error`, plus limit key `valid_soa_curve_required` or
`valid_system_soa_curve_required`. VDS/ID curve stress failures include
measured keys `vds_v`, `id_a`, `pulse_width_us`, `pulse_duty_cycle`,
`soa_current_margin_ratio`, `duration_covered_by_curve`,
`vds_above_curve_range`, and `motor_component`; stable limit keys include
`id_limit_a`, `required_id_a`, `soa_curve`, `curve_pulse_width_us`,
`curve_duty_cycle_max`, `min_soa_current_margin_ratio`, `interpolation`,
`source_document`, `source_figure`, `digitization_method`,
`digitization_confidence`, and optional `digitization_warning`.

`MOTOR_REGEN_CLAMP_VALID` reports are emitted by `motor_drive` scenarios that
declare a motor bridge, a named regeneration clamp/absorber component, explicit
single-event regeneration energy, bus capacitance, voltage window, and clamp
current/energy limits. Clamp current and energy limits may be explicit
scenario parameters or model-derived `regen_absorber` evidence. Stable
measured keys include `clamp_voltage_V`, `required_clamp_current_A`,
`regen_energy_J`, `bus_absorption_energy_J`, `total_absorption_energy_J`,
`motor_component`, and `clamp_component`, depending on which comparison fails.
Stable limit keys include `max_bus_voltage_V`, `clamp_current_rating_A`,
`required_absorption_energy_J`, and `min_regen_energy_margin_ratio`.

`MOTOR_ROUTE_CURRENT_VALID` reports are emitted by `motor_drive` scenarios that
declare routed motor/power nets and an explicit current-density policy. Stable
measured keys include `route_current_A`, `route_current_source`,
`min_route_width_mm`, and `motor_component` when motor-load evidence is used.
Stable limit keys include `max_current_density_A_per_mm` and
`required_route_width_mm`.

`MOTOR_CURRENT_SENSE_ACCURACY_VALID` reports are emitted by `motor_drive`
scenarios that declare shunt, gain, ADC, offset, tolerance, and error-budget
parameters. Stable measured keys include `peak_sense_output_voltage_V`,
`adc_counts_at_min_current`, `total_current_error_A`,
`quantization_error_A`, `offset_error_A`, `shunt_tolerance_error_A`,
`gain_error_A`, `adc_lsb_voltage_V`, and `motor_component`, depending on which
comparison fails. Stable limit keys include
`effective_adc_input_max_voltage_V`, `min_adc_counts_at_min_current`, and
`max_total_current_error_A`.

`MOTOR_CURRENT_SENSE_PLACEMENT_VALID` reports are emitted by `motor_drive`
scenarios that declare phase-shunt placements, paired phase routes, paired
sense routes, and explicit distance limits. Stable measured keys include
`shunt_to_reference_distance_mm`, `shunt_to_phase_route_distance_mm`,
`shunt_to_sense_route_distance_mm`, and `sense_route_length_mm`, depending on
which comparison fails. Stable limit keys include
`max_shunt_to_reference_distance_mm`,
`max_shunt_to_phase_route_distance_mm`,
`max_shunt_to_sense_route_distance_mm`, and `max_sense_route_length_mm`.

`LOAD_CONNECTOR_CURRENT_VALID` reports are emitted by `load_budget` scenarios
that declare the check. They compare one load power-pin current budget against
an explicit connector current rating or a connector model rating. Stable
measured keys include `connector_component`, `load_net`, and `load_current_A`.
Stable limit keys include `required_connector_current_A`,
`connector_current_rating_A`, `min_connector_current_margin_ratio`, and
`connector_voltage_rating_V` when voltage screening fails.

`MODEL_QUALITY_REQUIRED` reports are emitted by `model_quality` scenarios. They
compare named board components against an explicit sign-off policy for
`model_quality.source` and `model_quality.confidence`. Stable measured keys
include `model`, `model_source`, `model_confidence`, `missing_input`, and
`missing_component`. Stable limit keys include `allowed_sources` and
`min_confidence`.

`ANALOG_MODEL_COMPILER_PROVENANCE_MISSING`,
`ANALOG_MODEL_SOURCE_UNAVAILABLE`, and
`ANALOG_MODEL_SOURCE_HASH_MISMATCH` reports are emitted by analog scenarios
before solver execution when `analog.model_files[]` declares incomplete or
stale OpenVAF/OSDI source metadata.
`ANALOG_MODEL_COMPILER_COMMAND_MISMATCH`,
`ANALOG_MODEL_COMPILER_ARTIFACT_UNAVAILABLE`, and
`ANALOG_MODEL_COMPILER_ARTIFACT_HASH_MISMATCH` reports are emitted when the
declared OpenVAF command does not reference the pinned source/output, the OSDI
artifact is missing, or the compiled artifact hash is stale.
`ANALOG_MODEL_COMPILER_BACKEND_UNSUPPORTED` is emitted when an OSDI shared
object is paired with a backend that has no trusted OSDI loading contract, such
as explicit `backend: xyce` or `backend: embedded_ngspice`.
`ANALOG_MODEL_COMPILER_XYCE_PLUGIN_UNSUPPORTED` is emitted when a
`xyce_adms_plugin` entry has pinned source, plugin, and conformance artifacts
but CircuitCI has not yet enabled a real-Xyce `-plugin` loader and conformance
path.
`ANALOG_MODEL_PACKAGE_LOCK_MISSING`,
`ANALOG_MODEL_PACKAGE_LOCK_UNAVAILABLE`,
`ANALOG_MODEL_PACKAGE_LOCK_HASH_MISMATCH`,
`ANALOG_MODEL_PACKAGE_LOCK_INVALID`, and
`ANALOG_MODEL_PACKAGE_LOCK_ARTIFACT_MISMATCH` are emitted when reusable
compact-model package metadata is incomplete, the JSON/YAML lock file is
missing or stale, or the lock does not contain the exact package/artifact row
declared by `analog.model_files[]`.
`ANALOG_MODEL_PACKAGE_REGISTRY_MISSING`,
`ANALOG_MODEL_PACKAGE_REGISTRY_UNAVAILABLE`,
`ANALOG_MODEL_PACKAGE_REGISTRY_HASH_MISMATCH`,
`ANALOG_MODEL_PACKAGE_REGISTRY_INVALID`, and
`ANALOG_MODEL_PACKAGE_REGISTRY_ENTRY_MISMATCH` are emitted when a reusable
compact-model package registry is partially declared, missing, stale,
malformed, lacks the selected entry, or conflicts with explicit scenario
metadata.
`ANALOG_MODEL_COMPILER_BUILD_FAILED` is emitted only when
`CIRCUITCI_RUN_OPENVAF_BUILDS=1` opts into direct OpenVAF execution and the
declared command fails. Stable measured keys include `model_file`,
`source_path`, `compiler`, `compiler_version`, `compiler_command`,
`compiler_available_on_path`, `artifact_format`, `requested_backend`,
`plugin_load_command`, `xyce_version`, `xyce_adms_template_revision`,
`xyce_configure_options`, `conformance_artifact`, `model_package_name`,
`model_package_version`, `model_package_artifact_id`,
`model_package_lock_path`, `model_package_lock_sha256`,
`model_package_registry_path`, `model_package_registry_sha256`,
`model_package_registry_entry`, `research_evidence`,
optional `stdout`/`stderr` prefixes for failed builds, and `sha256` for hash
mismatches. Stable limit keys include `required_field`, `required_artifact`,
`required_build_step`, `required_backend_adapter`, `required_conformance`,
`required_configure_option`, `mismatch`, `source_path`, `model_file`,
`output_path`, `expected_sha256`, `supported_backend`, and
`xyce_required_model_path`. These
findings keep Verilog-A compact-model use
fail-closed until the compiled OSDI artifact is tied to source path/hash,
OpenVAF compiler identity/version, reproducible compiler command metadata, and
a backend with a documented loading contract. For external ngspice transient
runs that reach solver execution, OSDI artifacts are loaded through generated
`pre_osdi` wrapper commands. If the runtime rejects those commands or cannot
load the OSDI artifact, the normal `SPICE_TRANSIENT_ANALYSIS` finding preserves
the wrapper and log artifacts and its message identifies OSDI model loading as
the failing boundary. Xyce does not load OpenVAF `*.osdi` artifacts; its
documented Verilog-A path is Xyce/ADMS-generated C++ linked into Xyce or loaded
as a Xyce plugin from a shareable Xyce build. Successful solver manifests include
`inputs.model_file_provenance[]` records for OpenVAF/OSDI artifacts. Stable
fields include `model_file`, `artifact_format`, `source_path`,
`source_sha256_declared`, `source_sha256_actual`,
`artifact_sha256_declared`, `artifact_sha256_actual`, `compiler`,
`compiler_version`, `compiler_command`, `compiler_available_on_path`,
`build_env_enabled`, `rebuild_mode`, `produced_by_circuitci`, and optional
package-lock fields `model_package_name`, `model_package_version`,
`model_package_artifact_id`, `model_package_lock_path`, and
`model_package_lock_sha256`, plus optional registry fields
`model_package_registry_path`, `model_package_registry_sha256`, and
`model_package_registry_entry`. Reports also project those records into top-level
`model_file_provenance[]` entries with the solver `scenario`, `analysis`,
`backend`, and `manifest` path, so report consumers can inspect compiled-model
provenance without opening `solver_manifest.json` first. Markdown reports
include the same information in the "Model File Provenance" section.
Standalone compact-model package preflights use
`schemas/model_package_verification_report.schema.json`, written by
`circuitci verify-model-package`. That report records lock and optional registry
hashes, per-artifact hash status, projected `conformance_checks[]`, and stable
`MODEL_PACKAGE_*` finding ids. The command also writes a sibling Markdown
summary. Normal validation reports project any retained package-verification
JSON artifact into top-level `model_package_conformance_checks[]`, and the GUI
Simulation report panel displays the same compact rows.
When a lock includes an artifact with
`artifact_format: model_conformance_report`, the verifier also validates that
document against `schemas/model_conformance_report.schema.json` and requires it
to bind to a package artifact id, runtime artifact SHA-256, matching package
identity, an overall `pass` result, and passing check rows. Each projected
conformance check records the report artifact id/path, target artifact id and
SHA-256, check name, analysis, optional solver, result, and referenced check
artifacts.
Portable compact-model package bundles use
`schemas/model_package_bundle_manifest.schema.json`, written by
`circuitci export-model-package-bundle`. The manifest records the bundled lock
and optional registry hashes, verification report paths, copied package
artifacts, and projected conformance checks so a directory-level package can be
reviewed before any scenario imports it. Bundle preflights use
`schemas/model_package_bundle_verification_report.schema.json`, written by
`circuitci verify-model-package-bundle`. That report records manifest, lock,
registry, README, package-verification, copied-artifact hash status, projected
conformance checks, and stable `MODEL_PACKAGE_BUNDLE_*` findings. Bundle
install reports use `schemas/model_package_bundle_install_report.schema.json`,
written by `circuitci install-model-package-bundle`. The install report records
source and installed bundle paths, installed registry hash status when
`--registry-output` is used, and a `scenario_import` object containing the
registry path, registry SHA-256, registry entry, lock path, lock SHA-256, and
artifact id needed in `analog.model_files[]`.

`POWER_SWITCH_BUDGET_VALID` reports are emitted by `load_budget` scenarios that
declare a selected power-switch budget. Stable measured keys include
`load_component`, `load_current_A`, `switch_output_net`, `load_net`,
`input_net_voltage_V`, `output_net_voltage_V`, `thermal_current_A`,
`on_resistance_ohm`, `conduction_loss_W`, and
`estimated_junction_temperature_C` when those branches run. Stable limit keys
include `required_switch_current_A`, `switch_max_output_current_A`,
`required_current_limit_A`, `switch_current_limit_A`,
`input_pin_operating_voltage_max_V`, `output_pin_operating_voltage_max_V`,
`ambient_temperature_C`, `max_junction_temperature_C`,
`max_junction_temperature_margin_C`, and
`thermal_resistance_junction_to_ambient_C_per_W`.

`POWER_SWITCH_REVERSE_CURRENT_VALID` reports are emitted by `load_budget`
scenarios that require a selected switch to block e-stop rail backfeed. Stable
measured keys include `reverse_current_blocking_mode`, `switch_output_net`, and
`load_net`. Stable limit keys include
`reverse_current_blocking_mode_required`.

`POWER_SWITCH_INRUSH_VALID` reports are emitted by `load_budget` scenarios that
declare a switched-rail soft-start/inrush screen. Stable measured keys include
`load_component`, `load_net`, `load_voltage_V`, `switched_capacitance_F`,
`soft_start_time_us`, and `estimated_inrush_current_A`. Stable limit keys
include `required_inrush_current_A`, `switch_max_inrush_current_A`, and
`min_inrush_current_margin_ratio`.

`LOAD_CABLE_CURRENT_VALID` reports are emitted by `load_budget` scenarios that
declare a cable assembly current screen. Stable measured keys include
`cable_component`, `load_net`, `load_current_A`, and `load_voltage_V` when
voltage screening fails. Stable limit keys include
`required_cable_current_A`, `cable_current_rating_A`,
`min_cable_current_margin_ratio`, and `cable_voltage_rating_V`.

`LOAD_CABLE_THERMAL_DERATING_VALID` reports are emitted by `load_budget`
scenarios that declare a cable assembly thermal-rise screen. Stable measured
keys include `cable_component`, `load_net`, `load_current_A`,
`thermal_current_A`, `temperature_rise_test_current_A`,
`temperature_rise_at_test_current_C`, and `estimated_temperature_rise_C`.
Stable limit keys include `max_cable_temperature_rise_C` and
`thermal_current_margin_ratio`.

`LOAD_CABLE_VOLTAGE_DROP_VALID` reports are emitted by `load_budget` scenarios
that declare a cable assembly voltage-drop screen. Stable measured keys include
`cable_component`, `load_net`, `load_current_A`, `drop_current_A`,
`cable_loop_resistance_ohm`, `estimated_voltage_drop_V`, and
`estimated_power_loss_W`. Stable limit keys include
`max_cable_voltage_drop_V`, optional `max_cable_power_loss_W`, and
`drop_current_margin_ratio`.

`FUNCTIONAL_MCU_FIRMWARE` reports are emitted by `firmware_in_loop` scenarios.
For QEMU-backed scenarios, a pass requires successful QEMU execution plus
matching `CIRCUITCI_PIN` observations for every declared expected board-facing
pin state. If `firmware.build` is declared, the build must complete and every
declared output must exist before QEMU starts. Missing backend configuration,
missing firmware images, build failures, missing build outputs, QEMU launch or
timeout failures, malformed traces, conflicting observations, and pin
mismatches fail closed under this rule. Stable measured keys include
`target_component`, `target_model`, `backend`, `firmware_image`, optional
`machine`, and `expected_pin_states`; build/QEMU log-write failures may include
`artifact_error`; pin mismatches also include
`pin_component`, `pin`, `observed_mode`, and `observed_state`. Stable limit
keys include `functional_blackbox_boundary`,
`transistor_level_mcu_required: false`, and, for mismatches, `expected_mode`
and `expected_state`. QEMU scenarios include a `qemu.log` artifact when the
artifact directory can be created; scenarios with declared builds also include
`firmware_build.log` and declared build outputs as artifacts. This rule is for
functional firmware execution and MCU pin behavior; it must not imply
transistor-level MCU simulation.

`SPICE_OPERATING_LIMIT` reports are emitted by physical analog validation when
generated Board IR MOSFET/BJT/diode operating probes exceed datasheet absolute
maximum ratings. Stable measured keys include `component`, `rating`,
`quantity`, `expression`, `max_abs`, `time_of_max_us`, and `unit`; stable limit
keys include `rating`, `rating_value`, `max_abs`, `effective_limit`, and
`unit`. `rating_value` preserves the signed datasheet value while `max_abs` and
`effective_limit` are the comparison limit after any scenario derating.
Temperature-aware findings also include `scenario_temperature_c`,
`derate_above_c`, and `derating_per_c`. Pulse-aware current findings include
`pulse_duration_us`, `pulse_duty_cycle`, `pulse_rating`,
`pulse_rating_value`, `pulse_max_abs`, `pulse_width_us`, and
`pulse_duty_cycle_max` when pulse metadata was considered. If a generated
semiconductor model lacks the required absolute-maximum metadata, the same rule
id is emitted with measured `component`, `model`, `quantity`,
`missing_rating`, and `unit` keys. Missing derating metadata uses
`temperature_derating_required`; missing pulse qualifiers use
`pulse_width_and_duty_required`.

Digitized SOA findings also use `SPICE_OPERATING_LIMIT` with measured
`rating: SOA`, `vds_v`, `id_a`, `time_us`, `soa_margin_ratio`,
`pulse_duration_us`, `pulse_duty_cycle`, and flags for curve range and duration
coverage. Stable SOA limit keys include `id_limit_a`, `soa_curve`,
`curve_pulse_width_us`, `curve_duty_cycle_max`, `interpolation: log_log`,
`source_document`, `source_figure`, `digitization_method`,
`digitization_confidence`, and optional `digitization_warning`.

Declared executable checks with missing required inputs must produce a critical `VALIDATION_INPUT_MISSING` finding so the report cannot pass by skipping validation.

## Markdown Report

Markdown reports must include:

1. Executive summary.
2. Pass/fail table.
3. Critical failures.
4. Warnings.
5. Suggested fixes.
6. Unmodeled or low-confidence areas.
7. Reproduction command.
