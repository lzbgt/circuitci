# KiCad Multi-Unit Symbol Import

Native `.kicad_sch` import supports a conservative subset of KiCad multi-unit
library symbols.

## Source Facts

The saved KiCad schematic reference states that schematic symbol instances can
carry `(unit UNIT)`, where `UNIT` is the integer ordinal for the represented
symbol unit. The saved KiCad S-expression introduction states that embedded
symbol unit identifiers use `NAME_UNIT_STYLE`, with unit `0` meaning pins common
to all units.

## Import Contract

The importer builds pin geometry per library symbol as:

- direct pins on the top library symbol: common pins,
- embedded child symbol `PARENT_0_STYLE`: common pins,
- embedded child symbol `PARENT_N_STYLE`: unit `N` pins.

For an instance:

- missing `(unit ...)` defaults to unit `1`,
- common pins are always included,
- if the library symbol declares any unit-specific pins, only the selected
  unit's pins are included,
- duplicate pin geometry within the same common/unit scope fails closed,
- unit-specific symbols whose identifier does not match `PARENT_UNIT_STYLE`
  fail closed when they contain pins.

The importer does not infer package-internal cross-unit connectivity. Separate
units with the same reference designator are not merged into one Board IR
component in this slice; that requires a later explicit package/unit model so
physical simulation can map pins to the correct package pins without guessing.

## Rationale

Real KiCad schematics commonly use multi-unit op amps, logic packages, and
connectors. Importing every embedded unit pin for every instance creates false
connectivity and can make a generated SPICE deck simulate pins that are not
present in the placed unit. Selecting only the declared unit keeps Board IR
connectivity faithful enough for downstream SPICE generation and schematic
validation.
