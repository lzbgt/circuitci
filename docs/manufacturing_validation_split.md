# Manufacturing Validation Split

`src/validation/manufacturing.rs` owns manufacturing-rule orchestration and
stable report construction for drill-edge, copper-edge, and copper-spacing
checks.

`src/validation/manufacturing/annular_ring.rs` owns
`DRILL_ANNULAR_RING_VALID`, including annular-ring parameter parsing,
pad/via/copper owner consistency, required copper-layer checks, and stable
annular-ring report construction.

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
