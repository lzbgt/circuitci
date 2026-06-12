# USB Connector Entry Aperture Fixture

`examples/bad_usb_connector_entry_clearance_aperture/` proves that
`USB_CONNECTOR_ENTRY_CLEARANCE_VALID` can use connector-model aperture geometry
instead of only centering the cable-entry corridor on the footprint placement.

The fixture connector model declares:

```yaml
usb_connector:
  entry_aperture_front_offset_mm: 0.25
  entry_aperture_lateral_offset_mm: 1.0
  entry_aperture_width_mm: 0.5
```

The connector footprint front is at `x = 0.5 mm`, so the checked aperture front
is `x = 0.75 mm`. The aperture centerline is shifted to `y = 1.0 mm`, and the
effective checked width is widened from the scenario's `0.2 mm` cable width to
the model's `0.5 mm` aperture width. The resistor courtyard is outside the old
placement-centered corridor but inside this aperture corridor, so validation
must fail and report `entry_aperture_source: component_model_aperture`.
