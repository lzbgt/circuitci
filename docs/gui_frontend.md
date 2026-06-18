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
components and unreferenced nets plus validated component pin assignment.
`src/gui/simulation.rs` owns the Simulation stage UI, waveform CSV parsing, and
plotting. `src/gui/analog.rs` owns structured analog transient scenario and
assertion YAML generation for generated-from-Board simulations.

## Workflow Shell

The first GUI slice provides EDA-style stages rather than a single command
form:

- Project: choose a Board IR project and output directory, then load or
  validate it.
- Import: import native KiCad schematic evidence into Board IR, or enrich an
  imported Board IR project with KiCad PCB placement/routing evidence.
- Sketch: shows a visual Board IR graph with selectable component/net nodes,
  an inspector for component bindings and net connections, structured scalar
  edits for existing component and net properties, add/remove controls for
  components and unreferenced nets, pin-to-net assignment/removal for selected
  components, and a raw Board IR YAML editor with parse-validated save.
- Library: shows library bindings and scenario suggestion YAML.
- Simulation: can append a generated-from-Board `analog_transient` scenario
  with ground/probe net selection, add sample/min/max probe assertions, then
  runs validation through the engine, plots emitted CSV waveforms, and lists
  generated SPICE decks, artifacts, findings, and limitations.
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

1. load Board IR or import KiCad schematic/PCB evidence,
2. inspect the imported/sketched component and net graph,
3. edit selected component model/part-number and net kind/voltage/powered
   fields through structured controls,
4. add components, add nets, or remove selected components and unreferenced
   nets through validated graph controls,
5. assign or remove selected component pin bindings to existing nets,
6. edit Board IR YAML evidence when the project needs a correction outside the
   structured controls,
7. append a generated-from-Board analog transient scenario with a voltage probe,
8. add sample or windowed min/max waveform assertions against declared probes,
9. bind sourced component models,
10. run declared validation and `analog_transient` scenarios,
11. observe generated decks, plotted CSV waveforms, findings, and report
   artifacts,
12. edit the project/model evidence and rerun.

Full schematic-canvas editing, visual wire routing, advanced waveform
cursors/measurements, arbitrary file-backed SPICE deck authoring, automatic
arbitrary schematic-to-SPICE conversion, and vendor macromodel acquisition are
future GUI stages. They must reuse the existing Board IR, importer, model, and
validation contracts instead of creating a parallel EDA model.
