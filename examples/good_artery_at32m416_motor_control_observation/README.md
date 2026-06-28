# AT32M416 Motor-Control Observation

This fixture exercises the source-backed `vendor.artery.at32m416_motor_control`
model through CircuitCI's generated-SPICE path. Its `simulation.spice` face
points to CircuitCI's reduced motor-control MCU macro-model in
`models/spice/generic/analog_behavioral.lib`.

The scenario checks the 3.3 V VDD rail, CAN TX/RX idle state, six PWM output
states, DRV8323-style enable/fault/SPI lines, current-sense midscale nodes,
encoder inputs, board enable, and fault output. The model is intentionally a
board-boundary observation aid, not firmware, PWM-timing, ADC, FOC, gate-drive,
layout, thermal, or EMC sign-off.
