# PAC/PXF Backend Evidence Boundary

Date: 2026-07-04

## Decision

CircuitCI should keep `SPICE_PERIODIC_AC_ANALYSIS` fail-closed. The current
primary-source review does not identify a trustworthy open-source backend path
that can execute PAC/PXF-style periodic small-signal analysis and emit the
normalized evidence contract required by CircuitCI:

- `pac_response`
- `pac_sidebands`
- `pac_convergence`
- `pss_convergence`
- raw solver output
- `solver_manifest.json`

The `analog_periodic_ac` contract is still useful as durable intent: it records
carrier frequency, sideband count, small-signal sweep bounds, output
expression, input source, drive-source provenance, and the required future
evidence without allowing false pass results.

## Saved Sources

- `sources/Xyce_Reference_Guide_7.8.pdf`
- `sources/Xyce_Reference_Guide_7.8.txt`
- `sources/Xyce_AppNote_GenExt.pdf`
- `sources/Xyce_AppNote_GenExt.txt`
- `sources/ngspice_manual.xhtml`
- `sources/ngspice_pss_periodic_steady_state.html`
- `sources/arxiv_2512.10373_qucs_phase_noise.pdf`
- `sources/arxiv_2512.10373_qucs_phase_noise.txt`
- `sources/arxiv_2603.07828_qucs_phase_noise_part2.pdf`
- `sources/arxiv_2603.07828_qucs_phase_noise_part2.txt`
- `sources/qucs_technical.html`

## Xyce

The Xyce 7.8 Reference Guide documents `.HB` and `.PRINT HB`/`HB_FD` output,
including CSV frequency-domain harmonic-balance output. The same saved
reference/user-guide text did not contain `PAC` or `PXF` command syntax.

The Xyce external-interface application note strengthens the boundary: it says
frequency-domain coupling is limited to harmonic balance at this time and that
Xyce AC analysis does not support direct frequency-domain loads. That is useful
for CircuitCI's HB adapter, but it is not a PAC/PXF adapter contract. Treating
HB spectra as periodic small-signal PAC/PXF response would drop the required
small-signal injection, sideband mapping, and periodic-linearization
convergence evidence.

Current status: `backend: xyce` must remain fail-closed for
`SPICE_PERIODIC_AC_ANALYSIS` until a documented PAC/PXF-equivalent command,
output schema, and real-Xyce conformance fixture exist.

## ngspice

The saved ngspice manual describes PSS as experimental, based on time-domain
shooting, and limited to autonomous circuits in the documented text. It also
states that PSS results are the basis for periodical large-signal analyses such
as PAC or PNoise. That statement is not enough for an adapter because the saved
manual material does not provide a stable PAC/PXF command syntax, output
format, convergence artifact, or build/runtime conformance contract.

The QUCS-COPEN Part II paper independently notes this gap: it quotes the
ngspice PSS/PAC/PNoise relationship, then states that PAC or PNoise are not
otherwise documented in the manual and concludes that the features are not
properly implemented, tested, or documented for credible use.

Current status: `backend: ngspice` must remain fail-closed for
`SPICE_PERIODIC_AC_ANALYSIS`. Standard `.AC` is already supported by
`SPICE_AC_ANALYSIS`, but `.AC` linearizes about a DC operating point and is not
periodic small-signal analysis.

## QUCS / QUCS-COPEN

The saved QUCS technical page includes harmonic-balance material, but it does
not provide an installable PAC/PXF backend target with CircuitCI-ready command
syntax and artifacts.

The QUCS-COPEN papers are valuable theory references for PSS and PNOISE. They
describe a PSS solver and a PNOISE solver coupled to that PSS object, with
datasets for PSS time-domain and spectrum outputs. Previous repository
searches saved under `sources/github_search_*` did not find a public source
repository, build path, license, or adapter contract for these modules.

Current status: QUCS-COPEN remains a theory/reference source, not an
executable PAC/PXF backend target.

## Required Adapter Evidence Before Enabling

A future PAC/PXF adapter must provide all of the following before CircuitCI may
mark `SPICE_PERIODIC_AC_ANALYSIS` passing:

- primary documentation for command syntax and output files
- deterministic wrapper/deck generation
- raw output retention
- normalized `pac_response.csv` with frequency, sideband/harmonic index,
  complex value, magnitude, and phase
- normalized sideband metadata for input/output mapping
- convergence metadata from the periodic operating point and periodic
  linearization
- `solver_manifest.json` with backend version, command, source netlist, model
  provenance, and normalized artifact paths
- opt-in real-solver conformance coverage that skips cleanly when the backend
  is unavailable
