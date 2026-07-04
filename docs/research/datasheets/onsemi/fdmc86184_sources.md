# onsemi FDMC86184 Source Notes

Retrieved on 2026-06-12 from onsemi:

- Source URL:
  <https://www.onsemi.com/download/data-sheet/pdf/fdmc86184-d.pdf>
- Local original:
  `docs/research/datasheets/onsemi/fdmc86184.pdf`
- SHA-256:
  `d338e4ae50dfd32e06bfeea148fc369220a837f36dd3b97610dcc8c33a46fa4e`

Facts retained in `vendor.onsemi.fdmc86184`:

- The datasheet covers the FDMC86184 N-channel Shielded Gate PowerTrench
  MOSFET.
- It lists `VDSS = 100 V`, continuous `VGSS = +/-20 V`, and continuous drain
  current `ID = 12 A` at `TA = 25 C`.
- It lists pulsed drain current `IDM = 266 A`; datasheet notes bind pulsed
  testing to `< 300 us` pulse width and `< 2.0%` duty cycle and direct users to
  the SOA graph.
- It lists total power dissipation `PD = 2.3 W` at `TA = 25 C`; model metadata
  records `18.4 mW/C` derating from the retained thermal values.
- It lists `RDS(on) = 8.5 mohm` maximum at `VGS = 10 V`, `ID = 21 A`, and
  `TJ = 25 C`.
- It lists `VGS(th) = 2.0 V` minimum, `3.1 V` typical, and `4.0 V` maximum at
  `VGS = VDS` and `ID = 110 uA`.
- It lists `Ciss = 2090 pF`, `Coss = 1270 pF`, and `Crss = 25 pF` maximum at
  `VDS = 50 V`, `VGS = 0 V`, and `f = 1 MHz`.
- It lists `Qg = 21 nC` typical and `30 nC` maximum at `VDD = 50 V`,
  `ID = 21 A`, and `VGS = 10 V`.
- Figure 11 forward-bias SOA points are retained as preliminary hand-digitized
  screening evidence in model metadata and in
  `docs/research/datasheets/pulse_soa_datasheet_sources.md`.

Model boundary:

- The pack supports generated Board IR SPICE plumbing and source-backed
  operating-limit checks for MOSFET terminal voltages, continuous/pulsed drain
  current, derated power dissipation, and digitized SOA screening.
- The bundled SPICE card is a reduced preliminary Level-1 fit, not a vendor
  compact model.
- The SOA points are hand-digitized screening evidence, not final sign-off.
- It does not sign off switching loss, package thermal coupling, avalanche,
  machine-readable vendor SOA, gate-drive margin, EMI, or final production
  hardware behavior without vendor or bench calibration.
