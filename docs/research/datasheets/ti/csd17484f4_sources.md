# TI CSD17484F4 Sources

Source snapshot:

- `docs/research/datasheets/ti/csd17484f4.pdf`
- URL: https://www.ti.com/lit/gpn/CSD17484F4
- SHA256: `b9fb6128120a0901624bc9cb4454d797c20b39b44b106628389d00f853efb5e0`
- Retrieved: 2026-06-14

CircuitCI model:

- `libs/vendor/ti/discrete/csd17484f4.model.yaml`

Use notes:

- The model records datasheet ratings and switching-related parameters for exact part binding and preliminary static checks.
- No vendor or bench-calibrated SPICE model is stored with this source snapshot.
- Analog transient, gate-drive waveform, laser pulse SOA, and thermal sign-off must remain fail-closed unless a sourced SPICE model or measured waveform evidence is supplied.

TOF-R5001 relevance:

- The Altium schematic text identifies `Q2` as `CSD17484F4`.
- `Q2` is the low-side switch in the `LED-TX-` path.
- The board-specific analysis in `out/tof_r5001_analysis/analysis.md` uses this part identity together with the known `VLD = 21.8 V` and `Q9` not-mounted evidence.
