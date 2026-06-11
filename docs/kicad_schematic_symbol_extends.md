# KiCad Library Symbol Extends

## Source Fact

KiCad's S-expression symbol format allows a top-level library symbol to include
`(extends "LIBRARY_ID")`. The saved KiCad reference under
`docs/research/kicad/sexpr-intro.html` states that an extended symbol derives
from another symbol in the same library and that extended symbols currently can
only have different symbol properties than the parent.

## CircuitCI Contract

Native `.kicad_sch` import treats `extends` as pin-geometry inheritance:

- a property-only derived symbol inherits all parent common, unit, and hidden
  power-pin geometry,
- inheritance can chain through multiple symbols,
- schematic instances of the derived symbol use the inherited geometry with the
  same rotation, mirror, unit, no-connect, and hidden-power handling as ordinary
  symbols.

The importer fails closed when inheritance would affect connectivity
ambiguously:

- the parent `LIBRARY_ID` is missing from root `lib_symbols`,
- inheritance forms a cycle,
- the derived symbol declares direct pins,
- the derived symbol declares embedded unit symbols.

CircuitCI does not merge child pins with parent pins. KiCad documents extended
symbols as property-only aliases, so redefining connectivity is rejected instead
of guessed.
