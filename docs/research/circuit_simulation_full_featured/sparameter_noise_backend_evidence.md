# S-Parameter Noise Backend Evidence

Status as of 2026-07-04: CircuitCI treats RF S-parameter noise sign-off as
adapter-ready for ngspice only.

## Evidence

- `sources/ngspice_manual.xhtml` documents `.SP` S-parameter analysis with the
  optional `donoise` argument. It states that `.SP` emits S-matrix outputs and
  that two-port `donoise=1` runs add NF, NFmin, Rn, and SOpt outputs.
- `sources/Xyce_Reference_Guide_7.8.txt` documents Xyce S/Y/Z Touchstone output
  and ordinary `.NOISE` analysis/measure support. The retained source snapshot
  does not document an equivalent SP-noise command/output contract that emits
  NF, NFmin, Rn, and optimum source reflection from one two-port S-parameter
  run.

## CircuitCI Contract

`analysis.s_parameter_noise_assertions[]` requires normalized
`s_parameter_noise_summary.csv` evidence containing worst-case NF, NFmin,
equivalent noise resistance, and `|SOpt|`. A backend must also retain raw
solver output and `solver_manifest.json` provenance.

ngspice `.SP ... donoise=1` now satisfies that contract and is covered by
opt-in live conformance:

```sh
CIRCUITCI_RUN_REAL_NGSPICE=1 cargo test --test analog_sparameter_cli
```

The Xyce Touchstone path and other non-ngspice paths fail closed for these
RF-noise assertions until they provide a trusted normalized SP-noise artifact
contract with real-solver conformance.
