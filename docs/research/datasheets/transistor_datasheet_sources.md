# Transistor Datasheet Sources

Downloaded on 2026-06-12 for analog model provenance.

| Part | Manufacturer | Source URL | Local file | SHA-256 |
| --- | --- | --- | --- | --- |
| SS8050 | onsemi | https://www.onsemi.com/download/data-sheet/pdf/ss8050-d.pdf | `docs/research/datasheets/onsemi_ss8050-d.pdf` | `50d1896f9ea662a42c176077b3e87b81a0b561f128b8642bdc6804d3fafb1188` |
| SS8550 | onsemi | https://www.onsemi.com/download/data-sheet/pdf/ss8550-d.pdf | `docs/research/datasheets/onsemi_ss8550-d.pdf` | `82c3aab9b43a6c887d8360cf1c57e3bb89d7a5437ff01b5d0b7368340c575063` |

The current SPICE cards in `models/spice/onsemi/ss8050_ss8550.lib` are
datasheet-fit placeholders. They are sufficient to exercise model provenance
plumbing, but final physical acceptance requires vendor SPICE models, calibrated
fits, or bench-validated parameters for the actual board population.
