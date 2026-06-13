# Gerber Solder-Paste Importer

`circuitci import-gerber-solder-paste` appends solder-paste stencil evidence
from a Gerber paste layer into an existing Board IR project.

Supported subset:

- RS-274X millimeter units (`%MOMM*%`)
- absolute coordinates (`G90`, the default)
- dark polarity flashes (`D03`) for circle, rectangle, oval, and observed
  EasyEDA `RoundRect` apertures
- dark linear and sampled `G02`/`G03` arc draws with circular apertures
- dark single-contour regions

Imported evidence is written under `board.layout.solder_paste.features`,
`board.layout.solder_paste.segments`, and `board.layout.solder_paste.regions`.
Dark solder-paste primitives are treated as stencil openings.
Multi-contour, nested, open, degenerate, or flashed regions fail closed rather
than being approximated as stencil openings.

When the input project already contains PCB layout pad evidence, flash, draw,
and region openings can inherit conservative owner metadata: `net`,
`owner_kind: pad`,
`component`, and `pin` from a unique matching pad on the corresponding copper
layer. Via ownership is intentionally not copied into paste openings because
vias are not normally paste-bearing stencil features.

For ownership matching, EasyEDA `TopPasteMaskLayer` and
`BottomPasteMaskLayer` are treated as stencil openings above `F.Cu` and
`B.Cu`.

The CLI summary reports owner-associated flash, draw, and region opening
counts. A zero owner-associated count means the Gerber paste primitives were
still imported, but the input project did not yet contain unique pad layout
evidence for those openings.

Example:

```bash
circuitci import-gerber-solder-paste fabrication/F_Paste.gtp \
  --project out/imported_with_mask.project.yaml \
  --output out/imported_with_paste.project.yaml
```

`SOLDER_PASTE_OPENING_VALID` consumes flash, circular-aperture draw, and
single-contour region opening evidence from `board.layout.solder_paste`
together with Gerber copper flashes under `board.layout.copper.features`. It
maps `F.Cu` to `F.Paste` and `B.Cu` to `B.Paste`, then checks that co-located
paste aperture area stays within scenario-provided min/max area-ratio bounds.
When multiple paste openings are co-located with the same copper flash, their
areas are summed before checking the ratio, which supports static screening of
windowed exposed-pad stencil patterns.

`SOLDER_PASTE_SPACING_VALID` consumes the same solder-paste feature, segment,
and region opening evidence to check same-layer opening-to-opening spacing for
static stencil manufacturability.

`SOLDER_PASTE_APERTURE_SIZE_VALID` consumes solder-paste flash features and
circular-aperture draw segments to check the narrow opening dimension against a
stencil fabrication floor. With
`fabrication_process: jlcpcb_stencil_aperture_min_2026_06`, apertures must be
greater than the source-backed JLCPCB 0.08 mm minimum aperture size. Arbitrary
region openings are not used for this minimum-width screen yet because the rule
does not approximate polygon neck width.

`SOLDER_PASTE_APERTURE_AREA_RATIO_VALID` consumes solder-paste flash features,
circular-aperture draw segments, and single-contour regions. With
`fabrication_process: jlcpcb_stencil_area_ratio_2026_06`, apertures must meet
the source-backed `0.66` JLCPCB/IPC-7525 area-ratio floor; scenarios must also
provide `stencil_thickness_mm` because area ratio is aperture opening area
divided by aperture wall area.

`SOLDER_PASTE_IC_PIN_APERTURE_VALID` and `SOLDER_PASTE_BGA_APERTURE_VALID`
consume pad-owned solder-paste evidence for package-specific JLCPCB stencil
opening table rows. IC checks can use paste features, circular draw openings,
and single-contour regions. BGA checks intentionally use flash features only,
because the source table gives ball-grid aperture sizes rather than arbitrary
draw or polygon paste geometry.

The importer intentionally does not infer package-specific stencil reductions,
step-stencil process rules, paste volume, or paste-bearing pad intent from
Gerber alone. Use pad or Gerber ownership evidence where available, and tune
`SOLDER_PASTE_OPENING_VALID` and `SOLDER_PASTE_SPACING_VALID` thresholds to the
package and fabricator process.
