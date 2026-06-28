# TI DRV8323 Model

`vendor.ti.drv8323` is a source-backed TI DRV8323 three-phase smart gate-driver
model for preliminary board-facing observation checks.

Saved source material:

- Product page: `docs/research/smart_robot/sources/drv8323_product.html`
- Product page URL: <https://www.ti.com/product/DRV8323>
- Product page SHA-256:
  `58097fe705d14a8c40b82d6404a500c2db99e6d1a1330da6191fdbc929c8feed`
- Datasheet: `docs/research/smart_robot/sources/drv8323_datasheet.pdf`
- Datasheet URL: <https://www.ti.com/lit/ds/symlink/drv8323.pdf>
- Datasheet SHA-256:
  `dd8386c972e8d0a57278e432da6e9d8b2bc2f73768dd97eac69594da20d24208`
- Retrieved: `2026-06-14`

The static metadata captures the 6 V to 60 V motor-supply operating range,
3 V to 5.5 V `DVDD` range, 2 V logic input-high threshold, `nFAULT` and `SDO`
digital-output metadata, and the presence of the three low-side current-sense
amplifier outputs.

The generated-SPICE face is `CIRCUITCI_DRV8323_GATE_DRIVER` in
`models/spice/generic/analog_behavioral.lib`. It maps optional Board IR
component parameters into explicit instance parameters:

- `observation_nfault_state` to `NFAULT_STATE`
- `observation_sdo_state` to `SDO_STATE`
- `observation_soa_v` to `SOA_V`
- `observation_sob_v` to `SOB_V`
- `observation_soc_v` to `SOC_V`

`examples/good_drv8323_gate_driver_observation` checks VM, DVDD, ENABLE,
released `nFAULT`, low `SDO`, and nominal SOA/SOB/SOC output presence in a
direct-open GUI example.

The generated-SPICE face deliberately does not model MOSFET gate-drive
strength, high-side/low-side switching, charge-pump/bootstrap behavior, dead
time, SPI register/protection behavior, shunt gain/offset/noise, phase
switching, motor dynamics, layout, EMI, or thermal sign-off.
