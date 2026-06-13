# TI TPS62162 3.3 V Model

## Source

- Datasheet:
  `docs/research/datasheets/ti/tps62160.pdf`
- Source URL:
  <https://www.ti.com/lit/ds/symlink/tps62160.pdf>
- Product page:
  <https://www.ti.com/product/TPS62162>
- SHA-256:
  `6c26eb5b607f7af543c24ed99b8ed942e4819326352ae9fb87e8396686d06659`
- Retrieved: 2026-06-13

## Modeled Facts

The `vendor.ti.tps62162_3v3` model captures static board-level facts for the
fixed 3.3 V TPS62162 synchronous buck regulator:

- Input operating range: `3.0 V` to `17.0 V`.
- Fixed output voltage: `3.3 V`; the model uses a conservative static
  acceptance window of `3.201 V` to `3.399 V` so Board IR nominal rails can be
  screened with the existing regulator rule.
- Maximum output current: `1.0 A`.
- Typical switching frequency: `2.25 MHz`, recorded as datasheet metadata only.
- Typical support-component screen: `10 uF` input capacitance and `22 uF`
  output capacitance to ground.
- Static output inductor screen: `SW` must connect to the output rail through
  at least `2.2 uH` direct modeled inductance. This follows the datasheet
  inductor-selection section, which states that TPS6216x can operate as low as
  `2.2 uH` and recommends `3.3 uH` for low input voltage/full-current designs.

## Validation Use

`POWER_TREE_VALID` uses this model through `power_conversion` metadata:

- `VIN` and `VOS` must connect to explicit power rails.
- `VIN` must be inside the modeled input-voltage range.
- The output rail must be a 3.3 V-class rail inside the modeled static output
  window.
- The summed declared output load must not exceed `1.0 A`.
- The board must include at least `10 uF` input and `22 uF` output capacitance
  to ground.
- The board must include modeled output inductance directly between the `SW`
  net and `VOS` output rail. This is a static direct-link screen; it does not
  prove saturation current, DCR, ripple, or loop stability.

This is a static buck-regulator screen. It is not valid for saturation-current
selection, loop compensation, ripple, transient response, switch-node stress,
thermal sign-off, EMI, or PCB layout sign-off.
