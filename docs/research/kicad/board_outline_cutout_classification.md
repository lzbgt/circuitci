# KiCad Board Outline Cutout Classification

CircuitCI classifies imported KiCad `Edge.Cuts` outline segments into
`external`, `cutout`, or `unknown` boundary roles.

Local source:

- `docs/research/kicad/kicad_9_pcbnew.html`

Relevant KiCad PCB editor documentation:

- KiCad uses graphical objects on `Edge.Cuts` to define the board outline.
- A valid outline is made from closed shapes whose endpoints coincide exactly.
- Multiple closed shapes can exist on `Edge.Cuts`.
- When one closed outline completely encloses another, KiCad treats the
  outermost shape as the outside board edge and enclosed closed shapes as
  interior cutouts.

Implementation note:

- The KiCad PCB importer samples curved `gr_circle` and `gr_arc` outline
  graphics into bounded straight segments.
- It groups connected sampled/native segments into closed contours.
- A contour contained inside another larger closed contour is marked
  `boundary_role: cutout`; top-level closed contours are marked
  `boundary_role: external`; unclosed or unclassified evidence remains
  `boundary_role: unknown`.
- USB connector edge-selection logic ignores `cutout` segments but keeps
  `external` and `unknown` segments available for backward compatibility with
  older or hand-authored Board IR.
