# EasyEDA/JLC Flying-Probe Importer

`circuitci import-easyeda-flying-probe` enriches an existing Board IR project
with plaintext pad and net evidence from the JLC/EasyEDA
`FlyingProbeTesting.json` fabrication artifact.

```bash
circuitci import-easyeda-flying-probe fabrication/FlyingProbeTesting.json \
  --project out/imported_assembly.project.yaml \
  --output out/imported_with_probe_pads.project.yaml
```

## Imported Evidence

The importer reads `pins.fields` / `pins.rows` and requires these columns:

- `PIN_NAME`
- `PIN_X`
- `PIN_Y`
- `LAYER`
- `PIN_TYPE`
- `NET_NAME`
- `PAD_SHAPE`
- `PAD_SIZEX`
- `PAD_SIZEY`
- `HOLE_SIZE`
- `PAD_ANGLE`

Observed EasyEDA/JLC files use `lengthUnit: "mil"`, which CircuitCI converts
to millimeters. The importer also accepts `lengthUnit: "mm"`.

Connected rows become `board.layout.pads.<component>.<pin>` evidence. `PIN_NAME`
is split at the last underscore, so `C1_1` becomes component `C1`, pin `1`.
`T` maps to `F.Cu`, and `B` maps to `B.Cu`.

The importer creates `board.nets` entries for observed net names. Because the
flying-probe file proves connectivity but not rail semantics, imported nets are
classified conservatively:

- `GND`, `*_GND`, and `*-GND`: `kind: ground`
- all other imported nets: `kind: digital_or_analog`

If a pad-owning component reference is missing from the existing project, the
importer creates a pad-only placeholder component using
`generic.schematic.imported_component`. It intentionally does not add imported
pad names to `board.components.*.pins`, because common fabricated packages can
exceed the generic placeholder model's declared pin count. The pad evidence is
used for fabrication artwork ownership association; schematic-grade electrical
connectivity still requires a schematic/layout source.

## Duplicate Rows

JLC/EasyEDA flying-probe files can contain duplicate rows:

- identical duplicate rows are deduplicated and counted as duplicate pin rows;
- same component/pin/net rows with distinct same-layer geometry are treated as
  multipart pad geometry and imported with stable synthetic pad keys such as
  `2#2`;
- conflicting duplicate rows with different electrical ownership fail closed.

This keeps real multipart pads usable for Gerber owner association while still
rejecting ambiguous net ownership.

## Downstream Use

This importer is most useful before Gerber copper, solder-mask, solder-paste,
and Excellon drill imports. Those fabrication importers can then associate
artwork or drill hits with `net`, `component`, and `pin` evidence instead of
reporting anonymous objects.

The importer does not infer routes, vias, zone islands, power-rail voltage,
component models, or schematic intent.
