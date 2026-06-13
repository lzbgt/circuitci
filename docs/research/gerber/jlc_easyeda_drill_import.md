# JLC/EasyEDA Excellon Drill Import Notes

## Sources

- Official Ucamco XNC/NC-format specification saved locally:
  `docs/research/gerber/ucamco_xnc_file_format_specification.pdf`
  from <https://www.ucamco.com/the_xnc_file_format_specification.pdf>
- Peer fabricated release:
  `../urine_monitor/docs/fresh_design/artifacts/jlc_eda_releases/DELIVERY_20260428_combined_v01/fabrication/gerber_STM32_ESP32_V01_2026-04-28.zip`
- Observed drill members:
  `Drill_PTH_Through.DRL`, `Drill_NPTH_Through.DRL`,
  `Drill_PTH_Through_Via.DRL`

## Observed Peer Headers

The peer plated-through file starts with:

```text
;TYPE=PLATED
;Layer: PTH_Through
M48
METRIC,LZ,0000.00000
T01C0.30500
```

The peer non-plated-through file starts with:

```text
;TYPE=NON_PLATED
;Layer: NPTH_Through
M48
METRIC,LZ,0000.00000
T01C0.40000
```

Coordinates are absolute metric hits after tool selection, for example
`X29.3Y-8.64001`.

## CircuitCI Import Contract

CircuitCI imports this subset into `board.layout.drills[]` with hole center,
diameter, plating class, source layer, selected tool, and source hit index.
Unsupported NC constructs fail closed instead of being approximated.

Observed peer import counts for the aggregate drill files:

```text
Drill_PTH_Through.DRL: 1275 hits, 8 tools, plated
Drill_NPTH_Through.DRL: 31 hits, 4 tools, non-plated
```

The same archive also contains `Drill_PTH_Through_Via.DRL`, which imports as
`1179` plated via-drill hits with one tool. In this release, those coordinates
overlap the aggregate `Drill_PTH_Through.DRL` file, so the standard peer flow
imports the aggregate PTH file plus the NPTH file rather than appending the
via-only file and double-counting holes.
