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
  -> src/gui/shell.rs
  -> src/gui/import_flow.rs
  -> src/gui/file_dialogs.rs
  -> src/gui/jobs.rs
  -> src/gui/project.rs
  -> src/gui/sketch.rs
  -> src/gui/sketch_actions.rs
  -> src/gui/sketch_bundles.rs
  -> src/gui/sketch_canvas.rs
  -> src/gui/sketch_hierarchy.rs
  -> src/gui/sketch_inspector.rs
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
validation/report command helpers.
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
graph layout helpers, persisted schematic node positions/styles, view-state
transforms, schematic grid/snap helpers, orthogonal wire geometry, wire
hit-testing, fit-all and fit-selection bounds, bounded full-list logical layout
for pannable imported designs, and model-aware pin-anchor layout/rendering
primitives.
`src/gui/sketch_duplicate.rs` owns the selected-component/local-net duplication
YAML mutation used by the Sketch toolbar, shortcut, and context menu.
`src/gui/sketch_rename.rs` owns structured component/net rename mutations that
rewrite Board IR graph keys, schematic metadata keys, and generated analog
scenario references before reparsing the edited YAML.
`src/gui/sketch_spice.rs` owns selected-component SPICE primitive/value edits
for Board IR component-level evidence. It validates the required pin convention
for passive and independent-source primitives, updates exact generated/source
branch probe expressions when a primitive prefix changes, and reparses the
edited YAML before the GUI accepts the mutation.
`src/gui/sketch_canvas.rs` owns the Sketch-stage canvas shell: canvas drawing
order, viewport input, hit-test routing, marquee and drag event routing, wire
preview drawing, node/wire/probe/bundle/hierarchy connector tooltips, and
right-click context menus over component, net, wire, and probe badge targets.
`src/gui/sketch_symbols.rs` owns model/id-inferred common-class
symbol selection and the egui glyph drawing used by sketch nodes.
`src/gui/sketch_actions.rs` owns canvas selection state operations,
fit-all/fit-selection/home viewport commands, multi-selected
drag/nudge/alignment/distribution, and batched selected-item deletion as
validated Board IR YAML edits. It also owns the transient component clipboard
used to copy selected components and paste duplicates at a target canvas
location through validated Board IR mutations.
Visual wire creation can start from a
rendered pin anchor and can terminate on another pin anchor or an existing net
node. Pin-to-pin wiring reuses an existing pin net when possible and otherwise
creates a generated Board IR net through the same mutation path instead of
introducing a second connection model.
`src/gui/sketch_bundles.rs` owns conservative, derived net-bundle grouping,
overlay drawing, badge hit-testing, and bundle multi-selection for bracketed,
dot-qualified, and common paired interface nets. Bundle overlays are a visual
navigation aid over scalar Board IR nets and do not persist bus evidence.
`src/gui/sketch_hierarchy.rs` owns the Sketch-stage schematic hierarchy panel:
it derives sheet-like groups from imported KiCad `source.instances[*].path`
metadata and importer namespace prefixes such as `sheet__R1`, then selects or
fits those components and nets without writing a persisted hierarchy model.
It also owns transient focus/isolate view filters that dim or hide unrelated
canvas objects while preserving the same flattened Board IR connectivity.
When a focused net is also used by components outside the focused group, it
draws an off-sheet connector badge listing those external component pins.
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
`src/gui/sketch_probes.rs` owns derived schematic voltage/current/power probe
badge targeting, badge layout, badge hit-testing, and badge drawing.
`src/gui/library.rs` owns component model browsing over the active project
library set, text filtering, selected-model staging, selected-component model
assignment, and model-backed component insertion through the same Board IR YAML
mutation path. Inserted components use the selected model's declared ports to
seed editable Board IR pin bindings and generated per-pin nets.
`src/gui/simulation.rs` owns the Simulation/Scopes stage UI: a runtime-first
oscilloscope workspace with model-run controls and secondary docked
scenario/model/assertion panels. `src/gui/analog_overview.rs` owns the
read-only generated scenario audit snapshot, readiness diagnostics, and quick
editor navigation actions shown before edit panels.
`src/gui/waveform.rs` owns waveform CSV parsing, the primary scope plot,
simulation-time scrub/playback controls, cursor measurement tools, GUI-only
derived waveform channels, promotion of representable derived channels to Board
IR probes/assertions, and graph-hover/runtime activity extraction from loaded
waveform artifacts.
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
passives and independent sources, including generated pin nets, component-level
SPICE evidence, and schematic placement in one validated Board IR edit.

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
  scalar edits, rename controls, a primitive palette that inserts generic
  resistors, capacitors, inductors, DC voltage/current sources, and pulse
  voltage/current sources with pins, nets, SPICE evidence, and schematic
  placement, and component-level SPICE primitive/value editing for existing
  component properties, schematic-only
  rotate/flip/pin-side controls for selected components, add/remove controls
  for components and unreferenced nets, draggable component/net node positions,
  schematic grid/snap controls, orthogonal wire visuals with net labels and
  junction dots, derived net-bundle trunks/badges for bracketed, dot-qualified,
  and common paired interface nets, derived schematic hierarchy sheet groups
  from imported source paths and namespaced component IDs, off-sheet connector
  badges for focused nets with external endpoints, clickable wire-to-net
  selection, blank-canvas drag and touchpad-scroll viewport panning,
  pointer-focused pinch/Cmd-scroll zoom, plus Home, Fit All, and Fit Selection
  controls,
  schematic hierarchy search/select/fit/focus/isolate controls and object
  navigator search/select/fit controls,
  Shift-drag marquee selection, group drag/nudge/edge-align/center-align/
  distribute controls for multi-selected sketch items, keyboard or button
  deletion for selected components/nets, batched deletion of multi-selected
  sketch items, pin-to-net assignment/removal for selected components, visual
  pin-to-pin and pin-to-net
  wire assignment by clicking component pin anchors or net nodes, graph-node
  runtime tinting and hover readouts for
  matching waveform probes, visible voltage/current/power probe badges derived
  from analog scenario probes, badge pass/fail/unknown/unasserted markers
  derived from the latest validation report, badge clicks that open the
  corresponding Simulation-stage probe context, right-click probe-badge action
  menus, right-click component/net/wire action menus, hovered-badge assertion
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
  from the schematic `Run`/`Scopes` controls, with a dominant plot, waveform and
  probe selection, play/scrub controls, and A/B cursor measurements. Scenario
  setup is secondary and docked: users can append a generated-from-Board
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
4. edit selected component IDs, component model/part-number, net IDs, and net
   kind/voltage/powered fields through structured controls,
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
12. rotate, flip, or choose pin side for selected components through
   `board.schematic.node_styles`,
13. Shift-drag a marquee to select multiple visible components/nets,
14. drag, nudge, align edges/centers, or distribute multi-selected sketch nodes
   as one validated Board IR edit,
15. duplicate selected components with their internally referenced local nets
   as one validated Board IR edit while keeping external nets shared,
16. copy selected components and paste them at the pointer or canvas center
   using the same validated duplication and schematic-position mutation path,
17. rename selected components or nets through validated Board IR edits that
   update schematic metadata and generated analog bindings,
18. assign or remove selected component pin bindings to existing nets,
19. create a visual wire by clicking a rendered source pin anchor and then a
   destination pin anchor or net node,
20. inspect Board IR connections through orthogonal wire routes, net labels,
   junction dots, and clickable wire-to-net selection rendered over the
   persisted pin/net graph,
21. delete selected components or unreferenced nets from the canvas or toolbar,
22. undo or redo Board IR graph/property/wire/YAML edits through the shared
   editor history,
23. receive an unsaved-change confirmation before load/import/quit replaces
   dirty Board IR YAML or loaded file-backed SPICE deck edits,
24. search the active model libraries, insert selected models as sketched
   components with generated pin nets, and assign selected models to existing
   components,
25. edit Board IR YAML evidence when the project needs a correction outside the
   structured controls,
26. append a generated-from-Board analog transient scenario with a voltage probe,
27. select a net or wire on the sketch canvas and append another voltage probe
   to an existing analog scenario when that scenario has a node binding for the
   net,
28. select a component on the sketch canvas and append a current probe when the
   target generated-from-Board scenario includes a source primitive branch or a
   passive/diode/BJT/MOSFET branch where CircuitCI can generate a current-sense
   source,
29. select a supported component branch and append a power probe that composes
   the branch voltage and branch current as an explicit Board IR power probe,
30. add sample or windowed min/max waveform assertions against declared probes,
31. load, edit, save, and rerun file-backed SPICE decks from declared analog
   scenarios,
32. browse, hash, add, and remove SHA-backed SPICE model/include files for
   declared analog scenarios,
33. run KiCad/SPICE imports, scenario suggestions, declared validation, and
   `analog_transient` scenarios in background workers while the desktop shell
   remains responsive,
34. cancel a running background job from the Simulation menu, left project
   panel, or status panel; external ngspice validation subprocesses are
   terminated where possible, scenario suggestions and importers stop at safe
   phase checkpoints, and embedded backend calls still finish before their
   result is ignored; supported checkpoint stops are recorded as canceled job
   outcomes instead of failed job outcomes,
35. review recent background job outcomes in the status panel, including
   elapsed time, output path, and a compact diagnostic detail,
36. watch active background job stages in the status panel as imports,
   suggestions, validation, and analog simulation advance; KiCad/SPICE imports
   report parser, mapping/load, Board IR build/merge, and write phases, while
   validation reports project loading, model loading/binding, scenario
   execution, analog transient scenario/deck/backend/waveform phases, profile
   coverage, report assembly, report writing, and markdown report loading,
37. scrub or play the simulation time cursor to drive graph runtime tinting,
38. hover graph nodes to inspect matching voltage/current/power probe values at
   the current waveform cursor,
39. use visible schematic probe badges to find voltage/current/power probes,
   see latest assertion pass/fail/unknown/unasserted status, jump to their
   Simulation-stage selected-probe assertion panel, add an assertion from the
   current assertion-editor settings with `A`, edit or delete one assertion
   row from the selected-probe panel, quick-add an above-current-sample check
   with Shift+A or a below-current-sample check with Shift+B, clear assertions
   for the probe with `X`, use the right-click badge menu for the same probe
   actions, or remove a hovered probe badge with Delete/Backspace,
40. use right-click component, net, and wire menus for common sketch actions
   such as inspect/select, start wire, connect an active wire, add voltage,
   current, or power probes, and delete through the same validated Board IR
   mutation paths as the inspector and keyboard actions,
41. observe generated decks, plotted CSV waveforms, derived waveform math
   channels, promote representable derived channels to persistent probes/assertions,
   cursor values, min/max measurements, findings, and report artifacts,
42. edit the project/model evidence and rerun.

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
The sketch grid, snap controls, net labels, junction dots, and orthogonal wire
routes are also editor affordances. Snapping may update persisted schematic
node positions, and clicking a wire may select its underlying Board IR net, but
grid visibility, net-label placement, junction dot rendering, hit-test regions,
and orthogonal routing style do not create independent electrical connectivity
or physical PCB placement evidence.

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
Right-clicking a probe badge opens an explicit action menu for opening the
probe in Simulation, adding an assertion from current settings, quick adding
above/below cursor-sample assertions, clearing assertions, or removing the
probe. These menu actions call the same validated Board IR mutation paths as
the keyboard shortcuts.
Right-clicking a component, net node, or wire opens the common sketch action
menu for that target. Component menus can inspect/select, start wire mode from
an existing/default pin, add current or power probes, or delete the component.
Net and wire menus can inspect/select the underlying net, connect the active
wire to that net, add a voltage probe, or delete the net through the existing
net-removal rules; a wire is still just a rendered view of Board IR pin-to-net
bindings, not a separate persisted edge model.
Removing a hovered badge deletes the underlying Board IR analog probe and any
analog assertions that reference it, then re-parses the edited Board IR before
updating the canvas. The Simulation stage mirrors the selected badge context in
a compact assertion table that shows each assertion name, aggregation, relation,
threshold, timing, latest status, and matching failure message when one exists.
Each row can be loaded into the structured assertion editor for name,
threshold, aggregation, relation, or timing changes, or deleted without removing
the probe or sibling assertions.
