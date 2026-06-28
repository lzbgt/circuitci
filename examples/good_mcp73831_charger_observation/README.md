# MCP73831 Charger Observation Example

This fixture exercises a datasheet-backed Microchip MCP73831-2 single-cell
Li-Ion charger component in generated Board IR SPICE. The vendor model keeps
the MCP73831 datasheet pinout, PROG resistor charge-current equation, 4.2 V
regulation target, and static `battery_charger` limits, while its
`simulation.spice` face points to CircuitCI's reduced charger macro-model.

The circuit drives `VDD` from a 5 V USB source, programs fast-charge current
with a 10 kOhm `RPROG`, inserts a zero-volt `VSENSE` source for charge-current
measurement, and charges a small 1 uF battery-node capacitor. The run setup
checks that current is near the datasheet 100 mA point, that `VBAT` rises past
4 V, and that the battery node remains below the regulation ceiling.

This is useful for generated-SPICE charger wiring, PROG resistor, probe,
assertion, and GUI model-pack workflows. It is not an MCP73831 thermal,
preconditioning, charge-termination, timer, STAT, battery-chemistry, cell
safety, package-dissipation, or final charger sign-off model.

The fixture is registered in the GUI Examples picker as `MCP73831 Charger`.
Opening it lands in Sketch with the routed schematic, and `Create Checks`
regenerates a model-aware observation preset for `UCHG`.

Sources:

- `docs/research/datasheets/microchip/mcp73831-family-datasheet.pdf`
- `docs/microchip_mcp73831_model.md`
