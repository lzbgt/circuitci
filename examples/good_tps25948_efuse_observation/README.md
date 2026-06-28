# TPS25948 eFuse Observation Example

This fixture exercises the source-backed TI TPS25948 eFuse/load-switch model in
generated Board IR SPICE. `vendor.ti.tps25948_8a_rcb_dvdt` keeps TPS25948
datasheet voltage, EN/UVLO threshold, current-limit, on-resistance,
reverse-current-blocking, pinout, and static `power_switch` limits, while its
`simulation.spice` face points to CircuitCI's reduced enabled eFuse
macro-model.

The circuit drives `VIN` from a 12 V source, drives `EN` from a 2 V logic source
above the datasheet EN/UVLO rising threshold, and loads `VOUT` through a
120 ohm resistor. The analog run setup checks that the protected rail stays
near 12 V and that the generated branch-current probe stays near 100 mA.

This is useful for generated-SPICE workflow, eFuse/load-switch wiring
observation, source-pinned model browsing, probe placement, assertion, and GUI
model-pack checks. It is not a TPS25948 dVdt slew-rate, ILM/ITIMER current
limit, fault timer, FLT/SPLYGD status, OVLO, RCBCTRL, reverse-current dynamic,
thermal, inrush, or final protection sign-off model.

The fixture is registered in the GUI Examples picker as `TPS25948 eFuse`.
Opening it lands in Sketch with the routed schematic, and `Create Checks`
regenerates a model-aware observation preset for `UEFUSE`.

Sources:

- `docs/research/smart_robot/sources/tps25948_product.html`
- `docs/research/smart_robot/sources/tps25948_datasheet.pdf`
- `docs/ti_tps25948_model.md`
