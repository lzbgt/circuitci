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
`board_edge_end_y_mm`, and optional `board_edge_layer`. Stable limit keys include
`max_connector_to_board_edge_distance_mm`.

`USB_CONNECTOR_BODY_OVERHANG_VALID` reports are emitted by
`interface_protection` scenarios that combine `usb_connector` metadata,
`board.layout.placements`, straight `board.layout.outline.segments`, and
connector `fabrication` or `courtyard` footprint graphics. Stable measured keys
include `connector_body_overhang_mm`, `connector_edge_reference`, optional
`footprint_graphic_layer`, optional `footprint_graphic_kind`,
`board_edge_start_x_mm`, `board_edge_start_y_mm`, `board_edge_end_x_mm`,
`board_edge_end_y_mm`, optional `board_edge_layer`, `edge_angle_deg`, and
`outward_normal_deg`. Stable limit keys include
`max_connector_body_overhang_mm`.

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
`support_capacitance_F`, `support_capacitors`,
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
`power_conversion_pin`, `power_conversion_field`, `control_pin`,
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
