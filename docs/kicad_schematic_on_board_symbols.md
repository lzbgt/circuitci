# KiCad `on_board` Symbol Handling

Native `.kicad_sch` import now treats KiCad's symbol-level `on_board` metadata
as part of the Board IR contract.

## Source Fact

The saved KiCad schematic reference says the `on_board` token determines
whether the footprint associated with a schematic symbol is exported to the
board through the netlist.

## Import Contract

- Missing `on_board` defaults to `yes`, matching normal KiCad symbols.
- `on_board yes` symbols are imported normally.
- `on_board no` non-power symbols are skipped and do not become Board IR
  components or net nodes.
- KiCad power symbols still contribute labels even though they are not physical
  board components.
- Malformed `on_board` values fail closed.

`in_bom` is intentionally not used for physical connectivity. A symbol can be
excluded from the bill of materials while still being present on the board.

## Rationale

CircuitCI Board IR represents physical board components for validation and SPICE
generation. Importing a symbol that KiCad marks as not exported to the board can
create false physical nodes or model requirements. Skipping non-board symbols is
therefore safer than pretending they are components.
