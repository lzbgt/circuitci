# Gerber Importer Modules

The Gerber importer is split so parser growth and evidence association stay
separate.

## `src/importers/gerber.rs`

Owns the CLI-facing Gerber import flows:

- RS-274X outline parsing,
- RS-274X copper flash/draw/region parsing,
- import summaries and fail-closed parser errors,
- shared outline-contour geometry helpers.

## `src/importers/gerber/board_ir.rs`

Owns Board IR projection for parsed Gerber evidence:

- outline YAML serialization,
- copper, solder-mask, and solder-paste YAML serialization,
- import summary construction for parsed outline and artwork evidence.

## `src/importers/gerber/ownership.rs`

Owns conservative copper ownership association for imported Gerber copper.
It can annotate copper when existing Board IR layout evidence has exactly one
matching owner:

- `net` from pad overlap in `board.layout.pads`,
- `net` from route overlap in `board.layout.routes`,
- `net` and zone-derived `island_id` from zone containment in
  `board.layout.zones`.
- solder-mask flash-opening owner metadata from unique pad or via overlap on
  the corresponding copper layer,
- solder-paste flash-opening owner metadata from unique pad overlap on the
  corresponding copper layer.

It intentionally does not infer new connectivity, component ownership, or pad
names from Gerber data alone. Ambiguous or missing ownership evidence leaves
imported Gerber artwork anonymous.
