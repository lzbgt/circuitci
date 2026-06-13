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
unzip -p ../urine_monitor/docs/fresh_design/artifacts/jlc_eda_releases/DELIVERY_20260428_combined_v01/fabrication/gerber_STM32_ESP32_V01_2026-04-28.zip \
  FlyingProbeTesting.json > out/urine-monitor-flying-probe.json
circuitci import-easyeda-flying-probe out/urine-monitor-flying-probe.json \
  --project out/urine-monitor-jlc-assembly.project.yaml \
  --output out/urine-monitor-jlc-assembly-probe.project.yaml
```

This is an assembly-traceability import, not a board sign-off.

The peer release's Gerber outline member can then be extracted and merged:

```bash
unzip -p ../urine_monitor/docs/fresh_design/artifacts/jlc_eda_releases/DELIVERY_20260428_combined_v01/fabrication/gerber_STM32_ESP32_V01_2026-04-28.zip \
  Gerber_BoardOutlineLayer.GKO > out/urine-monitor-board-outline.gko
circuitci import-gerber-outline out/urine-monitor-board-outline.gko \
  --project out/urine-monitor-jlc-assembly-probe.project.yaml \
  --output out/urine-monitor-jlc-assembly-outline.project.yaml
unzip -p ../urine_monitor/docs/fresh_design/artifacts/jlc_eda_releases/DELIVERY_20260428_combined_v01/fabrication/gerber_STM32_ESP32_V01_2026-04-28.zip \
  Gerber_TopLayer.GTL > out/urine-monitor-top-copper.gtl
unzip -p ../urine_monitor/docs/fresh_design/artifacts/jlc_eda_releases/DELIVERY_20260428_combined_v01/fabrication/gerber_STM32_ESP32_V01_2026-04-28.zip \
  Gerber_BottomLayer.GBL > out/urine-monitor-bottom-copper.gbl
unzip -p ../urine_monitor/docs/fresh_design/artifacts/jlc_eda_releases/DELIVERY_20260428_combined_v01/fabrication/gerber_STM32_ESP32_V01_2026-04-28.zip \
  Gerber_TopSolderMaskLayer.GTS > out/urine-monitor-top-mask.gts
unzip -p ../urine_monitor/docs/fresh_design/artifacts/jlc_eda_releases/DELIVERY_20260428_combined_v01/fabrication/gerber_STM32_ESP32_V01_2026-04-28.zip \
  Gerber_BottomSolderMaskLayer.GBS > out/urine-monitor-bottom-mask.gbs
unzip -p ../urine_monitor/docs/fresh_design/artifacts/jlc_eda_releases/DELIVERY_20260428_combined_v01/fabrication/gerber_STM32_ESP32_V01_2026-04-28.zip \
  Gerber_TopPasteMaskLayer.GTP > out/urine-monitor-top-paste.gtp
unzip -p ../urine_monitor/docs/fresh_design/artifacts/jlc_eda_releases/DELIVERY_20260428_combined_v01/fabrication/gerber_STM32_ESP32_V01_2026-04-28.zip \
  Drill_PTH_Through.DRL > out/urine-monitor-pth.drl
unzip -p ../urine_monitor/docs/fresh_design/artifacts/jlc_eda_releases/DELIVERY_20260428_combined_v01/fabrication/gerber_STM32_ESP32_V01_2026-04-28.zip \
  Drill_PTH_Through_Via.DRL > out/urine-monitor-pth-via.drl
unzip -p ../urine_monitor/docs/fresh_design/artifacts/jlc_eda_releases/DELIVERY_20260428_combined_v01/fabrication/gerber_STM32_ESP32_V01_2026-04-28.zip \
  Drill_NPTH_Through.DRL > out/urine-monitor-npth.drl
circuitci import-gerber-copper out/urine-monitor-top-copper.gtl \
  --project out/urine-monitor-jlc-assembly-outline.project.yaml \
  --output out/urine-monitor-jlc-assembly-top-copper.project.yaml
circuitci import-gerber-copper out/urine-monitor-bottom-copper.gbl \
  --project out/urine-monitor-jlc-assembly-top-copper.project.yaml \
  --output out/urine-monitor-jlc-assembly-copper.project.yaml
circuitci import-gerber-solder-mask out/urine-monitor-top-mask.gts \
  --project out/urine-monitor-jlc-assembly-copper.project.yaml \
  --output out/urine-monitor-jlc-assembly-top-mask.project.yaml
circuitci import-gerber-solder-mask out/urine-monitor-bottom-mask.gbs \
  --project out/urine-monitor-jlc-assembly-top-mask.project.yaml \
  --output out/urine-monitor-jlc-assembly-mask.project.yaml
circuitci import-gerber-solder-paste out/urine-monitor-top-paste.gtp \
  --project out/urine-monitor-jlc-assembly-mask.project.yaml \
  --output out/urine-monitor-jlc-assembly-paste.project.yaml
circuitci import-excellon-drill out/urine-monitor-pth.drl \
  --project out/urine-monitor-jlc-assembly-paste.project.yaml \
  --output out/urine-monitor-jlc-assembly-outline-pth.project.yaml
circuitci import-excellon-drill out/urine-monitor-pth-via.drl \
  --project out/urine-monitor-jlc-assembly-outline-pth.project.yaml \
  --output out/urine-monitor-jlc-assembly-outline-pth-via.project.yaml
circuitci import-excellon-drill out/urine-monitor-npth.drl \
  --project out/urine-monitor-jlc-assembly-outline-pth-via.project.yaml \
  --output out/urine-monitor-jlc-assembly-outline-drills.project.yaml
```

Observed on 2026-06-13:

```text
CircuitCI imported JLC/EasyEDA assembly: 450 components, 100 BOM rows, 450 placements, 450 BOM-matched components, 450 placement-matched components
CircuitCI imported EasyEDA/JLC flying-probe pads: 3168 pin rows, 2985 connected pin rows, 2985 pads imported, 183 duplicate pin rows, 17 multipart pin rows, 0 unconnected pins skipped, 1432 components created, 440 nets imported
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

Observed Gerber copper/mask/paste enrichment after adding observed EasyEDA
`RoundRect` aperture and `G02`/`G03` circular-arc draw support:

```text
CircuitCI imported Gerber copper: 2725 flash features, 2567 trace segments, 22 regions, 0 net-associated features, 0 net-associated segments, 0 net-associated regions, 0 island-associated features, 0 island-associated segments, 0 island-associated regions, 120 apertures, 0 ignored draw records, 0 skipped clear flashes, 0 skipped clear regions
CircuitCI imported Gerber copper: 1275 flash features, 854 trace segments, 3 regions, 0 net-associated features, 0 net-associated segments, 0 net-associated regions, 0 island-associated features, 0 island-associated segments, 0 island-associated regions, 30 apertures, 0 ignored draw records, 0 skipped clear flashes, 0 skipped clear regions
CircuitCI imported Gerber solder mask: 1546 flash openings, 0 draw openings, 7 region openings, 107 apertures, 0 ignored draw records, 0 skipped clear flashes, 0 skipped clear regions
CircuitCI imported Gerber solder mask: 96 flash openings, 0 draw openings, 0 region openings, 19 apertures, 0 ignored draw records, 0 skipped clear flashes, 0 skipped clear regions
CircuitCI imported Gerber solder paste: 1111 flash openings, 0 draw openings, 354 region openings, 75 apertures, 0 ignored draw records, 0 skipped clear flashes, 0 skipped clear regions
```

After importing `FlyingProbeTesting.json` first and adding EasyEDA Gerber layer
aliases (`TopLayer`, `TopSolderMaskLayer`, and `TopPasteMaskLayer`), observed
owner association becomes:

```text
Top copper: 2018 net-associated features, 1255 net-associated segments, 22 net-associated regions
Bottom copper: 96 net-associated features and 33 net-associated segments after applying JLC placement side evidence
Top solder mask: 121 owner-associated flash openings
Bottom solder mask: 9 owner-associated flash openings after applying JLC placement side evidence
Top solder paste: 103 owner-associated flash openings, 9 owner-associated region openings
PTH drills: 9 pad-associated hits
```

Observed PTH/NPTH drill enrichment:

```text
CircuitCI imported Excellon/NC drill evidence: 1267 hits, 8 routed slots, 8 tools (1267 plated, 0 non-plated, 0 unknown plating, 9 pad-associated, 0 via-associated)
CircuitCI imported Excellon/NC drill evidence: 1179 hits, 0 routed slots, 1 tools (1179 plated, 0 non-plated, 0 unknown plating, 0 pad-associated, 472 via-associated)
CircuitCI imported Excellon/NC drill evidence: 31 hits, 0 routed slots, 4 tools (0 plated, 31 non-plated, 0 unknown plating, 0 pad-associated, 0 via-associated)
```

Observed static mask/paste manufacturability checks on the same imported peer
release:

```text
min_solder_mask_dam_mm: 0.05 and min_solder_paste_spacing_mm: 0.05
CircuitCI urine_monitor_jlc_assembly: pass (critical=0, warning=0, info=0)

min_solder_mask_dam_mm: 0.10 and min_solder_paste_spacing_mm: 0.10
CircuitCI urine_monitor_jlc_assembly: fail (critical=148, warning=0, info=0)
```

The `0.10 mm` screen produced only `SOLDER_MASK_DAM_VALID` findings; paste
spacing had no findings at that threshold. The reported solder-mask dam range
was approximately `0.093982..0.099850 mm`, all on `TopSolderMaskLayer`.

## Peer Manufacturing Suggestion Checklist

Observed on 2026-06-13 after importing the full peer release through assembly,
flying-probe pads, outline, top/bottom copper, top/bottom solder mask, top
paste, aggregate PTH drills, and NPTH drills:

```text
CircuitCI suggested 13 scenarios for urine_monitor_jlc_assembly -> out/peer-manufacturing-suggestions-current
CircuitCI urine_monitor_jlc_assembly: pass (critical=0, warning=0, info=0)
```

Current split: 9 runnable source-backed suggestions and 4 non-runnable
threshold-gated suggestions.

Runnable manufacturing suggestions generated from named source-backed presets:

| Suggestion | Check | Preset |
| --- | --- | --- |
| `drill_diameter_valid` | `DRILL_DIAMETER_VALID` | `jlcpcb_drill_diameter_range_2026_06` |
| `slot_width_valid` | `SLOT_WIDTH_VALID` | `jlcpcb_slot_min_2026_06` |
| `drill_annular_ring_valid` | `DRILL_ANNULAR_RING_VALID` | `jlcpcb_double_sided_via_min_2026_06` |
| `copper_to_board_edge_clearance` | `COPPER_TO_BOARD_EDGE_CLEARANCE_VALID` | `jlcpcb_routed_edge_copper_clearance_2026_06` |
| `copper_spacing_valid` | `COPPER_SPACING_VALID` | `jlcpcb_1oz_copper_spacing_2026_06` |
| `solder_mask_opening_valid` | `SOLDER_MASK_OPENING_VALID` | `jlcpcb_standard_2026_06` |
| `solder_mask_dam_valid` | `SOLDER_MASK_DAM_VALID` | `jlcpcb_standard_2026_06` |
| `solder_paste_aperture_size_valid` | `SOLDER_PASTE_APERTURE_SIZE_VALID` | `jlcpcb_stencil_aperture_min_2026_06` |

Runnable manufacturing suggestions generated from source-backed package evidence:

| Suggestion | Check | Inferred evidence |
| --- | --- | --- |
| `solder_paste_ic_pin_aperture_valid` | `SOLDER_PASTE_IC_PIN_APERTURE_VALID` | repeated pad-owned `0.5 mm` paste pitch on target `U19` |

Non-runnable suggestions generated because the imported evidence proves the
geometry exists but the process threshold is not yet pinned to an authoritative
named preset:

| Suggestion | Check | Required source-pinned threshold |
| --- | --- | --- |
| `drill_to_board_edge_clearance` | `DRILL_TO_BOARD_EDGE_CLEARANCE_VALID` | `min_drill_edge_clearance_mm` |
| `slot_to_board_edge_clearance` | `SLOT_TO_BOARD_EDGE_CLEARANCE_VALID` | `min_slot_edge_clearance_mm` |
| `solder_paste_opening_valid` | `SOLDER_PASTE_OPENING_VALID` | `min_paste_area_ratio`, `max_paste_area_ratio` |
| `solder_paste_spacing_valid` | `SOLDER_PASTE_SPACING_VALID` | `min_solder_paste_spacing_mm` |

This confirms the fabricated-release ingestion is now strong enough to produce
a concrete manufacturing checklist automatically. The remaining gap is process
evidence, not detection plumbing: those four non-runnable checks need exact,
condition-scoped JLCPCB or package/stencil source values before CircuitCI should
turn them into preset-backed runnable scenarios.

The JLCPCB stencil capability source is pinned for one generic stencil
manufacturability floor: minimum aperture size `>0.08mm`. CircuitCI therefore
emits runnable `SOLDER_PASTE_APERTURE_SIZE_VALID` with
`jlcpcb_stencil_aperture_min_2026_06`. The separate JLCPCB stencil opening
standard is package- and pitch-specific, so paste area-ratio and paste-spacing
suggestions still require explicit package/process limits.

CircuitCI now also encodes the source-backed JLCPCB BGA stencil aperture table
as `SOLDER_PASTE_BGA_APERTURE_VALID`. It remains package-scoped rather than a
generic paste preset. The real `urine_monitor` imported release still generated
13 suggestions after this addition, with no BGA stencil suggestion, because the
current owner-backed solder-paste evidence did not prove a repeated two-axis
BGA grid for one component.

Follow-up source review on 2026-06-13 found JLCPCB castellated-hole edge
material, but not a generic drill-to-board-edge or slot-to-board-edge process
floor. The saved diagram `Hole_to_board_edge.892a998.png` labels castellated
pad-to-board-edge, castellated hole diameter, and castellated hole-to-hole
conditions. Those values are not used for the peer release's generic
`DRILL_TO_BOARD_EDGE_CLEARANCE_VALID` or `SLOT_TO_BOARD_EDGE_CLEARANCE_VALID`
suggestions because the imported release evidence does not classify
castellated pads/holes, and the current drill-edge rule measures generic
circular hole edge clearance.
