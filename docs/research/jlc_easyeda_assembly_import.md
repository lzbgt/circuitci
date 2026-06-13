# JLC/EasyEDA Assembly Import Research

## Peer Evidence

The first importer target is the fabricated peer release:

`../urine_monitor/docs/fresh_design/artifacts/jlc_eda_releases/DELIVERY_20260428_combined_v01/`

Relevant assembly files:

- `assembly/bom_STM32_ESP32_V01_2026-04-28.csv`
- `assembly/placement_STM32_ESP32_V01_2026-04-28.csv`

Relevant fabrication file:

- `fabrication/gerber_STM32_ESP32_V01_2026-04-28.zip`, member
  `Gerber_BoardOutlineLayer.GKO`, `Drill_PTH_Through.DRL`, and
  `Drill_NPTH_Through.DRL`

Observed BOM header:

```text
No.,Quantity,Comment,Designator,Footprint,Value,Manufacturer Part,Manufacturer,Supplier Part,Supplier,LCSC Price,JLCPCB Price
```

Observed placement header:

```text
Designator,Device,Footprint,Mid X,Mid Y,Ref X,Ref Y,Pad X,Pad Y,Pins,Layer,Rotation,SMD,Comment,Name
```

The BOM uses quoted comma-separated designator groups such as `"C1,C3,C6"`.
The placement file uses millimeter strings such as `20.7mm`, `Layer` values
such as `T`, degree rotations, and SMD flags such as `Yes`.

## CircuitCI Import Contract

The initial `import-jlc-assembly` slice imports the peer release shape into
Board IR assembly evidence:

- BOM rows are expanded to per-designator component records.
- Quantity must match the expanded designator count.
- Placement coordinates are normalized to millimeters under
  `board.layout.placements`.
- `Layer: T` maps to `side: top`; `B` maps to `side: bottom`.
- Component `source` metadata preserves BOM and placement fields.

No nets or pins are inferred from assembly data. This avoids false electrical
confidence and leaves connectivity to schematic/layout importers.

The companion `import-gerber-outline` command can add board-outline segment
evidence from the release's Gerber outline layer. It imports linear outline
draw records only and does not infer copper, pads, drills, routes, or nets.

The companion `import-excellon-drill` command can append fabrication drill-hit
evidence from the release's PTH and NPTH drill files. It imports hole centers,
diameters, plating class, layer, and selected tool, but still does not infer
pad copper, annular rings, component ownership, or nets.

## Manual Peer Verification

The full peer release can be imported with:

```bash
circuitci import-jlc-assembly \
  --bom ../urine_monitor/docs/fresh_design/artifacts/jlc_eda_releases/DELIVERY_20260428_combined_v01/assembly/bom_STM32_ESP32_V01_2026-04-28.csv \
  --placement ../urine_monitor/docs/fresh_design/artifacts/jlc_eda_releases/DELIVERY_20260428_combined_v01/assembly/placement_STM32_ESP32_V01_2026-04-28.csv \
  --output out/urine-monitor-jlc-assembly.project.yaml \
  --name urine_monitor_jlc_assembly
```

This is an assembly-traceability import, not a board sign-off.

The peer release's Gerber outline member can then be extracted and merged:

```bash
unzip -p ../urine_monitor/docs/fresh_design/artifacts/jlc_eda_releases/DELIVERY_20260428_combined_v01/fabrication/gerber_STM32_ESP32_V01_2026-04-28.zip \
  Gerber_BoardOutlineLayer.GKO > out/urine-monitor-board-outline.gko
circuitci import-gerber-outline out/urine-monitor-board-outline.gko \
  --project out/urine-monitor-jlc-assembly.project.yaml \
  --output out/urine-monitor-jlc-assembly-outline.project.yaml
unzip -p ../urine_monitor/docs/fresh_design/artifacts/jlc_eda_releases/DELIVERY_20260428_combined_v01/fabrication/gerber_STM32_ESP32_V01_2026-04-28.zip \
  Drill_PTH_Through.DRL > out/urine-monitor-pth.drl
unzip -p ../urine_monitor/docs/fresh_design/artifacts/jlc_eda_releases/DELIVERY_20260428_combined_v01/fabrication/gerber_STM32_ESP32_V01_2026-04-28.zip \
  Drill_NPTH_Through.DRL > out/urine-monitor-npth.drl
circuitci import-excellon-drill out/urine-monitor-pth.drl \
  --project out/urine-monitor-jlc-assembly-outline.project.yaml \
  --output out/urine-monitor-jlc-assembly-outline-pth.project.yaml
circuitci import-excellon-drill out/urine-monitor-npth.drl \
  --project out/urine-monitor-jlc-assembly-outline-pth.project.yaml \
  --output out/urine-monitor-jlc-assembly-outline-drills.project.yaml
```

Observed on 2026-06-13:

```text
CircuitCI imported JLC/EasyEDA assembly: 450 components, 100 BOM rows, 450 placements, 450 BOM-matched components, 450 placement-matched components
```

The generated assembly-only Board IR also passed baseline schema/binding
validation:

```text
CircuitCI urine_monitor_jlc_assembly: pass (critical=0, warning=0, info=0)
```

Observed Gerber outline enrichment:

```text
CircuitCI imported Gerber outline: 16 segments (4 external, 12 cutout, 0 unknown)
```

Observed PTH/NPTH drill enrichment:

```text
CircuitCI imported Excellon/NC drill evidence: 1275 hits, 8 tools (1275 plated, 0 non-plated, 0 unknown plating)
CircuitCI imported Excellon/NC drill evidence: 31 hits, 4 tools (0 plated, 31 non-plated, 0 unknown plating)
```
