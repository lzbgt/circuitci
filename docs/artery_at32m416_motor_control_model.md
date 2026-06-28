# Artery AT32M416 Motor-Control Model

`vendor.artery.at32m416_motor_control` is a source-backed Artery AT32M416 MCU
model for preliminary smart-robot wheel-actuator rail and driver-interface
screening.

## Sources

- Product-page HTML:
  `docs/research/smart_robot/sources/at32m416_product.html`
- Source URL:
  `https://www.arterytek.com/cn/product/AT32M416.jsp`
- SHA-256:
  `1d100588bde163e80f3d6b715b1e76ff6e8c8717e1582510b41169b1b17967ce`
- Retrieved: `2026-06-14`

## Modeled Facts

- Motor-control MCU project supply range: `2.6 V` to `3.6 V`.
- Preliminary supply-current class: `0.12 A`.
- Board-facing digital inputs use `2.0 V` high and `0.8 V` low thresholds.
- Board-facing digital outputs are represented with `3.3 V` high-state
  metadata and `50 ohm` source impedance.
- The model covers the wheel-actuator links to CAN, six PWM drive-control
  lines, DRV8323-style enable/fault/SPI pins, current-sense nodes, encoder
  inputs, and board enable/fault lines.

## Generated-SPICE Face

`CIRCUITCI_AT32M416_MOTOR_CONTROL_MCU` is a reduced observation model for:

- VDD rail checks.
- CAN TX/RX logic-state checks.
- Six static PWM output-state checks.
- Gate-driver enable, nFAULT, and SPI line-state checks.
- Current-sense node voltage checks.
- Encoder input checks.
- Board enable input and fault output checks.

The output states are explicit Board IR component parameters:

- `observation_can_tx_state`
- `observation_pwm_uh_state`
- `observation_pwm_ul_state`
- `observation_pwm_vh_state`
- `observation_pwm_vl_state`
- `observation_pwm_wh_state`
- `observation_pwm_wl_state`
- `observation_drv_en_state`
- `observation_drv_spi_sck_state`
- `observation_drv_spi_mosi_state`
- `observation_drv_spi_cs_state`
- `observation_fault_out_state`

The direct-open GUI fixture is:

- `examples/good_artery_at32m416_motor_control_observation/project.yaml`

Its `Create Checks` action regenerates VDD, CAN, PWM, driver-control, SPI,
encoder, enable, and fault checks for the placed MCU without editing YAML.

## Limits

This model is not valid for firmware execution, reset or clock timing, PWM
timer waveform generation, ADC conversion behavior, current reconstruction,
FOC loops, dead-time sign-off, exact package pin assignment, gate-drive
physics, layout, thermal behavior, EMC, or final signal-integrity sign-off.
Those require separate firmware, timing, measurement, package-symbol, layout,
thermal, and SI evidence.
