# Diode Datasheet Sources

Downloaded on 2026-06-12 for analog model provenance.

| Part | Manufacturer | Source URL | Local file | SHA-256 |
| --- | --- | --- | --- | --- |
| 1N4148WS | onsemi | https://www.onsemi.com/download/data-sheet/pdf/1n4148ws-d.pdf | `docs/research/datasheets/onsemi_1n4148ws-d.pdf` | `11f014f05f4ab6ba5eddb0bd8fc0c27f49f9fc25433800d0a327595d4031f148` |

The onsemi 1N4148WS metadata records these datasheet-backed operating limits
for generated SPICE operating-limit validation:

- `VRRM = 100 V`
- `IF_AV = 0.15 A`
- `PD = 0.2 W`

The current SPICE card in `models/spice/onsemi/1n4148ws.lib` is a preliminary
fit reused from the generic 1N4148 switching-diode model. It is sufficient to
exercise generated-deck plumbing and operating-limit report evidence, but final
physical sign-off requires a vendor SPICE model, calibrated fit, or bench
validation for the actual board population.
