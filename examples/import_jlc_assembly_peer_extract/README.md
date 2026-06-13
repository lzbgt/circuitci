# JLC/EasyEDA Assembly Import Peer Extract

This fixture is a small, stable extract shaped like the fabricated
`../urine_monitor` JLC/EasyEDA Pro release assembly evidence:

- peer BOM:
  `../urine_monitor/docs/fresh_design/artifacts/jlc_eda_releases/DELIVERY_20260428_combined_v01/assembly/bom_STM32_ESP32_V01_2026-04-28.csv`
- peer placement:
  `../urine_monitor/docs/fresh_design/artifacts/jlc_eda_releases/DELIVERY_20260428_combined_v01/assembly/placement_STM32_ESP32_V01_2026-04-28.csv`

It is intentionally assembly-only. The importer preserves BOM/component and
placement evidence but does not infer nets, pins, or electrical sign-off from
these CSV files.
