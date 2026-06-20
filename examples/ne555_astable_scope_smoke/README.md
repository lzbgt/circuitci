# NE555 Astable Scope Smoke

This fixture is a GUI scope workflow smoke test for a typical NE555 astable application.

`deck.cir` uses an idealized `PULSE` source for the timer output rather than a vendor NE555 macro-model. The circuit is intended to verify schematic import, Run, scope waveform loading, voltage/current probes, and frequency-domain inspection with a familiar astable output/timing-node shape.

Expected workflow:

1. In the GUI Import stage, click `Use NE555 Astable` to fill the SPICE import
   fields, or `Import NE555` to start the import directly.
2. Open the generated project in Sketch or Scopes.
3. Use `Run + Scopes` or Scopes `Run`; Auto-before-Run should keep useful V/I
   probes available.
4. Inspect `v(out)`, `v(timing)`, and supply/load current traces, including the
   Scope Activity overlay on the schematic.

The same deck can still be imported headlessly with `circuitci import-spice`.

The output pulse period is `686 us`, so the dominant frequency should be near `1.46 kHz`.
