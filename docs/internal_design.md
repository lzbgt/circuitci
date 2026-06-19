# Internal Design

This document is for agents and maintainers changing CircuitCI internals. It
records the current seams between importers, Board IR, scenario suggestions,
validation rules, reports, and verification.

## Frontend Boundary

The default runtime remains the CLI/library engine. The optional
`circuitci-gui` binary is enabled only by the `gui` feature and must call the
same Board IR, library binding, scenario suggestion, validation, and report
APIs used by the CLI. It must not grow a second project model, a second
validation dispatcher, or an in-house analog solver.

The GUI stages are allowed to organize user workflow around project loading,
KiCad/SPICE import, sketch/model review, library suggestions, simulation
artifact observation, and report viewing. Schematic-canvas editing and waveform
plotting should still persist through Board IR, importer metadata, generated or
file-backed SPICE decks, and report artifacts so headless agents can reproduce
GUI actions.

The Sketch stage should follow a schematic-first editor convention similar to
modeling tools such as Simulink: the schematic canvas is the mandatory primary
scene, Run and scope/observation controls remain directly reachable from that
scene, and project/library/import/YAML details must stay secondary or docked so
they do not shrink the model view into a preview pane.

The Simulation stage should follow the same convention from the runtime side:
it is a Scopes workspace first, with the oscilloscope plot, waveform/probe
selection, and play/scrub controls treated as the primary view. Analog scenario,
model-file, assertion, deck, artifact, and finding editors may be available in
secondary docks, but they must not force the runtime view back into a
form-heavy page.

GUI implementation is split so the stage shell does not accumulate all desktop
logic in one source file. `src/gui.rs` owns application state, the `eframe`
update loop, and shared validation/report command helpers. `src/gui/shell.rs`
owns menus, workflow stage routing, the left project panel, the status panel,
Project/Reports views, and finding/limitation rendering. `src/gui/jobs.rs` owns background GUI job
state, worker-thread launch, channel polling, stale-result rejection, and
cancel-request handling for validation, scenario suggestion, and KiCad/SPICE
import actions. Canceling a background job must not try to kill a Rust thread.
It sets a shared worker cancellation flag, terminates interruptible external
ngspice child processes during validation, stops scenario suggestion and
KiCad/SPICE import workflows at safe phase-boundary checkpoints, and marks any
late result as ignored when the worker returns. Embedded backend calls are
still non-preemptive unless their internals expose safe cancellation points.
Active job events and recent job records are capped, in-memory workflow
diagnostics with stage, label, outcome, elapsed time, detail, and optional
output path; they must not be serialized into Board IR or treated as design
evidence. Safe checkpoint cancellation paths should return the shared typed
operation-canceled error so the GUI records a `canceled` outcome instead of a
generic `failed` worker error. Validation progress should be emitted from the same path
that loads the project, binds models, executes scenarios, prepares and runs
analog transients, loads waveform artifacts, applies profile coverage,
assembles reports, and writes artifacts so the GUI does not drift from headless
validation semantics. KiCad/SPICE import progress should be emitted inside the
importer entry points that parse source files, load mappings or Board IR
inputs, build or merge Board IR evidence, and write output YAML; GUI jobs
should only route those events to the status panel.
`src/gui/import_flow.rs` owns the Import stage UI plus KiCad
schematic, KiCad PCB, and SPICE deck import command wiring.
`src/gui/project.rs`
owns project summary/YAML load, save, parse validation, import path/name
helpers, and the shared Board IR edit history. `src/gui/sketch.rs`
owns Board IR graph snapshots, graph layout helpers, bounded full-list logical
layout for pannable imported designs, schematic grid/snap helpers, orthogonal
wire geometry, schematic wire route waypoint metadata, wire hit-testing,
shared sketch YAML helpers, and model-port default pin/net seeding for
library-backed component insertion. `src/gui/sketch_routes.rs` owns shared
orthogonal wire-route geometry helpers so display, hit-testing, insertion,
active wire preview, and drag preview all use the same route semantics.
`src/gui/sketch_wire_draft.rs` owns transient in-progress wire-bend points while
the user is drawing a pin connection.
`src/gui/sketch_duplicate.rs` owns selected-component duplication YAML
mutation. `src/gui/sketch_canvas.rs` owns the
Sketch-stage canvas shell: drawing order, event routing, pin-anchor drag-to-wire
completion, overview-minimap event routing, and runtime tint routing.
`src/gui/sketch_canvas_tools.rs` owns helper actions for active multi-bend wire
drawing, direct wire-route edits, component placement orientation controls,
selected-component orientation transforms, canvas probe defaults, and viewport
pan/zoom input.
`src/gui/sketch_alignment.rs` owns transient alignment-guide derivation,
drawing, and optional guide-snap target adjustment for component placement plus
selected-node and selected-group drag affordances. The primary schematic toolbar
owns the transient Grid, grid-step, and Free/Grid/Guides/Grid+Guides snap-mode
controls. Alignment guides do not create a second Board IR model; persistence
still routes through validated schematic position edits.
`src/gui/sketch_canvas_interaction.rs` owns reusable interaction primitives:
viewport zoom math, schematic canvas sizing, wire target hit-testing,
route-handle hit-testing, and placement orientation cycling. `src/gui/sketch_render.rs` owns graph node and pin-anchor
painting, including runtime tinting, opacity handling, symbol glyph dispatch,
and kind-aware pin chips. `src/gui/sketch_canvas_render.rs` owns the
canvas-local paint and tooltip helpers for wires, route handles, wire previews,
wire target affordances, snap/free target feedback, and placement ghosts. `src/gui/sketch_canvas_menus.rs` owns
right-click menus for component, net, wire, probe badge, route-handle, and
blank-canvas targets, including route handle insertion/deletion and
custom-route clearing. Blank-canvas primary drag
and touchpad scroll should pan the schematic viewport, pointer-focused
pinch/Cmd-scroll should zoom around the cursor, Shift-drag should replace the
selection with boxed visible items, Cmd/Ctrl-drag should add boxed items,
Alt/Option-drag should subtract boxed items, and holding `L` while starting
those same selection drag chords should use a freehand lasso instead of a box.
Pin-anchor drag should start visual wire mode instead of moving the component,
and component drags must only move objects when the drag starts on a component.
Active wire drags should use the same pin/net/wire target hit test
for preview and release so the highlighted target matches the eventual Board IR
mutation. Inserted library
components may create generated per-pin nets from source-backed model port
declarations, but those nets are still ordinary Board IR sketch connections
that the user can rewire. `src/gui/sketch_symbols.rs`
owns visual-only symbol-style rendering: it may infer common glyph classes from
reference designators and model IDs, but it must continue to persist only Board
IR components, nets, pins, and optional schematic node positions/styles.
`src/gui/sketch_actions.rs`
owns sketch canvas selection, fit-content, multi-selected movement/alignment/distribution,
selected-item deletion, transient selected-component clipboard state, and
selected-component duplicate/copy/paste actions that compose lower-level sketch
YAML mutations. Clipboard state may remember component IDs only; paste must
still produce an ordinary validated Board IR edit. `src/gui/sketch_duplicate.rs` may copy
selected components and nets whose references are wholly inside the selected
component set, but it must
leave externally referenced nets shared, strip imported component source
provenance from the duplicates, avoid copying PCB/layout evidence, and re-parse
Board IR before committing the edit. Paste may reposition the duplicated group
to a canvas target, but that placement must be written through
`board.schematic.node_positions`, not physical layout evidence.
Primitive and library placement may use click-to-arm, drag/drop, or context
menu insertion, but the live placement ghost and snap/free target marker are GUI-only feedback and the
drop result must still pass through the same validated Board IR mutation path.
`src/gui/sketch_navigator.rs` owns derived searchable component,
net-bundle, net, wire, and probe rows for the Sketch-stage object navigator.
Navigator selection and fit actions may update GUI selection, analog probe
editor context, and viewport state, but they must not mutate Board IR by
themselves. `src/gui/sketch_bundles.rs` owns conservative derived grouping for
bracketed, dot-qualified, and common paired interface nets, plus visual
bundle trunks/badges and bundle multi-selection. Bundle overlays must remain
GUI-only navigation aids over scalar Board IR nets, not persisted bus
topology/evidence. `src/gui/sketch_net_labels.rs` owns persisted schematic
named-net and off-page connector labels under `board.schematic.net_labels`.
Those labels may be placed from selected-net controls, typed net-label controls,
or net/wire context menus. A typed label may reuse an existing net or create a
missing Board IR net with an explicit net kind before placing the display
badge. Labels may be dragged to reposition their display point, double-clicked
or context-edited with existing-net autocomplete, clicked to select the
underlying net, used as active-wire drop targets for that same underlying net,
cycled to the next peer label on the same net, converted between local and
off-page presentation, removed when their net is removed, and rewritten when a
net is renamed. Inline editing to an existing net retargets only that label;
inline editing to a missing net renames the underlying Board IR net through
`sketch_rename`. Selecting a net may transiently highlight matching wires, peer
labels, and connected pin anchors, but this derived trace is not persisted. They
annotate ordinary Board IR nets and must not create hidden connectivity, bus
topology, sheet ports, or PCB
evidence. `src/gui/sketch_hierarchy.rs` owns derived schematic
hierarchy grouping for the Sketch stage. It may use imported KiCad
`component.source.instances[*].path` records and importer-generated namespaced
component IDs such as `sheet__R1` to select, multi-select, or fit related
components and nets, but it must remain a GUI-only navigation aid over the
flattened Board IR graph. Focus and isolate modes may dim or hide unrelated
canvas objects, but they are transient view filters and must not remove,
rewrite, or re-scope Board IR components, nets, probes, buses, wires, or
connectivity. Off-sheet connector badges may be shown for focused nets when
the same flattened net has pins on components outside the focused group; those
badges must be derived from current Board IR component pin bindings and must
not persist sheet ports, hierarchical labels, or alternate net ties. It must
not persist a hierarchy tree, sheet primitive, or alternate connectivity model.
`src/gui/sketch_inspector.rs` owns the selected component/net
inspector, structured scalar YAML edit actions, conservative component and
unreferenced-net add/remove operations, structured component/net rename controls,
schematic symbol style edits, validated component pin assignment, visual wire
assignment mutations, selected-net local/off-page label placement controls, and selected-net voltage-probe insertion controls plus selected-component current-probe
insertion for generated source branches plus generated passive and
diode/BJT/MOSFET current-sense branches, and selected-component power-probe
insertion for those same supported generated branches.
`src/gui/sketch_selection_inspector.rs` owns the multi-selection summary,
on-canvas selected-group frame/move handle and quick toolbar, and quick actions;
it must reuse existing viewport, group-action, orientation, clipboard,
duplication, and delete paths instead of creating another mutation model.
`src/gui/sketch_probes.rs`
owns derived schematic voltage/current/power probe badge targeting, layout,
hit-testing, and drawing. Rendered pin anchors are UI affordances derived from
component pin bindings and connected net kind. They may show color and
pin/kind chips for hover, selection, connectivity, and active wire targeting,
but must not persist a second port or connectivity model. Clicking or dragging
an anchor may start or complete a wire assignment. Pin-to-pin wiring should
reuse a source or target pin net when
one already exists, or create a generated Board IR net when both pins are
unbound. Wire-drag previews may highlight and snap to a pin, net node, net
label badge, or rendered wire, but the persisted result must remain
`board.nets` plus component `pins`, not a second edge list. Canvas pan/zoom, Home, Fit All, Fit
Selection, and marquee selection are GUI view/selection state only; they must
not be serialized as board evidence. Fit commands should operate on current
schematic graph bounds so imported designs can be recovered after pan/zoom and
selected components/nets can become the viewport focus.
Single-node and multi-node drag/alignment persistence must
invert the viewport transform before writing `board.schematic.node_positions`.
If snap is enabled, the snapped logical schematic coordinates are written to
`board.schematic.node_positions`; grid visibility and grid spacing remain GUI
editor state. The schematic minimap/overview is also GUI-only viewport state:
it renders current graph bounds and maps pointer clicks or drags to `sketch_pan`
without persisting model evidence. Orthogonal wire routing is display-only and
must render from component pin bindings and net membership rather than
persisting a parallel edge list. A custom schematic route may persist
display-only waypoints under
`board.schematic.wire_routes` keyed by `component.pin->net`, but that metadata
may only shape the rendered pin-to-net edge. It must not create, remove, or
retarget electrical connectivity, and it must not be treated as PCB copper
route evidence. Wire hit-testing may select the underlying Board IR net,
connect an active source pin to that net, add transient blank-canvas bend points
while drawing an active source pin connection, drag a visible display waypoint
for the specific rendered edge, drag a named-net label display point, finish an
active source pin connection on a named-net label's underlying net, or clear
those display waypoints. Completing an active multi-bend wire may persist the pending bend points as
`board.schematic.wire_routes` in the same validated edit as the pin-to-net
connection. None of those actions may persist a separate wire object.
Rotate/flip/pin-side editor actions and canvas `R` / `Shift+R` rotation,
`F` flip, and `Shift+F` pin-side shortcuts for selected components or armed
component placement must write `board.schematic.node_styles` and remain
schematic-only UI state.
Rename actions must rewrite explicit Board IR IDs rather than introducing a
display alias. Component rename must move `board.components` keys, schematic
component metadata keys, generated analog component lists, analog pin-binding
endpoints, and supported generated/source branch probe expressions such as
`I(VCCI_R1)` before reparsing the edited YAML. Net rename must move
`board.nets` keys, component pin net values, schematic net metadata keys,
generated analog ground/net bindings, and then revalidate. It must also rewrite
`board.schematic.net_labels` entries that point at the old net ID. It may
assign or remove component pin bindings only when the component exists and any
assigned target net exists. It may persist schematic graph node
positions under
`board.schematic.node_positions` and symbol orientation under
`board.schematic.node_styles`; it must not use
`board.layout.placements` for schematic drag state because those coordinates
are physical PCB evidence consumed by placement/layout validators. Visual wire
routing should keep using these Board IR mutation helpers rather than
introducing a parallel connection model. The rendered wire path may be
orthogonal and may include net labels and junction dots for readability, but
the persisted connection is still only the target component pin's net binding.
`src/gui/sketch_spice.rs` owns selected-component SPICE primitive/value edits
from the Sketch inspector. It must validate that passive primitives have `A`/`B`
pins and independent-source primitives have `P`/`N` pins before writing
component-level `spice` evidence, reject non-finite or nonsensical numeric
values, update exact generated/source branch probe expressions when the
primitive prefix changes, and reparse Board IR before accepting the edit.
`src/gui/sketch_inline_edit.rs` owns direct canvas inline component ID and
single-value SPICE edits. It must remain a thin UI layer over
`sketch_rename.rs` and `sketch_spice.rs`; it may parse convenience engineering
suffixes for scalar values, but it must not bypass pin-convention validation,
branch-expression rewrite rules, or Board IR reparse validation.
`src/gui/sketch_component_labels.rs` owns visible component reference/value
labels. Label text must always be derived from Board IR component IDs and
scalar component-level SPICE evidence; persisted
`board.schematic.component_labels` entries may store only optional display
positions for those derived labels. Component rename mutations must rewrite
component-label metadata keys so stale display positions do not remain.
Reference/value visibility toggles are transient GUI state. Auto-arrange may
write component-label positions, but it must not change component IDs, SPICE
values, pins, nets, assertions, or analog evidence.
`src/gui/sketch_palette.rs` owns primitive insertion for user-sketched generic
R/C/L and independent voltage/current source components. Each insertion must
create the component, editable pin nets, component-level SPICE evidence, and
schematic placement together through the same validated Board IR YAML mutation
path whether it is inserted at the current view center, by an armed canvas
click, or from the blank-canvas context menu; the palette must not create a
separate temporary schematic model.
The GUI shared undo/redo history in
`src/gui/project.rs` is a capped in-memory stack of Board IR YAML snapshots;
graph, property, wire, and text edits should enter that history through the
same application-level mutation boundary rather than keeping per-widget state.
Loading or importing a different project clears history, while saving the
current project preserves it so the user can still step backward from a saved
edit. Load, import, and quit actions must pass through the `src/gui/project.rs`
pending-action dirty-state guard when Board IR YAML or a loaded file-backed
SPICE deck has unsaved edits. Confirmed discard may clear editor state and
history before executing the action; canceled actions must leave the current
workspace untouched. Native path pickers in `src/gui/file_dialogs.rs` only
populate existing project/import/output path fields or request the same guarded
project load action; they must not bypass validation, import, save, or the
dirty-state guard. `src/gui/simulation.rs` owns the Simulation/Scopes stage UI,
keeping the runtime oscilloscope primary while analog scenario/model/assertion
panels stay docked as secondary controls. `src/gui/waveform.rs` owns streaming,
cancel-aware waveform CSV parsing, Scopes state orchestration, simulation-time scrub/playback
controls, cursor readouts, selected-plus-pinned cursor readout rows, cursor/visible-window region statistics with snapshot capture, actionable transient cursor-region, region-stat, and trigger-event measurement snapshots with editable labels/notes, search/source filters, sort/group controls, plot markers, and filtered CSV/Markdown copy/export,
min/max/delta measurements, transient selected-probe trace pinning/comparison
overlays, transient per-trace overlay visibility/color styles, GUI-only derived
waveform channels, promotion of representable derived channels to explicit
Board IR analog probes/assertions, exact probe-value lookup, and normalized
runtime activity values for graph tinting.
`src/gui/waveform/waveform_trace_selector.rs` owns waveform and
searchable/grouped trace selection, transient saved compare sets, transient
trace-style controls, split-unit lane toggling, and selected-trace reset
behavior.
`src/gui/waveform/waveform_context.rs` owns
pending schematic probe-to-scope focus, runtime trace/event-to-schematic cross-focus selection, selected-trace
schematic-context strip actions, and scope probe lookup.
`src/gui/waveform/waveform_plot.rs` owns the primary scope plot drawing,
draggable/click-set A/B cursor handles, direct plot drag/wheel/Shift-wheel
interactions, trace overlay selection, Alt/Option-drag box zoom, and
transient measurement snapshot marker chips with hover and click actions, and shared-axis or per-unit lane
axis scaling, including a transient min/max decimated trace-point cache for
large CSV plot rendering. `src/gui/waveform/waveform_export.rs` owns deterministic runtime
SVG rendering for the current Scopes plot, including visible traces,
split-unit lanes, cursors, trigger markers, snapshot chips, bounded decimated
trace polylines, report-size presets, and annotation inclusion toggles.
`src/gui/waveform/waveform_view.rs`
owns Scopes plot orchestration, cursor readout rows, region statistics display and capture, playback controls,
visible time-window and value-window fit/zoom/pan helpers, Back/Forward
view-window history, scope plot SVG copy/export actions, and measurement
snapshot display.
`src/gui/waveform/waveform_snapshots.rs` owns transient cursor-region,
region-stat, and trigger-event measurement snapshot capture, editable labels
and notes, search/source filtering, sort/group projection, plot-marker
derivation, filtered CSV/Markdown serialization/export, timestamped report
bundle export with the configured plot SVG and README manifest, transient
recent-bundle folder opening and previewed/confirmed bounded old-bundle cleanup, Jump
restore, schematic Focus, and rendering over loaded waveform artifacts. `src/gui/waveform/waveform_trigger.rs` owns transient selected-trace trigger edge/threshold controls, CSV-derived crossing interpolation, exact event readout rows, and previous/next or row-level trigger jumps. It may display graph hover
readouts, activity coloring, pinned trace overlays over the currently loaded
CSV set, and derived difference, sum, product, or ratio channels for runtime
waveform probes, but those values must come from report waveform artifacts and
the shared waveform interpolation helpers rather than an unsynchronized live
simulation model. Validation workers load waveform artifacts with bounded preflight size/row estimates, large-artifact progress warnings, progress and
cancel checks, loaded/skipped file diagnostics, and report/waveform co-application,
so large CSV parsing does not run on the UI thread, skipped traces and slow artifacts are visible
to the user, diagnostics can be copied as CSV, and stale waveform data does not outlive its report.
`src/gui/waveform/waveform_load.rs` owns bounded CSV preflight estimates plus filterable/copyable transient waveform-load diagnostics for loaded/skipped CSV artifacts.
Focused waveform and Scopes regressions are split into
`src/gui/waveform/waveform_tests.rs` for parser/plot/trigger helpers and
`src/gui/waveform/waveform_measurement_tests.rs` for cursor, region-stat, and
snapshot measurement behavior, and `src/gui/waveform/waveform_scope_tests.rs`
for app-level Scopes context, style, lane, probe, and compare-set behavior;
production waveform code should stay in `src/gui/waveform.rs` /
`src/gui/waveform/waveform_plot.rs` and avoid depending on test-only helpers.
Schematic probe badges are derived in `src/gui/sketch_probes.rs` from existing
analog scenario probes: voltage expressions attach to Board IR nets through
`analog.node_bindings`, while current and power expressions attach to
components only when their `I(...)` branch maps to a generated/source branch
name CircuitCI can prove. Badge assertion-status markers are derived from the
latest loaded `ValidationReport`, not from live simulation state. A badge is
unasserted when no Board IR assertion references its probe, unknown when no
report is loaded or the scenario had a non-assertion failure, failed when a
report finding names one of the probe's assertions, and passed only when the
latest report has no matching assertion failure. Badge clicks, object-navigator
probe rows, and primary-toolbar scope actions select the existing scenario/probe
in the Simulation stage and may focus a matching waveform trace when loaded; the
selected-probe assertion table must still be derived from Board IR assertions
plus the latest `ValidationReport` and must not cache a parallel assertion
model. Pressing `A` on a hovered badge
may append a normal Board IR assertion using the current assertion-editor
settings. Pressing Shift+A or Shift+B on a hovered badge may append a normal
sample assertion whose threshold is derived from an exact loaded waveform probe
match at the current cursor with a small pass-at-current-sample margin; if no
matching waveform column is loaded, the quick action must fail closed without
editing Board IR. The probe badge right-click menu may expose these same
actions, but it must call the same validated mutation paths as the keyboard
shortcuts rather than creating separate menu-only behavior. Pressing `X` may
remove assertions for that probe while keeping the probe, and Delete/Backspace
may remove the underlying Board IR probe through `src/gui/analog.rs`; probe
removal must also remove assertions that reference that probe before re-parsing
Board IR. Badges must not become a second persisted probe store. Component,
net, and wire context menus in `src/gui/sketch_canvas.rs` are interaction routing only. They
may select/inspect the target, start or complete visual wire mode, add
supported voltage/current/power probes, or delete through the existing
validated Board IR mutation helpers. Rendered wire menus operate on the
underlying Board IR net; the drawn wire must not become a separately persisted
edge object.
`src/gui/library.rs` owns active-library model browsing, model filtering,
component model assignment, and model-backed component insertion/placement
through the same validated Board IR YAML mutation helpers used by the sketch
inspector. Canvas placement must write generated default pin nets and
`board.schematic.node_positions` plus non-default schematic orientation in
`board.schematic.node_styles` in one accepted YAML edit, whether the target is
the current view center, an armed blank-canvas click, a drag/drop release with
orientation-aware live ghost and snap/pin-side feedback, or the blank-canvas
context menu pointer.
`src/gui/analog_models.rs` owns analog `model_files` listing and mutation. GUI
additions must hash the selected file and write an explicit SHA-256 alongside
the path, while removal must only delete the selected model-file entry from the
target analog scenario.
`src/gui/analog_overview.rs` owns read-only generated analog scenario audit
snapshots for Simulation-stage display. It may summarize timing/backend,
included components, source primitives, probes, assertions, model files, and
node bindings from Board IR, and it may derive readiness diagnostics for missing
source primitive, probe, assertion, model SHA, node binding, and pin binding
coverage. Readiness actions may preselect existing Simulation-stage editor
fields for those gaps, but they must not mutate Board IR by themselves. It must
not introduce a second analog netlist or sign-off model.
`src/gui/analog.rs` owns generated-from-Board analog transient scenario creation,
selected-net voltage-probe insertion, selected-component source, passive, and
semiconductor current-probe insertion, selected-component source, passive, and
semiconductor power-probe insertion into existing analog scenarios, and
structured sample/min/max assertion authoring. Its focused regression tests live
in `src/gui/analog_tests.rs` so the production module stays below the source
line guard. It may derive node and pin bindings from Board IR for observation
scenarios. Selected-net probe insertion must fail
closed when the target scenario has no node binding for the selected Board IR
net. Selected-component current-probe insertion must fail closed unless the
target scenario is `generated_from_board`, includes the component, and the
component branch is a Board IR voltage/current source primitive, a Board IR
resistor/capacitor/inductor primitive that can receive a generated current-
sense source, or a bound diode/BJT/MOSFET model branch with CircuitCI's
generated current-sense source. Selected-component power-probe insertion must
use the same component set and compose explicit branch voltage and current
expressions rather than relying on hidden waveform math.
Generated scenario settings and component membership editing lives in
`src/gui/analog_generated.rs`. It may mutate `analog.analysis` timing,
`analog.generated.ground_net`, `analog.node_bindings`,
`analog.generated.components`, and the scenario pin bindings needed for included
component pins. Ground edits must keep the selected ground net bound to SPICE
node `0`, node edits must keep node names unique per scenario, component edits
must keep at least one generated component, and all edits must reject unknown
board nets or components.
The branch expression derivation itself lives in `src/gui/analog_branches.rs`
and must fail closed for unsupported subcircuits, file-backed deck internals, or
components that lack the required model/pin evidence.
Structured source-stimulus editing lives in `src/gui/analog_stimulus.rs` and
mutates only existing generated-scenario source primitives on Board IR
components (`dc_v`, `dc_a`, `pulse`, or `current_pulse`). It must not create a
second scenario-local stimulus store, and it must reject stale UI state when the
scenario, component, or primitive kind no longer matches the current Board IR.
It must append or remove normal Board IR analog probes and assertions rather
than creating a GUI-only probe list; selected-probe assertion summaries must be
recomputed from Board IR plus the latest report, assertion clearing for a probe
must leave the probe itself intact, row-level assertion edits/deletes must
replace or remove exactly one named Board IR assertion after revalidation, and
probe removal must drop dependent assertions so analog scenarios do not retain
dangling assertion references.
`src/gui/spice.rs` owns
file-backed SPICE deck discovery, loading, saving, and save-and-run actions for
analog scenarios. It must resolve relative deck paths from the project YAML
directory and keep the Board IR analog scenario as the source of truth, rather
than introducing a second analog project model. GUI-derived waveform math channels must remain observation-only until explicitly
promoted. Promotion may only create Board IR analog probes for representable
voltage/current/power expressions and may optionally add an assertion through the
same structured assertion validation path; dimensionless ratios must remain
GUI-only until the schema has a quantity for them.

## Evidence Model

Board IR is the only data model consumed by validation. Importers may add or
enrich these evidence families:

- component graph: `board.components`, `board.nets`, and component
  `source` metadata;
- schematic GUI evidence: `board.schematic.node_positions`, which stores
  component/net graph positions for editor usability, and
  `board.schematic.node_styles`, which stores schematic-only symbol rotation,
  mirror, and pin-side preferences, and `board.schematic.wire_routes`, which
  stores display-only pin-to-net route waypoints, and
  `board.schematic.component_labels`, which stores display-only reference/value
  label positions for existing Board IR components, and
  `board.schematic.net_labels`, which stores display-only local/off-page labels
  for existing Board IR nets. These are intentionally separate from physical
  `board.layout.placements` and
  `board.layout.routes`;
- board-level manufacturing facts: `board.manufacturing`, currently including
  `stencil_thickness_mm`, `min_drill_edge_clearance_mm`, and
  `min_slot_edge_clearance_mm`, plus optional paste area-ratio and paste-spacing
  limits when those are supplied by board/order evidence;
- board-level layout policy: `board.layout.constraints`, currently including
  imported per-net route rules, explicit USB connector mechanical/layout policy
  under `usb_connector`, explicit USB data-route policy under `usb_route`,
  explicit VBUS power-entry route policy under `usb_vbus_route`, and explicit
  USB return-path policy under `usb_return_path`;
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

Profile coverage is intentionally handled as a report-stage limitation, not as
hidden validation dispatch. `profile_coverage_limitations()` compares
project-declared checks with the core `iot_basic_v0` executable coverage set
and emits one non-blocking `PROFILE_COVERAGE_PARTIAL` limitation when coverage
is incomplete. Do not make profiles auto-add scenarios; use
`suggest-scenarios` or importer metadata to produce explicit scenario inputs.

Resistor-programmed charger current inference is centralized in
`src/charger_programming.rs`. It requires exactly one positive resistor between
the model-declared programming and reference pins and computes
`current_A = current_gain_V / resistor_ohm`; ambiguous evidence returns no value
so callers preserve fail-closed behavior.

Power-mux selected-input inference is centralized in
`src/power_mux_selection.rs`. It is intentionally narrower than control-pin
truth-table simulation: it only derives a selected input when the mux output is
declared powered and every declared input rail has explicit powered/unpowered
metadata with exactly one powered input. Ambiguous or incomplete source-state
evidence returns no value so validation still requires the explicit
`selected_input_parameter`.

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

## Motor Drive Budgets

`src/validation/motor_drive.rs` owns deterministic first-pass motor-drive
validator entry points for bridge budget, SOA, regen clamp, route-current,
current-sense placement, and current-sense accuracy.
`src/validation/motor_drive_bridge.rs` owns focused bridge electrical screens:
motor supply voltage, bridge loss/thermal, and bridge switching.
`src/validation/motor_drive_common.rs` owns shared parameter parsing,
motor-load evidence resolution, route/placement geometry helpers, and common
finding builders. `MOTOR_LOAD_SUPPLY_VALID` and the bridge-budget rule consume
explicit scenario parameters and optional `parameters.motor_component` evidence
from a bound component model with `motor_load`; they do not infer motor
behavior from topology.
`MOTOR_REGEN_CLAMP_VALID` similarly accepts explicit scenario current/energy
ratings for what-if studies or `parameters.clamp_component` evidence from a
bound component model with `regen_absorber`. Required current, connector,
shunt, gate-resistor, dead-time, PWM, and absorber values must be finite and
positive where appropriate; unknown values produce `VALIDATION_INPUT_MISSING`.

Current-sense placement checks consume only component placements and route
polylines already present in `board.layout`. They compare phase shunts against
declared reference-component, phase-route, sense-route, and sense-route-length
limits. Current-sense accuracy checks consume explicit shunt, gain, ADC,
offset, and tolerance budgets, then compute peak ADC range,
minimum-current ADC counts, and conservative worst-case current error. They
deliberately avoid calculating shunt parasitics, PWM rejection, thermal drift,
or dynamic current-loop stability.

Bridge loss/thermal checks consume typed `motor_bridge` model metadata and
explicit board thermal-budget parameters. The only built-in loss model is
source-reference scaling by RMS phase current. Bridge switching checks consume
typed `motor_bridge` gate-charge/rise/fall metadata and explicit gate-drive/PWM
scenario budgets to estimate transition loss and average gate-drive charge
current. Regeneration clamp checks consume
an explicit single-event energy envelope, bus capacitance, voltage window, and
named absorber component; they do not derive rotor energy from topology or
motor guesses. Keep SOA curves, peak gate-current timing, switch-node ringing,
transient thermal impedance,
PWM sampling/common-mode behavior, repeated-pulse clamp heating, and measured
temperature as separate evidence-backed rules.

Keep this module limited to checks that can be evaluated from declared design
budget numbers, component binding evidence, and explicit layout geometry.
MOSFET SOA, switching loss, thermal paths, current-sense waveform behavior,
selected regeneration absorber behavior, and PCB copper temperature rise need
source-backed model data or separate validation rules before they are treated
as sign-off.

## Load Connector And Cable Budgets

`src/validation/load_budget.rs` owns deterministic first-pass load connector
and cable budget checks. The connector rule consumes one target load power pin,
its model-declared `max_supply_current_A`, and either explicit connector rating
parameters or a bound connector component model with `connector` metadata. The
cable current rule uses the same target load evidence, but requires explicit
cable ratings or a bound cable assembly model with `cable_assembly` metadata.
The cable thermal rule requires a declared temperature-rise test point and
maximum allowed rise, then scales rise by I^2 from the test current.
The cable voltage-drop rule requires declared loop resistance and an allowed
drop limit, then computes DC voltage drop and optional power loss at the
declared margin current.

`src/validation/load_budget_power_switch.rs` owns selected power-switch gates
for the same `load_budget` scenario type. The static budget rule consumes one
target load power pin and a bound `power_switch` component, then checks
switched-rail connectivity, voltage rating, output-current rating,
current-limit setting, and static conduction thermal budget. The
reverse-current and inrush rules use the same target/switch binding but require
explicit backfeed-blocking and soft-start evidence before e-stop rails can be
signed off.

Keep this module limited to static current, nominal-voltage, thermal-rise, and
DC voltage-drop budget screens.
Connector contact heating, wire gauge derating, crimp quality, harness routing,
pulsed current, PWM ripple, vibration, regeneration, and hot-plug behavior need
more specific source-backed evidence before they are treated as sign-off.

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
`ScenarioSuggestionReport::new` and
`schemas/scenario_suggestion_report.schema.json` enforce that split: runnable
suggestions must omit `required_inputs`, non-runnable suggestions must include
at least one concrete missing input, and required-input text must not be blank.

Profile-aware suggestion remediation is opt-in. `suggest-scenarios --profile
iot_basic_v0` first emits the normal evidence-driven suggestions, then compares
declared and already-suggested checks against the shared core profile list in
`validation_profiles.rs`. Missing core checks become non-runnable remediation
templates with one concrete `required_inputs` entry. Do not duplicate a profile
template when a normal suggestion already covers the check.

`set-manufacturing-metadata` is an evidence-enrichment command for board/order
facts that cannot be inferred from Gerber, Excellon, schematic, or component
model data. It may add or replace fields under `board.manufacturing`, but it
must not mutate layout artifacts or invent process defaults. Generated projects
use absolute library paths so follow-on validation and suggestion commands do
not depend on the caller's current directory.

Power-tree load-switch suggestions may infer the control pin state only from a
direct rail/ground tie or exactly one positive-valued pull resistor to a direct
rail/ground state matching the model's `power_switch.enabled_state`. Dividers,
multiple pulls, opposite-state pulls, and MCU/digital control nets must remain
non-runnable until explicit pin-state evidence is supplied.

When adding a new manufacturing rule, update suggestions only when the Board IR
evidence can identify the applicable source condition. For package-scoped
stencil checks, require owner-backed repeated pitch/grid evidence before
suggesting the rule.

Reset/boot suggestions use the same discipline: UART bootloader sync templates
become runnable only when the sender endpoint is proven output-capable, reset
timing is derived from explicit RC evidence, exactly one matching
`board.runtime.reset_release[]` record, or exactly one datasheet-backed reset
supervisor that monitors the same target rail and drives the same reset net.
Any required boot mode must still be proven by direct strap state rather than
assumed firmware behavior. Standalone `RESET_RELEASE_AFTER_POWER_VALID`
suggestions use the same runtime and source-backed supervisor timing evidence.
`CONTROL_LINE_RELEASE_SEQUENCE` suggestions are generated only from complete
`board.runtime.control_line_sequences[]` records; the record is already the
reviewed reduced semantic model of host-line effects and events.
Standalone `BOOT_STRAP_DEFINED` suggestions may fill `straps[].actual` and
become runnable only when every required strap is directly tied to a declared
powered rail or ground; resistor bias and digital nets remain explicit
evidence-gathering templates.

GPIO backdrive suggestions likewise require explicit runtime evidence before
they become runnable. Connectivity and power-state metadata can identify a risk,
but `board.runtime.gpio_backdrive[]` must prove the high driver state, input
victim mode, and schematic series resistance for the exact endpoint pair.

Interface-protection channel suggestions are runnable only for non-generic
datasheet-backed models with complete channel direction, supply-pin,
supply-power-state, active supply-constraint voltage, and unpowered-isolation
metadata. If a declared enable/OE pin is directly tied to a powered rail or
ground in the disabled state, suggestions may include that pin-state evidence.
Generic models and incomplete channel metadata must remain non-runnable review
templates.

`BUS_TERMINATION_VALID` is scenario-driven rather than inferred from a net name
or protocol. The validator requires explicit endpoint-role metadata, declared
line nets, declared expected resistance/tolerance, and a specific resistor
component with `spice.value_ohm`. This keeps reusable boards from being
incorrectly treated as always-terminated when only some harness population
variants should install the 120 ohm resistor.

`BUS_PROTECTION_PLACEMENT_VALID` is also scenario-driven. It requires explicit
bus line nets, a reference component, a checked protection or termination
component, finite placement coordinates, and ordered continuous route evidence
for both lines. The validator projects each component onto each line route and
checks both off-route tolerance and route distance. It is a deterministic layout
evidence guard; it does not model ESD surge current, parasitic inductance, cable
EMC, or differential signal integrity.

`MOTOR_ROUTE_CURRENT_VALID` keeps motor copper screening similarly explicit. It
does not infer current capacity from copper weight or router defaults. A
scenario must provide route nets, route current evidence or a motor-load current
source, and `max_current_density_A_per_mm`. The validator then compares that
policy to the minimum imported segment width for each route. This gives CAD
imports an executable first-pass guard while leaving SOA, switching loss,
temperature rise, thermal vias, and shared copper pours to later evidence.

`MOTOR_BRIDGE_SOA_VALID` follows the same evidence discipline for power-stage
stress. It uses the shared analog SOA metadata parser/interpolator, but emits
motor-drive findings keyed to the bridge scenario when VDS/ID curves are used.
For power-block datasheets, it first checks
`motor_bridge.system_soa.output_current_temperature_curves` with explicit
current-source and board-temperature inputs. Missing SOA curves are critical so
a fabrication review cannot pass on reference-loss and switching screens alone.

`MOTOR_CURRENT_SENSE_PLACEMENT_VALID` follows the same explicit-evidence
pattern for shunt placement and sense routes. It requires component placement
records and route polylines; it does not derive Kelvin quality from schematic
net names or infer current-sense accuracy from shunt value alone.

`MODEL_QUALITY_REQUIRED` is deliberately generic. Validation already emits
non-blocking `LOW_CONFIDENCE_MODEL` limitations for every weak model, but
fabrication sign-off needs a narrower critical gate: only the named components
in the scenario are compared against the explicit allowed source list and
minimum confidence threshold. This keeps exploratory design envelopes usable
while making selected critical components fail closed.

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
