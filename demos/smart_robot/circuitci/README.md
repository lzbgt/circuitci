# Smart Robot CircuitCI Design Package

This folder contains machine-checkable logical schematics for the reusable smart
robot board stack before committing to KiCad or JLC EDA Pro schematic/PCB CAD.

## Current Slices

`motion_core/project.yaml` models the first reusable motion-core board:

- LicheeRV Nano WiFi module as a purchased brain module.
- AT32F435 motion-control MCU.
- ICM-42688-P IMU over SPI plus interrupt.
- MCU-side CAN transceiver interface.
- MCU-side RS485 smart-servo interface.
- Static rail budget and 3.3 V logic-level compatibility.

`pmu/project.yaml` models the first reusable PMU board:

- BQ25798 charger input/battery current-budget screening.
- TPS54331 5 V buck static voltage/current screening.
- TPS62162 3.3 V buck support inductor/capacitor screening.
- Design-policy e-stop switch placeholders for servo and wheel rails.

`wheel_actuator/project.yaml` models the reusable left/right wheel actuator
board:

- AT32M416 motor-control MCU power and logic-level screening.
- DRV8323 three-phase smart gate-driver interface.
- Six independent PWM lines for a normal 3-phase BLDC bridge.
- 3.3 V encoder/Hall input compatibility.
- CAN command-link MCU-side compatibility.
- Wheel power-stage load-budget placeholder.

Validation command:

```sh
cargo run -- validate demos/smart_robot/circuitci/motion_core/project.yaml \
  --output out/smart_robot_motion_core_validate
cargo run -- validate demos/smart_robot/circuitci/pmu/project.yaml \
  --output out/smart_robot_pmu_validate
cargo run -- validate demos/smart_robot/circuitci/wheel_actuator/project.yaml \
  --output out/smart_robot_wheel_actuator_validate
```

Expected result:

```text
CircuitCI smart_robot_motion_core_v0: pass (critical=0, warning=0, info=0)
CircuitCI smart_robot_pmu_v0: pass (critical=0, warning=0, info=0)
CircuitCI smart_robot_wheel_actuator_v0: pass (critical=0, warning=0, info=0)
```

## What This Does Not Yet Prove

- Exact AT32F435 package pin assignment.
- Exact LicheeRV Nano header pin numbers and mechanical footprint.
- CAN/RS485 transceiver part choice, termination, common-mode range, and ESD.
- High-current servo/wheel e-stop switch part selection, inrush, thermal,
  reverse-current behavior, connector heating, and battery safety.
- Wheel actuator MOSFET selection, shunt values, gate-charge/dead-time,
  current-sense accuracy, MOSFET SOA, thermal, and layout.
- True 6-phase motor/inverter control. The current wheel slice is 3-phase BLDC
  with six PWM gate-control signals.

Those are separate slices and should each get their own CircuitCI project or
scenario before schematic capture is treated as fabrication-ready.
