# Microchip MCP23017 Source Notes

## Retained Source

| Document | Source URL | Local copy | SHA-256 |
| --- | --- | --- | --- |
| MCP23017/MCP23S17 data sheet DS20001952D | <https://ww1.microchip.com/downloads/aemDocuments/documents/APID/ProductDocuments/DataSheets/MCP23017-Data-Sheet-DS20001952.pdf> | `docs/research/datasheets/microchip/mcp23017_mcp23s17_datasheet_ds20001952d.pdf` | `63cb5f2bec44434cdeeada1790d0316c9dc06b33febb489ad87bb0e2d540496a` |

Retrieved on 2026-07-13 from Microchip's official `ww1.microchip.com`
document host. A local text extraction is retained at
`docs/research/datasheets/microchip/mcp23017_mcp23s17_datasheet_ds20001952d.txt`
for agent-side source review.

## Modeled Facts

- MCP23017 is the I2C member of the MCP23017/MCP23S17 16-bit I/O expander
  family.
- The retained datasheet lists the MCP23017 operating voltage range as 1.8 V
  to 5.5 V, with 1.7 MHz maximum I2C interface class.
- The 28-lead SSOP pinout includes `VDD`, `VSS`, `SCL`, `SDA`, `A0`, `A1`,
  `A2`, `RESET`, `INTA`, `INTB`, eight `GPA` pins, and eight `GPB` pins.
- The datasheet says `A0`/`A1`/`A2` and `RESET` must be externally biased.
- `INTA` and `INTB` are configurable interrupt outputs and are associated with
  PORTA and PORTB by default.
- I/O pins default to inputs; the datasheet flags `GPA7` and `GPB7` as
  output-only for MCP23017.
- The model records 1 mA maximum supply current at 1 MHz, 1 uA maximum standby
  current over -40 C to +85 C, 25 mA absolute maximum per-output source/sink
  current, VDD-ratio Schmitt input thresholds, and 40 uA to 115 uA weak pull-up
  current metadata.

## Model Boundary

`vendor.microchip.mcp23017` is a board-boundary model. It supports power-tree
voltage/current screening, explicit I2C/GPIO pin binding review, and a reduced
generated-SPICE board-observation model for VDD, I2C idle, address pins,
reset, interrupt outputs, and representative GPIO states.

The model does not emulate I2C transactions, register configuration, GPIO
direction, interrupt-on-change logic, weak pull-up behavior, output-load
thermal limits, firmware pin-state sequencing, or high-speed signal-integrity
timing.
