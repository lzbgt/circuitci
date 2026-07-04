# Kingbright APT1608SURCK Model

`vendor.kingbright.apt1608surck` is a source-backed red 0603 indicator LED
model for generated Board IR SPICE checks.

The model records Kingbright's static operating limits:

- 30 mA DC forward current.
- 5 V reverse voltage.
- 75 mW power dissipation.
- 1.95 V typical and 2.5 V maximum forward voltage at 20 mA.
- 35 pF capacitance, 630 nm dominant wavelength, and 120 degree viewing angle.

The bundled SPICE card is a reduced electrical fit for generated transient
plumbing and operating-limit probes. It is not an optical, thermal, aging, or
pulse-drive sign-off model.

## Evidence

The official Kingbright PDF is retained at
`docs/research/datasheets/kingbright/apt1608surck.pdf`. Source notes and hashes
are recorded in `docs/research/datasheets/kingbright/apt1608surck_sources.md`.
