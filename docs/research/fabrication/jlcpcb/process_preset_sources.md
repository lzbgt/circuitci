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
- `jlcpcb_1oz_copper_spacing_2026_06`
- `jlcpcb_routed_edge_copper_clearance_2026_06`

Implemented default:

- `min_mask_expansion_mm: 0.05`
- `min_solder_mask_dam_mm: 0.10`
- `min_annular_ring_mm: 0.05`
- `min_plated_slot_width_mm: 0.65`
- `min_non_plated_slot_width_mm: 1.00`
- `min_drill_diameter_mm: 0.15`
- `max_drill_diameter_mm: 6.30`
- `min_copper_spacing_mm: 0.10`
- `min_copper_edge_clearance_mm: 0.20`

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
- The saved JLCPCB PCB capability page bundle resolves the traces table rows in
  `6874ee7eeb2cbc6b6a3f.js`: `i18n_web_app_232` is "Min. track width and
  spacing (1 oz)" with capability `0.10 / 0.10 mm (4 / 4 mil)`, and
  `i18n_web_app_244` is "Pad to track clearance" with capability `0.1mm`.
  CircuitCI encodes `min_copper_spacing_mm: 0.10` as a narrowly named 1 oz
  copper-spacing preset. It is intentionally not used as a package-specific SMD
  pad-spacing or stencil-spacing rule.
- The saved JLCPCB PCB capability page bundle also resolves the outline table:
  `i18n_web_app_274` is "Routed", `i18n_web_app_274_a` gives capability
  `0.2mm`, and `i18n_web_app_277` says copper clearance from routed board
  edges and routed slots is at least 0.2 mm. CircuitCI encodes
  `min_copper_edge_clearance_mm: 0.20` as a routed-edge copper clearance preset.
  `i18n_web_app_279` separately states the same 0.2 mm copper clearance from
  non-mouse-bite board edges. V-cut copper clearance is not encoded in this
  preset because V-cut panelization has different edge semantics.

Observed but not yet encoded as process defaults:

- The JLCPCB capability page bundle references drilling, trace, solder-mask,
  outline, and stencil capability tables. Some values, including the 1 oz trace
  spacing and routed-edge copper clearance rows, are available through runtime
  i18n keys in the saved JavaScript; other current table values still need
  exact extraction or separate source evidence before encoding.

Next source work before expanding presets:

- Pin exact text values for drill-to-edge, slot-to-edge, paste-area-ratio, and
  stencil-spacing thresholds from official JLCPCB material, package stencil
  guidance, or an exported process capability document.
- Add those values only with process-condition names precise enough to avoid
  mixing standard, multilayer, HDI, via-in-pad, stencil, or special-order rules.
