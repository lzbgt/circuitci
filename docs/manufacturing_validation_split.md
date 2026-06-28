# Manufacturing Validation Split

`src/validation/manufacturing.rs` owns manufacturing-rule orchestration,
shared manufacturing report helpers, stable copper-edge/copper-spacing report
construction, and the explicit net-pair same-layer conductor
creepage/clearance screen.

`src/validation/manufacturing/controlled_impedance.rs` owns
`CONTROLLED_IMPEDANCE_GEOMETRY_VALID`, including explicit impedance-target
parameter parsing, imported route width/gap evidence selection, fail-closed
missing route/gap evidence, and stable report construction. It intentionally
does not solve impedance.

`src/validation/manufacturing/controlled_impedance_stackup.rs` owns
`CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID`, including explicit
route/reference/dielectric layer parameter parsing, stackup material and
copper-thickness evidence checks, dielectric-between-layer topology validation,
and stable report construction. It intentionally does not solve impedance.

`src/validation/manufacturing/adjacent_plane.rs` owns
`ADJACENT_PLANE_RETURN_PATH_VALID` and
`REFERENCE_PLANE_SLOT_CROSSING_VALID`, including explicit stackup/plane
parameter parsing, adjacent reference-plane selection, sampled route-to-zone
coverage, route centerline coverage-interval extraction, fail-closed missing
stackup/zone evidence, and stable report construction. It intentionally does
not model electromagnetic return current.

`src/validation/manufacturing/stitching_via.rs` owns
`RETURN_PATH_STITCHING_VIA_VALID`, including explicit route-via layer-span
parameter parsing, reference-net via matching, fail-closed missing stackup/via
evidence, and stable report construction. It intentionally does not model via
inductance, stitching density, or electromagnetic return current.

`src/validation/manufacturing/annular_ring.rs` owns
`DRILL_ANNULAR_RING_VALID`, including annular-ring parameter parsing,
pad/via/copper owner consistency, required copper-layer checks, and stable
annular-ring report construction.

`src/validation/manufacturing/drill_slot.rs` owns circular drill diameter,
drill-to-board-edge, routed-slot edge/width/aspect-ratio, and
castellated-hole checks. It keeps drill/slot-specific finding construction with
those rules while reusing shared board-edge and drill-measurement serializers
from the parent module.

`src/validation/manufacturing/solder_mask.rs` owns
`SOLDER_MASK_OPENING_VALID`, `SOLDER_MASK_DAM_VALID`, and
`SOLDER_PASTE_OPENING_VALID`/`SOLDER_PASTE_APERTURE_SIZE_VALID`/
`SOLDER_PASTE_APERTURE_AREA_RATIO_VALID`/`SOLDER_PASTE_SPACING_VALID`,
including solder-mask opening/dam and
solder-paste opening/aperture/spacing parameter parsing, supported mask/paste
object validation,
opening-selection logic, and stable mask/paste report construction.

`src/validation/manufacturing/artwork_measurements.rs` owns repeated
solder-mask and solder-paste artwork measurement serialization for report
payloads. Rule modules keep the finding decisions and suggested fixes; this
helper module keeps shared feature/segment/region field names consistent without
growing the rule modules.

`src/validation/manufacturing/solder_paste_ic.rs` owns
`SOLDER_PASTE_IC_PIN_APERTURE_VALID`, including the JLCPCB pitch-conditioned IC
stencil table, optional target-component filtering, and stable IC aperture
report construction.

`src/validation/manufacturing/pin1_orientation.rs` owns
`PIN_1_ORIENTATION_VALID`, including imported body-center to pin-1 marker
direction measurement, explicit expected-direction parameter parsing, and
stable pin-1 orientation report construction.

`src/validation/manufacturing/geometry.rs` owns shared 2D geometry and evidence
selection for imported fabrication data:

- drill, copper flash, copper segment, and outline-segment input validation,
- drill-to-outline clearance selection,
- copper-to-outline clearance selection,
- annular-ring geometry,
- copper-to-copper spacing geometry.

`src/validation/manufacturing/process.rs` owns named fabrication process
preset lookup and shared manufacturing parameter parsing. Rule modules should
call its required/optional numeric helpers so explicit scenario parameters keep
overriding process defaults consistently.

Keep stable report keys and suggested fixes in the module that owns the rule.
Add pure distance, overlap, sampling, and nearest-evidence helpers to
`geometry.rs` so new fabrication checks do not push the rule modules toward the
repository line-count guard.
