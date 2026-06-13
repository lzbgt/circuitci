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

## Runtime Behavior

The CLI accepts `--profile iot_basic_v0` and runs checks declared by the
project scenario list. Scenario declarations remain the executable source of
truth: the profile does not synthesize scenarios, import missing evidence, or
turn scenario suggestions into validation evidence.

When `iot_basic_v0` is requested, reports also include a non-blocking
`PROFILE_COVERAGE_PARTIAL` limitation if core profile checks are not declared.
This preserves the schema `pass`/`fail` contract while making incomplete
profile coverage explicit for sign-off review. The core coverage set is:

- `POWER_TREE_VALID`
- `RESET_RELEASE_AFTER_POWER_VALID`
- `BOOT_STRAP_DEFINED`
- `GPIO_BACKDRIVE`
- `UART_BOOTLOADER_SYNC`

`iot_basic_v0` recognizes these executable checks when project scenarios
declare them:

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
- `USB_CONNECTOR_COMPONENT_CLEARANCE_VALID`
- `USB_CONNECTOR_ENTRY_CLEARANCE_VALID`
- `USB_ROUTE_GEOMETRY_VALID`
- `USB_VBUS_ROUTE_VALID`
- `USB_RETURN_PATH_VALID`
- `CLOCK_SOURCE_VALID`
- `POWER_TREE_VALID`
- `DRILL_DIAMETER_VALID`
- `DRILL_TO_BOARD_EDGE_CLEARANCE_VALID`
- `SLOT_TO_BOARD_EDGE_CLEARANCE_VALID`
- `SLOT_WIDTH_VALID`
- `SLOT_ASPECT_RATIO_VALID`
- `CASTELLATED_HOLE_VALID`
- `DRILL_ANNULAR_RING_VALID`
- `COPPER_TO_BOARD_EDGE_CLEARANCE_VALID`
- `COPPER_SPACING_VALID`
- `SOLDER_MASK_OPENING_VALID`
- `SOLDER_MASK_DAM_VALID`
- `SOLDER_PASTE_OPENING_VALID`
- `SOLDER_PASTE_APERTURE_SIZE_VALID`
- `SOLDER_PASTE_APERTURE_AREA_RATIO_VALID`
- `SOLDER_PASTE_IC_PIN_APERTURE_VALID`
- `SOLDER_PASTE_BGA_APERTURE_VALID`
- `SOLDER_PASTE_SPACING_VALID`
- `IO_VOLTAGE_COMPATIBLE`
- `SPICE_TRANSIENT_ANALYSIS`

`circuitci suggest-scenarios` can propose missing `power_tree`, reset timing,
boot-strap, UART bootloader, GPIO backdrive, and interface-protection scenario
templates before profile validation. Suggestions are not automatically treated
as passing validation evidence.

Use `circuitci suggest-scenarios --profile iot_basic_v0` to append remediation
templates for missing core profile checks. These profile remediation templates
are non-runnable until the required scenario evidence is supplied, unless the
normal evidence-driven suggestion pass already produced a complete suggestion
for that check.

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
