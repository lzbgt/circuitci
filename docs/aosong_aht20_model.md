# Aosong AHT20 Model

`vendor.aosong.aht20` is a source-backed reduced model for Aosong's AHT20
humidity and temperature sensor.

The model captures the board-facing `VDD`, `GND`, `SDA`, and `SCL` pins, the
2.2 V to 5.5 V supply range, I2C address `0x38`, and the datasheet's external
I2C pull-up and 10 uF VDD decoupling guidance. Table 3 provides 980 uA typical
measure current and 250 nA maximum dormant current at VCC = 3.3 V and
T < 60 C, but no maximum measurement current, so current-budget sign-off must
stay explicit rather than relying on `max_supply_current_A`.

The generated-SPICE face is:

```spice
CIRCUITCI_AHT20_I2C_OBSERVATION
```

It is intentionally high impedance. External sources and pull-ups define the
observed `VDD`, `SDA`, and `SCL` states.

Useful examples:

- `examples/good_aosong_aht20_i2c_power/project.yaml`
- `examples/good_aosong_aht20_i2c_observation/project.yaml`
- `examples/bad_aosong_aht20_vdd_overvoltage/project.yaml`

This model does not validate humidity or temperature accuracy, calibration,
command protocol, measurement timing, power-on delay timing, self-heating,
contamination/recovery behavior, reflow drift, or final I2C signal integrity.
