# Loop Stability Bode Scope Example

This fixture is a direct-open GUI example for AC loop-stability observation.
It uses a deterministic file-backed SPICE deck rather than a vendor op-amp
macro-model, so the example focuses on Bode plotting and executable stability
margin checks.

`deck_ac.cir` implements this open-loop transfer shape:

```text
L(s) = 100 / ((1 + s/w1) * (1 + s/w2) * (1 + s/w3))
w1 = 2*pi*10 Hz
w2 = 2*pi*100 kHz
w3 = 2*pi*1 MHz
```

The dominant pole keeps the falling 0 dB crossing near `1 kHz` with about
`90 deg` of phase margin. The two higher poles make the phase cross
`-180 deg`, where the loop magnitude is well below unity and the gain margin
is comfortably above `6 dB`.

Expected workflow:

1. Open `project.yaml` from the GUI `Examples` picker.
2. Use `Run + Scopes` to validate and open Bode inspection.
3. Inspect `loop_mag_db`, `loop_phase_deg`, and `loop_mag` from `bode.csv`.
4. Confirm the executable checks pass:
   - phase margin above `45 deg`
   - gain margin above `6 dB`

The direct-open project includes display-only KiCad symbol bindings,
textbook-style orientation metadata, and schematic wire-route waypoints so the
Sketch canvas opens as a readable AC source, gain block, dominant pole, and
two high-frequency pole network. The electrical behavior remains defined by
the file-backed SPICE deck and the Board IR `analog_ac` observation.
