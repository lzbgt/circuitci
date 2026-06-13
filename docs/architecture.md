# CircuitCI Architecture

CircuitCI is a headless board-assessment runtime for embedded electronics. It
normalizes design and fabrication artifacts into Board IR, binds component
models, runs explicit scenarios, and emits deterministic JSON/Markdown reports
with measured evidence and limits. The product boundary is pre-fabrication and
release-artifact validation; CircuitCI is not a schematic editor, PCB router, or
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
| `reports` | Convert findings into stable `report.json` and readable `report.md`. |
| `suite` | Run acceptance/public fixture suites against a built CLI. |
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

Manufacturing rules are static geometry and process screens. They currently
cover circular drills, routed slots, annular rings, castellated holes, copper
edge/spacing, solder-mask openings/dams, solder-paste openings/size/area ratio,
IC/BGA stencil aperture rows, and paste spacing. Shared geometry lives in
`validation::manufacturing::geometry`; larger rule families are split into
focused modules so source files stay below the 2000-line guard.

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

## Scenario Suggestions

`suggest-scenarios` converts evidence into candidate scenario YAML:

- runnable suggestions include enough parameters or process presets to execute
  immediately;
- non-runnable suggestions identify exactly which source-backed threshold or
  board fact is still missing;
- package-scoped stencil suggestions are inferred only from conservative
  owner-backed geometry patterns and discrete source-backed pitch rows.

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
