# onsemi 2N3904 Source Notes

Retrieved on 2026-07-05 from onsemi:

- Source URL:
  <https://www.onsemi.com/download/data-sheet/pdf/2n3903-d.pdf>
- Local original:
  `docs/research/datasheets/onsemi/2n3903_2n3904.pdf`
- SHA-256:
  `5e36f5a5c3e5c2b42be84b234ec256386cedc27240859b4744d88e9d42ccafa4`

Facts retained in `vendor.onsemi.npn_2n3904`:

- The datasheet covers 2N3903 and 2N3904 NPN silicon general-purpose
  transistors.
- It lists maximum ratings of 40 V `VCEO`, 60 V `VCBO`, 6 V `VEBO`, and
  200 mA continuous `IC`.
- It lists total device dissipation of 625 mW at 25 C ambient with 5 mW/C
  derating above 25 C, plus a junction/storage temperature range of -55 C to
  150 C.
- It lists TO-92 pinout as pin 1 emitter, pin 2 base, and pin 3 collector.
- For 2N3904 at 25 C, it lists DC current gain of 100 to 300 at
  `IC = 10 mA` and `VCE = 1 V`, minimum 60 at `IC = 50 mA`, 0.3 V maximum
  collector-emitter saturation at `IC = 50 mA` and `IB = 5 mA`, and 300 MHz
  minimum current-gain-bandwidth product at `IC = 10 mA`.
- It lists 4 pF maximum output capacitance and 8 pF maximum input capacitance.

Model boundary:

- The pack supports generated Board IR SPICE plumbing and source-backed
  operating-limit checks for BJT terminal voltages, collector current, and
  power dissipation.
- The bundled SPICE card is a reduced preliminary electrical fit, not a vendor
  compact model.
- It does not sign off gain spread, saturation margin across process and
  temperature, switching storage time, noise figure, package thermal coupling,
  or final production hardware behavior without vendor or bench calibration.
