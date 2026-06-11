# KiCad Multi-Unit Package Merge

## Source Facts

KiCad schematic symbols can carry `(unit UNIT)`, where `UNIT` selects one
library-symbol unit. Real schematics often place multiple units of one physical
package as separate symbol instances with the same `Reference`, for example
`U1` unit 1 and `U1` unit 2.

## CircuitCI Contract

Native `.kicad_sch` import merges same-reference symbol instances into one
Board IR component only when the merge is unambiguous:

- all instances have the same library symbol, value, fields, `in_bom`, library,
  and part metadata,
- each instance has a distinct positive unit number,
- all connected package pins resolve to one net per final mapped pin,
- duplicate common pins, such as hidden power pins, are accepted only when they
  resolve to the same net.

Merged source metadata serializes:

- `source.units`: sorted unit numbers for merged packages,
- `source.instances`: all validated KiCad instance path records.

Single-unit components keep the existing scalar `source.unit` metadata.

## Fail-Closed Cases

The importer rejects:

- duplicate same-reference instances with the same unit,
- same-reference instances with different library/value/field metadata,
- same-reference package pins that resolve to different nets,
- mapping that aliases two different imported pins onto one final model pin on
  different nets.

This keeps physical Board IR package identity explicit without inferring
package-internal behavior or hidden connectivity.
