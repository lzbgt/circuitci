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

`SOLDER_PASTE_SPACING_VALID` consumes the same solder-paste feature, segment,
and region opening evidence to check same-layer opening-to-opening spacing for
static stencil manufacturability.

The importer intentionally does not infer package-specific stencil reductions,
step-stencil process rules, paste windowing for exposed pads, or paste-bearing
pad intent. Use pad or Gerber ownership evidence where available, and tune
`SOLDER_PASTE_OPENING_VALID` and `SOLDER_PASTE_SPACING_VALID` thresholds to the
package and fabricator process.
