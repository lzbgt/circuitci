# TI THVD1450 Model Notes

`vendor.ti.thvd1450` is a source-backed TI THVD1450 half-duplex RS-485
transceiver model.

Saved source material:

- `docs/research/smart_robot/sources/thvd1450_product.html`
- `docs/research/smart_robot/sources/thvd1450_datasheet.pdf`

Sourced facts captured in the component pack:

- 3.0 V to 5.5 V supply operating range.
- 50 Mbps maximum data-rate class.
- One-eighth-unit-load receiver loading and up to 256 bus nodes.
- TI product-page IEC ESD positioning of `+/-18 kV`.
- Pin roles for VCC, GND, DI, RO, DE, RE_N, A, and B.
- MCU-side input thresholds of `VIH >= 2.0 V` and `VIL <= 0.8 V`.

Generated-SPICE observation face:

The model uses `CIRCUITCI_THVD1450_RS485_TRANSCEIVER` from
`models/spice/generic/analog_behavioral.lib` with pin order:

`VCC GND DI RO DE RE_N A B`

The subcircuit is deliberately reduced-fidelity. It uses explicit Board IR
component parameters to create deterministic line-state observations:

- `observation_di_state` -> `DI_STATE`
- `observation_de_state` -> `DE_STATE`
- `observation_re_n_state` -> `RE_N_STATE`

The direct-open GUI example
`examples/good_ti_thvd1450_rs485_transceiver_observation` checks a 3.3 V
enabled-driver snapshot with DI high, DE high, RE_N low, RO high, A high, and
B low.

This face is intended for early executable observation of supply and line-state
wiring only. It is not valid for RS-485 termination, failsafe biasing,
common-mode range, cable length, EMC, ESD/fault energy, propagation delay,
timing skew, bus loading, eye margin, or final signal-integrity sign-off.
