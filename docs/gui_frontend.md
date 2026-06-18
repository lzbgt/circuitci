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
  -> src/gui/project.rs
  -> src/gui/sketch.rs
  -> src/gui/sketch_actions.rs
  -> src/gui/sketch_symbols.rs
  -> src/gui/library.rs
  -> src/gui/simulation.rs
  -> src/gui/analog.rs
  -> src/gui/spice.rs
  -> Board IR loading
  -> component model binding
  -> scenario suggestions
  -> validation/report engine
  -> waveform/artifact/report observation
```

Normal `cargo build --release` and CI validation builds do not compile GUI
dependencies unless `--features gui` is explicitly enabled.

`src/gui.rs` owns the application shell, stage routing, import command wiring,
and validation/report calls. `src/gui/project.rs` owns project summary/YAML
load, save, parse validation, import path/name helpers, and shared Board IR
undo/redo history. `src/gui/sketch.rs` owns the Board IR graph snapshot, sketch
graph layout/drawing helpers, and structured scalar YAML edit helpers for
selected components and nets, including conservative add/remove operations for
components and unreferenced nets, persisted schematic node positions/styles,
drag updates, view-state pan/zoom transforms, fit-content bounds, Shift-drag
marquee selection, model-aware pin-anchor rendering, and validated component
pin assignment. `src/gui/sketch_symbols.rs` owns model/id-inferred common-class
symbol selection and the egui glyph drawing used by sketch nodes.
`src/gui/sketch_actions.rs` owns canvas selection state operations,
fit-content application, multi-selected drag/nudge/alignment, and batched
selected-item deletion as validated Board IR YAML edits. Visual wire creation
can start from a
rendered pin anchor and can terminate on another pin anchor or an existing net
node. Pin-to-pin wiring reuses an existing pin net when possible and otherwise
creates a generated Board IR net through the same mutation path instead of
introducing a second connection model.
`src/gui/library.rs` owns component model browsing over the active project
library set, text filtering, selected-model staging, selected-component model
assignment, and model-backed component insertion through the same Board IR YAML
mutation path. Inserted components use the selected model's declared ports to
seed editable Board IR pin bindings and generated per-pin nets.
`src/gui/simulation.rs` owns the Simulation stage UI, waveform CSV parsing,
plotting, simulation-time scrub/playback controls, cursor measurement tools,
and graph-hover/runtime activity extraction from loaded waveform artifacts.
`src/gui/analog.rs` owns structured analog transient scenario and assertion YAML
generation for generated-from-Board simulations. `src/gui/spice.rs` owns
file-backed SPICE deck discovery, loading, editing, saving, and save-and-run
actions for imported or hand-authored analog scenarios.

## Workflow Shell

The first GUI slice provides EDA-style stages rather than a single command
form:

- Project: choose a Board IR project and output directory, then load or
  validate it.
- Import: import native KiCad schematic evidence or SPICE decks into Board IR,
  or enrich an imported Board IR project with KiCad PCB placement/routing
  evidence.
- Sketch: shows a visual Board IR graph with selectable component/net nodes,
  common-class symbol-style rendering for resistors, capacitors, inductors,
  diodes, sources, connectors, ICs, and generic blocks, rendered component pin
  anchors, an inspector for component bindings and net connections, structured
  scalar edits for existing component and net properties, schematic-only
  rotate/flip/pin-side controls for selected components, add/remove controls
  for components and unreferenced nets, draggable component/net node positions,
  pan/zoom plus reset-view and
  fit-content controls, Shift-drag marquee selection, group drag/nudge/left-
  align/top-align controls for multi-selected sketch items, keyboard or button
  deletion for selected components/nets, batched deletion of multi-selected
  sketch items, pin-to-net assignment/removal for selected components, visual
  pin-to-pin and pin-to-net
  wire assignment by clicking component pin anchors or net nodes, graph-node
  runtime tinting and hover readouts for
  matching waveform probes, shared undo/redo for Board IR graph/property/wire/
  YAML edits, and a raw Board IR YAML editor with parse-validated save.
- Library: shows library bindings, searches the active component model set,
  stages a model for new components, inserts selected models as Board IR
  components with generated default pin nets, assigns a selected model to the
  selected component, and shows scenario suggestion YAML.
- Simulation: can append a generated-from-Board `analog_transient` scenario
  with ground/probe net selection, add sample/min/max probe assertions, edit
  file-backed SPICE decks declared by analog scenarios, then runs validation
  through the engine, plots emitted CSV waveforms, provides simulation-time
  scrub/playback, A/B cursor measurements with min/max and delta values, and
  lists generated SPICE decks, artifacts, findings, and limitations.
- Reports: displays the generated Markdown validation report.

The menu bar exposes File, Workflow, Simulation, and Help actions. These are the
hooks for later schematic editing, import flows, model inspection, and waveform
plotting without disturbing the CLI runtime.

## Simulation Boundary

The GUI does not make CircuitCI a full in-house analog simulator. CircuitCI
continues to use SPICE-class backends such as ngspice for nonlinear analog
solving and fails closed when required models, assertions, or backends are
missing.

The supported desktop simulation path is:

1. load Board IR or import KiCad schematic/PCB/SPICE evidence,
2. inspect the imported/sketched component and net graph,
3. edit selected component model/part-number and net kind/voltage/powered
   fields through structured controls,
4. add components, add nets, or remove selected components and unreferenced
   nets through validated graph controls,
5. drag component/net graph nodes to persist `board.schematic.node_positions`,
6. pan, zoom, reset, or fit the sketch viewport without changing Board IR
   evidence,
7. rotate, flip, or choose pin side for selected components through
   `board.schematic.node_styles`,
8. Shift-drag a marquee to select multiple visible components/nets,
9. drag, nudge, or align multi-selected sketch nodes as one validated Board IR
   edit,
10. assign or remove selected component pin bindings to existing nets,
11. create a visual wire by clicking a rendered source pin anchor and then a
   destination pin anchor or net node,
12. delete selected components or unreferenced nets from the canvas or toolbar,
13. undo or redo Board IR graph/property/wire/YAML edits through the shared
   editor history,
14. search the active model libraries, insert selected models as sketched
   components with generated pin nets, and assign selected models to existing
   components,
15. edit Board IR YAML evidence when the project needs a correction outside the
   structured controls,
16. append a generated-from-Board analog transient scenario with a voltage probe,
17. add sample or windowed min/max waveform assertions against declared probes,
18. load, edit, save, and rerun file-backed SPICE decks from declared analog
   scenarios,
19. bind sourced component models,
20. run declared validation and `analog_transient` scenarios,
21. scrub or play the simulation time cursor to drive graph runtime tinting,
22. hover graph nodes to inspect matching voltage/current/power probe values at
   the current waveform cursor,
23. observe generated decks, plotted CSV waveforms, cursor values, min/max
   measurements, findings, and report artifacts,
24. edit the project/model evidence and rerun.

Standards-complete symbol libraries and symbol editors, buses, hierarchical
schematic sheets, advanced waveform math channels, advanced SPICE source
tooling, automatic arbitrary
schematic-to-SPICE conversion, and vendor macromodel acquisition are future GUI
stages. Basic file-backed deck edits are supported, but must still reuse the
existing Board IR, importer, model, and validation contracts instead of creating
a parallel EDA model.

The sketch canvas symbol rendering is deliberately a view-layer affordance. It
infers a compact glyph from the component reference designator and model ID, then
continues to persist only Board IR components, nets, pins, and optional
`board.schematic.node_positions` / `board.schematic.node_styles`.
