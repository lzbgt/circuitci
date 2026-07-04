# onsemi BSS84 Source Notes

Retrieved on 2026-06-12 from onsemi:

- Source URL:
  <https://www.onsemi.com/download/data-sheet/pdf/bss84-d.pdf>
- Local original:
  `docs/research/datasheets/onsemi/bss84.pdf`
- SHA-256:
  `8531adc677bb06835cc4dee425b4fa2850be9e80d9b8a26d0ffe86c314c8463a`

Facts retained in `vendor.onsemi.bss84`:

- The datasheet covers the BSS84 P-channel enhancement-mode MOSFET.
- It lists `VDSS = -50 V`, continuous `VGSS = +/-20 V`, and continuous drain
  current `ID = -130 mA`.
- It lists pulsed drain current `IDM = -520 mA`; this is retained as an
  unqualified pulse rating and must not waive continuous-current overstress.
- It lists total power dissipation `PD = 360 mW` at `TA = 25 C` with
  `2.88 mW/C` derating above `25 C`.
- It lists SOT-23 pinout as pin 1 gate, pin 2 source, and pin 3 drain.
- It lists `VGS(th) = -0.8 V` minimum, `-1.7 V` typical, and `-2.0 V` maximum
  at `VDS = VGS`, `ID = -1 mA`, and `TA = 25 C`.
- It lists `RDS(on) = 10 ohm` maximum at `VGS = -5 V` and `ID = -100 mA`.
- It lists `Ciss = 73 pF`, `Coss = 10 pF`, and `Crss = 5 pF` typical at
  `VDS = -25 V`, `VGS = 0 V`, and `f = 1 MHz`.
- It lists `Qg = 0.9 nC` typical and `1.3 nC` maximum at `VDS = -25 V`,
  `ID = -100 mA`, and `VGS = -5 V`.

Model boundary:

- The pack supports generated Board IR SPICE plumbing and source-backed
  operating-limit checks for MOSFET terminal voltages, continuous drain
  current, and derated power dissipation.
- The bundled SPICE card is a reduced preliminary Level-1 fit, not a vendor
  compact model.
- It does not sign off switching loss, package thermal coupling, avalanche,
  qualified pulse-current SOA, gate-drive margin, EMI, or final production
  hardware behavior without vendor or bench calibration.
