# Embedded SPICE Engine Port Strategy

CircuitCI may embed or port an open-source SPICE engine, but it must not grow a
small partial SPICE clone. A partial solver would be dangerous for agent-driven
board design because it can produce confident but physically incomplete
answers.

## Decision

Keep the analog solver behind the same adapter contract for every backend:

```text
Board IR + model provenance
  -> SPICE-compatible deck and model files
  -> solver backend
  -> logs, convergence diagnostics, waveforms
  -> CircuitCI waveform assertions and repair report
```

Backends can be:

- `ngspice`: external executable.
- `xyce`: external executable.
- `embedded_ngspice`: mature ngspice-derived solver linked or vendored into
  CircuitCI.
- Future backends that implement the same evidence contract.

The `embedded_ngspice` backend is allowed only when the real solver kernel is
present. It must fail with `ANALOG_EMBEDDED_SOLVER_UNAVAILABLE` until then.

## Why Ngspice First

The local research record in `docs/research/spice_engines/` shows:

- ngspice is based on Spice3f5, Cider, and Xspice, with heterogeneous licensing.
- Berkeley SPICE analog simulation code is listed as new/modified BSD in the
  ngspice license file.
- Some ngspice-related areas are LGPL or require careful license boundaries.
- Xyce is GPLv3 and has a larger C++/MPI/Trilinos integration surface.

This makes ngspice the first candidate for an embedded CircuitCI backend, while
Xyce remains valuable as an external/reference backend.

## Porting Rules

- Import or link the mature solver as a third-party kernel.
- Preserve license files, notices, attribution, source provenance, and build
  scripts.
- Keep GPL/LGPL/shared-library boundaries explicit.
- Do not rewrite device models one-by-one into a partial Rust substitute.
- Do not report physical analog pass unless solver logs and waveform artifacts
  are present.
- Keep vendor SPICE model redistribution rights separate from the engine
  license; model cards may be proprietary even when the solver is open source.

## Implementation Slices

1. Keep `embedded_ngspice` as an explicit backend selector that fails until a
   real mature solver is linked. Done for missing libraries; when
   `libngspice` is present, CircuitCI dynamically loads the mature shared
   ngspice engine.
2. Implement external `ngspice` execution and waveform parsing first, because it
   validates the adapter and assertion contract without changing license shape.
3. Add a vendored/shared ngspice build under a dedicated third-party boundary.
   The first implementation dynamically loads a system `libngspice` and keeps
   the dependency optional at runtime.
4. Run upstream solver regression tests plus CircuitCI UM downloader physical
   acceptance.
5. Only after broader parity is proven, make embedded ngspice the preferred local
   backend.

## Implemented Shared-Library Adapter

The current adapter uses the public `sharedspice.h` API:

- `ngSpice_Init` initializes callbacks.
- `ngSpice_Circ` loads a CircuitCI-generated circuit deck.
- `ngSpice_Command` sends `set wr_vecnames`, `set wr_singlescale`, `tran`, and
  `wrdata` commands.
- Callback output is written into the same `ngspice.log` artifact used by the
  external executable path.

On the tested Homebrew `libngspice` 46 build,
`ngSpice_nospinit()` crashed before initialization, so CircuitCI does not call
that optional function. The adapter still records the resulting initialization
warning in `ngspice.log` and requires waveform evidence before any physical
pass.
