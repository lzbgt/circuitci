# Smart Robot CircuitCI Design Package

This folder contains machine-checkable logical schematics for the reusable smart
robot board stack before committing to KiCad or JLC EDA Pro schematic/PCB CAD.
The first KiCad connectivity skeletons for all current board slices are tracked
under `../kicad/` and import back through CircuitCI, but this folder remains
the validation source of truth until CAD import round-trips all power-domain,
scenario, and routed-layout evidence.
`../kicad/wheel_actuator/wheel_actuator.kicad_pcb` is the first PCB bridge for
layout evidence import; it is still a compact evidence fixture, not fabrication
layout sign-off.

## Current Slices

`motion_core/project.yaml` models the first reusable motion-core board:

- LicheeRV Nano WiFi module as a purchased brain module.
- AT32F435 motion-control MCU.
- ICM-42688-P IMU over SPI plus interrupt.
- TI TCAN3413 3.3 V CAN transceiver rail and MCU-side interface.
- TI ESD2CAN24-Q1 static CANH/CANL clamp presence and ground-reference review.
- TI THVD1450 3.3 V RS485 smart-servo transceiver rail and MCU-side
  interface.
- TI ESDS552 static RS485 A/B clamp presence and ground-reference review.
- Explicit 120 ohm endpoint termination resistor checks for CAN and RS485.
- First-pass CAN/RS485 TVS and termination route-placement contracts from
  explicit layout evidence.
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
- TI TCAN3413 3.3 V CAN command-link rail and MCU-side compatibility.
- TI ESD2CAN24-Q1 static CANH/CANL clamp presence and ground-reference review.
- Explicit 120 ohm endpoint termination resistor check for the CAN endpoint
  population option.
- First-pass CAN TVS and termination route-placement contracts from explicit
  layout evidence.
- Preliminary 3x CSD88599Q5DC half-bridge wheel power-stage budget.
- `M1` motor-load design envelope feeding the bridge budget: 10 A phase peak,
  6 A phase RMS, 6 A regeneration, 5 mohm / 1 W phase shunts,
  8 A motor connector rating, 10 ohm gate resistors, 200 ns dead time, and
  20 kHz PWM.
- `M1` intentionally remains a generic low-confidence model. Validation reports
  must keep emitting a non-blocking `LOW_CONFIDENCE_MODEL` limitation for
  `component:M1:model:demo.smart_robot.wheel_motor_design_envelope` until a
  selected motor datasheet or measured load envelope replaces it.
- `MOTOR_BRIDGE_LOSS_THERMAL_VALID` first-pass CSD88599Q5DC bridge screening:
  12.6 V max bus, 40 A current class, and scaled 3 W at 30 A reference-loss
  evidence against a 2 W board thermal budget with 2x margin.
- `MOTOR_ROUTE_CURRENT_VALID` first-pass route-width policies for the
  1.2 mm phase routes and 1.5 mm switched-battery route. These are explicit
  A/mm layout-policy checks, not copper-temperature or SOA proof.
- `MOTOR_CURRENT_SENSE_PLACEMENT_VALID` first-pass shunt and current-sense
  route placement checks for the three phase shunts near the bridge and phase
  copper.
- JST VH 8-pin actuator-bus connector screening for the switched wheel rail
  with 1.5x current margin.

`servo_payload/project.yaml` models the reusable low-load servo/payload hub:

- AT32F435 I2C2 host pins.
- NXP PCA9685 16-channel, 12-bit I2C PWM driver at 3.3 V.
- Four 6.0 V to 8.4 V PWM-servo design-load channels on the separate
  `VSERVO` rail.
- Static 3.3 V I2C/PWM logic-level compatibility.
- Static `VSERVO` budget of four 1 A low-load servo envelopes.
- JST XH-style 3-pin servo connector current screening with 1.5x current
  margin against each low-load servo envelope.

Validation command:

```sh
cargo run -- validate demos/smart_robot/circuitci/motion_core/project.yaml \
  --output out/smart_robot_motion_core_validate
cargo run -- validate demos/smart_robot/circuitci/pmu/project.yaml \
  --output out/smart_robot_pmu_validate
cargo run -- validate demos/smart_robot/circuitci/wheel_actuator/project.yaml \
  --output out/smart_robot_wheel_actuator_validate
cargo run -- validate demos/smart_robot/circuitci/servo_payload/project.yaml \
  --output out/smart_robot_servo_payload_validate
```

Expected result:

```text
CircuitCI smart_robot_motion_core_v0: pass (critical=0, warning=0, info=0)
CircuitCI smart_robot_pmu_v0: pass (critical=0, warning=0, info=0)
CircuitCI smart_robot_wheel_actuator_v0: pass (critical=0, warning=0, info=0)
CircuitCI smart_robot_servo_payload_v0: pass (critical=0, warning=0, info=0)
```

## What This Does Not Yet Prove

- Exact AT32F435 package pin assignment.
- Exact LicheeRV Nano header pin numbers and mechanical footprint.
- CAN/RS485 cable length, common-mode range, connector pinout,
  imported-final-layout route evidence, surge-energy policy, and EMC behavior.
- High-current servo/wheel e-stop switch part selection, inrush, thermal,
  reverse-current behavior, connector heating, and battery safety.
- Selected wheel motor datasheet/measurement evidence, current-sense electrical
  accuracy, true MOSFET SOA, switching transition loss, transient thermal
  impedance, regeneration clamp energy, cable assembly evidence, and final
  routed layout copper beyond the first-pass bridge-loss, route-width, and
  shunt-placement checks.
- Selected servo model, stall current, regeneration, position feedback,
  connector heating, cable assembly quality, and balance-critical actuator
  control.
- True 6-phase motor/inverter control. The current wheel slice is 3-phase BLDC
  with six PWM gate-control signals.

Those are separate slices and should each get their own CircuitCI project or
scenario before schematic capture is treated as fabrication-ready.
