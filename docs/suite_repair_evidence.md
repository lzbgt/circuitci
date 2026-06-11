# Suite Repair Evidence

## Purpose

The acceptance suite currently proves that CircuitCI can detect bad fixtures and
accept fixed fixtures. The original demo target also requires an agent repair
loop: consume `report.json`, identify the hardware bug, patch the design
artifact, rerun validation, and reach pass or documented limitation.

This slice makes the repair path first-class in the suite report without
hardcoding UM, STM32, or CH340 behavior.

## Generic Model

A suite may declare repair links:

```yaml
repairs:
  - id: fix_backdrive
    detects_case: bad_backdrive_detected
    fixed_case: fixed_backdrive_passes
    fixes_findings:
      - GPIO_BACKDRIVE
```

Each link says:

- `detects_case` is expected to be a suite-passing case whose project report
  actually fails with the listed finding IDs.
- `fixed_case` is expected to be a suite-passing case whose project report
  actually passes.
- `fixes_findings` lists the rule IDs the fixed case is evidence for.

The runner validates repair links after all cases run. It does not mutate files
or infer a schematic patch. It proves that a concrete fixed design artifact in
the suite resolves the same rule class detected in the bad design artifact.

For agent usability, a repair link must report enough evidence for the next
agent step:

- failing project path and report path;
- fixed project path and report path;
- matched finding details from the failing report;
- suggested fixes from the failing report;
- suite reproduction command.

This is still not automatic schematic patch synthesis, but it is a concrete,
machine-readable repair trail from defect report to fixed design artifact.

## Report Shape

The aggregate suite report adds:

```json
{
  "summary": {
    "cases": 12,
    "passed": 12,
    "failed": 0,
    "repairs": 7,
    "repairs_passed": 7,
    "repairs_failed": 0
  },
  "repairs": [
    {
      "id": "fix_backdrive",
      "detects_case": "bad_backdrive_detected",
      "fixed_case": "fixed_backdrive_passes",
      "fixes_findings": ["GPIO_BACKDRIVE"],
      "detect_project": "../examples/bad_backdrive_board/project.yaml",
      "fixed_project": "../examples/good_backdrive_fixed_board/project.yaml",
      "detect_report": "cases/bad_backdrive_detected/report.json",
      "fixed_report": "cases/fixed_backdrive_passes/report.json",
      "matched_findings": [
        {
          "id": "GPIO_BACKDRIVE",
          "severity": "critical",
          "scenario": "usb_uart_backdrive",
          "component": "U1",
          "net": "uart_rx",
          "message": "Powered component U2.TXD drives unpowered component U1.PA10 on net uart_rx.",
          "suggested_fixes": ["Add a series resistor sized to keep injection current below the receiving pin limit."]
        }
      ],
      "suggested_fixes": ["Add a series resistor sized to keep injection current below the receiving pin limit."],
      "result": "pass",
      "messages": []
    }
  ]
}
```

Suite `result` remains `pass` only when all cases and all repair links pass.

## Validation Semantics

For every repair link:

- `id` must be unique and path-safe.
- `detects_case` and `fixed_case` must reference existing cases.
- `fixes_findings` must be nonempty.
- `detects_case` must be `expect: fail`, have suite case result `pass`, have
  actual project result `fail`, and contain every listed `fixes_findings` as
  critical findings.
- `fixed_case` must be `expect: pass`, have suite case result `pass`, have
  actual project result `pass`, and have no unallowed blocking limitations.

The repair report records matched finding evidence using a normalized subset of
the validation finding: `id`, `severity`, `scenario`, optional `component`,
optional `net`, `message`, and `suggested_fixes`.

## Acceptance Repair Links

The UM acceptance suite should declare these repair links:

| Repair ID | Detects case | Fixed case | Finding ID |
| --- | --- | --- | --- |
| `fix_backdrive` | `bad_backdrive_detected` | `fixed_backdrive_passes` | `GPIO_BACKDRIVE` |
| `fix_reset_release` | `bad_reset_release_detected` | `fixed_app_boot_passes` | `RESET_RELEASE_AFTER_POWER_VALID` |
| `fix_app_boot_strap` | `bad_app_boot_strap_detected` | `fixed_app_boot_passes` | `BOOT_STRAP_DEFINED` |
| `fix_uart_wiring` | `wrong_uart_wiring_detected` | `rom_downloader_entry_passes` | `UART_BOOTLOADER_SYNC` |
| `fix_resident_update_sequence` | `resident_update_missing_finish_detected` | `resident_update_activate_passes` | `RESIDENT_BOOTLOADER_UPDATE_SEQUENCE` |
| `fix_resident_update_chunking` | `resident_update_oversize_chunk_detected` | `resident_update_activate_passes` | `RESIDENT_BOOTLOADER_UPDATE_SEQUENCE` |
| `fix_control_line_release` | `control_line_bad_release_detected` | `control_line_fixed_release_passes` | `CONTROL_LINE_RELEASE_SEQUENCE` |

## Non-Goals

- No automatic schematic patch synthesis.
- No mutation of project fixtures.
- No assumption that every possible board repair is represented by a paired
  fixture.
- No chip- or board-specific engine branch.
