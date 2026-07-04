# onsemi 1N5819 Source Notes

Retrieved on 2026-07-05 from onsemi:

- Source URL:
  <https://www.onsemi.com/download/data-sheet/pdf/1n5817-d.pdf>
- Local original:
  `docs/research/datasheets/onsemi/1n5817_1n5818_1n5819.pdf`
- SHA-256:
  `ac67aa23beb66d3e48ca1ffb6275cbe3beff1c801c65448ad2dd7863977d8307`

Facts retained in `vendor.onsemi.1n5819`:

- The datasheet covers 1N5817, 1N5818, and 1N5819 Schottky barrier rectifiers.
- It identifies the family as 1.0 A, 20 V, 30 V, and 40 V axial lead
  rectifiers for low-voltage high-frequency inverters, freewheeling diodes,
  and polarity-protection diodes.
- For 1N5819, it lists 40 V `VRRM`/`VRWM`/`VR`, 48 V `VRSM`, and 28 V
  `VR(RMS)`.
- It lists 1.0 A average rectified forward current under the stated mounting
  and temperature conditions, represented in the model as `IF_AV` for the
  existing diode operating-limit probe path.
- It lists 25 A non-repetitive peak surge current for one 60 Hz half-wave
  cycle, -65 C to 125 C operating/storage junction range with reverse voltage,
  and 150 C peak operating junction temperature with forward current.
- The model's `PD = 0.875 W` is a conservative derived ambient power screen:
  `(125 C - 55 C) / 80 C/W = 0.875 W`, using the datasheet junction maximum,
  rated-current ambient condition, and junction-to-ambient thermal resistance.
- At 25 C lead temperature, it lists maximum forward voltage for 1N5819 as
  0.34 V at 0.1 A, 0.6 V at 1 A, and 0.9 V at 3 A.
- It lists maximum reverse current at rated DC voltage as 1 mA at 25 C and
  10 mA at 100 C.

Model boundary:

- The pack supports generated Board IR SPICE plumbing and source-backed
  operating-limit checks for reverse voltage and average forward current.
- The bundled SPICE card is a reduced preliminary Schottky fit, not a vendor
  compact model.
- It does not sign off thermal-runaway margin, repetitive surge behavior,
  rectifier waveform heating, reverse leakage over temperature, soldering
  exposure, or final production hardware behavior without vendor or bench
  calibration.
