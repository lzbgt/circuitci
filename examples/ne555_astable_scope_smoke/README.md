# NE555 Astable Scope Smoke

This fixture is a GUI scope workflow smoke test for a typical NE555 astable application.

`deck.cir` uses an idealized `PULSE` source for the timer output rather than a vendor NE555 macro-model. The circuit is intended to verify schematic import, Run, scope waveform loading, voltage/current probes, and frequency-domain inspection with a familiar astable output/timing-node shape.

Expected workflow:

1. Import `deck.cir` with `circuitci import-spice`.
2. Open the generated project in the GUI.
3. Run validation from Scopes.
4. Inspect `v(out)`, `v(timing)`, and supply/load current traces.

The output pulse period is `686 us`, so the dominant frequency should be near `1.46 kHz`.
