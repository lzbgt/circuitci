# Acceptance Demo Direction

`demo_project_circuit_for_acceptance.md` defines the long-term acceptance target: CircuitCI should help an agent validate and fix a board that powers and interfaces with the STM32L4-based `um` module in the peer `../urine_monitor` project, including bootloader/app download behavior and USB downloader circuit bugs.

This is a product acceptance fixture, not an engine specialization.

## Required Direction

- STM32L4 must be represented as a component-library model pack.
- USB downloader circuitry must be represented through generic board IR, nets, scenarios, and validation rules.
- Physical USB downloader claims must be backed by generic SPICE-class analog
  transient simulation, not by circuit-specific behavioral delay rules.
- The engine must remain equally able to load model packs for common chips such as ESP32, STM32F1/F4/L1/L4, STM8, C51/STC-class MCUs, 555 timers, USB-UART bridges, regulators, and passives.
- The agent repair loop must consume `report.json`, patch the design artifact, rerun validation, and reach a pass or explicit documented limitation.

## Iteration Path

1. Generic Rust validation backbone.
2. USB-UART-to-MCU backdrive detection and fixed fixture.
3. Reset/boot strap validation.
4. STM32-like bootloader UART-download scenario.
5. STM32L4 vendor pack and `../urine_monitor` acceptance fixture.
6. SPICE-class analog transient backend for quantitative voltage/current
   waveform assertions.

## Current Acceptance Interpretation

The first downloader-related acceptance milestone is not full firmware execution
or full physical analog proof. It is an agent-readable behavioral validation
loop that can:

1. detect unsafe USB-UART-to-MCU electrical behavior,
2. detect reset release before the MCU rail is valid,
3. detect wrong or floating boot strap state,
4. detect missing abstract UART bootloader sync,
5. emit JSON findings with concrete fixes,
6. validate a fixed project fixture as passing.

The current one-command acceptance target is:

```sh
circuitci validate-suite suites/um_stm32l4_downloader_acceptance.yaml --output out/acceptance/um_stm32l4
```

The suite is generic orchestration over Board IR projects. It expects deliberate
bad fixtures to fail with specific critical rule IDs, and fixed fixtures to pass
without blocking limitations.

This suite is not sufficient for the known UM USB downloader Q2/Q3 saturation
failure. That failure requires nonlinear transient analog simulation. The
physical analog gate is:

```sh
circuitci validate-suite suites/um_stm32l4_downloader_physical_acceptance.yaml --output out/acceptance/um_stm32l4_physical
```

On a host without `ngspice` or `Xyce`, this suite must fail with
`ANALOG_BACKEND_UNAVAILABLE`. A physical pass requires a SPICE-class backend,
the Q2/Q3/D13 netlist, model cards, solver logs, waveform artifacts, and
quantitative BOOT0/NRST assertions.

The existing `um_stm32l4_downloader_acceptance` suite remains a behavioral
pre-physics suite. It must not be used as evidence that the Q2/Q3 saturated
transistor reset circuit is physically reliable.
