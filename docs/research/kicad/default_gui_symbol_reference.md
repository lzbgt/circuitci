# KiCad Device Symbols Used By The GUI Sketch Defaults

## Source

Local KiCad install inspected on this host:

- `/Applications/KiCad/KiCad.app/Contents/SharedSupport/symbols/Device.kicad_sym`
- `/Applications/KiCad/KiCad.app/Contents/SharedSupport/symbols/Simulation_SPICE.kicad_sym`

The GUI Sketch defaults should treat these libraries as the canonical reference
for built-in schematic symbols. Runtime rendering keeps a checked-in geometry
fallback so the GUI remains deterministic on machines without KiCad installed.

## Symbols Checked

- `Device:R`: IEC rectangular resistor body with two passive pins.
- `Device:C`: two parallel capacitor plates with two passive pins.
- `Device:L`: four-arc inductor coil with two passive pins.
- `Device:D`: diode triangle/bar geometry with two passive pins.
- `Device:Voltmeter_AC`, `Device:Voltmeter_DC`, `Device:Ammeter_AC`,
  `Device:Ammeter_DC`, and `Device:Oscilloscope`: measurement/probe-class
  library symbols suitable as the visual reference for GUI probe elements.

## GUI Contract

- Common two-terminal primitives render as compact KiCad-style schematic
  symbols, not large component cards.
- Their pins attach to opposite symbol terminals, with rotation/mirroring
  applied to the terminal positions.
- Nets render as lightweight schematic labels/junction targets, not green
  boxes.
- Probe placement is exposed as Probe Elements in the Sketch side dock and maps
  to Board IR analog probes plus runtime Scopes waveform binding.
