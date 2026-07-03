# Xyce, OpenVAF, and OSDI Compatibility Boundary

Date: 2026-07-04

## Question

Can CircuitCI reuse the same OpenVAF-produced `*.osdi` model artifacts for
`backend: xyce`, or must OpenVAF/OSDI remain an external-ngspice-only model
loading path?

## Sources Cached

Primary source artifacts downloaded into this repository:

- `sources/xyce_adms_users_guide.html`
  - URL: <https://xyce.sandia.gov/documentation-tutorials/xyce-adms-users-guide/>
- `sources/xyce_tutorial_adding_device.html`
  - URL: <https://xyce.sandia.gov/documentation-tutorials/tutorial-adding-a-device-to-xyce/>
- `sources/openvaf_usage.html`
  - URL: <https://openvaf.semimod.de/docs/getting-started/usage/>
- `sources/openvaf_osdi_details.html`
  - URL: <https://openvaf.semimod.de/docs/details/osdi/>
- `sources/osdi_v0p3.pdf`
  - URL: <https://openvaf.semimod.de/osdi/osdi_v0p3.pdf>
- `sources/ngspice_osdi.html`
  - URL: <https://ngspice.sourceforge.io/osdi.html>

Previously cached sources also used:

- `sources/openvaf_README.md`
- `sources/openvaf_home.html`
- `sources/ngspice_manual.xhtml`
- `sources/Xyce_Reference_Guide_7.8.txt`
- `sources/Xyce_Users_Guide_7.8.txt`

## Evidence

### OpenVAF Output Contract

OpenVAF usage documentation says running `openvaf <file>.va` generates a
`<file>.osdi` library that can be used by circuit simulators implementing the
OSDI interface, and specifically describes ngspice loading with `osdi` or
`pre_osdi` commands. The OpenVAF OSDI page describes OSDI as a simulator
independent interface for runtime shared-object loading and says an internal
spice API to OSDI bridge has been added to ngspice.

The OpenVAF repository README similarly states that OpenVAF can compile
Verilog-A files to shared objects complying with the simulator-independent OSDI
interface, and that OpenVAF had been tested with an ngspice prototype and
Melange. It does not claim Xyce support.

### ngspice OSDI Contract

The ngspice OSDI page states that ngspice uses OSDI/OpenVAF for including
Verilog-A compact device models, that ngspice contains the OSDI interface since
version 39, and that compiled Verilog-A compact device models can be loaded at
runtime. Its getting-started list explicitly includes compiling a model with
OpenVAF to obtain `*.osdi`, loading `*.osdi` into ngspice, and using `N` device
lines for the loaded models.

The ngspice manual cache adds the operational details: OSDI models are loaded
with `osdi` or `pre_osdi` control commands, `pre_osdi` should be added at the
beginning of `.control`, `osdi_enabled` reflects whether OSDI was compiled in,
and `--enable-osdi` enables the interface.

### Xyce Verilog-A Contract

The Xyce/ADMS users guide describes a different flow. Xyce/ADMS is a set of
ADMS XML templates that emit C++ code for a Xyce device model. The guide states
that Xyce does not have direct Verilog-A import through a netlist; Verilog-A
must first be converted to C++ using Xyce/ADMS, compiled, and linked into
Xyce.

The same guide gives two Xyce integration paths:

- direct linking into a Xyce build, and
- a shared-library plugin built by `buildxyceplugin`.

The guide says the plugin method requires a specially built shareable Xyce
(`--enable-shared` and `--enable-xyce-shareable`) and that standard binary
distributions were not built with the feature at the time the page was written.
Plugins are loaded with the Xyce command-line `-plugin` option and are Xyce
plugin shared libraries, not OSDI shared objects.

The older "Adding a device to Xyce" tutorial is consistent: it describes
running ADMS with Xyce templates to create `.C`/`.h` files, adding them to the
Xyce build, rebuilding Xyce, or building a Xyce shared-library plugin. It also
states that plugins are not automatically detected and require `-plugin`.

### Negative Search

Local searches for `OSDI`, `OpenVAF`, `pre_osdi`, and `.osdi` in the cached
Xyce ADMS guide, Xyce device tutorial, Xyce 7.8 reference guide, Xyce 7.8 users
guide, Xyce README, and Xyce INSTALL notes returned no Xyce-side OSDI or
OpenVAF loading contract. This is absence evidence, not a proof that no
experimental branch exists, but it is enough to reject a production adapter
without a primary Xyce document and runnable conformance artifact.

## Decision

OpenVAF-produced `*.osdi` artifacts are currently supported only for external
ngspice in CircuitCI.

CircuitCI must not pass `*.osdi` artifacts through to Xyce. The trustworthy
Xyce path is a separate Xyce/ADMS contract: Verilog-A source is translated to
C++ with Xyce templates, then linked into Xyce or built as a Xyce plugin for a
shareable Xyce build and loaded with `-plugin`.

## Implementation Boundary

- `backend: ngspice` may load OpenVAF/OSDI artifacts with generated
  `pre_osdi` commands and conformance coverage.
- `backend: auto` remains acceptable because current analog auto-selection
  chooses external ngspice before any other backend for this model path.
- Explicit `backend: xyce` with `artifact_format: osdi_shared_object` must fail
  closed with `ANALOG_MODEL_COMPILER_BACKEND_UNSUPPORTED`.
- Explicit `backend: embedded_ngspice` must also fail closed until a mature
  linked ngspice runtime exposes and proves OSDI loading.

## Future Xyce Work

CircuitCI now reserves a separate Xyce compact-model artifact format,
`artifact_format: xyce_adms_plugin`. It does not reuse
`artifact_format: osdi_shared_object`.

The contract is currently a fail-closed provenance and conformance boundary,
not an execution adapter. A valid entry must declare:

- Verilog-A source path/hash.
- `compiler: xyce_adms`, compiler version, and a reproducible
  `buildxyceplugin` command that references the declared source and plugin
  output.
- Xyce version and Xyce/ADMS template revision.
- Xyce source/build identity and configure options, including
  `--enable-shared` and `--enable-xyce-shareable`.
- Produced plugin path/hash.
- Xyce command-line `-plugin` load command.
- Retained real-Xyce conformance artifact path/hash proving the plugin was
  loaded and the generated model was active.

Even when all of those fields are valid, CircuitCI emits
`ANALOG_MODEL_COMPILER_XYCE_PLUGIN_UNSUPPORTED` until a real-Xyce plugin loader
and solver manifest contract are implemented. The opt-in qualification command
`CIRCUITCI_RUN_REAL_XYCE_ADMS_PLUGIN=1 cargo test --test analog_model_compiler_cli`
now exercises the primary-source build/load path when the host has
`buildxyceplugin` and `Xyce`/`xyce` on `PATH`: it builds the Xyce tutorial RLC
Verilog-A plugin, runs `Xyce -plugin`, retains the conformance log, pins its
hash, and verifies CircuitCI's fail-closed contract against those real
artifacts. This keeps Xyce model extension as a backend-qualification target
while preserving the metadata needed for that future adapter.
