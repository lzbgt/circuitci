# Acceptance Suite Runner

## Purpose

CircuitCI can validate one board project today. The acceptance demo needs a
single agent-facing command that runs the relevant board scenarios together and
reports whether the tool can detect known bad designs and accept fixed designs.

This slice adds a generic validation-suite runner. The UM STM32L4 demo is only
one suite manifest; the runner itself must work for any future ESP32, STM32,
STM8, C51/STC, 555, regulator, or board-family suite.

## Product Boundary

The runner is not a simulator rule and not a chip-specific profile. It is an
orchestration layer over existing `project.yaml` fixtures:

```text
suite.yaml
  -> validate each listed Board IR project
  -> compare actual pass/fail result with expected result
  -> write per-case reports
  -> write aggregate suite report
```

This gives the agent repair loop one stable acceptance target while preserving
the generic board IR and model-library architecture.

Suites may also declare repair links from a known-bad case to a fixed case.
Those links are evaluated into agent-readable repair evidence: failing project
path, fixed project path, report paths, matched finding details, and suggested
fixes.

## Suite Manifest

Proposed manifest path for the current acceptance target:

```text
suites/um_stm32l4_downloader_acceptance.yaml
```

Manifest shape:

```yaml
suite:
  name: um_stm32l4_downloader_acceptance
  version: 0.1.0
  validation_profile: iot_basic_v0
  description: >
    Acceptance suite for USB-UART power, boot, downloader, and firmware-update
    behavior around the UM STM32L4 fixture.
cases:
  - id: bad_backdrive_detected
    project: ../examples/bad_backdrive_board/project.yaml
    expect: fail
    required_findings:
      - id: GPIO_BACKDRIVE
        severity: critical
  - id: fixed_backdrive_passes
    project: ../examples/good_backdrive_fixed_board/project.yaml
    expect: pass
```

Paths are resolved relative to the manifest file.

## Result Semantics

Each case runs the same validation pipeline as `circuitci validate`.

A case passes the suite when:

- the generated project report result equals `expect`;
- every `required_findings` entry appears with the requested severity;
- every `expect: fail` case declares at least one required critical finding;
- no expected-passing case has critical findings;
- no expected-passing case has `limitations[].blocking == true`, unless the
  manifest explicitly lists that limitation ID in `allowed_blocking_limitations`.

The suite result is:

- `pass` when every case passes suite expectations;
- `fail` when any case result or required rule ID does not match.

This is intentionally different from project validation: a deliberately bad
fixture can be a passing suite case when CircuitCI detects the expected failure.

## CLI

Add:

```sh
circuitci validate-suite suites/um_stm32l4_downloader_acceptance.yaml --output out/acceptance/um_stm32l4
```

The output directory contains:

```text
report.json
report.md
cases/<case-id>/report.json
cases/<case-id>/report.md
```

When the manifest declares `repairs`, `report.json` also includes a `repairs`
array and repair summary counters.

## Suite Report JSON

The aggregate report is machine-readable and stable enough for an agent:

```json
{
  "schema_version": "0.1.0",
  "suite": "um_stm32l4_downloader_acceptance",
  "validation_profile": "iot_basic_v0",
  "result": "pass",
  "summary": {
    "cases": 12,
    "passed": 12,
    "failed": 0
  },
  "cases": [
    {
      "id": "bad_backdrive_detected",
      "project": "../examples/bad_backdrive_board/project.yaml",
      "expect": "fail",
      "actual": "fail",
      "result": "pass",
      "required_findings": [{"id": "GPIO_BACKDRIVE", "severity": "critical"}],
      "matched_findings": [{"id": "GPIO_BACKDRIVE", "severity": "critical"}],
      "report": "cases/bad_backdrive_detected/report.json"
    }
  ],
  "reproduction": {
    "command": "circuitci validate-suite suites/um_stm32l4_downloader_acceptance.yaml --output out/acceptance/um_stm32l4"
  }
}
```

## Acceptance Demo Cases

The first suite should include these exact cases:

| Case ID | Project | Expect | Required critical finding |
| --- | --- | --- | --- |
| `bad_backdrive_detected` | `examples/bad_backdrive_board/project.yaml` | `fail` | `GPIO_BACKDRIVE` |
| `fixed_backdrive_passes` | `examples/good_backdrive_fixed_board/project.yaml` | `pass` | none |
| `bad_reset_release_detected` | `examples/bad_reset_release_board/project.yaml` | `fail` | `RESET_RELEASE_AFTER_POWER_VALID` |
| `bad_app_boot_strap_detected` | `examples/um_stm32l4_app_boot_bad_release/project.yaml` | `fail` | `BOOT_STRAP_DEFINED` |
| `fixed_app_boot_passes` | `examples/um_stm32l4_app_boot_fixed_release/project.yaml` | `pass` | none |
| `wrong_uart_wiring_detected` | `examples/um_stm32l4_rom_download_wrong_uart/project.yaml` | `fail` | `UART_BOOTLOADER_SYNC` |
| `rom_downloader_entry_passes` | `examples/um_stm32l4_rom_download_entry/project.yaml` | `pass` | none |
| `resident_update_activate_passes` | `examples/um_stm32l4_resident_update_activate/project.yaml` | `pass` | none |
| `resident_update_missing_finish_detected` | `examples/um_stm32l4_resident_update_missing_finish/project.yaml` | `fail` | `RESIDENT_BOOTLOADER_UPDATE_SEQUENCE` |
| `resident_update_oversize_chunk_detected` | `examples/um_stm32l4_resident_update_oversize_chunk/project.yaml` | `fail` | `RESIDENT_BOOTLOADER_UPDATE_SEQUENCE` |
| `control_line_bad_release_detected` | `examples/um_stm32l4_control_line_app_release_bad/project.yaml` | `fail` | `CONTROL_LINE_RELEASE_SEQUENCE` |
| `control_line_fixed_release_passes` | `examples/um_stm32l4_control_line_app_release_fixed/project.yaml` | `pass` | none |

The suite can include generic non-UM fixtures as guard cases, but the acceptance
contract above is the minimum for the current demo.

## Schemas

Add:

- `schemas/suite_manifest.schema.json`
- `schemas/suite_report.schema.json`

Tests must validate the committed suite manifest and generated aggregate report
against these schemas.

## Verification

Run:

```sh
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
circuitci validate-suite suites/um_stm32l4_downloader_acceptance.yaml --output out/acceptance/um_stm32l4
```

The suite command must exit successfully only when suite expectations pass.

## Non-Goals

- No new circuit rule semantics.
- No physical waveform aggregation.
- No firmware execution.
- No hardcoded UM, STM32, or CH340 branches in the suite runner.
