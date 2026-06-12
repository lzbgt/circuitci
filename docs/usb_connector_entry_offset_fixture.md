# USB Connector Entry Offset Fixture

`examples/bad_usb_connector_entry_clearance_model_offset/` proves that
`USB_CONNECTOR_ENTRY_CLEARANCE_VALID` can derive cable insertion direction from
component-model metadata instead of assuming imported footprint rotation is the
entry direction.

The fixture connector model declares:

```yaml
usb_connector:
  entry_direction_offset_deg: 90.0
```

The board places `J1` at `rotation_deg: 270.0`, so the computed default entry
direction is `0.0` degrees. The resistor footprint sits in that forward entry
corridor and the validation must fail. Without the model offset, the check
would look toward `270.0` degrees and miss the obstruction.

`project_suggestions.yaml` omits scenarios so `suggest-scenarios` must also
emit a `USB_CONNECTOR_ENTRY_CLEARANCE_VALID` template with
`entry_direction_deg: 0.0`, `entry_direction_source: component_model_offset`,
and `entry_direction_offset_deg: 90.0`.

The suggestion fixture also includes a simple board edge whose outward normal
is `0.0` degrees. `suggest-scenarios` must infer
`expected_connector_rotation_deg: 270.0` for
`USB_CONNECTOR_ORIENTATION_VALID`, because the connector footprint rotation
should equal cable-entry direction minus the model's `90.0` degree offset.
