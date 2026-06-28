# Scenario Suggestions Module Split

## Purpose

`src/scenario_suggestions.rs` was approaching the repository 2000-line source
limit after adding power-tree, reset, boot strap, UART, backdrive,
interface-protection, clock, and regulator evidence generation. The split keeps
behavior unchanged while preserving room for new agent-facing board-validation
suggestions.

## Module Ownership

- `src/scenario_suggestions.rs` owns suggestion orchestration and recognition
  logic over bound Board IR: power-tree checks, clocks, reset supervisors,
  regulators, shared passive evidence collection, and cross-family helper
  functions.
- `src/scenario_suggestions/backdrive.rs` owns GPIO backdrive risk recognition,
  runtime-evidence gating, duplicate detection, and `GPIO_BACKDRIVE` template
  construction.
- `src/scenario_suggestions/control_line.rs` owns
  `board.runtime.control_line_sequences[]` projection, duplicate detection, and
  `CONTROL_LINE_RELEASE_SEQUENCE` template construction.
- `src/scenario_suggestions/reset_boot.rs` owns reset-release, boot-strap, boot
  strap bias, and UART bootloader sync suggestions, including RC reset evidence,
  runtime reset timing, datasheet-backed reset-supervisor timing, direct strap
  state proof, and duplicate detection for reset/boot scenarios.
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
- `src/scenario_suggestions/manufacturing.rs` owns fabrication and assembly
  suggestion templates, including process-preset suggestions, manufacturing
  metadata-gated templates, assembly/footprint evidence alignment, pin-1 marker
  templates, RF antenna keepout/feed-path/measured-performance templates from
  reviewed layout/measurement constraints and imported copper, route, pad, and
  placement evidence, and manufacturing suggestion orchestration.
- `src/scenario_suggestions/manufacturing/thermal.rs` owns reviewed
  thermal-copper area, via/stackup, via-plating/plating-thickness/barrel
  cross-section, derating-environment,
  package-temperature, and measured-temperature templates from board
  manufacturing metadata, model metadata, and imported copper, route-via,
  stackup, and drill evidence.
- `src/scenario_suggestions/manufacturing/route_physics.rs` owns
  stackup-aware adjacent-plane return-path and reference-plane slot-crossing
  discovery from explicit route/stackup/zone evidence, stitching-via discovery
  from explicit route/stackup/via evidence, controlled-impedance geometry
  discovery from reviewed board manufacturing targets plus imported route
  width/gap evidence, and controlled-impedance stackup-evidence discovery from
  reviewed targets plus explicit copper/dielectric stackup metadata.
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
