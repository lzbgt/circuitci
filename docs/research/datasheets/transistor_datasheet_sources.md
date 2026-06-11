# Transistor Datasheet Sources

Downloaded on 2026-06-12 for analog model provenance.

| Part | Manufacturer | Source URL | Local file | SHA-256 |
| --- | --- | --- | --- | --- |
| SS8050 | onsemi | https://www.onsemi.com/download/data-sheet/pdf/ss8050-d.pdf | `docs/research/datasheets/onsemi_ss8050-d.pdf` | `50d1896f9ea662a42c176077b3e87b81a0b561f128b8642bdc6804d3fafb1188` |
| SS8550 | onsemi | https://www.onsemi.com/download/data-sheet/pdf/ss8550-d.pdf | `docs/research/datasheets/onsemi_ss8550-d.pdf` | `82c3aab9b43a6c887d8360cf1c57e3bb89d7a5437ff01b5d0b7368340c575063` |
| NDS7002A | onsemi | https://www.onsemi.com/download/data-sheet/pdf/nds7002a-d.pdf | `docs/research/datasheets/onsemi_nds7002a-d.pdf` | `160c0e7cdbee397ba4490112aa442e0df20f159c21519e2e17ae52456152e38e` |

The current SPICE cards in `models/spice/onsemi/ss8050_ss8550.lib` are
datasheet-fit placeholders. They are sufficient to exercise model provenance
plumbing, but final physical acceptance requires vendor SPICE models, calibrated
fits, or bench-validated parameters for the actual board population.

The current NDS7002A SPICE card in `models/spice/onsemi/nds7002a.lib` is also a
datasheet-fit placeholder. Its metadata records the datasheet values used for
simulation relevance:

- `VDSS = 60 V`, `VGSS = +/-20 V`, `ID = 280 mA` continuous for NDS7002A.
- `VGS(th) = 1.0 V min, 2.1 V typ, 2.5 V max` at `VDS = VGS`, `ID = 250 uA`.
- `RDS(on) = 1.2 ohm typ, 2.0 ohm max` at `VGS = 10 V`, `ID = 500 mA`.
- `RDS(on) = 1.7 ohm typ, 3.0 ohm max` at `VGS = 5 V`, `ID = 50 mA`.
- `Ciss = 20 pF typ, 50 pF max`, `Coss = 11 pF typ, 25 pF max`, and
  `Crss = 4 pF typ, 5 pF max` at `VDS = 25 V`, `VGS = 0 V`, `f = 1 MHz`.
- `Qg ~= 1.4 nC` is an approximate read from datasheet Figure 10 at
  `VDS = 25 V`, `ID = 500 mA`.
