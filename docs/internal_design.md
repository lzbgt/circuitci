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
sketch/model review, library suggestions, simulation artifact observation, and
report viewing. Schematic-canvas editing and waveform plotting should still
persist through Board IR, importer metadata, generated SPICE decks, and report
artifacts so headless agents can reproduce GUI actions.

GUI implementation is split so the stage shell does not accumulate all desktop
logic in one source file. `src/gui.rs` owns application state, menus, stage
routing, and validation/report calls. `src/gui/sketch.rs`
owns Board IR graph snapshots, graph layout/drawing helpers, and structured
scalar YAML edit helpers for existing components and nets. It may add or remove
component entries and may remove only nets that are not referenced by component
pins. It may assign or remove component pin bindings only when the component
exists and any assigned target net exists. Visual wire routing should keep using
these Board IR mutation helpers rather than introducing a parallel connection
model. `src/gui/simulation.rs` owns the Simulation stage UI, waveform CSV
parsing, plotting, cursor readouts, and min/max/delta measurements.
It may display graph hover readouts for runtime waveform probes, but those
values must come from report waveform artifacts and the shared waveform
interpolation helpers rather than an unsynchronized live simulation model.
`src/gui/library.rs` owns active-library model browsing, model filtering, and
component model assignment through the same validated Board IR YAML mutation
helpers used by the sketch inspector.
`src/gui/analog.rs` owns generated-from-Board analog transient scenario creation
and structured sample/min/max assertion authoring. It may derive node and pin
bindings from Board IR for observation scenarios, but file-backed SPICE deck
editing and advanced waveform math/channel analysis should be separate focused
modules so waveform sign-off semantics stay explicit.

## Evidence Model

Board IR is the only data model consumed by validation. Importers may add or
enrich these evidence families:

- component graph: `board.components`, `board.nets`, and component
  `source` metadata;
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
