# SPICE Engine Port Research

Downloaded source files on 2026-06-12:

| Engine | Local files | SHA-256 |
| --- | --- | --- |
| ngspice | `docs/research/spice_engines/ngspice_COPYING` | `07b8adb715ea31e6729732d6001f1abdadf944682f7d19cd91701ca72a8b0219` |
| ngspice | `docs/research/spice_engines/ngspice_README` | `cb53e3beb64ad2cc31076dd77b08773c69c7581b5ec469d83c22d6c9d6af49fd` |
| Xyce | `docs/research/spice_engines/xyce_COPYING` | `8ceb4b9ee5adedde47b31e975c1d90c73ad27b6b165a1dcd80c7c545eb65b903` |
| Xyce | `docs/research/spice_engines/xyce_README.md` | `7f8fb22dda70ca3e5465b76bdbf8a80d17bbbe4b073c1921aca94ea9b62bdded` |

## Findings

- ngspice is based on Spice3f5, Cider, and Xspice. Its `COPYING` file says the
  original Berkeley code is under the modified BSD license, but the project is
  heterogeneous and preserves original contributor licenses.
- ngspice `COPYING` explicitly warns that license tracking matters and that GPL
  code is not suitable for code directly linked into ngspice except through
  shared-object-library boundaries.
- Xyce `COPYING` is GPLv3. Its README describes Xyce as an open-source,
  SPICE-compatible, high-performance analog circuit simulator written in C++
  with MPI and Trilinos solver-library dependencies.

## Engineering Consequence

CircuitCI should not implement a small in-house SPICE subset. The credible
paths are:

1. Embed or vendor a mature solver kernel with its nonlinear solver, device
   equations, parser semantics, and regression tests intact.
2. Invoke a mature external solver until an embedded build is ready.
3. Keep physical analog acceptance failing when neither path is available.

For an in-process engine, ngspice is the first candidate because its Berkeley
SPICE core and shared-library integration path are closer to CircuitCI's Rust
runtime goals. Xyce remains valuable as a GPLv3 external/optional backend and
as a high-performance reference, but statically porting Xyce would bring a much
larger C++/MPI/Trilinos integration surface and GPLv3 distribution obligations.

## Port Boundary

The embedded solver must be treated as a third-party numerical kernel, not
rewritten feature-by-feature. CircuitCI owns:

- board/schematic import,
- model provenance and datasheet metadata,
- netlist generation,
- scenario orchestration,
- waveform assertion evaluation,
- reports and repair guidance.

The embedded solver owns:

- modified nodal analysis,
- nonlinear Newton iteration,
- transient integration,
- compact device equations,
- SPICE netlist semantics,
- convergence diagnostics.

## Required Next Step

Add an explicit `embedded_ngspice` backend option that fails unless a mature
vendored/shared ngspice engine is actually compiled or linked. This lets
projects request the future in-process engine without permitting a toy solver
or a fake pass.

