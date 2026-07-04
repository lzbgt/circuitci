# Xyce `.TF` / `.PZ` Backend Evidence

Date: 2026-07-04

## Sources Checked

- `docs/research/circuit_simulation_full_featured/sources/Xyce_Reference_Guide_7.8.txt`
- `docs/research/circuit_simulation_full_featured/sources/Xyce_Users_Guide_7.8.txt`

## Evidence

- The saved Xyce 7.8 Reference Guide documents `.SENS` and `.PRINT SENS FORMAT=CSV` output, which is why CircuitCI now has an explicit Xyce sensitivity adapter.
- Full-text searches of the saved Xyce 7.8 Reference Guide and User Guide did not find native `.TF` or `.PZ` command entries, transfer-function result files, or pole-zero result files comparable to the ngspice `.TF` and `.PZ` contracts.
- Xyce has AC analysis, but deriving `.TF` input/output resistance or `.PZ` roots from AC samples would not preserve the current normalized evidence contracts. CircuitCI should not silently replace `.TF` or `.PZ` with an approximation.

## CircuitCI Boundary

- `backend: xyce` for `analog_transfer_function` remains fail-closed with `SPICE_TRANSFER_FUNCTION_ANALYSIS`.
- `backend: xyce` for `analog_pole_zero` remains fail-closed with `SPICE_POLE_ZERO_ANALYSIS`.
- Both findings now include:
  - `measured.adapter_blocker`
  - `measured.evidence_sources[]`
- Future support should only be added if a trusted Xyce output path or an explicitly reviewed derivation contract emits:
  - `transfer_function_summary.csv`
  - `pole_zero_summary.csv`
  - `solver_manifest.json`
