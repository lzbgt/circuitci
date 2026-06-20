# KiCad Device Symbols Used By The GUI Sketch Defaults

## Source

Local KiCad install inspected on this host:

- `/Applications/KiCad/KiCad.app/Contents/SharedSupport/symbols/Device.kicad_sym`
- `/Applications/KiCad/KiCad.app/Contents/SharedSupport/symbols/Simulation_SPICE.kicad_sym`

The GUI Sketch defaults treat installed KiCad symbol libraries as the canonical
source for built-in schematic symbols. Runtime rendering discovers installed
KiCad symbol directories, catalogs every top-level symbol from `.kicad_sym`
files for the Library panel, parses selected drawing primitives on demand,
caches them, and falls back to checked-in geometry only when KiCad is
unavailable or a symbol cannot be parsed. User-provided `.kicad_sym` files can
also be imported into the GUI library; imported symbols share the same catalog,
pin extraction, insertion, and drawing cache path.

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
- Installed KiCad library drawings are preferred for explicit
  `board.schematic.component_symbols` glyphs and for default R/C/L/D/source
  glyphs; built-in geometry is a compatibility fallback, not the primary
  source.
- Their pins attach to opposite symbol terminals, with rotation/mirroring
  applied to the terminal positions.
- Nets render as lightweight schematic labels/junction targets, not green
  boxes.
- Probe placement is exposed as Probe Elements in the Sketch side dock and maps
  to Board IR analog probes plus runtime Scopes waveform binding.
