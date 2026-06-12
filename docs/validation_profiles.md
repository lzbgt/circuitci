# Validation Profiles

A validation profile is a reusable set of scenarios and pass criteria.

## iot_basic_v0

```yaml
profile: iot_basic_v0
scenarios:
  - power_up
  - power_down
  - power_tree
  - usb_hot_plug
  - reset_boot
  - serial_programming
  - gpio_backdrive
  - interface_protection
  - i2c_bus
  - spi_bus
  - uart
  - sleep_current
  - brownout
pass_criteria:
  no_critical_electrical_limit_violation: true
  no_unknown_power_domain: true
  no_unresolved_component_model_for_critical_path: true
  reset_release_after_vdd_valid: true
  boot_straps_defined_during_sampling: true
  no_gpio_backdrive_above_default_limit: true
  programming_interface_valid_if_declared: true
```

## MVP Behavior

The CLI accepts `--profile iot_basic_v0` and runs checks declared by the project scenario list. The profile is report metadata plus future policy until profile expansion is implemented. Missing future-profile checks are reported as limitations only when the project declares them.

For this stage, `iot_basic_v0` recognizes these executable checks when project scenarios declare them:

- `GPIO_BACKDRIVE`
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

`circuitci suggest-scenarios` can propose missing `power_tree`, reset timing,
boot-strap, UART bootloader, GPIO backdrive, and interface-protection scenario
templates before profile validation. Suggestions are not automatically treated
as passing validation evidence.

## Rule Completion Standard

A validation rule is complete only when it has:

- stable rule ID
- deterministic pass/fail condition
- at least one passing fixture
- at least one failing fixture
- expected severity
- suggested fix class
- JSON report documentation
- human report documentation
