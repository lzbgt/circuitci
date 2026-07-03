# PSS / Oscillator Backend Evidence

Date: 2026-07-04

This note records the backend decision for CircuitCI `analog_pss` support.
Original sources are stored in
`docs/research/circuit_simulation_full_featured/sources/`.

## Source Artifacts

- `sources/ngspice_ANALYSES`
- `sources/ngspice_manual.xhtml`
- `sources/Xyce_Reference_Guide_7.8.txt`
- `sources/Xyce_Users_Guide_7.8.txt`
- `sources/qucs_rf_simulation.html`
- `sources/spiceopus_nutmeg.html`
- `sources/spiceopus_release.html`

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

Until then, `analog_pss` is valuable as machine-readable oscillator intent,
frequency-guess, stabilization, convergence, and provenance evidence, but it
must not pass oscillator sign-off.
