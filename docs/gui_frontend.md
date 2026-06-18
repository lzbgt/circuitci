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
  -> src/gui/sketch.rs
  -> src/gui/library.rs
  -> src/gui/simulation.rs
  -> src/gui/analog.rs
  -> Board IR loading
  -> component model binding
  -> scenario suggestions
  -> validation/report engine
  -> waveform/artifact/report observation
```

Normal `cargo build --release` and CI validation builds do not compile GUI
dependencies unless `--features gui` is explicitly enabled.

`src/gui.rs` owns the application shell, stage routing, and validation/report
calls. `src/gui/sketch.rs` owns the Board IR graph snapshot, sketch graph
layout/drawing helpers, and structured scalar YAML edit helpers for selected
components and nets, including conservative add/remove operations for
components and unreferenced nets, persisted schematic node positions, drag
updates, and validated component pin assignment. Visual wire creation reuses the
same pin-to-net mutation path instead of introducing a second connection model.
`src/gui/library.rs` owns component model browsing over the active project
library set, text filtering, selected-model staging, and selected-component
model assignment through the same Board IR YAML mutation path.
`src/gui/simulation.rs` owns the Simulation stage UI, waveform CSV parsing,
plotting, cursor measurement tools, and graph-hover runtime probe extraction
from loaded waveform artifacts. `src/gui/analog.rs` owns structured analog
transient scenario and assertion YAML generation for generated-from-Board
simulations.

## Workflow Shell

The first GUI slice provides EDA-style stages rather than a single command
form:

- Project: choose a Board IR project and output directory, then load or
  validate it.
- Import: import native KiCad schematic evidence or SPICE decks into Board IR,
  or enrich an imported Board IR project with KiCad PCB placement/routing
  evidence.
- Sketch: shows a visual Board IR graph with selectable component/net nodes,
  an inspector for component bindings and net connections, structured scalar
  edits for existing component and net properties, add/remove controls for
  components and unreferenced nets, draggable component/net node positions,
  pin-to-net assignment/removal for selected components, visual wire assignment
  by starting from a component pin and clicking a net node, graph-node hover
  readouts for matching runtime waveform probes, and a raw Board IR YAML editor
  with parse-validated save.
- Library: shows library bindings, searches the active component model set,
  stages a model for new components, assigns a selected model to the selected
  component, and shows scenario suggestion YAML.
- Simulation: can append a generated-from-Board `analog_transient` scenario
  with ground/probe net selection, add sample/min/max probe assertions, then
  runs validation through the engine, plots emitted CSV waveforms, provides A/B
  cursor measurements with min/max and delta values, and lists generated SPICE
  decks, artifacts, findings, and limitations.
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
6. assign or remove selected component pin bindings to existing nets,
7. create a visual wire by selecting a source component pin and clicking a net
   node,
8. search the active model libraries and assign selected models to components,
9. edit Board IR YAML evidence when the project needs a correction outside the
   structured controls,
10. append a generated-from-Board analog transient scenario with a voltage probe,
11. add sample or windowed min/max waveform assertions against declared probes,
12. bind sourced component models,
13. run declared validation and `analog_transient` scenarios,
14. hover graph nodes to inspect matching voltage/current/power probe values at
   the current waveform cursor,
15. observe generated decks, plotted CSV waveforms, cursor values, min/max
   measurements, findings, and report artifacts,
16. edit the project/model evidence and rerun.

Full symbol graphics, buses, hierarchical schematic sheets, advanced waveform
math channels, in-app file-backed SPICE deck authoring, automatic arbitrary
schematic-to-SPICE conversion, and vendor macromodel acquisition are future GUI
stages. They must reuse the existing Board IR, importer, model, and validation
contracts instead of creating a parallel EDA model.
