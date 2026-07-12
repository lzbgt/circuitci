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
  -> src/gui/project_example_workflow_tests.rs (GUI example workflow tests only)
  -> src/gui/shell.rs
  -> src/gui/import_flow.rs
  -> src/gui/file_dialogs.rs
  -> src/gui/jobs.rs
  -> src/gui/kicad_symbol_library.rs
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
`src/gui/gui_core_tests.rs` and direct-open example workflow regressions split
into `src/gui/project_example_workflow_tests.rs`. For developer visual QA,
set `CIRCUITCI_GUI_OPEN_PROJECT=/path/to/project.yaml` before running
`cargo run --features gui --bin circuitci-gui`; the app opens the project
directly in the Sketch stage and immediately requests Fit All, avoiding manual
click automation in screenshot checks. Imported high-pin KiCad symbols keep
their pin connection coordinates, but Sketch spreads side labels into readable
lanes before Fit All bounds are computed so dense symbol blocks do not collapse
into a KiCad-like text cluster.
For headless visual QA, prefer
`cargo run --features gui --bin circuitci -- export-sketch-svg project.yaml -o out/sketch.svg`.
The command uses the same Sketch graph layout and Fit All bounds as the GUI,
then writes a deterministic SVG with schematic symbols, wires, metadata-backed
pin anchors, and visible block/IC/connector pin labels without depending on OS
screenshot permissions. Compact passive and source symbols keep pin metadata on
their anchors but omit default pin-name text so generated schematic snapshots
stay readable.
Sketch graph layout also derives compact schematic detail text from explicit
part numbers, SPICE primitive values, or the final model-id segment, so canvas
and SVG labels avoid long internal model paths.
`src/gui/shell.rs` owns the desktop shell chrome: menu bar, workflow overlay
bar, project overlay, status panel, permanent Sketch central canvas,
secondary overlay windows, Project landing view, Reports view, and
finding/limitation rendering. Sketch is the mandatory main workspace: Project,
Import, Library, Scopes/Simulation, and Reports open as disposable floating
overlays on demand instead of replacing the schematic canvas. `src/gui/import_flow.rs` owns
the Import stage UI and KiCad schematic, KiCad PCB, and SPICE deck import
command wiring.
`src/gui/file_dialogs.rs` owns native open/save/folder dialog integration for
project, import, and output path fields. The dialogs are compiled only with the
optional GUI feature and do not affect the default CLI build.
`src/gui/jobs.rs` owns GUI background jobs for validation, scenario
suggestions, KiCad/SPICE import actions, and bundle-install metadata repair.
Long-running work runs on a worker thread and reports back to the UI through a
channel; cancel requests set a shared worker flag, terminate external ngspice
validation processes where possible, stop scenario suggestions and KiCad/SPICE
import jobs at safe phase-boundary checkpoints, and still mark any late
in-flight result to be ignored when a worker returns. It also owns lightweight
progress events for the active job and the capped recent-job history used by
the status panel to show final outcome, elapsed time, diagnostics, and output
paths for completed, failed, stale, or canceled background actions. Supported
early-stop paths use a typed operation-canceled error so checkpoint cancellation
is shown as `canceled`, not as an ordinary failure. Bundle-install metadata
repair is intentionally non-destructive: the Scopes artifact panel button runs
the same `repair-yaml --finding bundle-install-package-metadata` flow as the
CLI, writes a copied repaired project under
`<validation-output>/repair_bundle_import/<install-report-stem>/`, and records
the repaired project or repair report path in recent job history without
changing the loaded project path. After completion the generated
`repair_report.json` is attached to the loaded validation report artifacts, so
the Scopes artifact panel plus the current `report.json`/`report.md` show a
`YAML Repairs` summary with applied/blocked counts, repaired-project paths, and
proof status.
`src/gui/project.rs` owns project summary/YAML
load, save, parse validation, import path/name helpers, shared Board IR
undo/redo history, and the unsaved-change confirmation guard used before
load/import/quit actions. `src/gui/project_examples.rs` owns the scope-ready
GUI example catalog, expected trace/frequency copy, and workflow metadata used
by Project/Sketch example actions. `src/gui/sketch.rs` owns the Board IR graph snapshot,
sketch data types, persisted schematic node positions/styles, schematic wire
route waypoint metadata, shared sketch YAML helpers, and model-port default
pin/net seeding for library-backed component insertion. `src/gui/sketch_layout.rs`
owns graph layout helpers, view-state transforms, schematic grid/snap helpers,
fit-all and fit-selection bounds, bounded full-list logical layout for pannable
imported designs, layered circuit-flow fallback placement for projects without
persisted schematic coordinates, orthogonal wire geometry, and wire hit-testing.
`src/gui/sketch_layout_pins.rs` owns model-aware pin-anchor layout primitives,
KiCad symbol pin projection, high-pin label lane spreading, and component/net
node sizing. The fallback follows a
classical schematic shape using the same graph-drawing primitives used by ELK,
Graphviz, and OGDF: source-seeded signal-flow ranks, explicit KiCad pin-anchor
ports, bounded barycentric ordering inside each rank to reduce sibling branch
crossings, power and ground rails, and orthogonal routes. Power nets stay near
the top rail, ground nets near the bottom rail, source components stay on the
left, series signal-path components/nets advance left-to-right by rank, and
shunts land vertically between signal and rail. Imported high-pin block and IC
fallback symbols scale their rectangle height from the visible pin count, while
simple KiCad device symbols stay compact and suppress default pin-name text, so
anchors do not collapse into a KiCad-like unreadable cluster before a user
saves explicit schematic coordinates. Source references are side-placed to stay
clear of vertical rail terminals, and base net labels share deterministic
collision lanes between the live canvas and SVG export. The Sketch
`Auto Layout` action
persists that same classical placement into `board.schematic.node_positions`
and writes standard two-terminal orientation metadata, including vertical
shunts to ground and horizontal signal-path parts. It also derives display-only
`board.schematic.wire_routes` waypoints from the post-layout pin anchors so
power/ground connections route toward rail-like lines and signal connections
route horizontally before vertical drops. It also positions placed probe
elements into non-overlapping lanes near their pin/wire targets so live readout
strips stay readable after Run; newly placed probes use the same collision-aware
lane planner immediately, without requiring a separate Auto Layout pass.
Focused classical-layout regressions live in `src/gui/sketch_layout_tests.rs`. For
installed or imported KiCad symbols,
matching pin anchors project from the symbol's own numbered pin lines, so wire
starts and hit targets line up with the rendered schematic symbol instead of a
generic component box. Pin anchors are colored from the connected Board IR net
kind and show compact pin/kind chips while hovered, selected,
connected-highlighted, or used as an active wire target; this is a canvas
affordance, not another connectivity model. `src/gui/sketch_routes.rs` owns orthogonal schematic wire-route
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
`src/gui/sketch_scope_activity.rs` owns the runtime Scope Activity floating
window, small reopen button, schematic-chip visibility checkbox, searchable loaded-trace browser,
Cursor A scrub control, per-trace previous/next edge stepping, cursor-sampled
value/time readouts, compact frequency/period readouts, bounded per-trace
sparklines, activity-snapshot status plus
visible-list snapshot capture, row-level and visible-list CSV/Markdown copy,
row-level and visible-list report bundle export/index-open, recent-bundle reopen/path-copy shortcuts, source-specific snapshot clearing for sample and frequency rows,
source-filtered Open Snapshots routing, and
direct Scopes trace-open,
compare pin/unpin/clear/named-save/load/delete, and Open Compare actions for loaded schematic targets.
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
`src/gui/sketch_symbols.rs` owns model/id-inferred symbol selection and
KiCad Device-style default symbol drawing used by sketch nodes. Common
two-terminal primitives use compact schematic footprints with opposite-end
terminal anchors instead of large component cards; nets render as lightweight
schematic labels/junction targets rather than green boxes. The checked local
KiCad libraries documented in `docs/research/kicad/default_gui_symbol_reference.md`
are the canonical visual source. `src/gui/kicad_symbol_library.rs` discovers
installed KiCad symbol directories, parses and caches the needed `.kicad_sym`
drawing primitives and numbered pin-line anchors, and leaves deterministic
built-in fallback geometry for machines without KiCad.
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
the runtime-only Scope Activity floating window and searchable loaded-trace jump
browser with schematic-chip visibility control, Cursor A scrubbing, per-trace edge stepping, cursor-sampled
value/time readouts, row-level/visible-list report bundle export/index-open, and recent-bundle reopen/path-copy shortcuts; those chips and tints mark loaded waveform trace targets
and are not schematic components, pins, or nets.
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
probe insertion for those same supported branches. Probe insertion writes both
the analog scenario probe and a display-only schematic probe element binding in
one validated YAML edit.
`src/gui/sketch_selection_inspector.rs` owns the multi-selection inspector
summary, on-canvas selection frame/move handle, quick toolbar, and quick
actions for fitting, clearing, nudge, align, distribute, orientation changes,
copy, duplicate, and delete; these actions reuse existing canvas selection and
validated Board IR edit paths.
`src/gui/sketch_probes.rs` owns schematic voltage/current/power probe element
targeting, `board.schematic.probe_elements` upserts, badge layout, badge
hit-testing, badge drawing, and manual probe-element position edits.
`src/gui/library.rs` owns installed/imported KiCad symbol browsing plus
component model browsing over the active project library set. The KiCad symbol
browser scans installed `.kicad_sym` files, accepts user-imported
`.kicad_sym` files, filters by library/symbol/pin metadata, can apply a KiCad
`Library:Symbol` id to a selected component, and can insert a generic
schematic component with default pins derived from KiCad pin numbers while
persisting the display glyph under `board.schematic.component_symbols`. When
the component pin IDs match the KiCad symbol pin numbers, Sketch routes from
the symbol's real pin locations. The
model browser remains the source of Board IR model semantics: it filters
component models, stages selected models, assigns models to selected
components, and inserts model-backed components through the same Board IR YAML
mutation path. Models that declare `simulation.spice` metadata are marked with
a `SPICE` readiness column and selected-model details show the generated model
type, model name, required model file, provenance, pin order, and first
valid-operating note so users can distinguish simulation-observation parts from
validation-only library models before placing them. For a selected placed
component with a SPICE-ready model, the browser can create a generated
observation preset: it infers the ground net, creates a generated-from-board
run setup, voltage-probes the component's non-ground pin nets in SPICE pin
order, reuses the generated model-file inference path for SHA-pinned
`analog.model_files`, and adds model-aware default checks when metadata supports
them. When two modeled pins intentionally share one Board IR net, such as
nRF52840 `VDD` and `VDDH`, the preset reuses one voltage probe but keeps
pin-qualified assertion names so both pin limits remain visible and
non-duplicated. The op-amp buffer and comparator scope examples expose the same generator
as a compact `Create Checks` workflow action, so users can regenerate useful
observations from the example schematic without navigating the full model
browser. Regulator presets check output min/max voltage from the output port
electrical limits; reset-supervisor presets add active/released output samples
when the monitored rail is driven by a detectable pulse source; op-amp presets
add follower tracking checks when feedback ties the inverting input to the
output and the non-inverting input is pulse-driven; comparator presets add
output-state checks when one input is pulse-driven and the other is a fixed
reference. Direct-open regulator and charger/power-path examples include
AMS1117, TPS54331, TPS62162, TPS63802, TPS61023, TPS2121, TPS2115A, MCP73831, BQ24075, and BQ25798
fixtures for generated-SPICE rail, charge-current, and load-current
observations. Direct-open USB/debug bridge examples include CH340C, CH340N,
CP2102N, FT232R, CH347, CMSIS-DAP SWD, STM32L431 boot/UART/SWD,
nRF52840 board, ESP32-WROOM-32E boot/UART, ESP32-S3-WROOM
boot/USB, LicheeRV-Nano-W, AT32F435 motion-core, and AT32M416 motor-control fixtures for
generated-SPICE rail, reset/strap, USB, UART, SWD, motion-enable, fault-IRQ, CAN,
RS-485, servo-enable, PWM, driver-interface, encoder, current-sense, and
output-state observations. The nRF52840 board fixture opens a generated-SPICE
VDD/VDDH/VBUS, reset, SWD, UART/GPIO idle, USB boundary, and antenna feed-state
observation. The
TXS0108E level-shifter fixture opens a
mixed-voltage generated-SPICE A-to-B observation with rail, OE, input, and
translated-output checks. The NL27WZ17 logic-buffer fixture opens a
generated-SPICE VCC, input-state, and non-inverted output-state observation
with explicit Board IR state parameters. The JST XH and JST VH connector
fixtures open generated-SPICE pass-through contact-drop observations with
explicit board-side and load-side connector pins. The TPD2EUSB30 USB ESD fixture opens a
normal-operation generated-SPICE D+/D- standoff observation with the
source-backed line-capacitance load. The PRTR5V0U2X USB ESD fixture opens a
rail-to-rail generated-SPICE VBUS/IO1/IO2 standoff observation with
source-backed IO/VCC capacitance loads. The ESD2CAN24-Q1 CAN ESD fixture opens
a generated-SPICE CANH/CANL standoff observation with the source-backed 3 pF
line-capacitance load. The TCAN3413 CAN transceiver fixture opens a
generated-SPICE VCC/VIO, TXD/STB, RXD, and CANH/CANL dominant line-state
observation with explicit Board IR state parameters. The DRV8323 gate-driver
fixture opens a generated-SPICE VM/DVDD/ENABLE, nFAULT/SDO, and SOA/SOB/SOC
current-sense observation. The PCA9685 PWM-driver fixture opens a
generated-SPICE VDD/OE, idle SCL/SDA, and low-load 50 Hz PWM output observation.
The ICM-42688-P IMU fixture opens a generated-SPICE VDD/VDDIO, SPI idle-line,
SDO, and INT1 line-state observation. The BME280 I2C sensor fixture opens a
generated-SPICE VDD/VDDIO, I2C pull-up, `CSB`, and `SDO` address-select
observation. The SHT31-DIS I2C sensor fixture opens a generated-SPICE VDD,
I2C pull-up, `ADDR`, `nRESET`, and `ALERT` line-state observation. The
W25Q64JV SPI flash fixture opens a generated-SPICE VCC and
standby SPI/QSPI line-bias observation. The ESDS552 RS-485 ESD fixture opens a
generated-SPICE
A/B standoff observation with the source-backed 11 pF maximum line-capacitance
load. The THVD1450 RS-485 transceiver fixture opens a generated-SPICE VCC,
DI/DE/RE_N, RO, and A/B line-state observation with explicit Board IR state
parameters.
Inserted model-backed components use the selected model's declared
ports to seed editable Board IR pin bindings and generated per-pin nets, and
Sketch-stage placement can target the current view center, an armed blank-canvas
click, a drag/drop release with live ghost and snap feedback, or the
blank-canvas context-menu pointer. `R` / `Shift+R` rotate armed component
placement ghosts before insertion, `F` flips them, and `Shift+F` cycles the
previewed pin side. The accepted placement persists that schematic-only
orientation under `board.schematic.node_styles`.
`src/gui/simulation.rs` owns the Observations/Scopes overlay shell: a runtime-first
oscilloscope workspace, model-run controls, side-dock orchestration, and
scope-run preparation.
`src/gui/simulation_editors.rs` owns the docked run-setup/model/source/check
editors. `src/gui/simulation_probe_assertions.rs` owns the selected-probe check
table and canvas-triggered assertion add/clear/quick-threshold actions.
`src/gui/analog_run_setup.rs` owns generated transient, AC/Bode,
DC operating-point, noise, and harmonic-balance
run-setup YAML creation, including model-file inference and generated SPICE
node/pin bindings. `src/gui/simulation_sweeps.rs`
owns the user-facing Run Input Sweeps panel. Sweep creation is
valid-by-construction: adding a new
sweep also adds its first SPICE `.param` input or generated component value
input plus a comma-separated value list, while extra parameters, component
values, and vendor model-library section lists can be added or removed without
editing YAML. Generated run setups show inferred load/source candidates such as
`RLOAD.value_ohm` and `VSUPPLY.dc_v` that fill the component-value sweep fields.
Model-section sweeps emit `.lib "path" section` cards during ngspice execution.
The same panel exposes one-click corner presets for supply, load, temperature,
model-selector, and RC-tolerance sweeps, all persisted as ordinary
`analog.sweeps` data. Monte Carlo sweeps can be created from generated
component-value candidates by choosing sample count, seed, nominal value, and
tolerance percent plus a uniform or normal distribution; extra sampled component
targets can be added or removed while keeping at least one target. The same
panel can set or clear minimum yield percent and P1/P5/P50/P95 margin criteria,
all serialized as ordinary `analog.sweeps[].monte_carlo` data.
The run-setup editor can create generated transient, AC/Bode, DC
operating-point, noise, or harmonic-balance observations. AC/Bode creation writes a normal `analog_ac`
scenario with start/stop frequency, points per decade, generated board
component inclusion, ground binding, and an initial voltage probe. DC creation
writes a normal `analog_dc` scenario with `analysis: {type: op}`, generated
board component inclusion, ground binding, and an initial voltage probe, so
bias observations can be authored from the GUI without a hand-authored SPICE
deck. Noise creation writes a normal `analog_noise` scenario with
`analysis: {type: noise}`, a selected output net, selected input source,
start/stop frequency, points per decade, output/input noise probes, generated
board component inclusion, ground binding, and model-file inference. Harmonic
balance creation writes a normal `analog_harmonic_balance` scenario with
`analysis: {type: hb}`, selected output probe net, fundamental frequency,
harmonic count, selected drive source, generated board component inclusion,
ground binding, and model-file inference.
Fourier creation writes a normal `analog_fourier` scenario with
`analysis: {type: fourier}`, transient stop/max-step timing, selected output
probe net, fundamental frequency, harmonic count, generated board component
inclusion, ground binding, and model-file inference.
DC sweep creation writes a normal `analog_dc_sweep` scenario with
`analysis: {type: dc_sweep}`, selected swept source, start/stop/step sweep
range, selected output probe net, generated board component inclusion, ground
binding, and model-file inference.
Transfer-function creation writes a normal `analog_transfer_function` scenario
with `analysis: {type: tf}`, selected output probe net, selected input source,
generated board component inclusion, ground binding, and model-file inference.
Pole-zero creation writes a normal `analog_pole_zero` scenario with
`analysis: {type: pz}`, selected output probe net, selected input source,
selected pole/zero extraction mode, generated board component inclusion, ground
binding, and model-file inference.
Sensitivity creation writes a normal `analog_sensitivity` scenario with
`analysis: {type: sens}`, selected output probe net, DC or AC sensitivity mode,
optional AC frequency sweep bounds, explicit parameter filters, generated board
component inclusion, ground binding, and model-file inference.
Distortion creation writes a normal `analog_distortion` scenario with
`analysis: {type: disto}`, selected output probe net, harmonic or
intermodulation mode, frequency sweep bounds, selected F1 source, optional F2
source plus F2/F1 ratio for intermodulation, generated board component
inclusion, ground binding, and model-file inference.
Measure creation writes a normal `analog_measure` scenario with
`analysis: {type: measure}`, transient or AC measure mode, selected output
probe net, simulation bounds, and one portable `measure_templates[]` entry for
bounded `avg`, `max`, `min`, or `rms` scalar extraction.
The Fourier Check editor writes `analysis.fourier_assertions[]` for harmonic
magnitude, normalized magnitude, phase, normalized phase, or THD-percent
limits, and Fourier report failure actions load the failed check back into that
editor.
The Sensitivity Check editor writes `analysis.sensitivity_assertions[]` for
real, imaginary, or magnitude limits by parameter and optional AC frequency, and
sensitivity report failure actions load the failed check back into that editor.
The Transfer Function Check editor writes
`analysis.transfer_function_assertions[]` for transfer gain, input resistance,
or output resistance limits, and `.TF` report failure actions load the failed
check back into that editor.
The Pole-Zero Check editor writes `analysis.pole_zero_assertions[]` for pole or
zero real-part, imaginary-part, or derived-frequency limits with optional root
index selection, and `.PZ` report failure actions load the failed check back
into that editor.
The Distortion Check editor writes `analysis.distortion_assertions[]` for
normalized distortion component max-magnitude limits, and distortion report
failure actions load the failed check back into that editor.
The DC Sweep Check editor writes `analysis.dc_sweep_assertions[]` for min,
max, mean, or sample limits on declared sweep probes, and DC sweep report
failure actions load the failed check back into that editor.
The Measure Check editor writes `measure_assertions[]` for declared
measurement scalar limits, and `.MEASURE` report failure actions load the
failed check back into that editor.
The HB Check editor writes `analysis.hb_assertions[]` for magnitude, phase,
real, or imaginary limits at a selected harmonic, and HB report failure actions
load the failed check back into that editor.
The check editor can author transient, AC/Bode, DC operating-point, and noise
assertions. AC checks expose frequency fields for gain, phase, or group delay
at a frequency, gain crossing-frequency limits, and threshold-only phase/gain
margin checks, then serialize normal `analog.assertions` entries with
`at_hz`, `frequency_limit_hz`, `threshold_db`, `threshold_deg`, or
`threshold_s`. DC checks
serialize `aggregation: operating_point` with probe-unit thresholds and no
time or frequency fields. Noise checks serialize density-at-frequency and
integrated-RMS assertions with the correct `threshold_v_per_sqrt_hz` or
`threshold_v` units. For AC/Bode run setups, the same editor offers Bode check
presets that append ordinary assertion rows for common low-pass, unity-gain,
and loop-stability observations. For DC run setups, it offers 3.3 V rail, 5 V
rail, and 2.5 V midpoint presets. For noise run setups, it offers output and
input-referred density/RMS presets. These checks automatically participate in
sweep worst-corner margin summaries, reports, and scope bundles.
`src/gui/simulation_forms.rs` owns shared Observations/Scopes form defaults,
run-setup/net/probe combo widgets, stimulus field loading, and status-color
helpers used by those docked editors. `src/gui/analog_overview.rs` owns the
read-only generated run-setup audit snapshot, readiness diagnostics, and quick
editor navigation actions shown before edit panels. When a completed run emits
analog sweep margin summaries, the same overview shows the selected run setup's
worst-corner assertion margins with the limiting sweep corner, parameter values,
component value inputs, measured value, limit, margin, and evaluated corner
count.
`src/gui/waveform.rs` owns Scopes state orchestration,
simulation-time scrub/playback controls, value-scale controls, cursor
measurement tools, selected-plus-pinned cursor readout table, cursor/visible-window region statistics with snapshot capture, actionable transient cursor-region, region-stat, trigger-event, and Scope Activity sample and frequency measurement snapshots with editable labels/notes, search/source filters for Scope Activity samples and frequency rows, sort/group controls, plot markers, and filtered CSV/Markdown copy/export, GUI-only
transient trace pinning/comparison overlays, per-trace overlay visibility/color
styles, derived waveform channels, promotion of representable derived channels
to Board IR probes/assertions, and exact probe-value lookup for badge quick
assertions. Validation workers parse waveform artifacts with
bounded preflight size/row estimates, large-artifact progress warnings, optional
large-artifact deferral, progress/cancel checks, and loaded/deferred/skipped file
diagnostics before the GUI applies the completed report, keeping large CSV files
out of the UI thread while making missing, deferred, and slow artifacts
filterable, actionable, and exportable from Scopes with compact preview-column loaded/unloaded summaries and preview-load-state filters. Deferred artifacts keep header-only
trace previews, can be filtered by file/probe/detail from the selector, and can
be force-loaded individually, all visible matches, or all deferred files through
the same background waveform loader without changing Board IR or the validation
report. Matching-column, remaining-preview-column, and searchable exact preview-column picker loads append selected traces, mark loaded preview labels, skip already loaded columns, and preserve the full
deferred artifact placeholder for later all-column loading. Loaded full artifacts and selected-column loads can be inspected through footprint readouts with compact source memory totals that can be copied as CSV or Markdown, classified/grouped/filtered as full CSV, selected-column, or runtime-only views, sorted/filtered by runtime cost, copied/exported as visible-row CSV memory diagnostics, warned when the estimated f64 data footprint exceeds the runtime budget, and unloaded individually or through guarded visible-row/largest-first preview/confirmation from the runtime Scopes state to free memory; full loads become deferred reload placeholders again, and selected-column loads mark those preview columns unloaded without changing Board IR or reports. The Simulation report panel also surfaces retained distortion summary rows from `distortion_summaries[]`, Fourier harmonic/THD rows from `fourier_summaries[]`, harmonic-balance spectrum rows from `hb_summaries[]`, pole-zero root rows from `pole_zero_summaries[]`, sensitivity rows from `sensitivity_summaries[]`, transfer-function scalar rows from `transfer_function_summaries[]`, S-parameter RF sign-off rows from `s_parameter_summaries[]`, S-parameter two-port network quality rows from `s_parameter_network_summaries[]`, S-parameter RF noise rows from `s_parameter_noise_summaries[]`, compact-model package verification summaries from `model_package_conformance_checks[]`, including check name, analysis, solver, target artifact hash, source package-verification report, and evidence artifact paths. It also surfaces retained model-package bundle verification, install, and import summaries from `model_package_bundle_verifications[]`, `model_package_bundle_installs[]`, and `model_package_bundle_imports[]`, including bundle hashes, copied artifact counts, conformance counts, finding counts, installed registry hashes, scenario-ready registry/lock/artifact pins, repaired project paths, and the generated `repair-yaml --finding bundle-install-package-metadata` command when available.
The GUI Run Setup editor can author generated-from-board two-port
`analog_sparameter` scenarios with selected port nets, sweep bounds, and
reference impedance. It also authors generated-from-board PSS, phase-noise,
and PAC/PXF planning scenarios for the existing fail-closed periodic-analysis
contracts without enabling those backends to pass sign-off. The Scopes side dock includes dedicated RF Port Check, RF
Network Check, and RF Noise Check editors for `analog_sparameter` scenarios; they write
`analysis.s_parameter_assertions[]` entries for magnitude, return loss,
insertion loss, VSWR, mismatch-loss, group-delay, and reflection-impedance limits, plus two-port
`analysis.s_parameter_network_assertions[]` entries for reciprocity,
passivity, Rollet K, stability `|Delta|`, MAG, MSG, unilateral-gain, and
source/load-dependent gain limits, plus RF-noise `analysis.s_parameter_noise_assertions[]`
entries for NF, NFmin, Rn, and `|SOpt|` limits. The RF Network Check editor can also write
required `s_parameter_source_reflection` and `s_parameter_load_reflection`
coefficient provenance for Gt/Ga/Gp checks, with YAML
validation before the edited project is accepted. The same Scopes artifact panel surfaces
retained S-parameter port/network/noise assertion failures with row-level actions
that hydrate the matching RF Port Check, RF Network Check, or RF Noise Check editor state from
the report metadata, including mismatch-loss, reflection-impedance,
MAG/MSG/unilateral-gain/source-load-gain, RF-noise metric, aggregation,
relation, threshold, and source/load reflection fields. The generic Findings
panel also renders `adapter_blocker` and `evidence_sources[]` rows when a
fail-closed planned backend finding carries retained blocker provenance.
`src/gui/waveform/waveform_io.rs` owns streaming, cancel-aware waveform CSV parsing, report/path/request loading, and selected-column waveform requests used by deferred artifact loads. It recognizes `bode.csv`, `s_parameters.csv`, `noise_spectrum.csv`, `distortion_spectrum.csv`, `fourier_summary.csv`, AC `sensitivity_summary.csv`, and RF `s_parameter_noise_raw.csv` frequency-domain artifacts, converts them into frequency-axis Scopes artifacts, maps AC and S-parameter magnitude/phase/linear columns into unit-aware trace labels, derives group-delay traces from unwrapped phase columns, maps output/input noise-density columns into `V/sqrt(Hz)` labels, maps RF SP-noise raw columns into NF/NFmin/Rn/`|SOpt|` traces, maps distortion components into magnitude/phase/real/imaginary traces, maps Fourier harmonics into magnitude/phase plus normalized magnitude/phase traces, and maps sensitivity parameters into magnitude/real/imaginary traces so frequency-domain output reuses the same plot, compare, and bundle pipeline as transient waveform CSVs. `src/gui/waveform/waveform_sparameters.rs` owns derived S-parameter traces, including return-loss, insertion-loss, VSWR, mismatch-loss, reflection impedance real/imaginary/magnitude, two-port reciprocity-error, passivity singular-value, stability `|Delta|`, Rollet K, MAG, MSG, unilateral-gain, and source/load-dependent transducer/available/operating gain when source/load reflection metadata is present; the waveform loader treats `reference_impedance_ohm`, `source_reflection_*`, and `load_reflection_*` as retained metadata columns rather than plotted raw probes. `src/gui/waveform/waveform_monte_carlo.rs` owns the Scopes Monte Carlo yield table with scenario, sweep, limiting sample, check, probe, pass state, yield percent, pass/fail sample counts, mean/sigma/worst and percentile margins, compact min/max/P5-P95/median/zero-margin distribution strips, input summary, and CSV/Markdown copy actions. `src/gui/waveform/waveform_operating_point.rs` owns DC `operating_point.csv` artifact loading and the compact Scopes table with scenario, sweep, corner, probe, value, worst-corner marking, artifact label, and Copy CSV action. `src/gui/waveform/waveform_noise.rs` owns `noise_total.csv` artifact loading and the compact Scopes table with scenario, sweep, corner, output/input integrated RMS noise, output/input worst-corner marking, artifact label, CSV/Markdown copy actions, and noise-only report-bundle export. `src/gui/waveform/waveform_load.rs` owns bounded CSV preflight estimates, header-only trace previews, selected-column diagnostic merging that marks loaded preview labels, skips duplicate selected-column reloads, preserves full deferred placeholders until full load, and converts unloaded full artifacts back into deferred diagnostics. `src/gui/waveform/waveform_load_diagnostics.rs` owns filterable/copyable transient waveform-load diagnostics for loaded/deferred/skipped CSV artifacts, including preview-column loaded/unloaded audit metadata, preview-load-state filtering, row-level selected-column load shortcuts, exact preview-column picking, and runtime unload controls for loaded rows.
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
lookup. `src/gui/waveform/waveform_runtime.rs` owns runtime probe matching
between loaded waveform artifacts and Sketch selections, graph-hover readout
lines, normalized activity values for graph tinting, exact Scope Activity
sample rows, compact frequency/period readouts, bounded Scope Activity
sparkline samples, per-trace edge stepping, and row-level sample and frequency snapshot capture plus row-level and visible-list CSV/Markdown copy for schematic-side observation.
`src/gui/waveform/waveform_plot.rs`
owns the primary scope plot drawing, draggable/click-set A/B cursor handles,
direct plot drag/wheel/Shift-wheel interactions, Alt/Option-drag box zoom,
trace overlay selection, min/max decimated trace-point caching for large CSVs,
transient measurement snapshot marker chips with hover and click actions, and
shared-axis or per-unit lane axis scaling. It filters selected/pinned overlays
by x-axis kind so time-domain waveform traces and frequency-domain Bode traces
do not share an invalid cursor scale.
`src/gui/waveform/waveform_export.rs` owns deterministic runtime SVG rendering
for the current Scopes plot, including visible traces, split-unit lanes,
cursors, trigger markers, snapshot chips, and bounded decimated trace polylines
for copy/export workflows. Export options stay transient and cover report-size
presets plus independent cursor, trigger, and snapshot annotation inclusion.
`src/gui/waveform/waveform_view.rs` owns the Scopes plot orchestration, cursor
readout table, playback controls, visible time/frequency-window and
value-window fit/zoom/pan helpers, Back/Forward view-window history, scope plot
SVG copy/export actions, and measurement snapshot display.
`src/gui/waveform/waveform_snapshots.rs` owns transient cursor-region,
region-stat, trigger-event, and Scope Activity sample and frequency measurement snapshot
capture, editable labels
and notes, search/source filtering that includes Scope Activity samples and frequency rows, sort/group projection, plot-marker
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
snapshot measurement coverage, `src/gui/waveform/waveform_scope_activity_tests.rs`
for Sketch-side Scope Activity observation/report coverage,
`src/gui/waveform/waveform_bundle_tests.rs` for scope report bundle filesystem
and integrity coverage, and `src/gui/waveform/waveform_scope_tests.rs` for
app-level Scopes context, lane, probe, and runtime coverage, with
`src/gui/waveform/waveform_scope_compare_tests.rs` covering compare pins,
trace styles, and saved compare-set coverage so interaction work can grow
without turning the runtime module into a test fixture container.
`src/gui/analog_models.rs` owns SHA-backed analog `model_files` listing,
selection, add, hash computation, and remove mutations for declared analog
scenarios. Generated run-setup creation and generated component inclusion use
`src/gui/analog_model_files.rs` to infer required model files from active
component-library `simulation.spice.model_path` metadata, resolve paths from
the project directory and its ancestors, and add missing SHA-pinned entries
without requiring manual path/hash entry.
`src/gui/analog.rs` owns structured analog transient scenario and assertion YAML
generation for generated-from-Board simulations, with assertion display and YAML
field-name mapping split into `src/gui/analog_assertion_fields.rs` and focused
regression coverage split into `src/gui/analog_tests.rs`.
`src/gui/analog_sweeps.rs` owns analog input-sweep YAML mutation, with focused
sweep regressions split into `src/gui/analog_sweeps_tests.rs`.
`src/gui/analog_generated.rs` owns generated scenario analysis settings, ground/node mapping, component
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
  using native file/folder pickers, then load or validate it. The File menu has
  a registry-backed `Examples` command menu, and the Project overlay has a
  compact example picker that shows each entry's category, purpose, expected
  traces or observations, and expected frequency or operating point before
  `Open` or `Run + Scopes`. The current entries cover the checked-in NE555
  astable-style fixture, RC low-pass sine fixture, comparator threshold
  fixture, op-amp buffer fixture, CH340C USB-UART bridge fixture, CH340N
  USB-UART bridge fixture, CP2102N USB-UART bridge fixture, FT232R USB-UART bridge fixture, CH347 USB-JTAG
  debug bridge fixture, CMSIS-DAP SWD probe fixture, STM32L431 boot/UART/SWD fixture, nRF52840 board fixture, ESP32-WROOM-32E boot/UART
  fixture, ESP32-S3-WROOM boot/USB fixture, LicheeRV-Nano-W module fixture, AT32F435 motion-core fixture, AT32M416 motor-control fixture, TXS0108E level-shifter
  fixture, NL27WZ17 logic-buffer fixture, TPD2EUSB30 USB ESD fixture, PRTR5V0U2X USB ESD fixture, ESD2CAN24-Q1 CAN ESD fixture, TCAN3413 CAN transceiver fixture, DRV8323 gate-driver fixture, PCA9685 PWM-driver fixture, ICM-42688-P IMU fixture, BME280 I2C sensor fixture, SHT31-DIS I2C sensor fixture, W25Q64JV SPI flash fixture, ESDS552 RS-485 ESD fixture, THVD1450 RS-485 transceiver fixture, AP2112K LDO rail fixture, AMS1117 LDO rail fixture, TPS54331 buck rail fixture,
  TPS62162 buck rail fixture,
  TPS63802 buck-boost rail fixture, TPS61023 boost rail fixture, TPS2121
  power-mux fixture, TPS2115A power-mux fixture, TPS22918 load-switch fixture, TPS25948 eFuse fixture, TPS24751 hot-swap fixture, MCP73831 charger fixture, BQ24075
  power-path charger fixture, BQ25798 NVDC charger fixture, TLV803
  reset-supervisor fixture, loop-stability Bode fixture, DC divider-bias
  fixture, divider-noise fixture, and RC Monte Carlo yield fixture; all
  direct-open projects include display-only routed schematic waypoints. Opening
  one lands directly in Sketch with a deferred Fit All, so the first view is the
  readable connected network. When a scope-ready fixture
  is active, the Project overlay also shows
  a workflow status with direct `Run + Scopes` / `Open Scope Activity` actions,
  and the Sketch side dock mirrors those compact workflow actions above Run
  Readiness. Examples that contain a known SPICE-ready function block, such as
  the comparator threshold, op-amp buffer, CH340C USB-UART, CH340N USB-UART,
  CP2102N USB-UART, FT232R USB-UART, CH347 USB-JTAG, CMSIS-DAP SWD,
  STM32L431 boot/UART/SWD, nRF52840 board, ESP32-WROOM-32E boot/UART, ESP32-S3-WROOM boot/USB, LicheeRV-Nano-W module,
  AT32F435 motion core, AT32M416 motor control,
  TXS0108E level shifter, NL27WZ17 logic buffer,
  TPD2EUSB30 USB ESD, PRTR5V0U2X USB ESD, ESD2CAN24-Q1 CAN ESD, TCAN3413 CAN transceiver, DRV8323 gate driver, PCA9685 PWM driver, ICM-42688-P IMU, BME280 I2C sensor, SHT31-DIS I2C sensor, W25Q64JV SPI flash, ESDS552 RS-485 ESD, THVD1450 RS-485 transceiver, AP2112K LDO, AMS1117 LDO, TPS54331 buck, TPS62162 buck, TPS63802 buck-boost, TPS61023 boost, TPS2121 power mux, TPS2115A power mux,
  TPS22918 load-switch, TPS25948 eFuse, TPS24751 hot-swap, MCP73831 charger, BQ24075 power-path charger, and
  TLV803 reset fixtures, also show `Create Checks` to append a generated run
  setup with model-aware probes and observation checks for the placed component.
- Import: import native KiCad schematic evidence or SPICE decks into Board IR,
  or enrich an imported Board IR project with KiCad PCB placement/routing
  evidence. Import source and output paths can be typed or selected with native
  open/save dialogs. The SPICE import panel includes a scope-ready NE555
  astable preset that fills or directly imports the verified transient deck for
  Run + Scopes and Scope Activity inspection; the same fixture also has a
  checked-in `examples/ne555_astable_scope_smoke/project.yaml` for direct GUI
  opening without an import step.
- Sketch: is the always-visible main workspace and shows a visual Board IR graph with selectable component/net nodes,
  a schematic-first model-editor layout with a dominant canvas, compact Run
  control, secondary detail/navigation dock, and collapsed YAML editor,
  KiCad Device-style default rendering for resistors, capacitors, inductors,
  diodes, and sources, connector/IC/block rendering for larger parts, rendered
  component pin anchors, an inspector for component bindings and net connections, structured
  scalar edits, rename controls, inline canvas component ID/value editing,
  visible draggable component reference/value labels with transient visibility
  and auto-arranged display positions, a primitive palette that places generic
  resistors, capacitors, inductors, DC voltage/current sources, and pulse
  voltage/current sources at the current view, a canvas click, drag/drop release
  with orientation-aware snap ghost feedback, or a context-menu pointer with
  pins, nets, SPICE evidence, schematic placement, and optional schematic
  orientation, a Probe Elements palette that places voltage/current/power
  oscilloscope probes on schematic targets with runtime Scopes binding, and
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
  runtime tinting, a closeable floating Scope Activity window with a small reopen button and searchable
  loaded-trace jump browser, Cursor A scrub control, per-trace edge stepping,
  cursor-sampled value/time rows, compact frequency/period readouts,
  bounded per-trace sparklines, row-level and visible-list sample and frequency snapshot capture plus row-level and visible-list CSV/Markdown copy with a live
  activity-snapshot count, source-specific clearing, and direct Open Snapshots routing,
  trace compare pin/unpin/clear/named-save/load/delete actions, Open Compare jumps, hoverable/clickable `scope` activity chips, hover
  readouts, and context-menu Scopes jumps for matching loaded waveform probes,
  primary-toolbar probe controls that add voltage probes for selected nets or
  current/power probes for selected components,
  one-click `Run + Scopes` validation from the schematic toolbar,
  Auto Probes action and Auto-before-Run option for bounded
  voltage/source-current probe population before validation, Run Readiness
  preview rows for the probes Run will create,
  plus direct `Scope V`, `Scope I`, and `Scope P` actions that create the
  missing probe when needed and open the Observations-stage Scopes workspace,
  armed `Scope Tool` V/I/P placement buttons that consume a canvas click on a
  net, wire, pin, label, or component before normal selection/wire routing,
  with canvas-hover `V`/`I`/`P` shortcuts, valid/invalid hover target feedback
  before the click, and same-key/Esc cancellation,
  visible voltage/current/power probe elements derived from analog run-setup
  probes and rendered with KiCad meter/scope symbols when available, probe
  pass/fail/unknown/unasserted markers derived from the latest validation
  report, probe-element clicks that open and focus the corresponding
  Observations-stage scope/probe context, right-click probe-element action menus,
  right-click component/net/wire action menus, hovered-probe check
  creation/clearing, hovered-probe deletion that removes the probe and
  dependent checks through a validated Board IR edit, selected-net
  voltage-probe insertion into existing analog run setups,
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
- Observations/Scopes: opens as a floating overlay over Sketch and presents a runtime-first oscilloscope workspace linked
  from the schematic `Run`/`Scopes` controls, with a dominant plot, waveform
  selection, searchable/grouped trace selection, transient and AC/Bode selected-probe trace
  pinning, saved compare sets, and per-trace visibility/color styles for
  multi-trace comparison overlays, optional per-unit split lanes for mixed
  voltage/current/power or Bode magnitude/phase compares, Alt/Option-drag box zoom for shared-axis
  time/frequency/value windows or split-lane time/frequency windows, Back/Forward view-window
  history, direct
  plot drag time/frequency/value-window panning, wheel time/frequency zoom, Shift-wheel value zoom, explicit time/frequency-window and value-scale controls,
  draggable/click-set A/B cursor handles, selected-trace trigger threshold markers, exact event readout rows, previous/next or row-level edge jumps, trace/edge-to-schematic focus with context strip `Open Sketch`/`Fit Context` actions, play/scrub controls,
  selected-plus-pinned A/B cursor readouts, cursor/visible-window region statistics with min/max/mean/RMS rows and snapshot capture, current-plot SVG copy/export for reports, and searchable/source-filtered transient measurement snapshots from cursor regions, region stats, or trigger events with editable labels/notes, interactive plot marker chips plus row-level Jump, schematic Focus, filtered CSV/Markdown copy/export actions, and timestamped report bundles containing the configured plot SVG, filtered snapshot CSV/Markdown, local index page, and README manifest with loaded-waveform footprint source totals.
  DC operating-point runs show a compact table above the waveform selector
  when `operating_point.csv` artifacts are present; pure DC runs stay in
  Observations instead of falling through to Reports, and sweep margin
  summaries mark the limiting table rows directly. Noise runs show
  `noise_spectrum.csv` as frequency-axis output/input density traces and
  `noise_total.csv` as a compact integrated output/input RMS noise table with
  output/input limiting-corner markers plus CSV/Markdown copy and Export
  Bundle actions. If a schematic has no analog scope probes, `Run` prepares the Scopes workflow
  by adding a generated transient voltage probe on the default non-ground net
  before validation. The selected trace side dock also includes a bounded
  frequency-domain peak readout derived from the loaded transient waveform.
  Input and observation setup is secondary and docked: users can append a
  generated-from-Board `analog_transient` run setup with ground/probe net
  selection, audit generated run timing/backend,
  source/probe/check/model-file/node-binding coverage plus readiness gaps with
  quick editor navigation, edit generated run stop time/max step, ground net,
  SPICE node bindings, and component membership, add selected-net voltage probes
  to existing analog run setups, inspect a selected probe element's check rows
  with threshold/timing/status/failure details, edit or delete one check without
  clearing sibling checks on that selected probe, add or clear checks for that
  selected probe, quick-add cursor-sampled above/below checks from a
  hovered schematic probe element, add sample/window/timing/duty probe
  checks, edit
  file-backed SPICE decks declared by analog run setups, run validation through
  the engine, plot emitted transient waveform CSVs and AC `bode.csv`
  magnitude/phase/linear traces, add GUI-only derived
  difference/sum/product/ratio channels, promote representable derived channels
  to explicit analog probes or probes plus checks, pin the selected trace across
  all loaded transient or Bode sweep corners, pin report-identified worst-corner traces for the
  selected probe, and inspect generated SPICE decks, artifacts, findings, and
  limitations without turning the scope view into a form-first page.
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
6. drag component/net graph nodes or press Sketch `Auto Layout` to persist
   `board.schematic.node_positions`; Auto Layout also writes standard
   orientation metadata, display-only route waypoints for the generated
   textbook-style wiring, and display-only placed-probe positions,
7. snap dragged or auto-laid schematic positions and generated route waypoints
   to the visible grid when snap is enabled,
8. pan, zoom, Home-reset, Fit All, or Fit Selection the sketch viewport without
   changing Board IR evidence,
9. search components, net bundles, nets, wires, and probe elements in the object navigator,
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
32. add sample, windowed min/max/mean/RMS/integral/energy, rising/falling
   crossing-time, minimum high/low pulse-width, duty-cycle, crossing-count,
   settling-time, overshoot, two-probe phase-delay, or setup/hold waveform
   observation checks against declared probes,
33. author bounded run-input sweeps from the Observations overlay, including
   executable first-parameter or first-component-value creation, generated
   load/source candidate selection, supply/load/temperature/model-selector/RC
   corner presets, extra parameter/component/model-section add/remove controls,
   Monte Carlo sampled component-value creation/editing, Monte Carlo
   uniform/normal distribution selection, yield/percentile criteria set/clear
   controls, declared corner counts, and Board IR YAML persistence,
34. load, edit, save, and rerun file-backed SPICE decks from declared analog
   run setups,
35. browse, hash, add, and remove SHA-backed SPICE model/include files for
   declared analog scenarios,
36. run KiCad/SPICE imports, scenario suggestions, declared validation, and
   `analog_transient` scenarios in background workers while the desktop shell
   remains responsive,
37. cancel a running background job from the Observe menu, left project
   panel, or status panel; external ngspice validation subprocesses are
   terminated where possible, scenario suggestions and importers stop at safe
   phase checkpoints, and embedded backend calls still finish before their
   result is ignored; supported checkpoint stops are recorded as canceled job
   outcomes instead of failed job outcomes,
38. review recent background job outcomes in the status panel, including
   elapsed time, output path, and a compact diagnostic detail,
39. watch active background job stages in the status panel as imports,
   suggestions, validation, and analog simulation advance; KiCad/SPICE imports
   report parser, mapping/load, Board IR build/merge, and write phases, while
   validation reports project loading, model loading/binding, scenario
   execution, analog transient scenario/deck/backend/waveform phases, profile
   coverage, report assembly, report writing, and markdown report loading,
40. scrub or play the simulation time cursor to drive graph runtime tinting,
41. hover graph nodes to inspect matching voltage/current/power probe values at
   the current waveform cursor,
42. add schematic probes from the primary toolbar for the selected net or
   component, use visible schematic probe elements to find voltage/current/power
   probes, see latest check pass/fail/unknown/unasserted status, jump to
   their Observations-stage scope trace and selected-probe check panel after
   waveform data is loaded, add a check from the current check-editor settings
   with `A`, edit or delete one check row from the selected-probe panel,
   quick-add an above-current-sample check with Shift+A or a
   below-current-sample check with Shift+B, clear checks for the probe with
   `X`, use the right-click probe menu for the same probe actions, or remove a
   hovered probe element with Delete/Backspace,
43. use right-click component, net, and wire menus for common sketch actions
   such as inspect/select, start wire, connect an active wire, add voltage,
   current, or power probes, place local/off-page net labels, and delete
   through the same validated Board IR mutation paths as the inspector and
   keyboard actions,
44. observe generated decks, plotted CSV waveforms, derived waveform math
   channels, promote representable derived channels to persistent probes/assertions,
   cursor values, min/max measurements, findings, and report artifacts,
45. edit the project/model evidence and rerun.

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
selected Board IR net, then persists a matching schematic probe element under
`board.schematic.probe_elements`. The selected-component inspector appends a current probe
only for generated-from-Board analog scenarios and only when the component
branch is source-backed by a Board IR voltage/current source primitive, by a
Board IR resistor/capacitor/inductor primitive with a generated zero-volt
current-sense source, or by a bound diode/BJT/MOSFET model branch with
CircuitCI's generated zero-volt current-sense source. It appends power probes
for the same supported component set by composing explicit branch voltage and
branch current expressions, then persists matching schematic probe elements for
the selected component. Subcircuit internals and file-backed deck branch probes
still require explicit deck/model evidence.

Schematic probe elements are first-class display metadata over existing analog
scenario probes. The analog probe remains the runtime source of truth for SPICE
expressions, quantities, waveform columns, assertions, and reports;
`board.schematic.probe_elements` stores the placed schematic element ID, target
net/component, attachment kind (`node`, `pin`, or `wire`), and optional
component-pin source when placement came from a pin or routed wire. Pin and
wire placements render from actual pin/wire geometry; creation writes a
collision-aware default display-only `x`/`y` lane near the target, and dragging
a probe element updates those coordinates so the probe behaves like a placed
EDA symbol without changing solver connectivity. After simulation, a loaded
matching waveform paints a compact strip beside the probe with the current
Cursor A sample, optional frequency/period readout, and a small
sparkline; the strip is non-mutating and does not replace Scopes or measurement
snapshots. The strip is part of probe hit-testing and fit bounds, so users can
click, right-click, or start dragging from the displayed waveform area without
returning to Scopes. Voltage
probe elements attach to Board IR nets only when the probe expression's SPICE
node maps back through `analog.node_bindings`; current and power elements attach
to components only when the expression references a generated/source branch
that CircuitCI can map back to a Board IR component. Probe status markers are derived from the latest
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
Clicking a probe element, choosing a probe row in the object navigator, using
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
Export Bundle writes the same filtered snapshot CSV and Markdown, loaded DC
operating-point CSV/Markdown, loaded noise-total CSV/Markdown, sweep
worst-corner margin CSV/Markdown and Monte Carlo yield CSV/Markdown, including
sampled-margin percentiles, from the loaded validation report, the
configured or scalar-observation placeholder plot SVG, a local `index.html`
that preserves Monte Carlo min/max, P5-P95, median, and zero-margin
distribution strips, a README manifest, and `artifact_manifest.csv` plus
optional artifact integrity detail CSV/Markdown audit files into a timestamped output folder,
keeping report artifacts together while still avoiding persisted project truth.
Compare Sets also provide `Bundle Set` and `Bundle + Open`; these create
ephemeral selected/pinned cursor rows for the active compare view, then export
the same bundle format so nominal, all-corner, or worst-corner overlays can be
preserved with the limiting sweep-margin rows as design evidence without first
adding persistent measurement snapshot rows.
The index links the plot SVG, snapshot CSV/Markdown, DC operating-point
CSV/Markdown, sweep-margin CSV/Markdown, Monte Carlo yield CSV/Markdown,
README, manifest, and optional integrity detail files while surfacing the same
snapshot, DC bias, sweep-margin, Monte Carlo yield distribution, plot, selected-trace,
generated content artifact size/SHA-256 metadata, and loaded-waveform footprint
summary context for quick browser review. The README records active snapshot
filters, DC operating-point row counts, sweep-margin row counts, Monte Carlo
yield row counts, plot SVG options, selected trace context,
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
regenerates the plot SVG, snapshot CSV/Markdown, operating-point CSV/Markdown,
sweep-margin CSV/Markdown, index, README, and manifest from the current
filtered Scopes state into that same
`scope_report_bundle_*` folder. `Clean Old Bundles` previews older direct child directories named
`scope_report_bundle_*` under the configured output directory; `Confirm Cleanup`
then removes only those previewed bundle folders, preserving the newest bounded
set and unrelated output folders.
Right-clicking a probe element
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
analog checks that reference it, then re-parses the edited Board IR before
updating the canvas. The Observations stage mirrors the selected badge context in
a compact check table that shows each check name, aggregation, relation,
threshold, timing, latest status, and matching failure message when one exists.
Each row can be loaded into the structured check editor for name, threshold,
aggregation, relation, or timing changes, or deleted without removing the probe
or sibling checks.
