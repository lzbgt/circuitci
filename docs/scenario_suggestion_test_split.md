# Scenario Suggestion Test Split

## Purpose

Scenario suggestion coverage primarily lives in `tests/scenario_suggestions_cli.rs`
instead of `tests/backdrive_cli.rs`. Manufacturing-heavy suggestion coverage
that would push the primary file toward the source line-count guard lives in
`tests/scenario_suggestions_manufacturing_cli.rs`. The split keeps behavioral
validation tests focused while giving automatic agent-facing validation
suggestions their own integration-test home.

## Contract

- The split is mechanical: CLI arguments, fixture paths, schema validation, and
  assertion coverage remain unchanged.
- Suggestion tests must validate `schemas/scenario_suggestion_report.schema.json`
  because these reports are intended for downstream agents.
- Runnable and non-runnable suggestions should both be asserted explicitly so the
  tool does not silently invent missing observations.
- Manufacturing suggestion tests should include preset-backed runnable templates,
  threshold-required non-runnable templates, assembly/footprint evidence checks,
  pin-1 orientation checks, stencil pitch checks, route-physics suggestions,
  thermal suggestions, and RF antenna keepout/feed/matching/measured-performance
  suggestions. Fabrication release evidence often proves geometry exists before
  it proves every process limit.

## Module Ownership

- `src/scenario_suggestions/manufacturing.rs` owns the manufacturing suggestion
  dispatcher plus fabrication, solder-mask, solder-paste, assembly, and pin-1
  suggestion discovery.
- `src/scenario_suggestions/manufacturing/route_physics.rs` owns controlled
  impedance, adjacent-plane, slot-crossing, and stitching-via suggestion
  discovery.
- `src/scenario_suggestions/manufacturing/thermal.rs` owns reviewed thermal
  copper, via, package, measured-temperature, and environment suggestion
  discovery.
- `src/scenario_suggestions/manufacturing/rf_antenna.rs` owns reviewed RF
  antenna keepout, feed-path, matching-topology, measured-performance, sweep,
  limit, and measurement-condition suggestion discovery.
