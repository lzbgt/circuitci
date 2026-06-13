# Scenario Suggestions Module Split

## Purpose

`src/scenario_suggestions.rs` was approaching the repository 2000-line source
limit after adding power-tree, reset, boot strap, UART, backdrive,
interface-protection, clock, and regulator evidence generation. The split keeps
behavior unchanged while preserving room for new agent-facing board-validation
suggestions.

## Module Ownership

- `src/scenario_suggestions.rs` owns suggestion orchestration and recognition
  logic over bound Board IR: power-tree checks, reset/boot templates, UART,
  clocks, reset supervisors, regulators, and passive evidence collection.
- `src/scenario_suggestions/backdrive.rs` owns GPIO backdrive risk recognition,
  runtime-evidence gating, duplicate detection, and `GPIO_BACKDRIVE` template
  construction.
- `src/scenario_suggestions/interface_protection.rs` owns recognition for
  signal-conditioning channels and clamp-only interface-protection templates.
- `src/scenario_suggestions/interface_protection/usb.rs` owns USB connector
  protection coverage, protection-placement, route-geometry, VBUS-route, and
  return-path suggestion template construction from schematic and PCB layout
  evidence.
- `src/scenario_suggestions/interface_protection/usb/edge_evidence.rs` owns USB
  connector footprint serialization, nearest-board-edge evidence, and
  nearest-component clearance evidence helpers.
- `src/scenario_suggestions/interface_protection/usb/mechanical.rs` owns USB
  connector orientation, edge-proximity, body-overhang, component-clearance,
  and entry-clearance suggestion templates plus their duplicate-check
  recognizers.
- `src/scenario_suggestions/interface_protection/usb/route_evidence.rs` owns
  measured USB route, pad-contact, return-path, filled-zone, and ground-contact
  evidence helpers used by USB suggestion templates.
- `src/scenario_suggestions/types.rs` owns the serializable suggestion report
  DTOs that must stay aligned with
  `schemas/scenario_suggestion_report.schema.json`.

## Review Checklist

- Keep schema-visible fields in `types.rs` and update the JSON schema plus
  docs in the same change.
- Keep circuit-recognition behavior in `scenario_suggestions.rs` unless a rule
  family grows large enough to justify its own focused submodule.
- Run `cargo test --test scenario_suggestions_cli` after any suggestion-shape
  or schema change.
