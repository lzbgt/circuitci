# CircuitCI

Agent-native embedded board validation runtime.

CircuitCI is a headless validation tool for embedded and IoT circuit designs. It
imports board artifacts, binds component models, runs deterministic validation
scenarios, and emits machine-readable reports that AI agents and engineers can
use before PCB fabrication.

It is not a schematic editor, PCB layout editor, or full EDA suite. The goal is
CI-style feedback for board designs: catch power, reset, boot, interface,
layout-evidence, and firmware-facing mistakes early enough that an agent or
engineer can fix the design and rerun validation.

## What It Does

- Parses Board IR YAML projects and component model libraries.
- Imports SPICE decks, KiCad XML netlists, native KiCad schematics, KiCad PCB
  placement/route evidence, JLC/EasyEDA BOM/CPL assembly evidence, Gerber
  board-outline/copper/solder-mask/solder-paste evidence, and Excellon/NC drill
  evidence.
- Suggests missing validation scenarios from board/model evidence.
- Validates power trees, regulator constraints, reset supervisors, boot straps,
  GPIO backdrive paths, I/O voltage compatibility, USB protection, USB route
  geometry, clocks, serial bootloaders, control-line sequences, and selected
  SPICE-backed analog checks.
- Writes stable JSON/Markdown reports with measurements, limits, severity, and
  suggested fixes.
- Keeps component behavior in data-driven model packs instead of hardcoding one
  MCU, USB-UART bridge, regulator, or board family into the engine.

## Current Scope

CircuitCI is currently an early Rust CLI focused on deterministic pre-fab board
validation. It already includes fixtures and model packs for common IoT-board
building blocks such as MCU boot/reset behavior, USB connectors, USB ESD
protection, LDO regulators, load switches, power muxes, battery chargers,
reset supervisors, USB-UART bridges, and level shifters.

The runtime deliberately reports limitations for areas it does not prove, such
as full USB PHY behavior, RF/antenna performance, complete PCB signal integrity,
thermal sign-off, arbitrary KiCad DRC semantics, and vendor-silicon internals.
See [docs/limitations.md](docs/limitations.md) and
[docs/common_iot_board_readiness_gaps.md](docs/common_iot_board_readiness_gaps.md).
For the prioritized backlog that makes CircuitCI more useful to board-validation
agents, see
[docs/agent_validation_usefulness_backlog.md](docs/agent_validation_usefulness_backlog.md).

## Install

Prerequisites:

- Rust toolchain with edition 2024 support.
- Optional: `ngspice` or `libngspice` for SPICE-backed analog scenarios.

Build:

```bash
cargo build --release
```

This creates the CLI at `target/release/circuitci`. Add it to your shell path
or call it directly from the repository:

```bash
export PATH="$PWD/target/release:$PATH"
circuitci --help
```

Run tests:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
```

Optional native desktop GUI:

```bash
cargo run --features gui --bin circuitci-gui
```

The GUI is a Rust `egui`/`eframe` desktop shell with EDA-style workflow stages
for project loading, sketch/model review, scenario suggestions, validation,
simulation artifact observation, and report viewing. It uses the same engine as
the CLI and does not compile into default CLI builds. See
[docs/gui_frontend.md](docs/gui_frontend.md).

## CLI

Validate a Board IR project:

```bash
circuitci validate examples/good_power_tree_board/project.yaml \
  --output out/good_power_tree
```

Write a scenario suggestion artifact:

```bash
circuitci suggest-scenarios examples/scenario_suggestions_usb_connector_protection/project.yaml \
  --output out/scenario_suggestions.yaml
```

Add profile-coverage remediation templates for missing `iot_basic_v0` core
checks:

```bash
circuitci suggest-scenarios examples/good_backdrive_fixed_board/project.yaml \
  --profile iot_basic_v0 \
  --output out/profile_suggestions.yaml
```

Import a KiCad schematic:

```bash
circuitci import-kicad-schematic path/to/root.kicad_sch \
  --mapping path/to/circuitci.kicad-map.yaml \
  --output out/imported.project.yaml
```

Enrich an imported project with KiCad PCB placement and route evidence:

```bash
circuitci import-kicad-pcb path/to/board.kicad_pcb \
  --project out/imported.project.yaml \
  --output out/imported_with_layout.project.yaml
```

Import JLC/EasyEDA assembly BOM and placement evidence:

```bash
circuitci import-jlc-assembly \
  --bom path/to/bom.csv \
  --placement path/to/placement.csv \
  --output out/imported_assembly.project.yaml
```

Inspect an EasyEDA Pro `.eprj2` SQLite project envelope:

```bash
circuitci inspect-easyeda-pro path/to/project.eprj2 \
  --output out/easyeda_pro_inspection.md
```

Append EasyEDA/JLC flying-probe pad and net evidence:

```bash
circuitci import-easyeda-flying-probe path/to/FlyingProbeTesting.json \
  --project out/imported_assembly.project.yaml \
  --output out/imported_with_probe_pads.project.yaml
```

Enrich an imported project with Gerber board-outline evidence:

```bash
circuitci import-gerber-outline path/to/Gerber_BoardOutlineLayer.GKO \
  --project out/imported_assembly.project.yaml \
  --output out/imported_with_outline.project.yaml
```

Append Gerber copper evidence:

```bash
circuitci import-gerber-copper path/to/Gerber_TopLayer.GTL \
  --project out/imported_with_outline.project.yaml \
  --output out/imported_with_copper.project.yaml
```

Append Gerber solder-mask evidence:

```bash
circuitci import-gerber-solder-mask path/to/Gerber_TopSolderMaskLayer.GTS \
  --project out/imported_with_copper.project.yaml \
  --output out/imported_with_mask.project.yaml
```

Append Gerber solder-paste evidence:

```bash
circuitci import-gerber-solder-paste path/to/Gerber_TopPasteMaskLayer.GTP \
  --project out/imported_with_mask.project.yaml \
  --output out/imported_with_paste.project.yaml
```

Append Excellon/NC drill evidence:

```bash
circuitci import-excellon-drill path/to/Drill_PTH_Through.DRL \
  --project out/imported_with_paste.project.yaml \
  --output out/imported_with_drills.project.yaml

circuitci import-excellon-drill path/to/Drill_PTH_Through_Via.DRL \
  --project out/imported_with_drills.project.yaml \
  --output out/imported_with_drills_and_vias.project.yaml
```

Attach explicit order/manufacturing metadata when those values are known from
the fabrication order, stencil order, or another reviewed process record:

```bash
circuitci set-manufacturing-metadata out/imported_with_drills_and_vias.project.yaml \
  --output out/imported_with_order_metadata.project.yaml \
  --stencil-thickness-mm 0.10 \
  --min-drill-edge-clearance-mm 0.50 \
  --min-slot-edge-clearance-mm 0.50 \
  --min-paste-area-ratio 0.70 \
  --max-paste-area-ratio 1.00 \
  --min-solder-paste-spacing-mm 0.15 \
  --source jlc_order_metadata
```

Or import the same reviewed facts from a source CSV while preserving row-level
evidence in a JSON manifest. Repeated `field=controlled_impedance_net` and
`field=controlled_impedance_pair` rows create or replace reviewed
`board.manufacturing.controlled_impedance` targets, including optional
solder-mask loading metadata, repeated `field=controlled_impedance_coupon`
rows create or replace reviewed fabricator coupon measurement evidence that
validation maps back to matching reviewed board impedance targets, repeated
`field=controlled_impedance_coupon_sample` rows attach reviewed coupon batch
sample measurements under named coupons, and coupon rows can carry reviewed
lot/panel/stackup plus trace layer/width/gap correlation metadata. Repeated
`field=thermal_copper` rows create or replace reviewed
`board.manufacturing.thermal_copper[]` policy entries, and repeated
`field=thermal_measurement` rows append reviewed measured-temperature evidence
under `board.manufacturing.thermal_measurements[]`. Repeated
`field=thermal_package` rows create or replace reviewed component package
thermal metadata under `board.manufacturing.thermal_packages[]`. Repeated
`field=thermal_environment` rows create or replace reviewed operating
environment evidence under `board.manufacturing.thermal_environments[]`.
Repeated `field=thermal_limit` rows create or replace reviewed measured and
package temperature limits under `board.manufacturing.thermal_limits[]`.
Repeated
`field=stackup_layer` rows create or replace reviewed
`board.layout.stackup.layers[]` entries so stackup evidence can travel with
fabrication/order metadata without hand-authored YAML. Repeated
`field=rf_antenna_keepout`, `field=rf_antenna_feed_path`,
`field=rf_antenna_matching_network`, `field=rf_antenna_measurement`, and
`field=rf_antenna_performance_limit` rows create or replace reviewed RF
antenna layout/topology/measurement/limit constraints, including optional
sampled sweep-coverage policy and measurement-condition evidence, under
`board.layout.constraints.rf_antenna`:

```bash
circuitci import-manufacturing-metadata \
  --project out/imported_with_drills_and_vias.project.yaml \
  --metadata order_metadata.csv \
  --output out/imported_with_order_metadata.project.yaml \
  --source jlc_order_metadata \
  --allow-unknown-fields
```

`suggest-scenarios` consumes these Board IR fields to make matching
manufacturing templates runnable without turning order-specific limits into
global process defaults.

Run an acceptance suite:

```bash
circuitci validate-suite suites/um_stm32l4_downloader_acceptance.yaml \
  --output out/acceptance
```

## Project Model

CircuitCI uses a normalized Board IR:

```text
project.yaml / KiCad / SPICE
  -> Board Graph IR
  -> component model binding
  -> scenario execution
  -> validation rules and optional SPICE backend
  -> JSON + Markdown reports
```

Important contracts:

- [docs/current_capabilities.md](docs/current_capabilities.md)
- [docs/board_ir.md](docs/board_ir.md)
- [docs/component_model_contract.md](docs/component_model_contract.md)
- [docs/scenario_language.md](docs/scenario_language.md)
- [docs/fabrication_process_presets.md](docs/fabrication_process_presets.md)
- [docs/manufacturing_metadata_importer.md](docs/manufacturing_metadata_importer.md)
- [docs/gui_frontend.md](docs/gui_frontend.md)
- [docs/report_schema.md](docs/report_schema.md)
- [docs/scenario_suggestions.md](docs/scenario_suggestions.md)

## Repository Layout

```text
src/
  board_ir/       Board IR parsing and normalized project types
  cli/            Command-line entry points
  gui.rs          Optional Rust desktop frontend behind the gui feature
  importers/      KiCad, SPICE, and layout-evidence importers
  library/        Component model loading and binding
  reports/        JSON and Markdown report generation
  validation/     Deterministic rule implementations

libs/
  generic/        Generic component models
  vendor/         Datasheet-backed vendor model packs

models/spice/     SHA-pinned SPICE model cards and generic behavioral
                  macro-model libraries used by generated analog scenarios
examples/         Passing/failing fixtures and importer demos
schemas/          JSON schemas for Board IR and reports
suites/           Acceptance-suite manifests
tests/            CLI and integration regressions
docs/             Engineering contracts and research notes
```

## Validation Philosophy

CircuitCI is conservative:

- A rule fails closed when required evidence is missing.
- Reports distinguish proven failures from low-confidence or unmodeled areas.
- Datasheet-backed component packs record their source artifacts and modeled
  facts.
- Suggestions never invent observations such as reset-release timestamps,
  strap states, protection resistance, load-switch enable state, or board layout
  limits.

## License

MIT. See [LICENSE](LICENSE).
