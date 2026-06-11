# KiCad Schematic Parser Modules

Native `.kicad_sch` import now has two parser layers:

- `src/importers/kicad_sch.rs` owns KiCad schematic semantics: supported
  constructs, symbol pin placement, labels, no-connect markers, junctions, and
  conversion into the shared KiCad Board IR builder.
- `src/importers/kicad_sch/sexp.rs` owns generic KiCad S-expression tokenizing,
  string/atom/list accessors, and small list traversal helpers.

The S-expression module must stay syntax-only. It should not know about nets,
symbols, KiCad electrical meaning, Board IR, SPICE, or validation rules. That
boundary keeps future hierarchy and bus support from mixing low-level parsing
with connectivity semantics.
