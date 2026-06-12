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
- It emits reset templates when a component model declares reset behavior, the
  reset pin is connected, and the target power rail declares `power_valid_at_us`.
- Reset suggestions are marked `runnable: false` until real
  `timing.reset_release_at_us` evidence is filled from a reset supervisor, RC
  model, control-line model, firmware/host trace, or analog waveform.
- It emits boot-strap templates when model boot modes declare required straps
  and the strap pins are connected.
- It emits UART bootloader templates when model bootloader metadata declares a
  UART interface. If an output-capable sender pin is already wired to the target
  RX net, the template includes that sender; otherwise it records the missing
  sender as required input.
- It never invents boot strap states, reset-release timestamps, power-good
  delays, or SPICE assertions.

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
