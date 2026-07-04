# onsemi NDS7002A Model

`vendor.onsemi.nds7002a` is a source-backed generated-SPICE MOSFET model for
onsemi's SOT-23 NDS7002A N-channel enhancement-mode MOSFET.

The model records onsemi's static board-level facts:

- 60 V drain-source voltage limit.
- +/-20 V continuous gate-source voltage limit.
- 280 mA continuous drain-current limit.
- 1.5 A unqualified pulsed-current metadata, retained but not used to waive
  continuous overstress.
- 300 mW power-dissipation limit at 25 C ambient with 2.4 mW/C derating.
- SOT-23 pinout: gate, source, drain.
- Threshold, on-resistance, capacitance, and approximate gate-charge metadata.

Generated SPICE uses a reduced preliminary Level-1 MOSFET fit and the shared
MOSFET operating-limit probes for terminal voltages, drain current, and power
dissipation. It is intended for generated-deck plumbing and source-backed
low-side switch operating-limit screening. It does not prove switching loss,
package thermal coupling, avalanche, qualified pulse-current SOA, gate-drive
margin, EMI, or final production hardware behavior.

## Evidence

The official onsemi PDF is retained at
`docs/research/datasheets/onsemi/nds7002a.pdf`. Source notes and hashes are
recorded in `docs/research/datasheets/onsemi/nds7002a_sources.md`.
