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
- `MODEL_QUALITY_REQUIRED` fabrication gate for `U_SERVO_SW` and `U_WHEEL_SW`,
  so the PMU cannot be signed off while the high-current switched-rail parts
  are still low-confidence design-policy placeholders.
- `POWER_SWITCH_BUDGET_VALID` gates for those two switched rails. They
  intentionally fail closed on missing selected switch current-limit,
  on-resistance, thermal-resistance, and junction-temperature evidence.
- `POWER_SWITCH_REVERSE_CURRENT_VALID` and `POWER_SWITCH_INRUSH_VALID` gates,
  which intentionally fail closed until selected switches declare backfeed
  blocking, soft-start/inrush current, and switched-capacitance evidence.

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
- `MODEL_QUALITY_REQUIRED` is also declared for `M1` and `REGEN1`, so the wheel
  actuator is intentionally blocked from fabrication sign-off while those
  critical load/absorber models remain generic placeholders.
- `MOTOR_BRIDGE_LOSS_THERMAL_VALID` first-pass CSD88599Q5DC bridge screening:
  12.6 V max bus, 40 A current class, and scaled 3 W at 30 A reference-loss
  evidence against a 2 W board thermal budget with 2x margin.
- `MOTOR_BRIDGE_SWITCHING_VALID` first-pass CSD88599Q5DC switching screening:
  56 nC max total gate charge at 10 V, 20 ns rise, 3 ns fall, 20 kHz PWM,
  six switching/gate-charge events per PWM cycle, 0.5 W switching-loss budget
  with 2x margin, and 20 mA average gate-drive charge-current budget.
- `MOTOR_BRIDGE_SOA_VALID` is declared for the CSD88599Q5DC bridge and
  checks the TI Figure 4-3 system SOA curve at 115 C board temperature, 10 A
  phase-peak current, and 2x current margin. The curve is a typical
  power-block current/temperature boundary, not final measured waveform or
  transient thermal proof.
- `MOTOR_REGEN_CLAMP_VALID` first-pass regeneration absorber screening for
  `REGEN1`: 1 J single-event energy envelope, 1 mF wheel-bus capacitance,
  12.6 V nominal-to-16 V clamp window, 10 A clamp current envelope, 1.5 J clamp
  energy envelope, and 1.5x current/energy margins.
- `REGEN1` intentionally remains a generic low-confidence design envelope.
  Validation reports must keep emitting a non-blocking `LOW_CONFIDENCE_MODEL`
  limitation for `component:REGEN1:model:demo.smart_robot.regen_clamp_design_envelope`
  until a selected brake resistor, active clamp, TVS, eFuse, or upstream
  energy sink replaces it.
- `MOTOR_ROUTE_CURRENT_VALID` first-pass route-width policies for the
  1.2 mm phase routes and 1.5 mm switched-battery route. These are explicit
  A/mm layout-policy checks, not copper-temperature or SOA proof.
- `MOTOR_CURRENT_SENSE_PLACEMENT_VALID` first-pass shunt and current-sense
  route placement checks for the three phase shunts near the bridge and phase
  copper.
- `MOTOR_CURRENT_SENSE_ACCURACY_VALID` first-pass static measurement-chain
  screen: 5 mohm shunt, 1% shunt tolerance, 20 V/V gain, 0.5% gain error,
  100 uV input offset, 3.3 V 12-bit ADC reference, 3.0 V usable ADC input
  range, at least 20 ADC counts at 0.5 A, and 0.25 A maximum worst-case static
  current error.
- JST VH 8-pin actuator-bus connector screening for the switched wheel rail
  with 1.5x current margin.
- `LOAD_CABLE_CURRENT_VALID` is declared for the actuator-bus harness and is
  intentionally missing cable evidence until a selected wire/crimp/cable
  assembly rating is sourced.
- `LOAD_CABLE_THERMAL_DERATING_VALID` is also declared for actuator-bus
  harness temperature rise and intentionally fails closed until selected
  cable temperature-rise evidence is sourced.
- `LOAD_CABLE_VOLTAGE_DROP_VALID` is declared for actuator-bus harness loop
  resistance, voltage drop, and power loss, and intentionally fails closed
  until selected harness resistance evidence is sourced.

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
CircuitCI smart_robot_pmu_v0: fail (critical=8, warning=0, info=0)
CircuitCI smart_robot_wheel_actuator_v0: fail (critical=5, warning=0, info=0)
CircuitCI smart_robot_servo_payload_v0: pass (critical=0, warning=0, info=0)
```

The PMU failure is expected until `U_SERVO_SW` and `U_WHEEL_SW` are replaced
by source-backed selected high-current eFuse, load-switch, or MOSFET-driver
evidence with current-limit, static thermal, reverse-current, soft-start, and
switched-capacitance metadata. The lower level BQ25798, TPS54331, TPS62162,
and power-tree checks should still remain clean.

The wheel actuator failure is expected until `M1`, `REGEN1`, and the
actuator-bus cable assembly are replaced by source-backed selected components,
ratings, temperature-rise data, loop-resistance data, or measured
load/absorber/harness evidence. The lower level bridge, CAN, connector, route,
current-sense, SOA, switching, and regen budget checks should still remain
clean.

## What This Does Not Yet Prove

- Exact AT32F435 package pin assignment.
- Exact LicheeRV Nano header pin numbers and mechanical footprint.
- CAN/RS485 cable length, common-mode range, connector pinout,
  imported-final-layout route evidence, surge-energy policy, and EMC behavior.
- High-current servo/wheel e-stop switch part selection, inrush, thermal,
  reverse-current behavior, connector heating, and battery safety. The current
  PMU slice blocks sign-off on selected switch evidence and static switch
  current/thermal/reverse-current/inrush metadata, but does not yet validate
  selected switch SOA, current-limit transient waveform, upstream rail droop,
  or PCB copper temperature.
- Selected wheel motor datasheet/measurement evidence, true sourced bridge SOA
  curves, measured switching waveforms, transient thermal
  impedance, selected regeneration clamp part/repeated-pulse behavior, cable
  assembly current/thermal/drop evidence, and final routed layout copper beyond the first-pass
  bridge-loss, switching, regen-envelope, route-width, shunt-placement, and
  current-sense static-accuracy checks.
- Selected servo model, stall current, regeneration, position feedback,
  connector heating, cable assembly quality, and balance-critical actuator
  control.
- True 6-phase motor/inverter control. The current wheel slice is 3-phase BLDC
  with six PWM gate-control signals.

Those are separate slices and should each get their own CircuitCI project or
scenario before schematic capture is treated as fabrication-ready.
