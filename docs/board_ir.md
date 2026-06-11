# Board Graph IR

Board Graph IR is CircuitCI's normalized representation of a circuit design. It is intentionally independent of KiCad, EasyEDA, Altium, JITX, or any one component family.

## Project File

The MVP accepts a YAML project file:

```yaml
project:
  name: bad_backdrive_board
  version: 0.1.0

libraries:
  - ../../libs/generic

board:
  components:
  U1:
      model: generic.mcu.basic
      part_number: GENERIC_MCU
      power_domains:
        VDD: mcu_3v3
      pins:
        VDD: mcu_3v3
        GND: gnd
        RX: uart_rx

  nets:
    mcu_3v3:
      kind: power
      nominal_voltage: 3.3
      powered: false
    gnd:
      kind: ground
    uart_rx:
      kind: digital_or_analog

scenarios:
  - name: usb_hot_plug_mcu_unpowered
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
      - driver:
          component: U2
          pin: TXD
        victim:
          component: U1
          pin: RX
        series_resistance_ohm: 0
```

## Required Concepts

| Concept | Required fields | Notes |
| --- | --- | --- |
| project | `name`, `version` | Stable report identity. |
| libraries | list of paths | Relative paths resolve from the project file directory. |
| components | ID map | Each component references one component model. |
| component pins | pin-to-net map | Pin names must exist in the bound model. |
| component power domains | power-port-to-net map | `power_domains` maps model power pins to rails. |
| nets | ID map | Nets describe power, ground, and mixed signal domains. |
| scenarios | list | MVP scenarios select validation checks. |

## Net Kinds

- `power`: rail with `nominal_voltage` and optional `powered` state.
- `ground`: reference net.
- `digital_or_analog`: mixed-signal net, such as reset, UART, GPIO, and I2C.

The MVP uses declared `powered` states. Future scenarios can change source states over time.

Power semantics:

- `powered: true`: actual rail voltage equals `nominal_voltage`.
- `powered: false`: actual rail voltage is `0.0 V`.
- missing `powered`: rail state is unknown and rules that require actual voltage must emit an informational or warning finding instead of guessing.

## Consistency Rules

The parser and binder must report:

- unknown or malformed project sections
- component without model ID
- model not found in configured libraries
- component pin not declared by its model
- required model pin not connected
- component power-domain net missing
- power-domain net not declared as `power`

These checks produce report findings instead of runtime panics.

## GPIO Backdrive Scenario Shape

For the first slice, `gpio_backdrive` scenarios must declare `pin_states` and `paths`.

`pin_states` declares runtime direction/state for IO-capable pins:

- `component`
- `pin`
- `mode`: `input`, `output`, or `high_z`
- `state`: `high`, `low`, or `z` when applicable

`paths` declares direct electrical paths to evaluate before passive topology traversal exists:

- `driver.component`
- `driver.pin`
- `victim.component`
- `victim.pin`
- `series_resistance_ohm`, default `0`

The driver and victim pins must be connected to the same net in the board IR.
