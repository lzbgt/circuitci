# Native KiCad Symbol Rotation

## Purpose

Real KiCad schematics commonly rotate symbols, especially passives and source
symbols. Native `.kicad_sch` import must apply symbol rotation to library pin
offsets before building nets, otherwise rotated parts either fail import or map
pins to the wrong coordinates.

## Source Basis

The saved KiCad S-expression reference under
`docs/research/kicad/sexpr-intro.html` defines `(at X Y [ANGLE])` as the
position identifier and states that non-text angles are stored in degrees.

## Import Contract

The native schematic importer supports cardinal symbol rotations:

- `0`
- `90`
- `180`
- `270`
- equivalent wrapped values such as `-90` or `450`

If an angle field is present, it must parse as a finite number. Malformed,
non-finite, or non-cardinal angles fail closed.

The transform is applied to each library pin coordinate before adding the
symbol origin:

- `0`: `(x, y)`
- `90`: `(-y, x)`
- `180`: `(-x, -y)`
- `270`: `(y, -x)`

Non-cardinal rotations fail closed. This keeps coordinate equality, wire
attachment, label attachment, and `no_connect` matching deterministic on the
same quantized grid used by the existing importer.

Power symbols use the same transform before their one-pin power label is
injected. Schematic `no_connect` markers also match transformed pin
coordinates. A marker at the old unrotated coordinate is treated as unattached
and rejected.

## Non-Goals

This slice does not add support for:

- mirrored symbols,
- rotated labels or text orientation semantics,
- hidden pins,
- hierarchical sheets,
- buses.

Those features need separate import contracts because they affect connectivity
or naming beyond a simple pin-coordinate transform.

## Required Coverage

- rotated 90-degree passives import and validate through the existing mapped
  generated-SPICE path,
- equivalent wrapped rotations produce the same pin placement,
- malformed, non-finite, non-cardinal, and wrapped non-cardinal rotations fail
  closed,
- unsupported mirror syntax fails closed,
- labels and wire segments attached to transformed pins import successfully,
- existing no-connect and unconnected-pin checks still operate on transformed
  pin coordinates,
- rotated power symbols transform their one exposed pin before label injection.
