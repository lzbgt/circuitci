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
| CLI | Default `circuitci` binary for import, suggestion, validation, suites, report generation, reviewed manufacturing metadata CSV import, and the first Board IR YAML repair loop via `repair-yaml` for `INVALID_POWER_DOMAIN`, `NET_NOT_FOUND`, `PIN_NOT_DECLARED`, and `REQUIRED_PIN_FLOATING` findings plus additive `analog-model-package-metadata` and `bundle-install-package-metadata` migrations. | Primary automation surface for CI and agents. YAML repair is intentionally narrow: it edits only copied projects, changes an existing non-power net to `kind: power` when a model power pin proves that classification, adds a missing net with kind inferred from declared model port kind, removes a stray pin binding not declared by the resolved model, connects a missing required pin only to an existing compatible net already named by component power-domain metadata, adds missing package-lock/registry metadata to generated analog model files when the component library provides matching provenance, or imports package pins from a passing bundle install report. `repair-yaml --dry-run` writes original validation plus candidate proposals without writing or validating a repaired copy; `--apply-report` replays a matching dry-run report, and repeated `--proposal-id` values select a subset of proposed edits for partial apply. Repair reports include stable `reason_codes[]` and proposal-level `reason_code` values so agents can branch on blocked, skipped, no-op, and remaining-finding cases without parsing prose. |
| Desktop GUI | Optional `circuitci-gui` Rust desktop app behind `--features gui`, with native open/save/folder pickers for project, import, and output paths, background KiCad/SPICE import, scenario suggestion, validation, and simulation execution with elapsed status, cancel-result handling plus external ngspice process termination and safe import/suggestion checkpoint cancellation on cancel, active stage-event display including KiCad/SPICE import parser/build/write phases and validation project/model/scenario/report plus analog transient scenario/deck/backend/waveform phases, and a capped recent-job history panel with outcome, elapsed time, output path, diagnostic detail, and distinct canceled outcomes for supported checkpoint stops, EDA-style stages for KiCad schematic/PCB import, SPICE deck import, project loading, schematic-first Sketch workspace with a dominant model canvas, direct Run/scope controls, and one-click Run + Scopes validation-to-observation, runtime-first Scopes workspace with a dominant oscilloscope plot, optional large-waveform deferral with filterable header-only trace previews, background row/visible/all/matching-column and remaining-preview-column Load Deferred recovery with searchable exact preview-column picker actions plus select-visible helpers that mark loaded preview labels, skip duplicate selected-column reloads, and preserve full deferred placeholders after partial trace loads, and loaded/deferred/skipped artifact diagnostics with compact preview-column loaded/unloaded audit metadata, preview-load-state filtering, row-level preview-column load shortcuts, exact column picking, and runtime unload controls that free loaded waveform memory while keeping artifacts reloadable, loaded-waveform footprint readouts with sort/filter controls and guarded bulk unload preview/confirmation, waveform selection, searchable/grouped trace selection, transient pinned trace comparison overlays, saved compare sets, per-trace overlay visibility/color styles, optional per-unit split lanes for mixed-unit compares, direct plot drag time/value-window panning, Alt/Option-drag box zoom, Back/Forward view-window history, wheel time zoom, Shift-wheel value zoom, Fit Y/Y-zoom/Y-pan value scale controls, draggable/click-set A/B cursor handles, selected-trace trigger threshold controls with edge markers, exact event readout rows, previous/next or row-level edge jumps, trace/event-to-schematic cross-focus selection with a visible schematic-context strip and Open Sketch/Fit Context actions, selected-plus-pinned cursor readout rows, cursor/visible-window region statistics with min/max/mean/RMS rows and snapshot capture, searchable/source-filtered transient cursor-region, region-stat, trigger-event, and Scope Activity sample and frequency measurement snapshots with editable labels/notes, interactive plot marker chips plus row-level Jump, schematic Focus, and filtered CSV/Markdown copy/export actions, play/scrub controls, and docked run-setup/check/artifact setup, visual Board IR component/net graph inspection, common-class symbol-style rendering for resistors, capacitors, inductors, diodes, sources, connectors, ICs, and generic blocks, schematic-only rotate/flip/pin-side controls for selected components plus canvas `R` / `Shift+R` rotation, `F` flip, and `Shift+F` pin-side shortcuts for selected components or armed component placement, rendered component pin anchors with net-kind color and active pin/kind chips, draggable graph-node positions, primary-toolbar grid visibility, grid-step, and snap-mode controls, orthogonal wire visuals with net labels and junction dots plus direct schematic-only wire-route shaping by dragging rendered wires or visible route handles, active wire-mode blank-canvas bend clicks, and clearing custom routes from wire menus, derived net-bundle trunks/badges for bracketed, dot-qualified, CAN, I2C, USB, and RS-485-style scalar nets, derived schematic hierarchy sheet groups from imported KiCad source paths and importer namespace prefixes, hierarchy focus/isolate view filters, off-sheet connector badges for focused nets with external endpoints, clickable wire-to-net selection and inspection, right-click component/net/wire action menus, primary-toolbar probe controls for selected nets and components, visible voltage/current/power probe elements derived from analog run-setup probes with latest-report check pass/fail/unknown/unasserted markers, badge clicks and navigator probe rows that select and focus the corresponding Observations-stage scope/probe context, right-click probe-element action menus, selected-probe check table with threshold/timing/status/failure details plus row-level edit/delete actions, hovered-probe check add/clear shortcuts, hovered-probe cursor-sampled quick above/below check shortcuts, hovered-probe Delete/Backspace removal with dependent check cleanup, multi-selected group drag, nudge, edge/center align, and distribute controls, blank-canvas drag, touchpad-scroll, and overview-minimap viewport panning, pointer-focused pinch/Cmd-scroll zoom, Home/Fit All/Fit Selection controls, searchable schematic hierarchy select/fit/focus/isolate controls and object navigator select/fit controls for sheet groups, components, net bundles, nets, wires, and probe elements, Shift-drag replace selection boxes, Cmd/Ctrl-drag additive selection boxes, Alt/Option-drag subtractive selection boxes, L-key freehand lasso variants for those same selection drag chords, multi-selection inspector summary/actions, on-canvas selected-group frame/move handle with snap/free target feedback, transient alignment guides with optional guide snapping, and quick toolbar with rotate/flip/pin-side actions, selected-component duplication and copy/paste with internally referenced local-net copies and shared external nets, multi-selected sketch-item deletion, keyboard/button delete for selected sketch items, pin-to-pin, pin-to-net, and pin-to-wire visual wire assignment by click or drag with multi-bend snap preview, target highlighting, and pin/kind target chips backed by Board IR net reuse/creation, selected-net voltage probe insertion into existing analog run setups, selected-component current probe insertion for generated source branches plus generated passive and diode/BJT/MOSFET current-sense branches, selected-component power probe insertion for those same supported generated branches, graph-node runtime tinting with a closeable floating Scope Activity window and small reopen button, searchable loaded-trace jump browser, bounded per-trace sparklines, row-level sample and frequency snapshot capture plus row-level and visible-list CSV/Markdown copy, row-level/visible-list report bundle export/index-open, and recent-bundle reopen/path-copy shortcuts, trace compare pin/unpin/clear/named-save/load/delete actions, and Open Compare jumps, hoverable/clickable `scope` activity chips, hover readouts, and context-menu Scopes jumps for matching loaded waveform probes, shared undo/redo for Board IR graph/property/wire/YAML edits, unsaved-change confirmation before load/import/quit replaces dirty Board IR YAML or file-backed SPICE deck edits, structured scalar editing, validated rename controls, primitive palette insertion for generic R/C/L and independent voltage/current source components at the current view, canvas click, drag/drop release with orientation-aware live ghost/snap/pin-side feedback, or context-menu pointer with generated pins, nets, SPICE evidence, schematic placement, and optional schematic orientation, and component-level SPICE primitive/value editing for existing component/net properties, add/remove controls for components and unreferenced nets, selected-component pin assignment/removal to existing nets, active-library model search, selected-component model assignment, model-backed component insertion/placement at the current view, canvas click, drag/drop release with orientation-aware live ghost/snap/pin-side feedback, or context-menu pointer with generated default pin nets and optional schematic orientation, generated-from-Board analog transient run-setup creation, generated analog run-setup overview/audit panels with readiness diagnostics and editor navigation actions, generated run-setup analysis setting, ground/node binding, and component include/exclude editing with pin-binding repair, structured DC and pulse source stimulus editing for generated analog run-setup source primitives, SHA-backed SPICE model/include file management for analog run setups, structured sample/window/crossing-time analog check authoring, file-backed SPICE deck editing with save-and-run, Board IR YAML editing with parse-validated save, library suggestions, CSV waveform plotting, simulation-time scrub/playback, A/B cursor values, cursor/visible-window min/max/mean/RMS region statistics with transient snapshots, min/max and delta waveform measurements, GUI-only derived difference/sum/product/ratio waveform channels with promotion to explicit Board IR analog probes or probes plus checks when quantities are representable, simulation artifact observation, and report viewing. | Workflow and observation shell; standards-complete symbol libraries, symbol editors, persisted bus primitives, persisted hierarchical sheet primitives/editing, advanced SPICE model management, subcircuit-internal current/power probes and advanced multi-channel persisted waveform-analysis sign-off are future stages. |

GUI scope usability note: the Sketch canvas opens with derived net-bundle boxes hidden by default, so the connected circuit network is primary; selected nets/components have direct `Scope V`, `Scope I`, and `Scope P` actions that create the missing Board IR probe when needed and open Scopes; `Auto Probes` adds bounded missing voltage probes for analog node bindings and current probes for supported source branches while skipping existing expressions, `Auto before Run` applies the same pass from Sketch or Scopes Run before validation, `Run + Scopes` starts schematic validation and opens Scopes immediately, and the Run Readiness panel previews the exact planned voltage/current probe names, expressions, and targets; the `Scope Tool` V/I/P buttons and canvas-hover `V`/`I`/`P` shortcuts let users arm voltage/current/power probe placement, see hover feedback for the target that will receive the probe, and click a net, wire, pin, label, or component directly on the canvas; pressing the same tool key again or Esc cancels the armed tool; loaded waveform data marks matching schematic nodes/components with transient hoverable/clickable `scope` activity chips, direct searchable Scope Activity floating-window trace jumps with Cursor A scrubbing, per-trace edge stepping, cursor-sampled value/time readouts, compact dominant frequency/period readouts, bounded per-trace sparklines, row-level and visible-list sample and frequency snapshot capture plus row-level and visible-list CSV/Markdown copy, row-level/visible-list report bundle export/index-open, recent-bundle reopen/path-copy shortcuts, live activity-snapshot count, source-specific clear action, and direct `Open Snapshots` jump into source-filtered Scopes snapshots, trace compare pin/unpin/clear/named-save/load/delete actions, sweep-corner compare pinning for the selected trace, report-driven worst-corner trace pinning for sweep margin summaries, compare-set bundle export/index-open for nominal/all-corner/worst-corner waveform evidence plus sweep-margin and Monte Carlo yield rows, and Open Compare jumps, plus an `Open Runtime Trace in Scopes` context-menu jump; Scopes `Run` can create a default transient voltage probe when no analog probes exist; selected traces include a bounded frequency-domain peak readout for quick time/frequency inspection.

Package workflow usability note: when validation reports include model-package
bundle install evidence, Scopes shows scenario-ready registry/lock/artifact
pins and can launch a non-destructive `Repair YAML` background job that imports
those pins into a copied project under the validation output directory. The
generated repair report is retained as a report artifact and projected into
`yaml_repairs[]`/Scopes `YAML repairs` rows with applied/blocked counts and
proof booleans.

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
| KiCad PCB | Placement, footprint property provenance, footprint drawing plus pin-1/body/courtyard semantic summaries, pad geometry plus raw pad-local mask/paste/zone fabrication overrides, board-outline, route, via, zone, and net-rule evidence for selected validations. | Compact layout evidence, not full fabrication DRC, derived solder-mask/paste artwork, BOM/PNP orientation proof, enclosure analysis, or field solving. |
| JLC/EasyEDA assembly | BOM/CPL import with JSON source/row/component/side-confidence/orientation-confidence manifest, EasyEDA Pro envelope inspection with Markdown plus JSON table/payload/object-hash/plaintext-shape manifest, flying-probe pad/net evidence. | Assembly evidence does not imply schematic intent, electrical pin behavior, or final assembly polarity by itself; opaque EasyEDA Pro history payloads block geometry conversion. |
| Gerber/Excellon | Outline, copper, mask, paste, PTH/NPTH/via drill evidence and manufacturing metadata overlays. | Complex/nested hole-bearing regions remain bounded by Board IR geometry support. |

## Validation Coverage

| Area | Current executable checks |
| --- | --- |
| Power and boot | `POWER_TREE_VALID`, `RESET_RELEASE_AFTER_POWER_VALID`, `BOOT_STRAP_DEFINED`, `BOOT_STRAP_BIAS_VALID`, `CLOCK_SOURCE_VALID`. |
| Firmware-facing behavior | `UART_BOOTLOADER_SYNC`, `RESIDENT_BOOTLOADER_UPDATE_SEQUENCE`, `CONTROL_LINE_RELEASE_SEQUENCE`, limited `FUNCTIONAL_MCU_FIRMWARE` QEMU pin-observation contracts. |
| IO and protection | `GPIO_BACKDRIVE`, `IO_VOLTAGE_COMPATIBLE`, `INTERFACE_PROTECTION_REVIEW`, USB connector/protection/placement/route/VBUS/return-path checks, CAN/RS485 termination and protection placement. |
| Manufacturing | Drill diameter, drill/slot edge clearance, slot width/aspect, annular ring, copper edge/spacing, explicit same-layer conductor creepage/clearance screens, controlled-impedance route geometry checks against reviewed target evidence, controlled-impedance stackup material/copper-thickness evidence checks, controlled-impedance solder-mask loading artwork checks, controlled-impedance coupon measurement, batch-statistics, trace-correlation, reviewed solver-result artifact provenance, signed-artifact metadata, output-schema and configuration-lock provenance, runtime option allowlists, solver license/feature entitlement evidence, solver execution environment lock evidence, solver run-log, deterministic-rerun, convergence-window, residual-trend, and numerical-precision reproducibility evidence, input-deck setup plus copper-roughness, etch-compensation, material-library artifact-content field coverage, fabricator material-acceptance, material lot/process drift, material-corner, and fabricator stackup-signoff consistency, board-target mapping, solver tool/version qualification, and solver sweep/corner sample screens, stackup-aware adjacent-plane return-path coverage, slot-crossing, stitching-via transition screens, reviewed RF antenna keepout-to-copper, feed-path route/proximity, matching-network topology, measured return-loss, sampled sweep-coverage, and measurement-condition screens, reviewed thermal copper-area, thermal via/stackup, thermal via-plating/drill/plating-thickness/barrel cross section, package static temperature-rise, and measured-temperature evidence screens, solder-mask opening/dam, solder-paste opening/size/area/spacing, IC/BGA paste aperture screens, assembly-vs-footprint evidence alignment, explicit pin-1 marker orientation, plus reviewed order/process CSV import for `board.manufacturing`, stackup-layer, package thermal, operating-environment, thermal-limit, controlled-impedance solder-mask/coupon/coupon-sample/coupon-trace-correlation/solver-result artifact/signature/output-schema/config-lock/runtime-allowlist/entitlement/environment-lock/run-log/rerun/convergence-sample/residual-trend/precision-policy/input/roughness/etch/material-library/material-corner/stackup-signoff/sample/qualification metadata plus solver material-library artifact-content fields, fabricator material-acceptance, and material lot/process drift metadata, and RF antenna layout/topology/measurement/limit/condition metadata with a JSON row-evidence manifest. |
| Motor and load | Motor supply, bridge budget, loss/thermal, switching, SOA, regen clamp, route-current, current-sense accuracy/placement, connector current, cable current/thermal/drop, power-switch budget/reverse/inrush. |
| Model quality | `LOW_CONFIDENCE_MODEL` report limitations, blocking `MODEL_QUALITY_REQUIRED` sign-off gates, fail-closed OpenVAF/OSDI model compiler provenance checks for compiled Verilog-A compact-model artifacts, schema-backed compact-model package-lock and pinned registry validation for reusable OpenVAF/OSDI and Xyce/ADMS artifacts, built-in generic behavioral SPICE package fixtures imported by generated analog model packs, standalone `circuitci export-model-package` generation, `export-model-conformance-report` generation from validation reports, `merge-model-package-registry` shared registry aggregation, deterministic `export-model-package-bundle` directories plus `verify-model-package-bundle`, `install-model-package-bundle`, and `import-model-package-bundle` reports for portable lock/registry/artifact/conformance evidence, validation-run and validation-suite `model_package_bundle_imports` retention with fail-closed `MODEL_PACKAGE_BUNDLE_IMPORT_FAILED` findings and checkpoint runtime budgets, scenario-ready registry pins, installed-package verification, and YAML repair evidence, validation-report/Markdown/GUI projection of bundle verification/install/import summaries plus generated bundle-install repair commands, and `verify-model-package` preflight reports for package locks/registries, multi-artifact package locks for source/OSDI/Xyce-plugin/conformance artifact sets, schema-backed `model_conformance_report` validation that binds pass/fail checks to runtime artifact hashes, report/solver-manifest `model_file_provenance[]` records that distinguish prebuilt from CI-rebuilt and package-qualified OSDI artifacts, external-ngspice `pre_osdi` wrapper loading, explicit Xyce/embedded backend rejection for OSDI artifacts, fail-closed Xyce/ADMS `xyce_adms_plugin` provenance/conformance contracts, opt-in real ngspice/OpenVAF OSDI conformance through `CIRCUITCI_RUN_REAL_NGSPICE_OSDI=1 cargo test --test analog_model_compiler_cli`, and opt-in real Xyce/ADMS plugin build/load qualification through `CIRCUITCI_RUN_REAL_XYCE_ADMS_PLUGIN=1 cargo test --test analog_model_compiler_cli`. |
| Suggestions | Evidence-driven `suggest-scenarios`, including `iot_basic_v0` profile remediation and runnable/non-runnable input contracts. |
| Reports | JSON and Markdown with findings, measurements, limits, severities, limitations, suggested fixes, and planned-backend `adapter_blocker` / `evidence_sources[]` provenance when present. |

## Source-Backed Common MCU Packs

Current source-backed MCU/module static packs include STM32L431,
STM8S003F3P6, STC15W408AS, ESP32-WROOM, ESP32-S3-WROOM, RP2040, and nRF52840.
The RP2040 pack uses official Raspberry Pi source documents to model
IOVDD/VREG_VIN/DVDD/USB_VDD/ADC_AVDD supply ranges, internal `VREG_VOUT`
`100 mA` core-regulator output, RUN active-low reset, and QSPI_SS
external-flash versus BOOTSEL-USB strap checks. The nRF52840 pack uses
official Nordic documentation and a retained product-spec PDF mirror to model
`VDD`, optional `VDDH`, optional `VBUS`, reset, SWD, USB, and RF antenna
board-boundary pins. The STM8S003F3P6 pack uses official ST documentation to
model `VDD`, `VCAP`, active-low `NRST`, SWIM, and UART1 board-boundary pins.
The STC15W408AS pack uses official STC documentation to model the `VCC` range,
active-high reset boundary, and primary/alternate UART pins. See
`docs/raspberrypi_rp2040_model.md`, `docs/nordic_nrf52840_model.md`,
`docs/st_stm8s003f3p6_model.md`, and `docs/stc_stc15w408as_model.md`.

## Source-Backed Common Support IC Packs

`vendor.ti.ne555` provides a source-backed static model for the TI NE555 timer
family. It models the 4.5 V to 16 V NE555 supply range, GND, timer
board-boundary pins, no-load supply-current metadata, output-current class, and
VCC bypass recommendation. It does not implement RC timing equations,
threshold-spread sign-off, output-drive/load sign-off, or generated-SPICE timer
behavior. See `docs/ti_ne555_model.md`.

`vendor.abracon.abm3_8mhz_18pf` provides a source-backed static model for an
Abracon ABM3 8 MHz crystal with the datasheet standard 18 pF load capacitance.
It lets `CLOCK_SOURCE_VALID` screen board-level crystal placement and load
capacitors while leaving oscillator startup, negative-resistance margin,
drive-level stress, ppm accuracy, layout parasitics, and phase noise out of
scope. See `docs/abracon_abm3_8mhz_18pf_model.md`.

`vendor.winbond.w25q64jv` provides a source-backed static model for Winbond's
W25Q64JV 3 V 64 Mbit SPI/QSPI NOR flash family. It models the 2.7 V to 3.6 V
VCC range, 25 mA program/erase/write-current class, 8-pin SPI/QSPI
board-boundary pins, and memory-geometry metadata. It does not emulate SPI
commands, JEDEC ID, SFDP tables, flash contents, XIP behavior, write-protect
policy, or signal-integrity timing. See `docs/winbond_w25q64jv_model.md`.

`vendor.bosch.bme280` provides a source-backed static model for Bosch's BME280
humidity, pressure, and temperature sensor. It models the 1.71 V to 3.6 V
`VDD` range, the 1.2 V to 3.6 V `VDDIO` range, I2C/SPI board-boundary pins,
low-power current metadata, I2C/SPI clock-class metadata, and SDO-selected I2C
address metadata. It does not validate sensor accuracy, compensation
algorithms, register protocol, bus timing, response time, noise, drift,
self-heating, or calibration. See `docs/bosch_bme280_model.md`.

`vendor.kingbright.apt1608surck` provides a source-backed static/generated-SPICE
model for a common 0603 Hyper Red indicator LED. It records the 30 mA DC
forward-current limit, 5 V reverse-voltage limit, 75 mW power limit,
1.95 V typical / 2.5 V maximum forward voltage at 20 mA, 35 pF capacitance,
and optical metadata. The bundled SPICE card is a reduced preliminary
electrical fit for generated-transient plumbing and source-backed
operating-limit probes; it does not sign off brightness, color bins, lifetime,
thermal board coupling, pulse-drive derating, or production behavior. See
`docs/kingbright_apt1608surck_model.md`.

## Analog Simulation Status

CircuitCI supports bounded SPICE-backed analog validation. It does not fully
support arbitrary analog circuit simulation.

Current analog support:

- `analog_transient` scenarios with `SPICE_TRANSIENT_ANALYSIS`.
- `analog_ac` scenarios with `SPICE_AC_ANALYSIS` for external-ngspice and
  explicit-Xyce small-signal Bode exports. AC runs write `bode.csv` artifacts
  with frequency, per-probe magnitude in dB, phase in degrees, and linear
  magnitude. AC assertions support gain, phase, and group delay at a
  frequency, rising or falling gain-crossing frequency checks, and
  loop-stability phase/gain margin checks. The GUI Scopes loader treats
  `bode.csv` as a frequency-axis artifact, shows magnitude/phase/linear
  traces plus derived group-delay traces in unit-aware lanes, and reuses
  sweep-corner and worst-corner compare pinning for Bode corners. The GUI
  observation-check editor also offers
  low-pass, unity-gain, and loop-stability Bode check presets that write
  normal AC assertions. `backend: auto` prefers ngspice and falls back to Xyce
  for AC when ngspice is absent.
- `analog_dc` scenarios with `SPICE_DC_ANALYSIS` for external-ngspice and
  explicit-Xyce operating-point exports. DC runs write normalized
  `operating_point.csv` artifacts with one column per declared probe, support
  generated-from-board and file-backed decks, and evaluate `operating_point`
  assertions for bias,
  rail, and quiescent checks across the same run-input sweep corners used by
  transient and AC/Bode workflows. The GUI run-setup editor can author
  generated DC operating-point observations directly, and the check editor
  offers ordinary assertion presets for 3.3 V rails, 5 V rails, and 2.5 V
  midpoint bias windows. Scopes loads `operating_point.csv` artifacts into a
  compact DC table with scenario, sweep, corner, probe, value, worst-corner
  marker, artifact label, Copy CSV/Markdown actions, and report-bundle export,
  so bias runs are inspectable and preservable without opening report files.
  `backend: auto` prefers ngspice and falls back to Xyce for DC operating-point
  when ngspice is absent.
- `analog_dc_sweep` scenarios with `SPICE_DC_SWEEP_ANALYSIS` for swept-source
  DC transfer curves. The Board IR/schema can declare `analysis.type:
  dc_sweep`, a swept independent source, start/stop/step values, probes, and
  optional `dc_sweep_assertions[]` over `min`, `max`, `mean`, or a sampled
  sweep point. External `ngspice` and explicit `backend: xyce` run `.dc`/`.DC`,
  write `dc_sweep_raw.csv`, normalize rows into `dc_sweep.csv`, and record the
  run in `solver_manifest.json`. Embedded ngspice remains fail-closed with
  planning evidence for this path. DC sweep assertions feed the shared
  worst-corner and Monte Carlo yield summary machinery. The GUI Run Setup
  editor can author generated-from-board `analog_dc_sweep` scenarios, and the
  GUI DC Sweep Check editor authors `dc_sweep_assertions[]` entries and report
  failure actions hydrate failed checks with probe, aggregation, threshold, and
  sampled sweep value metadata. Opt-in real-solver conformance is available through
  `CIRCUITCI_RUN_REAL_NGSPICE=1 cargo test
  --test analog_dc_sweep_cli` and `CIRCUITCI_RUN_REAL_XYCE=1 cargo test --test
  analog_dc_sweep_cli`; each path skips unless the requested solver is on
  `PATH`. `backend: auto` prefers ngspice and falls back to Xyce for DC sweep
  when ngspice is absent.
- `analog_noise` scenarios with `SPICE_NOISE_ANALYSIS` for external-ngspice
  and explicit-Xyce small-signal noise observations. Noise runs write
  normalized `noise_spectrum.csv` artifacts with frequency, output noise
  density, and input-referred noise density, plus `noise_total.csv` artifacts
  with integrated RMS output and input-referred noise over the declared
  frequency band. Noise assertions support output/input density at a frequency
  and integrated output/input RMS noise checks. The GUI run-setup editor can author
  generated noise observations directly by choosing an output net, input source,
  and frequency band, and the check editor offers output and input-referred
  density/RMS noise presets. Scopes plots `noise_spectrum.csv` as
  frequency-axis output/input density traces and loads `noise_total.csv` into a
  compact integrated-RMS noise table with CSV/Markdown copy actions and
  report-bundle export. Sweep margin summaries mark the limiting output or
  input integrated-RMS noise total directly in Scopes and exported tables.
  Monte Carlo yield summaries appear in Scopes with compact min/max,
  P5-P95, median, and zero-margin distribution strips plus mean/sigma,
  worst-case, percentile margin rows, and CSV/Markdown copy actions, and are
  exported beside scope evidence when the loaded report contains sampled
  tolerance runs; bundle HTML preserves the same distribution strips.
  `backend: auto` prefers ngspice and falls back to Xyce for ordinary `.NOISE`
  when ngspice is absent.
- `analog_sparameter` scenarios with `SPICE_S_PARAMETER_ANALYSIS` for
  frequency-domain S-parameter simulation contracts. The Board IR/schema can
  declare `analysis.type: sparam`, frequency bounds, points per decade, and
  explicit port nodes/reference impedances. The GUI Run Setup editor can
  author generated-from-board two-port `analog_sparameter` scenarios with
  selected port nets, sweep bounds, and shared reference impedance before RF
  port/network/noise checks are added. Explicit `backend: xyce` generates Xyce
  port devices, runs `.AC` plus `.LIN SPARCALC=1`, captures Touchstone RI
  output, and normalizes it to `s_parameters.csv`; explicit
  `backend: ngspice` emits RF port voltage sources, runs `.SP`, exports
  `S_i_j` vectors, and normalizes them to the same CSV contract. Both paths
  retain solver-manifest provenance; when source/load reflection coefficients
  are declared, the normalized CSV carries those metadata columns before
  manifest hashing so downstream reports and Scopes traces use the retained
  solver artifact.
  Validation derives `s_parameter_summary.csv`; optional
  `analysis.s_parameter_assertions[]` can sign off magnitude, return loss,
  insertion loss, VSWR, mismatch loss, group-delay, and reflection-impedance
  limits with fail-closed metric applicability checks. Scopes includes an RF
  Port Check editor for authoring those per-parameter checks without
  hand-editing YAML, and retained S-parameter assertion failures, including
  mismatch-loss and reflection-impedance limits, can be loaded back into the RF
  Port Check editor from the report artifact panel.
  Optional `analysis.s_parameter_network_assertions[]` can sign off two-port
  reciprocity (`max |S21-S12|`) and passivity (maximum singular value of the
  complex S matrix), plus active-network stability margins (`rollet_k_min`
  and `stability_delta_magnitude_max`) and gain ceilings/floors
  (`maximum_available_gain_db_min`, `maximum_stable_gain_db_min`, and
  `maximum_unilateral_gain_db_min`), from
  full `s_parameters.csv` data. Maximum available gain is retained only for
  unconditionally stable rows (`K > 1`, `|Delta| < 1`). Optional
  `s_parameter_source_reflection` and `s_parameter_load_reflection` provenance
  unlock source/load-dependent transducer, available, and operating gain
  metrics (`transducer_gain_db_min`, `available_gain_db_min`, and
  `operating_gain_db_min`) with fail-closed missing-provenance behavior.
  Optional `analysis.s_parameter_noise_assertions[]` signs off RF two-port
  noise figure, minimum noise figure, equivalent noise resistance, and optimum
  source reflection magnitude from normalized `s_parameter_noise_summary.csv`
  evidence emitted by the ngspice `.SP ... donoise=1` adapter. That adapter
  now also normalizes ngspice S-matrix output into retained
  `s_parameters.csv`, so mixed RF term, network, and noise assertions can run
  from one ngspice SP-noise solver pass. The Xyce Touchstone path still fails
  closed for these noise metrics because it does not emit NF/NFmin/Rn/SOpt
  evidence; the fail-closed finding carries `adapter_blocker` and
  `evidence_sources[]` metadata pointing to the retained RF SP-noise backend
  evidence. Opt-in real-ngspice S-parameter conformance is available through
  `CIRCUITCI_RUN_REAL_NGSPICE=1 cargo test --test analog_sparameter_cli`; it
  skips unless `ngspice` is on `PATH` and covers ordinary `.SP` matrix
  normalization plus mixed S-parameter term, network, and RF-noise sign-off
  from one `.SP ... donoise=1` run.
  Reports project retained rows into `s_parameter_summaries[]`, retained
  network-quality rows into `s_parameter_network_summaries[]`, and retained RF
  noise rows into `s_parameter_noise_summaries[]`. Markdown includes
  "S-Parameter Summary", "S-Parameter Network Summary", and "S-Parameter Noise
  Summary" sections, the GUI artifact panel shows compact RF term, network,
  and RF-noise sign-off rows.
  GUI Scopes also loads retained `s_parameter_noise_raw.csv` artifacts as
  frequency-axis traces for NF, NFmin, equivalent noise resistance, and
  `|SOpt|`, using the same raw column aliases accepted by the ngspice adapter.
  Scopes includes an RF Network Check editor for authoring two-port
  reciprocity, passivity, Rollet K, `|Delta|`, MAG, MSG, unilateral-gain, and
  source/load gain sign-off checks without hand-editing YAML, including the
  required source/load reflection coefficient provenance for Gt/Ga/Gp checks.
  Scopes also includes an RF Noise Check editor for authoring
  `noise_figure_db_max`, `minimum_noise_figure_db_max`,
  `equivalent_noise_resistance_ohm_max`, and
  `optimum_source_reflection_magnitude_max` limits without hand-editing YAML.
  Retained network assertion failures, including
  MAG/MSG/unilateral-gain/source-load-gain limits, can be loaded back into
  that editor from the report artifact panel with metric, relation, threshold,
  and source/load reflection fields hydrated when report metadata provides
  them. Retained RF noise assertion failures and fail-closed missing-evidence
  RF noise findings can likewise hydrate the RF Noise Check editor with
  scenario, check name, metric, relation, and threshold. Scopes derives
  return-loss, insertion-loss, VSWR, mismatch-loss, group-delay,
  reflection-impedance, two-port reciprocity error, passivity singular-value,
  stability `|Delta|`, Rollet K, MAG, MSG, unilateral-gain, and
  source/load-dependent transducer/available/operating gain traces from full
  `s_parameters.csv` artifacts when the required reflection metadata columns
  are present.
  `backend: auto` prefers ngspice and can fall back to Xyce for ordinary
  S-parameter term/network sign-off when ngspice is absent. RF SP-noise
  assertions remain evidence-gated: auto-selected Xyce can normalize the
  S-matrix, but the RF-noise assertion fails closed with blocker/evidence
  metadata until a backend emits `s_parameter_noise_summary.csv`.
- `analog_transfer_function` scenarios with
  `SPICE_TRANSFER_FUNCTION_ANALYSIS` for `.TF` small-signal transfer
  contracts. The Board IR/schema can declare `analysis.type: tf`,
  `transfer_output_expression`, and `transfer_input_source`. External
  `ngspice` runs write `transfer_function_raw.txt`,
  `transfer_function_summary.csv`, and `solver_manifest.json`; the normalized
  summary records gain, input resistance, and output resistance.
  `analysis.transfer_function_assertions[]` can sign off gain or resistance
  limits, with malformed summaries and failed limits failing closed.
  Validation reports project retained rows into `transfer_function_summaries[]`,
  Markdown includes a "Transfer Function Summary" section, and the GUI artifact
  panel shows compact `.TF` scalar rows. The GUI Transfer Function Check editor
  authors `analysis.transfer_function_assertions[]` entries and failure actions
  hydrate metric, relation, and threshold from retained findings. The GUI Run
  Setup editor can author generated-from-board `analog_transfer_function`
  scenarios from a selected output probe net and input source.
  Opt-in real-ngspice conformance
  coverage is available through
  `CIRCUITCI_RUN_REAL_NGSPICE=1 cargo test --test analog_transfer_function_cli`;
  it skips unless `ngspice` is on `PATH`. Xyce remains fail-closed with
  `adapter_blocker` and `evidence_sources[]` metadata because the saved Xyce
  7.8 primary docs do not document a native `.TF` command or equivalent result
  artifact; embedded ngspice remains fail-closed with planning evidence.
- `analog_pole_zero` scenarios with `SPICE_POLE_ZERO_ANALYSIS` for `.PZ`
  small-signal pole/zero extraction contracts. The Board IR/schema can declare
  `analysis.type: pz`, output and reference nodes, an input source, and a mode
  of `poles`, `zeros`, or `poles_and_zeros`. External `ngspice` runs write
  `pole_zero_raw.txt`, `pole_zero_summary.csv`, and `solver_manifest.json`;
  the normalized summary records each pole/zero as complex rad/s coordinates
  plus derived frequency. Pole-zero assertions can sign off root real part,
  imaginary part, or derived frequency by root kind and optional root index;
  missing or ambiguous roots fail closed. Validation reports project retained
  root rows into `pole_zero_summaries[]`, Markdown reports include a
  "Pole-Zero Summary" section, and the GUI artifact panel shows compact root
  rows. The GUI Pole-Zero Check editor authors
  `analysis.pole_zero_assertions[]` entries and failure actions hydrate root
  kind, optional root index, metric, relation, and threshold from retained
  findings. The GUI Run Setup editor can author generated-from-board
  `analog_pole_zero` scenarios from a selected output probe net, input source,
  and extraction mode. Opt-in real-ngspice conformance coverage is available
  through
  `CIRCUITCI_RUN_REAL_NGSPICE=1 cargo test --test analog_pole_zero_cli`;
  it skips unless `ngspice` is on `PATH`. Xyce remains fail-closed with
  `adapter_blocker` and `evidence_sources[]` metadata because the saved Xyce
  7.8 primary docs do not document a native `.PZ` command or equivalent result
  artifact; embedded ngspice remains fail-closed with planning evidence.
- `analog_sensitivity` scenarios with `SPICE_SENSITIVITY_ANALYSIS` for
  ngspice/Xyce `.SENS` sensitivity contracts. The Board IR/schema can declare
  `analysis.type: sens`, `sensitivity_output_expression`, `sensitivity_mode`
  (`dc` or `ac`), optional `sensitivity_filters[]`, and AC frequency bounds.
  External `ngspice` runs write `sensitivity_raw.txt`,
  `sensitivity_summary.csv`, and `solver_manifest.json`; the normalized summary
  records DC scalar sensitivities or AC per-frequency complex sensitivity
  values plus magnitude. Explicit `backend: xyce` runs write
  `circuitci_xyce_sens.cir`, `xyce_sens.log`, `sensitivity_raw.csv`,
  `sensitivity_summary.csv`, and `solver_manifest.json`; Xyce requires
  `sensitivity_filters[]` because Xyce `.SENS` needs an explicit `param=`
  list, and generated resistor/capacitor/inductor names are mapped to Xyce
  `R`/`C`/`L` parameter tokens in the wrapper while summary rows retain the
  declared filter names. `analysis.sensitivity_assertions[]` can sign off real,
  imaginary, or magnitude rows by parameter and optional AC frequency, with
  missing or ambiguous rows failing closed. Validation reports project retained
  rows into `sensitivity_summaries[]`, Markdown includes a "Sensitivity Summary"
  section, the GUI artifact panel shows compact sensitivity rows, and Scopes
  loads AC `sensitivity_summary.csv` rows as parameter-grouped frequency traces
  for magnitude, real, and imaginary sensitivity. The GUI Sensitivity Check
  editor authors `analysis.sensitivity_assertions[]` entries and failure
  actions hydrate parameter, optional frequency, metric, relation, and threshold
  from retained findings. The GUI Run Setup editor can author
  generated-from-board `analog_sensitivity` scenarios with DC/AC mode,
  selected output probe net, AC sweep bounds when needed, and explicit
  sensitivity filters. Opt-in real-ngspice
  conformance coverage is available through
  `CIRCUITCI_RUN_REAL_NGSPICE=1 cargo test --test analog_sensitivity_cli`;
  it skips unless `ngspice` is on `PATH`. Opt-in real-Xyce `.SENS`
  conformance is available through `CIRCUITCI_RUN_REAL_XYCE=1 cargo test --test
  analog_sensitivity_cli`; it skips unless `Xyce` or `xyce` is on `PATH`.
  `backend: auto` prefers ngspice and falls back to Xyce for `.SENS` when
  ngspice is absent and the Xyce filter contract is satisfied.
  Embedded ngspice remains fail-closed with planning evidence for this path.
- `analog_distortion` scenarios with `SPICE_DISTORTION_ANALYSIS` for
  ngspice-backed small-signal distortion analysis. The Board IR/schema can declare
  `analysis.type: disto`, `distortion_mode: harmonic|intermodulation`,
  frequency sweep bounds, points per decade, a bound
  `distortion_output_expression`, `distortion_f1_sources[]`, optional
  `distortion_f2_sources[]`, `distortion_f2_over_f1` for intermodulation, and
  optional `distortion_assertions[]` over normalized component max magnitudes.
  For `backend: ngspice`, CircuitCI annotates declared sources with explicit
  `DISTOF1`/`DISTOF2` defaults, runs `.disto dec`, and emits
  `distortion_spectrum.csv`, `distortion_summary.csv`,
  `distortion_convergence.json`, raw solver output, and `solver_manifest.json`.
  Distortion assertions fail scenarios when `distortion_summary.csv` component
  maxima violate declared `above`/`below` limits or when a required component
  is absent. Validation reports project `distortion_summary.csv` rows into
  `distortion_summaries[]`, and Scopes recognizes `distortion_spectrum.csv` as
  frequency-axis magnitude/phase/real/imaginary traces grouped by distortion
  component. The GUI Run Setup editor can author generated-from-board
  `analog_distortion` scenarios with harmonic/intermodulation mode, frequency
  sweep bounds, selected output probe net, F1 source, and F2 source plus
  F2/F1 ratio for intermodulation. The GUI Distortion Check editor authors
  `analysis.distortion_assertions[]` entries and failure actions hydrate the
  component, relation, and threshold from retained findings.
  Opt-in real-ngspice conformance is available through
  `CIRCUITCI_RUN_REAL_NGSPICE_DISTO=1 cargo test --test analog_distortion_cli`.
  Xyce remains fail-closed because the saved Xyce reference text does not
  document a matching distortion analysis command; fail-closed non-ngspice
  findings carry `adapter_blocker` and `evidence_sources[]` metadata pointing
  back to the retained distortion backend evidence.
- `analog_fourier` scenarios with `SPICE_FOURIER_ANALYSIS` for
  transient-backed `.FOUR` harmonic extraction contracts. The Board IR/schema
  can declare `analysis.type: fourier`, transient stop/max-step timing, a
  fundamental frequency, a bound voltage/current output expression, and an
  optional harmonic count. Optional `fourier_assertions[]` can sign off
  harmonic magnitude, normalized magnitude, phase, normalized phase, and
  THD-percent limits from `fourier_summary.csv`. External `ngspice` runs write
  `fourier_raw.txt`, `fourier_summary.csv`, and `solver_manifest.json`;
  explicit `backend: xyce` writes the documented Xyce `<netlist>.four0` output
  and the same normalized `fourier_summary.csv`/manifest contract. The
  normalized summary records DC plus harmonic rows, frequency, magnitude/phase,
  normalized magnitude/phase, and THD/grid metadata when the backend provides
  it.
  Validation reports project retained Fourier rows into `fourier_summaries[]`,
  Markdown reports include a "Fourier Summary" section, and the GUI artifact
  panel shows compact harmonic/THD rows. Scopes loads `fourier_summary.csv` as
  frequency-axis magnitude/phase and normalized magnitude/phase traces for
  plotting. The GUI Run Setup editor can author generated-from-board Fourier
  scenarios with stop/max-step timing, selected output probe net, fundamental
  frequency, and harmonic count; the GUI Fourier Check editor authors
  `analysis.fourier_assertions[]` and failure actions hydrate the editor with
  the failed harmonic or THD metric, relation, and threshold. Opt-in
  real-ngspice conformance coverage is available through
  `CIRCUITCI_RUN_REAL_NGSPICE=1 cargo test --test analog_fourier_cli`; it skips
  unless `ngspice` is on `PATH`. Opt-in real-Xyce `.FOUR` conformance is
  available through `CIRCUITCI_RUN_REAL_XYCE=1 cargo test --test
  analog_fourier_cli`; it skips unless `Xyce` or `xyce` is on `PATH`.
  `backend: auto` prefers ngspice and falls back to Xyce for `.FOUR` when
  ngspice is absent.
  Embedded ngspice remains fail-closed with planning evidence.
- `analog_harmonic_balance` scenarios with
  `SPICE_HARMONIC_BALANCE_ANALYSIS` for periodic steady-state spectrum
  contracts. The Board IR/schema can declare `analysis.type: hb`,
  `hb_fundamental_frequency_hz`, a bound `hb_output_expression`, optional
  `hb_harmonics`, and `hb_drive_sources[]` tied to generated source
  components when the deck is generated from Board IR. Explicit `backend:
  xyce` runs `.HB`, emits `hb_spectrum_raw.csv`, normalizes signed harmonic
  complex spectrum rows into `hb_spectrum.csv`, and records the run in
  `solver_manifest.json`. Optional `hb_assertions[]` entries sign off
  retained spectrum magnitude, phase, real, and imaginary values by harmonic,
  with fail-closed behavior for missing harmonic rows. Validation reports
  project retained spectrum rows into `hb_summaries[]`, Markdown reports include
  a "Harmonic Balance Summary" section, and the GUI report panel surfaces the
  same rows. The GUI Scopes loader recognizes `hb_spectrum.csv` artifacts and
  plots non-negative harmonic rows as frequency-axis magnitude, phase, real,
  and imaginary traces. The GUI Run Setup editor can author
  generated-from-board harmonic-balance scenarios with fundamental frequency,
  harmonic count, selected output probe net, and selected drive source. The GUI
  HB Check editor authors `analysis.hb_assertions[]` entries and failure
  actions hydrate the editor with the failed harmonic, metric, relation, and
  threshold from retained validation findings.
  Opt-in real-Xyce conformance is available
  through `CIRCUITCI_RUN_REAL_XYCE=1 cargo test --test
  analog_harmonic_balance_cli`; it skips unless `Xyce` or `xyce` is on `PATH`.
  `backend: auto` is Xyce-first for this analysis because Xyce is the only
  backend that currently emits normalized `hb_spectrum` evidence. Explicit
  `ngspice` and embedded ngspice remain fail-closed with planning evidence plus
  `adapter_blocker` and `evidence_sources[]` metadata until they emit the same
  normalized output contract.
- `analog_pss` scenarios with `SPICE_PSS_ANALYSIS` for periodic steady-state
  and oscillator evidence planning. The Board IR/schema can declare
  `analysis.type: pss`, `pss_mode: driven|autonomous`,
  `pss_frequency_guess_hz`, `pss_stabilization_time_us`, a bound
  `pss_output_expression`, optional `pss_periods`, optional convergence
  thresholds, and `pss_drive_sources[]` for driven generated-source
  provenance. The GUI Run Setup editor can author generated-from-board PSS
  planning scenarios with driven/autonomous mode, selected output net,
  frequency guess, stabilization interval, periods, and driven-source
  provenance. This path intentionally fails closed today: it records
  manifest-compatible planning evidence for future `pss_waveform`,
  `pss_spectrum`, and `pss_convergence` artifacts, but no backend is allowed to
  pass PSS or oscillator sign-off until those normalized outputs exist. The
  planning finding records the current source-backed backend boundary: Xyce HB
  is not treated as PSS, ngspice PSS remains experimental/build-gated without a
  trusted normalized artifact contract here, and SPICE OPUS `ssse` needs a
  dedicated adapter plus conformance before use. The finding also carries
  compact `adapter_blocker` and `evidence_sources[]` metadata pointing to the
  retained PSS backend evidence.
- `analog_phase_noise` scenarios with `SPICE_PHASE_NOISE_ANALYSIS` for
  oscillator phase-noise evidence planning. The Board IR/schema can declare
  `analysis.type: phase_noise`, `phase_noise_mode: driven|autonomous`,
  `phase_noise_carrier_frequency_hz`, offset sweep fields,
  `phase_noise_output_expression`, optional integration window fields, and
  `phase_noise_drive_sources[]` for driven generated-source provenance. The
  GUI Run Setup editor can author generated-from-board phase-noise planning
  scenarios with driven/autonomous mode, selected output net, carrier,
  offset sweep, and driven-source provenance. This
  path intentionally fails closed today: it records manifest-compatible
  planning evidence for future `phase_noise_spectrum`,
  `phase_noise_integrated_jitter`, `phase_noise_convergence`, and
  `pss_convergence` artifacts, but no backend is allowed to pass oscillator
  phase-noise sign-off until a trusted PSS/PNOISE chain exists. The planning
  finding records that QUCS-COPEN is currently a theory/reference source, not
  an executable backend target, because no public source repository or adapter
  contract was found. The finding also carries a compact `adapter_blocker` and
  `evidence_sources[]` list pointing to the retained research notes needed to
  unblock a future adapter.
- `analog_periodic_ac` scenarios with `SPICE_PERIODIC_AC_ANALYSIS` for
  PAC/PXF-style periodic small-signal evidence planning. The Board IR/schema
  can declare `analysis.type: pac`, `pac_mode: pac|pxf`,
  `pac_carrier_frequency_hz`, frequency sweep bounds, points per decade, a
  bound `pac_output_expression`, `pac_input_source`, optional
  `pac_sidebands`, and `pac_drive_sources[]` for generated large-signal source
  provenance. The GUI Run Setup editor can author generated-from-board
  PAC/PXF planning scenarios with selected output net, carrier, small-signal
  sweep, input source, drive source, and sideband count. This path
  intentionally fails closed today: it records
  manifest-compatible planning evidence for future `pac_response`,
  `pac_sidebands`, `pac_convergence`, and `pss_convergence` artifacts, but no
  backend is allowed to pass periodic small-signal sign-off until a trusted
  PSS/HB linearization chain and PAC/PXF normalizer exist. The saved
  PAC/PXF evidence note records why Xyce HB output, experimental ngspice PSS,
  and QUCS-COPEN papers are not yet adapter-ready. The finding also carries a
  compact `adapter_blocker` and `evidence_sources[]` list pointing to the
  retained research notes needed to unblock a future adapter.
- `analog_measure` scenarios with `SPICE_MEASURE_ANALYSIS` for scalar
  extraction from transient or AC runs. The Board IR/schema
  can declare `analysis.type: measure`, `measure_mode`, reviewed raw
  `measure_statements[]`, or portable `measure_templates[]` with operation,
  expression, time/frequency windows, transient `delay`/`slew`
  trigger-target measurements, or transient `threshold_time` crossing
  measurements. External `ngspice` runs raw statements or templates; explicit
  `backend: xyce` runs structured templates only. Both write
  `measure_raw.txt`, `measure_summary.csv`, and `solver_manifest.json`; the
  normalized summary records measurement name, mode, scalar value, and raw
  solver line. Optional `measure_assertions[]` compare named normalized
  measurements against finite `above`/`below` thresholds so scalar simulation
  specs can directly pass or fail validation; swept measure assertions emit the
  same worst-corner and Monte Carlo yield summaries used by waveform, AC, DC,
  and noise assertions. The GUI Run Setup editor can author template-based
  generated-from-board `analog_measure` scenarios for transient or AC
  `avg`/`max`/`min`/`rms` measurements over bounded time/frequency windows. The
  GUI Measure Check editor authors `measure_assertions[]` entries from declared
  measurement names and failure actions hydrate measurement, relation, and
  threshold from retained findings.
  The Xyce path records measure result files such as
  `.mt0` when emitted. Raw `measure_statements[]` remain ngspice-only and fail
  closed on Xyce with `adapter_blocker` and `evidence_sources[]` metadata
  pointing to retained measure backend evidence. Opt-in real-ngspice conformance
  coverage is available through
  `CIRCUITCI_RUN_REAL_NGSPICE=1 cargo test --test analog_measure_cli`; it skips
  unless `ngspice` is on `PATH`. Opt-in real-Xyce template conformance uses
  `CIRCUITCI_RUN_REAL_XYCE=1 cargo test --test analog_measure_cli`; it skips
  unless `Xyce` or `xyce` is on `PATH`. `backend: auto` prefers ngspice, but
  can fall back to Xyce for template-only measure scenarios when ngspice is
  absent. Embedded ngspice remains fail-closed with planning evidence for this
  path.
- Successful analog solver runs write a versioned `solver_manifest.json`
  artifact beside normalized outputs. The manifest records backend selection,
  solver command/status, source deck, wrapper deck, log, model files, sweep
  overrides, raw outputs, and normalized outputs so future Xyce/RF adapters can
  target the same provenance contract. OpenVAF/OSDI model files also record
  source/artifact hash verification and rebuild mode under
  `inputs.model_file_provenance[]`, and reports project those rows into
  top-level `model_file_provenance[]` plus the GUI artifact drawer. The
  OpenVAF/OSDI path is external-ngspice-only. Xyce compact-model loading uses
  the separate `xyce_adms_plugin` contract with pinned source/plugin and
  conformance artifacts, but remains fail-closed until a real-Xyce plugin
  loader is implemented. The opt-in Xyce/ADMS qualification hook is
  `CIRCUITCI_RUN_REAL_XYCE_ADMS_PLUGIN=1 cargo test --test analog_model_compiler_cli`;
  it skips unless `Xyce`/`xyce` and `buildxyceplugin` are on `PATH`.
- Explicit `backend: xyce` is detected when `Xyce` or `xyce` is on `PATH`.
  Transient, AC, DC, DC sweep, and noise Xyce runs can export CSV-like solver
  data, normalize it into the `transient_waveform`, `ac_bode`,
  `operating_point`, `dc_sweep`, `noise_spectrum`, and `noise_total`
  contracts, and write solver manifests.
  `backend: auto` prefers ngspice, can fall back to Xyce for transient, AC, DC,
  DC sweep, ordinary noise, ordinary S-parameter term/network, sensitivity,
  Fourier, and template-only measure scenarios, prefers Xyce for harmonic
  balance, and can select Xyce for planned-only `.TF`, `.PZ`, distortion, PSS,
  phase-noise, and PAC/PXF scenarios only to emit the retained source-backed
  fail-closed blocker metadata when ngspice is absent. The opt-in real-solver
  conformance paths are
  `CIRCUITCI_RUN_REAL_XYCE=1 cargo test --test analog_spice_xyce_cli` for
  transient/AC/DC/noise,
  `CIRCUITCI_RUN_REAL_XYCE=1 cargo test --test analog_dc_sweep_cli` for DC
  sweep,
  `CIRCUITCI_RUN_REAL_XYCE=1 cargo test --test analog_sparameter_cli` for
  S-parameter, and
  `CIRCUITCI_RUN_REAL_XYCE=1 cargo test --test analog_measure_cli` for
  structured measure templates. They skip unless `Xyce` or `xyce` is on `PATH`.
- External `ngspice`, dynamic `libngspice`, and fail-closed backend selection.
- File-backed SPICE deck import through `import-spice`.
- GUI editing and save-and-run for file-backed SPICE decks referenced by
  analog run setups.
- Board IR bindings from SPICE nodes and pins back to board nets/components.
- Generated Board IR transient, AC/Bode, DC operating-point, DC sweep,
  transfer-function, pole-zero, sensitivity, and noise decks for passives,
  independent
  voltage and current sources, sourced diodes/BJTs/MOSFETs, and subcircuits.
  GUI run-setup creation can author generated AC/Bode observations with
  frequency limits and an initial voltage probe; generated AC sources emit a
  unity small-signal `AC 1` drive while preserving their DC or pulse operating
  point. GUI-created generated DC observations write `.op` analysis setups
  with an initial voltage probe; GUI-created generated transfer-function
  observations write `.tf` analysis setups with selected output/input source
  metadata; GUI-created generated pole-zero observations write `.pz` analysis
  setups with selected output/reference/input-source and extraction-mode
  metadata; GUI-created generated sensitivity observations write `.sens`
  analysis setups with selected output, mode, frequency bounds when needed, and
  explicit parameter filters; GUI-created generated noise observations write
  `.noise` analysis setups with output/input noise probes. All generated
  observation types reuse the same ground/node/component editors.
- Bounded analog run-input sweeps, with each sweep corner exported as its own
  waveform, Bode, or operating-point artifact set, tagged on findings, and
  summarized with per-assertion worst-corner margin info findings for
  transient, AC/Bode, DC operating-point, and noise assertions.
  Sweeps can use raw SPICE `.param` values, generated component value inputs
  such as `RLOAD.value_ohm` or `VSUPPLY.dc_v`, and vendor model-library
  sections through section-specific ngspice `.lib` cards. They can also use
  deterministic Monte Carlo component-value samples for generated-board
  tolerance observations using uniform or normal distributions, with sampled
  values represented as normal component value corners and summarized with
  per-assertion yield percent, pass/fail counts, mean margin, margin standard
  deviation, min/max margin, and
  linearly interpolated P1/P5/P50/P95 sampled-margin percentiles. Optional
  Monte Carlo criteria can require minimum yield percent or percentile margins,
  causing sampled tolerance observations to fail when design-yield targets are
  missed while preserving individual sampled assertion failures as evidence rows
  instead of direct run failures.
- GUI Run Input Sweeps editing for analog run setups, including sweep creation
  with an executable first parameter or component value, generated load/source
  candidate selection, extra parameter/component/model-section add/remove
  controls, Monte Carlo sampled component-value creation/editing, Monte Carlo
  criteria set/clear controls, and declared corner-count summaries without
  editing YAML by hand.
  Built-in corner presets add common supply, load, temperature, model-selector,
  and RC-tolerance sweeps; `TEMP_C` and `TEMPERATURE_C` also drive ngspice
  `.temp`. Declared Monte Carlo sweeps are shown with their sample counts and
  sampled component fields/distributions in the same table, and users can
  author sample count, seed, nominal/tolerance targets, uniform/normal
  distribution, minimum yield percent, and P1/P5/P50/P95 margin targets from
  the GUI.
- GUI generated run-setup overview rows for completed sweep worst-corner
  assertion margins, including limiting corner, parameter values, component
  value inputs, selected model sections, measured value, limit, margin,
  pass/fail state, and evaluated corner count.
- Required model-file existence and SHA-256 checks.
- Voltage/current/power probes and waveform assertions, including single-point
  samples, min/max/mean/RMS windows, signed voltage/current/power integration
  windows, power-probe energy windows, rising/falling crossing-time checks,
  minimum high/low pulse-width checks, duty-cycle checks, threshold
  crossing-count checks for no-recross or ringing budgets, settling-time
  checks against target/tolerance bands, and overshoot-percent checks against
  target values, rising/falling phase-delay checks between two probes, and
  setup/hold timing checks around reference probe edges.
- Automatic `SPICE_OPERATING_LIMIT` checks for supported generated Board IR
  semiconductor stress limits, including selected derating, pulse, and SOA
  metadata paths.
- Generic reusable behavioral macro-model pack entries for preliminary
  generated-board simulation of op-amp buffers, comparator threshold behavior,
  enabled 3.3 V regulator rails, CH340C-style USB-UART output-state
  observations, CP2102N-style USB-UART VREGIN/VDD and output-state
  observations, FT232R-style USB-UART 3V3OUT/VCCIO and output-state
  observations, CH347-style USB-JTAG debug bridge line-state observations,
  CMSIS-DAP-style SWD probe line-state observations,
  STM32L431-style VDD, NRST, BOOT0, USART1, and SWD line-state observations,
  ESP32-WROOM-32E-style module supply, EN, GPIO0/GPIO2 boot-strap, and TXD0
  idle-state observations, ESP32-S3-WROOM-style module supply, EN, boot-strap,
  and USB D-/D+ line-state observations, LicheeRV-Nano-W-style 5 V module,
  UART0, motion-enable, and fault-IRQ line-state observations,
  AT32F435-style motion-core VDD, UART, CAN, RS-485, and control-line
  observations,
  AT32M416-style motor-control VDD, CAN, PWM, driver-interface, current-sense,
  encoder, enable, and fault-line observations,
  TXS0108E-style A-to-B mixed-voltage level-shifter observations,
  NL27WZ17-style dual non-inverting Schmitt-buffer input/output line-state
  observations,
  TPD2EUSB30-style USB ESD normal-operation standoff and line-capacitance
  observations,
  PRTR5V0U2X-style rail-to-rail USB ESD normal-operation standoff and
  capacitance observations,
  ESD2CAN24-Q1-style CAN ESD normal-operation standoff and line-capacitance
  observations,
  TCAN3413-style CAN transceiver line-state observations,
  ESDS552-style RS-485/RS-422 ESD/surge normal-operation standoff and
  line-capacitance observations,
  THVD1450-style RS-485 transceiver line-state observations,
  DRV8323-style three-phase gate-driver supply, output-state, and current-sense
  observation-node checks,
  PCA9685-style PWM-driver VDD/OE, I2C idle-line, and low-load PWM output
  observations, ICM-42688-P-style IMU VDD/VDDIO, SPI line-state, and INT1
  observations,
  AMS1117-style fixed 3.3 V LDO rail observations,
  TPS54331-style 5 V buck-regulator rail observations,
  TPS62162-style 3.3 V buck-regulator rail observations,
  TPS63802-style 3.3 V buck-boost rail observations, TPS61023-style 5 V
  boost-regulator rail observations, TPS2121/TPS2115A-style selected-source power-mux
  observations, enabled load-switch paths, MCP73831-style PROG-programmed
  Li-Ion charger observations, BQ24075-style power-path charger observations,
  and BQ25798-style buck-boost/NVDC charger observations through explicit `simulation.spice`
  subcircuits. These models are low-confidence
  workflow/topology aids, not vendor sign-off evidence. The GUI Examples picker
  includes direct-open observation fixtures for NE555, RC low-pass, comparator
  threshold, op-amp buffer, CH340C USB-UART bridge, CP2102N USB-UART bridge,
  FT232R USB-UART bridge, CH347 USB-JTAG debug bridge, CMSIS-DAP SWD probe,
  STM32L431 boot/UART/SWD, ESP32-WROOM-32E boot/UART, ESP32-S3-WROOM boot/USB,
  LicheeRV-Nano-W module, AT32F435 motion core, AT32M416 motor control,
  TXS0108E level shifter, TPD2EUSB30 USB ESD, PRTR5V0U2X USB ESD,
  ESD2CAN24-Q1 CAN ESD, TCAN3413 CAN transceiver, DRV8323 gate driver, PCA9685 PWM driver, ICM-42688-P IMU, ESDS552 RS-485 ESD, THVD1450 RS-485 transceiver, AP2112K LDO rail, AMS1117 LDO rail, TPS54331 buck rail, TPS62162 buck rail, TPS63802 buck-boost rail,
  TPS61023 boost rail, TPS2121 power mux, TPS2115A power mux,
  TPS22918 load switch, TPS25948 eFuse, TPS24751 hot-swap, MCP73831 charger, BQ24075 power path, BQ25798 NVDC
  power path, TLV803 reset-supervisor, loop-stability Bode, DC divider-bias,
  divider-noise, NL27WZ17 logic-buffer, JST XH/VH connector contact-drop, and
  RC Monte Carlo yield workflows.
- The AP2112K-3.3 vendor component pack now has a datasheet-backed generated
  SPICE observation face: it keeps Diodes Incorporated voltage/dropout/current
  metadata and pin order while using the reduced-fidelity generic enabled
  3.3 V LDO macro-model for preliminary rail observation.
- The AMS1117-3.3 vendor component pack now has a datasheet-backed generated
  SPICE observation face for a fixed 3.3 V rail with a 5 V input, 22 uF output
  capacitor, and minimum-load resistor in the direct-open GUI fixture. Its
  15 V absolute VIN limit, 3.201 V to 3.399 V output window, 1.3 V dropout
  limit, 10 mA minimum load, 0.8 A output-current screen, and 22 uF output
  capacitance requirement remain source-backed, while the transient face stays
  explicitly reduced-fidelity and omits loop stability, ESR/material effects,
  current-limit behavior, thermal behavior, PSRR/noise, and startup timing.
- The ESP32-S3-WROOM-1U-N16R8 vendor component pack now has a source-backed
  generated SPICE observation face for preliminary module supply, EN release,
  GPIO0/GPIO46 boot-strap, USB D-/D+ line-state, and TXD0 idle checks. Its
  direct-open GUI fixture keeps USB PHY, RF, firmware, peak-current, thermal,
  and EMC behavior explicitly out of scope.
- The ESP32-WROOM-32E vendor component pack now has a source-backed generated
  SPICE observation face for preliminary module supply, EN release,
  GPIO0/GPIO2 boot-strap, TXD0 idle, and RXD0 high-impedance connectivity
  checks. Its direct-open GUI fixture keeps RF, firmware, ROM serial protocol,
  flash/PSRAM mux safety, peak-current, thermal, and EMC behavior explicitly
  out of scope.
- The Sipeed LicheeRV-Nano-W vendor component pack now has a source-backed
  generated SPICE observation face for preliminary 5 V module power, UART0
  TX/RX line-state, motion-enable output, and fault-IRQ input checks. Its
  direct-open GUI fixture keeps Linux boot power transients, internal SoC
  rails, firmware behavior, USB/MIPI/high-speed interfaces, RF/Wi-Fi, exact
  header-numbering sign-off, thermal behavior, and EMC behavior explicitly out
  of scope.
- The Artery AT32F435 motion-core vendor component pack now has a source-backed
  generated SPICE observation face for preliminary 3.3 V VDD, LicheeRV UART,
  motion-enable/fault, CAN, RS-485, and servo PWM enable line-state checks. Its
  direct-open GUI fixture keeps firmware execution, reset/clock timing,
  CAN/RS-485 protocol timing, ADC and motor-control behavior, exact package pin
  assignment, layout, thermal behavior, and EMC behavior explicitly out of
  scope.
- The Artery AT32M416 motor-control vendor component pack now has a
  source-backed generated SPICE observation face for preliminary 3.3 V VDD,
  CAN, six PWM outputs, DRV8323-style enable/fault/SPI lines, current-sense
  nodes, encoder inputs, board enable, and fault output checks. Its direct-open
  GUI fixture keeps firmware execution, reset/clock timing, PWM timer
  waveforms, ADC conversion/current reconstruction, FOC loops, dead-time,
  package assignment, gate-drive physics, layout, thermal behavior, and EMC
  behavior explicitly out of scope.
- The ST STM32L431 and UM STM32L4 resident MCU packs now have saved ST source
  provenance and a source-backed generated SPICE observation face for
  preliminary VDD, NRST, BOOT0, USART1 PA9/PA10, and SWD PA13/PA14
  board-boundary line-state checks. The direct-open GUI fixture keeps firmware
  execution, oscillator accuracy, reset timing, UART/SWD protocol timing, flash
  programming effects, exhaustive package mapping, layout, thermal behavior,
  and EMC behavior explicitly out of scope.
- The JST XH and VH connector packs now have source-backed generated SPICE
  observation faces for mated-contact voltage drop using the datasheet
  20 mOhm post-test/environment contact-resistance maximum per contact. Their
  direct-open GUI fixtures bind explicit load-side connector pins and keep
  cable resistance, crimp quality, temperature rise, retention, vibration,
  signal integrity, and harness qualification explicitly out of scope.
- The Silicon Labs CP2102N vendor component pack now has a datasheet-backed
  generated SPICE observation face for VREGIN-to-VDD regulator and UART
  output-state checks. Its VREGIN/VDD/VIO ranges, regulator output-current
  class, reset pull-up note, and UART threshold metadata remain source-backed,
  while the transient face stays explicitly reduced-fidelity and omits USB PHY
  behavior, enumeration, baud timing, oscillator accuracy, suspend behavior,
  regulator stability, modem-line transistor circuitry, and final I/O
  injection-current sign-off.
- The FTDI FT232R vendor component pack now has a source-backed generated
  SPICE observation face for VCC-to-3V3OUT regulator and UART output-state
  checks. Its VCC/VCCIO/3V3OUT ranges, regulator output-current class,
  reset-pin note, CBUS configuration note, and UART threshold metadata remain
  source-backed, while the transient face stays explicitly reduced-fidelity and
  omits USB PHY behavior, enumeration, EEPROM/CBUS programming, baud timing,
  oscillator accuracy, suspend behavior, regulator stability, modem-line
  transistor circuitry, and final I/O injection-current or thermal sign-off.
- The WCH CH347 vendor component pack now has a source-backed generated SPICE
  observation face for a USB-JTAG/debug bridge line-state workflow. Its 3.3 V
  VCC range, current class, 5 V-tolerant input notes, UART1/JTAG pin roles, and
  output-high metadata remain source-backed, while the transient face stays
  explicitly reduced-fidelity and omits USB PHY behavior, enumeration,
  driver-mode selection, UART/JTAG timing, TAP state, external-clock accuracy,
  and final I/O injection-current sign-off.
- The generic CMSIS-DAP SWD probe pack now has a source-backed generated SPICE
  observation face for VTREF-referenced SWD line-state workflows. Its SWD/JTAG
  pin-role metadata and default SWD setup behavior are sourced from Arm
  CMSIS-DAP repository documentation, while the transient face stays explicitly
  reduced-fidelity and omits USB transport, SWD turnaround/protocol transfers,
  target-specific voltage limits, trace bandwidth, and probe-vendor drive
  strength.
- The TI TXS0108E vendor component pack now has a datasheet-backed generated
  SPICE observation face for an enabled A1-to-B1 mixed-voltage level-shift
  workflow. Its supply ranges, `VCCA <= VCCB` constraint, OE guidance, and
  channel pin roles remain source-backed, while the transient face stays
  explicitly reduced-fidelity and omits automatic bidirectional sensing,
  one-shot edge accelerators, pass-gate analog behavior, high-speed timing, and
  signal-integrity sign-off.
- The TI TPD2EUSB30 vendor component pack now has a datasheet-backed generated
  SPICE observation face for normal-operation USB D+/D- standoff checks with
  the source-backed `0.7 pF` IO-to-ground capacitance load. Its static clamp
  metadata still covers 5.5 V standoff and capacitance-budget review, while the
  transient face explicitly omits IEC ESD pulse clamping, dynamic snapback,
  leakage over temperature, USB eye margin, and layout sign-off.
- The Nexperia PRTR5V0U2X vendor component pack now has a datasheet-backed
  generated SPICE observation face for normal-operation rail-to-rail USB ESD
  checks. Its static clamp metadata still covers VCC reference validation,
  5.5 V standoff, and capacitance-budget review, while the transient face
  applies source-backed IO/VCC capacitance loads and explicitly omits IEC ESD
  pulse clamping, rail-to-rail snapback dynamics, leakage over temperature,
  USB eye margin, differential impedance, and layout sign-off.
- The TI ESD2CAN24-Q1 vendor component pack now has a datasheet-backed
  generated SPICE observation face for normal-operation CANH/CANL standoff
  checks with source-backed `3 pF` line capacitance. Its static clamp metadata
  still covers CAN ESD presence and ground-reference review, while the
  transient face explicitly omits ISO 7637, ISO 10605, IEC ESD pulse clamping,
  surge energy, CAN signal integrity, cable-harness behavior, route placement,
  stub length, and final layout sign-off.
- The TI TCAN3413 vendor component pack now has a datasheet-backed generated
  SPICE observation face for 3.3 V VCC/VIO, TXD/STB state, RXD state, and
  CANH/CANL dominant line-state checks. Its static metadata still covers the
  3.0 V to 3.6 V VCC range, 1.7 V to 3.6 V VIO range, MCU-side logic
  thresholds, 5 Mbps CAN FD class, 8 Mbps light-bus class, and `+/-58 V`
  bus-fault class, while the transient face explicitly omits CAN termination,
  stub length, common-mode range, cable behavior, CAN FD timing, EMC,
  bus-fault energy, and final signal-integrity sign-off.
- The TI ESDS552 vendor component pack now has a datasheet-backed generated
  SPICE observation face for normal-operation RS-485/RS-422 A/B standoff checks
  with source-backed `11 pF` maximum line capacitance. Its static clamp metadata
  still covers RS-485 protection presence and ground-reference review, while
  the transient face explicitly omits IEC 61000-4-2, IEC 61000-4-5, ESD/surge
  pulse clamping, surge response, common-mode stress, bus termination,
  cable-harness behavior, signal integrity, route placement, stub length, and
  final layout sign-off.
- The TI THVD1450 vendor component pack now has a datasheet-backed generated
  SPICE observation face for 3.3 V VCC, DI/DE/RE_N state, RO state, and A/B
  line-state checks. Its static metadata still covers the 3.0 V to 5.5 V supply
  range, MCU-side logic thresholds, 50 Mbps class, 1/8 unit-load receiver, up
  to 256 nodes, and TI product-page ESD class, while the transient face
  explicitly omits RS-485 termination, failsafe biasing, common-mode range,
  cable behavior, timing, EMC, ESD/fault energy, and final signal-integrity
  sign-off.
- The TI TPS54331-5V vendor component pack now has a datasheet-backed
  generated SPICE observation face for VIN/EN/VSENSE rail checks. Its static
  input range, 3 A output-current class, switching-frequency class, and
  5 V configured output metadata remain source-backed, while the transient face
  stays explicitly reduced-fidelity and omits PH/BOOT switching, compensation,
  inductor ripple/current, output ripple, current limit, Eco-mode, startup
  timing, thermal behavior, layout, EMI, and loop-stability sign-off.
- The TI TPS62162-3.3 vendor component pack now has a datasheet-backed
  generated SPICE observation face for VIN/EN/VOS rail checks. Its 3.0 V to
  17 V input range, 1 A output-current class, fixed 3.3 V output metadata, and
  static support-component checks remain source-backed, while the transient face
  stays explicitly reduced-fidelity and omits SW switching, PG behavior,
  DCS-Control dynamics, inductor ripple/current, output ripple, current limit,
  thermal behavior, layout, EMI, and loop-stability sign-off.
- The TI TPS63802-3.3 vendor component pack now has a datasheet-backed
  generated SPICE observation face for VIN/EN/VOUT rail checks. Its 1.3 V to
  5.5 V operating range, 1.8 V startup floor, 2 A output-current class, 3.3 V
  configured output metadata, and static L1-L2 inductance checks remain
  source-backed, while the transient face stays explicitly reduced-fidelity and
  omits buck-boost switching, FB dynamics, MODE/PG behavior, inductor current,
  output ripple, thermal behavior, layout, EMI, and loop-stability sign-off.
- The TI TPS61023-5V vendor component pack now has a datasheet-backed
  generated SPICE observation face for VIN/EN/VOUT rail checks. Its 0.5 V to
  5.5 V operating range, 1.8 V startup floor, 5 V configured output metadata,
  and static input-inductor/support-capacitor checks remain source-backed,
  while the transient face stays explicitly reduced-fidelity and omits boost
  switching, FB-loop dynamics, inductor current/ripple, output ripple, valley
  current-limit behavior, thermal behavior, layout, EMI, and loop-stability
  sign-off.
- The TI TPS2121 vendor component pack now has a datasheet-backed generated
  SPICE observation face for selected IN1/IN2 to OUT rail checks. Its 2.8 V to
  22 V input range, 4.5 A current class, reverse-blocking metadata, and static
  selected-input power-mux checks remain source-backed, while the transient face
  stays explicitly reduced-fidelity and omits priority threshold comparators,
  switchover droop, reverse-current magnitude, ILIM-derived current limit,
  soft-start timing, thermal behavior, status output, layout, and final mux
  sign-off.
- The TI TPS2115A vendor component pack now has a datasheet-backed generated
  SPICE observation face for selected IN1/IN2 to OUT rail checks. Its 2.8 V to
  5.5 V input range, 1 A output-current class, reverse/cross-conduction
  blocking metadata, and static selected-input power-mux checks remain
  source-backed, while the transient face stays explicitly reduced-fidelity and
  omits EN/D0/D1/VSNS autoswitch truth-table behavior, switchover droop,
  reverse-current magnitude, ILIM-derived current limit, thermal behavior,
  package limits, layout, and final mux sign-off.
- The TI DRV8323 gate-driver pack now has a source-backed generated SPICE
  observation face for VM/DVDD/ENABLE, nFAULT/SDO output-state, and SOA/SOB/SOC
  current-sense output presence checks. Its 6 V to 60 V motor-supply range,
  3 V to 5.5 V DVDD range, 2 V logic input-high threshold, digital-output
  metadata, and three current-sense amplifier metadata remain source-backed,
  while the transient face stays explicitly reduced-fidelity and omits MOSFET
  gate-drive strength, half-bridge switching, charge-pump/bootstrap behavior,
  dead time, SPI register/protection behavior, current-sense gain/offset/noise,
  motor dynamics, layout, EMI, and thermal sign-off.
- The NXP PCA9685 PWM-driver pack now has a source-backed generated SPICE
  observation face for VDD/OE, I2C idle-line state, and four representative
  low-load PWM outputs. Its 2.3 V to 5.5 V VDD range, Fast-mode Plus I2C role,
  12-bit PWM controller role, and output-current class remain source-backed,
  while the transient face stays explicitly reduced-fidelity and omits I2C
  protocol/register behavior, oscillator tolerance, phase staggering,
  LED/servo output current, pull-up rise time, servo dynamics, disabled-output
  high-Z behavior, thermal behavior, and final PWM timing sign-off.
- The TDK InvenSense ICM-42688-P IMU pack now has a source-backed generated
  SPICE observation face for VDD/VDDIO rails, host-driven SPI line states, SDO
  output state, and INT1 interrupt-output state. Its 1.71 V to 3.6 V rail
  ranges, SPI thresholds, and output metadata remain source-backed, while the
  transient face stays explicitly reduced-fidelity and omits sensor dynamics,
  register protocol, FIFO behavior, sampling timing, noise, bias stability,
  vibration, package stress, layout coupling, and final SPI timing sign-off.
- The onsemi NL27WZ17 logic-buffer pack now has a source-backed generated
  SPICE observation face for VCC, 1A/2A input-state, and non-inverted 1Y/2Y
  output-state checks. Its 1.65 V to 5.5 V supply range, SC-88/SOT-363 pinout,
  and dual non-inverting Schmitt-trigger buffer role remain source-backed,
  while the transient face stays explicitly reduced-fidelity and omits Schmitt
  thresholds/hysteresis, propagation delay, output drive strength, capacitive
  loading, signal integrity, and final switching sign-off.
- The TI TLV803EA29 reset-supervisor pack now has a datasheet-backed generated
  SPICE observation face for active-low open-drain threshold behavior with an
  external pull-up. Datasheet delay and threshold metadata remain available for
  static reset timing suggestions, while the transient face stays explicitly
  reduced-fidelity.
- The TI TPS22918 load-switch pack now has a datasheet-backed generated SPICE
  observation face for active-high enabled load-path behavior. Datasheet
  voltage/current/ON-threshold metadata remains available for static
  power-tree checks, while the transient face stays explicitly reduced-fidelity
  and omits CT, QOD, reverse-current, current-limit, inrush, and thermal
  sign-off behavior.
- The TI TPS25948 eFuse/load-switch pack now has a source-backed generated
  SPICE observation face for active-high enabled 12 V protected-rail behavior.
  Datasheet voltage/current-limit/on-resistance and reverse-current-blocking
  metadata remains available for static power-switch checks, while the
  transient face stays explicitly reduced-fidelity and omits dVdt, ILM/ITIMER,
  FLT/SPLYGD, OVLO, RCBCTRL, reverse-current dynamics, inrush, and thermal
  sign-off behavior.
- The TI TPS24751 + CSD17501Q5A hot-swap pack now has a source-backed
  generated SPICE observation face for active-high enabled 12 V
  reverse-blocking protected-rail behavior. Datasheet voltage/current-limit/
  path-resistance and disabled-state reverse-current-blocking metadata remains
  available for static power-switch checks, while the transient face stays
  explicitly reduced-fidelity and omits TIMER/PROG/SET fault timing, FLTb/PGb
  outputs, external-FET gate-drive dynamics, reverse-current dynamics, inrush
  accuracy, and thermal sign-off behavior.
- The Microchip MCP73831-2 charger pack now has a datasheet-backed generated
  SPICE observation face for 4.2 V PROG-programmed constant-current/
  constant-voltage behavior. Datasheet charge-current, PROG equation, input,
  and battery-voltage metadata remain available for static charger checks,
  while the transient face stays explicitly reduced-fidelity and omits
  preconditioning, termination, STAT, thermal, timer, battery-chemistry, cell
  safety, and final charger sign-off behavior.
- The TI BQ24075 power-path charger pack now has a datasheet-backed generated
  SPICE observation face for adapter-fed OUT rail and ISET-programmed BAT
  current behavior. Datasheet charge-current, ISET equation, IN/BAT/OUT
  voltage metadata, and static power-tree limits remain source-backed, while
  the transient face stays explicitly reduced-fidelity and omits DPPM,
  supplement mode, ILIM/EN current-limit derivation, status pins, termination,
  thermal, timer, battery-chemistry, cell safety, and final charger sign-off
  behavior.
- The TI BQ25798 buck-boost/NVDC charger pack now has a datasheet-backed
  generated SPICE observation face for adapter/SYS/BAT wiring and
  host-programmed charge-current behavior. The generated subcircuit can map
  Board IR component parameters such as `programmed_charge_current_A` into
  SPICE instance parameters, while the transient face stays explicitly
  reduced-fidelity and omits buck-boost switching, DPM/MPPT, supplement mode,
  BATFET dynamics, register sequencing, thermal regulation, timers,
  battery-chemistry, cell safety, and final charger sign-off behavior.
- GUI generated run-setup creation and generated component inclusion infer
  required `simulation.spice.model_path` files from active component-library
  metadata, resolve them the same way validation does, and write SHA-256-pinned
  `analog.model_files` entries automatically when missing.
- The GUI component model browser marks `simulation.spice`-backed library parts
  as SPICE-ready, makes that metadata searchable, and shows model type, model
  name, model file, provenance, pin order, and the first operating note before
  the user places the part on the schematic.
- For a selected placed SPICE-ready component, the GUI can create a generated
  observation preset that includes the board context, binds ground, voltage
  probes the component's non-ground pin nets, infers required model files, and
  adds model-aware default checks for regulator/load-switch/power-mux output
  voltage limits, comms output voltage states, or pulse-driven reset-supervisor,
  op-amp follower, and comparator output behavior when the needed metadata and
  surrounding stimulus/reference topology are present.
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
