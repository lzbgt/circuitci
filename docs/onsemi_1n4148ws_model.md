# onsemi 1N4148WS Model

`vendor.onsemi.1n4148ws` is a source-backed generated-SPICE diode model for
onsemi's SOD-323 1N4148WS small-signal switching diode.

The model records onsemi's static board-level facts:

- 100 V repetitive peak reverse-voltage limit.
- 150 mA average rectified forward-current limit.
- 200 mW total power-dissipation limit.
- 1.0 V maximum forward voltage at 10 mA.
- 5 uA maximum reverse current at 75 V.
- 4 pF maximum capacitance and 4 ns maximum reverse-recovery metadata.
- SOD-323 anode/cathode pin boundary.

Generated SPICE uses a reduced preliminary 1N4148 diode fit and the shared
diode operating-limit probes for reverse voltage, forward current, and power
dissipation. It is intended for generated-deck plumbing and source-backed
operating-limit screening. It does not prove pulse-current derating,
reverse-recovery behavior across process and temperature, leakage over
temperature, package thermal coupling, or final production hardware behavior.

## Evidence

The official onsemi PDF is retained at
`docs/research/datasheets/onsemi/1n4148ws.pdf`. Source notes and hashes are
recorded in `docs/research/datasheets/onsemi/1n4148ws_sources.md`.
