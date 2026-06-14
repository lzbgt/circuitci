# Smart Robot Design Review

Date: 2026-06-14

Input document:

- `demos/smart_robot/overwall_design.md`

## Review Result

The modular direction is sound: keep the LicheeRV Nano as a purchased Linux
brain module and split real-time control into reusable PMU, motion-core, and
wheel-actuator boards. This reduces fabrication risk because charger/battery
safety, Linux IO, IMU/CAN/servo logic, and high-current BLDC switching can be
validated independently.

## Corrections And Constraints

- Use the Sipeed LicheeRV Nano source docs, not memory, for module facts. The
  first motion-core slice only relies on UART0 `A16/A17`, 5 V input, ground,
  and two GPIO-style control lines.
- Treat LicheeRV `3V3` as a reference or IO-domain signal only. Do not power the
  robot motion board from the module.
- Keep CAN and RS485 transceiver selection open until bus voltage, cable length,
  ESD target, connector, and termination policy are chosen.
- Keep PMU/charger design separate from the motion core. Battery charging and
  e-stop hardware must not depend on Linux software.
- Do not proceed to fabrication from the high-level Markdown alone. The first
  checked artifact is `demos/smart_robot/circuitci/motion_core/project.yaml`,
  which passes static power and IO-voltage validation.

## Immediate Board Partition

1. Motion core board:
   - LicheeRV connector
   - AT32F435
   - ICM-42688-P
   - CAN transceiver
   - RS485 transceiver
   - debug and PMU connectors
2. Left wheel actuator board:
   - AT32M416
   - gate driver and MOSFET bridge
   - current sense and encoder/Hall
   - CAN input and local fault/enable
3. Right wheel actuator board:
   - same schematic as left wheel board, with address/side strapping
4. PMU board:
   - USB-C PD input
   - BQ25798 charger/power path
   - protected battery-pack interface
   - e-stop and switched motor/servo rails

## Next Validation Slice

The next highest-risk electrical slice is the PMU board, because wrong battery,
charger, e-stop, or switched-rail behavior can damage hardware. The PMU slice
should encode:

- BQ25798 input, system, and battery rails.
- Protected 2S pack assumption for first fabrication.
- `5V_SYS`, `3V3_LOGIC`, `VBAT_SW`, and `VSERVO` rail budgets.
- Charging interlock that disables motor/servo power unless an explicit debug
  override is modeled.
- INA226 or equivalent current monitor on motor rail.

## PMU Slice Status

`demos/smart_robot/circuitci/pmu/project.yaml` now models the first PMU pass:

- 20 V USB-C PD input budget to BQ25798.
- Protected 2S battery pack assumption.
- 2 A configured charge current, checked against the BQ25798 5 A class and the
  declared 20 V input-source current budget.
- TPS54331 5 V system buck candidate.
- TPS62162 3.3 V logic buck with explicit 2.2 uH inductor, 10 uF input
  capacitor, and 22 uF output capacitor.
- Servo and wheel switched rails behind explicit e-stop policy placeholders.

The placeholder e-stop switches are not fabrication-ready components. Before
layout, replace them with source-backed high-current eFuse/load-switch or
MOSFET-driver models and validate current limit, thermal, inrush, reverse
current, and connector ratings.

## Wheel Actuator Slice Status

`demos/smart_robot/circuitci/wheel_actuator/project.yaml` now models the first
left/right reusable wheel controller pass:

- AT32M416 motor-control MCU.
- DRV8323 three-phase smart gate driver.
- Six MCU PWM outputs for a standard 3-phase BLDC bridge (`UH/UL`, `VH/VL`,
  `WH/WL`).
- DRV8323 SPI and nFAULT interface.
- 3.3 V encoder/Hall signal compatibility.
- Generic CAN transceiver MCU-side compatibility.
- Preliminary 3x CSD88599Q5DC half-bridge power-stage candidate on the PMU
  switched wheel rail.
- First-pass motor bridge budget: 10 A phase peak, 6 A phase RMS, 6 A
  regeneration, 5 mohm / 1 W phase shunts with 2x power margin, 8 A motor
  connector rating, 10 ohm gate resistors, 200 ns dead time, and 20 kHz PWM.

Do not change the first fabrication to a true 6-phase motor/inverter unless
there is a sourced motor requirement. True 6-phase can reduce torque ripple or
add redundancy, but it doubles the inverter, current sensing, firmware timing,
layout, and validation burden. For balancing robots, the better first target is
standard 3-phase FOC with encoder/Hall feedback and enough control-loop rate.

Before layout, treat CSD88599Q5DC as a sourced preliminary bridge candidate
with a checked static budget, not final power-stage sign-off. The next
validation slice must replace the policy current envelope with selected motor
evidence, then add gate-charge/switching-loss checks, MOSFET SOA, thermal
paths, regeneration clamp behavior, and PCB copper-temperature validation.
