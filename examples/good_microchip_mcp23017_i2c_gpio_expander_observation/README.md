# Microchip MCP23017 I2C GPIO Expander Observation

This fixture exercises the source-backed `vendor.microchip.mcp23017` model with
the reduced high-impedance generated-SPICE observation face in
`models/spice/microchip/mcp23017_i2c_gpio_expander_observation.lib`.

The scenario checks a 3.3 V `VDD` rail, idle-high `SDA`/`SCL`, low address
pins for the base I2C address, released `RESET`, high idle interrupt outputs,
and representative GPIO line states. It intentionally does not emulate I2C
transactions, register configuration, GPIO direction, interrupt-on-change
logic, weak pull-ups, output loading, firmware sequencing, or signal integrity.
