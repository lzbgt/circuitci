# onsemi 1N5819 Model

`vendor.onsemi.1n5819` is a source-backed 1 A, 40 V Schottky barrier
rectifier model for generated Board IR SPICE checks.

The model records onsemi's static operating limits:

- 40 V repetitive reverse voltage.
- 48 V non-repetitive peak reverse voltage.
- 28 V RMS reverse voltage.
- 1 A average rectified forward current.
- 25 A non-repetitive surge current for one 60 Hz half-wave cycle.
- 0.875 W conservative ambient power screen derived from the datasheet
  125 C junction limit, 55 C rated-current ambient condition, and 80 C/W
  junction-to-ambient thermal resistance.
- 125 C operating junction maximum with reverse voltage applied.
- Axial lead package with cathode indicated by the polarity band.

The bundled SPICE card is a reduced electrical fit for generated transient
plumbing and operating-limit probes. It is not a thermal-runaway, surge,
leakage, rectifier-waveform, or final production sign-off model.

## Evidence

The official onsemi PDF is retained at
`docs/research/datasheets/onsemi/1n5817_1n5818_1n5819.pdf`. Source notes and
hashes are recorded in `docs/research/datasheets/onsemi/1n5819_sources.md`.
