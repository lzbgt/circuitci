# Microchip MCP23017 I2C I/O Expander Model

`vendor.microchip.mcp23017` is a source-backed board-boundary model for
Microchip's MCP23017 16-bit I2C GPIO expander. It includes static power/pin
metadata plus a reduced generated-SPICE I2C observation face.

The retained source is Microchip's official DS20001952D datasheet:
`docs/research/datasheets/microchip/mcp23017_mcp23s17_datasheet_ds20001952d.pdf`.

## Encoded Facts

- Power pins: `VDD` and `VSS`.
- I2C/configuration pins: `SCL`, `SDA`, `A0`, `A1`, `A2`, and `RESET`.
- Interrupt outputs: `INTA` and `INTB`.
- GPIO pins: `GPA0` through `GPA7` and `GPB0` through `GPB7`; `GPA7` and
  `GPB7` are recorded as output-only for MCP23017.
- Static supply range: `1.8 V` to `5.5 V`.
- Datasheet metadata records 1 mA maximum supply current at 1 MHz, 1 uA
  standby current, 1.7 MHz I2C class, 16 GPIO bits, eight hardware addresses,
  and 25 mA absolute per-output source/sink current.
- `CIRCUITCI_MCP23017_I2C_GPIO_EXPANDER_OBSERVATION` checks board-level VDD,
  I2C pull-ups, address-select pins, reset release, interrupt idle outputs, and
  representative GPIO line states while leaving all pins high impedance.

## Validation Use

The good fixture powers the expander from a 3.3 V rail, pulls `SDA`/`SCL` and
`RESET` high, ties `A0`/`A1`/`A2` low for the base I2C address, pulls `INTA` and
`INTB` high, and binds every 28-pin SSOP board-boundary pin. The bad fixture
powers the same part from a 6 V rail, which fails `POWER_TREE_VALID` against
the source-backed 5.5 V maximum.

`examples/good_microchip_mcp23017_i2c_gpio_expander_observation/project.yaml`
is a direct-open GUI fixture that runs the reduced observation model.

## Boundary

This is not an I/O-expander protocol or firmware model. It does not prove I2C
transactions, register configuration, GPIO direction, interrupt-on-change
logic, weak-pull-up behavior, output-load thermal limits, firmware pin-state
sequences, or high-speed layout/signal integrity.
