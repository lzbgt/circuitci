# TDK InvenSense ICM-42688-P Model

`vendor.tdk.icm42688p` is a source-backed TDK InvenSense ICM-42688-P
six-axis IMU model for preliminary power-tree and SPI/interrupt line-state
screening.

## Sources

- Datasheet PDF:
  `docs/research/smart_robot/sources/icm42688p_datasheet.pdf`
- Source URL:
  `https://product.tdk.com/system/files/dam/doc/product/sensor/mortion-inertial/imu/data_sheet/ds-000347-icm-42688-p-v1.6.pdf`
- SHA-256:
  `9663bf7d68e1ecc67486f452c9c62f0b85e1c22c569845ea7f66b4d91fee04a1`
- Retrieved: `2026-06-14`

## Modeled Facts

- VDD operating range: `1.71 V` to `3.6 V`.
- VDDIO operating range: `1.71 V` to `3.6 V`.
- SPI input thresholds: `2.0 V` high and `0.8 V` low.
- SDO and INT1 are represented as board-facing digital outputs with a
  `3.3 V` high-state metadata level and `80 ohm` source impedance.

## Generated-SPICE Face

`CIRCUITCI_ICM42688P_SPI_IMU` is a reduced observation model for:

- VDD and VDDIO rail checks.
- Host-driven SCLK, SDI, and CS line-state checks.
- SDO and INT1 static output-state checks.

The output states are explicit Board IR component parameters:

- `observation_sdo_state`
- `observation_int1_state`

The direct-open GUI fixture is:

- `examples/good_tdk_icm42688p_imu_observation/project.yaml`

Its `Create Checks` action regenerates VDD/VDDIO, SPI input, SDO, and INT1
checks for the placed IMU without editing YAML.

## Limits

This model is not valid for sensor dynamics, register protocol, FIFO behavior,
sampling timing, noise density, bias stability, sensor-fusion accuracy,
vibration analysis, package stress, layout coupling, or final SPI timing
sign-off. Those require separate runtime, measurement, layout, or signal
integrity evidence.
