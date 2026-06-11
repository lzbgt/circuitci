# Pulse and SOA Datasheet Sources

Downloaded on 2026-06-12 for qualified pulse operating-limit validation.

| Part | Manufacturer | Source URL | Local file | SHA-256 |
| --- | --- | --- | --- | --- |
| FDMC86184 | onsemi | https://www.onsemi.com/download/data-sheet/pdf/fdmc86184-d.pdf | `docs/research/datasheets/onsemi_fdmc86184-d.pdf` | `d338e4ae50dfd32e06bfeea148fc369220a837f36dd3b97610dcc8c33a46fa4e` |

## FDMC86184 Facts Used

The FDMC86184 datasheet maximum-ratings table lists:

- `VDS = 100 V`
- `VGS = +/-20 V`
- `ID = 12 A` continuous at `TA = 25 C`, note 1
- `IDM = 266 A` pulsed, note 4
- `PD = 2.3 W` at `TA = 25 C`, note 1

The same datasheet notes:

- pulse test width is `< 300 us`
- duty cycle is `< 2.0%`
- pulsed `ID` should refer to the SOA graph for details
- continuous current is computed from maximum junction temperature and practical
  current is limited by board thermal/electromechanical design

CircuitCI encodes the scalar pulse current, pulse width, duty cycle, and
preliminary hand-digitized Figure 11 SOA points in `vendor.onsemi.fdmc86184`.
The hand-digitized points are intentionally marked as preliminary model
evidence. They are useful for automated board review and regression tests, but
they are not a substitute for vendor machine-readable SOA data or bench
calibration.

Recorded Figure 11 points:

| Curve | VDS/ID points |
| --- | --- |
| 100 us | `(1 V, 200 A)`, `(10 V, 90 A)`, `(20 V, 45 A)`, `(50 V, 12 A)`, `(100 V, 3 A)` |
| 1 ms | `(1 V, 160 A)`, `(10 V, 40 A)`, `(20 V, 18 A)`, `(50 V, 5 A)`, `(100 V, 1 A)` |

For power derating, the model records `PD = 2.3 W` at `TA = 25 C` and
`derating_per_c = 0.0184 W/C`, derived from the datasheet's `RthetaJA = 53 C/W`
and `TJ(max) = 150 C`.
