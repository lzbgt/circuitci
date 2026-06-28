# Artery AT32F435 Motion-Core Model

`vendor.artery.at32f435_motion_core` is a source-backed Artery AT32F435 MCU
model for preliminary smart-robot motion-core rail and low-speed control-link
screening.

## Sources

- Product-page HTML:
  `docs/research/smart_robot/sources/at32f435_product.html`
- Source URL:
  `https://www.arterytek.com/cn/product/AT32F435.jsp`
- SHA-256:
  `ff88d97074371dcfa1f677d6df7422dee2158488d81fea6deac9399117921bd7`
- Retrieved: `2026-06-14`

## Modeled Facts

- Motion-core project supply range: `2.6 V` to `3.6 V`.
- Preliminary supply-current class: `0.12 A`.
- Board-facing digital inputs use `2.0 V` high and `0.8 V` low thresholds.
- Board-facing digital outputs are represented with `3.3 V` high-state
  metadata and `50 ohm` source impedance.
- The model covers the smart-robot motion-core links to LicheeRV UART,
  motion-enable/fault, CAN, RS-485, and servo PWM enable nets.

## Generated-SPICE Face

`CIRCUITCI_AT32F435_MOTION_CORE_MCU` is a reduced observation model for:

- VDD rail checks.
- Host-driven LicheeRV UART RX and motion-enable input checks.
- MCU-driven LicheeRV UART TX and motion-fault output checks.
- CAN TX/RX logic-state checks.
- RS-485 TX/RX/DE logic-state checks.
- Servo PWM output-enable checks.

The output states are explicit Board IR component parameters:

- `observation_lrv_uart_tx_state`
- `observation_motion_fault_irq_state`
- `observation_can_tx_state`
- `observation_rs485_tx_state`
- `observation_rs485_de_state`
- `observation_servo_pwm_oe_state`

The direct-open GUI fixture is:

- `examples/good_artery_at32f435_motion_core_observation/project.yaml`

Its `Create Checks` action regenerates VDD, UART, motion-control, CAN,
RS-485, and servo-enable checks for the placed MCU without editing YAML.

## Limits

This model is not valid for firmware execution, reset or clock timing, CAN or
RS-485 protocol timing, ADC behavior, motor-control loops, exact package pin
assignment, thermal behavior, layout, EMC, or final signal-integrity sign-off.
Those require separate firmware, measurement, package-symbol, layout, and SI
evidence.
