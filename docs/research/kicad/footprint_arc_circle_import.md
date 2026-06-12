# KiCad Footprint Arc/Circle Import Notes

CircuitCI's KiCad PCB importer now carries footprint `fp_circle` and `fp_arc`
drawing evidence into Board IR for mechanical/layout review.

Source basis:

- Local pinned KiCad S-expression format reference:
  `docs/research/kicad/sexpr-intro.html`
- `fp_circle` format section: `(center X Y)`, `(end X Y)`, layer, stroke/fill,
  lock, UUID. The center defines the circle center and end defines the radius
  endpoint.
- `fp_arc` format section: `(start X Y)`, `(mid X Y)`, `(end X Y)`, layer,
  stroke, lock, UUID. The start/mid/end points define the displayed arc.

Implementation decision:

- Board IR stores circles as transformed `center` plus `end` points and arcs as
  transformed `start`, `mid`, and `end` points, preserving the defining KiCad
  geometry instead of flattening it at import time.
- Static USB connector edge and body-overhang checks sample circles/arcs into
  bounded 2D polylines only when measuring distance/overhang evidence.
- This remains drawing evidence, not 3D enclosure or connector-shell sign-off.
