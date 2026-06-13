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

Supported aliases for the JLCPCB routed-slot minimum preset:

- `jlcpcb_slot_min_2026_06`
- `jlcpcb_slot_min`

Supported aliases for the JLCPCB circular drill diameter range preset:

- `jlcpcb_drill_diameter_range_2026_06`
- `jlcpcb_drill_diameter_range`

Supported aliases for the JLCPCB 1 oz copper spacing preset:

- `jlcpcb_1oz_copper_spacing_2026_06`
- `jlcpcb_1oz_copper_spacing`
- `jlcpcb_1oz_trace_spacing_2026_06`

Current preset defaults:

| Parameter | Value | Source |
| --- | ---: | --- |
| `min_mask_expansion_mm` | `0.05` | JLCPCB solder-mask design article: solder-mask windows are generally 0.1-0.2 mm larger overall, equivalent to 0.05-0.1 mm per side. |
| `min_solder_mask_dam_mm` | `0.10` | JLCPCB solder-mask color article: precision LPI supports minimum solder-mask dams as small as 0.1 mm between pads. |
| `min_annular_ring_mm` | `0.05` | JLCPCB via article: double-sided/multilayer vias have 0.15 mm minimum inner diameter and 0.25 mm minimum outer diameter; `(0.25 - 0.15) / 2 = 0.05`. |
| `min_plated_slot_width_mm` | `0.65` | JLCPCB via article: smallest slot drill size is 0.65 mm for metallized slots. |
| `min_non_plated_slot_width_mm` | `1.00` | JLCPCB via article: smallest non-metallized slot routing bit is 1.0 mm. |
| `min_drill_diameter_mm` | `0.15` | JLCPCB via article: circular drill bits range from 0.15 mm to 6.30 mm in diameter. |
| `max_drill_diameter_mm` | `6.30` | JLCPCB via article: circular drill bits range from 0.15 mm to 6.30 mm in diameter. |
| `min_copper_spacing_mm` | `0.10` | JLCPCB PCB capability page: 1 oz minimum track width and spacing is 0.10 / 0.10 mm, and pad-to-track clearance is 0.1 mm. |

Unsupported process IDs fail closed when a rule needs a missing numeric
parameter from the preset.

Other manufacturing thresholds still require explicit scenario parameters until
the repo has pinned source evidence for the exact process condition. The 1 oz
copper spacing preset is a general fabrication floor for `COPPER_SPACING_VALID`;
package-specific SMD pad spacing, stencil aperture spacing, board-edge
clearance, and special-order constraints still need narrower presets or explicit
scenario parameters.
