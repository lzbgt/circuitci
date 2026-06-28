# Board IR YAML Repair Loop

`circuitci repair-yaml` is the first concrete agent repair command. It does not
edit the input project in place. It validates the original project, generates a
machine-readable patch proposal for one supported Board IR YAML finding class,
writes a repaired project copy, validates that copy, and reports whether the
target finding disappeared without introducing new critical findings.

Supported repair class:

- `invalid-power-domain`: fixes `INVALID_POWER_DOMAIN` by changing the affected
  `board.nets.<net>.kind` value to `power` when a declared model power pin is
  connected to an existing non-power net.

Example:

```sh
circuitci repair-yaml path/to/project.yaml \
  --finding invalid-power-domain \
  --profile iot_basic_v0 \
  --output out/repair
```

The output directory contains:

- `original/report.json` and `original/report.md`
- `repaired/project.yaml`
- `repaired/report.json` and `repaired/report.md`
- `repair_report.json` and `repair_report.md`

`repair_report.json` follows `schemas/repair_report.schema.json`. Its
`proposals[].edits[]` entries carry the YAML path, operation, previous value,
new value, and reason. Its `proof` block records original matching findings,
repaired matching findings, and any new critical findings.

This command is intentionally narrow. It does not infer missing nets, choose
power rail voltages, edit component models, or repair schematic/PCB geometry.
Relative `libraries:` entries are converted to absolute paths in the repaired
copy so the copied project validates from the repair output directory.
