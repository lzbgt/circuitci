# TI TPS54331 5 V Model Notes

`vendor.ti.tps54331_5v` is a source-backed static and generated-SPICE model for
preliminary 5 V buck-regulator rail checks.

Source references are pinned in
`libs/vendor/ti/regulators/tps54331_5v.model.yaml`:

- TI TPS54331 datasheet:
  `docs/research/smart_robot/sources/tps54331_datasheet.pdf`

The source-backed static metadata captures the datasheet-backed 3.5 V to
28 V input range, 3 A output-current class, 570 kHz switching-frequency class,
and a 5 V configured output rail represented through the VSENSE-connected
output net.

The model also declares `simulation.spice` metadata for generated Board IR
observation. Its SPICE face points to
`models/spice/generic/analog_behavioral.lib` with pin order:

```text
VIN EN GND VSENSE
```

The generated subcircuit instance can map a Board IR component parameter into
the output target:

```yaml
instance_parameters:
  - spice_name: VOUT_V
    component_parameter: observation_output_voltage_V
    default_value: 5.0
```

This keeps the output rail target visible in component-model metadata while
letting GUI-generated setups run without a hand-authored parameter.

## Scope

The reduced behavioral macro-model is valid for:

- VIN, EN, VSENSE wiring observation,
- preliminary 5 V output-window checks,
- preliminary load-current probe/check workflows,
- GUI workflow and report-bundle evidence generation.

It is not valid for:

- PH/BOOT switching waveform sign-off,
- compensation or control-loop stability,
- inductor ripple/current, DCR, or saturation,
- output ripple or load-transient behavior,
- current limit, Eco-mode, startup timing, thermal behavior, layout, EMI, or
  final regulator sign-off.

The direct-open GUI fixture lives at:

```text
examples/good_tps54331_5v_buck_observation/project.yaml
```
