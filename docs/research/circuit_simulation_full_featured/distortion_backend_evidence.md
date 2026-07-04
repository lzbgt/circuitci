# Small-Signal Distortion Backend Evidence Boundary

Date: 2026-07-04

## Decision

CircuitCI should keep `SPICE_DISTORTION_ANALYSIS` fail-closed until the first
real ngspice `.DISTO` wrapper, normalizer, and conformance fixture exist. The
saved ngspice manual provides the clearest open-source syntax target, but
CircuitCI does not yet emit or parse `.DISTO` artifacts.

Required future normalized artifacts:

- `distortion_spectrum`
- `distortion_summary`
- `distortion_convergence`
- raw solver output
- `solver_manifest.json`

## Saved Sources

- `sources/ngspice_manual.xhtml`
- `sources/ngspice_ANALYSES`
- `sources/Xyce_Reference_Guide_7.8.txt`
- `sources/qucs_s_home.html`
- `sources/spiceopus_release.html`

## ngspice

The saved ngspice manual documents `.disto dec|oct|lin ... <f2overf1>` and
independent-source `DISTOF1` / `DISTOF2` source keywords. It describes harmonic
analysis when `f2overf1` is absent and spectral/intermodulation analysis when
`f2overf1` is present. The manual also states that the method uses a Volterra
series expansion around the operating point and that only a subset of nonlinear
device models supports direct distortion analysis.

That is enough to define a future adapter target, but not enough to mark
CircuitCI scenarios passing until the adapter emits stable normalized outputs
and conformance evidence.

## Xyce

The saved Xyce 7.8 reference guide covers AC, HB, noise, transfer, pole-zero,
sensitivity, and other analyses, but the saved text does not document a
`.DISTO` or equivalent distortion-analysis command. CircuitCI should therefore
keep explicit `backend: xyce` fail-closed for `SPICE_DISTORTION_ANALYSIS`.

## QUCS-S and SPICE OPUS

The saved QUCS-S project page lists distortion support as an advanced SPICE
simulation capability, but that is not a distinct CircuitCI backend contract.
The saved SPICE OPUS release material mentions distortion parameters such as
`distof1` and `distof2`, but CircuitCI does not have a SPICE OPUS adapter or
real-solver conformance path.

## Required Adapter Evidence Before Enabling

A future distortion adapter must provide:

- primary documentation for command/source syntax and output variables
- deterministic wrapper/deck generation that annotates `DISTOF1`/`DISTOF2`
  source inputs
- raw output retention
- normalized `distortion_spectrum.csv` with frequency, component label,
  complex value, magnitude, and phase
- normalized `distortion_summary.csv` for HD2/HD3 and intermodulation products
  when available
- convergence/model-support metadata, including unsupported-device warnings
- `solver_manifest.json` with backend version, command, model provenance, and
  normalized artifact paths
- opt-in real-ngspice conformance coverage with clean skip behavior
