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
- `required-pin-floating`: fixes `REQUIRED_PIN_FLOATING` by adding a missing
  required `board.components.<component>.pins.<pin>` binding only when the
  component already declares a compatible existing net for that exact pin
  through `power_domains.<pin>`, or for an electrical-power pin through
  `power_domain`.
- `analog-model-package-metadata`: migrates older generated analog scenarios by
  adding missing `analog.model_files[]` package-lock and registry fields inferred
  from component-library `simulation.spice` metadata. It is additive only; if an
  existing package field disagrees with the library metadata, the proposal is
  blocked with `reason_code: package_metadata_conflict` instead of overwriting
  provenance.
- `bundle-install-package-metadata`: imports scenario-ready pins from a passing
  `install-model-package-bundle` report. It requires `--bundle-install-report`,
  matches analog model files by `model_package_artifact_id` or installed runtime
  artifact path, and adds missing package name/version, registry path/SHA/entry,
  lock path/SHA, and artifact id fields. Existing conflicting package fields
  block the proposal with
  `reason_code: bundle_install_package_metadata_conflict`.

Example:

```sh
circuitci repair-yaml path/to/project.yaml \
  --finding net-not-found \
  --profile iot_basic_v0 \
  --output out/repair
```

Import package pins from a bundle install report:

```sh
circuitci repair-yaml path/to/project.yaml \
  --finding bundle-install-package-metadata \
  --bundle-install-report out/tiny_resistor_bundle_install.json \
  --profile iot_basic_v0 \
  --output out/repair_bundle_import
```

Use `--dry-run` to stop after original validation and proposal generation:

```sh
circuitci repair-yaml path/to/project.yaml \
  --finding net-not-found \
  --profile iot_basic_v0 \
  --output out/repair \
  --dry-run
```

Use `--apply-report` to consume a previous dry-run report, verify it still
matches the current project and finding, then apply exactly those proposed edits
to a copied project:

```sh
circuitci repair-yaml path/to/project.yaml \
  --finding net-not-found \
  --profile iot_basic_v0 \
  --output out/repair_apply \
  --apply-report out/repair_dry/repair_report.json
```

When a dry-run report contains multiple proposed edits, pass one or more
`--proposal-id` values to apply only the approved proposal ids:

```sh
circuitci repair-yaml path/to/project.yaml \
  --finding net-not-found \
  --profile iot_basic_v0 \
  --output out/repair_apply_one \
  --apply-report out/repair_dry/repair_report.json \
  --proposal-id net_not_found_1
```

Apply-mode output contains:

- `original/report.json` and `original/report.md`
- `repaired/project.yaml`
- `repaired/report.json` and `repaired/report.md`
- `repair_report.json` and `repair_report.md`

Dry-run output contains `original/report.json`, `original/report.md`,
`repair_report.json`, and `repair_report.md`; it does not write
`repaired/project.yaml` or run repaired validation.

Report-driven apply mode writes the same copied-project and validation artifacts
as normal apply mode, but sets `mode: apply_report`. It requires the prior
report to have `mode: dry_run` and `result: dry_run`, and verifies the project
path, project name, profile, requested finding, original matching findings, and
regenerated proposal list before applying. If any of those inputs drift, the
command fails before writing a repaired copy so agents can regenerate a fresh
dry-run report. `--proposal-id` is valid only with `--apply-report`; requested
ids must be unique, present in the dry-run report, and still in `status:
proposed`. Non-selected proposed edits are written back as `status: skipped` in
the apply report, and unresolved same-class findings remain visible in
`proof.repaired_matching_findings`. `--bundle-install-report` is valid only
with `--finding bundle-install-package-metadata`; relative report paths are
resolved from the install report location before the copied project is written.

`repair_report.json` follows `schemas/repair_report.schema.json`. Its
`proposals[].edits[]` entries carry the YAML path, operation, previous value,
new value, and reason. `proposals[].affected_pins[]` records the component pins
that justified the edit, and `proposals[].reason_code` records why a proposal
is blocked or skipped when the status is not actionable. The top-level
`reason_codes[]` array mirrors `messages[]` with stable machine-readable codes
such as `target_finding_absent`, `no_supported_proposal`,
`proposal_blocked`, `proposal_skipped_not_selected`, and
`target_finding_remains`. The `summary` and `proof` blocks record original and
repaired matching findings across failures, warnings, and infos, while still
tracking whether the repair introduced new critical findings.
`summary.selected` records how many proposed edits were selected for application
in the current run. `mode: dry_run` reports `result: dry_run`, leaves applicable
proposals in `status: proposed`, sets `repaired_project` and `repaired_report`
to `null`, and leaves proof booleans such as `original_finding_removed` as
`null` because no repaired copy was validated.

When a repair class is requested but no safe edit can be applied, the command
still writes `repair_report.json` and `repair_report.md` with `result: fail`.
Agents should read `reason_codes[]`, `summary.blocked`, `summary.skipped`, and
`proposals[].reason_code` instead of parsing stderr or prose. Ambiguous
missing-net repairs are represented as `proposals[].status: blocked`,
`proposals[].reason_code: conflicting_inferred_net_kinds`, an empty `edits[]`
list, and a description of the conflicting inferred net kinds. If the requested
finding is absent, the report has zero proposals and `reason_codes[]` includes
`target_finding_absent`; if the finding exists but no safe edit is known,
`reason_codes[]` includes `no_supported_proposal`.

This command is intentionally narrow. It does not choose nominal rail voltages,
invent nets for undeclared model pins, connect floating required pins without
explicit compatible component metadata, repair ambiguous missing nets with
conflicting inferred kinds, remove required declared pins, edit component
models, or repair schematic/PCB geometry. Relative `libraries:` entries are
converted to absolute paths in the repaired copy so the copied project validates
from the repair output directory. Relative analog model-file paths and
package-lock/registry paths are also rewritten to absolute paths in repaired
copies for the same reason.
