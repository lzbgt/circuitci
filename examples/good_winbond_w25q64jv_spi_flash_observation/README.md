# Winbond W25Q64JV SPI Flash Observation

This fixture opens directly in the GUI as `W25Q64JV SPI Flash`. It exercises
the `CIRCUITCI_W25Q64JV_SPI_OBSERVATION` generated-SPICE face in
`models/spice/winbond/w25q64jv_spi_observation.lib`.

The observation checks a 3.3 V `VCC` rail, `/CS` high standby state,
`/WP` and `/HOLD or /RESET` pull-ups, explicit low idle-bias on `CLK` and
`DI/IO0`, and a high-impedance `DO/IO1` reference. The model is intentionally
high impedance and does not
emulate SPI commands, JEDEC ID, SFDP, flash contents, erase/program state,
write-protect policy, XIP behavior, retention, endurance, or high-speed signal
integrity.
