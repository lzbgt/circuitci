# AP2112K 3.3 V LDO Observation Example

This fixture exercises a datasheet-backed vendor component in generated Board IR
SPICE. `vendor.diodes.ap2112k_3v3` keeps AP2112K datasheet voltage, dropout,
current, enable, and capacitor limits, while its `simulation.spice` face points
to CircuitCI's generic enabled 3.3 V LDO macro-model.

The circuit drives `VIN` and `EN` from explicit 5 V sources and loads `VOUT`
through a 3.3 kOhm resistor. The analog run setup checks that the observed
output rail stays inside the AP2112K fixed 3.3 V +/-1.5% datasheet window and
that the generated branch-current probe stays between about `0.9 mA` and
`1.1 mA` in magnitude.

This is useful for generated-SPICE workflow, probe placement, assertion, and GUI
model-pack checks. It is not an AP2112K transient, stability, thermal, PSRR,
noise, startup, current-limit, or output-capacitor ESR sign-off model.

Sources:

- `docs/research/datasheets/diodes/ap2112.pdf`
- `docs/research/datasheets/diodes/ap2112_sources.md`
- `docs/diodes_ap2112k_3v3_model.md`
