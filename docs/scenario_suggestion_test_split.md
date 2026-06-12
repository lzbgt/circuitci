# Scenario Suggestion Test Split

## Purpose

Scenario suggestion coverage now lives in `tests/scenario_suggestions_cli.rs`
instead of `tests/backdrive_cli.rs`. The split keeps behavioral validation tests
focused while giving automatic agent-facing validation suggestions their own
integration-test home.

## Contract

- The split is mechanical: CLI arguments, fixture paths, schema validation, and
  assertion coverage remain unchanged.
- Suggestion tests must validate `schemas/scenario_suggestion_report.schema.json`
  because these reports are intended for downstream agents.
- Runnable and non-runnable suggestions should both be asserted explicitly so the
  tool does not silently invent missing observations.
