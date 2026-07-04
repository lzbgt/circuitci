# Nexperia PESD5V0S1UL Source Notes

Retrieved on 2026-07-05 from Nexperia:

- Source URL:
  <https://assets.nexperia.com/documents/data-sheet/PESD5V0S1UL.pdf>
- Local original:
  `docs/research/datasheets/nexperia/pesd5v0s1ul.pdf`
- SHA-256:
  `8ddea76afa74f87de5d3662e4d9149bf7397761fa99dc29b44bfbe42872d447e`

Facts retained in `vendor.nexperia.pesd5v0s1ul`:

- The datasheet identifies `PESD5V0S1UL` as a unidirectional ESD protection
  diode in DFN1006-2 / SOD882.
- Pin 1 is cathode `K`, identified by the marking bar; pin 2 is anode `A`.
- The quick-reference and characteristics tables list `VRWM = 5 V` maximum.
- Diode capacitance at `f = 1 MHz`, `VR = 0 V`, and `Tamb = 25 C` is
  `152 pF` typical and `200 pF` maximum.
- Breakdown voltage at `IR = 5 mA` is `6.4 V` minimum, `6.8 V` typical, and
  `7.2 V` maximum.
- Reverse leakage at `VRWM = 5 V` and `Tamb = 25 C` is `0.1 uA` typical and
  `1 uA` maximum.
- The limiting-values table lists `150 W` rated peak pulse power and `15 A`
  rated peak pulse current for a non-repetitive `8/20 us` exponential pulse.
- It lists `30 kV` IEC 61000-4-2 contact-discharge ESD and `10 kV` HBM
  ratings.

Model boundary:

- The pack supports static VBUS-to-ground clamp coverage through
  `INTERFACE_PROTECTION_REVIEW` and USB connector VBUS protection checks.
- The generated-SPICE metadata is reduced to normal-operation high-impedance
  observation and capacitance/standoff screening.
- It does not sign off ESD pulse waveform, surge thermal behavior, leakage over
  temperature, USB inrush, return-path quality, connector placement, or final
  hardware robustness without board-specific evidence.
