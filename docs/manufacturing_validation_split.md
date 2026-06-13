# Manufacturing Validation Split

`src/validation/manufacturing.rs` owns manufacturing-rule orchestration and
stable report construction for drill, annular-ring, copper-edge, and
copper-spacing checks.

`src/validation/manufacturing/geometry.rs` owns shared 2D geometry and evidence
selection for imported fabrication data:

- drill, copper flash, copper segment, and outline-segment input validation,
- drill-to-outline clearance selection,
- copper-to-outline clearance selection,
- annular-ring geometry,
- copper-to-copper spacing geometry.

Keep stable report keys and suggested fixes in the parent module. Add pure
distance, overlap, sampling, and nearest-evidence helpers to `geometry.rs` so
new fabrication checks do not push the parent module toward the repository
line-count guard.
