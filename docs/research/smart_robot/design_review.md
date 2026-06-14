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
