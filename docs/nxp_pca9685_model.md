# NXP PCA9685 Model

`vendor.nxp.pca9685` is a source-backed NXP PCA9685 16-channel, 12-bit I2C PWM
driver model for preliminary board-facing observation checks.

Saved source material:

- Product page: `docs/research/smart_robot/sources/pca9685_product.html`
- Product page URL:
  <https://www.nxp.com/products/power-drivers/lighting-driver-and-controller-ics/led-drivers/16-channel-12-bit-pwm-fm-plus-ic-bus-led-driver:PCA9685>
- Product page SHA-256:
  `28cbfe16e1a9b64c21ee3dec97f01f1277aa08013b6d67e11084a08536804468`
- Datasheet: `docs/research/smart_robot/sources/pca9685_datasheet.pdf`
- Datasheet URL: <https://www.nxp.com/docs/en/data-sheet/PCA9685.pdf>
- Datasheet SHA-256:
  `237d47f339cac4c3a0d56a5f0b4d3c93df71e3eb43f36ac57ea4ff38e6b2e585`
- Retrieved: `2026-06-14`

The static metadata captures the 2.3 V to 5.5 V `VDD` range, Fast-mode Plus I2C
role, 12-bit PWM controller role, low-load output-source/sink class, and four
representative PWM output ports.

The generated-SPICE face is `CIRCUITCI_PCA9685_PWM_DRIVER` in
`models/spice/generic/analog_behavioral.lib`. It maps optional Board IR
component parameters into explicit instance parameters:

- `observation_pwm_high_v` to `PWM_HIGH_V`
- `observation_pwm_frequency_hz` to `PWM_FREQ_HZ`
- `observation_pwm0_duty_percent` to `PWM0_DUTY_PERCENT`
- `observation_pwm1_duty_percent` to `PWM1_DUTY_PERCENT`
- `observation_pwm2_duty_percent` to `PWM2_DUTY_PERCENT`
- `observation_pwm3_duty_percent` to `PWM3_DUTY_PERCENT`
- `observation_scl_state` to `SCL_STATE`
- `observation_sda_state` to `SDA_STATE`

`examples/good_pca9685_pwm_driver_observation` checks VDD, OE low, idle SCL/SDA,
and four representative 50 Hz PWM output high/low samples in a direct-open GUI
example. Model-aware `Create Checks` presets also expand PWM-driver generated
observations to a two-cycle transient window before adding VDD, OE, I2C idle,
and PWM sample checks.

The generated-SPICE face deliberately does not model I2C protocol/register
behavior, oscillator tolerance, phase staggering, LED/servo output current,
pull-up rise time, servo position, servo stall or regeneration, disabled-output
high-Z behavior, thermal behavior, or final PWM timing sign-off.
