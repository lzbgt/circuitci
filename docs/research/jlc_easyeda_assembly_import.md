# JLC/EasyEDA Assembly Import Research

## Peer Evidence

The first importer target is the fabricated peer release:

`../urine_monitor/docs/fresh_design/artifacts/jlc_eda_releases/DELIVERY_20260428_combined_v01/`

Relevant assembly files:

- `assembly/bom_STM32_ESP32_V01_2026-04-28.csv`
- `assembly/placement_STM32_ESP32_V01_2026-04-28.csv`

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

Observed on 2026-06-13:

```text
CircuitCI imported JLC/EasyEDA assembly: 450 components, 100 BOM rows, 450 placements, 450 BOM-matched components, 450 placement-matched components
```

The generated assembly-only Board IR also passed baseline schema/binding
validation:

```text
CircuitCI urine_monitor_jlc_assembly: pass (critical=0, warning=0, info=0)
```
