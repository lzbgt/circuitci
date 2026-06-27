# Generic Ideal Op-Amp Buffer

This fixture exercises the generic behavioral analog model pack. It uses a
generated Board IR SPICE deck with `generic.analog.ideal_opamp`, a pulse input,
and a 10 kOhm output load.

The op-amp model is intentionally low-confidence and generic. It is useful for
topology, probe, waveform, and assertion workflow checks, but it is not valid
for vendor-part sign-off, stability, slew-rate, noise, offset, output-current,
or thermal analysis.
