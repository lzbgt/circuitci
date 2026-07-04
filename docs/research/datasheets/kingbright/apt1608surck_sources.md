# Kingbright APT1608SURCK Source Notes

Retrieved on 2026-07-05 from Kingbright USA:

- Source URL:
  <https://www.kingbrightusa.com/images/catalog/SPEC/APT1608SURCK.pdf>
- Local original: `docs/research/datasheets/kingbright/apt1608surck.pdf`
- SHA-256:
  `9e1b5b4425e7251bf67a228b42f881ef87ed5f4088d89dc1bcb74b29789dd156`

Facts retained in `vendor.kingbright.apt1608surck`:

- APT1608SURCK is a 1.6 mm x 0.8 mm x 0.75 mm SMD chip LED lamp.
- It is a Hyper Red AlGaInP LED with water-clear lens.
- Kingbright lists absolute maximum ratings at 25 C: 75 mW power dissipation,
  5 V reverse voltage, 115 C junction temperature, 30 mA DC forward current,
  and 185 mA peak forward current for 1/10 duty cycle and 0.1 ms pulse width.
- Kingbright lists forward voltage at 20 mA and 25 C as 1.95 V typical and
  2.5 V maximum.
- Kingbright lists reverse current at 5 V as 10 uA maximum.
- Kingbright lists capacitance as 35 pF, dominant wavelength as 630 nm, peak
  wavelength as 645 nm, and viewing angle as 120 degrees.

Model boundary:

- The pack supports generated Board IR SPICE plumbing and source-backed
  operating-limit checks for forward current, reverse voltage, and power.
- The bundled SPICE card is a reduced preliminary electrical fit, not a vendor
  optical/electrical model.
- It does not sign off brightness, color bins, luminous-intensity degradation,
  thermal board coupling, reflow process conditions, pulse-current derating, or
  final production hardware behavior without vendor or bench calibration.
