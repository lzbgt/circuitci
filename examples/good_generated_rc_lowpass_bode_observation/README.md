# Generated RC Low-Pass Bode Observation

This fixture proves the generated-from-board AC/Bode workflow without a hand-authored SPICE deck.

The Board IR describes a `1 kOhm` / `100 nF` first-order RC low-pass filter. The generated SPICE netlist emits source `V1` as `DC 1 AC 1` for AC analysis, so `V(filtered)` is directly interpretable as transfer gain from a unity small-signal input.

Expected cutoff:

```text
fc = 1 / (2*pi*1000*100e-9) ~= 1.59 kHz
```

The observation checks:

- `V(filtered)` is below `-1 dB` at `1 kHz`.
- `V(filtered)` phase is below `-20 deg` at `1 kHz`.
- The falling `-3 dB` crossing stays above `1.4 kHz`.

This is the backend shape created by the GUI AC/Bode run setup controls: generated netlist, AC analysis, Bode CSV artifact, and executable frequency-domain checks.
