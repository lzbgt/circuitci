# Smart Robot CircuitCI Design Package

This folder contains machine-checkable logical schematics for the reusable smart
robot board stack before committing to KiCad or JLC EDA Pro schematic/PCB CAD.

## Current Slice

`motion_core/project.yaml` models the first reusable motion-core board:

- LicheeRV Nano WiFi module as a purchased brain module.
- AT32F435 motion-control MCU.
- ICM-42688-P IMU over SPI plus interrupt.
- MCU-side CAN transceiver interface.
- MCU-side RS485 smart-servo interface.
- Static rail budget and 3.3 V logic-level compatibility.

Validation command:

```sh
cargo run -- validate demos/smart_robot/circuitci/motion_core/project.yaml \
  --output out/smart_robot_motion_core_validate
```

Expected result:

```text
CircuitCI smart_robot_motion_core_v0: pass (critical=0, warning=0, info=0)
```

## What This Does Not Yet Prove

- Exact AT32F435 package pin assignment.
- Exact LicheeRV Nano header pin numbers and mechanical footprint.
- CAN/RS485 transceiver part choice, termination, common-mode range, and ESD.
- PMU rails, charging interlock, battery safety, and e-stop hardware.
- Wheel actuator BLDC gate-driver current sense, MOSFET SOA, thermal, and
  layout.

Those are separate slices and should each get their own CircuitCI project or
scenario before schematic capture is treated as fabrication-ready.
