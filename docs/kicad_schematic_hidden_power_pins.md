# KiCad Hidden Power Pin Import

Native `.kicad_sch` import supports a conservative hidden power-pin subset for
library symbols.

## Source Facts

The saved KiCad symbol S-expression reference defines library `pin` entries with
an electrical type, graphical style, position, `name`, and `number`. The
electrical type table includes `power_in`.

Real KiCad schematics commonly omit hidden power pins from each placed symbol
instance. Those pins still matter for Board IR because generated validation and
SPICE mapping need package pins such as `VCC`, `VDD`, `VSS`, or `GND`.

## Import Contract

The importer accepts omitted hidden pins only when all of these are true:

- the library pin has the `hide` token,
- the pin electrical type is `power_in`,
- the pin has a non-empty `name`,
- the pin belongs to the selected symbol unit after multi-unit filtering.

When accepted, the importer adds the hidden pin to the component and labels that
pin's transformed coordinate with the pin name. This uses the same net-label
path as explicit schematic labels, so multiple hidden `VCC` pins resolve to the
same Board IR net.

Hidden pins with any other electrical type fail closed. Hidden power pins
without names also fail closed.

## Non-Goals

This does not infer regulator outputs, package-internal power domains, or model
pin roles. It only preserves a KiCad library symbol's explicit hidden
`power_in` pin as a named net in Board IR. Physical meaning still comes from the
component model and mapping file.
