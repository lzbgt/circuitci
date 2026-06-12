# Native KiCad Symbol Rotation And Mirroring

## Purpose

Real KiCad schematics commonly rotate and mirror symbols, especially passives,
connectors, op-amps, and source symbols. Native `.kicad_sch` import must apply
symbol transforms to library pin offsets before building nets, otherwise
transformed parts either fail import or map pins to the wrong coordinates.

## Source Basis

The saved KiCad S-expression reference under
`docs/research/kicad/sexpr-intro.html` defines `(at X Y [ANGLE])` as the
position identifier and states that non-text angles are stored in degrees.
The schematic importer also accepts observed KiCad symbol mirror tokens in the
form `(mirror x)` and `(mirror y)`.

## Import Contract

The native schematic importer supports finite symbol rotations in degrees.
Equivalent wrapped values such as `-90`, `360`, `405`, and `450.1` are
normalized before transforming library pin offsets.

If an angle field is present, it must parse as a finite number. Malformed or
non-finite angles fail closed.

The transform is applied to each library pin coordinate before adding the
symbol origin. Cardinal rotations use exact integer transforms:

- `0`: `(x, y)`
- `90`: `(-y, x)`
- `180`: `(-x, -y)`
- `270`: `(y, -x)`

Non-cardinal rotations use the standard 2-D rotation matrix and round the
result to the same one-nanometer integer grid used by the existing importer:

- `x' = round(x cos(theta) - y sin(theta))`
- `y' = round(x sin(theta) + y cos(theta))`

This keeps wire attachment, label attachment, and `no_connect` matching
deterministic without requiring KiCad's graphical editor to place symbols only
on cardinal axes.

Mirror transforms are applied before cardinal rotation:

- no mirror: `(x, y)`
- `mirror x`: `(x, -y)`
- `mirror y`: `(-x, y)`

Malformed mirror tokens and unsupported mirror axes fail closed.

Power symbols use the same transform before their one-pin power label is
injected. Schematic `no_connect` markers also match transformed pin
coordinates. A marker at the old unrotated coordinate is treated as unattached
and rejected.

## Non-Goals

This slice does not interpret rotated text orientation semantics. Labels still
attach by their coordinate, not by the visual text angle.

## Required Coverage

- rotated 90-degree passives import and validate through the existing mapped
  generated-SPICE path,
- non-cardinal symbol rotations attach labels at the rounded transformed pin
  coordinates,
- equivalent wrapped rotations produce the same pin placement,
- malformed and non-finite rotations fail closed,
- mirrored symbols transform their pin offsets before label/wire matching,
- malformed or unsupported mirror syntax fails closed,
- labels and wire segments attached to transformed pins import successfully,
- existing no-connect and unconnected-pin checks still operate on transformed
  pin coordinates,
- rotated power symbols transform their one exposed pin before label injection.
