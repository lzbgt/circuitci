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
- `pcb_stencil_manufacturing.html` from
  <https://jlcpcb.com/capabilities/pcb-stencil-manufacturing>
- `opening_process_standard_of_stencil.html` from
  <https://jlcpcb.com/help/article/opening-process-standard-of-stencil>
- `Hole_to_board_edge.892a998.png` from
  <https://jlcpcb.com/ssr/img/Hole_to_board_edge.892a998.png>

Implemented preset:

- `jlcpcb_standard_2026_06`
- `jlcpcb_double_sided_via_min_2026_06`
- `jlcpcb_slot_min_2026_06`
- `jlcpcb_drill_diameter_range_2026_06`
- `jlcpcb_1oz_copper_spacing_2026_06`
- `jlcpcb_routed_edge_copper_clearance_2026_06`
- `jlcpcb_stencil_aperture_min_2026_06`

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
- `min_solder_paste_aperture_size_mm: 0.08`

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
- The saved JLCPCB stencil capability page bundle resolves the stencil table:
  `i18n_web_app_capabilities_stencil_feature_3` is "Minimum Aperture Size" and
  `i18n_web_app_capabilities_stencil_capabilities_3` is `>0.08mm`. CircuitCI
  encodes `min_solder_paste_aperture_size_mm: 0.08` as a narrowly named
  laser-cut stencil aperture floor. The validator rejects apertures at or below
  0.08 mm to preserve the source's greater-than wording.
- The saved JLCPCB stencil opening-process standard gives package-specific
  aperture optimization examples for ICs, BGAs, connectors, and through-hole
  pads. CircuitCI does not encode those values as generic
  `min_paste_area_ratio`, `max_paste_area_ratio`, or
  `min_solder_paste_spacing_mm` defaults because they depend on package pitch,
  pad geometry, and order instructions.

Observed but not yet encoded as process defaults:

- The JLCPCB capability page bundle references drilling, trace, solder-mask,
  outline, and stencil capability tables. Some values, including the 1 oz trace
  spacing and routed-edge copper clearance rows, are available through runtime
  i18n keys in the saved JavaScript; other current table values still need
  exact extraction or separate source evidence before encoding.
- The saved JLCPCB capability page JavaScript contains castellated-hole text
  with "Hole to board edge" requirements, including
  `i18n_web_app_211_1` with `Hole to board edge (L): >= 1 mm`. The referenced
  saved diagram, `Hole_to_board_edge.892a998.png`, labels a different
  castellated-pad condition: `Castellated pad to board edge >= 0.5 mm`,
  `Castellated hole diameter >= 0.3 mm`, and `Castellated hole to hole >=
  0.4 mm`. CircuitCI does not encode either value as
  `min_drill_edge_clearance_mm` because the current rule measures generic
  circular drill edge-to-outline clearance, while the source evidence is
  specifically castellated-pad/castellated-hole geometry and the text/image
  conditions are not the same.
- The saved JLCPCB stencil opening-process article gives package- and
  pitch-specific aperture optimization examples for IC, BGA, connector,
  high-power transistor, through-hole, and red-glue stencil cases. It does not
  provide a single generic solder-paste area ratio or paste-aperture spacing
  floor suitable for all `SOLDER_PASTE_OPENING_VALID` or
  `SOLDER_PASTE_SPACING_VALID` scenarios.
- CircuitCI implements the IC pitch part of that stencil opening-process
  article as the opt-in `SOLDER_PASTE_IC_PIN_APERTURE_VALID` check. The rule
  requires explicit `pin_pitch_mm` and pad-owned paste evidence, then applies
  only the source-backed IC pitch rows: 0.8-1.27 mm pitch uses width 45%-60% of
  pitch; 0.635-0.65 mm pitch uses 0.30-0.33 mm; 0.5 mm uses 0.24 mm; 0.4 mm
  uses 0.19 mm; 0.35 mm uses 0.17 mm; and 0.3 mm uses 0.16 mm. This is not a
  fabrication-process preset because the condition is package-class and pitch
  specific.
- `suggest-scenarios` may infer `pin_pitch_mm` only when imported pad-owned
  solder-paste flashes for one component show at least two repeated gaps
  matching the discrete 0.3, 0.35, 0.4, 0.5, or 0.65 mm source rows. It may
  also infer representative exact 0.8, 1.0, or 1.27 mm pitches from the
  source-backed 0.8-1.27 mm IC table row, but only when one component has at
  least three repeated same-pitch gaps. The generated scenario is
  target-scoped to that component, so the inferred pitch is not applied to
  unrelated pad-owned paste elsewhere on the board.

Next source work before expanding presets:

- Pin exact text values for generic drill-to-edge, slot-to-edge,
  paste-area-ratio, and stencil-spacing thresholds from official JLCPCB
  material, package stencil guidance, or an exported process capability
  document. Castellated-pad edge values should become a separate
  castellated-specific rule or preset only after the Board IR can identify that
  condition explicitly.
- Add those values only with process-condition names precise enough to avoid
  mixing standard, multilayer, HDI, via-in-pad, stencil, or special-order rules.
