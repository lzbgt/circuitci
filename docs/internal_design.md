# Internal Design

This document is for agents and maintainers changing CircuitCI internals. It
records the current seams between importers, Board IR, scenario suggestions,
validation rules, reports, and verification.

## Evidence Model

Board IR is the only data model consumed by validation. Importers may add or
enrich these evidence families:

- component graph: `board.components`, `board.nets`, and component
  `source` metadata;
- board-level manufacturing facts: `board.manufacturing`, currently including
  `stencil_thickness_mm`, `min_drill_edge_clearance_mm`, and
  `min_slot_edge_clearance_mm`, plus optional paste area-ratio and paste-spacing
  limits when those are supplied by board/order evidence;
- board-level layout policy: `board.layout.constraints`, currently including
  imported per-net route rules, explicit USB connector mechanical/layout policy
  under `usb_connector`, explicit USB data-route policy under `usb_route`, and
  explicit USB return-path policy under `usb_return_path`;
- placement/layout evidence: `placements`, `footprints`, `pads`, `routes`,
  `zones`, and `outline`;
- fabrication evidence: `drills`, `slots`, `copper`, `solder_mask`, and
  `solder_paste`.

Geometry is stored in millimeters. Gerber/Excellon source traceability stays on
the imported object through fields such as `source_primitive`,
`source_primitive_index`, `aperture`, `tool`, and hit/slot indices. Owner
metadata (`net`, `island_id`, `owner_kind`, `component`, `pin`, `via_index`) is
optional and must be assigned only when a unique source-backed match exists.

## Importer Rules

Importer code should preserve evidence and reject ambiguity:

1. Parse the smallest source subset that is actually supported.
2. Fail closed for unsupported units, coordinate modes, malformed geometry, or
   ambiguous multi-contour artwork.
3. Append to existing Board IR instead of replacing unrelated evidence.
4. Use existing layout evidence for owner association only when exactly one
   matching pad, route, zone, or via owner is proven.
5. Report counts in CLI summaries so an agent can see whether imported artwork
   remained anonymous or became owner-associated.

Do not infer nets from BOM/CPL placement data. Do not decode proprietary
payloads unless the encoding is documented or otherwise proven in the repo.

## Validation Dispatch

`src/validation/mod.rs` owns check ID dispatch. Each rule function receives a
`BoundBoard`, the selected `Scenario`, and a mutable finding vector. A rule
should return by pushing findings, not by panicking or mutating Board IR.

Input handling follows this order:

1. Explicit scenario parameters.
2. Source-backed `fabrication_process` preset defaults where the rule supports
   the requested parameter.
3. Board-level metadata only for facts that are truly board-wide, such as
   stencil thickness.
4. `VALIDATION_INPUT_MISSING` when the input remains unknown.

Explicit scenario parameters override defaults so users can run what-if checks.

Resistor-programmed charger current inference is centralized in
`src/charger_programming.rs`. It requires exactly one positive resistor between
the model-declared programming and reference pins and computes
`current_A = current_gain_V / resistor_ohm`; ambiguous evidence returns no value
so callers preserve fail-closed behavior.

## Manufacturing Geometry

Manufacturing validation uses shared geometry helpers for points, line
segments, copper flashes, circular-aperture draw capsules, and single-contour
regions. The rules intentionally stay static and two-dimensional:

- drill and slot checks use Excellon hit/slot evidence;
- annular-ring checks combine drills with owner-consistent copper flashes;
- copper edge/spacing checks combine Gerber copper objects and board outline
  segments;
- solder-mask checks compare copper flashes and mask openings, then mask
  opening-to-opening dams;
- solder-paste checks compare copper flashes, paste openings, source-backed
  stencil constraints, and package/pitch-scoped IC/BGA rows.

If a new geometry primitive cannot be represented precisely enough for the rule,
add fail-closed importer coverage before adding an approximation.

## Process Presets

Process presets live in `validation::manufacturing::process`. They are named
collections of numeric defaults, not hidden board profiles. Add a preset only
when the source text or saved source snapshot supports the exact condition.

Good preset examples:

- JLCPCB circular drill diameter range for `DRILL_DIAMETER_VALID`.
- JLCPCB routed-edge copper clearance for
  `COPPER_TO_BOARD_EDGE_CLEARANCE_VALID`.
- JLCPCB castellated-hole limits for `CASTELLATED_HOLE_VALID`.

Bad preset examples:

- Reusing castellated-hole edge limits for every drill.
- Turning package-specific stencil table values into global paste-spacing
  defaults.
- Inferring stencil thickness from Gerber paste apertures.

## Scenario Suggestions

Scenario suggestions are generated from available evidence, not from desired
coverage. A runnable suggestion must include all required rule parameters,
either directly or through process presets and board metadata. A non-runnable
suggestion must list the missing inputs in `required_inputs`.

When adding a new manufacturing rule, update suggestions only when the Board IR
evidence can identify the applicable source condition. For package-scoped
stencil checks, require owner-backed repeated pitch/grid evidence before
suggesting the rule.

## Reports

Reports are a stable API. New findings should include:

- rule ID and severity,
- scenario name/check context,
- measured geometry/process values,
- limit values,
- source indices and owner metadata when available,
- an actionable fix string.

Do not rename existing report keys without a compatibility reason and matching
fixture updates.

## Tests And Guardrails

For source changes, run the narrow tests first, then broad verification:

- focused importer/rule tests for the changed behavior,
- schema sweeps for changed Board IR/report shapes,
- `cargo fmt --check`,
- `cargo test`,
- `cargo clippy --all-targets --all-features -- -D warnings`,
- release CLI build and public/acceptance suites when behavior affects users,
- `git diff --check`,
- line-count guard.

When a source file approaches the 2000-line guard, split by rule family or
projection concern before adding more logic.
