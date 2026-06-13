# Manufacturing Validation Split

`src/validation/manufacturing.rs` owns manufacturing-rule orchestration and
stable report construction for drill-edge, copper-edge, and copper-spacing
checks.

`src/validation/manufacturing/annular_ring.rs` owns
`DRILL_ANNULAR_RING_VALID`, including annular-ring parameter parsing,
pad/via/copper owner consistency, required copper-layer checks, and stable
annular-ring report construction.

`src/validation/manufacturing/geometry.rs` owns shared 2D geometry and evidence
selection for imported fabrication data:

- drill, copper flash, copper segment, and outline-segment input validation,
- drill-to-outline clearance selection,
- copper-to-outline clearance selection,
- annular-ring geometry,
- copper-to-copper spacing geometry.

Keep stable report keys and suggested fixes in the module that owns the rule.
Add pure distance, overlap, sampling, and nearest-evidence helpers to
`geometry.rs` so new fabrication checks do not push the rule modules toward the
repository line-count guard.
