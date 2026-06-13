# Gerber Importer Modules

The Gerber importer is split so parser growth and evidence association stay
separate.

## `src/importers/gerber.rs`

Owns the CLI-facing Gerber import flows:

- RS-274X outline parsing and Board IR outline serialization,
- RS-274X copper flash/draw/region parsing,
- Board IR copper serialization,
- import summaries and fail-closed parser errors,
- shared outline-contour geometry helpers.

## `src/importers/gerber/ownership.rs`

Owns conservative copper ownership association for imported Gerber copper.
It can annotate copper with `net` when existing Board IR layout evidence has
exactly one matching owner:

- pad overlap from `board.layout.pads`,
- route overlap from `board.layout.routes`,
- zone containment from `board.layout.zones`.

It intentionally does not infer new connectivity, `island_id`, component
ownership, or pad names from Gerber data alone. Ambiguous or missing ownership
evidence leaves imported copper anonymous.
