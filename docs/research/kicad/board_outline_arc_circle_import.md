# KiCad Board Outline Arc/Circle Import

CircuitCI imports KiCad board-outline graphics from `Edge.Cuts` into
`board.layout.outline.segments`.

Source reference:

- Local copy: `docs/research/kicad/sexpr-intro.html`
- Relevant sections: graphical `gr_circle` and `gr_arc`

Facts used from the KiCad S-expression reference:

- `gr_circle` carries `(center X Y)`, `(end X Y)`, and `(layer LAYER_DEFINITION)`.
  The `end` point defines the radius endpoint.
- `gr_arc` carries `(start X Y)`, `(mid X Y)`, `(end X Y)`, and
  `(layer LAYER_DEFINITION)`.
- The layer token gives the canonical KiCad layer. CircuitCI only treats these
  graphics as board-outline evidence when the layer is exactly `Edge.Cuts`.

Implementation decision:

- Board IR continues to expose outline evidence as straight `segments`.
- `gr_line` items are imported as one segment.
- `gr_circle` items are sampled into 32 bounded segments.
- `gr_arc` items are sampled using at most 11.25 degrees per segment, capped at
  64 segments.

This keeps USB orientation, edge-proximity, and body-overhang validators on one
simple outline geometry contract while allowing KiCad curved board-edge
evidence to affect nearest-edge calculations.
