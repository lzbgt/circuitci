# Comparator Threshold Scope Example

This fixture is a direct-open GUI scope workflow example for threshold
comparison.

It uses `generic.analog.ideal_comparator` from the generic behavioral SPICE
model pack. A pulse source drives the non-inverting input from `0.5 V` to
`2.5 V`; a DC source holds the inverting input at `1.2 V`. The output should
stay low before the input crosses the reference and then switch high toward the
`5 V` supply.

Expected workflow:

1. Open `project.yaml` from the GUI `Examples` picker.
2. Use `Run + Scopes` to validate and open waveform inspection.
3. Inspect `v(input)`, `v(reference)`, `v(output)`, and `v(vcc)` in Scopes or
   the closeable Scope Activity window from Sketch.
4. Select the comparator in the library/model workflow and create an
   observation preset to generate the same kind of default output-state checks
   from the model's `analog_function` metadata.

This is a reduced-fidelity behavioral example for topology and GUI workflow
checks. It does not model real comparator propagation delay, hysteresis,
open-drain pull-up behavior, input offset/noise, common-mode limits, or output
drive strength.

The direct-open project includes display-only KiCad symbol bindings,
textbook-style orientation metadata, and schematic wire-route waypoints so the
Sketch canvas opens as a readable source-reference-comparator network. The
electrical behavior remains defined by Board IR component pin bindings and the
generated SPICE scenario.
