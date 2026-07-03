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
Scenario-level `analog.model_files[]` may inline lock metadata or import a
registry entry with `model_package_registry_path`,
`model_package_registry_sha256`, and `model_package_registry_entry`; the
standalone verifier is the preflight for those reusable package artifacts.
