# ICM-42688-P IMU Observation

This direct-open GUI example proves the source-backed TDK InvenSense
ICM-42688-P IMU model can be used in generated SPICE observations.

The fixture uses 3.3 V VDD and VDDIO rails, explicit host-driven SPI idle
states, SDO low, and INT1 high. It is intended for preliminary rail, SPI
line-state, and interrupt-output checks, not sensor dynamics, register
protocol, FIFO timing, noise, bias stability, vibration, package stress, or
layout-coupling sign-off.
