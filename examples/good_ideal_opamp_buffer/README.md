# Generic Ideal Op-Amp Buffer

This fixture exercises the generic behavioral analog model pack and is also a
direct-open GUI scope workflow example. It uses a generated Board IR SPICE deck
with `generic.analog.ideal_opamp`, a pulse input, unity-gain feedback, and a
10 kOhm output load.

Expected workflow:

1. Open `project.yaml` from the GUI `Examples` picker.
2. Use `Run + Scopes` to validate and open waveform inspection.
3. Inspect `v(input)`, `v(output)`, and `v(vcc)` in Scopes or the closeable
   Scope Activity window from Sketch.
4. Select the op-amp in the library/model workflow and create an observation
   preset to generate the same kind of default follower tracking checks from
   the model's `analog_function` metadata.

The op-amp model is intentionally low-confidence and generic. It is useful for
topology, probe, waveform, and assertion workflow checks, but it is not valid
for vendor-part sign-off, stability, slew-rate, noise, offset, output-current,
or thermal analysis.

The direct-open project includes display-only KiCad symbol bindings,
textbook-style orientation metadata, and schematic wire-route waypoints so the
Sketch canvas opens as a readable source-buffer-load network. The electrical
behavior remains defined by Board IR component pin bindings and the generated
SPICE scenario.
