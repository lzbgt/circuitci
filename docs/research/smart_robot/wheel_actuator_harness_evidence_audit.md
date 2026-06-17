# Wheel Actuator Harness Evidence Audit

Date: 2026-06-17

This note records the actuator-bus harness evidence reviewed for the smart
robot wheel actuator sign-off gates.

## Sources Reviewed

| Source | Local file | SHA-256 |
| --- | --- | --- |
| JST VH connector datasheet | `docs/research/smart_robot/sources/jst_vh_connector_datasheet.pdf` | `d51e669c597988b20c0963daf5bef7356cbd2104c1f867e9107c6fa6cd2b899c` |
| JST handling precautions for terminals and connectors | `docs/research/smart_robot/sources/jst_handling_precautions_terminals_connectors.pdf` | `c0f1b065990fe550066f77576dc7a16b8c83fcde9a364b29475497bc334cd8c4` |
| National Wire UL AWM 1015 datasheet | `docs/research/smart_robot/sources/national_wire_ul_awm_1015.pdf` | `8b98c03ef1dac8985a36ca275badda37587a2d8d674c0a5725fbbb698fa635ea` |

## Evidence That Is Sufficient

- The JST VH datasheet supports the selected connector family and the current
  model values used by CircuitCI: 10 A current rating with AWG16 wire and
  250 V voltage rating.
- The National Wire UL AWM 1015 datasheet supports the first-pass AWG16
  resistance model. For AWG16 N600-2630U it lists 4.59 ohm per 1000 ft at
  20 C. For a 0.5 m two-wire loop, CircuitCI uses 0.01506 ohm.
- The JST handling precautions support keeping current paths explicitly
  modeled instead of assuming parallel connector circuits share current
  equally. This matches the current validation approach: one declared harness
  current path, one declared return path, and no inferred current sharing.

## Evidence That Is Not Sufficient

The reviewed sources do not provide a numeric temperature-rise test vector for
the final actuator harness. In particular, they do not specify:

- test current for the complete JST VH/AWG16 harness assembly,
- measured temperature rise at that current,
- allowable temperature rise for this installation, or
- derating for enclosure airflow, bundling, flexing, and crimp process.

Therefore `LOAD_CABLE_THERMAL_DERATING_VALID` must continue to fail closed for
`JACT1_CABLE`. The validator should not derive a temperature-rise point from
wire gauge, connector current rating, or conductor resistance alone.

## Required Evidence To Close The Gate

To clear the remaining cable thermal sign-off gate, add one of:

- a selected cable assembly datasheet with test current, temperature rise, and
  operating-temperature limit for the actual wire, crimp, connector, and cable
  construction; or
- a measured harness test for the selected assembly, using the expected wheel
  rail current, installed length, routing, bundling, and enclosure conditions.

Until then, the wheel actuator validation should remain non-fabrication-ready
even though connector current and voltage-drop screens pass.
