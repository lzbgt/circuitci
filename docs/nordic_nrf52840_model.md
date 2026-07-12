# Nordic nRF52840 Model

## Source

- Official Nordic Product Specification HTML:
  `https://docs.nordicsemi.com/bundle/ps_nrf52840/page/keyfeatures_html5.html`
- Retained PDF mirror:
  `docs/research/datasheets/nordic/nrf52840-product-spec-farnell.pdf`
- Source note:
  `docs/research/datasheets/nordic/nrf52840_sources.md`
- Retrieved: 2026-07-05

## Modeled Facts

The `vendor.nordic.nrf52840` model captures source-backed board-boundary
checks for nRF52-class designs:

- `VDD`: `1.7 V` to `3.6 V` normal-voltage supply range.
- `VDDH`: optional `2.5 V` to `5.5 V` high-voltage supply range.
- `VBUS`: optional `4.35 V` to `5.5 V` USB regulator input range.
- `nRESET`: active-low reset boundary on configurable `P0.18`.
- `SWDCLK` and `SWDIO`: debug/programming pins.
- `USB_DP`, `USB_DM`, and `ANT`: retained as named connectivity boundaries.
- A reduced generated-SPICE high-impedance board-observation face for
  preliminary rail, reset, SWD, UART/GPIO idle, USB boundary, and antenna
  feed-state checks.

## Validation Use

`POWER_TREE_VALID` screens nRF52840 supply voltages against the source-backed
recommended operating ranges. The passing public fixture is:

- `examples/good_nordic_nrf52840_normal_voltage_power/project.yaml`
- `examples/good_nordic_nrf52840_board_observation/project.yaml`

The paired injected-error fixture is:

- `examples/bad_nordic_nrf52840_vdd_overvoltage/project.yaml`

## Limits

The generated observation macro is high impedance and only observes board-side
biasing through explicit external sources, pull resistors, and loads. This
model is not valid for GPIO threshold or drive-strength sign-off,
high-voltage-mode regulator sequencing, DCDC inductor/decoupling review, USB
signal integrity, RF antenna matching, NFC behavior, protocol behavior, UICR
reset-configuration programming, firmware execution, thermal sign-off, or
transient current waveforms. Those require separate source evidence and rules.
