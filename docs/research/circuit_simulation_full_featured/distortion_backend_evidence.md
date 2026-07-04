# Small-Signal Distortion Backend Evidence

Date: 2026-07-04

## Decision

CircuitCI now enables `SPICE_DISTORTION_ANALYSIS` for external ngspice. Xyce
and embedded ngspice remain fail-closed because the saved primary material does
not provide an equivalent trusted adapter contract for those backends.

The ngspice adapter emits:

- `distortion_spectrum.csv`
- `distortion_summary.csv`
- `distortion_convergence.json`
- raw solver output
- `solver_manifest.json`

## Saved Sources

- `sources/ngspice_manual.xhtml`
- `sources/ngspice_manual_disto_distortionanalysis.html`
- `sources/ngspice_manual_2026_07_04.pdf`
- `sources/ngspice_ANALYSES`
- `sources/ngspice_src_ngspice_txt_master.txt.gz`
- `sources/ngspice_source_distoan.c.gz`
- `sources/ngspice_source_distodef.h.gz`
- `sources/ngspice_source_inp2dot.c.gz`
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

The saved ngspice source confirms plot creation for `DISTORTION - 2nd
harmonic`, `DISTORTION - 3rd harmonic`, `DISTORTION - IM: f1+f2`,
`DISTORTION - IM: f1-f2`, and `DISTORTION - IM: 2f1-f2`. Local probing against
ngspice 46 showed that a bare `DISTOF1` token is rejected by the parser, so the
CircuitCI wrapper emits explicit defaults: `DISTOF1 1.0 0.0` and `DISTOF2 1.0
0.0`.

The adapter selects the expected ngspice plots, prints the requested bound
output expression, parses the printed complex rows, and writes normalized
spectrum, summary, and convergence artifacts. The opt-in real-solver
conformance command `CIRCUITCI_RUN_REAL_NGSPICE_DISTO=1 cargo test --test
analog_distortion_cli` passed on this host with `/opt/homebrew/bin/ngspice`.

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

## Adapter Evidence Requirements

The enabled ngspice adapter provides:

- primary documentation for command/source syntax and output variables
- deterministic wrapper/deck generation that annotates `DISTOF1`/`DISTOF2`
  source inputs
- raw output retention
- normalized `distortion_spectrum.csv` with frequency, component label,
  complex value, magnitude, and phase
- normalized `distortion_summary.csv` for harmonic and intermodulation
  components
- normalized `distortion_convergence.json` with backend, mode, row count,
  selected components, output expression, and non-convergence status
- convergence/numerical-failure detection through the shared ngspice log scanner
- `solver_manifest.json` with backend version, command, model provenance, and
  normalized artifact paths
- opt-in real-ngspice conformance coverage with clean skip behavior

The report/GUI surface is also covered: validation reports project
`distortion_summary.csv` rows into top-level `distortion_summaries[]`,
Markdown reports include a "Distortion Summary" section, and GUI Scopes loads
`distortion_spectrum.csv` as frequency-axis magnitude, phase, real, and
imaginary traces per distortion component.
