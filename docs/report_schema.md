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

Projects using `generic`, `estimated`, or `low` confidence component models must include non-blocking `LOW_CONFIDENCE_MODEL` limitations scoped to the component and model.

## Additional Rule Findings

Reset/boot/download rules use the same finding object. Required IDs:

- `RESET_RELEASE_AFTER_POWER_VALID`
- `BOOT_STRAP_DEFINED`
- `BOOT_STRAP_BIAS_VALID`
- `UART_BOOTLOADER_SYNC`
- `RESIDENT_BOOTLOADER_UPDATE_SEQUENCE`
- `CONTROL_LINE_RELEASE_SEQUENCE`
- `FUNCTIONAL_MCU_FIRMWARE`
- `INTERFACE_PROTECTION_REVIEW`
- `USB_CONNECTOR_PROTECTION_VALID`
- `USB_PROTECTION_PLACEMENT_VALID`
- `USB_CONNECTOR_ORIENTATION_VALID`
- `USB_CONNECTOR_EDGE_PROXIMITY_VALID`
- `USB_CONNECTOR_BODY_OVERHANG_VALID`
- `USB_ROUTE_GEOMETRY_VALID`
- `USB_VBUS_ROUTE_VALID`
- `USB_RETURN_PATH_VALID`
- `CLOCK_SOURCE_VALID`
- `POWER_TREE_VALID`
- `IO_VOLTAGE_COMPATIBLE`
- `SPICE_TRANSIENT_ANALYSIS`
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
`drill_layer`, optional `drill_tool`, optional `source_hit_index`, optional
`drill_owner_kind`, optional `drill_net`, optional `drill_component`, optional
`drill_pin`, and optional `drill_via_index`. Stable limit keys include
`min_drill_diameter_mm` and `max_drill_diameter_mm`.

`DRILL_TO_BOARD_EDGE_CLEARANCE_VALID` reports are emitted by `manufacturing`
scenarios that combine `board.layout.drills` evidence with
`board.layout.outline.segments`. Stable measured keys include `drill_index`,
`drill_x_mm`, `drill_y_mm`, `drill_mm`, `drill_radius_mm`, `clearance_mm`,
`center_to_board_edge_distance_mm`, `drill_plating`, optional `drill_layer`,
optional `drill_tool`, optional `source_hit_index`, optional
`drill_owner_kind`, optional `drill_net`, optional `drill_component`, optional
`drill_pin`, optional `drill_via_index`, `board_edge_start`, `board_edge_end`,
optional `board_edge_layer`, optional
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

`SOLDER_PASTE_IC_PIN_APERTURE_VALID` reports are emitted by `manufacturing`
scenarios that compare pad-owned Gerber solder-paste opening evidence with the
saved JLCPCB IC pin pitch aperture-width table. Stable measured keys include
`solder_paste_kind`, `solder_paste_ic_pin_aperture_width_mm`,
`pin_pitch_mm`, and `source_condition`. Feature, segment, and region findings
reuse the same `solder_paste_feature_*`, `solder_paste_segment_*`, and
`solder_paste_region_*` keys documented for solder-paste opening reports.
Stable limit keys include
`min_solder_paste_ic_pin_aperture_width_mm` and
`max_solder_paste_ic_pin_aperture_width_mm`.

`SOLDER_PASTE_BGA_APERTURE_VALID` reports are emitted by `manufacturing`
scenarios that compare pad-owned Gerber solder-paste flash evidence with the
saved JLCPCB BGA pitch aperture-size table. Stable measured keys include
`solder_paste_kind`, `solder_paste_bga_aperture_size_mm`, `pin_pitch_mm`, and
`source_condition`. Feature findings reuse the same `solder_paste_feature_*`
keys documented for solder-paste opening reports. Stable limit keys include
`solder_paste_bga_aperture_size_mm`.

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
`selected_input_powered`, `required_reverse_blocking`, `allowed_inputs`, and
`power_mux_max_output_current_A`, `power_mux_field`.

`IO_VOLTAGE_COMPATIBLE` reports are emitted by `power_tree` scenarios that
declare the check. They compare same-net digital output/input pairs when model
metadata is present. Stable measured keys include `driver_high_voltage_V`,
`receiver_rail_voltage_V`, `source_impedance_ohm`, `diode_drop_V`, and
`injection_current_A`. Stable limit keys include `receiver_vih_min_V` and
`injection_current_A`.

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
