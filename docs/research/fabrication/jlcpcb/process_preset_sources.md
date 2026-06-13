# JLCPCB Process Preset Source Notes

Saved source artifacts:

- `pcb_capabilities.html` from <https://jlcpcb.com/capabilities/pcb-capabilities>
- Nuxt page assets from the same page under this directory.
- `basic_design_of_solder_mask.html` from
  <https://jlcpcb.com/blog/basic-design-of-solder-mask>
- `pcb_via_design_best_practices.html` from
  <https://jlcpcb.com/blog/pcb-via-design-best-practices>

Implemented preset:

- `jlcpcb_standard_2026_06`

Implemented default:

- `min_mask_expansion_mm: 0.05`

Rationale:

- The JLCPCB solder-mask article says solder-mask windows are generally
  0.1-0.2 mm larger overall, equivalent to 0.05-0.1 mm expansion on each side.
  CircuitCI uses the lower bound as the minimum expansion screen.
- The same article notes multilayer boards may use 1:1 solder-mask windows.
  Therefore explicit scenario parameters remain authoritative and can override
  the process preset for package-specific, layer-specific, or order-specific
  fabrication instructions.

Observed but not yet encoded as process defaults:

- The JLCPCB via article says mechanical drilling uses 0.15-6.30 mm circular
  drills, metallized slots use a smallest 0.65 mm drill, non-metallized slot
  routing uses a smallest 1.0 mm bit, and double-sided/multilayer vias have
  0.15 mm minimum inner diameter and 0.25 mm minimum outer diameter.
- The JLCPCB capability page bundle references drilling, trace, solder-mask,
  outline, and stencil capability tables, but many visible table values are
  resolved through runtime i18n keys rather than plaintext in the saved HTML.

Next source work before expanding presets:

- Pin exact text values for drill-to-edge, slot-to-edge, annular ring,
  copper-spacing, solder-mask dam, and stencil-spacing thresholds from official
  JLCPCB material or from an exported process capability document.
- Add those values only with process-condition names precise enough to avoid
  mixing standard, multilayer, HDI, via-in-pad, stencil, or special-order rules.
