# CircuitCI Architecture

CircuitCI is a board-assessment runtime for embedded electronics with a
headless CLI and an optional native Rust desktop frontend. It normalizes design
and fabrication artifacts into Board IR, binds component models, runs explicit
scenarios, and emits deterministic JSON/Markdown reports with measured evidence
and limits. The product boundary is pre-fabrication and release-artifact
validation; CircuitCI is not yet a full schematic editor, PCB router, or
replacement for RF/SI/thermal solvers.

## Runtime Flow

```text
source artifacts
  -> importers append Board IR evidence
  -> component libraries bind exact model IDs
  -> scenario suggestions propose missing checks
  -> validation dispatch runs selected rules
  -> reports serialize findings, measurements, limits, and fixes
```

Supported source paths include hand-authored Board IR YAML, SPICE decks, KiCad
XML/schematic/PCB artifacts, JLC/EasyEDA BOM+CPL assembly files, EasyEDA
flying-probe pad evidence, Gerber outline/copper/solder-mask/solder-paste
layers, Excellon drill and routed-slot files, and EasyEDA Pro `.eprj2` envelope
inspection. Importers are adapters into the same Board IR shape. Validation
rules do not branch on the original EDA source after import; they consume only
normalized board, layout, library, scenario, and process evidence.

## Module Map

| Module | Responsibility |
| --- | --- |
| `board_ir` | Deserialize project YAML into components, nets, layout evidence, manufacturing metadata, and scenarios. |
| `library` | Load component model packs, bind board components to exact `component_id` values, and emit binding findings. |
| `importers` | Convert external artifacts into Board IR while preserving provenance and failing closed on unsupported constructs. |
| `scenario_suggestions` | Inspect bound board evidence and propose runnable or non-runnable scenario YAML templates. |
| `validation` | Dispatch scenario checks and collect deterministic findings. |
| `validation::manufacturing` | Static fabrication/manufacturing rules over Gerber, Excellon, layout, and process-preset evidence. |
| `validation::interface_protection` | Static signal-protection, routed-interface, and topology-scoped bus termination rules. |
| `validation::motor_drive` | Static motor bridge budget, loss/thermal, regen-clamp, route-current, and current-sense rules over explicit current, shunt, connector, gate-timing, layout, and ADC evidence. |
| `validation::load_budget` | Static load-to-connector and load-to-cable current/voltage/thermal-rise/voltage-drop budget rules over explicit load, connector, and cable metadata. |
| `validation::model_quality` | Fabrication sign-off gates that require selected components to use source-backed component models with explicit confidence thresholds. |
| `reports` | Convert findings into stable `report.json` and readable `report.md`. |
| `suite` | Run acceptance/public fixture suites against a built CLI. |
| `gui` | Optional `egui`/`eframe` desktop shell for project loading, KiCad/SPICE import, staged review, validation, simulation artifact observation, and report viewing. `gui::project` owns project YAML load/save/parse actions, import path/name helpers, and the shared Board IR YAML undo/redo history used by graph, property, wire, and text edits. `gui::sketch` owns Board IR graph snapshots/layout, view-state pan/zoom transforms, fit-content bounds, rendered component pin anchors, schematic node position/style YAML mutations, structured scalar component/net edits, conservative component/unreferenced-net add/remove mutations, model-port default pin/net seeding, persisted schematic node positions, visual pin-to-pin and pin-to-net assignment, validated component pin assignment, and graph hover/runtime tint display. `gui::sketch_symbols` owns model/id-inferred common-class symbol selection and egui glyph drawing for sketch nodes. `gui::sketch_actions` owns canvas selection state, fit-content application, group drag/nudge/alignment, and selected-item deletion composition. `gui::library` owns active model browsing, filtering, selected-component model assignment, and model-backed component insertion. `gui::simulation` owns the Simulation stage UI, waveform CSV parsing, plotting, simulation-time scrub/playback, cursor readouts, min/max/delta measurements, runtime probe values for graph hovers, and normalized runtime activity for graph tinting. `gui::analog` owns generated-from-Board analog transient scenario creation and sample/min/max assertion authoring. `gui::spice` owns file-backed SPICE deck discovery, load/save, and save-and-run actions for analog scenarios. |
| `main` | Own the command-line interface and import/validate/suggest command wiring. |

## Core Contracts

| Type | Owner | Purpose |
| --- | --- | --- |
| `BoardProject` | `board_ir` | Project metadata, library paths, normalized board evidence, scenarios, and source directory. |
| `Board` | `board_ir` | Component/net graph plus layout and board-level manufacturing facts such as stencil thickness. |
| `ComponentLibrary` | `library` | Deterministic model map loaded from `*.model.yaml` files. |
| `BoundBoard` | `library` | Board plus resolved component models and model binding diagnostics. |
| `Scenario` | `board_ir` | User-authored validation intent: scenario type, checks, targets, parameters, events, and paths. |
| `ScenarioSuggestion` | `scenario_suggestions` | Agent-facing scenario template with confidence, runnability, and required inputs. |
| `Finding` | `reports` | Stable diagnostic payload containing rule ID, severity, measured evidence, limits, and suggested fixes. |
| `ValidationReport` | `reports` | Final pass/fail result and serialized finding set. |

Binding diagnostics such as `MODEL_NOT_FOUND` and `PIN_NOT_DECLARED` are report
findings. Rule implementations should rely on `BoundBoard` rather than
duplicating library binding checks.

Source-backed model equations and conservative board-state derivations, such as
charger PROG/ISET resistor current programming or exact-one-powered power-mux
source selection, live behind shared helpers. Validators and scenario
suggestions use those helpers before treating the check as runnable, and the
helpers return no value for ambiguous evidence.

## Importer Design

Importers append evidence instead of guessing missing intent. Examples:

- JLC/EasyEDA BOM+CPL import adds components and placements, but does not infer
  nets or pins from assembly files.
- Gerber copper import records flashes, circular-aperture draw segments, and
  single-contour regions. Copper is anonymous until existing pad, route, zone,
  or flying-probe evidence uniquely proves net/island/owner metadata.
- Gerber solder-mask and solder-paste importers use the same artwork evidence
  shape, with layer mapping to corresponding copper layers for owner matching.
- Excellon import records circular drill hits and `G85` routed slots, then adds
  pad/via owner metadata only when layout or copper evidence uniquely matches.
- EasyEDA Pro `.eprj2` inspection documents the SQLite envelope and encoded
  payload status; it does not fabricate pad/net geometry from encoded history.

Unsupported source constructs fail closed or are counted as ignored when they
cannot be represented without losing the engineering meaning. The importer
contract is evidence preservation, not optimistic reconstruction.

## Validation Design

Validation is scenario-driven. A scenario selects one or more check IDs, and
`validation::mod` dispatches each ID to a rule implementation. Rules must:

- require every measurement source they consume,
- emit `VALIDATION_INPUT_MISSING` when a required source or parameter is absent,
- report measured values and limit values with stable keys,
- preserve provenance fields such as source primitive indices, component pins,
  route/via indices, Gerber apertures, and Excellon tools,
- avoid changing thresholds to make examples pass.

Validation profiles annotate coverage but do not synthesize hidden scenarios.
For `iot_basic_v0`, report assembly adds a non-blocking
`PROFILE_COVERAGE_PARTIAL` limitation when the project does not declare the
core executable checks needed for full-profile sign-off. This keeps
`report.result` tied to critical findings while making partial coverage visible.

Manufacturing rules are static geometry and process screens. They currently
cover circular drills, routed slots, annular rings, castellated holes, copper
edge/spacing, solder-mask openings/dams, solder-paste openings/size/area ratio,
IC/BGA stencil aperture rows, and paste spacing. Shared geometry lives in
`validation::manufacturing::geometry`; larger rule families are split into
focused modules so source files stay below the 2000-line guard.

Motor-drive rules are static design-budget screens. `motor_drive` scenarios
require explicit supply-voltage, current, connector, shunt, gate-timing,
regeneration, and layout parameters, or a declared motor/load component model
with `motor_load` voltage and current evidence, so a robot actuator bridge can
fail closed on missing or undersized first-pass values before schematic
capture.
SOA, regeneration, route-current, and current-sense entry points live in
`validation::motor_drive`. Bridge supply/loss/switching screens live in
`validation::motor_drive_bridge`. Shared parameter parsing, motor-load
evidence, route/placement geometry, and common finding builders live in
`validation::motor_drive_common` so more motor checks do not grow one
monolithic source file.
`MOTOR_BRIDGE_LOSS_THERMAL_VALID` adds a source-backed reference-loss
thermal-budget screen from component `motor_bridge` metadata.
`MOTOR_LOAD_SUPPLY_VALID` checks the declared motor bus window against
selected motor supply-envelope evidence from scenario parameters or
`motor_load` metadata.
`MOTOR_BRIDGE_SWITCHING_VALID` adds a source-backed static transition-loss and
average gate-charge screen from bridge rise/fall and total gate-charge
metadata.
`MOTOR_BRIDGE_SOA_VALID` adds a fail-closed static SOA screen. It can consume
power-block system SOA curves from `motor_bridge.system_soa` as
current-versus-temperature limits, or fall back to classic
`datasheet.safe_operating_area.vds_id_curves` for VDS/ID pulse curves.
`MOTOR_REGEN_CLAMP_VALID` checks an explicitly declared single-event
regeneration envelope against bus capacitance and a named absorber/clamp
component. Absorber current and energy ratings can come from scenario
parameters for what-if studies or from the named component model's
`regen_absorber` metadata for selected-part evidence.
`MOTOR_CURRENT_SENSE_ACCURACY_VALID` checks declared shunt, gain, ADC, offset,
tolerance, and error budgets. These rules still do not imply FOC, true MOSFET
SOA, switching-waveform, PWM sampling behavior, peak gate-current behavior,
repeated-pulse
thermal, firmware regeneration-control, or PCB copper-temperature sign-off.

Load-budget rules are static connector/load screens. `load_budget` scenarios
target a load power pin and compare its declared `max_supply_current_A` against
an explicit connector rating or a connector component model with `connector`
metadata. `POWER_SWITCH_BUDGET_VALID` applies the same load evidence to a
selected `power_switch` component and checks output-current rating,
current-limit setting, pin voltage ratings, and a static conduction thermal
budget. `POWER_SWITCH_REVERSE_CURRENT_VALID` and
`POWER_SWITCH_INRUSH_VALID` add explicit selected-switch gates for e-stop
backfeed and capacitive turn-on evidence. `LOAD_CABLE_CURRENT_VALID` applies
the same current evidence to explicit cable ratings or `cable_assembly`
metadata.
`LOAD_CABLE_THERMAL_DERATING_VALID` estimates cable temperature rise by I^2
scaling from explicit harness test evidence. `LOAD_CABLE_VOLTAGE_DROP_VALID`
estimates DC harness voltage drop and power loss from explicit loop-resistance
evidence. These checks are useful for reusable robot payload and actuator
connectors before CAD capture; they do not prove crimp quality, vibration
retention, harness routing, bundle derating, PWM ripple, or pulsed-load
behavior.

Validator entry points are split by ownership:
`validation::load_budget` owns connector and cable rules, while
`validation::load_budget_power_switch` owns selected switch budget,
reverse-current, and inrush rules. They share only small parsing/finding
helpers so adding more switch-specific checks does not grow the connector/cable
module.

Model-quality rules are explicit sign-off gates. `MODEL_QUALITY_REQUIRED` turns
selected component model provenance into critical findings when a fabrication
review must not rely on `generic`, `estimated`, or low-confidence envelopes.
The always-on `LOW_CONFIDENCE_MODEL` limitation remains non-blocking context;
the scenario decides which components are actually sign-off blockers.

Interface-protection rules cover clamp devices, level shifters, USB connector
mechanics/routing, explicit bus termination, and explicit bus protection
placement. `BUS_TERMINATION_VALID` is topology-scoped: it requires the scenario
to declare endpoint role, bus nets, expected resistance, tolerance, and a
specific resistor with `spice.value_ohm`. It intentionally does not infer that
every CAN/RS485 board should carry local termination.
`BUS_PROTECTION_PLACEMENT_VALID` consumes board layout placements and ordered
route polylines for both bus lines, then checks declared TVS or termination
components against project-specific distance limits.

Motor-drive route checks consume the same `board.layout.routes` evidence.
`MOTOR_ROUTE_CURRENT_VALID` requires scenarios to name routed motor/power nets,
select current evidence, and declare an explicit A/mm route-width policy. It
compares imported segment widths against that policy without claiming MOSFET
SOA, switching-loss, copper-temperature, or regeneration-transient sign-off.
`MOTOR_CURRENT_SENSE_PLACEMENT_VALID` consumes `board.layout.placements` plus
phase and sense routes to keep phase shunts near the bridge, phase copper, and
current-sense traces. It is a deterministic layout-distance guard; it does not
prove amplifier accuracy, Kelvin parasitics, thermal drift, or ADC noise.

## Process Presets

`parameters.fabrication_process` is a named source-backed default set for
numeric manufacturing limits. Scenario numeric parameters always override
process defaults. Presets may be combined as a list, but validation fails closed
if two presets provide conflicting defaults for the same parameter.

Presets are deliberately narrow. JLCPCB castellated-hole values are only exposed
through `CASTELLATED_HOLE_VALID`; they are not reused as generic drill-edge
clearance. JLC stencil table rows are package/pitch-scoped rules rather than
global paste-spacing or paste-area presets. Board-level process facts that are
not present in Gerbers, such as `board.manufacturing.stencil_thickness_mm`,
`board.manufacturing.min_drill_edge_clearance_mm`,
`board.manufacturing.min_slot_edge_clearance_mm`, and board/order-specific paste
coverage and paste-spacing limits, are stored as Board IR metadata and remain
explicit evidence.
Layout policy that is not a manufacturing process fact lives under
`board.layout.constraints`, for example explicit USB connector mechanical
limits and USB return-path budgets.

## Scenario Suggestions

`suggest-scenarios` converts evidence into candidate scenario YAML:

- runnable suggestions include enough parameters or process presets to execute
  immediately;
- non-runnable suggestions identify exactly which source-backed threshold or
  board fact is still missing;
- the report constructor and schema reject runnable suggestions with
  `required_inputs` and non-runnable suggestions without concrete
  `required_inputs`, so optional review notes cannot masquerade as missing
  executable inputs;
- package-scoped stencil suggestions are inferred only from conservative
  owner-backed geometry patterns and discrete source-backed pitch rows.
- reset-release timing suggestions become runnable only from explicit
  `board.runtime` evidence for the exact component/pin or from one unique
  datasheet-backed reset-supervisor model that monitors the target rail and
  drives the target reset net.
- runtime-state suggestions such as control-line release sequences and GPIO
  backdrive become runnable only from explicit `board.runtime` evidence for the
  exact sequence or endpoint pair.
- profile-aware suggestions are opt-in through `--profile`; for
  `iot_basic_v0`, missing core checks become non-runnable remediation templates
  unless ordinary evidence-driven suggestions already cover them.

The suggestion engine is not a hidden validator. It never silently adds
thresholds that are missing from the project, source documents, or Board IR.

## Solver Boundary

Most mature checks are deterministic static rules. `analog_transient` scenarios
can run file-backed or generated SPICE-class simulations through ngspice when
configured. Missing or unavailable simulation backends fail closed. MCU support
is modeled as externally observable pin behavior, reset/boot state, electrical
limits, and firmware-visible behavior; internal MCU transistor simulation is a
non-goal.

## Verification Strategy

The repo uses focused fixture tests for each rule/importer, schema sweeps for
example projects and reports, public fixture suites for release binaries,
behavioral/physical acceptance suites, clippy, formatting, diff checks, and a
source line-count guard. Real peer-board research notes under `docs/research/`
record imported `urine_monitor` evidence and distinguish runnable checks from
threshold-gated checks.

See [internal_design.md](internal_design.md) for implementation-level contracts
and rule/module ownership.
