# Diodes Incorporated AP2112 Sources

Retrieved on 2026-06-12.

## Primary Source

- Official Diodes Incorporated product page:
  <https://www.diodes.com/part/view/AP2112>
- Official Diodes Incorporated data sheet:
  <https://www.diodes.com/datasheet/download/AP2112.pdf>
- Local copy:
  `docs/research/datasheets/diodes/ap2112.pdf`
- SHA-256:
  `ef8d376f2ec356e29172eb9e053819a0ebdcc576dba7fc9ab0505c568427920f`

## Extracted Facts Used In `vendor.diodes.ap2112k_3v3`

- AP2112 is a CMOS low-dropout regulator with active-high enable.
- AP2112K is the SOT25 package option.
- AP2112K-3.3 is a fixed `3.3 V` output regulator.
- Recommended `VIN` operating range is `2.5 V` to `6.0 V`.
- AP2112-3.3 output accuracy is `+/-1.5%`, so the model uses:
  - `VOUT min = 3.2505 V`
  - `VOUT typ = 3.3 V`
  - `VOUT max = 3.3495 V`
- Maximum output current table guarantees at least `600 mA` for the 3.3 V
  option under the datasheet condition.
- Dropout voltage at `IOUT = 600 mA` is `250 mV` typical and `400 mV`
  maximum. The model uses the `400 mV` maximum for static margin checks.
- Quiescent current is `55 uA` typical and `80 uA` maximum at the 3.3 V
  datasheet condition.
- `EN` high threshold is `1.5 V` minimum; `EN` low threshold is `0.4 V`
  maximum.
- No-load startup time is `20 us` typical only. The model records this in
  datasheet metadata but does not use it as a hard `startup_delay_us` rule.
- The typical application recommends `1.0 uF` input and output capacitors.

## Modeling Notes

The model is a board-level static regulator screen. `POWER_TREE_VALID` can use
it to catch wrong rail voltage, insufficient nominal dropout margin, and output
load current above the 600 mA guarantee. It does not sign off load-dependent
dropout curves, thermal derating, output capacitor ESR/stability, enable
startup waveform, transient response, or short-circuit behavior.
