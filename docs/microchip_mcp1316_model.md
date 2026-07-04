# Microchip MCP1316T-29LE/OT Model

`vendor.microchip.mcp1316t_29le_ot` is a source-backed reset-supervisor model
for Microchip's MCP1316 2.90 V active-low push-pull voltage supervisor.

The model records Microchip's static board-level facts:

- VDD operating range from 1.0 V to 5.5 V.
- 2.90 V standard trip option with 2.828 V minimum and 2.973 V maximum over
  -40 C to +125 C.
- 280 ms conservative reset-release delay from the standard timeout maximum.
- Active-low push-pull reset output.
- Optional active-low manual-reset input and watchdog input.
- 5-lead SOT-23 package.
- 10 uA maximum operating-current class while watchdog or reset delay is active.

This model is intended for static `POWER_TREE_VALID` reset-threshold screening
and reset-release timing suggestions. It does not prove reset waveform shape,
VDD glitch immunity, watchdog behavior, manual-reset debounce, low-VDD
output-valid external circuitry, propagation delay, or final hardware reset
robustness.

## Evidence

The official Microchip PDF is retained at
`docs/research/datasheets/microchip/mcp131x_2x_voltage_supervisor.pdf`. Source
notes and hashes are recorded in
`docs/research/datasheets/microchip/mcp1316_sources.md`.
