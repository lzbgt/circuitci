# onsemi SS8550 Source Notes

Retrieved on 2026-06-12 from onsemi:

- Source URL:
  <https://www.onsemi.com/download/data-sheet/pdf/ss8550-d.pdf>
- Local original:
  `docs/research/datasheets/onsemi/ss8550.pdf`
- SHA-256:
  `82c3aab9b43a6c887d8360cf1c57e3bb89d7a5437ff01b5d0b7368340c575063`

Facts retained in `vendor.onsemi.ss8550`:

- The datasheet covers the SS8550 PNP epitaxial silicon transistor.
- It lists `VCBO = -40 V`, `VCEO = -25 V`, `VEBO = -6 V`, and `IC = -1.5 A`.
- It lists total power dissipation `PD = 1 W`.
- It lists TO-92 pinout as pin 1 emitter, pin 2 base, and pin 3 collector.
- It lists `VCE(sat) = -0.5 V` maximum at `IC = -800 mA`, `IB = -80 mA`, and
  `TC = 25 C`.
- It lists DC current gain from 120 to 400 at `VCE = -1 V`, `IC = -100 mA`,
  and `TC = 25 C`.
- It lists `Cob = 10 pF` typical at `VCB = -10 V`, `IE = 0`, `f = 1 MHz`.
- It lists transition frequency `fT = 100 MHz` typical at `VCE = -10 V`,
  `IC = -50 mA`, and `f = 100 MHz`.

Model boundary:

- The pack supports generated Board IR SPICE plumbing and source-backed
  operating-limit checks for BJT terminal voltages, collector current, and
  power dissipation.
- The bundled SPICE card is a reduced preliminary electrical fit, not a vendor
  compact model.
- It does not sign off gain spread, saturation margin across process and
  temperature, switching storage time, noise, package thermal coupling, or final
  production hardware behavior without vendor or bench calibration.
