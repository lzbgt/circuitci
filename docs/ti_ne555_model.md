# TI NE555 Model

`vendor.ti.ne555` is a source-backed static model for the Texas Instruments
NE555 precision timer.

The model covers:

- `VCC` operating range from 4.5 V to 16 V for the NA555/NE555/SA555 family.
- `GND` required ground.
- Timer board-boundary pins `TRIG`, `OUT`, `RESET`, `CONT`, `THRES`, and
  `DISCH`.
- No-load supply-current metadata from the datasheet's VCC = 15 V electrical
  table.
- Output current class and VCC bypass recommendation as retained datasheet
  metadata.

The source original and notes are under
`docs/research/datasheets/ti/ne555.pdf` and
`docs/research/datasheets/ti/ne555_sources.md`.

The model is intended for static power-tree and pin-boundary checks. It is not
an RC timing calculator, generated-SPICE timer model, output-drive sign-off, or
thermal/transient-current sign-off model.
