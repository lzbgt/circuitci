# Microchip MCP1316T-29LE/OT Source Notes

Retrieved on 2026-07-05 from Microchip:

- Source URL:
  <https://ww1.microchip.com/downloads/aemDocuments/documents/APID/ProductDocuments/DataSheets/MCP131X-2X-Voltage-Supervisor-DS20001985.pdf>
- Local original:
  `docs/research/datasheets/microchip/mcp131x_2x_voltage_supervisor.pdf`
- SHA-256:
  `234c38794c2dc0e5bfbedd521d1705af020129a21a3c3574392bdc86e780c21e`

Facts retained in `vendor.microchip.mcp1316t_29le_ot`:

- The datasheet covers MCP131X/2X voltage supervisors.
- MCP1316 is a push-pull active-low reset-output device with manual reset and
  watchdog inputs.
- The standard ordering table lists `MCP1316T-29LE/OT` as a 2.90 V threshold
  option with `140 ms` minimum and `200 ms` typical reset timeout, plus `1.12 s`
  minimum and `1.6 s` typical watchdog timeout.
- The timing table lists the standard reset active time as `140 ms` minimum,
  `200 ms` typical, and `280 ms` maximum; the model uses `280000 us` as the
  conservative release-delay metadata.
- The trip-point table lists `MCP13XX-29` over `-40 C` to `+125 C` as
  `2.828 V` minimum, `2.90 V` typical, and `2.973 V` maximum.
- The recommended operating range is `1.0 V` to `5.5 V`.
- The operating current table lists up to `10 uA` while the watchdog or reset
  delay timer is active, and up to `2 uA` while the watchdog is inactive.
- The absolute-maximum table lists `7.0 V` supply voltage, `10 mA` VDD input
  current, `10 mA` reset-output current, `-65 C` to `+150 C` storage
  temperature, `-40 C` to `+125 C` ambient with power applied, `+150 C`
  junction temperature, `240 mW` 5-pin SOT-23A dissipation, and at least `4 kV`
  ESD protection on all pins.

Model boundary:

- The pack supports static reset-supervisor rail-threshold and reset-release
  timing screening.
- The bundled generated-SPICE face is reduced to preliminary reset-observation
  plumbing.
- It does not sign off reset waveform shape, VDD glitch immunity, watchdog
  protocol behavior, manual-reset debounce, low-VDD output-valid external
  circuitry, propagation delay, or final hardware reset robustness.
