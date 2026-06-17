# Smart Robot Source Review

Date: 2026-06-14

This note records the source-backed facts used for the first reusable smart
robot control-stack design pass.

## Saved Sources

| Source | Local file | SHA-256 |
| --- | --- | --- |
| Sipeed LicheeRV Nano intro page | `docs/research/smart_robot/sources/licheerv_nano_intro.html` | `7b8a90dbe05c8f03c0b9036ad9204ef851fcdd4256b982870592b00e2bccecf3` |
| Sipeed LicheeRV Nano 70405 schematic | `docs/research/smart_robot/sources/licheerv_nano_70405_schematic.pdf` | `b09ec99069e7f696498b3501785f5296fd0ecaed6d1895d16de2c2e057c2fd19` |
| Artery AT32F435 product page | `docs/research/smart_robot/sources/at32f435_product.html` | `ff88d97074371dcfa1f677d6df7422dee2158488d81fea6deac9399117921bd7` |
| Artery AT32M416 product page | `docs/research/smart_robot/sources/at32m416_product.html` | `1d100588bde163e80f3d6b715b1e76ff6e8c8717e1582510b41169b1b17967ce` |
| TDK ICM-42688-P datasheet | `docs/research/smart_robot/sources/icm42688p_datasheet.pdf` | `9663bf7d68e1ecc67486f452c9c62f0b85e1c22c569845ea7f66b4d91fee04a1` |
| TI BQ25798 product page | `docs/research/smart_robot/sources/bq25798_product.html` | `ef6f404d190782132bf993cc98e4476dddb6f5711f53502a1d0ef59dfe3268be` |
| TI BQ25798 datasheet | `docs/research/smart_robot/sources/bq25798_datasheet.pdf` | `631d541679512116b5cf7ade0516000e78bc95f060b8b958e726c1b6eb74c27f` |
| TI TPS54331 datasheet | `docs/research/smart_robot/sources/tps54331_datasheet.pdf` | `3867bc82cb0f8e3e898de7c0d220d01cd2a30d0281178d17d10eeb5b866439a3` |
| TI DRV8323 product page | `docs/research/smart_robot/sources/drv8323_product.html` | `58097fe705d14a8c40b82d6404a500c2db99e6d1a1330da6191fdbc929c8feed` |
| TI DRV8323 datasheet | `docs/research/smart_robot/sources/drv8323_datasheet.pdf` | `dd8386c972e8d0a57278e432da6e9d8b2bc2f73768dd97eac69594da20d24208` |
| TI CSD88599Q5DC product page | `docs/research/smart_robot/sources/csd88599q5dc_product.html` | `7b0c6ddf956afd16edf5980d99069782a5ed7517053a7ba063defa8427b02aab` |
| TI CSD88599Q5DC datasheet | `docs/research/smart_robot/sources/csd88599q5dc_datasheet.pdf` | `193fc1b1e214064fef48e7fe73a3394c30b13ef07977cbb8663d36e9bfbbdd65` |
| TI TCAN3413 product page | `docs/research/smart_robot/sources/tcan3413_product.html` | `500dcecf530224b361cbd2ebeb3c5051a6ceeca03221e09b4eb8ed99c19f0064` |
| TI TCAN3413 datasheet | `docs/research/smart_robot/sources/tcan3413_datasheet.pdf` | `2c0e8963e7762bc91edf30a365cc10c31ce88e2ddda3b67a120c4158ae52930b` |
| TI ESD2CAN24-Q1 datasheet | `docs/research/smart_robot/sources/ti_esd2can24_q1_datasheet.pdf` | `305b0aafdec918e96476fdf7a385cd2143e8b7664383842160c3ef1522d2bc5e` |
| TI CAN ESD / overvoltage application note | `docs/research/smart_robot/sources/ti_can_esd_overvoltage_app_note.pdf` | `207ed023ca70af6e1cb54c356e799d588ea9d4754f66875d09cfdc76f57eb364` |
| TI THVD1450 product page | `docs/research/smart_robot/sources/thvd1450_product.html` | `5dbdd9efd4169c8ea8ac78dd4879bf40aec7e68232388a4887031570c05af132` |
| TI THVD1450 datasheet | `docs/research/smart_robot/sources/thvd1450_datasheet.pdf` | `c8d27b57c6cd2018d5a38d65fcc030f7cd47f2221232e26b05dc8671095693ca` |
| TI ESDS552 product page | `docs/research/smart_robot/sources/esds552_product.html` | `7757b8460f208a6f0e7c2fb05e19bf62e09ef686f2ba540f6f3e8eef33bf0e23` |
| TI ESDS552 datasheet | `docs/research/smart_robot/sources/esds552_datasheet.pdf` | `3ce62d5dbb2b1637cb8591437db132fd4d1771059051ee0ab58aa2d0332fc936` |
| TI RS-485 Design Guide | `docs/research/smart_robot/sources/ti_rs485_design_guide.pdf` | `b613117a93a95ff14444710f7d21aa67f70a3e77f6ddd8d4fcda8666477417db` |
| TI CAN selectable termination reference design | `docs/research/smart_robot/sources/ti_can_selectable_termination_ref_design.pdf` | `07604fc7e33c90edd8d03eb07641d8893f6319e9d999b1985329f7552c48c21e` |
| NXP PCA9685 product page | `docs/research/smart_robot/sources/pca9685_product.html` | `28cbfe16e1a9b64c21ee3dec97f01f1277aa08013b6d67e11084a08536804468` |
| NXP PCA9685 datasheet | `docs/research/smart_robot/sources/pca9685_datasheet.pdf` | `237d47f339cac4c3a0d56a5f0b4d3c93df71e3eb43f36ac57ea4ff38e6b2e585` |
| JST XH connector datasheet | `docs/research/smart_robot/sources/jst_xh_connector_datasheet.pdf` | `9426b136902f11900825077535e5c65032b7fbc31ffb59c5e9e1f463bb20fb90` |
| JST VH connector datasheet | `docs/research/smart_robot/sources/jst_vh_connector_datasheet.pdf` | `d51e669c597988b20c0963daf5bef7356cbd2104c1f867e9107c6fa6cd2b899c` |
| National Wire UL AWM 1015 datasheet | `docs/research/smart_robot/sources/national_wire_ul_awm_1015.pdf` | `8b98c03ef1dac8985a36ca275badda37587a2d8d674c0a5725fbbb698fa635ea` |
| Vishay Dale RH/NH aluminum-housed resistor datasheet | `docs/research/smart_robot/sources/vishay_rh_nh_aluminum_housed_resistors.pdf` | `b4741aa9ec7437a150fd8aaea5218dbbf3cde96428fa358712e5c7c0b25bb500` |

## Confirmed Facts

- Sipeed describes LicheeRV Nano as a `22.86 mm x 35.56 mm` development board
  with SG2002, 256 MB DDR3, USB, SPI, UART, I2C, CSI/DSI, and half-hole /
  through-hole production-friendly mounting.
- The Sipeed peripheral page identifies UART0 on `A16 (TX)` and `A17 (RX)`.
  That supports keeping the LicheeRV interface minimal: UART plus motion enable
  and fault interrupt.
- Artery's AT32F435 product page confirms the 288 MHz Cortex-M4F class,
  multiple UART/CAN/SPI/I2C resources, and industrial-temperature positioning.
- Artery's AT32M416 product page confirms the motor-control positioning:
  180 MHz Cortex-M4F, CAN-FD, high-speed ADCs, comparators, op-amps/PGA, and an
  advanced PWM timer.
- TDK's ICM-42688-P datasheet confirms SPI/I2C/I3C host interface, FIFO, and
  interrupt support for the motion-core IMU role.
- TI's BQ25798 datasheet confirms 1-4 cell buck-boost charger support and 5 A
  charge-current class for the PMU-board direction.
- TI's BQ25798 product page describes an I2C controlled 1-4-cell, 5-A
  buck-boost charger with dual-input selector, MPPT, integrated switching
  MOSFETs, BATFET, and NVDC power-path behavior.
- TI's TPS54331 datasheet identifies a 3.5 V to 28 V input, 3 A step-down
  converter. That makes it a reasonable first 2S-pack-to-5V_SYS candidate for
  the reusable PMU, pending detailed compensation, thermal, and layout review.
- TI's DRV8323 product page and datasheet identify a 6 V to 60 V three-phase
  smart gate-driver family with current shunt amplifier support and nFAULT/SPI
  diagnostics. That fits the first wheel-board direction better than a true
  6-phase inverter because it keeps the first fabrication to one 3-phase bridge
  while still allowing six independent PWM gate-control signals.
- TI's CSD88599Q5DC product page identifies a 60 V, 40 A N-channel half-bridge
  NexFET power block in a 5 mm x 6 mm Dual-Cool package. TI also lists
  BOOSTXL-DRV8323RH/RS as 15 A, 3-phase BLDC drive stages based on DRV8323 and
  CSD88599Q5DC power blocks. That makes three CSD88599Q5DC devices a sourced
  preliminary bridge candidate for the 2S wheel actuator board, not a final SOA
  or thermal sign-off.
- TI's TCAN3413 product page identifies a 3.3 V CAN FD transceiver with
  separate VIO support from 1.7 V to 3.6 V, VCC operation from 3.0 V to 3.6 V,
  ISO 11898-2:2016 compliance, standby mode, and bus fault protection up to
  plus/minus 58 V. That makes it a sourced first CAN transceiver for the 3.3 V
  motion-core and wheel-actuator control buses.
- TI's ESD2CAN24-Q1 datasheet identifies an automotive 24 V, two-channel ESD
  protection diode for in-vehicle network lines including CANH/CANL. The first
  CircuitCI model uses that source to check CANH/CANL clamp presence and ground
  reference on the motion-core and wheel-actuator CAN ports. This is not an
  ISO transient, placement, stub-length, or signal-integrity sign-off.
- TI's THVD1450 product page identifies a 3.3 V to 5 V RS485/RS422
  transceiver with 50 Mbps signaling, one-eighth-unit-load bus loading,
  up to 256 bus nodes, and plus/minus 18 kV IEC ESD positioning. That makes it
  a sourced first RS485 transceiver for the motion-core smart-servo bus.
- TI's ESDS552 product page identifies a 12 V, two-channel bidirectional ESD
  and surge protection diode for RS-485 and RS-422. The first CircuitCI model
  uses that source to check RS485 A/B clamp presence and ground reference on
  the motion-core smart-servo port. This is not IEC surge/ESD pulse,
  termination, placement, common-mode, or signal-integrity sign-off.
- TI's RS-485 and CAN termination source material supports using explicit
  endpoint topology before validating 120 ohm line termination. CircuitCI now
  checks the smart-robot endpoint-population variants by requiring a declared
  resistor across the exact bus nets and a declared tolerance. The demos also
  add explicit `board.layout.placements` and ordered route evidence to check
  first-pass TVS and termination route distance. This is not a replacement for
  imported final CAD, surge-energy, EMC, common-mode, or signal-integrity
  validation.
- NXP's PCA9685 page and datasheet identify a 16-channel, 12-bit PWM
  Fast-mode Plus I2C LED controller. The saved datasheet states 2.3 V to 5.5 V
  supply operation, 5.5 V tolerant inputs/outputs, Fm+ operation up to 1 MHz,
  and totem-pole output capability of 25 mA sink / 10 mA source at 5 V. That is
  enough for static 3.3 V servo-signal screening, but not enough to sign off
  servo stall, regeneration, or balance-critical actuator feedback.
- The saved JST XH connector datasheet is used as the source for the first
  low-load servo connector model. The CircuitCI model records a 3.0 A current
  rating and 250 V voltage rating for static screening. Treat that as connector
  budget evidence only; selected wire gauge, crimp, cable assembly, vibration,
  and temperature rise still need separate evidence.
- The saved JST VH connector datasheet is used as the source for the first
  wheel-actuator power/control connector model. The CircuitCI model records a
  10.0 A current rating and 250 V voltage rating for static screening of the
  switched wheel rail. Treat this as a connector budget check only; cable
  assembly, wire gauge, crimp pullout, vibration, temperature rise, CAN
  termination, and surge/ESD protection remain separate checks.
- The wheel actuator now uses `JACT1_CABLE`, a selected first-pass JST
  VH/AWG16, 0.5 m actuator-bus harness model. Its current rating comes from the
  JST VH AWG16 specification, and its loop resistance comes from the National
  Wire UL AWM 1015 AWG16 N600-2630U table. This clears the cable-current and
  cable-voltage-drop gates for the current 6 A load and 1.5x margin, but it is
  still not final harness sign-off.
- The wheel actuator also has a separate `LOAD_CABLE_THERMAL_DERATING_VALID`
  gate for actuator-bus harness temperature-rise evidence. It intentionally
  reports `VALIDATION_INPUT_MISSING` until the project selects a cable assembly
  with test-current/rise evidence or measures the final harness.
- The wheel actuator also has a separate `LOAD_CABLE_VOLTAGE_DROP_VALID` gate
  for actuator-bus harness loop-resistance evidence. The selected first-pass
  harness now clears this gate with 0.01506 ohm loop resistance for a 0.5 m
  two-wire AWG16 path.

## Design Review Notes

- The original high-level document is directionally sound: keep the PMU,
  motion core, and wheel actuator boards separate for reuse and safety.
- Do not draw the final KiCad/JLC schematic until each board has a passing
  CircuitCI logical schematic. This catches rail and I/O mistakes before CAD
  symbol/footprint work. The first KiCad skeletons for all current smart-robot
  board slices are now tracked under `demos/smart_robot/kicad/` and import back
  through CircuitCI as connectivity artifacts.
- The first PCB-layout evidence bridge is
  `demos/smart_robot/kicad/wheel_actuator/wheel_actuator.kicad_pcb`. It is a
  compact import fixture for placement, pad, route, via, zone, outline, and
  net-rule evidence, not a fabrication-ready motor-drive layout. It now
  preserves compact ordered CANH/CANL route evidence so the wheel actuator CAN
  TVS and endpoint-termination placement scenarios can be overlaid onto the
  imported CAD project and validated without manual layout metadata. It also
  preserves the phase and switched-battery route widths used by
  `MOTOR_ROUTE_CURRENT_VALID` for a first-pass route-current screen, plus
  phase-shunt placement and current-sense routes used by
  `MOTOR_CURRENT_SENSE_PLACEMENT_VALID`.
- The first verifiable slice is
  `demos/smart_robot/circuitci/motion_core/project.yaml`. It covers the
  LicheeRV-to-AT32F435 UART/enable/fault link, AT32F435 rail budget,
  ICM-42688-P SPI/interrupt interface, and MCU-side CAN/RS485 logic levels.
- The CAN and RS485 transceiver placeholders have been replaced with sourced TI
  TCAN3413 and THVD1450 models. The motion-core CAN port also includes a sourced
  ESD2CAN24-Q1 clamp review on CANH/CANL, and the RS485 port includes a sourced
  ESDS552 clamp review on A/B. These verify rail, MCU-side I/O, and static bus
  clamp presence/reference. The first endpoint-population scenarios also check
  120 ohm CAN/RS485 termination components within explicit 5% tolerance.
  First-pass route-placement contracts now check selected TVS and termination
  components against explicit layout placements/routes. The wheel actuator PCB
  bridge proves that path with imported KiCad PCB evidence. The same PCB bridge
  now proves first-pass motor route-width checks for phase and switched-battery
  routes, and first-pass phase-shunt/current-sense placement checks. Cable
  length, connector pinout, EMC, copper temperature rise, current-sense
  accuracy, and final routed layout still require board-level evidence.
- `M1` now carries a 6.0 V to 12.6 V generic supply envelope in addition to
  the first-pass current envelope. `MOTOR_LOAD_SUPPLY_VALID` checks that the
  declared wheel bus range fits the selected motor model, but this remains
  placeholder evidence until a motor datasheet or measured envelope replaces
  `demo.smart_robot.wheel_motor_design_envelope`.
- The first PMU validation slice is
  `demos/smart_robot/circuitci/pmu/project.yaml`. It verifies BQ25798 input and
  charge-current budget, TPS54331 5 V output budget, TPS62162 3.3 V support
  parts, and e-stop rail gating policy. `U_SERVO_SW` now uses a source-backed
  configured TI TPS25948 8 A eFuse model with always-on reverse-current
  blocking mode and dVdt/inrush evidence. `U_WHEEL_SW` now uses a
  source-backed TPS24751/CSD17501Q5A reverse-blocking switch path with
  current-limit, thermal, off-state reverse-current isolation, and first-pass
  inrush evidence. The TPS25985 datasheet was cached as a wheel candidate, but
  it is not selected because the current review did not prove the required
  off-state reverse-current isolation mode.
- The first wheel-actuator validation slice is
  `demos/smart_robot/circuitci/wheel_actuator/project.yaml`. It verifies the
  AT32M416-to-DRV8323 six-PWM interface, SPI/fault pins, 3.3 V encoder/Hall
  inputs, TCAN3413 CAN transceiver rail and MCU-side logic, ESD2CAN24-Q1
  CANH/CANL clamp presence/reference, explicit 120 ohm CAN endpoint termination
  evidence for the endpoint-population option, CAN TVS/termination
  route-placement contracts, rail budgets, and a preliminary 3x CSD88599Q5DC
  wheel bridge candidate.
- The wheel actuator now also checks `M1`, a modeled first-pass motor-load
  design envelope: 10 A phase peak, 6 A phase RMS, 6 A regeneration,
  5 mohm / 1 W phase shunts, 8 A motor connector rating, 10 ohm gate
  resistors, 200 ns dead time, and 20 kHz PWM. These values are design-policy
  inputs for a reusable small robot actuator, not sourced motor
  characterization. Wheel validation reports must retain the non-blocking
  `LOW_CONFIDENCE_MODEL` limitation for
  `component:M1:model:demo.smart_robot.wheel_motor_design_envelope` until a
  selected motor datasheet or measured envelope replaces this model.
- The wheel actuator now also declares a blocking `MODEL_QUALITY_REQUIRED`
  fabrication gate for `M1`. The gate requires model source `datasheet` or
  `measured` and at least `medium` confidence, so the current wheel report must
  fail sign-off while the motor remains a generic design envelope.
- The wheel actuator also declares a blocking `LOAD_CABLE_CURRENT_VALID`
  harness-current screen. The selected JST VH/AWG16 0.5 m actuator harness now
  clears the current screen with source-backed 10 A evidence.
- The wheel actuator also declares a blocking
  `LOAD_CABLE_THERMAL_DERATING_VALID` harness-temperature screen. The current
  report must fail until selected actuator-bus cable temperature-rise evidence
  is supplied.
- The wheel actuator also declares a blocking `LOAD_CABLE_VOLTAGE_DROP_VALID`
  harness-drop screen. The selected harness now clears the drop screen using
  0.01506 ohm loop resistance for a 0.5 m two-wire AWG16 path.
- The wheel actuator now also checks the preliminary CSD88599Q5DC bridge model
  with `MOTOR_BRIDGE_LOSS_THERMAL_VALID`: 12.6 V maximum bus, 40 A current
  class, and the retained 3 W at 30 A reference-loss point scaled to the 6 A
  RMS design envelope against an explicit 2 W board thermal budget with 2x
  margin.
- The wheel actuator now also checks the preliminary CSD88599Q5DC bridge model
  with `MOTOR_BRIDGE_SWITCHING_VALID`: cached TI datasheet page 4 exposes
  56 nC maximum total gate charge at 10 V, 20 ns rise time, and 3 ns fall
  time. The current screen uses 12.6 V bus, 10 A phase peak, 20 kHz PWM,
  six switching/gate-charge events per PWM cycle, 0.5 W switching budget with
  2x margin, and 20 mA average gate-drive charge-current budget. This is still
  not final waveform, SOA, peak gate-current, ringing, or measured
  board-temperature sign-off.
- The wheel actuator now declares `MOTOR_BRIDGE_SOA_VALID` for the
  CSD88599Q5DC bridge. The preliminary model encodes TI datasheet Figure 4-3
  as a system SOA curve: output current versus board temperature under the
  datasheet's stated 36 V, 10 V gate-drive, 50% duty-cycle, 20 kHz, 480 uH,
  2 oz, 6-layer test conditions. The wheel screen checks 115 C board
  temperature, 10 A phase-peak current, and 2x margin. This closes the
  previous missing-SOA metadata finding, but it remains a typical static curve
  screen rather than final measured waveform/thermal sign-off.
- The wheel actuator now also checks `REGEN1`, a selected Vishay RH100 1 ohm /
  100 W aluminum-housed resistor, with `MOTOR_REGEN_CLAMP_VALID`: 1 J
  single-event energy, 1 mF wheel-bus capacitance, a 12.6 V nominal-to-16 V
  clamp window, 10 A clamp current envelope, 1.5 J clamp energy envelope, and
  1.5x current/energy margins. The saved Vishay datasheet gives the RH100
  100 W rating and includes 1 ohm values in the supported range. The
  `regen_absorber` model metadata now carries the 10 A / 1.5 J first-pass
  ratings consumed by the validator, so the scenario no longer duplicates those
  selected-part values. The model is source-backed for first-pass absorption
  screening, but not for repeated-pulse resistor thermal sign-off, enclosure
  heat sinking, or firmware regeneration control.
- The wheel actuator now includes a JST VH 8-pin actuator-bus connector model
  and validates the preliminary CSD88599Q5DC bridge load against that connector
  with 1.5x current margin.
- The wheel slice now validates first-pass phase-shunt placement and
  current-sense route distances from explicit layout evidence. It also validates
  a first-pass static current-sense accuracy budget: 5 mohm shunt, 1% shunt
  tolerance, 20 V/V gain, 0.5% gain error, 100 uV input offset, 3.3 V 12-bit
  ADC reference, 3.0 V usable ADC input range, at least 20 ADC counts at 0.5 A,
  and 0.25 A maximum worst-case static current error. It does not yet validate
  true MOSFET SOA, peak gate current, switch-node ringing, selected amplifier
  bandwidth/common-mode behavior, PWM sampling, transient thermal impedance,
  selected regeneration absorber repeated-pulse behavior, PCB copper
  temperature rise, final harness temperature rise, final harness voltage
  transients, or motor-control loop stability.
- The first servo/payload validation slice is
  `demos/smart_robot/circuitci/servo_payload/project.yaml`. It verifies the
  AT32F435 I2C2 host pins, NXP PCA9685 3.3 V power and logic domain, four
  low-load PWM-servo design envelopes on the separate `VSERVO` rail, and static
  3.3 V I2C/PWM compatibility. It also checks four JST XH-style servo
  connector current budgets with 1.5x margin against the modeled 1 A low-load
  servo envelopes.
- The servo/payload slice is for camera/head/light payload servos only. It does
  not prove selected servo stall current, regeneration into `VSERVO`, connector
  heating, cable assembly quality, mechanical torque/speed, or position
  feedback. Balance-critical mass-shift actuation should use RS485 smart servos
  or a local actuator board with feedback.
