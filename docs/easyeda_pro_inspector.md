# EasyEDA Pro Inspector

`circuitci inspect-easyeda-pro` reads an EasyEDA Pro `.eprj2` SQLite project
file and writes a Markdown evidence report plus a schema-validated JSON
inspection manifest. By default, the manifest is written beside the Markdown
report using the same path with a `.json` extension; pass `--manifest` to choose
an explicit path.

The command currently extracts only the plaintext SQLite envelope:

- project rows,
- branch rows,
- the latest `project_structures.structure` JSON object,
- board, schematic, sheet, and PCB identifiers from that structure,
- history payload counts and whether those payloads look like plaintext JSON.

Example:

```bash
circuitci inspect-easyeda-pro source/project.eprj2 \
  --output out/easyeda_pro_inspection.md
```

For automation:

```bash
circuitci inspect-easyeda-pro source/project.eprj2 \
  --output out/easyeda_pro_inspection.md \
  --manifest out/easyeda_pro_inspection_manifest.json
```

The JSON manifest conforms to
`schemas/easyeda_pro_inspection.schema.json` and records:

- the source `.eprj2` size and SHA-256,
- SQLite table names, column metadata, and row counts,
- project and branch rows,
- the latest plaintext project-structure object with length and SHA-256,
- `history_data` payload ids, byte lengths, SHA-256 values, and JSON-prefix
  classification,
- an importability status explaining whether geometry conversion is blocked by
  encoded/non-JSON history payloads.

The observed `urine_monitor` EasyEDA Pro release stores project structure
metadata as plaintext JSON, but design-object history payloads are encoded or
application-protected non-JSON strings. CircuitCI therefore does not infer
pad, via, route, zone, or net geometry from those payloads.

Use this command as an evidence-quality gate before fabricated-release
enrichment:

- If history payloads are encoded, import BOM/CPL, Gerber, and Excellon
  evidence normally, but expect owner-associated fabricated artwork counts to
  remain zero until an unencoded layout export is available.
- If a future EasyEDA Pro export exposes plaintext layout objects, add a
  focused adapter for that exported shape rather than guessing from opaque
  history blobs.

The inspector requires the `sqlite3` command-line tool at runtime. It fails
closed for non-SQLite files and for SQLite files missing the expected EasyEDA
Pro envelope tables.
