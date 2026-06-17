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
  -> Board IR loading
  -> component model binding
  -> scenario suggestions
  -> validation/report engine
  -> waveform/artifact/report observation
```

Normal `cargo build --release` and CI validation builds do not compile GUI
dependencies unless `--features gui` is explicitly enabled.

## Workflow Shell

The first GUI slice provides EDA-style stages rather than a single command
form:

- Project: choose a Board IR project and output directory, then load or
  validate it.
- Sketch: shows the imported/sketched board graph summary. Canvas editing will
  build on this stage.
- Library: shows library bindings and scenario suggestion YAML.
- Simulation: runs validation through the engine, plots emitted CSV waveforms,
  and lists generated SPICE decks, artifacts, findings, and limitations.
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

1. load or import board evidence,
2. bind sourced component models,
3. run declared validation and `analog_transient` scenarios,
4. observe generated decks, plotted CSV waveforms, findings, and report
   artifacts,
5. edit the project/model evidence and rerun.

Full schematic-canvas editing, advanced waveform cursors/measurements,
automatic arbitrary schematic-to-SPICE conversion, and vendor macromodel
acquisition are future GUI stages. They must reuse the existing Board IR,
importer, model, and validation contracts instead of creating a parallel EDA
model.
