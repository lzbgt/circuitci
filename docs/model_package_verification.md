# Compact Model Package Verification

`circuitci export-model-package` creates reusable compact-model package locks
from real artifact hashes, and `circuitci verify-model-package` validates those
locks before a scenario references them.

```bash
circuitci export-model-package \
  --package-name org.circuitci.test.tiny_resistor \
  --package-version 1.0.0 \
  --artifact-id tiny_resistor_osdi \
  --artifact tiny_resistor.osdi \
  --artifact-format osdi_shared_object \
  --compiler openvaf \
  --output compact_model.lock.json \
  --registry-output compact_model_registry.json \
  --registry-entry tiny_resistor_qualified_osdi
```

The export command writes deterministic JSON, hashes the artifact, writes a
schema-compatible lock, and optionally writes a registry entry pointing to that
lock. Its stdout includes the lock and registry SHA-256 values needed for
scenario pins.

For qualified compact models that need paired artifacts, use repeatable
`--package-artifact` specs instead of the single-artifact flags:

```bash
circuitci export-model-package \
  --package-name org.circuitci.test.tiny_resistor \
  --package-version 1.0.0 \
  --package-artifact id=tiny_source,path=tiny_resistor.va,artifact_format=verilog_a_source \
  --package-artifact id=tiny_ngspice,path=tiny_resistor.osdi,artifact_format=osdi_shared_object,compiler=openvaf \
  --package-artifact id=tiny_xyce,path=tiny_resistor_xyce.so,artifact_format=xyce_adms_plugin,compiler=xyce_adms \
  --package-artifact id=tiny_conformance,path=tiny_resistor_conformance.json,artifact_format=model_conformance_report \
  --output compact_model.lock.json \
  --registry-output compact_model_registry.json \
  --registry-entry tiny_resistor_qualified \
  --registry-artifact-id tiny_ngspice
```

Each spec uses comma-separated `key=value` pairs with required `id`, `path`,
and `artifact_format`; `format` is accepted as an alias for `artifact_format`.
The optional `compiler` value is per artifact. When a registry is emitted, use
`--registry-artifact-id` to choose the artifact id imported by scenario
registry references; if omitted, the registry points to the first artifact in
the exported lock.

Conformance reports can be generated deterministically from a CircuitCI
`report.json` instead of hand-authored:

```bash
circuitci export-model-conformance-report \
  --report out/model_check/report.json \
  --package-name org.circuitci.test.tiny_resistor \
  --package-version 1.0.0 \
  --artifact-id tiny_ngspice \
  --runtime-artifact tiny_resistor.osdi \
  --check-name transient_smoke \
  --analysis tran \
  --solver ngspice \
  --output tiny_resistor_conformance.json
```

The command hashes `--runtime-artifact`, reads the validation result from the
report, copies report artifact paths into the check row, and writes
`schemas/model_conformance_report.schema.json` JSON with no timestamp field so
identical inputs produce identical output. A failing validation report produces
a failing conformance report; package verification then rejects it when the
report is included as `artifact_format=model_conformance_report`.

Shared registries can be produced from exported package registries without
hand-editing JSON:

```bash
circuitci merge-model-package-registry \
  --input vendor_a/compact_model_registry.json \
  --input vendor_b/compact_model_registry.yaml \
  --output compact_model_registry.json
```

`--base existing_registry.json` may be supplied to retain already-qualified
entries. The merge command accepts JSON or YAML registries, rewrites every
entry's `lock_path` relative to the output registry directory, emits
deterministic JSON using `schemas/model_package_registry.schema.json`, drops
identical duplicate entries, and fails when duplicate entry ids disagree.

```bash
circuitci verify-model-package compact_model.lock.yaml \
  --registry compact_model_registry.yaml \
  --registry-entry tiny_resistor_qualified_osdi \
  --output out/model_package_verification.json
```

The command writes a JSON report using
`schemas/model_package_verification_report.schema.json` and exits non-zero when
any critical finding is present. It also writes a sibling Markdown report next
to the JSON output. It accepts JSON or YAML lock and registry documents. The
report includes `conformance_checks[]`, a compact projection of validated
`model_conformance_report` rows with report artifact id/path, target artifact
id/SHA-256, check name, analysis, solver, result, and referenced check
artifacts. Validation reports that retain a package-verification JSON artifact
project those rows into `model_package_conformance_checks[]` and the GUI
Simulation report panel.

The verifier checks:

- package name/version in the lock
- non-empty artifact rows with id, path, SHA-256, and artifact format
- each package artifact hash relative to the lock file directory
- `model_conformance_report` artifacts against
  `schemas/model_conformance_report.schema.json`
- conformance report package identity, target artifact id, runtime artifact
  SHA-256, overall pass result, and per-check pass results
- conformance check summaries for package-review dashboards
- optional registry entry existence
- registry package identity against the lock package identity
- registry lock path and lock SHA-256 against the supplied lock file

The package lock shape is defined by `schemas/model_package_lock.schema.json`.
The package registry shape is defined by
`schemas/model_package_registry.schema.json`.
The conformance report shape is defined by
`schemas/model_conformance_report.schema.json`.
Supported artifact formats include ordinary SPICE includes, Verilog-A source,
OpenVAF/OSDI shared objects, Xyce/ADMS plugins, and model conformance reports.
Scenario-level `analog.model_files[]` may inline lock metadata or import a
registry entry with `model_package_registry_path`,
`model_package_registry_sha256`, and `model_package_registry_entry`; the
standalone verifier is the preflight for those reusable package artifacts.

The repository ships a reusable generic behavioral SPICE package fixture:
`models/packages/generic/analog_behavioral.lock.json` is imported by
`models/packages/compact_model_registry.json` as
`generic_analog_behavioral_spice`. Generic analog component models reference
that registry entry, so generated analog scenarios preserve package provenance
automatically instead of emitting only an ad hoc `model_path` and SHA-256.
Older generated scenarios that predate those fields can be migrated with
`circuitci repair-yaml <project.yaml> --finding analog-model-package-metadata`;
the repair copy adds only missing package fields and validates the migrated
copy without editing the original project in place.
