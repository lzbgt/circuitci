# TPS22918 Load Switch Observation Example

This fixture exercises a datasheet-backed TI TPS22918 load-switch component in
generated Board IR SPICE. `vendor.ti.tps22918` keeps TPS22918 datasheet
voltage, ON-threshold, current, pinout, and static `power_switch` limits, while
its `simulation.spice` face points to CircuitCI's reduced enabled high-side
switch macro-model.

The circuit drives `VIN` and `ON` from explicit 5 V sources and loads `VOUT`
through a 1 kOhm resistor. The analog run setup checks that the switched rail
stays near 5 V and that the generated branch-current probe stays near 5 mA.

This is useful for generated-SPICE workflow, load-switch wiring observation,
probe placement, assertion, and GUI model-pack checks. It is not a TPS22918
CT slew-rate, quick-output-discharge, reverse-current, current-limit, inrush,
leakage, thermal, or final transient sign-off model.

The fixture is registered in the GUI Examples picker as `TPS22918 Load Switch`.
Opening it lands in Sketch with the routed schematic, and `Create Checks`
regenerates a model-aware observation preset for `USW`.

Sources:

- `docs/research/datasheets/ti/tps22918.pdf`
- `docs/ti_tps22918_model.md`
