# TI TPS62162 Datasheet Source

- Product page: <https://www.ti.com/product/TPS62162>
- Datasheet: <https://www.ti.com/lit/ds/symlink/tps62160.pdf>
- Local copy: `docs/research/datasheets/ti/tps62160.pdf`
- SHA-256:
  `6c26eb5b607f7af543c24ed99b8ed942e4819326352ae9fb87e8396686d06659`
- Retrieved: 2026-06-13

## Extracted Board-Level Facts

The saved TI datasheet and product page identify TPS62162 as a fixed-output
member of the TPS6216x synchronous step-down converter family. Board-level
facts used by `vendor.ti.tps62162_3v3` are:

- buck regulator topology,
- input range `3 V` to `17 V`,
- fixed `3.3 V` output variant,
- maximum output current `1 A`,
- typical quiescent current `17 uA`,
- typical switching frequency `2.25 MHz`,
- typical support-component screen using input and output ceramic capacitance.

CircuitCI currently uses only static power-tree facts from this source. The
model deliberately does not sign off inductor value/saturation current, ripple,
control-loop stability, switch-node stress, thermal behavior, EMI, or PCB
layout.
