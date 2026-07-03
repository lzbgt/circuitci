# Full Featured Circuit Simulation Research

Date: 2026-07-03

User goal: make CircuitCI support full featured time-domain and frequency-domain
circuit simulation. This research note reviews mainstream open-source circuit
analysis projects and selected arXiv papers, then translates the findings into
a realistic CircuitCI roadmap.

Original source artifacts are saved under
`docs/research/circuit_simulation_full_featured/sources/`.

## Executive Conclusion

CircuitCI should not try to become a complete native SPICE-class simulator from
scratch in the near term. Full featured circuit simulation requires a large
solver stack: netlist parsing, modified nodal analysis (MNA), nonlinear
Newton-Raphson iteration, adaptive transient integration, sparse linear algebra,
compact semiconductor models, Verilog-A/OSDI model loading, convergence aids,
noise models, RF/frequency-domain analysis, periodic steady state (PSS),
harmonic balance (HB), sensitivity, measurements, and waveform storage.

The highest-leverage path is:

1. Make CircuitCI a strong simulation orchestrator with a stable analysis IR,
   deterministic execution contracts, evidence manifests, and report schemas.
2. Support multiple proven external kernels first: ngspice for broad SPICE
   compatibility and Xyce for high-performance/parallel simulation plus HB.
3. Add model ecosystem support through Verilog-A/OSDI tooling such as OpenVAF.
4. Build conformance tests and result normalization around known circuits before
   considering any native solver work.
5. Only build native solver pieces when they serve CircuitCI-specific
   verification, such as tiny linear MNA checks, reduced-order sanity screens,
   or deterministic test fixtures.

## What Full Featured Means

A credible "full featured" target for CircuitCI should include:

- Operating point: `.OP`.
- DC sweep and transfer function: `.DC`, `.TF`.
- Time domain: `.TRAN`, adaptive timestep control, initial conditions,
  transient noise, `.MEASURE`, waveform output, and event/source support.
- Frequency domain: `.AC`, `.NOISE`, pole-zero `.PZ`, sensitivity `.SENS`,
  Fourier/spectrum measurements, and S-parameter workflows.
- Nonlinear periodic/RF: HB for driven large-signal steady state, PSS/shooting
  for oscillators/autonomous circuits, and phase-noise direction eventually.
- Sweeps and statistics: parameter sweeps, temperature sweeps, corners,
  Monte Carlo, yield and worst-corner reporting.
- Models: primitive R/L/C/sources, diodes, BJTs, JFETs, MOSFETs, BSIM-class
  compact models, subcircuits, behavioral sources, and Verilog-A/OSDI models.
- Integration: stable raw/waveform formats, report artifacts, provenance,
  error classification, solver version/config capture, cancellation, memory
  bounds, and GUI inspection.

CircuitCI already has bounded support for transient, AC, DC operating point,
noise, sweeps, generated decks, ngspice/libngspice execution, and GUI waveform
inspection. The missing work is breadth, backend abstraction, RF/PSS/HB,
conformance, and model ecosystem depth.

## Open Source Project Findings

### ngspice

Sources saved:

- `sources/ngspice_README`
- `sources/ngspice_ANALYSES`
- `sources/ngspice_DEVICES`
- `sources/ngspice_manual.xhtml`
- `pss_backend_evidence.md` records the PSS-specific backend decision.

Upstream:

- <https://github.com/ngspice/ngspice>
- <https://ngspice.sourceforge.io/docs.html>

Key facts:

- ngspice is based on Spice3f5, Cider, and XSPICE heritage.
- It supports major classic SPICE analyses: noise, operating point, DC sweep,
  pole-zero, distortion, AC, sensitivity, transfer function, transient, and
  experimental PSS.
- The PSS path is experimental, autonomous-circuit focused, and build-gated by
  `--enable-pss`; CircuitCI should keep it fail-closed until stable normalized
  outputs and real-solver conformance exist.
- The manual describes DC, AC, and transient as core analysis modes, with DC
  operating point used before transient and small-signal AC linearization.
- It has broad compact-device history and Verilog-A/XSPICE-related model
  mechanisms.

CircuitCI implication:

- ngspice should remain the default broad-compatibility backend.
- CircuitCI should expose more ngspice analyses through typed Board IR instead
  of hand-authored raw `.control` scripts only.
- The near-term win is better orchestration and result normalization, not
  replacing ngspice.

### Xyce

Sources saved:

- `sources/xyce_README.md`
- `sources/xyce_INSTALL.md`
- `sources/Xyce_Users_Guide_7.8.pdf`
- `sources/Xyce_Users_Guide_7.8.txt`
- `sources/Xyce_Reference_Guide_7.8.pdf`
- `sources/Xyce_Reference_Guide_7.8.txt`
- `pss_backend_evidence.md` records why Xyce HB is not treated as PSS.

Upstream:

- <https://github.com/Xyce/Xyce>
- <https://xyce.sandia.gov/>

Key facts:

- Xyce is an open-source, SPICE-compatible, high-performance analog circuit
  simulator.
- It supports DC, transient, AC, small-signal noise, harmonic balance,
  sensitivity, uncertainty/random sampling, `.FOUR`, and `.MEASURE`.
- Xyce 7.8 documents HB and HB time/frequency-domain print artifacts, but the
  saved guides do not document a separate `.PSS` command.
- Xyce was designed from scratch for parallel simulation using MPI and
  Trilinos-style solver infrastructure.
- It supports canonical SPICE compact models and custom models through its
  Verilog-A model compiler / XDM ecosystem.

CircuitCI implication:

- Xyce should be a first-class backend when users want large simulations,
  parallel execution, harmonic balance, or stronger high-performance behavior.
- The backend should be optional and version/provenance-captured, because
  packaging and available compact models differ by platform and installer.

### Qucs-S and QucsatorRF

Sources saved:

- `sources/qucs_s_README.md`
- `sources/qucs_s_home.html`
- `sources/qucs_rf_simulation.html`
- `sources/qucsator_rf_README.md`
- `sources/spiceopus_nutmeg.html`
- `sources/spiceopus_release.html`

Upstream:

- <https://github.com/ra3xdh/qucs_s>
- <https://ra3xdh.github.io/>
- <https://github.com/ra3xdh/qucsator_rf>

Key facts:

- Qucs-S is primarily a GUI and workflow layer around multiple kernels:
  ngspice, Xyce, SpiceOpus, Qucsator, and QucsatorRF.
- Its published feature list includes AC, DC, transient, S-parameter, FFT,
  distortion, pole-zero, noise, HB, parameter sweep, and RF workflows.
- The RF documentation explains that HB solves nonlinear steady state in the
  frequency domain using truncated Fourier representations, and that HB can be
  more efficient than transient when long settling time dominates.
- The same RF documentation and the saved SPICE OPUS primary pages document
  SPICE OPUS `ssse` transient-domain shooting, but CircuitCI has no SPICE OPUS
  backend or conformance path yet.
- QucsatorRF targets RF and microwave circuit simulation.

CircuitCI implication:

- Qucs-S is evidence that a multi-kernel orchestration layer is a practical OSS
  pattern.
- CircuitCI should copy the architectural lesson, not necessarily the GUI.
  It should define kernel-independent analysis requests and normalize outputs.

### Gnucap

Sources saved:

- `sources/gnucap_README`
- `sources/gnucap_INSTALL`

Upstream:

- <https://git.savannah.gnu.org/cgit/gnucap/gnucap.git>
- <https://github.com/gnucap/gnucap>

Key facts:

- Gnucap is a modular circuit analysis package with a plugin-oriented structure.
- Its README emphasizes Verilog-AMS direction and independent plugin modules.

CircuitCI implication:

- Gnucap is most relevant as an architecture reference for modularity and
  language/model plugin boundaries.
- It is less directly compelling than ngspice/Xyce as the first backend target
  for CircuitCI's full time/frequency feature expansion.

### SpiceSharp

Sources saved:

- `sources/spicesharp_README.md`
- `sources/spicesharp_home.html`

Upstream:

- <https://github.com/SpiceSharp/SpiceSharp>
- <https://spicesharp.github.io/SpiceSharp/>

Key facts:

- SpiceSharp is an embeddable circuit simulation library with customizable
  simulations, models, integration methods, and solvers.

CircuitCI implication:

- It is useful as a library-design reference. Because CircuitCI is Rust, direct
  adoption would add language/runtime friction unless isolated behind a process
  or FFI boundary.

### ahkab

Sources saved:

- `sources/ahkab_README.md`
- `sources/ahkab_home.html`

Upstream:

- <https://github.com/ahkab/ahkab>
- <https://ahkab.github.io/ahkab/>

Key facts:

- ahkab supports operating point, DC sweep, transient analysis with implicit
  Euler/trapezoidal/Gear formulas, AC, pole-zero, non-autonomous PSS by shooting
  or brute force, and symbolic small-signal analysis.

CircuitCI implication:

- ahkab is valuable as a readable reference for analysis semantics and test
  cases, but it should not be the primary production backend for CircuitCI.

### OpenVAF

Sources saved:

- `sources/openvaf_README.md`
- `sources/openvaf_home.html`
- `sources/openvaf_usage.html`
- `sources/openvaf_osdi_details.html`
- `sources/osdi_v0p3.pdf`
- `sources/ngspice_osdi.html`
- `sources/xyce_adms_users_guide.html`
- `sources/xyce_tutorial_adding_device.html`
- `xyce_openvaf_osdi_compatibility.md`

Upstream:

- <https://github.com/pascalkuthe/OpenVAF>
- <https://openvaf.semimod.de/>
- <https://ngspice.sourceforge.io/osdi.html>
- <https://xyce.sandia.gov/documentation-tutorials/xyce-adms-users-guide/>

Key facts:

- OpenVAF compiles Verilog-A files for use in circuit simulators.
- It can produce simulator-independent OSDI shared objects and targets compact
  model support.
- OpenVAF/OSDI is an external-ngspice model-loading path in CircuitCI. Primary
  Xyce docs describe Xyce/ADMS-generated C++ linked into Xyce or loaded with
  Xyce `-plugin`, not OpenVAF `*.osdi` runtime loading.
- The README states that some Verilog-A features remain unsupported, so model
  coverage must be tested and pinned.

CircuitCI implication:

- Model availability is a bigger bottleneck than analysis command support.
  OpenVAF/OSDI support should be part of any serious "full simulation" roadmap.
  Xyce compact-model support now has a separate fail-closed Xyce/ADMS plugin
  artifact contract plus opt-in plugin build/load qualification; execution
  still requires a real-Xyce plugin loader.

## arXiv Paper Findings

### Topological decoupling of MNA

Sources saved:

- `sources/arxiv_2604.20475_mna_decoupling.pdf`
- `sources/arxiv_2604.20475_mna_decoupling.txt`

Upstream:

- <https://arxiv.org/abs/2604.20475>

Key point:

- MNA is a differential-algebraic equation (DAE) foundation for SPICE-like
  simulators. The paper derives graph-based decoupling of MNA into a
  semi-explicit index-one DAE including controlled sources, preserving sparsity
  and structure.

CircuitCI implication:

- Native solver work must treat MNA as DAE math, not as ordinary ODE stepping.
  Graph structure in CircuitCI's Board IR could help with consistency checks
  and reduced solver preflight, but a full simulator is still a large project.

### EEspice parallel stamping

Sources saved:

- `sources/arxiv_2604.03079_eespice.pdf`
- `sources/arxiv_2604.03079_eespice.txt`

Upstream:

- <https://arxiv.org/abs/2604.03079>

Key point:

- SPICE transient analysis repeatedly performs nonlinear device evaluation,
  MNA stamping, Newton-Raphson iteration, and sparse linear solves. Parallel
  device evaluation alone leaves stamping as a bottleneck. EEspice explores
  graph-coloring-based parallel stamping for MOSFET-heavy circuits.

CircuitCI implication:

- Performance work should be backend-aware. For large transistor-level runs,
  backend choice and sparse/parallel solver architecture matter more than GUI
  features.

### Transient forward harmonic adjoint sensitivity

Sources saved:

- `sources/arxiv_2401.13496_tfha.pdf`
- `sources/arxiv_2401.13496_tfha.txt`

Upstream:

- <https://arxiv.org/abs/2401.13496>

Key point:

- Time-domain transient simulation and frequency-domain harmonic balance have
  complementary strengths. Hybrid transient/harmonic sensitivity methods can
  avoid some HB convergence costs while extracting frequency-domain sensitivity.

CircuitCI implication:

- Long-term sensitivity/yield features should be represented as analysis
  contracts independent of a specific solver, because different backends may
  implement transient, HB, and adjoint workflows differently.

### Data-driven MNA

Sources saved:

- `sources/arxiv_2303.03401_data_driven_mna.pdf`
- `sources/arxiv_2303.03401_data_driven_mna.txt`

Upstream:

- <https://arxiv.org/abs/2303.03401>

Key point:

- The paper reformulates MNA to use measurement data directly for elements
  whose model equations are unavailable, minimizing distance between states
  satisfying Kirchhoff laws and measured element behavior.

CircuitCI implication:

- This is relevant to future source-backed component models and lab-derived
  behavioral evidence, but it is research-grade. It should not replace SPICE
  model integration in the near-term roadmap.

### QUCS PSS / phase-noise direction

Sources saved:

- `sources/arxiv_2512.10373_qucs_phase_noise.pdf`
- `sources/arxiv_2512.10373_qucs_phase_noise.txt`
- `sources/arxiv_2512.10373_source.tar.gz`
- `sources/arxiv_2512.10373_source_README.json`
- `sources/arxiv_2603.07828_qucs_phase_noise_part2.pdf`
- `sources/arxiv_2603.07828_qucs_phase_noise_part2.html`
- `sources/arxiv_2603.07828_qucs_phase_noise_part2.txt`
- `sources/arxiv_2603.07828_source.tar.gz`
- `sources/arxiv_2603.07828_source_README.json`

Upstream:

- <https://arxiv.org/abs/2512.10373>

Key point:

- The paper extends QUCS toward time-domain PSS for autonomous oscillator
  circuits and phase-noise simulation. It highlights PSS convergence, basin of
  attraction, frequency guess quality, and stiff circuit dynamics.
- Part II documents a companion QUCS-COPEN PNOISE module that depends directly
  on a referenced PSS module instance. GitHub repository searches saved under
  `sources/github_search_*` found no public `QUCS-COPEN` or `psssolver qucs`
  repositories, and the arXiv source archives contain TeX/figure sources rather
  than reusable backend code.

CircuitCI implication:

- Oscillator/RF large-signal support is not just "run FFT after transient."
  It needs explicit PSS/HB contracts, convergence evidence, and careful
  limitations in reports.
- QUCS-COPEN is a valuable theory/reference path for autonomous oscillator PSS
  and PNOISE, but it is not a practical backend target until public source,
  build instructions, output datasets, and conformance circuits exist.

### SPICE for power-grid transient analysis

Sources saved:

- `sources/arxiv_2305.09122_power_grid_transient.pdf`
- `sources/arxiv_2305.09122_power_grid_transient.txt`

Upstream:

- <https://arxiv.org/abs/2305.09122>

Key point:

- SPICE-like simulation can be used outside small PCB circuits for transient
  system studies, but scaling, modeling assumptions, and solver choices matter.

CircuitCI implication:

- CircuitCI should preserve model-scope declarations and limitations for every
  generated simulation. A solver run is only as meaningful as the model fidelity
  and source evidence.

## Recommended CircuitCI Architecture

### 1. Analysis IR

Add a solver-independent analysis layer that can represent:

- OP/DC/TF/PZ/AC/NOISE/TRAN/FOUR/MEASURE.
- HB/PSS/S-parameter requests as optional advanced contracts.
- Sweep axes: parameters, component values, model sections, temperature,
  corners, Monte Carlo samples.
- Solver requirements: backend name, version constraints, model loaders,
  tolerances, max time/frequency points, memory budget, and timeout.
- Output requests: probes, branch currents, powers, waveform formats,
  measurements, sampled spectra, and report summaries.

This should be Board IR data, not only command-line flags.

### 2. Backend Adapters

Create explicit backend adapters:

- `ngspice`: default compatibility backend.
- `xyce`: high-performance and HB-capable backend.
- Future: `qucsator_rf` or `spiceopus` only if the use case is strong.

Each adapter should expose:

- Capability discovery.
- Version/config capture.
- Netlist emission.
- Process or library execution.
- Log classification.
- Artifact manifest.
- Result parsing into a common schema.
- Fail-closed unsupported-feature behavior.

### 3. Result Normalization

Normalize all solver outputs to CircuitCI artifacts:

- `operating_point.csv`
- `dc_sweep.csv`
- `transient.csv`
- `ac_bode.csv`
- `noise_spectrum.csv`
- `noise_total.csv`
- `s_parameters.csv` normalized from Touchstone and referenced by
  `solver_manifest.json`
- `harmonic_balance.csv`
- `pss_summary.json`
- `measurements.json`
- `solver_manifest.json`

The GUI and validators should consume these normalized files, not backend-
specific raw formats directly.

### 4. Model Ecosystem

Add a model pipeline:

- SHA-pinned SPICE include and model files.
- Verilog-A source metadata.
- OpenVAF/OSDI compiled artifact metadata.
- Backend compatibility matrix per model.
- Explicit reduced-fidelity labels for generated models.
- Model license/provenance fields.

Without this, "full simulation" will fail on real boards even if analyses are
available.

### 5. Conformance Suite

Before adding broad features, create a conformance suite with small canonical
circuits:

- RC transient and AC low-pass.
- RLC resonance and pole-zero.
- Diode rectifier transient and Fourier.
- Op-amp small-signal gain/bandwidth.
- BJT/MOS amplifier bias and AC.
- Switching converter reduced model.
- Oscillator PSS/HB/PAC placeholders with explicit backend requirements.
- S-parameter two-port fixture.

Each fixture should have expected tolerance windows for ngspice and Xyce.

### 6. GUI Scope

The GUI should grow around:

- Analysis setup forms.
- Backend/capability diagnostics.
- Solver log and convergence inspection.
- Waveform/spectrum/S-parameter plotting.
- Measurement tables.
- Run-to-schematic context.

The GUI should not become the solver.

## Implementation Priority

### Phase 1: Backend Capability Model

- Add backend capability discovery for ngspice and Xyce.
- Add a common solver manifest schema.
- Add typed OP/DC/TRAN/AC/NOISE run contracts.
- Normalize outputs.
- Preserve current validation behavior.

### Phase 2: Frequency-Domain Breadth

- Add `.TF`, `.PZ`, `.SENS`, `.FOUR`, and `.MEASURE` support where available.
- The first `.TF` path is external-ngspice execution with normalized
  `transfer_function_summary` plus opt-in real-ngspice conformance coverage;
  next add Xyce planning or adapter support where the backend exposes
  equivalent output.
- The first `.PZ` path is an external-ngspice adapter with a Board IR/schema
  contract for output/reference nodes, input source, mode, and normalized
  `pole_zero_summary` plus opt-in real-ngspice conformance; next add
  non-ngspice planning/adapters.
- The first `.SENS` path is an external-ngspice adapter with a Board IR/schema
  contract for DC or AC output sensitivity, optional filters, normalized
  `sensitivity_summary`, solver manifests, and opt-in real-ngspice conformance;
  next add non-ngspice planning/adapters.
- The first `.FOUR` path is an external-ngspice adapter with a Board IR/schema
  contract for transient-backed harmonic extraction, fundamental
  frequency/window validation, bound output provenance, normalized
  `fourier_summary`, solver manifests, and opt-in real-ngspice conformance;
  next add non-ngspice planning/adapters.
- The first `.MEASURE` path is an external-ngspice adapter with a Board
  IR/schema contract for reviewed transient/AC scalar extraction statements or
  portable measure templates, bound output provenance checks, normalized
  `measure_summary`, solver manifests, and opt-in real-ngspice conformance.
  Explicit Xyce now supports the portable `measure_templates[]` subset and
  emits the same `measure_summary`/manifest contract. The adapter records Xyce
  measure result files such as `.mt0` and includes opt-in real-Xyce template
  conformance through
  `CIRCUITCI_RUN_REAL_XYCE=1 cargo test --test analog_measure_cli`; raw
  `measure_statements[]` remain ngspice-only because their syntax is
  backend-specific. Portable templates now include simple scalar operations and
  transient threshold delay/slew/window crossing time through SPICE
  `TRIG`/`TARG` and `WHEN`. Normalized `measure_summary` rows can now feed
  `measure_assertions[]` so scalar simulation specs directly pass or fail
  validation, including shared worst-corner and Monte Carlo yield summaries
  across swept measure corners.
- The first DC sweep path is an external-ngspice adapter with a Board
  IR/schema contract for `analysis.type: dc_sweep`, swept-source start/stop/step
  fields, normalized `dc_sweep` curve rows, solver manifests, and
  `dc_sweep_assertions[]` for min/max/mean/sample scalar specs. Explicit Xyce
  now emits the same normalized `dc_sweep`/manifest contract. Opt-in
  real-solver conformance is available through
  `CIRCUITCI_RUN_REAL_NGSPICE=1 cargo test --test analog_dc_sweep_cli` and
  `CIRCUITCI_RUN_REAL_XYCE=1 cargo test --test analog_dc_sweep_cli`; each path
  skips unless the requested solver is on `PATH`. Embedded ngspice remains
  fail-closed with planning evidence until equivalent output normalization is
  implemented.
- The first harmonic-balance path is an explicit-Xyce adapter plus Board
  IR/schema contract for `analysis.type: hb`, declared periodic drive sources,
  a fundamental frequency, bound output expression, optional harmonic count,
  normalized `hb_spectrum` evidence, and solver-manifest metadata. The saved
  Xyce 7.8 Reference Guide documents `.HB <fundamental frequencies>`,
  `.OPTIONS HBINT NUMFREQ=...`, and `.PRINT HB_FD FORMAT=CSV`; CircuitCI uses
  those commands to write `hb_spectrum_raw.csv` and normalize signed harmonic
  complex spectrum rows. This keeps HB distinct from transient `.FOUR`
  extraction: Fourier analysis measures a transient waveform after integration,
  while HB solves the periodic steady-state frequency-domain problem directly.
  GUI Scopes now recognizes `hb_spectrum.csv` artifacts and adapts the
  non-negative harmonic rows into magnitude, phase, real, and imaginary
  frequency-axis traces.
  Opt-in real-Xyce HB conformance is covered by
  `CIRCUITCI_RUN_REAL_XYCE=1 cargo test --test analog_harmonic_balance_cli`;
  the test skips unless `Xyce` or `xyce` is on `PATH`.
- Extend the initial explicit-Xyce S-parameter path from opt-in real-solver
  conformance coverage into supported two-port test-bench generation.
- Add report limitations for backend-specific gaps.

### Phase 3: Xyce HB Path

- Add Xyce backend execution.
- Add `.HB` Board IR contract.
- Add HB artifact parser and GUI spectrum view. Done; Scopes loads
  `hb_spectrum.csv` as frequency-domain spectrum traces.
- Keep HB optional and fail-closed when Xyce is unavailable.

### Phase 4: PSS / Oscillator Evidence

- Add PSS contracts only after HB and result normalization are stable.
- Require convergence metadata, frequency guess, stabilization interval,
  residual/error thresholds, and explicit limitations.
- Added the first fail-closed Board IR/schema contract for
  `analysis.type: pss` and `SPICE_PSS_ANALYSIS`. Scenarios declare
  `pss_mode: driven|autonomous`, `pss_frequency_guess_hz`,
  `pss_stabilization_time_us`, a bound `pss_output_expression`, optional
  periods/iteration limits, optional residual and state-error tolerances, and
  driven-source provenance. The validator records manifest-compatible planning
  evidence for future `pss_waveform`, `pss_spectrum`, and `pss_convergence`
  normalized outputs, and intentionally fails closed until a trusted backend
  adapter emits those artifacts.
- Primary-source review in `pss_backend_evidence.md` found no trustworthy
  adapter target to enable immediately: Xyce 7.8 documents HB but not a
  separate PSS command; ngspice PSS is experimental/autonomous/build-gated and
  lacks a stable normalized output contract in this runtime; SPICE OPUS `ssse`
  is documented but would require a new backend adapter and conformance suite;
  QUCS-COPEN papers document `psssolver`/`pnsolver`, but no public source
  repository or adapter contract was found.
- Added the first fail-closed phase-noise evidence contract for
  `analysis.type: phase_noise` and `SPICE_PHASE_NOISE_ANALYSIS`. Scenarios
  declare carrier frequency, offset sweep bounds, output expression, optional
  integration window, and driven-source provenance when driven. The validator
  records required future artifacts `phase_noise_spectrum`,
  `phase_noise_integrated_jitter`, `phase_noise_convergence`, and
  `pss_convergence`, and intentionally fails closed until a trusted PSS/PNOISE
  solver chain plus real-solver conformance exists.

### Phase 5: Model Compiler Pipeline

- Added the first OpenVAF/OSDI artifact metadata gate. Analog `model_files[]`
  entries for compiled Verilog-A compact models can now declare
  `artifact_format: osdi_shared_object`, compiled artifact SHA, Verilog-A
  `source_path`/`source_sha256`, `compiler: openvaf`, compiler version, and
  compiler command. CircuitCI validates source availability and source hash
  before solver planning and fails closed when provenance is incomplete. The
  preflight also validates that the command invokes `openvaf` and references
  the declared source/output artifact.
- Added executable OpenVAF build/check planning for compiled OSDI artifacts:
  when the artifact is missing or hash-stale, the failure carries the declared
  compiler command, output path, and `openvaf` availability on `PATH` so CI can
  rebuild from pinned Verilog-A source before rerunning simulation.
- Added an opt-in OpenVAF execution path gated by
  `CIRCUITCI_RUN_OPENVAF_BUILDS=1`. CircuitCI executes the declared `openvaf`
  command directly from the project directory, rejects shell metacharacters in
  the command contract, and rechecks the produced OSDI artifact hash before
  solver planning continues.
- Added the first backend compatibility check for OpenVAF/OSDI models:
  generated Board IR netlists skip `.include` lines for OSDI binaries, external
  ngspice wrappers emit `pre_osdi` load commands, and runtimes that reject OSDI
  loading fail closed with wrapper/log evidence.
- Added opt-in real-ngspice OSDI conformance coverage using a small
  OpenVAF-compatible Verilog-A fixture. The command is
  `CIRCUITCI_RUN_REAL_NGSPICE_OSDI=1 cargo test --test analog_model_compiler_cli`;
  it compiles the fixture with OpenVAF to establish the expected artifact hash,
  deletes the artifact, lets CircuitCI rebuild it through the declared
  `compiler_command`, and verifies ngspice `pre_osdi` loading, normalized
  transient output, and solver manifest model-file provenance.
- Added solver-manifest generated-model provenance records for OpenVAF/OSDI
  artifacts. `inputs.model_file_provenance[]` records declared and actual
  source/artifact hashes, compiler identity and command, compiler availability,
  build-env state, rebuild mode, and whether CircuitCI produced the artifact in
  the current validation run.
- Added report and GUI projection for OpenVAF/OSDI generated-model provenance:
  validation report JSON/Markdown and the Scopes artifact drawer now surface
  `model_file_provenance[]` with scenario, analysis, backend, manifest path,
  rebuild mode, and declared/actual hash evidence.
- Researched the Xyce/OpenVAF/OSDI compatibility boundary from primary sources
  and saved the evidence in `xyce_openvaf_osdi_compatibility.md`. CircuitCI now
  rejects explicit Xyce or embedded-ngspice backends for OpenVAF `*.osdi`
  artifacts; future Xyce compact-model support needs a distinct Xyce/ADMS
  plugin artifact contract and real-Xyce conformance.
- Added a fail-closed Xyce/ADMS plugin artifact contract. Board IR accepts
  `artifact_format: xyce_adms_plugin` with pinned Verilog-A source, generated
  plugin, retained conformance artifact, `compiler: xyce_adms`,
  `buildxyceplugin` command, Xyce `-plugin` load command, Xyce version,
  Xyce/ADMS template revision, and shareable-build configure options. CircuitCI
  validates those fields and emits
  `ANALOG_MODEL_COMPILER_XYCE_PLUGIN_UNSUPPORTED` until a real-Xyce loader and
  conformance path are implemented.
- Added opt-in real-Xyce ADMS plugin qualification coverage:
  `CIRCUITCI_RUN_REAL_XYCE_ADMS_PLUGIN=1 cargo test --test analog_model_compiler_cli`.
  It skips unless `Xyce`/`xyce` and `buildxyceplugin` are on `PATH`; when
  runnable it builds the primary-source RLC Verilog-A plugin, loads it with
  `Xyce -plugin`, pins the retained conformance artifact, and verifies the
  fail-closed `xyce_adms_plugin` contract against those real artifacts.
- Added compact-model package-lock metadata for reusable model artifacts.
  `analog.model_files[]` can now carry `model_package_name`,
  `model_package_version`, `model_package_artifact_id`,
  `model_package_lock_path`, and `model_package_lock_sha256`; CircuitCI
  verifies the JSON/YAML lock package identity and artifact row against the
  scenario, retains the lock as an artifact, and projects package fields into
  solver manifests and report-level `model_file_provenance[]`.
- Added `schemas/model_package_lock.schema.json` plus pinned package registry
  imports. `analog.model_files[]` may now use
  `model_package_registry_path`, `model_package_registry_sha256`, and
  `model_package_registry_entry` to import package identity and lock pointers
  from a reusable registry. Registry files are hash-pinned, retained as report
  artifacts, and fail closed when entries are missing or conflict with explicit
  scenario metadata.
- Added `circuitci verify-model-package` and
  `schemas/model_package_verification_report.schema.json` so package authors
  can validate lock files, registry entries, and artifact hashes independently
  before scenarios depend on those compact-model packages.
- Added `circuitci export-model-package` so qualified compact-model packages can
  be generated deterministically from an artifact path, package identity,
  artifact format, and optional compiler/registry entry instead of being
  hand-authored before verification.
- Extended `circuitci export-model-package` with repeatable
  `--package-artifact` specs so one lock can qualify a full compact-model
  artifact set: Verilog-A source, OpenVAF/OSDI runtime object, Xyce/ADMS plugin,
  and retained conformance evidence. Registry export now supports an explicit
  `--registry-artifact-id` selector so shared package imports name the intended
  runtime artifact rather than relying on artifact order.
- Added `circuitci merge-model-package-registry` and
  `schemas/model_package_registry.schema.json` so exported compact-model
  registry entries can be aggregated into a shared deterministic registry. The
  command rewrites lock paths relative to the shared registry, deduplicates
  identical entries, and rejects conflicting duplicate ids.
- Added built-in reusable compact-model package fixtures for the generic
  analog behavioral SPICE include. Generated analog model-pack scenarios now
  infer the shared registry entry `generic_analog_behavioral_spice` from the
  component library and carry lock/registry pins into `analog.model_files[]`,
  solver manifests, report JSON, Markdown reports, and GUI artifact views.
- Added `repair-yaml --finding analog-model-package-metadata` so older
  generated analog scenarios can be migrated to package-qualified model-file
  metadata without editing the original project. The repair is additive,
  blocks on conflicting existing package fields, rewrites copied-project
  analog model/package paths to absolute paths for validation, and preserves
  stale-report proposal replay safeguards.
- Added `schemas/model_conformance_report.schema.json` plus semantic
  `verify-model-package` checks for `model_conformance_report` artifacts, so
  reusable compact-model packages now prove pass/fail behavioral qualification
  against a named runtime artifact SHA-256 instead of only pinning opaque
  conformance files by hash.
- Added `circuitci export-model-conformance-report`, which turns a CircuitCI
  validation `report.json` plus a hashed runtime artifact into deterministic
  `model_conformance_report` JSON. This closes the package-authoring loop:
  validate a compact-model scenario, generate evidence, export a multi-artifact
  lock, then verify the package without hand-authoring conformance JSON.
- Added `verify-model-package` `conformance_checks[]` projection so package
  reviewers can see qualified analyses, solvers, target artifact hashes, and
  referenced evidence artifacts in the package verification report without
  opening every retained conformance JSON file.
- Added package conformance surfacing outside raw JSON: `verify-model-package`
  now writes a sibling Markdown report, normal validation reports project
  retained package-verification artifacts into
  `model_package_conformance_checks[]`, and the GUI Simulation report panel
  displays those compact qualification rows.
- Added `circuitci export-model-package-bundle` and
  `schemas/model_package_bundle_manifest.schema.json` so a verified compact
  model package can be shipped as one deterministic directory containing a
  rewritten lock, optional registry, runtime/source/conformance artifacts,
  package verification JSON/Markdown, README, and bundle manifest. The bundled
  registry is immediately usable by `verify-model-package` and scenario
  package imports.
- Added `circuitci verify-model-package-bundle` and
  `schemas/model_package_bundle_verification_report.schema.json` so those
  portable directories can be validated as one unit before import or
  distribution. The verifier checks manifest, lock, optional registry,
  README/package-verification files, copied artifact hashes, projected
  conformance checks, and the bundled lock/registry through the normal package
  verifier.
- Added `circuitci install-model-package-bundle` and
  `schemas/model_package_bundle_install_report.schema.json` so a verified
  bundle can be copied into a project or shared model-package directory,
  optionally emit a shared registry entry, and produce scenario-ready registry
  path/SHA/entry and lock/artifact pins without manual path editing.
- Normal validation reports now project retained bundle verification/install
  reports into `model_package_bundle_verifications[]` and
  `model_package_bundle_installs[]`, with matching Markdown and GUI Scopes
  artifact-panel summaries for bundle hashes, copied artifact counts,
  conformance/finding counts, installed registry hashes, and scenario-ready
  registry/lock/artifact pins.
- Added `repair-yaml --finding bundle-install-package-metadata
  --bundle-install-report <report.json>` so a passing bundle install report can
  add missing scenario-ready registry/lock/artifact pins to matched
  `analog.model_files[]` entries in a copied project. The migration matches by
  package artifact id or installed runtime artifact path and blocks on existing
  conflicting package metadata.

## Risks

- Solver correctness risk: impossible to validate by inspection alone; needs
  conformance fixtures.
- Model fidelity risk: wrong compact models produce convincing wrong answers.
- Licensing/package risk: bundling external kernels or model libraries may
  impose distribution obligations.
- Reproducibility risk: solver versions, tolerances, and model builds change
  results.
- Runtime risk: full simulation can be slow and memory-heavy; CircuitCI needs
  explicit budgets and cancellation.
- User-trust risk: reports must clearly distinguish simulated evidence from
  physical sign-off.

## Decision

CircuitCI should pursue full time-domain and frequency-domain simulation as a
multi-backend orchestration capability, not as a from-scratch solver rewrite.

The durable product value is in:

- turning Board IR and imported design evidence into runnable simulation
  contracts,
- preserving solver/model provenance,
- normalizing results,
- producing stable machine-readable reports,
- connecting waveforms and spectra back to schematic context,
- and fail-closing when evidence or backend capability is insufficient.

That approach gives real design-verification benefit while staying aligned with
CircuitCI's existing role as a board-validation runtime.
