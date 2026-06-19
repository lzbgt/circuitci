# CircuitCI Rust Desktop Frontend

CircuitCI includes an optional native Rust desktop frontend behind the `gui`
feature. The GUI is a workflow and observation shell around the same validation
engine used by the CLI; it does not fork a second validation implementation.

Run it with:

```bash
cargo run --features gui --bin circuitci-gui
```

## Architecture

The desktop app is built with `egui`/`eframe` and is intentionally isolated from
the default CLI dependency graph:

```text
circuitci-gui
  -> src/gui.rs
  -> src/gui/gui_core_tests.rs (GUI test builds only)
  -> src/gui/shell.rs
  -> src/gui/import_flow.rs
  -> src/gui/file_dialogs.rs
  -> src/gui/jobs.rs
  -> src/gui/project.rs
  -> src/gui/sketch.rs
  -> src/gui/sketch_layout.rs
  -> src/gui/sketch_actions.rs
  -> src/gui/sketch_bundles.rs
  -> src/gui/sketch_canvas.rs
  -> src/gui/sketch_canvas_hits.rs
  -> src/gui/sketch_canvas_interaction.rs
  -> src/gui/sketch_canvas_menus.rs
  -> src/gui/sketch_canvas_render.rs
  -> src/gui/sketch_scope_activity.rs
  -> src/gui/sketch_scope_feedback.rs
  -> src/gui/scope_auto_probes.rs
  -> src/gui/sketch_hierarchy.rs
  -> src/gui/sketch_inspector.rs
  -> src/gui/sketch_selection_inspector.rs
  -> src/gui/sketch_net_labels.rs
  -> src/gui/sketch_navigator.rs
  -> src/gui/sketch_probes.rs
  -> src/gui/sketch_spice.rs
  -> src/gui/sketch_symbols.rs
  -> src/gui/library.rs
  -> src/gui/simulation.rs
  -> src/gui/analog_overview.rs
  -> src/gui/analog.rs
  -> src/gui/analog_tests.rs (GUI test builds only)
  -> src/gui/spice.rs
  -> Board IR loading
  -> component model binding
  -> scenario suggestions
  -> validation/report engine
  -> waveform/artifact/report observation
```

Normal `cargo build --release` and CI validation builds do not compile GUI
dependencies unless `--features gui` is explicitly enabled.

`src/gui.rs` owns application state, the `eframe` update loop, and shared
validation/report command helpers, with focused core GUI regressions split into
`src/gui/gui_core_tests.rs`.
`src/gui/shell.rs` owns the desktop shell chrome: menu bar, workflow stage bar,
left project panel, status panel, central stage routing, Project landing view,
Reports view, and finding/limitation rendering. The Sketch stage intentionally
hides the project side panel so the schematic canvas becomes the primary model
view, with project/import/library details available through other stages or
docked secondary panels. `src/gui/import_flow.rs` owns
the Import stage UI and KiCad schematic, KiCad PCB, and SPICE deck import
command wiring.
`src/gui/file_dialogs.rs` owns native open/save/folder dialog integration for
project, import, and output path fields. The dialogs are compiled only with the
optional GUI feature and do not affect the default CLI build.
`src/gui/jobs.rs` owns GUI background jobs for validation, scenario
suggestions, and KiCad/SPICE import actions. Long-running work runs on a worker
thread and reports back to the UI through a channel; cancel requests set a
shared worker flag, terminate external ngspice validation processes where
possible, stop scenario suggestions and KiCad/SPICE import jobs at safe
phase-boundary checkpoints, and still mark any late in-flight result to be
ignored when a worker returns. It also owns lightweight progress events for the
active job and the capped recent-job history used by the status panel to show
final outcome, elapsed time, diagnostics, and output paths for completed,
failed, stale, or canceled background actions. Supported early-stop paths use a
typed operation-canceled error so checkpoint cancellation is shown as
`canceled`, not as an ordinary failure.
`src/gui/project.rs` owns project summary/YAML
load, save, parse validation, import path/name helpers, shared Board IR
undo/redo history, and the unsaved-change confirmation guard used before
load/import/quit actions. `src/gui/sketch.rs` owns the Board IR graph snapshot,
sketch data types, persisted schematic node positions/styles, schematic wire
route waypoint metadata, shared sketch YAML helpers, and model-port default
pin/net seeding for library-backed component insertion. `src/gui/sketch_layout.rs`
owns graph layout helpers, view-state transforms, schematic grid/snap helpers,
fit-all and fit-selection bounds, bounded full-list logical layout for pannable
imported designs, orthogonal wire geometry, wire hit-testing, and model-aware
pin-anchor layout primitives. Pin anchors are colored from the connected Board IR net kind and
show compact pin/kind chips while hovered, selected, connected-highlighted, or
used as an active wire target; this is a canvas affordance, not another
connectivity model. `src/gui/sketch_routes.rs` owns orthogonal schematic wire-route
geometry helpers used for custom route display, hit-testing, insertion, active
wire previews, and drag previews. `src/gui/sketch_wire_draft.rs` owns the
transient in-progress wire-bend points while a pin connection is being drawn.
`src/gui/sketch_duplicate.rs` owns the selected-component/local-net duplication
YAML mutation used by the Sketch toolbar, shortcut, and context menu.
`src/gui/sketch_rename.rs` owns structured component/net rename mutations that
rewrite Board IR graph keys, schematic metadata keys, and generated analog
scenario references before reparsing the edited YAML. Focused rename
regressions live in `src/gui/sketch_rename_tests.rs`.
`src/gui/sketch_spice.rs` owns selected-component SPICE primitive/value edits
for Board IR component-level evidence. It validates the required pin convention
for passive and independent-source primitives, updates exact generated/source
branch probe expressions when a primitive prefix changes, and reparses the
edited YAML before the GUI accepts the mutation.
`src/gui/sketch_inline_edit.rs` owns canvas inline component ID and scalar
SPICE value editing. Double-clicking a scalar SPICE component opens the value
editor; double-clicking other components opens the ID editor; context menus
expose both actions explicitly when supported. Inline value edits accept
engineering suffixes such as `4.7k`, `100n`, `1u`, `10m`, and `2M`, then route
through the same validated rename/SPICE mutation helpers used by the inspector.
`src/gui/sketch_component_labels.rs` owns visible schematic component
reference/value labels and display-only `board.schematic.component_labels`
positions. Reference text derives from component IDs, value text derives from
scalar SPICE evidence, dragging a label only moves schematic metadata, and
double-click/context actions route back to the same inline ID/value editors.
Its dock controls expose transient reference/value visibility plus
auto-arrange/reset actions for persisted label positions.
`src/gui/sketch_canvas.rs` owns the Sketch-stage canvas shell: canvas drawing
order, event routing, marquee and drag dispatch, pin-anchor drag-to-wire
completion, overview-minimap event routing, and graph hover/runtime routing.
`src/gui/sketch_canvas_hits.rs` owns canvas hover and press-origin target
projection for rendered graph items, minimap exclusion, probe/bundle/label
badges, and runtime `scope` chip hit-testing.
`src/gui/sketch_scope_activity.rs` owns the runtime Scope Activity canvas
legend, overlay visibility checkbox, searchable loaded-trace browser, and
direct Scopes trace-open actions for loaded schematic targets.
`src/gui/sketch_canvas_tools.rs` owns canvas helper actions for active
multi-bend wire drawing, direct schematic wire-route edits, component placement
orientation controls, selected-component orientation transforms, canvas probe
defaults, and viewport pan/zoom input.
`src/gui/sketch_scope_feedback.rs` owns armed scope-probe hover target
projection, valid/invalid feedback geometry, and canvas feedback painting for
the V/I/P scope placement tools.
`src/gui/scope_auto_probes.rs` owns the Sketch/Scopes Auto Probes button plus
the guarded Auto-before-Run preference and Run Readiness probe preview. These
paths add or preview bounded missing voltage probes for analog scenario nodes
and source-branch current probes while skipping already covered expressions.
`src/gui/sketch_alignment.rs` owns transient alignment-guide derivation,
drawing, and optional guide-snap target adjustment for component placement plus
selected-node and selected-group drag affordances. The primary schematic toolbar
exposes Grid, grid-step, and Free/Grid/Guides/Grid+Guides snap modes so users
can tune placement behavior without opening secondary panels. It does not create
a second Board IR model; persistence still routes through validated schematic
position edits.
`src/gui/sketch_canvas_interaction.rs` owns reusable canvas interaction
primitives: viewport zoom math, schematic canvas sizing, wire target
hit-testing, route-handle hit-testing, and placement orientation cycling.
`src/gui/sketch_render.rs` owns graph node and pin-anchor
paint helpers, including runtime tinting, transient loaded-waveform `scope`
chips with shared paint/hit-test geometry, opacity handling, symbol glyph
dispatch, and kind-aware pin chips.
`src/gui/sketch_canvas_render.rs` owns canvas-local
tooltip and paint helpers for wires, route handles, wire previews, wire target
affordances, snap/free target feedback, and placement ghosts. `src/gui/sketch_canvas_menus.rs` owns
right-click menus for component, net, wire, probe-badge, and blank-canvas
targets, including route-handle actions and named-net/off-page label placement
actions.
`src/gui/sketch_symbols.rs` owns model/id-inferred common-class
symbol selection and the egui glyph drawing used by sketch nodes.
`src/gui/sketch_actions.rs` owns canvas selection state operations,
fit-all/fit-selection/home viewport commands, multi-selected
drag/nudge/alignment/distribution, and batched selected-item deletion as
validated Board IR YAML edits. It also owns the transient component clipboard
used to copy selected components and paste duplicates at a target canvas
location through validated Board IR mutations.
Visual wire creation can start from a rendered pin anchor by click or drag and
can terminate on another pin anchor, an existing net node, or an existing wire;
while dragging, the preview snaps to the valid release target and highlights
that pin, net, or wire.
Pin-to-pin wiring reuses an existing pin net when possible and otherwise
creates a generated Board IR net through the same mutation path instead of
introducing a second connection model.
`src/gui/sketch_bundles.rs` owns conservative, derived net-bundle grouping,
overlay drawing, badge hit-testing, and bundle multi-selection for bracketed,
dot-qualified, and common paired interface nets. Bundle overlays are a visual
navigation aid over scalar Board IR nets and do not persist bus evidence; the
Sketch canvas defaults them off so imported examples open as a connected
schematic network first, with the `Circuit View` toggle enabling the helper
overlay only when users explicitly audit derived groups. The same panel owns
the runtime-only Scope Activity overlay toggle and searchable loaded-trace jump
browser; those chips and tints mark loaded waveform trace targets and are not
schematic components, pins, or nets.
`src/gui/sketch_net_labels.rs` owns persisted schematic named-net and off-page
connector label metadata under `board.schematic.net_labels`, plus label badge
layout, drawing, hit-testing, typed create-or-reuse placement, inline
edit/autocomplete, selected-net rename routing, drag repositioning, active-wire
target handling, peer-label navigation, conversion, deletion, and net-rename
cleanup. Inline editing a label to an existing net retargets only that label;
editing it to a missing net renames the underlying Board IR net through the
validated rename path. Selecting a net traces its wires, peer labels, and
connected pin anchors as a transient canvas highlight. Labels select and
annotate ordinary Board IR nets; they do not create hidden net ties or
hierarchical connectivity.
`src/gui/sketch_hierarchy.rs` owns the Sketch-stage schematic hierarchy panel:
it derives sheet-like groups from imported KiCad `source.instances[*].path`
metadata and importer namespace prefixes such as `sheet__R1`, then selects or
fits those components and nets without writing a persisted hierarchy model.
It also owns transient focus/isolate view filters that dim or hide unrelated
canvas objects while preserving the same flattened Board IR connectivity.
When a focused net is also used by components outside the focused group, it
draws an off-sheet connector badge listing those external component pins.
`src/gui/sketch_minimap.rs` owns the transient schematic overview/minimap that
summarizes current graph bounds, draws the visible viewport rectangle, and maps
click/drag interaction back into `sketch_pan` for fast navigation. It does not
persist Board IR, hierarchy, bus, or layout evidence.
`src/gui/sketch_navigator.rs` owns the Sketch-stage object navigator: it
derives searchable component, net-bundle, net, wire, and probe rows from the active
`ProjectSnapshot`, selects the corresponding canvas target, and fits the
viewport to visible navigator targets without writing Board IR.
`src/gui/sketch_inspector.rs` owns the selected component/net inspector,
structured scalar YAML edit actions, conservative component/unreferenced-net
add/remove operations, structured component/net rename controls, schematic
symbol style edits, validated component pin assignment, visual wire assignment
mutations, selected-net voltage-probe insertion controls, and selected-component
current-probe insertion for
generated source branches, generated passive current-sense branches, or
generated semiconductor current-sense branches, plus selected-component power
probe insertion for those same supported branches.
`src/gui/sketch_selection_inspector.rs` owns the multi-selection inspector
summary, on-canvas selection frame/move handle, quick toolbar, and quick
actions for fitting, clearing, nudge, align, distribute, orientation changes,
copy, duplicate, and delete; these actions reuse existing canvas selection and
validated Board IR edit paths.
`src/gui/sketch_probes.rs` owns derived schematic voltage/current/power probe
badge targeting, badge layout, badge hit-testing, and badge drawing.
`src/gui/library.rs` owns component model browsing over the active project
library set, text filtering, selected-model staging, selected-component model
assignment, and model-backed component insertion/placement through the same
Board IR YAML mutation path. Inserted components use the selected model's
declared ports to seed editable Board IR pin bindings and generated per-pin
nets, and Sketch-stage placement can target the current view center, an armed
blank-canvas click, a drag/drop release with live ghost and snap feedback, or
the blank-canvas context-menu pointer. `R` / `Shift+R` rotate armed component
placement ghosts before insertion, `F` flips them, and `Shift+F` cycles the
previewed pin side. The accepted placement persists that schematic-only
orientation under `board.schematic.node_styles`.
`src/gui/simulation.rs` owns the Simulation/Scopes stage UI: a runtime-first
oscilloscope workspace with model-run controls and secondary docked
scenario/model/assertion panels.
`src/gui/simulation_forms.rs` owns shared Simulation/Scopes form defaults,
scenario/net/probe combo widgets, stimulus field loading, and status-color
helpers used by those docked editors. `src/gui/analog_overview.rs` owns the
read-only generated scenario audit snapshot, readiness diagnostics, and quick
editor navigation actions shown before edit panels.
`src/gui/waveform.rs` owns Scopes state orchestration,
simulation-time scrub/playback controls, value-scale controls, cursor
measurement tools, selected-plus-pinned cursor readout table, cursor/visible-window region statistics with snapshot capture, actionable transient cursor-region, region-stat, and trigger-event measurement snapshots with editable labels/notes, search/source filters, sort/group controls, plot markers, and filtered CSV/Markdown copy/export, GUI-only
transient trace pinning/comparison overlays, per-trace overlay visibility/color
styles, derived waveform channels, promotion of representable derived channels
to Board IR probes/assertions, and graph-hover/runtime activity extraction from
loaded waveform artifacts. Validation workers parse waveform artifacts with
bounded preflight size/row estimates, large-artifact progress warnings, optional
large-artifact deferral, progress/cancel checks, and loaded/deferred/skipped file
diagnostics before the GUI applies the completed report, keeping large CSV files
out of the UI thread while making missing, deferred, and slow artifacts
filterable, actionable, and exportable from Scopes with compact preview-column loaded/unloaded summaries and preview-load-state filters. Deferred artifacts keep header-only
trace previews, can be filtered by file/probe/detail from the selector, and can
be force-loaded individually, all visible matches, or all deferred files through
the same background waveform loader without changing Board IR or the validation
report. Matching-column, remaining-preview-column, and searchable exact preview-column picker loads append selected traces, mark loaded preview labels, skip already loaded columns, and preserve the full
deferred artifact placeholder for later all-column loading. Loaded full artifacts and selected-column loads can be inspected through footprint readouts with compact source memory totals that can be copied as CSV or Markdown, classified/grouped/filtered as full CSV, selected-column, or runtime-only views, sorted/filtered by runtime cost, copied/exported as visible-row CSV memory diagnostics, warned when the estimated f64 data footprint exceeds the runtime budget, and unloaded individually or through guarded visible-row/largest-first preview/confirmation from the runtime Scopes state to free memory; full loads become deferred reload placeholders again, and selected-column loads mark those preview columns unloaded without changing Board IR or reports.
`src/gui/waveform/waveform_io.rs` owns streaming, cancel-aware waveform CSV parsing, report/path/request loading, and selected-column waveform requests used by deferred artifact loads. `src/gui/waveform/waveform_load.rs` owns bounded CSV preflight estimates, header-only trace previews, selected-column diagnostic merging that marks loaded preview labels, skips duplicate selected-column reloads, preserves full deferred placeholders until full load, and converts unloaded full artifacts back into deferred diagnostics. `src/gui/waveform/waveform_load_diagnostics.rs` owns filterable/copyable transient waveform-load diagnostics for loaded/deferred/skipped CSV artifacts, including preview-column loaded/unloaded audit metadata, preview-load-state filtering, row-level selected-column load shortcuts, exact preview-column picking, and runtime unload controls for loaded rows.
`src/gui/waveform/waveform_deferred.rs` owns deferred waveform artifact
placeholders with header-only probe previews, selector-side filtering, and
row/visible/all, matching-column, remaining-preview-column, or exact
searchable preview-column picker background load actions with select-visible helpers that mark loaded preview labels, skip duplicate selected-column
reloads, and keep full deferred artifacts available after partial column
loads.
`src/gui/waveform/waveform_trace_selector.rs` owns waveform and
searchable/grouped trace selection, transient saved compare sets, transient
trace-style controls, split-unit lane toggling, and selected-trace reset
behavior, including loaded-artifact unload actions that drop or shift transient trace references. `src/gui/waveform/waveform_footprint.rs` owns loaded-waveform footprint readouts, compact source memory summaries with CSV/Markdown copy helpers, diagnostics-derived source classification/grouping/filtering, sort/filter projection, visible-row CSV copy/export, memory-budget warnings, and guarded visible-row or largest-first bulk unload preview/confirmation.
`src/gui/waveform/waveform_context.rs` owns pending schematic
probe-to-scope focus, runtime trace/event-to-schematic cross-focus
selection, selected-trace schematic-context strip actions, and scope probe
lookup. `src/gui/waveform/waveform_plot.rs`
owns the primary scope plot drawing, draggable/click-set A/B cursor handles,
direct plot drag/wheel/Shift-wheel interactions, Alt/Option-drag box zoom,
trace overlay selection, min/max decimated trace-point caching for large CSVs,
transient measurement snapshot marker chips with hover and click actions, and
shared-axis or per-unit lane axis scaling.
`src/gui/waveform/waveform_export.rs` owns deterministic runtime SVG rendering
for the current Scopes plot, including visible traces, split-unit lanes,
cursors, trigger markers, snapshot chips, and bounded decimated trace polylines
for copy/export workflows. Export options stay transient and cover report-size
presets plus independent cursor, trigger, and snapshot annotation inclusion.
`src/gui/waveform/waveform_view.rs` owns the Scopes plot orchestration, cursor
readout table, playback controls, transient visible time-window and
value-window fit/zoom/pan helpers, Back/Forward view-window history, scope plot
SVG copy/export actions, and measurement snapshot display.
`src/gui/waveform/waveform_snapshots.rs` owns transient cursor-region,
region-stat, and trigger-event measurement snapshot capture, editable labels
and notes, search/source filtering, sort/group projection, plot-marker
derivation, filtered CSV/Markdown serialization/export, Jump
restore, schematic Focus, and rendering over loaded waveform artifacts.
`src/gui/waveform/waveform_bundles.rs` owns timestamped report bundle export
with the configured plot SVG, local index page, README manifest, optional
artifact integrity detail files, and loaded-waveform footprint source totals.
`src/gui/waveform/waveform_bundle_recent.rs` owns recent-bundle folder/index
and integrity-audit opening, path copy actions, missing-folder pruning, guarded
refresh, and previewed/confirmed bounded old-bundle cleanup.
`src/gui/waveform/waveform_bundle_integrity.rs` owns report-bundle artifact
size/SHA-256 metadata, `artifact_manifest.csv`, missing/changed artifact status
checks, and expected/current artifact integrity detail rows.
`src/gui/waveform/waveform_trigger.rs` owns transient selected-trace trigger edge/threshold controls, CSV-derived crossing interpolation, exact event readout rows, and previous/next or row-level trigger jumps.
Focused waveform and scope regressions live in
`src/gui/waveform/waveform_tests.rs` for parser/plot/trigger helper coverage,
`src/gui/waveform/waveform_loading_tests.rs` for waveform loading, deferred-artifact, diagnostics, and footprint coverage,
`src/gui/waveform/waveform_measurement_tests.rs` for cursor, region-stat, and
snapshot measurement coverage, and `src/gui/waveform/waveform_scope_tests.rs`
for app-level Scopes context, style, lane, probe, and compare-set coverage so
interaction work can grow without turning the runtime module into a test
fixture container.
`src/gui/analog_models.rs` owns SHA-backed analog `model_files` listing,
selection, add, hash computation, and remove mutations for declared analog
scenarios.
`src/gui/analog.rs` owns structured analog transient scenario and assertion YAML
generation for generated-from-Board simulations, with focused regression
coverage split into `src/gui/analog_tests.rs`. `src/gui/analog_generated.rs`
owns generated scenario analysis settings, ground/node mapping, component
membership, and associated pin-binding repair.
`src/gui/analog_branches.rs`
owns supported source/passive/semiconductor branch current and power expression
derivation for component probes. `src/gui/analog_stimulus.rs` owns structured
DC and pulse source-primitive listing and value/timing mutation for components
included in generated analog scenarios. `src/gui/spice.rs` owns
file-backed SPICE deck discovery, loading, editing, saving, and save-and-run
actions for imported or hand-authored analog scenarios.
`src/gui/sketch_palette.rs` owns schematic primitive insertion for generic
passives and independent sources. Insertion can target the current view center,
an armed blank-canvas click, a drag/drop release with live ghost and snap
feedback, or the blank-canvas context-menu pointer, and each path writes
generated pin nets, component-level SPICE evidence, and schematic placement in
one validated Board IR edit. Armed primitive placement shares the same
`R` / `Shift+R` rotation shortcuts and persists the accepted orientation as
schematic-only node style metadata.

## Workflow Shell

The first GUI slice provides EDA-style stages rather than a single command
form:

- Project: choose a Board IR project and output directory by typing paths or
  using native file/folder pickers, then load or validate it.
- Import: import native KiCad schematic evidence or SPICE decks into Board IR,
  or enrich an imported Board IR project with KiCad PCB placement/routing
  evidence. Import source and output paths can be typed or selected with native
  open/save dialogs.
- Sketch: shows a visual Board IR graph with selectable component/net nodes,
  a schematic-first model-editor layout with a dominant canvas, compact Run
  control, secondary detail/navigation dock, and collapsed YAML editor,
  common-class symbol-style rendering for resistors, capacitors, inductors,
  diodes, sources, connectors, ICs, and generic blocks, rendered component pin
  anchors, an inspector for component bindings and net connections, structured
  scalar edits, rename controls, inline canvas component ID/value editing,
  visible draggable component reference/value labels with transient visibility
  and auto-arranged display positions, a primitive palette that places generic
  resistors, capacitors, inductors, DC voltage/current sources, and pulse
  voltage/current sources at the current view, a canvas click, drag/drop release
  with orientation-aware snap ghost feedback, or a context-menu pointer with
  pins, nets, SPICE evidence, schematic placement, and optional schematic
  orientation, and
  component-level SPICE primitive/value editing for existing
  component properties, schematic-only
  rotate/flip/pin-side controls for selected components, add/remove controls
  for components and unreferenced nets, draggable component/net node positions,
  primary-toolbar grid visibility, grid-step, and snap-mode controls,
  overview-minimap click/drag panning,
  orthogonal wire visuals with net labels, placed local/off-page named-net
  labels, and junction dots, derived
  net-bundle trunks/badges for bracketed, dot-qualified,
  and common paired interface nets, derived schematic hierarchy sheet groups
  from imported source paths and namespaced component IDs, off-sheet connector
  badges for focused nets with external endpoints, clickable wire-to-net
  selection, blank-canvas drag and touchpad-scroll viewport panning,
  pointer-focused pinch/Cmd-scroll zoom, plus Home, Fit All, and Fit Selection
  controls,
  schematic hierarchy search/select/fit/focus/isolate controls and object
  navigator search/select/fit controls,
  Shift-drag replace selection boxes, Cmd/Ctrl-drag additive selection boxes,
  Alt/Option-drag subtractive selection boxes, `L` plus those same drag chords
  for freehand lasso selection, multi-selection inspector summary/actions,
  on-canvas selected-group frame/move handle with snap/free target feedback,
  alignment guides with optional guide snapping, and quick toolbar with
  rotate/flip/pin-side actions,
  group drag/nudge/edge-align/center-align/distribute controls for
  multi-selected sketch items, keyboard or button
  deletion for selected components/nets, batched deletion of multi-selected
  sketch items, pin-to-net assignment/removal for selected components, visual
  pin-to-pin and pin-to-net
  wire assignment by clicking or dragging component pin anchors to pins, nets,
  or wires with target highlighting and snap preview, clicked intermediate bend
  points while an active wire is being drawn, graph-node
  direct schematic wire-route shaping by dragging a rendered wire segment or
  existing route handle, route-handle insertion/deletion from wire context
  menus, placed net-label/off-page connector badges that select the underlying
  Board IR net, can be moved by dragging, accept active-wire drops onto their
  underlying net, and can be converted or deleted from their context menu,
  runtime tinting, an on-canvas Scope Activity legend/toggle with a searchable
  loaded-trace jump browser, hoverable/clickable `scope` activity chips, hover
  readouts, and context-menu Scopes jumps for matching loaded waveform probes,
  primary-toolbar probe controls that add voltage probes for selected nets or
  current/power probes for selected components,
  one-click `Run + Scopes` validation from the schematic toolbar,
  Auto Probes action and Auto-before-Run option for bounded
  voltage/source-current probe population before validation, Run Readiness
  preview rows for the probes Run will create,
  plus direct `Scope V`, `Scope I`, and `Scope P` actions that create the
  missing probe when needed and open the Simulation-stage Scopes workspace,
  armed `Scope Tool` V/I/P placement buttons that consume a canvas click on a
  net, wire, pin, label, or component before normal selection/wire routing,
  with canvas-hover `V`/`I`/`P` shortcuts, valid/invalid hover target feedback
  before the click, and same-key/Esc cancellation,
  visible voltage/current/power probe badges derived from analog scenario
  probes, badge pass/fail/unknown/unasserted markers derived from the latest
  validation report, badge clicks that open and focus the corresponding
  Simulation-stage scope/probe context, right-click probe-badge action menus,
  right-click component/net/wire action menus, hovered-badge assertion
  creation/clearing, hovered-badge deletion that removes the probe and
  dependent assertions through a validated Board IR edit, selected-net
  voltage-probe insertion into existing analog scenarios,
  selected-component current-probe insertion for
  supported generated SPICE branches, selected-component power-probe insertion
  for supported generated SPICE branches, shared undo/redo for Board IR
  graph/property/wire/YAML edits, duplicate selected components with copied
  local nets and offset schematic positions, copy/paste of selected components
  to a target canvas location, and a raw Board IR YAML editor with
  parse-validated save.
- Library: shows library bindings, searches the active component model set,
  stages a model for new components, inserts selected models as Board IR
  components with generated default pin nets, assigns a selected model to the
  selected component, and shows scenario suggestion YAML.
- Simulation/Scopes: presents a runtime-first oscilloscope workspace linked
  from the schematic `Run`/`Scopes` controls, with a dominant plot, waveform
  selection, searchable/grouped trace selection, transient selected-probe trace
  pinning, saved compare sets, and per-trace visibility/color styles for
  multi-trace comparison overlays, optional per-unit split lanes for mixed
  voltage/current/power compares, Alt/Option-drag box zoom for shared-axis
  time/value windows or split-lane time windows, Back/Forward view-window
  history, direct
  plot drag time/value-window panning, wheel time zoom, Shift-wheel value zoom, explicit time-window and value-scale controls,
  draggable/click-set A/B cursor handles, selected-trace trigger threshold markers, exact event readout rows, previous/next or row-level edge jumps, trace/edge-to-schematic focus with context strip `Open Sketch`/`Fit Context` actions, play/scrub controls,
  selected-plus-pinned A/B cursor readouts, cursor/visible-window region statistics with min/max/mean/RMS rows and snapshot capture, current-plot SVG copy/export for reports, and searchable/source-filtered transient measurement snapshots from cursor regions, region stats, or trigger events with editable labels/notes, interactive plot marker chips plus row-level Jump, schematic Focus, filtered CSV/Markdown copy/export actions, and timestamped report bundles containing the configured plot SVG, filtered snapshot CSV/Markdown, local index page, and README manifest with loaded-waveform footprint source totals.
  If a schematic has no analog scope probes, `Run` prepares the Scopes workflow
  by adding a generated transient voltage probe on the default non-ground net
  before validation. The selected trace side dock also includes a bounded
  frequency-domain peak readout derived from the loaded transient waveform.
  Scenario setup is secondary and docked: users can append a generated-from-Board
  `analog_transient` scenario with ground/probe net selection, audit generated
  scenario timing/backend, source/probe/assertion/model-file/node-binding
  coverage plus readiness gaps with quick editor navigation, edit generated
  scenario stop time/max step, ground net, SPICE node bindings, and component
  membership, add selected-net voltage probes to existing analog scenarios,
  inspect a selected probe badge's assertion rows with
  threshold/timing/status/failure details, edit or delete one assertion without
  clearing sibling checks on that selected probe, add or clear assertions for
  that selected probe, quick-add cursor-sampled above/below assertions from a
  hovered schematic probe badge, add sample/min/max probe assertions, edit
  file-backed SPICE decks declared by analog scenarios, run validation through
  the engine, plot emitted CSV waveforms, add GUI-only derived
  difference/sum/product/ratio channels, promote representable derived channels
  to explicit analog probes or probes plus assertions, and inspect generated
  SPICE decks, artifacts, findings, and limitations without turning the scope
  view into a form-first page.
- Reports: displays the generated Markdown validation report.

The menu bar exposes File, Workflow, Simulation, and Help actions. Load, import,
and quit actions route through the project dirty-state guard so unsaved Board IR
YAML or file-backed SPICE deck edits cannot be replaced without an explicit
save, discard, or cancel choice.

## Simulation Boundary

The GUI does not make CircuitCI a full in-house analog simulator. CircuitCI
continues to use SPICE-class backends such as ngspice for nonlinear analog
solving and fails closed when required models, assertions, or backends are
missing.

The supported desktop simulation path is:

1. load Board IR or import KiCad schematic/PCB/SPICE evidence,
2. choose project, output, KiCad, and SPICE import paths through native
   open/save/folder dialogs or direct text entry,
3. inspect the imported/sketched component and net graph,
4. edit selected component IDs and scalar SPICE values inline on the schematic
   canvas or through structured controls, and edit component
   model/part-number, net IDs, and net kind/voltage/powered fields through
   structured controls,
5. add components, add nets, or remove selected components and unreferenced
   nets through validated graph controls,
6. drag component/net graph nodes to persist `board.schematic.node_positions`,
7. snap dragged schematic positions to the visible grid when snap is enabled,
8. pan, zoom, Home-reset, Fit All, or Fit Selection the sketch viewport without
   changing Board IR evidence,
9. search components, net bundles, nets, wires, and probe badges in the object navigator,
   select matching canvas targets, and fit the viewport to visible targets
   without changing Board IR evidence,
10. use the schematic hierarchy panel to select, fit, focus, or isolate derived
   KiCad sheet or importer-namespace groups without changing Board IR evidence,
11. inspect off-sheet connector badges on focused nets to see which external
   component pins still use the same flattened Board IR net,
12. rotate selected components from the canvas with `R` / `Shift+R`, or rotate,
   flip, or choose pin side through the inspector; all write
   `board.schematic.node_styles`,
13. Shift-drag a selection box to replace selection, Cmd/Ctrl-drag to add
    visible components/nets, or Alt/Option-drag to subtract them; hold `L`
    while starting the same drag gestures to freehand-lasso irregular dense
    areas,
14. drag, nudge, align edges/centers, or distribute multi-selected sketch nodes
   as one validated Board IR edit,
15. duplicate selected components with their internally referenced local nets
   as one validated Board IR edit while keeping external nets shared,
16. copy selected components and paste them at the pointer or canvas center
   using the same validated duplication and schematic-position mutation path,
17. rename selected components or nets through validated Board IR edits that
   update schematic metadata and generated analog bindings,
18. assign or remove selected component pin bindings to existing nets,
19. create a visual wire by clicking or dragging from a rendered source pin
   anchor to a destination pin anchor, net node, or existing wire, with the
   preview snapping to and highlighting valid release targets,
20. drag a rendered wire segment to insert a schematic display waypoint under
   `board.schematic.wire_routes`, then drag the visible route handle to refine
   it, insert or delete individual route handles from the wire menu, or clear
   the custom route without changing the underlying pin/net connectivity,
21. place local named-net labels or off-page connector labels for a selected
   net, net node, wire, or typed net name under `board.schematic.net_labels`;
   create a missing typed net with an explicit net kind, rename a selected net
   to the typed label name, double-click or context-edit a label with existing
   net autocomplete, convert or delete label badges, drag them to reposition
   their schematic display point, jump to the next peer label on the same net,
   or finish an active wire on them without changing the underlying Board IR net
   identity,
22. inspect Board IR connections through orthogonal wire routes, net labels,
   junction dots, and clickable wire-to-net or label-to-net selection rendered
   over the persisted pin/net graph,
23. delete selected components or unreferenced nets from the canvas or toolbar,
24. undo or redo Board IR graph/property/wire/YAML edits through the shared
   editor history,
25. receive an unsaved-change confirmation before load/import/quit replaces
   dirty Board IR YAML or loaded file-backed SPICE deck edits,
26. search the active model libraries, insert selected models as sketched
   components with generated pin nets, and assign selected models to existing
   components,
27. edit Board IR YAML evidence when the project needs a correction outside the
   structured controls,
28. append a generated-from-Board analog transient scenario with a voltage probe,
29. select a net or wire on the sketch canvas and append another voltage probe
   to an existing analog scenario when that scenario has a node binding for the
   net,
30. select a component on the sketch canvas and append a current probe when the
   target generated-from-Board scenario includes a source primitive branch or a
   passive/diode/BJT/MOSFET branch where CircuitCI can generate a current-sense
   source,
31. select a supported component branch and append a power probe that composes
   the branch voltage and branch current as an explicit Board IR power probe,
32. add sample or windowed min/max waveform assertions against declared probes,
33. load, edit, save, and rerun file-backed SPICE decks from declared analog
   scenarios,
34. browse, hash, add, and remove SHA-backed SPICE model/include files for
   declared analog scenarios,
35. run KiCad/SPICE imports, scenario suggestions, declared validation, and
   `analog_transient` scenarios in background workers while the desktop shell
   remains responsive,
36. cancel a running background job from the Simulation menu, left project
   panel, or status panel; external ngspice validation subprocesses are
   terminated where possible, scenario suggestions and importers stop at safe
   phase checkpoints, and embedded backend calls still finish before their
   result is ignored; supported checkpoint stops are recorded as canceled job
   outcomes instead of failed job outcomes,
37. review recent background job outcomes in the status panel, including
   elapsed time, output path, and a compact diagnostic detail,
38. watch active background job stages in the status panel as imports,
   suggestions, validation, and analog simulation advance; KiCad/SPICE imports
   report parser, mapping/load, Board IR build/merge, and write phases, while
   validation reports project loading, model loading/binding, scenario
   execution, analog transient scenario/deck/backend/waveform phases, profile
   coverage, report assembly, report writing, and markdown report loading,
39. scrub or play the simulation time cursor to drive graph runtime tinting,
40. hover graph nodes to inspect matching voltage/current/power probe values at
   the current waveform cursor,
41. add schematic probes from the primary toolbar for the selected net or
   component, use visible schematic probe badges to find voltage/current/power
   probes, see latest assertion pass/fail/unknown/unasserted status, jump to
   their Simulation-stage scope trace and selected-probe assertion panel after
   waveform data is loaded, add an assertion from the current assertion-editor
   settings with `A`, edit or delete one assertion row from the selected-probe
   panel, quick-add an above-current-sample check with Shift+A or a
   below-current-sample check with Shift+B, clear assertions for the probe with
   `X`, use the right-click badge menu for the same probe actions, or remove a
   hovered probe badge with Delete/Backspace,
42. use right-click component, net, and wire menus for common sketch actions
   such as inspect/select, start wire, connect an active wire, add voltage,
   current, or power probes, place local/off-page net labels, and delete
   through the same validated Board IR mutation paths as the inspector and
   keyboard actions,
43. observe generated decks, plotted CSV waveforms, derived waveform math
   channels, promote representable derived channels to persistent probes/assertions,
   cursor values, min/max measurements, findings, and report artifacts,
44. edit the project/model evidence and rerun.

Standards-complete symbol libraries and symbol editors, buses, persisted
hierarchical schematic sheets, advanced multi-channel waveform-analysis sign-off, advanced SPICE source
tooling, automatic arbitrary
schematic-to-SPICE conversion, and vendor macromodel acquisition are future GUI
stages. Basic file-backed deck edits are supported, but must still reuse the
existing Board IR, importer, model, and validation contracts instead of creating
a parallel EDA model.

The sketch canvas symbol rendering is deliberately a view-layer affordance. It
infers a compact glyph from the component reference designator and model ID, then
continues to persist only Board IR components, nets, pins, and optional
`board.schematic.node_positions` / `board.schematic.node_styles`.
Visible component reference and scalar-value labels are also view-layer
affordances: their text is derived from Board IR component IDs and scalar SPICE
evidence, while optional dragged positions persist only under
`board.schematic.component_labels`. Reference/value visibility is transient GUI
state; auto-arrange and reset only add or remove those display positions.
The sketch grid, toolbar snap controls, net labels, junction dots, and
orthogonal wire routes are also editor affordances. Grid snapping and guide
snapping are transient user interaction modes; either may update persisted
schematic node positions only through accepted edits. Dragging a wire or one of
its visible route handles may update
`board.schematic.wire_routes` with display-only waypoints, active wire-mode
blank-canvas bend clicks and wire context-menu route-handle insertion/deletion
may update the same display metadata, and placing or dragging a local/off-page
net label may update `board.schematic.net_labels` with a net reference, label
kind, and schematic position. Clicking a net label selects the underlying Board
IR net, and completing an active wire on a net label connects to that same
underlying net. Clicking a wire may select its underlying Board IR net, but grid visibility,
junction dot rendering, hit-test regions, named-net label rendering, custom
schematic wire waypoints, and orthogonal routing style do not create independent
electrical connectivity, hidden net ties, hierarchy ports, or physical PCB
placement evidence.
Custom wire routes render as orthogonal schematic polylines between persisted
waypoints, so route handles can be placed freely while the visible schematic
path remains EDA-style horizontal/vertical geometry.
During wire mode, blank-canvas clicks add transient bend points; completing the
wire on a pin, net node, or existing wire persists those bends as the new
pin-to-net route metadata in the same validated Board IR edit as the electrical
connection. Pressing Delete/Backspace during wire mode removes the latest
pending bend, and Escape cancels the in-progress route.

Canvas probe insertion is also a Board IR scenario edit, not a hidden runtime
probe layer. The selected-net inspector appends a voltage probe to an existing
analog scenario only when that scenario already declares a node binding for the
selected Board IR net. The selected-component inspector appends a current probe
only for generated-from-Board analog scenarios and only when the component
branch is source-backed by a Board IR voltage/current source primitive, by a
Board IR resistor/capacitor/inductor primitive with a generated zero-volt
current-sense source, or by a bound diode/BJT/MOSFET model branch with
CircuitCI's generated zero-volt current-sense source. It appends power probes
for the same supported component set by composing explicit branch voltage and
branch current expressions. Subcircuit internals and file-backed deck branch
probes still require explicit deck/model evidence.

Schematic probe badges are derived overlays over existing analog scenario
probes. Voltage badges attach to Board IR nets only when the probe expression's
SPICE node maps back through `analog.node_bindings`; current and power badges
attach to components only when the expression references a generated/source
branch that CircuitCI can map back to a Board IR component. The badges are not a
second persisted probe model. Badge status markers are derived from the latest
loaded `ValidationReport`: unasserted probes are grey, asserted probes without a
loaded report are unknown, assertion failures are red, non-assertion scenario
failures remain unknown, and asserted probes pass only when the latest report
has no matching assertion failure. Pressing `A` on a hovered badge appends a
normal Board IR analog assertion for that probe using the current
assertion-editor aggregation, relation, threshold, and timing settings.
Pressing `X` removes assertions for that probe while keeping the probe.
Pressing Shift+A or Shift+B on a hovered badge reads the matching loaded
waveform value at the current simulation cursor and appends a sample assertion
with a small 1% threshold margin so the current sample initially satisfies the
new strict `above` or `below` check. If no waveform column matches the probe
expression at the cursor, the quick action fails closed and leaves Board IR
unchanged.
Clicking a probe badge, choosing a probe row in the object navigator, using
the primary toolbar/context-menu scope actions, or arming `Scope Tool` V/I/P
or pressing `V`, `I`, or `P` while the canvas is hovered and clicking the
schematic records the selected scenario/probe as the runtime scope target.
`Scope V`, `Scope I`, and `Scope P` first append the
corresponding Board IR voltage/current/power probe when the selected net or
component does not already have one. The armed voltage tool accepts nets,
wires, pins, and net labels; armed current/power tools accept components,
component labels, and pins. Pressing the same tool key again or Esc cancels the
armed tool. While armed, the canvas outlines the target that will receive the
probe and labels invalid hover targets before the click. `Auto Probes` adds
missing voltage probes for non-ground analog node bindings plus current probes
for supported source branches, bounded per action, and skips probe expressions
that already exist. When `Auto before Run` is enabled, the Sketch and Scopes
Run buttons perform the same missing-probe pass before validation. `Run +
Scopes` starts validation from the schematic and immediately opens Scopes so
the pending probe can focus when waveform traces arrive. The Run Readiness
panel in both workspaces lists the planned voltage/current probe names,
expressions, and targets before validation. If waveform artifacts are already
loaded, the Scopes view selects the matching trace immediately and the Sketch
canvas marks matching nodes/components with a transient `scope` activity chip;
otherwise the target is applied after the next successful Run loads waveform
CSV data. The activity chip uses a pointing cursor and primary-click jump to
the matching loaded trace. Matching runtime nodes also expose `Open Runtime
Trace in Scopes` in their context menu so users can jump from the schematic
back to the loaded trace. Selecting or focusing a
Scopes trace or trigger-event row can select the originating schematic probe's
net or component while staying in the runtime workspace. When that mapping
exists, Scopes shows a schematic-context strip with target, probe, scenario,
and expression plus `Open Sketch` and `Fit Context` actions. This is transient
GUI selection over the existing Board IR target, not persisted waveform
metadata.
When no analog probe exists yet, Scopes `Run` creates a default generated
transient scenario or adds a voltage probe to the first existing analog
scenario with node bindings, records that probe as the pending scope target,
then saves and validates the project so users get an inspectable voltage trace
instead of a report with no oscilloscope waveform.
Measurement snapshot labels and notes can be edited inline, searched,
source-filtered, sorted, and grouped across cursor/trigger/region observations;
Copy CSV/Markdown and Export CSV/Markdown operate on the currently visible projected rows and include those labels and
notes. Rows can also restore their captured
trace/cursor/time context or cross-focus the linked schematic probe while
remaining runtime-only. Visible snapshot marker chips are derived from the same
transient rows, draw only for currently visible selected/pinned traces, show
hover details, and support click-to-Jump plus Shift-click schematic Focus.
The current Scopes plot can also be copied or exported as SVG; this is a
runtime image artifact derived from loaded waveform CSVs, visible traces,
cursors, trigger markers, and snapshot chips, not a persisted project report.
The SVG export controls can choose compact/report/wide figure dimensions and
can omit cursor, trigger, or snapshot annotations for cleaner report figures
without changing the live Scopes view. Long traces are exported as bounded
min/max decimated polylines so report SVG files do not scale linearly with every
loaded waveform sample.
Export Bundle writes the same filtered snapshot CSV and Markdown plus the
configured plot SVG, a local `index.html`, a README manifest, and
`artifact_manifest.csv` plus optional artifact integrity detail CSV/Markdown
audit files into a timestamped output folder,
keeping report artifacts together while still avoiding persisted project truth.
The index links the plot SVG, snapshot CSV/Markdown, README, manifest, and
optional integrity detail files while surfacing the same snapshot, plot,
selected-trace, generated content artifact size/SHA-256 metadata, and
loaded-waveform footprint summary context for quick browser review. The README
records active snapshot filters, plot SVG options, selected trace context,
loaded-waveform footprint source totals, generated files, and human-readable
artifact size/SHA-256 metadata; the CSV manifest records expected size/SHA-256
metadata for required bundle files so the GUI can distinguish changed artifacts
from merely present artifacts. The optional integrity detail files are derived
from that manifest and intentionally not included in the manifest they
describe. The Scopes panel keeps a bounded transient list of recently exported
bundles, prunes entries whose folders no longer exist, shows whether each
remaining recent bundle still has required and unchanged artifacts, and can
open the latest bundle index, latest folder, or older bundle folder through
the host file manager. Recent bundles also expose direct open actions for the
generated integrity CSV/Markdown audit files, with explicit missing-file status
messages if an audit artifact was removed. A compact `Copy Path` control copies
the latest bundle folder, index, or integrity audit file path, and older bundle
rows can copy the folder path for issue trackers or lab notes. `Details` opens a transient
integrity table with each artifact's OK/Missing/Changed state plus
expected/current size and SHA-256 values, can reveal only missing, changed, or
untracked problem artifacts, and can copy the current detail projection as CSV
or Markdown for troubleshooting reports. If a recent bundle is missing required
files or has files
that no longer match the manifest, `Preview Refresh` / `Confirm Refresh`
regenerates the plot SVG, snapshot CSV/Markdown, index, README, and manifest
from the current filtered Scopes state into that same
`scope_report_bundle_*` folder. `Clean Old Bundles` previews older direct child directories named
`scope_report_bundle_*` under the configured output directory; `Confirm Cleanup`
then removes only those previewed bundle folders, preserving the newest bounded
set and unrelated output folders.
Right-clicking a probe badge
opens an explicit action menu for opening the probe in Simulation, adding an
assertion from current settings, quick adding above/below cursor-sample
assertions, clearing assertions, or removing the probe. These menu actions call
the same validated Board IR mutation paths as the keyboard shortcuts.
Right-clicking a component, net node, or wire opens the common sketch action
menu for that target. Component menus can inspect/select, start inline ID or
supported scalar-value editing, start wire mode from an existing/default pin,
add or scope current/power probes, or delete the component.
Net and wire menus can inspect/select the underlying net, connect the active
wire to that net, add or scope a voltage probe, or delete the net through the
existing net-removal rules; a wire is still just a rendered view of Board IR pin-to-net
bindings, optionally with schematic-only route waypoints that can be cleared
from the same wire menu, not a separate persisted electrical edge model.
Removing a hovered badge deletes the underlying Board IR analog probe and any
analog assertions that reference it, then re-parses the edited Board IR before
updating the canvas. The Simulation stage mirrors the selected badge context in
a compact assertion table that shows each assertion name, aggregation, relation,
threshold, timing, latest status, and matching failure message when one exists.
Each row can be loaded into the structured assertion editor for name,
threshold, aggregation, relation, or timing changes, or deleted without removing
the probe or sibling assertions.
