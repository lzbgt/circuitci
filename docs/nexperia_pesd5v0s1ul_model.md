# Nexperia PESD5V0S1UL Model

`vendor.nexperia.pesd5v0s1ul` is a source-backed single-line unidirectional
ESD protection diode model for static USB VBUS and 5 V line protection review.

The model records Nexperia's static board-level facts:

- 5.0 V maximum reverse standoff voltage.
- 200 pF maximum diode capacitance at 1 MHz and 0 V reverse bias.
- DFN1006-2 / SOD882 package.
- Cathode `K` on the protected positive line and anode `A` to ground.
- 150 W / 15 A non-repetitive 8/20 us pulse ratings.
- 30 kV IEC 61000-4-2 contact-discharge ESD rating.

This is a static interface-protection model. It can prove that a VBUS net has
source-backed clamp coverage with a known standoff and capacitance value, but it
does not simulate an ESD strike, surge heating, leakage over temperature, USB
inrush, connector placement, return path, or final hardware robustness.

## Evidence

The official Nexperia PDF is retained at
`docs/research/datasheets/nexperia/pesd5v0s1ul.pdf`. Source notes and hashes
are recorded in `docs/research/datasheets/nexperia/pesd5v0s1ul_sources.md`.
