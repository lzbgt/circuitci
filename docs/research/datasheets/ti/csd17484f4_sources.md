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
- `ID_pulsed = 18 A` is encoded with the datasheet condition
  `pulse_width_us = 100` and `duty_cycle_max = 0.01`.
- `PD = 0.5 W` at `TA = 25 C` is encoded with linear derating
  `derating_per_c = 0.004`, derived from derating to zero at `150 C`.
- `models/spice/ti/csd17484f4.lib` is a coarse Level-1 datasheet-fit card for generated-deck plumbing and preliminary low-side switch checks.
- No vendor or bench-calibrated SPICE model is stored with this source snapshot.
- Gate-drive waveform, laser pulse SOA, and thermal sign-off must remain non-final unless a sourced vendor model, bench-calibrated fit, or measured waveform evidence is supplied.

TOF-R5001 relevance:

- The Altium schematic text identifies `Q2` as `CSD17484F4`.
- `Q2` is the low-side switch in the `LED-TX-` path.
- The board-specific analysis in `out/tof_r5001_analysis/analysis.md` uses this part identity together with the known `VLD = 21.8 V` and `Q9` not-mounted evidence.
