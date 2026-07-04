# Measure Backend Evidence

Status as of 2026-07-04: CircuitCI supports raw reviewed
`measure_statements[]` on ngspice and portable `measure_templates[]` on ngspice
and Xyce.

## Evidence

- `sources/ngspice_manual.xhtml` documents ngspice `meas` / `.meas` control and
  deck syntax. CircuitCI preserves reviewed raw `measure_statements[]` for this
  backend and normalizes solver output into `measure_summary.csv`.
- `sources/Xyce_Reference_Guide_7.8.txt` documents Xyce `.MEASURE` / `.MEAS`
  syntax, supported TRAN/AC/DC/NOISE measure modes, and result files such as
  `.mt#`, `.ms#`, and `.ma#`.

## CircuitCI Contract

Raw `measure_statements[]` are treated as backend-specific text. CircuitCI does
not rewrite ngspice raw statements into Xyce `.MEASURE` syntax because the two
solver grammars, supported measure operators, result files, and edge/window
semantics are not identical enough to translate without a reviewed adapter.

For Xyce, users should declare portable `measure_templates[]`. CircuitCI renders
those templates into Xyce `.MEASURE` commands, retains raw measure result
artifacts, normalizes `measure_summary.csv`, records `solver_manifest.json`, and
has opt-in live conformance:

```sh
CIRCUITCI_RUN_REAL_XYCE=1 cargo test --test analog_measure_cli
```

`backend: auto` prefers ngspice. If ngspice is absent and the scenario contains
only portable `measure_templates[]`, CircuitCI may select Xyce/`xyce`
automatically because that path emits the same normalized `measure_summary.csv`
contract.

Explicit Xyce scenarios that still use raw `measure_statements[]` fail closed
with report-visible `adapter_blocker` and `evidence_sources[]` metadata until a
trusted raw-statement translator and real-solver conformance contract exists.
