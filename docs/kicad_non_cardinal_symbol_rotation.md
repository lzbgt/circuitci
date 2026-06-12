# Native KiCad Non-Cardinal Symbol Rotation

## Purpose

KiCad symbols can carry arbitrary finite rotation angles in their `(at X Y
[ANGLE])` field. The native schematic importer must preserve those pin
locations because real schematics can rotate symbols by non-cardinal angles,
and dropping or rejecting those coordinates blocks Board IR import before SPICE
validation can run.

## Source Basis

The saved KiCad S-expression reference in `docs/research/kicad/sexpr-intro.html`
documents the generic `(at X Y [ANGLE])` position identifier and stores angles
in degrees for non-text objects. CircuitCI already quantizes schematic
coordinates to an integer one-nanometer grid, so rotated pin offsets use the
same grid after transformation.

## Import Contract

For each schematic symbol:

- the angle defaults to `0` when omitted,
- the angle must parse as a finite number,
- wrapped values are normalized with Euclidean modulo `360`,
- exact cardinal angles keep integer transforms,
- non-cardinal angles use the 2-D rotation matrix and round to the nearest
  nanometer.

Malformed and non-finite angle fields fail closed. Labels, wires, and
`no_connect` markers attach only at the transformed coordinate.

The importer does not infer connectivity from visual text orientation. Label
text rotation remains presentation metadata; only the label coordinate affects
net construction.
