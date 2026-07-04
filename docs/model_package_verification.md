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

Portable package bundles can be exported after a lock or registry-backed
package verifies cleanly:

```bash
circuitci export-model-package-bundle compact_model.lock.json \
  --registry compact_model_registry.json \
  --registry-entry tiny_resistor_qualified \
  --output dist/tiny_resistor_bundle
```

The bundle exporter re-verifies the source package, copies every pinned
artifact under `artifacts/`, rewrites the bundled `package.lock.json` to those
portable paths, optionally writes a bundled `compact_model_registry.json`,
re-runs verification against the bundled copy, and writes:

- `model_package_bundle_manifest.json` using
  `schemas/model_package_bundle_manifest.schema.json`
- `model_package_verification.json` and sibling Markdown summary
- `README.md` with artifact hashes and conformance check summaries

The bundled registry can be used directly by scenarios or by
`verify-model-package`, so a package can ship runtime artifacts, source,
conformance evidence, and human-readable review material as one deterministic
directory.

Verify a portable bundle before importing or distributing it:

```bash
circuitci verify-model-package-bundle dist/tiny_resistor_bundle \
  --output out/tiny_resistor_bundle_verification.json
```

The bundle verifier reads `model_package_bundle_manifest.json`, checks the
manifest schema marker, validates the bundled lock and optional registry hashes,
checks every copied artifact hash, confirms README and package-verification
artifacts are present, and then runs the normal package verifier against the
bundled lock/registry. The output uses
`schemas/model_package_bundle_verification_report.schema.json`, includes the
projected package conformance checks, and exits non-zero on any missing,
tampered, or inconsistent bundle content. When retained as a normal validation
artifact, the report is projected into `model_package_bundle_verifications[]`
and shown in Markdown/GUI report summaries with package identity, manifest,
lock, registry, artifact, conformance, and finding counts.

Install a verified bundle into a project-local or shared package area and emit
scenario-ready registry pins:

```bash
circuitci install-model-package-bundle dist/tiny_resistor_bundle \
  --install-dir third_party/models/tiny_resistor \
  --registry-output third_party/models/compact_model_registry.json \
  --output out/tiny_resistor_bundle_install.json
```

The installer verifies the source bundle, refuses a non-empty destination,
copies the bundle directory, verifies the installed copy, and optionally writes
a shared registry entry whose `lock_path` is relative to
`--registry-output`. The install report uses
`schemas/model_package_bundle_install_report.schema.json` and includes a
`scenario_import` object with `model_package_registry_path`,
`model_package_registry_sha256`, `model_package_registry_entry`,
`model_package_lock_path`, `model_package_lock_sha256`, and
`model_package_artifact_id`. Use `--registry-entry` and
`--registry-artifact-id` to override the bundled registry entry when installing
into a shared registry. When retained as a normal validation artifact, the
install report is projected into `model_package_bundle_installs[]` and shown in
Markdown/GUI report summaries with installed registry hashes and scenario-ready
registry/lock/artifact pins.
To apply those pins to an existing project copy without manual YAML editing,
run:

```bash
circuitci repair-yaml path/to/project.yaml \
  --finding bundle-install-package-metadata \
  --bundle-install-report out/tiny_resistor_bundle_install.json \
  --output out/tiny_resistor_bundle_import
```

The repair matches `analog.model_files[]` entries by
`model_package_artifact_id` or by the installed runtime artifact path, adds only
missing package fields, blocks on conflicting existing metadata, writes a
repaired project copy, and validates that copy. Normal validation reports and
the GUI artifact panel project the same repair command as `repair_yaml_command`
when the report was produced by `circuitci validate`. In the GUI Scopes
artifact panel, the `Repair YAML` button beside a bundle install row runs that
same repair path as a background job, writes the repaired copy under the current
validation output directory, and records the repaired project or repair report
path in recent job history without replacing the loaded project. The generated
repair report is also attached to the loaded report artifacts and projected as a
`yaml_repairs[]` row in the current `report.json`/`report.md` with
applied/blocked counts and proof status.

For CI jobs that need the complete import flow as one reproducible artifact,
use:

```bash
circuitci import-model-package-bundle dist/tiny_resistor_bundle \
  --project path/to/project.yaml \
  --install-dir third_party/models/tiny_resistor \
  --registry-output third_party/models/compact_model_registry.json \
  --output out/tiny_resistor_bundle_import
```

The import pipeline writes `bundle_verification.json`,
`bundle_install.json`, `package_verification.json`,
`repair_yaml/repair_report.json`, and
`model_package_bundle_import.json`/`.md` under the output directory. It fails
closed unless bundle verification, install, installed-package verification, and
YAML repair all pass. When the import report is retained as a validation
artifact, it projects into `model_package_bundle_imports[]` with scenario pins,
subreport paths, repaired project path, and repair counts.

To retain that same pipeline evidence from a normal validation run, pass a
repeatable bundle-import spec:

```bash
circuitci validate path/to/project.yaml \
  --output out/project_validation \
  --model-package-bundle-import \
    bundle=dist/tiny_resistor_bundle,install_dir=third_party/models/tiny_resistor,registry_output=third_party/models/compact_model_registry.json
```

Validation writes each retained import under
`<validation-output>/model_package_bundle_imports/<id>/`, adds the generated
`model_package_bundle_import.json` to `artifacts[]`, and fails closed with
`MODEL_PACKAGE_BUNDLE_IMPORT_FAILED` if the requested import pipeline does not
pass. Optional spec keys are `id`, `registry_entry`, and
`registry_artifact_id`.

Validation suites can declare the same imports per case:

```yaml
cases:
  - id: qualified_model_case
    project: ../examples/qualified_model/project.yaml
    expect: pass
    model_package_bundle_imports:
      - id: tiny_resistor
        bundle: ../dist/tiny_resistor_bundle
        install_dir: ../third_party/models/tiny_resistor
        registry_output: ../third_party/models/compact_model_registry.json
```

Suite bundle, install, and registry paths are resolved relative to the suite
manifest. The retained import report can be listed in `required_artifacts` when
the suite must prove that the package import pipeline ran.

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
