# TI BQ25798 Model Notes

`vendor.ti.bq25798` is a source-backed static and generated-SPICE model for
preliminary PMU charger/power-path design checks.

Source references are pinned in
`libs/vendor/ti/chargers/bq25798.model.yaml`:

- TI BQ25798 product page:
  `docs/research/smart_robot/sources/bq25798_product.html`
- TI BQ25798 datasheet PDF:
  `docs/research/smart_robot/sources/bq25798_datasheet.pdf`

The source-backed static metadata captures that BQ25798 is an I2C-controlled
1- to 4-cell buck-boost charger with a 3.6 V to 24 V input range, a 5 A class
charge-current capability, and NVDC power-path behavior with integrated
switching MOSFETs and BATFET. Board IR uses
`programmed_charge_current_A` as the host-configured charge-current policy for
static current-budget checks.

The model also declares `simulation.spice` metadata for generated Board IR
observation. Its SPICE face points to
`models/spice/generic/analog_behavioral.lib` with pin order:

```text
VBUS SYS BAT GND
```

Generated subcircuit instances map Board IR component parameters into SPICE
instance parameters:

```yaml
instance_parameters:
  - spice_name: ICHG_A
    component_parameter: programmed_charge_current_A
  - spice_name: VSYS_V
    component_parameter: observation_system_voltage_V
    default_value: 12.0
```

This lets examples and GUI-created setups use the same placed BQ25798 component
metadata to drive an executable preliminary observation without hardcoding a
single charge-current value in the simulator. `VSYS_V` defaults to 12 V for
GUI-generated observation convenience; a project can override it with
`observation_system_voltage_V`.

## Scope

The reduced behavioral macro-model is valid for:

- adapter, SYS, BAT wiring observation,
- preliminary host-configured charge-current checks,
- preliminary SYS rail and battery-node waveform checks,
- GUI workflow and report-bundle evidence generation.

It is not valid for:

- buck-boost switching or loop-stability sign-off,
- DPM, MPPT, supplement mode, or BATFET dynamics,
- I2C register sequencing or status behavior,
- thermal regulation, safety timers, termination, battery chemistry, or final
  charger safety sign-off.

The direct-open GUI fixture lives at:

```text
examples/good_bq25798_nvdc_observation/project.yaml
```
