# RC Low-Pass Scope Example

This fixture is a direct-open GUI scope workflow example for a first-order RC low-pass filter.

`deck.cir` drives a `1 kOhm` / `100 nF` RC network with a `1 kHz`, `1 V` peak sine source. The cutoff frequency is:

```text
fc = 1 / (2*pi*R*C) = 1 / (2*pi*1000*100e-9) ~= 1.59 kHz
```

Because the input is below cutoff, `v(filtered)` should keep the same dominant
frequency near `1 kHz` while showing lower amplitude and phase lag relative to
`v(input)`.

Expected workflow:

1. Open `project.yaml` from the GUI `Examples` menu.
2. Use `Run + Scopes` to validate and open waveform inspection.
3. Compare `v(input)`, `v(filtered)`, and `i(VSIN)` in Scopes or the Scope
   Activity overlay.

This fixture intentionally keeps the sine source in the file-backed deck while
the Board IR shell provides a connected schematic and named probes for GUI work.
