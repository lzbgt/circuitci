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

The first KiCad CAD bridges for all current board slices live under
`demos/smart_robot/kicad/`, each with a `circuitci.kicad-map.yaml`. They import
through CircuitCI as schematic connectivity source artifacts, but they are not
yet the validation source of truth: the matching
`demos/smart_robot/circuitci/*/project.yaml` files still carry power-domain,
scenario, and pre-layout route-placement evidence that native KiCad import does
not round-trip yet.

The first PCB-layout bridge is
`demos/smart_robot/kicad/wheel_actuator/wheel_actuator.kicad_pcb`. It imports
component placements, pads, CAN routes, motor power/phase routes, a ground
zone, board outline, and net-class constraints into Board IR. It is an evidence
fixture for the CAD/import loop, not final power-stage layout sign-off.

## Corrections And Constraints

- Use the Sipeed LicheeRV Nano source docs, not memory, for module facts. The
  first motion-core slice only relies on UART0 `A16/A17`, 5 V input, ground,
  and two GPIO-style control lines.
- Treat LicheeRV `3V3` as a reference or IO-domain signal only. Do not power the
  robot motion board from the module.
- Use TCAN3413 for the first 3.3 V CAN control-bus pass, ESD2CAN24-Q1 for the
  first static CANH/CANL clamp review, and THVD1450 for the first 3.3 V RS485
  smart-servo bus pass with ESDS552 for the first static RS485 A/B clamp
  review. Use explicit endpoint-population metadata before installing 120 ohm
  CAN/RS485 termination, and use explicit layout placements/routes before
  validating TVS or termination route distance. Keep cable length, connector
  pinout, surge-energy target, EMC, and imported final routed CAD review as
  open board-level constraints.
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
   - TCAN3413 CAN transceiver
   - ESD2CAN24-Q1 CAN TVS
   - THVD1450 RS485 transceiver
   - ESDS552 RS485 TVS
   - debug and PMU connectors
2. Left wheel actuator board:
   - AT32M416
   - gate driver and MOSFET bridge
   - current sense and encoder/Hall
   - JST VH actuator-bus input with CAN, switched wheel power, local fault,
     enable, and sync
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
- TCAN3413 3.3 V CAN transceiver rail and MCU-side compatibility.
- ESD2CAN24-Q1 CANH/CANL clamp presence and ground-reference review.
- 120 ohm CAN endpoint termination resistor evidence checked within explicit
  5% tolerance for the current endpoint-population variant.
- First-pass CAN TVS and termination route-placement contracts from explicit
  board layout evidence.
- Preliminary 3x CSD88599Q5DC half-bridge power-stage candidate on the PMU
  switched wheel rail.
- JST VH 8-pin actuator-bus connector current and voltage budget for the
  switched wheel rail, checked with 1.5x margin against the preliminary bridge
  load.
- `M1` motor-load design envelope feeding the first-pass bridge budget:
  10 A phase peak, 6 A phase RMS, 6 A regeneration, 5 mohm / 1 W phase shunts
  with 2x power margin, 8 A motor connector rating, 10 ohm gate resistors,
  200 ns dead time, and 20 kHz PWM.
- `M1` is still a generic low-confidence design envelope, not selected motor
  evidence. The wheel validation report is expected to pass while retaining the
  non-blocking `LOW_CONFIDENCE_MODEL` limitation for
  `component:M1:model:demo.smart_robot.wheel_motor_design_envelope`.
- CSD88599Q5DC source-reference bridge loss/thermal budget: 12.6 V maximum
  bus, 40 A current class, and scaled 3 W at 30 A reference-loss evidence
  checked against a 2 W board thermal budget with 2x margin.
- CSD88599Q5DC first-pass switching budget: cached TI datasheet page 4 exposes
  56 nC maximum total gate charge at 10 V, 20 ns rise time, and 3 ns fall
  time. The wheel design checks 12.6 V bus, 10 A phase peak, 20 kHz PWM,
  six switching/gate-charge events per PWM cycle, 0.5 W switching budget with
  2x margin, and 20 mA average gate-drive charge-current budget. This remains a
  static screen, not final waveform or SOA proof.
- `REGEN1` first-pass regeneration absorber envelope: 1 J single-event energy,
  1 mF wheel-bus capacitance, 12.6 V nominal-to-16 V clamp window, 10 A clamp
  current envelope, 1.5 J clamp energy envelope, and 1.5x current/energy
  margins. `REGEN1` is still a generic low-confidence envelope; the validation
  report is expected to retain the non-blocking `LOW_CONFIDENCE_MODEL`
  limitation for
  `component:REGEN1:model:demo.smart_robot.regen_clamp_design_envelope`.
- First-pass phase-shunt/current-sense placement contract from explicit layout
  evidence, keeping the three phase shunts close to the bridge, phase copper,
  and sense traces.
- First-pass current-sense static accuracy contract: 5 mohm shunt, 1% shunt
  tolerance, 20 V/V gain, 0.5% gain error, 100 uV input offset, 3.3 V 12-bit
  ADC reference, 3.0 V usable ADC input range, at least 20 ADC counts at 0.5 A,
  and 0.25 A maximum worst-case static current error.

Do not change the first fabrication to a true 6-phase motor/inverter unless
there is a sourced motor requirement. True 6-phase can reduce torque ripple or
add redundancy, but it doubles the inverter, current sensing, firmware timing,
layout, and validation burden. For balancing robots, the better first target is
standard 3-phase FOC with encoder/Hall feedback and enough control-loop rate.

Before layout, treat CSD88599Q5DC as a sourced preliminary bridge candidate
with checked static, reference-loss, and first-pass switching budgets, not
final power-stage sign-off.
The next validation slice must replace
`demo.smart_robot.wheel_motor_design_envelope` with selected motor datasheet or
measurement evidence, then add true SOA curves, measured switching waveforms,
peak gate-current timing, transient thermal paths, selected regeneration
absorber behavior, repeated-pulse clamp heating, selected current-sense
amplifier/ADC part behavior, PWM sampling/common-mode rejection, imported final
layout evidence, cable assembly evidence, and PCB copper-temperature
validation.

## Servo Payload Slice Status

`demos/smart_robot/circuitci/servo_payload/project.yaml` now models the first
low-load servo/payload hub:

- AT32F435 I2C2 host pins from the motion-core model.
- NXP PCA9685 16-channel, 12-bit I2C PWM driver at 3.3 V.
- Four PWM-servo design-load channels on the separate 6.0 V to 8.4 V `VSERVO`
  rail.
- Static 3.3 V I2C and PWM logic-level compatibility.
- Static `VSERVO` budget of four 1 A low-load servo envelopes.
- Four JST XH-style 3-pin servo connectors checked against the modeled 1 A
  low-load servo envelopes with 1.5x current margin.

This is suitable for camera pitch, screen tilt, head, or light payload motion.
It is not suitable for balance-critical mass-shift control unless selected
servos provide position/current/thermal feedback or the board changes to an
RS485 smart-servo/local actuator architecture. Before layout, choose actual
servo models and cable assemblies, then validate stall current, regeneration,
connector heating, wire gauge/crimp evidence, and `VSERVO` transient behavior.
