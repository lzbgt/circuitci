# onsemi NDS7002A Source Notes

Retrieved on 2026-06-12 from onsemi:

- Source URL:
  <https://www.onsemi.com/download/data-sheet/pdf/nds7002a-d.pdf>
- Local original:
  `docs/research/datasheets/onsemi/nds7002a.pdf`
- SHA-256:
  `160c0e7cdbee397ba4490112aa442e0df20f159c21519e2e17ae52456152e38e`

Facts retained in `vendor.onsemi.nds7002a`:

- The datasheet covers 2N7000, 2N7002, and NDS7002A N-channel enhancement-mode
  MOSFETs.
- For NDS7002A, it lists `VDSS = 60 V`, continuous `VGSS = +/-20 V`, and
  continuous drain current `ID = 280 mA`.
- It lists pulsed drain current `IDM = 1.5 A`; this is retained as an
  unqualified pulse rating and must not waive continuous-current overstress.
- It lists total power dissipation `PD = 300 mW` at `TA = 25 C` with
  `2.4 mW/C` derating above `25 C`.
- It lists SOT-23 pinout as pin 1 gate, pin 2 source, and pin 3 drain.
- It lists `VGS(th) = 1.0 V` minimum, `2.1 V` typical, and `2.5 V` maximum at
  `VDS = VGS` and `ID = 250 uA`.
- It lists `RDS(on) = 2.0 ohm` maximum at `VGS = 10 V`, `ID = 500 mA`, and
  `3.0 ohm` maximum at `VGS = 5 V`, `ID = 50 mA`.
- It lists `Ciss = 50 pF`, `Coss = 25 pF`, and `Crss = 5 pF` maximum at
  `VDS = 25 V`, `VGS = 0 V`, and `f = 1 MHz`.
- The model retains approximate `Qg = 1.4 nC` from datasheet Figure 10 at
  `VDS = 25 V` and `ID = 500 mA`.

Model boundary:

- The pack supports generated Board IR SPICE plumbing and source-backed
  operating-limit checks for MOSFET terminal voltages, continuous drain
  current, and derated power dissipation.
- The bundled SPICE card is a reduced preliminary Level-1 fit, not a vendor
  compact model.
- It does not sign off switching loss, package thermal coupling, avalanche,
  qualified pulse-current SOA, gate-drive margin, EMI, or final production
  hardware behavior without vendor or bench calibration.
