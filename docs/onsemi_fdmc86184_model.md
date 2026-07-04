# onsemi FDMC86184 Model

`vendor.onsemi.fdmc86184` is a source-backed generated-SPICE MOSFET model for
onsemi's WDFN-8 FDMC86184 N-channel Shielded Gate PowerTrench MOSFET.

The model records onsemi's static and pulse/SOA board-level facts:

- 100 V drain-source voltage limit.
- +/-20 V continuous gate-source voltage limit.
- 12 A continuous drain-current limit at 25 C ambient.
- 266 A pulsed drain-current metadata, usable only with the encoded
  `< 300 us` pulse width and `< 2.0%` duty-cycle constraints.
- 2.3 W power-dissipation limit at 25 C ambient with 18.4 mW/C derating.
- WDFN-8 gate, drain, and source pin roles.
- Threshold, on-resistance, capacitance, and gate-charge metadata.
- Preliminary hand-digitized Figure 11 forward-bias SOA points for 100 us and
  1 ms curves.

Generated SPICE uses a reduced preliminary Level-1 MOSFET fit and the shared
MOSFET operating-limit probes for terminal voltages, continuous/pulsed drain
current, derated power dissipation, and digitized SOA screening. It is intended
for generated-deck plumbing and source-backed pulse/SOA regression coverage.
It does not prove switching loss, package thermal coupling, avalanche,
machine-readable vendor SOA, gate-drive margin, EMI, or final production
hardware behavior.

## Evidence

The official onsemi PDF is retained at
`docs/research/datasheets/onsemi/fdmc86184.pdf`. Source notes and hashes are
recorded in `docs/research/datasheets/onsemi/fdmc86184_sources.md`; the
retained pulse/SOA evidence table remains in
`docs/research/datasheets/pulse_soa_datasheet_sources.md`.
