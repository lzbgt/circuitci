# Smart Robot PMU Switch Selection

Date: 2026-06-17

This note records the first source-backed PMU switched-rail part selection for
the smart-robot demo. The goal is to replace placeholders only where source
evidence matches the validation contract.

## Servo Rail Selection

`U_SERVO_SW` is now modeled as a configured TI `TPS25948` eFuse variant:

- Source files:
  - `docs/research/smart_robot/sources/tps25948_datasheet.pdf`
  - `docs/research/smart_robot/sources/tps25948_product.html`
- CircuitCI model:
  - `libs/vendor/ti/efuses/tps25948_8a_rcb_dvdt.model.yaml`
- PMU schematic/model binding:
  - `U_SERVO_SW`
  - `vendor.ti.tps25948_8a_rcb_dvdt`

Datasheet facts encoded in the model:

- Recommended input range: 3.5 V to 23 V.
- Absolute maximum input range: 28 V.
- Continuous switch current: 8 A at `TJ <= 125 C`.
- Configured current limit: 8 A typical using `RILM = 604 ohm`, with 7.2 A
  minimum and 8.7 A maximum in the datasheet row.
- On-resistance: 20 mohm maximum over 3.5 V to 23 V, 3 A, and -40 C to
  125 C.
- Junction-to-ambient thermal resistance: 33.4 C/W on TI's custom 2s2p board.
- Recommended junction temperature range: -40 C to 125 C.
- `reverse_current_blocking_mode: always` is supported by the integrated
  back-to-back FET path.
- With `CdVdt = 3.3 nF`, the datasheet switching-characteristics table gives
  7.04 ms rise time at 12 V.

CircuitCI uses those facts for:

- `MODEL_QUALITY_REQUIRED`
- `POWER_SWITCH_BUDGET_VALID`
- `POWER_SWITCH_REVERSE_CURRENT_VALID`
- `POWER_SWITCH_INRUSH_VALID`

The current PMU scenario assumes `switched_capacitance_F: 0.001` for the servo
rail. That is a first-pass design envelope, not final BOM/layout evidence. It
must be replaced by downstream rail capacitance extracted from CAD/BOM before
fabrication sign-off.

## Wheel Rail Candidate Not Selected

TI `TPS25985` was downloaded and reviewed as a high-current eFuse candidate:

- Source files:
  - `docs/research/smart_robot/sources/tps25985_datasheet.pdf`
  - `docs/research/smart_robot/sources/tps25985_product.html`

Useful source facts:

- Recommended input range: 4.5 V to 16 V.
- RMS switch current: 60 A at `TJ <= 125 C`.
- Peak output current: 80 A at `TJ <= 125 C`.
- On-resistance: 1 mohm maximum over -40 C to 125 C at 8 A.
- Junction-to-ambient thermal resistance: 19.9 C/W on TI's custom 8-layer
  board.
- Adjustable inrush control through the `DVDT` pin.

It is not selected for `U_WHEEL_SW` yet because the cached datasheet/product
source review does not prove the required off-state reverse-current isolation
mode for the wheel e-stop rail. The wheel switched rail therefore remains
blocked on selected switch model quality, static switch budget, reverse-current
mode evidence, and inrush evidence.
