# Gerber Solder-Mask Importer

`circuitci import-gerber-solder-mask` appends solder-mask fabrication evidence
from a Gerber solder-mask layer into an existing Board IR project.

The importer uses the same conservative Gerber subset as the copper importer:

- millimeter units (`%MOMM*%`),
- absolute coordinates,
- linear interpolation,
- dark `D03` flashes for circle, rectangle, and oval apertures,
- dark circular-aperture linear `D01` draw openings,
- dark single-contour linear `G36`/`G37` region openings.

Imported evidence is written under `board.layout.solder_mask.features`,
`board.layout.solder_mask.segments`, and `board.layout.solder_mask.regions`.
Dark solder-mask primitives are treated as openings in the solder-mask layer.
Clear-polarity primitives are counted and skipped.

Example:

```bash
circuitci import-gerber-solder-mask fabrication/F_Mask.gts \
  --project with_copper.project.yaml \
  --output with_mask.project.yaml
```

`SOLDER_MASK_OPENING_VALID` currently consumes flash-opening evidence from
`board.layout.solder_mask.features` together with Gerber copper flashes under
`board.layout.copper.features`. It fails when a copper flash has no co-located
same-side mask opening or when the opening expansion is below
`parameters.min_mask_expansion_mm`.

`SOLDER_MASK_DAM_VALID` consumes same-layer flash, circular-aperture draw, and
region opening evidence from `board.layout.solder_mask`. It fails when adjacent
imported openings leave less mask web than
`parameters.min_solder_mask_dam_mm`.

The importer intentionally does not infer nets, component pins, or solder-mask
rules from Gerber alone. Use separate PCB or fabrication evidence when owner or
package-specific mask semantics matter.
