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
| NXP PCA9685 product page | `docs/research/smart_robot/sources/pca9685_product.html` | `28cbfe16e1a9b64c21ee3dec97f01f1277aa08013b6d67e11084a08536804468` |
| NXP PCA9685 datasheet | `docs/research/smart_robot/sources/pca9685_datasheet.pdf` | `237d47f339cac4c3a0d56a5f0b4d3c93df71e3eb43f36ac57ea4ff38e6b2e585` |
| JST XH connector datasheet | `docs/research/smart_robot/sources/jst_xh_connector_datasheet.pdf` | `9426b136902f11900825077535e5c65032b7fbc31ffb59c5e9e1f463bb20fb90` |

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

## Design Review Notes

- The original high-level document is directionally sound: keep the PMU,
  motion core, and wheel actuator boards separate for reuse and safety.
- Do not draw the final KiCad/JLC schematic until each board has a passing
  CircuitCI logical schematic. This catches rail and I/O mistakes before CAD
  symbol/footprint work.
- The first verifiable slice is
  `demos/smart_robot/circuitci/motion_core/project.yaml`. It covers the
  LicheeRV-to-AT32F435 UART/enable/fault link, AT32F435 rail budget,
  ICM-42688-P SPI/interrupt interface, and MCU-side CAN/RS485 logic levels.
- The CAN and RS485 transceiver models are intentionally generic placeholders.
  They verify MCU-side power and I/O levels only; exact bus protection,
  termination, common-mode range, ESD, and connector layout require concrete
  transceiver part selection.
- The first PMU validation slice is
  `demos/smart_robot/circuitci/pmu/project.yaml`. It verifies BQ25798 input and
  charge-current budget, TPS54331 5 V output budget, TPS62162 3.3 V support
  parts, and e-stop rail gating policy. The high-current servo/wheel switch
  model is a design-policy placeholder, not a fabrication-ready MOSFET/eFuse
  selection.
- The first wheel-actuator validation slice is
  `demos/smart_robot/circuitci/wheel_actuator/project.yaml`. It verifies the
  AT32M416-to-DRV8323 six-PWM interface, SPI/fault pins, 3.3 V encoder/Hall
  inputs, CAN transceiver MCU-side logic, rail budgets, and a preliminary
  3x CSD88599Q5DC wheel bridge candidate.
- The wheel actuator now also checks `M1`, a modeled first-pass motor-load
  design envelope: 10 A phase peak, 6 A phase RMS, 6 A regeneration,
  5 mohm / 1 W phase shunts, 8 A motor connector rating, 10 ohm gate
  resistors, 200 ns dead time, and 20 kHz PWM. These values are design-policy
  inputs for a reusable small robot actuator, not sourced motor
  characterization.
- The wheel slice does not yet validate MOSFET SOA, gate charge, switching
  loss, current-sense accuracy, thermal behavior, regeneration clamp energy,
  PCB copper temperature rise, or motor-control loop stability.
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
