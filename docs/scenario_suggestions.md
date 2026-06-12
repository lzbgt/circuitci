# Scenario Suggestions

`circuitci suggest-scenarios` inspects a Board IR project and writes an
agent-facing YAML artifact with validation scenarios that should be added or
completed.
The artifact is validated by
`schemas/scenario_suggestion_report.schema.json`.

```bash
circuitci suggest-scenarios project.yaml --output out/scenario_suggestions.yaml
```

The command is conservative:

- It emits runnable `power_tree` suggestions when a project declares power nets
  but has no `POWER_TREE_VALID` scenario.
- It emits runnable `IO_VOLTAGE_COMPATIBLE` suggestions when same-net digital
  output/input pairs have modeled I/O voltage metadata and no existing
  `power_tree` scenario declares that check.
  The suggestion includes `scenario.paths[]` entries with the implicated
  driver, receiver, and net so agents can inspect the exact interfaces the
  static rule will scan.
- It emits reset templates when a component model declares reset behavior, the
  reset pin is connected, and the target power rail declares `power_valid_at_us`.
- Reset suggestions are marked `runnable: false` until real
  `timing.reset_release_at_us` evidence is filled from a reset supervisor, RC
  model, control-line model, firmware/host trace, or analog waveform.
- It emits GPIO backdrive templates when a powered output-capable pin shares a
  net with an unpowered input-capable pin, model electrical metadata is present,
  and no existing `GPIO_BACKDRIVE` scenario covers that driver/victim path.
- GPIO backdrive templates are marked `runnable: false` until the agent confirms
  the driver can be high while the victim rail is unpowered and fills the actual
  protection-path series resistance.
- It emits interface-protection templates for component models that declare
  `signal_conditioning.channels`, such as level shifters, protection devices,
  series resistors, or bus switches.
- Interface-protection templates are marked `runnable: false`; they are review
  prompts for datasheet direction, voltage-domain, enable/OE, and
  unpowered-isolation evidence.
- It emits boot-strap templates when model boot modes declare required straps
  and the strap pins are connected.
- It emits runnable `BOOT_STRAP_BIAS_VALID` templates when required strap pins
  have explicit resistor bias evidence to declared power or ground nets.
  Imported KiCad schematics can provide this automatically when pull resistors
  are mapped as SPICE resistors with `value_ohm_from: schematic_value`; see
  `examples/import_kicad_bootstrap_bias_suggestions/`.
- It emits UART bootloader templates when model bootloader metadata declares a
  UART interface. If an output-capable sender pin is already wired to the target
  RX net, the template includes that sender; otherwise it records the missing
  sender as required input.
- It never invents boot strap states, reset-release timestamps, power-good
  delays, GPIO pin-state observations, protection-path resistance, strap
  current budgets, or SPICE assertions.

Example output shape:

```yaml
schema_version: 0.1.0
project: scenario_suggestions_power_reset
suggestions:
  - id: power_tree_valid
    kind: power_tree
    confidence: high
    runnable: true
    reason: Project declares power nets but no POWER_TREE_VALID scenario.
    scenario:
      name: scenario_suggestions_power_reset_power_tree
      type: power_tree
      checks:
        - POWER_TREE_VALID
  - id: io_voltage_compatible
    kind: power_tree
    confidence: medium
    runnable: true
    reason: Project has same-net digital output/input pairs with modeled I/O voltage metadata but no IO_VOLTAGE_COMPATIBLE check.
    scenario:
      name: scenario_suggestions_power_reset_io_voltage
      type: power_tree
      checks:
        - IO_VOLTAGE_COMPATIBLE
      paths:
        - driver:
            component: U1
            pin: TX
          victim:
            component: U2
            pin: RXD
          net: uart_mcu_tx
          series_resistance_ohm: 0
        - driver:
            component: U2
            pin: TXD
          victim:
            component: U1
            pin: RX
          net: uart_mcu_rx
          series_resistance_ohm: 0
  - id: reset_release_after_power_valid_u1
    kind: reset_boot
    confidence: medium
    runnable: false
    reason: Component U1 has reset behavior and target rail power_valid_at_us, but no RESET_RELEASE_AFTER_POWER_VALID scenario.
    scenario:
      name: u1_reset_release_after_power
      type: reset_boot
      checks:
        - RESET_RELEASE_AFTER_POWER_VALID
      target:
        component: U1
        power_pin: VDD
        reset_pin: NRST
      timing:
        power_valid_at_us: 1500
        reset_release_delay_us: 0
    required_inputs:
      - Fill timing.reset_release_at_us from reset supervisor, RC, control-line, or analog waveform evidence before validation.
      - Keep timing.power_valid_at_us equal to the target rail power_valid_at_us or remove duplicated stale timing.
  - id: boot_strap_defined_u1_bootloader
    kind: reset_boot
    confidence: medium
    runnable: false
    reason: Component U1 model declares boot mode bootloader, but no BOOT_STRAP_DEFINED scenario covers it.
    scenario:
      name: u1_boot_straps_bootloader
      type: reset_boot
      checks:
        - BOOT_STRAP_DEFINED
      target:
        component: U1
      required_boot_mode: bootloader
      straps:
        - component: U1
          pin: BOOT0
          net: boot0
    required_inputs:
      - Fill strap actual states for boot mode bootloader: U1.BOOT0=high.
  - id: gpio_backdrive_u2_txd_to_u1_rx
    kind: gpio_backdrive
    confidence: medium
    runnable: false
    reason: Powered output U2.TXD shares net uart_rx with unpowered input U1.RX, but no GPIO_BACKDRIVE scenario covers that path.
    scenario:
      name: u2_to_u1_backdrive
      type: gpio_backdrive
      checks:
        - GPIO_BACKDRIVE
      pin_states:
        - component: U2
          pin: TXD
          mode: output
          state: high
        - component: U1
          pin: RX
          mode: input
      paths:
        - driver: { component: U2, pin: TXD }
          victim: { component: U1, pin: RX }
          net: uart_rx
          series_resistance_ohm: 0
    required_inputs:
      - Confirm the driver can be high while the victim rail is unpowered, using firmware, host, reset-state, or hot-plug evidence.
      - Fill paths[].series_resistance_ohm from the schematic protection path; keep 0 only when there is no series resistor, switch, or protection element.
  - id: interface_protection_u3_ch1
    kind: interface_protection
    confidence: medium
    runnable: false
    reason: Component U3 model declares signal-conditioning channel ch1, but no interface protection review scenario covers it.
    scenario:
      name: u3_ch1_interface_protection
      type: interface_protection
      checks:
        - INTERFACE_PROTECTION_REVIEW
      target:
        component: U3
      conditioning:
        component: U3
        channel: ch1
        kind: level_shifter
        side_a:
          pin: A1
          net: mcu_rx_shifted
          supply_pin: VCCA
          supply_net: mcu_3v3
        side_b:
          pin: B1
          net: usb_uart_tx
          supply_pin: VCCB
          supply_net: usb_uart_3v3
        direction: bidirectional
        unpowered_isolation: false
    required_inputs:
      - Confirm the signal-conditioning part datasheet supports this direction, voltage range, and unpowered-side behavior.
      - Fill enable/OE/reset-state evidence when the part can disconnect or leave either side high impedance.
      - Add analog_transient or GPIO_BACKDRIVE scenarios for any datasheet condition that does not guarantee isolation.
  - id: uart_bootloader_sync_u1_uart
    kind: serial_programming
    confidence: medium
    runnable: false
    reason: Component U1 model declares bootloader interface uart, but no UART_BOOTLOADER_SYNC scenario covers it.
    scenario:
      name: u1_uart_bootloader_sync
      type: serial_programming
      checks:
        - UART_BOOTLOADER_SYNC
      target:
        component: U1
      bootloader:
        component: U1
        interface: uart
        sync_byte: 127
        expected_response: 121
      events:
        - action: uart_send
          from: { component: U2, pin: TXD }
          to: { component: U1, pin: RX }
          bytes: [127]
    required_inputs:
      - Fill event at_us after reset release and boot strap sampling evidence.
```

This is a planning aid, not validation sign-off. Agents should add runnable
scenarios directly and complete non-runnable templates with measured or modeled
evidence before running `circuitci validate`.
