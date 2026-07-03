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

Upstream:

- <https://github.com/ngspice/ngspice>
- <https://ngspice.sourceforge.io/docs.html>

Key facts:

- ngspice is based on Spice3f5, Cider, and XSPICE heritage.
- It supports major classic SPICE analyses: noise, operating point, DC sweep,
  pole-zero, distortion, AC, sensitivity, transfer function, transient, and
  experimental PSS.
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

Upstream:

- <https://github.com/Xyce/Xyce>
- <https://xyce.sandia.gov/>

Key facts:

- Xyce is an open-source, SPICE-compatible, high-performance analog circuit
  simulator.
- It supports DC, transient, AC, small-signal noise, harmonic balance,
  sensitivity, uncertainty/random sampling, `.FOUR`, and `.MEASURE`.
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

Upstream:

- <https://github.com/pascalkuthe/OpenVAF>
- <https://openvaf.semimod.de/>

Key facts:

- OpenVAF compiles Verilog-A files for use in circuit simulators.
- It can produce simulator-independent OSDI shared objects and targets compact
  model support.
- The README states that some Verilog-A features remain unsupported, so model
  coverage must be tested and pinned.

CircuitCI implication:

- Model availability is a bigger bottleneck than analysis command support.
  OpenVAF/OSDI support should be part of any serious "full simulation" roadmap.

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

Upstream:

- <https://arxiv.org/abs/2512.10373>

Key point:

- The paper extends QUCS toward time-domain PSS for autonomous oscillator
  circuits and phase-noise simulation. It highlights PSS convergence, basin of
  attraction, frequency guess quality, and stiff circuit dynamics.

CircuitCI implication:

- Oscillator/RF large-signal support is not just "run FFT after transient."
  It needs explicit PSS/HB contracts, convergence evidence, and careful
  limitations in reports.

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
- Extend the initial explicit-Xyce S-parameter path from opt-in real-solver
  conformance coverage into supported two-port test-bench generation.
- Add report limitations for backend-specific gaps.

### Phase 3: Xyce HB Path

- Add Xyce backend execution.
- Add `.HB` Board IR contract.
- Add HB artifact parser and GUI spectrum view.
- Keep HB optional and fail-closed when Xyce is unavailable.

### Phase 4: PSS / Oscillator Evidence

- Add PSS contracts only after HB and result normalization are stable.
- Require convergence metadata, frequency guess, stabilization interval,
  residual/error thresholds, and explicit limitations.

### Phase 5: Model Compiler Pipeline

- Add OpenVAF/OSDI artifact metadata and build/check commands.
- Add model compatibility tests per backend.
- Add artifact hashes and generated-model provenance to reports.

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
