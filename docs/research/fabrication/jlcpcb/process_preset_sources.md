# JLCPCB Process Preset Source Notes

Saved source artifacts:

- `pcb_capabilities.html` from <https://jlcpcb.com/capabilities/pcb-capabilities>
- Nuxt page assets from the same page under this directory.
- `basic_design_of_solder_mask.html` from
  <https://jlcpcb.com/blog/basic-design-of-solder-mask>
- `pcb_solder_mask_colors_performance.html` from
  <https://jlcpcb.com/blog/pcb-solder-mask-colors-performance>
- `pcb_solder_mask_expansion_guide.html` from
  <https://jlcpcb.com/blog/pcb-solder-mask-expansion-guide>
- `pcb_via_design_best_practices.html` from
  <https://jlcpcb.com/blog/pcb-via-design-best-practices>

Implemented preset:

- `jlcpcb_standard_2026_06`
- `jlcpcb_double_sided_via_min_2026_06`
- `jlcpcb_slot_min_2026_06`
- `jlcpcb_drill_diameter_range_2026_06`

Implemented default:

- `min_mask_expansion_mm: 0.05`
- `min_solder_mask_dam_mm: 0.10`
- `min_annular_ring_mm: 0.05`
- `min_plated_slot_width_mm: 0.65`
- `min_non_plated_slot_width_mm: 1.00`
- `min_drill_diameter_mm: 0.15`
- `max_drill_diameter_mm: 6.30`

Rationale:

- The JLCPCB solder-mask article says solder-mask windows are generally
  0.1-0.2 mm larger overall, equivalent to 0.05-0.1 mm expansion on each side.
  CircuitCI uses the lower bound as the minimum expansion screen.
- The same article notes multilayer boards may use 1:1 solder-mask windows.
  Therefore explicit scenario parameters remain authoritative and can override
  the process preset for package-specific, layer-specific, or order-specific
  fabrication instructions.
- The JLCPCB solder-mask color article says JLCPCB precision LPI supports
  minimum solder-mask dams as small as 0.1 mm between pads. CircuitCI uses
  that value for `min_solder_mask_dam_mm`.
- The JLCPCB solder-mask expansion guide also mentions 0.075 mm minimum dam
  width for reliable curing in high-density designs. CircuitCI does not encode
  0.075 mm into the standard JLCPCB preset because the 0.1 mm statement is the
  clearer JLCPCB process capability floor.
- The JLCPCB via article says double-sided and multilayer boards have
  0.15 mm minimum via inner diameter and 0.25 mm minimum via outer diameter.
  CircuitCI derives `min_annular_ring_mm = (0.25 - 0.15) / 2 = 0.05` for the
  dedicated double-sided/multilayer via minimum preset. This is not folded into
  the standard preset because component holes, slots, via-in-pad, and special
  HDI processes have different constraints.
- The same via article says the smallest slot drill size is 0.65 mm for
  metallized slots and the smallest non-metallized slot routing bit is 1.0 mm.
  CircuitCI encodes those as a dedicated routed-slot minimum preset.
- The same via article says mechanical drilling uses circular drill bits from
  0.15 mm to 6.30 mm in diameter. CircuitCI encodes those as a dedicated
  circular drill diameter range preset because routed slots and special-order
  drill processes are separate process conditions.

Observed but not yet encoded as process defaults:

- The JLCPCB capability page bundle references drilling, trace, solder-mask,
  outline, and stencil capability tables, but many visible table values are
  resolved through runtime i18n keys rather than plaintext in the saved HTML.

Next source work before expanding presets:

- Pin exact text values for drill-to-edge, slot-to-edge, annular ring,
  copper-spacing, and stencil-spacing thresholds from official JLCPCB material
  or from an exported process capability document.
- Add those values only with process-condition names precise enough to avoid
  mixing standard, multilayer, HDI, via-in-pad, stencil, or special-order rules.
