# onsemi 1N4148WS Source Notes

Retrieved on 2026-06-12 from onsemi:

- Source URL:
  <https://www.onsemi.com/download/data-sheet/pdf/1n4148ws-d.pdf>
- Local original:
  `docs/research/datasheets/onsemi/1n4148ws.pdf`
- SHA-256:
  `11f014f05f4ab6ba5eddb0bd8fc0c27f49f9fc25433800d0a327595d4031f148`

Facts retained in `vendor.onsemi.1n4148ws`:

- The datasheet identifies 1N4148WS as a small-signal switching diode in an
  SOD-323 package.
- It lists `VRRM = 100 V` and average rectified forward current `IF(AV) =
  150 mA`.
- It lists total power dissipation `PD = 200 mW`.
- It lists maximum forward voltage of `1.0 V` at `IF = 10 mA`.
- It lists maximum reverse current of `5 uA` at `VR = 75 V`.
- It lists maximum total capacitance of `4 pF` at `VR = 0 V`, `f = 1 MHz`.
- It lists maximum reverse recovery time of `4 ns` at `IF = 10 mA`,
  `VR = 6 V`, `IRR = 1 mA`, and `RL = 100 ohm`.

Model boundary:

- The pack supports generated Board IR SPICE plumbing and source-backed
  operating-limit checks for repetitive reverse voltage, average forward
  current, and total power dissipation.
- The bundled SPICE card is a reduced preliminary 1N4148 switching-diode fit,
  not a vendor compact model.
- It does not sign off pulse-current derating, reverse-recovery behavior across
  process and temperature, leakage over temperature, package thermal coupling,
  or final production hardware behavior without vendor or bench calibration.
