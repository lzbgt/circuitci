# NE555 Astable Scope Smoke

This fixture is a GUI scope workflow smoke test for a typical NE555 astable application.

`deck.cir` uses an idealized `PULSE` source for the timer output rather than a vendor NE555 macro-model. The circuit is intended to verify schematic import, Run, scope waveform loading, voltage/current probes, and frequency-domain inspection with a familiar astable output/timing-node shape.

`project.yaml` is the portable, direct-open Board IR version of the same deck.
It keeps `deck.cir` as the solver source, uses relative paths, and names the
schematic nets `out`, `timing`, `vcc`, and `gnd` for easier GUI inspection.
It also includes display-only KiCad symbol bindings, textbook orientation
metadata, and schematic wire-route waypoints so the Sketch view opens as a
connected, readable circuit; electrical connectivity still comes from the
component pin-to-net bindings and the file-backed SPICE deck.

Expected workflow:

1. Open `project.yaml` directly in the GUI, or import `deck.cir` from the
   Import stage with `Use NE555 Astable` / `Import NE555`.
2. Use `Run + Scopes` or Scopes `Run`; Auto-before-Run should keep useful V/I
   probes available.
3. Inspect `v(out)`, `v(timing)`, and supply/load current traces in Scopes or
   the closeable Scope Activity window from Sketch.

The same deck can still be imported headlessly with `circuitci import-spice`.

The output pulse period is `686 us`, so the dominant frequency should be near `1.46 kHz`.
