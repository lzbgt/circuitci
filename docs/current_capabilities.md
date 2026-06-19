# Current Capabilities

Date: 2026-06-19

CircuitCI is primarily a board-validation runtime with both a headless CLI and
an optional native Rust desktop frontend. It imports board evidence, binds
component models, runs deterministic validation scenarios, and emits
machine-readable reports for agents and engineers before fabrication.

It is not yet a full schematic editor, PCB layout editor, full EDA suite, RF/SI
solver, or general-purpose analog simulator, though the optional GUI now has a
bounded schematic graph canvas for Board IR edits.

## Frontends

| Area | Current support | Boundary |
| --- | --- | --- |
| CLI | Default `circuitci` binary for import, suggestion, validation, suites, and report generation. | Primary automation surface for CI and agents. |
| Desktop GUI | Optional `circuitci-gui` Rust desktop app behind `--features gui`, with native open/save/folder pickers for project, import, and output paths, background KiCad/SPICE import, scenario suggestion, validation, and simulation execution with elapsed status, cancel-result handling plus external ngspice process termination and safe import/suggestion checkpoint cancellation on cancel, active stage-event display including KiCad/SPICE import parser/build/write phases and validation project/model/scenario/report plus analog transient scenario/deck/backend/waveform phases, and a capped recent-job history panel with outcome, elapsed time, output path, diagnostic detail, and distinct canceled outcomes for supported checkpoint stops, EDA-style stages for KiCad schematic/PCB import, SPICE deck import, project loading, schematic-first Sketch workspace with a dominant model canvas and direct Run/scope controls, runtime-first Scopes workspace with a dominant oscilloscope plot, optional large-waveform deferral with filterable header-only trace previews, background row/visible/all/matching-column and remaining-preview-column Load Deferred recovery with searchable exact preview-column picker actions plus select-visible helpers that mark loaded preview labels, skip duplicate selected-column reloads, and preserve full deferred placeholders after partial trace loads, and loaded/deferred/skipped artifact diagnostics with compact preview-column loaded/unloaded audit metadata, preview-load-state filtering, row-level preview-column load shortcuts, exact column picking, and runtime unload controls that free loaded waveform memory while keeping artifacts reloadable, loaded-waveform footprint readouts with sort/filter controls and guarded bulk unload preview/confirmation, waveform selection, searchable/grouped trace selection, transient pinned trace comparison overlays, saved compare sets, per-trace overlay visibility/color styles, optional per-unit split lanes for mixed-unit compares, direct plot drag time/value-window panning, Alt/Option-drag box zoom, Back/Forward view-window history, wheel time zoom, Shift-wheel value zoom, Fit Y/Y-zoom/Y-pan value scale controls, draggable/click-set A/B cursor handles, selected-trace trigger threshold controls with edge markers, exact event readout rows, previous/next or row-level edge jumps, trace/event-to-schematic cross-focus selection with a visible schematic-context strip and Open Sketch/Fit Context actions, selected-plus-pinned cursor readout rows, cursor/visible-window region statistics with min/max/mean/RMS rows and snapshot capture, searchable/source-filtered transient cursor-region, region-stat, and trigger-event measurement snapshots with editable labels/notes, interactive plot marker chips plus row-level Jump, schematic Focus, and filtered CSV/Markdown copy/export actions, play/scrub controls, and docked scenario/assertion/artifact setup, visual Board IR component/net graph inspection, common-class symbol-style rendering for resistors, capacitors, inductors, diodes, sources, connectors, ICs, and generic blocks, schematic-only rotate/flip/pin-side controls for selected components plus canvas `R` / `Shift+R` rotation, `F` flip, and `Shift+F` pin-side shortcuts for selected components or armed component placement, rendered component pin anchors with net-kind color and active pin/kind chips, draggable graph-node positions, primary-toolbar grid visibility, grid-step, and snap-mode controls, orthogonal wire visuals with net labels and junction dots plus direct schematic-only wire-route shaping by dragging rendered wires or visible route handles, active wire-mode blank-canvas bend clicks, and clearing custom routes from wire menus, derived net-bundle trunks/badges for bracketed, dot-qualified, CAN, I2C, USB, and RS-485-style scalar nets, derived schematic hierarchy sheet groups from imported KiCad source paths and importer namespace prefixes, hierarchy focus/isolate view filters, off-sheet connector badges for focused nets with external endpoints, clickable wire-to-net selection and inspection, right-click component/net/wire action menus, primary-toolbar probe controls for selected nets and components, visible voltage/current/power probe badges derived from analog scenario probes with latest-report assertion pass/fail/unknown/unasserted markers, badge clicks and navigator probe rows that select and focus the corresponding Simulation-stage scope/probe context, right-click probe-badge action menus, selected-probe assertion table with threshold/timing/status/failure details plus row-level edit/delete actions, hovered-badge assertion add/clear shortcuts, hovered-badge cursor-sampled quick above/below assertion shortcuts, hovered-badge Delete/Backspace removal with dependent assertion cleanup, multi-selected group drag, nudge, edge/center align, and distribute controls, blank-canvas drag, touchpad-scroll, and overview-minimap viewport panning, pointer-focused pinch/Cmd-scroll zoom, Home/Fit All/Fit Selection controls, searchable schematic hierarchy select/fit/focus/isolate controls and object navigator select/fit controls for sheet groups, components, net bundles, nets, wires, and probe badges, Shift-drag replace selection boxes, Cmd/Ctrl-drag additive selection boxes, Alt/Option-drag subtractive selection boxes, L-key freehand lasso variants for those same selection drag chords, multi-selection inspector summary/actions, on-canvas selected-group frame/move handle with snap/free target feedback, transient alignment guides with optional guide snapping, and quick toolbar with rotate/flip/pin-side actions, selected-component duplication and copy/paste with internally referenced local-net copies and shared external nets, multi-selected sketch-item deletion, keyboard/button delete for selected sketch items, pin-to-pin, pin-to-net, and pin-to-wire visual wire assignment by click or drag with multi-bend snap preview, target highlighting, and pin/kind target chips backed by Board IR net reuse/creation, selected-net voltage probe insertion into existing analog scenarios, selected-component current probe insertion for generated source branches plus generated passive and diode/BJT/MOSFET current-sense branches, selected-component power probe insertion for those same supported generated branches, graph-node runtime tinting and hover readouts for matching waveform probes, shared undo/redo for Board IR graph/property/wire/YAML edits, unsaved-change confirmation before load/import/quit replaces dirty Board IR YAML or file-backed SPICE deck edits, structured scalar editing, validated rename controls, primitive palette insertion for generic R/C/L and independent voltage/current source components at the current view, canvas click, drag/drop release with orientation-aware live ghost/snap/pin-side feedback, or context-menu pointer with generated pins, nets, SPICE evidence, schematic placement, and optional schematic orientation, and component-level SPICE primitive/value editing for existing component/net properties, add/remove controls for components and unreferenced nets, selected-component pin assignment/removal to existing nets, active-library model search, selected-component model assignment, model-backed component insertion/placement at the current view, canvas click, drag/drop release with orientation-aware live ghost/snap/pin-side feedback, or context-menu pointer with generated default pin nets and optional schematic orientation, generated-from-Board analog transient scenario creation, generated analog scenario overview/audit panels with readiness diagnostics and editor navigation actions, generated scenario analysis setting, ground/node binding, and component include/exclude editing with pin-binding repair, structured DC and pulse source stimulus editing for generated analog scenario source primitives, SHA-backed SPICE model/include file management for analog scenarios, structured sample/min/max analog assertion authoring, file-backed SPICE deck editing with save-and-run, Board IR YAML editing with parse-validated save, library suggestions, CSV waveform plotting, simulation-time scrub/playback, A/B cursor values, cursor/visible-window min/max/mean/RMS region statistics with transient snapshots, min/max and delta waveform measurements, GUI-only derived difference/sum/product/ratio waveform channels with promotion to explicit Board IR analog probes or probes plus assertions when quantities are representable, simulation artifact observation, and report viewing. | Workflow and observation shell; standards-complete symbol libraries, symbol editors, persisted bus primitives, persisted hierarchical sheet primitives/editing, advanced SPICE model management, subcircuit-internal current/power probes and advanced multi-channel persisted waveform-analysis sign-off are future stages. |

GUI scope usability note: the Sketch canvas opens with derived net-bundle boxes hidden by default, so the connected circuit network is primary; selected nets/components have direct `Scope V`, `Scope I`, and `Scope P` actions that create the missing Board IR probe when needed and open Scopes; the `Scope Tool` V/I/P buttons and canvas-hover `V`/`I`/`P` shortcuts let users arm voltage/current/power probe placement, see hover feedback for the target that will receive the probe, and click a net, wire, pin, label, or component directly on the canvas; pressing the same tool key again or Esc cancels the armed tool; Scopes `Run` can create a default transient voltage probe when no analog probes exist; selected traces include a bounded frequency-domain peak readout for quick time/frequency inspection.

The Sketch canvas also supports inline component ID editing and inline scalar
SPICE value editing for passives and DC sources; both route through the same
validated Board IR mutation helpers used by the inspector.

Visible component reference/value labels are rendered directly on the
schematic. Reference text derives from the component ID, value text derives
from scalar SPICE evidence, and dragged label positions persist under
`board.schematic.component_labels` as display metadata only. The Sketch dock can
temporarily hide reference or value labels, auto-arrange the visible labels
around component symbols, or reset all custom label positions.

Wire-route context menus can also insert a route handle at the pointer or delete
one visible route handle, and dragging a routed wire segment inserts a waypoint
into the existing route instead of replacing it. Custom routes render as
orthogonal schematic polylines between persisted waypoints. These operations are
schematic display edits only and do not change Board IR connectivity.

Sketch net and wire context menus, plus the selected-net inspector, can place
local named-net labels or off-page connector labels on the schematic. These
labels persist as `board.schematic.net_labels` display metadata for ordinary
Board IR nets. A typed label panel can reuse an existing net, create a missing
net with an explicit net kind, place the label at the view center or by canvas
click/context menu, or rename the selected net to the typed name. Users can
double-click or context-edit a placed label with existing-net autocomplete:
choosing an existing net retargets that label, while typing a missing net
renames the underlying Board IR net through the validated rename path. Selecting
a net traces its wires, peer labels, and connected pin anchors, and label
context menus can jump to the next peer label on the same net. Users can drag
labels to reposition them and finish an active wire on a label to connect to
that label's underlying net; converting, moving, or deleting labels does not
create hidden net ties, hierarchical ports, or PCB
evidence.

## Evidence Import

| Area | Current support | Boundary |
| --- | --- | --- |
| Board IR | Native YAML projects and JSON-schema validation. | Requires explicit modeled evidence; it does not infer design intent from comments. |
| SPICE | `import-spice` preserves deck nodes, model includes, probes, and a file-backed transient scenario. | Imported decks need assertions for sign-off and accurate external models. |
| KiCad schematic | Native `.kicad_sch` import plus mapping files for model bindings, net classes, scenarios, and SPICE/passive metadata. | Connectivity bridge, not full KiCad ERC/DRC semantics. |
| KiCad PCB | Placement, pad, board-outline, route, via, zone, and net-rule evidence for selected validations. | Compact layout evidence, not full fabrication DRC or field solving. |
| JLC/EasyEDA assembly | BOM/CPL import, EasyEDA Pro envelope inspection, flying-probe pad/net evidence. | Assembly evidence does not imply schematic intent or electrical pin behavior by itself. |
| Gerber/Excellon | Outline, copper, mask, paste, PTH/NPTH/via drill evidence and manufacturing metadata overlays. | Complex/nested hole-bearing regions remain bounded by Board IR geometry support. |

## Validation Coverage

| Area | Current executable checks |
| --- | --- |
| Power and boot | `POWER_TREE_VALID`, `RESET_RELEASE_AFTER_POWER_VALID`, `BOOT_STRAP_DEFINED`, `BOOT_STRAP_BIAS_VALID`, `CLOCK_SOURCE_VALID`. |
| Firmware-facing behavior | `UART_BOOTLOADER_SYNC`, `RESIDENT_BOOTLOADER_UPDATE_SEQUENCE`, `CONTROL_LINE_RELEASE_SEQUENCE`, limited `FUNCTIONAL_MCU_FIRMWARE` QEMU pin-observation contracts. |
| IO and protection | `GPIO_BACKDRIVE`, `IO_VOLTAGE_COMPATIBLE`, `INTERFACE_PROTECTION_REVIEW`, USB connector/protection/placement/route/VBUS/return-path checks, CAN/RS485 termination and protection placement. |
| Manufacturing | Drill diameter, drill/slot edge clearance, slot width/aspect, annular ring, copper edge/spacing, solder-mask opening/dam, solder-paste opening/size/area/spacing, IC/BGA paste aperture screens. |
| Motor and load | Motor supply, bridge budget, loss/thermal, switching, SOA, regen clamp, route-current, current-sense accuracy/placement, connector current, cable current/thermal/drop, power-switch budget/reverse/inrush. |
| Model quality | `LOW_CONFIDENCE_MODEL` report limitations and blocking `MODEL_QUALITY_REQUIRED` sign-off gates. |
| Suggestions | Evidence-driven `suggest-scenarios`, including `iot_basic_v0` profile remediation and runnable/non-runnable input contracts. |
| Reports | JSON and Markdown with findings, measurements, limits, severities, limitations, and suggested fixes. |

## Analog Simulation Status

CircuitCI supports bounded SPICE-backed analog validation. It does not fully
support arbitrary analog circuit simulation.

Current analog support:

- `analog_transient` scenarios with `SPICE_TRANSIENT_ANALYSIS`.
- External `ngspice`, dynamic `libngspice`, and fail-closed backend selection.
- File-backed SPICE deck import through `import-spice`.
- GUI editing and save-and-run for file-backed SPICE decks referenced by
  analog scenarios.
- Board IR bindings from SPICE nodes and pins back to board nets/components.
- Generated Board IR transient decks for passives, independent voltage and
  current sources, sourced diodes/BJTs/MOSFETs, and subcircuits.
- Required model-file existence and SHA-256 checks.
- Voltage/current/power probes and waveform assertions.
- Automatic `SPICE_OPERATING_LIMIT` checks for supported generated Board IR
  semiconductor stress limits, including selected derating, pulse, and SOA
  metadata paths.
- Critical findings for missing backends, missing decks, missing model files,
  non-convergence, missing required analog model evidence, and failed waveform
  assertions.

Analog non-goals and open boundaries:

- CircuitCI is not its own full SPICE implementation.
- It does not automatically convert arbitrary KiCad, Altium, or EasyEDA
  schematics into complete physically accurate analog simulations.
- It does not create missing MOSFET, BJT, diode, regulator, charger, op-amp, or
  IC macromodels from datasheets.
- It does not fully solve SMPS compensation, load-transient stability, RF,
  antenna behavior, DDR/high-speed SI, USB PHY eye margin, enclosure physics,
  thermal coupling, or vendor silicon internals.
- Imported SPICE decks without quantitative assertions are waveform evidence,
  not design sign-off.

Use SPICE-backed scenarios for board-boundary analog failures where a real
deck, sourced models, stimuli, probes, and assertions exist. Use deterministic
model checks for static evidence such as voltage ratings, current budgets,
layout distances, SOA curves, cable drop, and manufacturing geometry.

## Practical Sign-Off Rule

A CircuitCI report can support fabrication sign-off only for the checks whose
evidence is present and whose limitations are acceptable. Missing datasheet,
measured, layout, manufacturing, or waveform evidence should remain visible as
critical findings or explicit limitations rather than being inferred.
