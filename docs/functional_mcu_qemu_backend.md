# Functional MCU QEMU Backend

`firmware_in_loop` is a functional MCU black-box path. It runs firmware in a
functional machine model and validates board-facing pin behavior. It is not a
transistor-level MCU simulation path.

## Local Backend Source

This implementation targets the locally installed `qemu-system-arm` interface.
During implementation, the local binary reported:

- `qemu-system-arm --version`: QEMU emulator version 10.0.3.
- `qemu-system-arm -machine help`: includes Cortex-M machines such as
  `stm32vldiscovery`, `b-l475e-iot01a`, `lm3s6965evb`, and `mps2-*`.

These facts come from the installed QEMU binary, not from an inferred project
assumption.

## Scenario Contract

QEMU scenarios declare:

- `firmware.backend: qemu`
- `firmware.image`: firmware image path, relative to the project file unless
  absolute
- `firmware.machine`: QEMU machine name passed to `-M`
- optional `firmware.qemu.executable`, defaulting to `qemu-system-arm`
- optional `firmware.qemu.extra_args`, appended as argv entries
- optional `firmware.qemu.timeout_ms`, defaulting to 5000 ms
- optional `firmware.qemu.pin_trace_prefix`, defaulting to `CIRCUITCI_PIN `
- at least one `firmware.expected_pin_states` entry

The runtime invokes QEMU with:

```text
qemu-system-arm -M <machine> -kernel <image> -nographic -semihosting <extra_args...>
```

`backend: auto` selects this QEMU path only when a machine is declared and the
selected QEMU executable is available.

## Pin Observation Contract

The firmware or functional board model must emit explicit observations:

```text
CIRCUITCI_PIN U1.TX mode=output state=high
```

Valid modes are `input`, `output`, and `high_z`. Valid states are `high`,
`low`, and `z`. Missing observations, malformed observations, duplicate
conflicting observations, QEMU failures, and mismatches against
`expected_pin_states` all fail closed with `FUNCTIONAL_MCU_FIRMWARE`.

The validator writes `qemu.log` as an artifact for successful and failing QEMU
runs when it can create the scenario artifact directory.

## Boundary

This backend proves firmware-visible, board-facing behavior. It must not be
used to imply internal MCU transistor fidelity. Analog board effects around the
MCU pins still belong in `analog_transient` SPICE scenarios with explicit
models and waveform assertions.
