# Embedded Ngspice Backend

CircuitCI supports `backend: embedded_ngspice` through the mature shared
`libngspice` engine. This is an in-process adapter around ngspice, not a
partial Rust SPICE implementation.

## Contract

The embedded backend preserves the same evidence contract as the external
`ngspice` executable path:

1. Build or load a Board-IR-bound SPICE deck.
2. Generate a CircuitCI wrapper deck with the rewritten model includes and
   circuit elements.
3. Execute that deck with a mature ngspice kernel and send `tran` / `wrdata`
   through the shared-library command API.
4. Capture solver text output in `ngspice.log`.
5. Parse `waveform.csv` with the same parser used by the external backend.
6. Run the same waveform assertions and datasheet operating-limit checks.

The report must not pass a physical analog scenario unless the generated deck,
wrapper deck, solver log, and waveform artifact are all present.

## Library Boundary

The Rust binary dynamically loads `libngspice` at runtime. Lookup order is:

- `CIRCUITCI_LIBNGSPICE` when set. This is a strict override; CircuitCI does
  not fall back to another shared library path if the configured library cannot
  be loaded.
- Common platform library names such as `libngspice.dylib`,
  `libngspice.so`, `libngspice.so.0`, and `ngspice.dll`.
- Homebrew/macOS library locations under `/opt/homebrew` and `/usr/local`.

If the shared library is unavailable, explicit `backend: embedded_ngspice`
still fails closed with `ANALOG_EMBEDDED_SOLVER_UNAVAILABLE`.

## Execution Model

`libngspice` exposes process-global simulator state. CircuitCI serializes
embedded runs through a global mutex, initializes ngspice callbacks for each
run, loads the circuit with `ngSpice_Circ`, sends `tran` and `wrdata` through
`ngSpice_Command`, captures callback text, then sends `destroy all` through
the command interface.

The embedded wrapper omits the interactive `.control` block and `quit` command
used by the external batch executable. This avoids requesting process/library
exit from inside the shared library while still producing the same
`waveform.csv` evidence.

## Non-Goals

- No in-house analog numerical solver.
- No device-model reimplementation in Rust.
- No physical pass without ngspice waveform evidence.
- No Xyce in-process adapter in this slice.
