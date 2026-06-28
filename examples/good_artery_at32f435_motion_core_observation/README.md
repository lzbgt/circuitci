# AT32F435 Motion-Core Observation

This example validates a source-backed reduced generated-SPICE face for the
Artery AT32F435 motion-core MCU role used in the smart-robot control stack.
It checks the 3.3 V VDD rail plus board-facing LicheeRV UART, motion-enable,
motion-fault, CAN, RS-485, and servo-PWM enable line states.

The model is intentionally limited to preliminary rail and low-speed line-state
evidence. It does not model firmware execution, reset/clock timing, CAN or
RS-485 protocol timing, ADC/motor-control behavior, exact package pin
assignment, thermal behavior, layout, or final signal-integrity sign-off.
