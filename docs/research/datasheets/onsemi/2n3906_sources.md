# onsemi 2N3906 Source Notes

Retrieved on 2026-07-05 from onsemi:

- Source URL:
  <https://www.onsemi.com/download/data-sheet/pdf/2n3906-d.pdf>
- Local original: `docs/research/datasheets/onsemi/2n3906.pdf`
- SHA-256:
  `da8fc11b3e356ef926445fc378f2918bd9add3104700137d88974501bd0f4795`

Facts retained in `vendor.onsemi.pnp_2n3906`:

- The datasheet covers the 2N3906 PNP silicon general-purpose transistor.
- It lists maximum ratings of 40 V `VCEO`, 40 V `VCBO`, 5 V `VEBO`, and
  200 mA continuous `IC`.
- It lists total device dissipation of 625 mW at 25 C ambient with 5 mW/C
  derating above 25 C, total power dissipation of 250 mW at 60 C ambient, and
  a junction/storage temperature range of -55 C to 150 C.
- It lists TO-92 pinout as pin 1 emitter, pin 2 base, and pin 3 collector.
- At 25 C, it lists DC current gain of 100 to 300 at `IC = 10 mA` and
  `VCE = 1 V`, minimum 60 at `IC = 50 mA`, 0.4 V maximum collector-emitter
  saturation at `IC = 50 mA` and `IB = 5 mA`, and 250 MHz minimum
  current-gain-bandwidth product at `IC = 10 mA`.
- It lists 4.5 pF maximum output capacitance and 10 pF maximum input
  capacitance.

Model boundary:

- The pack supports generated Board IR SPICE plumbing and source-backed
  operating-limit checks for BJT terminal voltages, collector current, and
  power dissipation.
- The bundled SPICE card is a reduced preliminary electrical fit, not a vendor
  compact model.
- It does not sign off gain spread, saturation margin across process and
  temperature, switching storage time, noise figure, package thermal coupling,
  or final production hardware behavior without vendor or bench calibration.
