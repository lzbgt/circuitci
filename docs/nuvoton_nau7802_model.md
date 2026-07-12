# Nuvoton NAU7802 Model

`vendor.nuvoton.nau7802` is a source-backed reduced model for Nuvoton's NAU7802
24-bit bridge/sensor ADC.

The model captures the board-facing SOP-16/QFN-16 pin roles, the 2.7 V to
5.5 V `DVDD` operating range, the 2.7 V to `DVDD` external `AVDD/LDO` supply
range, the 2-wire I2C-compatible slave interface, and the differential bridge
input/reference pins.

The generated-SPICE face is:

```spice
CIRCUITCI_NAU7802_BRIDGE_ADC_OBSERVATION
```

It is intentionally high impedance. External sources, pull-ups, bridge
resistors, and board metadata define observed `DVDD`, `AVDD`, `REFP`, `REFN`,
`VIN1P`, `VIN1N`, `VIN2P`, `VIN2N`, `SCLK`, `SDIO`, `DRDY`, and `VBG` states.

Useful examples:

- `examples/good_nuvoton_nau7802_bridge_adc_power/project.yaml`
- `examples/good_nuvoton_nau7802_bridge_adc_observation/project.yaml`
- `examples/bad_nuvoton_nau7802_dvdd_overvoltage/project.yaml`

This model does not validate conversion accuracy, PGA gain, calibration,
noise, register protocol, I2C timing, streaming data mode, bridge excitation
accuracy, oscillator behavior, or temperature sensor behavior.
