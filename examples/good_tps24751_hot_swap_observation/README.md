# TPS24751 Hot-Swap Observation

This generated-SPICE fixture exercises the source-backed
`vendor.ti.tps24751_csd17501q5a_12a_reverse_blocking` model as an enabled
12 V hot-swap/reverse-blocking path into a 120 ohm load.

The behavioral face is intentionally reduced fidelity. It observes wiring,
protected-rail voltage, and load current using the source-backed operating
range, EN threshold, 12 A current class, 11 A current-limit design point, and
9.7 mOhm path-resistance metadata. It is not a sign-off model for fault timing,
external-FET gate-drive dynamics, reverse-current behavior, inrush accuracy, or
thermal shutdown.

Run it with:

```sh
cargo run --release --bin circuitci -- validate examples/good_tps24751_hot_swap_observation/project.yaml
```
