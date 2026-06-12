# TI TLV803E/TLV809E/TLV810E Sources

Retrieved on 2026-06-12.

## Primary Source

- Official Texas Instruments data sheet:
  <https://www.ti.com/lit/ds/symlink/tlv803e.pdf>
- Local copy:
  `docs/research/datasheets/ti/tlv803e-tlv809e-tlv810e.pdf`
- SHA-256:
  `bce5287af45910a79177d546d5b2ca58cfec8aa3a1cd83dbfdd98308285d0fe7`

## Extracted Facts Used In `vendor.ti.tlv803ea29`

- The TLV803E/TLV809E/TLV810E family monitors `VDD` and asserts reset when
  `VDD` falls below the fixed falling threshold `VIT-`.
- TLV803E is the active-low open-drain output variant. It requires an external
  pull-up resistor on `RESET`.
- TLV803EA29 has nominal `VIT- = 2.93 V`.
- Threshold accuracy is `-2%` to `+2%` over `-40 C` to `125 C`, so the model
  uses:
  - `threshold_min_V = 2.8714`
  - `threshold_max_V = 2.9886`
- The recommended `VDD` operating range is `1.7 V` to `6.0 V`.
- Maximum supply current at `VDD = 3.3 V` and `VDD > VIT+` is `1 uA`.
- Delay option `A` has reset timeout/release delay:
  - minimum `130 ms`
  - typical `200 ms`
  - maximum `270 ms`
- The model stores `reset_release_delay_us = 270000` as a conservative
  board-level delay for reset timing checks.

## Modeling Notes

The current Board IR has digital output ports but no separate open-drain port
kind. The model therefore represents `RESET` as `digital_electrical_output` and
records the open-drain requirement in datasheet metadata and limitations.
Validation can check threshold and delay metadata, but the external pull-up
resistor value, reset waveform shape, low-voltage output validity, propagation
delay, and glitch immunity still need explicit circuit or SPICE evidence.
