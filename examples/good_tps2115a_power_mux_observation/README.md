# TPS2115A Power-Mux Observation

This generated-SPICE fixture exercises the source-backed `vendor.ti.tps2115a`
model as a USB-selected 5 V power mux feeding a 100 ohm system load while the
backup input is inactive.

The behavioral face is intentionally reduced fidelity. It observes selected
input/output wiring, output rail voltage, inactive-input standoff, and load
current using source-backed voltage, current-class, and reverse-blocking
metadata. It is not a sign-off model for autoswitch truth-table behavior,
switchover droop, reverse-current magnitude, ILIM resistor-derived current
limits, thermal behavior, package limits, or layout.

Run it with:

```sh
cargo run --release --bin circuitci -- validate examples/good_tps2115a_power_mux_observation/project.yaml
```
