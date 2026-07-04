# TI NE555 Source Notes

## Retrieved Source

- `ne555.pdf`
  - Source: <https://www.ti.com/lit/ds/symlink/ne555.pdf>
  - SHA-256: `c7c6ababde92bb367579e95105e9a9d2d229b05377f5624ae4184031e6266a89`
  - Retrieved: 2026-07-05

## Modeled Facts

- The retained Texas Instruments datasheet identifies the NA555, NE555, SA555,
  and SE555 as precision timers for monostable and astable timing circuits.
- The pin-function table identifies the 8-pin role set: `GND`, `TRIG`, `OUT`,
  `RESET`, `CONT`, `THRES`, `DISCH`, and `VCC`.
- The recommended operating conditions list NA555/NE555/SA555 supply operation
  from 4.5 V to 16 V; the SE555 18 V maximum is not used by the
  `vendor.ti.ne555` catalog model.
- The recommended operating conditions list output current as +/-200 mA, while
  absolute maximum output current is +/-225 mA.
- The electrical table lists no-load supply current maxima of 15 mA with output
  low and 13 mA with output high at VCC = 15 V for NA555/NE555/SA555.
- The power-supply recommendation calls for a VCC-to-ground bypass capacitor;
  the retained model records 0.1 uF as the typical recommendation.

## Modeling Boundary

`vendor.ti.ne555` is a static board-boundary model. It supports power-tree
screening for the NE555 supply range and exposes the source-backed timer pins
for board review. It intentionally does not implement RC timing equations,
threshold spread, output-drive sign-off, discharge-transistor saturation,
thermal behavior, or a generated-SPICE behavioral timer. The checked-in
`examples/ne555_astable_scope_smoke` fixture remains a GUI/SPICE workflow smoke
using an idealized pulse source rather than this component model.
