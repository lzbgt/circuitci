# TI CSD17484F4 Model

## Source

- Datasheet: `docs/research/datasheets/ti/csd17484f4.pdf`
- Source URL: <https://www.ti.com/lit/gpn/CSD17484F4>
- SHA-256:
  `b9fb6128120a0901624bc9cb4454d797c20b39b44b106628389d00f853efb5e0`
- Retrieved: 2026-06-14

## Modeled Facts

The `vendor.ti.csd17484f4` model captures source-backed metadata for exact part
binding and preliminary static MOSFET review:

- N-channel FemtoFET 0402 LGA pinout: `1=G`, `2=S`, `3=D`.
- Drain-source rating: `30 V`.
- Continuous gate-source rating: `12 V`.
- Continuous drain current: `3 A`.
- Pulsed drain current: `18 A`, scoped to the datasheet pulse condition
  `<=100 us` and `<=1%` duty cycle.
- Continuous gate current: `35 mA`.
- Power dissipation: `0.5 W` at `TA=25C`.
- RDS(on) rows for `VGS=1.8 V`, `2.5 V`, and `4.5 V`.
- Gate charge and capacitance values needed to reason about whether a logic
  output is enough evidence for a real gate-drive waveform.

## Validation Use

This pack is intentionally not a SPICE model. It lets CircuitCI bind an imported
component to the real `CSD17484F4` and carry the correct ratings into reports,
but transient switching, laser-pulse SOA, gate-drive waveform, and thermal
sign-off still require a sourced SPICE model or measured waveform evidence.

The TOF-R5001 Q2 analysis uses this model to avoid treating the Altium generic
`MOSFET-N` symbol as physical evidence. Without a sourced `CSD17484F4` model or
bench waveform, the correct result is fail-closed rather than simulated false
confidence.
