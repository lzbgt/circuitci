# CircuitCI Architecture

CircuitCI is a headless-first validation runtime for embedded board designs. It imports board artifacts, binds component models, runs scenario-driven checks, and emits machine-readable and human-readable reports. The core product boundary is validation before PCB fabrication, not schematic capture or PCB layout.

The engine backbone is Rust. C/C++ backends are acceptable for solver integrations, but the in-repository runtime, CLI, schemas, validation rules, and reports should be Rust-first. Python is allowed only for investigation scripts, data conversion experiments, or disposable research tooling.

## Goals

- Build an agent-facing board-validation system: findings must be deterministic,
  machine-readable, linked to measured evidence and limits, and useful for
  repair/rerun loops.
- Keep the engine generic: STM32, ESP32, 555 timers, STM8, C51/STC-class MCUs, CH340/CP210x/FT232 bridges, passives, MOSFETs, sensors, and regulators are library data, not core engine assumptions.
- Make validation deterministic enough for CI and AI-agent repair loops.
- Prefer a small verified vertical slice over a broad unverified simulator skeleton.
- Keep every result traceable to board IR, component model metadata, scenario inputs, and rule IDs.

## Runtime Pipeline

```text
project.yaml, imported KiCad schematic/netlist, or imported SPICE deck
  -> Board Graph IR
  -> component model binding
  -> scenario execution
  -> validation rules and optional SPICE-class analog solver
  -> report.json + report.md + waveform artifacts
```

The current runtime accepts hand-authored Board IR YAML, SPICE decks through
`import-spice`, KiCad generic XML netlists through `import-kicad-netlist`, and
native `.kicad_sch` schematics through `import-kicad-schematic`. It can also
enrich an imported Board IR project with KiCad `.kicad_pcb` layout evidence,
including component placements, routed segment/via geometry, and copper-zone
outlines, through
`import-kicad-pcb`. Importers are adapters into the same Board IR shape;
validation and reporting do not branch on the original EDA source after import.
EasyEDA, Altium, and JITX remain future adapter layers.

## Core Modules

| Module | Responsibility |
| --- | --- |
| `board_ir` | Parse and validate board projects, components, nets, power domains, and declared scenarios. |
| `library` | Load component model YAML files and expose pin/model metadata to validation. |
| `scenarios` | Normalize scenario definitions into events and selected validation checks. |
| `validation` | Run deterministic rules and produce typed findings. |
| `reports` | Serialize stable JSON and Markdown reports. |
| `cli` | Provide agent-friendly commands. |

## Internal Contracts

The first Rust implementation uses these data handoffs:

| Type | Owner | Purpose |
| --- | --- | --- |
| `BoardProject` | `board_ir` | Parsed YAML project with components, nets, libraries, and scenarios. |
| `ComponentLibrary` | `library` | Deterministic map from exact `component_id` to one loaded model. |
| `BoundBoard` | `library` | Board plus resolved component models and binding diagnostics. |
| `ScenarioPlan` | `scenarios` | Normalized scenario checks and pin-state assumptions. |
| `Finding` | `validation` | Typed diagnostic with rule ID, severity, scenario, endpoints, measurements, limits, and fixes. |
| `ValidationReport` | `reports` | Stable JSON/Markdown report assembled from findings and limitations. |

Binding diagnostics are findings with IDs such as `MODEL_NOT_FOUND` and `PIN_NOT_DECLARED`. Validation rules should not repeat binding checks.

## Rust Workspace Shape

The first implementation uses one Rust package with clear internal modules. When module boundaries stabilize, it can split into crates without changing CLI behavior:

```text
circuitci/
  src/
    board_ir/
    library/
    scenarios/
    validation/
    reports/
    cli/
```

Future crate split:

```text
crates/
  circuitci-core
  circuitci-validation
  circuitci-report
  circuitci-cli
  circuitci-sim
  circuitci-gui
```

## Execution Model

CircuitCI combines deterministic board-rule validation with solver-backed
analog validation. Behavioral rules use declared board/model metadata and
scenario events. For example, `GPIO_BACKDRIVE` uses declared power-domain state
and electrical pin metadata to validate:

```text
powered output pin drives an unpowered input pin above injection-current limit
```

`analog_transient` scenarios provide the physical waveform path. They can run
file-backed imported SPICE decks or generated Board IR netlists through a
SPICE-class backend. The supported mature backend path is ngspice, either via
the external `ngspice` binary or dynamically loaded `libngspice`; unavailable
or misconfigured backends fail closed with report findings instead of fabricated
passes. Generated semiconductor scenarios can also emit datasheet operating
limit findings for MOSFET, BJT, and diode ratings, including derating, qualified
pulse current, and digitized MOSFET SOA evidence when metadata is present.

## Library Contract

Library paths in a project are package roots. The loader recursively discovers `*.model.yaml` files under each root. Every file must contain one component model.

Rules:

- `board.components.*.model` is an exact `component_id` match.
- duplicate `component_id` values are binding errors.
- version selection is not implicit; a future schema can add semver-qualified model references.
- unreadable or malformed model files produce report findings.

Chip and IC support must arrive as library packs. For example, STM32L4 support for the acceptance demo should be a vendor model pack plus fixtures; it must not add STM32L4-specific branches to the validation engine. The same engine path must be able to load future packs for ESP32, STM32F1/F4/L1/L4, STM8, C51/STC, 555 timers, USB-UART bridges, regulators, and other common embedded parts.

## Simulation Kernel

The mixed-domain kernel uses replaceable adapters:

- analog solver adapter: external `ngspice`, embedded `libngspice`, and a
  fail-closed Xyce placeholder,
- digital event solver: scheduled state changes, protocol events, and pin
  modes encoded in scenarios,
- analog/digital bridge: explicit generated stimuli and probes; threshold
  crossing automation remains future work,
- firmware adapter: QEMU-backed functional firmware-in-loop validation with
  explicit board-facing pin observations; Renode remains a fail-closed future
  adapter. Firmware models should expose firmware-visible peripherals and
  package pin behavior, not internal MCU transistor implementation.

The CLI and JSON report schema must remain stable as solver fidelity increases.

The gap between this architecture and broad "verify any common IoT board"
coverage is tracked in
[common_iot_board_readiness_gaps.md](common_iot_board_readiness_gaps.md).

## Design Constraints

- A validation rule must never silently change pass criteria to make examples pass.
- A component model must declare model quality and unsupported use cases.
- Reports must include low-confidence or unmodeled areas.
- Backends must fail with actionable diagnostics instead of silent crashes.
- MCU internals should be modeled as functional black boxes. CircuitCI cares
  about the externally observable pin behavior, firmware-visible peripheral
  state, reset/boot sequencing, and electrical limits that affect the board.
