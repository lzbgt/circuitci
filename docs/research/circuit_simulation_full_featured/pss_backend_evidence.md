# PSS / Oscillator Backend Evidence

Date: 2026-07-04

This note records the backend decision for CircuitCI `analog_pss` support.
Original sources are stored in
`docs/research/circuit_simulation_full_featured/sources/`.
PSS fail-closed findings carry compact `adapter_blocker` and
`evidence_sources[]` metadata pointing to this note and the retained primary
sources, so reports and GUI views explain the blocked backend boundary without
requiring raw JSON inspection.

## Source Artifacts

- `sources/ngspice_ANALYSES`
- `sources/ngspice_manual.xhtml`
- `sources/Xyce_Reference_Guide_7.8.txt`
- `sources/Xyce_Users_Guide_7.8.txt`
- `sources/qucs_rf_simulation.html`
- `sources/spiceopus_nutmeg.html`
- `sources/spiceopus_release.html`
- `sources/arxiv_2512.10373_source.tar.gz`
- `sources/arxiv_2512.10373_source_README.json`
- `sources/arxiv_2603.07828_qucs_phase_noise_part2.pdf`
- `sources/arxiv_2603.07828_qucs_phase_noise_part2.html`
- `sources/arxiv_2603.07828_qucs_phase_noise_part2.txt`
- `sources/arxiv_2603.07828_source.tar.gz`
- `sources/arxiv_2603.07828_source_README.json`
- `sources/github_search_qucs_copen_repositories.json`
- `sources/github_search_psssolver_qucs_repositories.json`
- `sources/github_search_qucs_copen_code.json`
- `sources/github_repo_qucs_qucs.json`
- `sources/github_repo_ra3xdh_qucs_s.json`
- `sources/github_repo_ra3xdh_qucsator_rf.json`

## Findings

- Xyce 7.8 documents `.HB`, `.OPTIONS HBINT`, `.PRINT HB`,
  `.PRINT HB_FD`, `.PRINT HB_TD`, `.PRINT HB_IC`, and
  `.PRINT HB_STARTUP`, but no distinct `.PSS` analysis command was found in
  the saved Xyce user/reference guides. CircuitCI should not relabel Xyce HB
  as oscillator PSS.
- ngspice documents `.pss gfreq tstab oscnob psspoints harms sciter
  steadycoeff <uic>` for periodic steady-state analysis. The manual also marks
  PSS as an experimental feature, states that it is autonomous-circuit focused,
  and lists `--enable-pss` as the build option.
- The installed host ngspice `46` recognizes `.pss`, but a minimal probe did
  not produce a trusted normalized output artifact. The probe log is temporary
  evidence at `/tmp/circuitci_pss_probe.log`; it is not committed because it is
  host-specific diagnostic output.
- SPICE OPUS documents the `ssse` nutmeg command for transient-domain shooting
  with extrapolation:
  `ssse v(<posnode>[,<negnode>]) [<level> [<step> [<skip> [<periods>]]]] [history]`.
  That is real OSS-adjacent evidence for shooting PSS, but CircuitCI currently
  has no SPICE OPUS runtime adapter, version capture, output normalizer, or
  conformance fixtures.
- Qucs-S RF documentation maps steady-state shooting to SPICE OPUS `ssse` and
  HB to Xyce/QucsatorRF-style workflows, supporting the separation between HB
  spectrum evidence and oscillator PSS evidence.
- The QUCS-COPEN Part I arXiv paper describes a new `psssolver` C++ class that
  inherits from qucsator `trsolver`, produces stabilization transient, PSS
  time-domain, and absolute-spectrum datasets, and validates oscillator
  frequencies against Keysight ADS. The Part II paper describes a companion
  `pnsolver` C++ class that depends on a referenced `psssolver` instance.
- The arXiv source archives for Part I and Part II contain TeX and figure
  sources only; they do not include reusable `psssolver` or `pnsolver` source
  code. GitHub repository searches saved in this folder returned zero
  repositories for `QUCS-COPEN` and `psssolver qucs`; unauthenticated GitHub
  code search requires authentication and is saved only as an API limitation
  artifact.

## Decision

Do not add a runnable PSS adapter yet.

CircuitCI should keep `SPICE_PSS_ANALYSIS` fail-closed until one of these is
true:

- ngspice PSS has a stable public output contract that CircuitCI can normalize
  into `pss_waveform`, `pss_spectrum`, and `pss_convergence`, with opt-in
  real-ngspice conformance fixtures;
- SPICE OPUS is added as an explicit backend with a versioned runtime adapter,
  `ssse` wrapper generation, normalized output parsing, and conformance
  fixtures;
- another OSS backend exposes a documented PSS/shooting contract suitable for
  deterministic CI artifacts.
- QUCS-COPEN source becomes publicly available with a license, build
  instructions, versioned CLI/runtime invocation, output dataset contract, and
  conformance circuits.

Until then, `analog_pss` is valuable as machine-readable oscillator intent,
frequency-guess, stabilization, convergence, and provenance evidence, but it
must not pass oscillator sign-off.
