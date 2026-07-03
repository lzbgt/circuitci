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
any critical finding is present. It accepts JSON or YAML lock and registry
documents.

The verifier checks:

- package name/version in the lock
- non-empty artifact rows with id, path, SHA-256, and artifact format
- each package artifact hash relative to the lock file directory
- optional registry entry existence
- registry package identity against the lock package identity
- registry lock path and lock SHA-256 against the supplied lock file

The package lock shape is defined by `schemas/model_package_lock.schema.json`.
The package registry shape is defined by
`schemas/model_package_registry.schema.json`.
Supported artifact formats include ordinary SPICE includes, Verilog-A source,
OpenVAF/OSDI shared objects, Xyce/ADMS plugins, and model conformance reports.
Scenario-level `analog.model_files[]` may inline lock metadata or import a
registry entry with `model_package_registry_path`,
`model_package_registry_sha256`, and `model_package_registry_entry`; the
standalone verifier is the preflight for those reusable package artifacts.
