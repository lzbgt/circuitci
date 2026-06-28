# TI TPS24751 + CSD17501Q5A Model

## Source

- TPS24751 product page: `docs/research/smart_robot/sources/tps24751_product.html`
- TPS24751 datasheet: `docs/research/smart_robot/sources/tps24751_datasheet.pdf`
- CSD17501Q5A product page: `docs/research/smart_robot/sources/csd17501q5a_product.html`
- CSD17501Q5A datasheet: `docs/research/smart_robot/sources/csd17501q5a_datasheet.pdf`
- TPS24751 product URL: <https://www.ti.com/product/TPS24751>
- TPS24751 datasheet URL: <https://www.ti.com/lit/ds/symlink/tps24751.pdf>
- CSD17501Q5A product URL: <https://www.ti.com/product/CSD17501Q5A>
- CSD17501Q5A datasheet URL: <https://www.ti.com/lit/gpn/CSD17501Q5A>
- TPS24751 product-page SHA-256:
  `065e10285e78a7318bd00983d95a03187c4191c6f4f9d41db7e27313dde8f408`
- TPS24751 datasheet SHA-256:
  `7b9513d1185c74c94218653ded608e89e90d53d2886adbee477cf659a6bd3fdf`
- CSD17501Q5A product-page SHA-256:
  `f2a419d687a74b67d1ea187961a9981a3dfd70afaae5f860206489f2720d7aee`
- CSD17501Q5A datasheet SHA-256:
  `2b312e3e9c696e0cbb298a48ce855c538ff9c806d187c0092b5a086b393884f8`

## Modeled Facts

The `vendor.ti.tps24751_csd17501q5a_12a_reverse_blocking` model captures
board-level facts needed for static hot-swap and reverse-blocking validation:

- Active-high hot-swap path using TPS24751 with external CSD17501Q5A
  reverse-blocking FET.
- `VIN` recommended operating range: `2.5 V` to `18.0 V`.
- Active-high `EN` control with represented input-high threshold `1.3 V`.
- Continuous switch current class: `12.0 A`.
- Smart-robot wheel-rail current-limit design point: `11.0 A`.
- Conservative effective path on-resistance: `9.7 mOhm`, combining the
  TPS24751 hot `OUT` resistance and CSD17501Q5A `RDS(on)` source facts.
- Disabled-state reverse-current-blocking metadata.
- Pins represented: `VIN`, `VOUT`, `EN`, `GND`, `TIMER`, `PROG`, `SET`,
  `FLTb`, and `PGb`.

`TIMER`, `PROG`, `SET`, `FLTb`, and `PGb` are retained for source-pinned
component metadata and schematic/import preservation. Static validators use
`power_switch` metadata for selected-switch budget, first-pass inrush, and
reverse-current-blocking checks when scenarios declare those requirements.

## Generated SPICE Use

The model declares `simulation.spice` metadata for generated Board IR SPICE.
The generated-SPICE face points at the reduced-fidelity
`CIRCUITCI_TPS24751_CSD17501Q5A_HOT_SWAP` subcircuit in
`models/spice/generic/analog_behavioral.lib` with pin order:

`VIN, VOUT, GND, EN`

That subcircuit models an active-high smooth conductance near the conservative
`9.7 mOhm` path resistance so users can observe enabled hot-swap wiring,
protected-rail voltage, and load current. It intentionally omits TIMER/PROG/SET
current-limit and fault-timer behavior, FLTb/PGb status outputs, external-FET
gate-drive dynamics, disabled-state reverse-current dynamics, thermal shutdown,
inrush accuracy, and final protection sign-off.

`examples/good_tps24751_hot_swap_observation` proves the generated-SPICE
workflow with voltage/current probes and executable checks. The GUI Examples
picker registers the same fixture as `TPS24751 Hot-Swap`.
