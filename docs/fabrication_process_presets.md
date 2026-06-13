# Fabrication Process Presets

Manufacturing scenarios may set `parameters.fabrication_process` to use
source-backed process defaults for individual numeric thresholds. Explicit
scenario parameters always override preset defaults.

```yaml
scenarios:
  - name: solder_mask_opening
    type: manufacturing
    checks:
      - SOLDER_MASK_OPENING_VALID
    parameters:
      fabrication_process:
        - jlcpcb_standard_2026_06
        - jlcpcb_double_sided_via_min_2026_06
```

`fabrication_process` may be a single string or a list of strings. Lists are
useful when one scenario needs defaults from independent process slices. If two
listed presets ever provide different defaults for the same parameter,
validation fails closed instead of choosing one silently.

Supported aliases for the current JLCPCB standard preset:

- `jlcpcb_standard_2026_06`
- `jlcpcb_standard`
- `jlcpcb_2layer_standard_2026_06`

Supported aliases for the JLCPCB double-sided/multilayer via minimum preset:

- `jlcpcb_double_sided_via_min_2026_06`
- `jlcpcb_double_sided_via_min`
- `jlcpcb_multilayer_via_min_2026_06`

Current preset defaults:

| Parameter | Value | Source |
| --- | ---: | --- |
| `min_mask_expansion_mm` | `0.05` | JLCPCB solder-mask design article: solder-mask windows are generally 0.1-0.2 mm larger overall, equivalent to 0.05-0.1 mm per side. |
| `min_solder_mask_dam_mm` | `0.10` | JLCPCB solder-mask color article: precision LPI supports minimum solder-mask dams as small as 0.1 mm between pads. |
| `min_annular_ring_mm` | `0.05` | JLCPCB via article: double-sided/multilayer vias have 0.15 mm minimum inner diameter and 0.25 mm minimum outer diameter; `(0.25 - 0.15) / 2 = 0.05`. |

Unsupported process IDs fail closed when a rule needs a missing numeric
parameter from the preset.

Other manufacturing thresholds still require explicit scenario parameters until
the repo has pinned source evidence for the exact process condition. This is
intentional: the saved JLC capability page and assets identify many relevant
tables, but not every current table value is available in machine-readable
plaintext in the downloaded artifacts.
