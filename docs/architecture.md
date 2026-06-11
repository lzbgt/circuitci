# CircuitCI Architecture

CircuitCI is a headless-first validation runtime for embedded board designs. It imports board artifacts, binds component models, runs scenario-driven checks, and emits machine-readable and human-readable reports. The core product boundary is validation before PCB fabrication, not schematic capture or PCB layout.

The engine backbone is Rust. C/C++ backends are acceptable for solver integrations, but the in-repository runtime, CLI, schemas, validation rules, and reports should be Rust-first. Python is allowed only for investigation scripts, data conversion experiments, or disposable research tooling.

## Goals

- Keep the engine generic: STM32, ESP32, 555 timers, STM8, C51/STC-class MCUs, CH340/CP210x/FT232 bridges, passives, MOSFETs, sensors, and regulators are library data, not core engine assumptions.
- Make validation deterministic enough for CI and AI-agent repair loops.
- Prefer a small verified vertical slice over a broad unverified simulator skeleton.
- Keep every result traceable to board IR, component model metadata, scenario inputs, and rule IDs.

## Runtime Pipeline

```text
project.yaml or imported EDA artifact
  -> Board Graph IR
  -> component model binding
  -> scenario execution
  -> validation rules
  -> report.json + report.md + waveform artifacts
```

The MVP implements the pipeline for YAML project files first. KiCad, EasyEDA, Altium, SPICE, and JITX importers are future adapter layers that should produce the same Board Graph IR.

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

## MVP Execution Model

The first executable slice intentionally avoids full analog solving. It uses declared power-domain state and electrical pin metadata to validate the `GPIO_BACKDRIVE` condition:

```text
powered output pin drives an unpowered input pin above injection-current limit
```

This proves the IR, model binding, scenario, validation, and report contracts before adding ngspice/Xyce integration.

## Library Contract

Library paths in a project are package roots. The loader recursively discovers `*.model.yaml` files under each root. Every file must contain one component model.

Rules:

- `board.components.*.model` is an exact `component_id` match for the MVP.
- duplicate `component_id` values are binding errors.
- version selection is not implicit in the MVP; a future schema can add semver-qualified model references.
- unreadable or malformed model files produce report findings.

Chip and IC support must arrive as library packs. For example, STM32L4 support for the acceptance demo should be a vendor model pack plus fixtures; it must not add STM32L4-specific branches to the validation engine. The same engine path must be able to load future packs for ESP32, STM32F1/F4/L1/L4, STM8, C51/STC, 555 timers, USB-UART bridges, regulators, and other common embedded parts.

## Future Simulation Kernel

The future mixed-domain kernel should have replaceable adapters:

- analog solver adapter: ngspice first, Xyce later
- digital event solver: scheduled state changes, protocol events, pin modes
- analog/digital bridge: threshold crossings, power-state changes, digital output to analog source mapping
- firmware adapter: Renode or QEMU when firmware-in-loop validation is needed

The CLI and JSON report schema must remain stable as solver fidelity increases.

## Design Constraints

- A validation rule must never silently change pass criteria to make examples pass.
- A component model must declare model quality and unsupported use cases.
- Reports must include low-confidence or unmodeled areas.
- Backends must fail with actionable diagnostics instead of silent crashes.
