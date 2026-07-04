# onsemi BSS84 Model

`vendor.onsemi.bss84` is a source-backed generated-SPICE MOSFET model for
onsemi's SOT-23 BSS84 P-channel enhancement-mode MOSFET.

The model records onsemi's static board-level facts:

- -50 V drain-source voltage limit.
- +/-20 V continuous gate-source voltage limit.
- -130 mA continuous drain-current limit.
- -520 mA unqualified pulsed-current metadata, retained but not used to waive
  continuous overstress.
- 360 mW power-dissipation limit at 25 C ambient with 2.88 mW/C derating.
- SOT-23 pinout: gate, source, drain.
- Threshold, on-resistance, capacitance, and gate-charge metadata.

Generated SPICE uses a reduced preliminary Level-1 MOSFET fit and the shared
MOSFET operating-limit probes for terminal voltages, drain current, and power
dissipation. It is intended for generated-deck plumbing and source-backed
high-side switch operating-limit screening. It does not prove switching loss,
package thermal coupling, avalanche, qualified pulse-current SOA, gate-drive
margin, EMI, or final production hardware behavior.

## Evidence

The official onsemi PDF is retained at
`docs/research/datasheets/onsemi/bss84.pdf`. Source notes and hashes are
recorded in `docs/research/datasheets/onsemi/bss84_sources.md`.
