# EasyEDA Pro Inspector

`circuitci inspect-easyeda-pro` reads an EasyEDA Pro `.eprj2` SQLite project
file and writes a Markdown evidence report.

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
