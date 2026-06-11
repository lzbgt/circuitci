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
6. Evaluate declared `below` / `above` waveform assertions for voltage,
   current, and power probes.
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

Assertions can sample a named probe at `at_us` or evaluate `min` / `max` over a
`start_us` to `end_us` window. If a requested boundary is between solver
samples, CircuitCI uses linear interpolation between adjacent samples. Probe
`quantity` selects the required threshold unit:

- `voltage`: `threshold_v`
- `current`: `threshold_a`
- `power`: `threshold_w`

Exactly one threshold field is allowed on each assertion. Sample assertions use
`at_us` and must not also declare a window. Window assertions use
`aggregation: min|max` with `start_us` and `end_us`, and must not also declare
`at_us`.

CircuitCI does a conservative expression/quantity check before invoking the
solver: voltage probes must export `V(...)`, current probes must export `I(...)`
or a simple sign/magnitude wrapper around `I(...)`, and power probes must
combine `V(...)` and `I(...)` in one expression. This is not a full ngspice
expression type system; it is a fail-closed guard against labeling voltage as
current or power in `report.json`.

A failed assertion emits a critical `SPICE_TRANSIENT_ANALYSIS` finding with
measured value, unit, quantity, and limit data.

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
- assertion sample/window time outside waveform range,
- declared model-file SHA-256 mismatch.

This is intentionally minimal but real. Later assertion forms must add
crossing times, setup/hold, no-recross, pulse-width, integration/energy, and
corner sweeps.

## Evidence Requirements

A passing analog report must include:

- source netlist artifact,
- model file artifacts,
- generated wrapper artifact,
- solver stdout/stderr log artifact,
- waveform CSV artifact,
- waveform reference in `waveforms`.
