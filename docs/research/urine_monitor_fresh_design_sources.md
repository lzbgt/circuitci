# Peer `urine_monitor` Fresh-Design Evidence

This note records local evidence from `../urine_monitor` that should drive
CircuitCI component-library, importer, and public-board assessment work. The
peer project has fabricated JLC/EasyEDA Pro releases and curated local
datasheets, so it is a better next benchmark source than synthetic fixtures.

## As-Built Release

- `../urine_monitor/docs/fresh_design/artifacts/jlc_eda_releases/DELIVERY_20260428_combined_v01/`
- EasyEDA Pro source:
  `source/easyeda_pro_project_UM-STM32L4_V1.eprj2`
- Assembly evidence:
  `assembly/bom_STM32_ESP32_V01_2026-04-28.csv`,
  `assembly/placement_STM32_ESP32_V01_2026-04-28.csv`
- Fabrication evidence:
  `fabrication/gerber_STM32_ESP32_V01_2026-04-28.zip`
- Review evidence:
  `review/schematic_UM-STM32L4_V1.pdf`,
  `review/schematic_UM-STM32L4_V1.extracted.txt`,
  `review/pcb_layout_UM-STM32L4_V1.pdf`

This release is now the first peer-board import target for fabricated evidence:
`import-jlc-assembly` reads the JLC/EasyEDA BOM and placement CSV files into
Board IR component source metadata plus `board.layout.placements`;
`import-easyeda-flying-probe` reads `FlyingProbeTesting.json` into
`board.layout.pads` and imported net evidence; `import-gerber-outline` reads the
observed `Gerber_BoardOutlineLayer.GKO` subset into
`board.layout.outline.segments`; `import-gerber-copper` reads the observed
top/bottom copper Gerbers, including EasyEDA `RoundRect` apertures and sampled
`G02`/`G03` circular-arc draws; `import-gerber-solder-mask` and
`import-gerber-solder-paste` read real solder-mask and paste stencil evidence;
and `import-excellon-drill` reads the observed PTH, PTH-via, and NPTH NC drill
subsets into `board.layout.drills`. The remaining import gap is schematic-grade
electrical intent and route/via/zone evidence beyond what the flying-probe
artifact proves.

Observed owner-association evidence from the release flow with flying-probe
pads imported first:

- Flying-probe pads: `3168` pin rows, `2985` connected pad rows,
  `183` duplicate pin rows, `17` multipart pin rows, `1432` pad-only
  placeholder components, and `440` imported nets.
- Top copper: `2725` flash features, `2567` trace segments, `22` regions,
  `2018` net-associated features, `1255` net-associated segments, and `22`
  net-associated regions.
- Bottom copper: `1275` flash features, `854` trace segments, `3` regions,
  `96` net-associated features and `33` net-associated segments after applying
  JLC placement side evidence to flying-probe pad layers.
- Top solder mask: `1546` flash openings, `7` region openings, `121`
  owner-associated flash openings.
- Bottom solder mask: `96` flash openings and `9` owner-associated openings
  after applying JLC placement side evidence to flying-probe pad layers.
- Top solder paste: `1111` flash openings, `354` region openings, `103`
  owner-associated flash openings, and `9` owner-associated region openings.
- PTH drills: `1275` hits with `9` pad-associated hits.
- PTH via drills: `1179` hits with `472` via-associated hits from co-located
  Gerber copper net evidence.
- NPTH drills: `31` non-plated hits with no pad/via owners.

The baseline peer validation still passes with no findings:

```text
CircuitCI urine_monitor_jlc_assembly: pass (critical=0, warning=0, info=0)
```

`circuitci inspect-easyeda-pro` now records what the local EasyEDA Pro source
can prove without decoding private design-object payloads. On
`source/easyeda_pro_project_UM-STM32L4_V1.eprj2`, it finds:

- `1` project and `2` branches.
- `512` project-structure snapshots.
- Latest structure ticket `183013`.
- `1` board, `1` schematic, `11` sheets, and `1` PCB.
- `642` encoded/non-JSON history payloads.

The plaintext latest structure identifies board `dd654e1cf9b905cf`
(`UM-STM32L4_V01`) and PCB `f5d31030bb8dd7c8`
(`Voice_Agentic_PCB_V0`), but the pad/via/route/net-bearing history payloads
are encoded. CircuitCI should therefore require an unencoded EasyEDA layout
export or a documented decoder before claiming pin-level ownership from this
`.eprj2` file.

## Project-Level Evidence

- ESP32-S3 hub/node design:
  `../urine_monitor/docs/fresh_design/projects/um_esp32s3_v1/README.md`
- STM32L4 node design:
  `../urine_monitor/docs/fresh_design/projects/um_stm32l4_v1/README.md`
- ESP32-S3 BOM variants:
  `../urine_monitor/docs/fresh_design/projects/um_esp32s3_v1/bom/`
- STM32L4 BOM variants:
  `../urine_monitor/docs/fresh_design/projects/um_stm32l4_v1/bom/`
- Downloaded local datasheet manifest:
  `../urine_monitor/docs/fresh_design/lcsc_downloads/manifest.json`
- Downloaded local datasheets:
  `../urine_monitor/docs/fresh_design/lcsc_downloads/datasheets/`

The ESP32-S3 design README identifies the current baseline as the EasyEDA Pro
delivery above and calls out `DX-LR30-433M22S`, `TP4056`, `TPS63802DLAR`,
`TPS61023DRLR`, `NAU7802`, `SPH0645LM4H-1-8`, `MAX98357A`, and `SU-03T`.

The STM32L4 design README identifies fitted parts including
`DX-LR30-433M22S`, `TP4056`, `TPS63802DLAR`, `TPS61023DRLR`, `W5500`,
`ESP-12F`, `CH340C`, `W25Q32JVSSIQ`, `AT24C02`, `ATECC608A`, `AHT20`,
`CPS121`, and `MCP23017`.

## High-Value Datasheet-Backed Library Targets

Prioritize parts that appear on fabricated or fresh-design boards and improve
existing CircuitCI checks:

- MCU/module models: `ESP32-S3-WROOM-1-N16R8`, `STM32L431CBT6`,
  `STM32L431VCT6`, `ESP-12F`. CircuitCI now has a first
  `ESP32-S3-WROOM-1U-N16R8` static model for the peer ESP32 hub path.
- Power-path and regulators: `BQ24075RGTR`, `TPS61023DRLT`,
  `TPS63802DLAR`, `TPS63060DSCR`, `TPS63070RNMR`, `TPS61236*`,
  `TPS2121RUXT`, `TPS2113A*`, `TP4056`. CircuitCI now has a first
  `BQ24075RGTR` static charger model and a `TPS2121RUXT` static power-mux
  model for peer power paths. CircuitCI now also has a first `TPS61023DRLT`
  static 5 V boost model with input-inductor screening, and a first
  `TPS63802DLAR` static 3.3 V buck-boost model with direct L1-L2
  switch-inductor screening.
- Protection: `USBLC6-2SC6`, `TPD4E05U06DQAR`, `SMF5.0A`, `SM6T6V8CA`,
  resettable fuses.
- Memory/security/I/O expanders: `W25Q32JVSSIQ`, `AT24C02C-SSHM-T`,
  `ATECC608A-SSHDA-T`, `MCP23017-E/SS`.
- Sensors/audio: `NAU7802SGI`, `AHT20`, `CPS121`, `MAX98357A`, `INMP441`,
  `SU-03T`.
- RF modules and interfaces: `DX-LR30-433M22S`, E22 modules,
  `SX1262`/`LLCC68`, U.FL/SMA connectors, chip antennas.
- Board-entry/connectors: USB-C 16-pin, JST/GH/PH, FPC, antenna connectors.

## Recommended Work Queue

1. Extend direct support for JLC/EasyEDA release evidence beyond BOM/CPL,
   Gerber outline, and drill hits: pad/route geometry and eventually `.eprj2`
   schematic/layout import.
2. Add the remaining ESP32-S3-WROOM and STM32L431 component packs using the
   peer datasheets and public vendor datasheets as authoritative sources.
3. Expand `TPS63802`, `TPS61023`, and `TPS2121` beyond static screening with
   current-limit resistor,
   operating-point, switchover, and inductor saturation evidence.
4. Add memory/security/sensor packs for the fitted STM32L4 peripherals so
   scenario suggestions can recognize common pull-up, rail, and bus checks.
5. Use the fabricated release as an end-to-end benchmark suite once enough
   import coverage exists.
