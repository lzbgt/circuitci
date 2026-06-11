# Native KiCad Schematic Import

## Purpose

The existing KiCad importer consumes KiCad generic XML netlists. Agents often
have the native `.kicad_sch` source file before they have an exported netlist,
so CircuitCI needs a conservative native schematic ingestion path.

## Source Format

KiCad schematic files use an S-expression format and the `.kicad_sch`
extension. The developer documentation saved under `docs/research/kicad/`
describes the root `kicad_sch` token, schematic sections, symbols, wires, and
labels.

Saved references:

- `docs/research/kicad/sexpr-intro.html`
- `docs/research/kicad/sexpr-schematic.html`

## First Supported Subset

The first native importer slice supports only enough schematic semantics to
produce the same internal `ParsedKicadNetlist` used by the XML importer:

- root `.kicad_sch` files with recursive sheet hierarchy,
- root `symbol` instances with `lib_id`, `Reference`, and `Value`
  properties,
- root `lib_symbols` pin definitions for symbol pin coordinates,
- cardinal symbol rotations at `0`, `90`, `180`, or `270` degrees, including
  equivalent wrapped values,
- `mirror x` and `mirror y` symbol transforms,
- straight horizontal or vertical wires,
- horizontal/vertical bus graphics and bus entries when scalar wire labels are
  explicit,
- explicit junctions for mid-span wire crossings,
- local and global labels as net names,
- optional KiCad power symbols treated as one-pin labeled symbols.
- explicit schematic `no_connect` markers for intentionally open symbol pins.

Unsupported constructs fail closed:

- root-level hierarchical labels,
- hierarchy cycles,
- bus aliases and inferred bus expansion,
- malformed, non-finite, or non-cardinal symbol rotations,
- malformed or unsupported symbol mirror tokens,
- missing library pin geometry,
- malformed labels,
- duplicate or conflicting labels at one coordinate,
- conflicting labels on one connected net group,
- malformed, duplicate, floating, or redundant junctions,
- mid-span wire crossings without explicit junctions,
- malformed `no_connect` markers,
- unconnected symbol pins without `no_connect` evidence,
- `no_connect` markers that float, match multiple pins, or overlap live
  connectivity,
- importable components with no connected pins,
- treating library pin electrical type `no_connect` as schematic evidence,
- unlabeled multi-net ambiguity that would require guessing intended names.

## Safety Contract

The native parser performs connectivity extraction only. It does not infer
component physics, model selection, or simulation scenarios. After parsing, the
result flows through the existing KiCad mapping and scenario code path,
preserving SHA-pinned model checks, `SCHEMATIC_IMPORT_ONLY`, strict opt-in
passive value parsing, and all generated-SPICE fail-closed behavior.

## Fixtures

- `examples/import_kicad_schematic/basic_rc.kicad_sch` covers native schematic
  connectivity import and a mapped RC/diode generated-SPICE scenario.
- `examples/import_kicad_schematic/mosfet_soa.kicad_sch` covers native
  schematic import of a mapped FDMC86184 MOSFET scenario with datasheet-backed
  SOA evidence.
- `examples/import_kicad_schematic/rotated_rc.kicad_sch` covers cardinal
  symbol rotation and generated-SPICE validation from transformed pin
  coordinates.
