# TI TCAN3413 Model Notes

`vendor.ti.tcan3413` is a source-backed TI TCAN3413 3.3 V CAN FD transceiver
model.

Saved source material:

- `docs/research/smart_robot/sources/tcan3413_product.html`
- `docs/research/smart_robot/sources/tcan3413_datasheet.pdf`

Sourced facts captured in the component pack:

- 3.0 V to 3.6 V VCC operating range.
- 1.7 V to 3.6 V VIO logic range.
- 5 Mbps CAN FD data-rate class and 8 Mbps light-bus data-rate class.
- Bus fault protection class of `+/-58 V`.
- Pin roles for VCC, VIO, GND, TXD, RXD, STB, CANH, and CANL.
- MCU-side input thresholds of `VIH >= 2.0 V` and `VIL <= 0.8 V`.

Generated-SPICE observation face:

The model uses `CIRCUITCI_TCAN3413_CAN_TRANSCEIVER` from
`models/spice/generic/analog_behavioral.lib` with pin order:

`VCC VIO GND TXD RXD STB CANH CANL`

The subcircuit is deliberately reduced-fidelity. It uses explicit Board IR
component parameters to create deterministic line-state observations:

- `observation_txd_state` -> `TXD_STATE`
- `observation_stb_state` -> `STB_STATE`

The direct-open GUI example
`examples/good_ti_tcan3413_can_transceiver_observation` checks a 3.3 V
normal-mode dominant snapshot with TXD low, STB low, RXD low, CANH high, and
CANL low.

This face is intended for early executable observation of supply and line-state
wiring only. It is not valid for CAN termination, stub length, common-mode
range, cable length, EMC, bus fault energy, CAN FD timing, arbitration, eye
margin, or final signal-integrity sign-off.
