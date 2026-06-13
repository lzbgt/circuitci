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

This release should become the first peer-board import target. The immediate
CircuitCI gap is direct EasyEDA Pro/JLC BOM/CPL/Gerber ingestion or a narrow
adapter that converts those release files into Board IR plus scenario
suggestions.

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

1. Add direct support for JLC/EasyEDA release evidence: BOM, CPL placement,
   Gerber/outline, and eventually `.eprj2` schematic/layout import.
2. Add the remaining ESP32-S3-WROOM and STM32L431 component packs using the
   peer datasheets and public vendor datasheets as authoritative sources.
3. Expand `TPS63802`, `TPS61023`, and `TPS2121` beyond static screening with
   current-limit resistor,
   operating-point, switchover, and inductor saturation evidence.
4. Add memory/security/sensor packs for the fitted STM32L4 peripherals so
   scenario suggestions can recognize common pull-up, rail, and bus checks.
5. Use the fabricated release as an end-to-end benchmark suite once enough
   import coverage exists.
