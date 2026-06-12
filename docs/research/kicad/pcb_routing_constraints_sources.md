# KiCad PCB Routing Constraint Sources

Research date: 2026-06-12

Primary source:

- KiCad 9.0 PCB Editor documentation, saved locally as
  `docs/research/kicad/kicad_9_pcbnew.html`.
- Source URL: https://docs.kicad.org/9.0/en/pcbnew/pcbnew.html
- SHA-256:
  `d78334b29df713eb319beccbdd443dd5fc2e2872ff70f6922b51f44372974950`

Facts used by CircuitCI:

- KiCad net classes define router defaults for copper clearance, track width,
  via sizes, and differential-pair sizes.
- KiCad net-class track widths and via sizes are not hard DRC failures by
  themselves; custom rules are needed to restrict them.
- KiCad custom DRC rules can express `length` and `skew` constraints, including
  examples that apply by net class with `A.hasNetclass('high_speed')`.
- `skew` is the documented constraint for maximum skew between matched tracks,
  and can be scoped to differential pairs.

CircuitCI importer scope:

- Import net-class defaults as evidence under
  `board.layout.constraints.net_rules`.
- Import simple custom `length`/`skew` max constraints when the rule condition
  names a net class or explicit net.
- Do not claim full KiCad DRC compatibility; unsupported rule expressions remain
  outside the imported evidence.
