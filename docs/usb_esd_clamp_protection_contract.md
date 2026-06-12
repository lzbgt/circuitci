# USB ESD Clamp Protection Contract

`INTERFACE_PROTECTION_REVIEW` supports clamp-only protection devices through
`signal_conditioning.protection_clamps`. This is intended for USB ESD arrays and
similar parts that do not translate between two powered domains.

Example model metadata:

```yaml
signal_conditioning:
  protection_clamps:
    - name: dp
      protected_pin: DP
      reference_pin: GND
      reference: ground
      working_voltage_max_V: 5.5
      line_capacitance_F: 1.0e-12
```

Example scenario:

```yaml
scenarios:
  - name: usb_dp_esd_review
    type: interface_protection
    checks:
      - INTERFACE_PROTECTION_REVIEW
    target:
      component: UESD
    parameters:
      clamp: dp
      max_line_capacitance_F: 2.0e-12
```

The rule checks:

- the protected pin and reference pin are connected,
- the reference pin connects to the declared reference net kind,
- protected-net `nominal_voltage` does not exceed `working_voltage_max_V`,
- declared `line_capacitance_F` fits scenario `max_line_capacitance_F`.

This is static board-validation evidence. It does not prove ESD pulse behavior,
dynamic clamp current, USB eye margin, trace impedance, return path quality, or
connector-layout correctness.

`circuitci suggest-scenarios` emits clamp review templates automatically for
connected models with `signal_conditioning.protection_clamps`. The suggestions
include `parameters.clamp` and `scenario.protection_clamps[]` evidence, but
agents still need to fill `parameters.max_line_capacitance_F` from the actual
interface budget when capacitance screening is part of the sign-off.

Current fixtures:

- `examples/good_usb_esd_protection`
- `examples/bad_usb_esd_reference`
- `examples/bad_usb_esd_standoff`
- `examples/bad_usb_esd_line_capacitance`
- `examples/good_usb_connector_protection`
- `examples/bad_usb_connector_missing_data_protection`
- `examples/bad_usb_connector_missing_vbus_protection`

Connector-level validation:

- `USB_CONNECTOR_PROTECTION_VALID` targets a connector component whose model
  declares `usb_connector` pins.
- The rule requires clamp-only protection on D+ and D-.
- VBUS protection is required when the scenario declares
  `parameters.require_vbus_protection: true`.
- `parameters.data_working_voltage_min_V` and
  `parameters.vbus_working_voltage_min_V` optionally enforce minimum clamp
  reverse-standoff voltage.
- This check proves schematic coverage only. It does not prove ESD pulse
  robustness, connector placement, shield strategy, return-path quality, trace
  impedance, or USB eye margin.

Datasheet-backed model pack:

- `libs/vendor/ti/protection/tpd2eusb30.model.yaml`
- `docs/ti_tpd2eusb30_model.md`
- `examples/good_ti_tpd2eusb30_usb_esd`
- `examples/bad_ti_tpd2eusb30_usb_esd_standoff`
- `examples/bad_ti_tpd2eusb30_usb_esd_capacitance`
- `libs/vendor/nexperia/protection/prtr5v0u2x.model.yaml`
- `docs/nexperia_prtr5v0u2x_model.md`
- `examples/good_nexperia_prtr5v0u2x_usb_esd`
- `examples/bad_nexperia_prtr5v0u2x_usb_esd_reference`
- `examples/bad_nexperia_prtr5v0u2x_usb_esd_capacitance`
- `examples/import_kicad_prtr5v0u2x_usb_esd_suggestions`
