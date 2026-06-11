# External Ngspice Runner

The first executable analog backend should validate CircuitCI's solver adapter
contract without introducing a homegrown SPICE subset.

## Scope

This slice implements orchestration around an installed `ngspice` executable:

1. Validate analog scenario metadata, model files, node bindings, and pin
   bindings.
2. Generate a CircuitCI-owned ngspice control wrapper under the validation
   output directory.
3. Run `ngspice -b` from the per-scenario artifact directory on that wrapper.
4. Capture solver stdout/stderr into artifacts.
5. Parse a CSV-like waveform export.
6. Evaluate the declared single-point `below` / `above` voltage assertions.
7. Populate report `artifacts` and `waveforms`.

It does not implement any SPICE numerical algorithm. If `ngspice` is missing,
the scenario still fails with `ANALOG_BACKEND_UNAVAILABLE`.

## Netlist Contract

The hand-authored deck remains the physical circuit source. CircuitCI generates
a wrapper with absolute include paths and a `.control` block:

```spice
.include "/absolute/path/to/downloader_q2_q3.cir"
.control
set wr_vecnames
set wr_singlescale
tran 1u 8m
wrdata waveform.csv v(boot0) v(nrst)
quit
.endc
.end
```

For this to work, physical fixture decks should not rely on an internal
`.control` block. The wrapper owns analysis execution and waveform export.
Ngspice 46 accepts the unquoted `wrdata` basename form reliably when the process
working directory is the artifact directory; quoted output filenames produced
missing-file diagnostics during bring-up.

## Assertion Semantics

The first executable assertion form samples a named probe at a declared time.
If no exact sample exists, CircuitCI uses linear interpolation between adjacent
samples. A failed assertion emits a critical `SPICE_TRANSIENT_ANALYSIS` finding
with measured and limit data.

The runner fails closed. These conditions are critical failures:

- `ngspice` missing,
- solver launch failure,
- solver timeout,
- nonzero solver exit,
- non-convergence or singular-matrix diagnostics in solver output,
- missing waveform file,
- empty or malformed waveform file,
- non-finite waveform sample,
- missing probe column,
- assertion sample time outside waveform range,
- declared model-file SHA-256 mismatch.

This is intentionally minimal but real. Later assertion forms must add windows,
crossing times, setup/hold, no-recross, pulse-width, current, power, and corner
sweeps.

## Evidence Requirements

A passing analog report must include:

- source netlist artifact,
- model file artifacts,
- generated wrapper artifact,
- solver stdout/stderr log artifact,
- waveform CSV artifact,
- waveform reference in `waveforms`.
