# Agent Workflow

CircuitCI exists so hardware-design agents can iterate safely before fabrication.

## Required Loop

```text
1. Generate or modify design artifacts.
2. Export board IR, netlist, BOM, or project YAML.
3. Run CircuitCI validation.
4. Parse report.json.
5. Fix critical failures first.
6. Re-run validation.
7. Declare fabrication readiness only when validation passes or limitations are explicit.
```

## MVP Command

```bash
circuitci validate examples/bad_backdrive_board/project.yaml \
  --profile iot_basic_v0 \
  --output out/bad_backdrive
```

The companion fixed fixture is:

```bash
circuitci validate examples/good_backdrive_fixed_board/project.yaml \
  --profile iot_basic_v0 \
  --output out/good_backdrive_fixed
```

The bad fixture must produce `result: "fail"` with a critical `GPIO_BACKDRIVE` finding. The fixed fixture must produce `result: "pass"` and no critical findings.

## Agent Rules

Agents must not:

- claim a design is fabrication-ready without validation output
- ignore critical validation failures
- hide unmodeled assumptions
- hardcode board-specific behavior in the generic engine
- add component models without tests
- add validation rules without passing and failing fixtures

Agents must:

- keep features testable
- preserve stable JSON report behavior
- include reproduction commands
- label uncertain results as low confidence
- separate generic engine code from component-library data
