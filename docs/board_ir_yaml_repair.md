# Board IR YAML Repair Loop

`circuitci repair-yaml` is the first concrete agent repair command. It never
edits the input project in place. In normal apply mode it validates the original
project, generates a machine-readable patch proposal for one supported Board IR
YAML finding class, writes a repaired project copy, validates that copy, and
reports whether the target finding disappeared without introducing new critical
findings.

Supported repair classes:

- `invalid-power-domain`: fixes `INVALID_POWER_DOMAIN` by changing the affected
  `board.nets.<net>.kind` value to `power` when a declared model power pin is
  connected to an existing non-power net.
- `net-not-found`: fixes `NET_NOT_FOUND` by adding an existing pin reference's
  missing net when the pin is declared by the component model. The new net kind
  is inferred from the model port kind: power ports become `power`, ground ports
  become `ground`, and passive or digital ports become `digital_or_analog`.
- `pin-not-declared`: fixes warning-level `PIN_NOT_DECLARED` findings by
  removing a copied project's stray `board.components.<component>.pins.<pin>`
  binding when the resolved component model does not declare that pin.

Example:

```sh
circuitci repair-yaml path/to/project.yaml \
  --finding net-not-found \
  --profile iot_basic_v0 \
  --output out/repair
```

Use `--dry-run` to stop after original validation and proposal generation:

```sh
circuitci repair-yaml path/to/project.yaml \
  --finding net-not-found \
  --profile iot_basic_v0 \
  --output out/repair \
  --dry-run
```

Apply-mode output contains:

- `original/report.json` and `original/report.md`
- `repaired/project.yaml`
- `repaired/report.json` and `repaired/report.md`
- `repair_report.json` and `repair_report.md`

Dry-run output contains `original/report.json`, `original/report.md`,
`repair_report.json`, and `repair_report.md`; it does not write
`repaired/project.yaml` or run repaired validation.

`repair_report.json` follows `schemas/repair_report.schema.json`. Its
`proposals[].edits[]` entries carry the YAML path, operation, previous value,
new value, and reason. `proposals[].affected_pins[]` records the component pins
that justified the edit. The `summary` and `proof` blocks record original and
repaired matching findings across failures, warnings, and infos, while still
tracking whether the repair introduced new critical findings. `mode: dry_run`
reports `result: dry_run`, leaves applicable proposals in `status: proposed`,
sets `repaired_project` and `repaired_report` to `null`, and leaves proof
booleans such as `original_finding_removed` as `null` because no repaired copy
was validated.

When a repair class is requested but no safe edit can be applied, the command
still writes `repair_report.json` and `repair_report.md` with `result: fail`.
Agents should read `messages[]`, `summary.blocked`, and `summary.skipped`
instead of parsing stderr. Ambiguous missing-net repairs are represented as
`proposals[].status: blocked` with an empty `edits[]` list and a description of
the conflicting inferred net kinds. If the requested finding is absent, the
report has zero proposals and a message explaining that no matching finding was
available to repair.

This command is intentionally narrow. It does not choose nominal rail voltages,
invent nets for undeclared model pins, repair ambiguous missing nets with
conflicting inferred kinds, remove required declared pins, edit component
models, or repair schematic/PCB geometry. Relative `libraries:` entries are
converted to absolute paths in the repaired copy so the copied project validates
from the repair output directory.
