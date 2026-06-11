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

CircuitCI encodes only the scalar pulse current, pulse width, and duty cycle in
`vendor.onsemi.fdmc86184`. It does not digitize or enforce Figure 11 SOA curve
points yet. Reports and model-quality metadata must therefore state that the
fixture validates qualified short-pulse current handling, not full SOA sign-off.

For power derating, the model records `PD = 2.3 W` at `TA = 25 C` and
`derating_per_c = 0.0184 W/C`, derived from the datasheet's `RthetaJA = 53 C/W`
and `TJ(max) = 150 C`.
