Yes — then I would redesign it as a **reusable robot control stack**, not a one-off board for only this two-wheel robot.

The strongest architecture is:

```text
Reusable smart-robot stack
├─ Brain module/carrier: LicheeRV Nano WiFi
├─ Motion core board: AT32F435 + ICM-42688-P + CAN + servo bus
├─ Wheel actuator board × N: AT32M416 + BLDC FOC + encoder + local jump actuator
├─ Servo / payload board: PCA9685 or RS485 smart-servo hub
└─ PMU / battery-charge board: USB-C PD + charger + BMS/protection + rails
```

The first KiCad CAD bridges for all current board slices live under
`demos/smart_robot/kicad/`. They are schematic connectivity skeletons that
import back through CircuitCI with source-backed model mappings. The richer
validation sources of truth are still the matching
`demos/smart_robot/circuitci/*/project.yaml` files until the CAD import path
round-trips power-domain metadata, validation scenarios, and routed PCB
evidence.
`demos/smart_robot/kicad/wheel_actuator/wheel_actuator.kicad_pcb` is the first
PCB-layout evidence bridge for the highest-risk actuator board; it exists to
exercise placement, pad, route, via, zone, and net-rule import before full
fabrication layout work. The wheel PCB bridge now also round-trips enough CAN
route evidence for CircuitCI to validate the selected CAN TVS and termination
placement scenarios from imported CAD geometry, plus first-pass phase and
`VBAT_SW` route-width checks and phase-shunt/current-sense placement checks
for the selected wheel current envelope. The logical wheel model also runs a
first-pass CSD88599Q5DC bridge loss/thermal budget screen from the sourced
reference-loss point and a first-pass explicit `REGEN1` regeneration absorber
budget screen. It also runs a first-pass CSD88599Q5DC switching screen from
source-backed gate charge/rise/fall timing, plus a static shunt/gain/ADC
current-sense accuracy screen. `MOTOR_BRIDGE_SOA_VALID` is also declared and
uses TI Figure 4-3 as a typical system SOA curve, screening 115 C board
temperature, 10 A phase-peak current, and 2x current margin. These checks are
not final measured switching waveform, PWM sampling, repeated-pulse regen, or
selected amplifier sign-off. The logical wheel model now also includes
a blocking `MODEL_QUALITY_REQUIRED` gate for `M1`, plus a
`LOAD_CABLE_CURRENT_VALID` gate and `LOAD_CABLE_THERMAL_DERATING_VALID` gate
and `LOAD_CABLE_VOLTAGE_DROP_VALID` gate for actuator-bus harness evidence.
The selected Vishay RH100 1 ohm / 100 W resistor closes the first-pass regen
absorber evidence gate, but the actuator board remains intentionally
non-fabrication-ready until the motor and cable temperature-rise evidence are
selected from datasheets or measured evidence.

Do **not** merge the charger, LicheeRV, IMU, motor drivers, and servo power all onto one PCB at first. For a reusable platform, make the **PMU board**, **motion core board**, and **actuator boards** separate.

---

# 1. Revised modular board set

## Module A — Brain Carrier Board

Purpose:

```text
LicheeRV Nano WiFi
camera
audio
screen
Wi-Fi
vision/AI
Linux behavior logic
```

Interface to real-time system:

```text
UART0 + ENABLE + FAULT + optional RESET
```

Use from the attached LicheeRV Nano pinout:

| LicheeRV Nano pin  | Net name           |         Direction | Function                        |
| ------------------ | ------------------ | ----------------: | ------------------------------- |
| GPIOA16 / UART0 TX | `LRV_UART_TX`      | Nano → Motion MCU | command packets                 |
| GPIOA17 / UART0 RX | `LRV_UART_RX`      | Motion MCU → Nano | telemetry                       |
| GPIOA14            | `LRV_MOTION_EN`    | Nano → Motion MCU | enable robot motion             |
| GPIOA15            | `MOTION_FAULT_IRQ` | Motion MCU → Nano | fault interrupt                 |
| 5V                 | `5V_SYS`           |        PMU → Nano | main 5 V input                  |
| GND                | `GND`              |            common | ground                          |
| 3V3                | `3V3_LRV_REF`      |    reference only | do not power main board from it |

Avoid using the SDIO/eMMC/SPI1 side pins unless you are very sure they are free. For a reusable design, keep the LicheeRV dependency minimal.

---

## Module B — Reusable Motion Core Board

This is the board you reuse across different smart robots.

Main parts:

```text
AT32F435VGT7 / AT32F435RGT7
ICM-42688-P
TCAN3413 CAN transceiver
THVD1450 RS485 transceiver
ESD2CAN24-Q1 CANH/CANL TVS near connector
ESDS552 RS485 A/B TVS near connector
120R CAN/RS485 endpoint terminators as explicit population options
explicit CAN/RS485 TVS and terminator route-placement contracts
PCA9685 optional
power monitors
watchdog / e-stop logic
```

AT32F435 is a good fit because it has a 288 MHz Cortex-M4F core, rich UART/SPI/I2C/CAN resources, many GPIO, and enough compute for IMU fusion and balance control. Artery lists AT32F435/437 with 288 MHz Cortex-M4F, up to 116 GPIO, 4 SPI/I2S, 8 UART-class ports, 3 I2C, and 2 CAN 2.0B interfaces. ([arterytek.com][1])

ICM-42688-P is suitable as the main IMU: TDK lists SPI/I2C/I3C host interface, 2 KB FIFO, 2 interrupts, gyro noise 2.8 mdps/√Hz, accel noise 70 µg/√Hz, ±2000 dps gyro, and ±16 g accel ranges. ([TDK][2])

### Motion Core Board connectors

#### J1 — Brain link

```text
1  5V_SYS
2  GND
3  LRV_UART_TX_TO_MCU_RX
4  LRV_UART_RX_FROM_MCU_TX
5  LRV_MOTION_EN
6  MOTION_FAULT_IRQ
7  LRV_RESET_OPTIONAL
8  3V3_REF
```

#### J2 — Robot CAN actuator bus

Use this for wheel boards, arm boards, future joint boards.
First-pass connector family: JST VH, because it gives a source-backed
10 A / 250 V static connector budget for the switched wheel rail. Validate the
selected cable assembly, wire gauge, crimp, temperature rise, vibration,
termination, and protection separately before fabrication.

```text
1  VBAT_SW
2  GND_POWER
3  CANH
4  CANL
5  5V_AUX
6  ENABLE
7  FAULT
8  SYNC_OPTIONAL
```

Use this same connector on every actuator board. That is what makes the design reusable.

#### J3 — RS485 smart-servo bus

```text
1  VSERVO
2  GND_SERVO
3  RS485_A
4  RS485_B
5  SERVO_PWR_EN
6  SERVO_FAULT
```

#### J4 — I2C sensor / payload bus

```text
1  3V3_AUX
2  GND
3  I2C_SCL
4  I2C_SDA
5  INT_OPTIONAL
6  5V_AUX_OPTIONAL
```

#### J5 — debug

```text
1  3V3
2  SWDIO
3  SWCLK
4  GND
5  NRST
6  DEBUG_UART_TX
7  DEBUG_UART_RX
```

---

## Module C — Wheel Actuator Board

One board per wheel.

```text
AT32M416
3-phase gate driver
6 MOSFET bridge
3-shunt or 2-shunt current sensing
wheel encoder / Hall input
local jump servo / jump actuator output
TCAN3413 CAN bus
ESD2CAN24-Q1 CANH/CANL TVS near connector
120R CAN endpoint terminator as explicit population option
explicit CAN TVS and terminator route-placement contracts
local fault protection
```

AT32M416 is a good one-motor board MCU because it has 180 MHz Cortex-M4F, FPU/DSP, CAN-FD, one advanced motor PWM timer, 2× 2.5 Msps ADCs, 2 comparators, and 4 op-amps/PGA. ([arterytek.com][3])

### Wheel board connector map

#### J1 — CAN power/control input

```text
1  VBAT_SW
2  GND_POWER
3  CANH
4  CANL
5  5V_AUX
6  ENABLE_IN
7  FAULT_OUT
8  SYNC_IN
```

#### J2 — BLDC motor phase output

```text
1  PHASE_U
2  PHASE_V
3  PHASE_W
```

Use a high-current connector, not a small JST signal connector.

#### J3 — encoder / Hall

```text
1  5V_ENC
2  3V3_ENC
3  GND
4  ENC_A / HALL_A
5  ENC_B / HALL_B
6  ENC_Z / HALL_C
7  ENC_SPI_CS optional
8  ENC_SPI_SCK optional
9  ENC_SPI_MISO optional
10 ENC_SPI_MOSI optional
```

#### J4 — local jump actuator

For hobby PWM servo:

```text
1  GND_SERVO
2  VSERVO
3  SERVO_PWM
4  SERVO_FEEDBACK optional
```

For smart servo:

```text
1  VSERVO
2  GND_SERVO
3  RS485_A
4  RS485_B
```

My recommendation: use **smart RS485 servos** for jump if budget allows. Use PWM servos only if the jump mechanism is spring-release or low-load.

---

## Module D — Servo / Payload Board

For camera pitch, screen tilt, head, light mass arm:

```text
PCA9685 16-channel PWM servo driver
I2C input
separate VSERVO rail
bulk capacitor
per-channel signal resistor
```

For serious balance mass-arm actuation:

```text
RS485 smart servo bus
or local actuator board
```

Do not use dumb PWM servos for balance-critical mass shifting unless you add position feedback.

---

# 2. Power and charging module

This should be a separate **PMU board**. It is reusable in every future robot.

## Recommended PMU architecture

```text
USB-C PD input
  |
  +-- PD sink controller
  |
  +-- battery charger / power-path charger
  |
Battery pack + BMS/protection
  |
  +-- VBAT_SW        → wheel actuator boards
  +-- VSERVO         → servos / jump actuators
  +-- 5V_SYS         → LicheeRV Nano / sensors
  +-- 3V3_LOGIC      → MCU / logic
  +-- power monitors → Motion Core Board
```

For the charger IC, the clean high-quality choice is **BQ25798**. It is a 1–4 cell, 5 A buck-boost charger intended for USB PD-type input and integrates four switching MOSFETs and BATFET. ([Texas Instruments][4])

CircuitCI currently models the PMU charger, 5 V buck, 3.3 V buck, and switched
servo/wheel rails. `U_SERVO_SW` now uses a source-backed configured TI TPS25948
8 A eFuse model for first-pass servo-rail screening. `U_WHEEL_SW` now uses a
source-backed TPS24751/CSD17501Q5A reverse-blocking switch path for first-pass
wheel-rail screening.

For USB-C PD negotiation, use either:

```text
HUSB238 / HUSB238A
or
IP2721
```

HUSB238 is a USB PD sink controller rated up to 100 W, and HUSB238A supports I2C and GPIO modes for configuration/status. ([hynetek.com][5]) IP2721 is a cheaper hardware PD trigger-style controller that can request fixed PD voltages such as 5 V, 9 V, 12 V, 15 V, or 20 V depending variant/configuration. ([zrsc-ic.com][6])

For current/power telemetry, use INA226 or compatible. INA226 monitors both shunt voltage and bus voltage over I2C and gives direct current/power readings after calibration. ([Texas Instruments][7])

---

# 3. Battery voltage choice

For this robot, I would use:

```text
Prototype:
  2S LiPo/Li-ion, 7.4 V nominal / 8.4 V full

More powerful version:
  3S LiPo/Li-ion, 11.1 V nominal / 12.6 V full
```

### 2S advantages

```text
simpler
compatible with 7.4 V HV servos
lower buck stress
safer for small robot prototype
cheap battery packs available
```

### 3S advantages

```text
lower motor current for same power
better wheel motor headroom
better jump impulse
more efficient motor bus
```

My practical recommendation:

```text
First robot prototype:
  2S high-discharge pack

Reusable PMU design:
  make charger/power path configurable for 2S or 3S
```

BQ25798 supports 1–4 cell charging, so it gives you this migration path. ([Texas Instruments][4])

---

# 4. PMU board circuit blocks

## 4.1 USB-C PD input

```text
USB-C receptacle
  CC1/CC2 → HUSB238A or IP2721
  VBUS    → fuse/eFuse → charger input
  GND     → power ground
  D+/D-   → NC unless needed
  ESD     → USB-C ESD protection
```

Recommended default:

```text
HUSB238A if you want MCU-readable PD status
IP2721 if you want cheap fixed-voltage PD trigger
```

For 2S/3S charging, request:

```text
2S:
  12 V or 15 V PD input

3S:
  15 V or 20 V PD input
```

---

## 4.2 Charger / power-path

Use:

```text
BQ25798
```

Connections:

```text
VBUS_IN      ← PD input after fuse/eFuse
BAT          ↔ battery pack positive
SYS          → system power node
I2C          ↔ AT32F435
INT/STAT     → AT32F435 GPIO
TS/NTC       → battery thermistor
PROG/ILIM    → configured per datasheet
```

Design goals:

```text
charge current:
  1 A to 2 A for small packs
  3 A to 5 A only if thermals and pack allow it

system power:
  robot can run from USB-C PD while charging
  robot can shut down motor rail during charging
```

For safety, do not allow jump/motor power while charging unless you intentionally design for it. Default should be:

```text
USB-C plugged:
  allow brain + MCU + sensors
  disable wheel drivers and jump actuators
  allow charging
```

---

## 4.3 Battery protection / BMS

For prototype, the safest low-risk approach is:

```text
use a battery pack with built-in BMS/protection
or use a proven external 2S/3S BMS board
```

For an integrated 3S/4S product PMU, use a proper battery monitor/protector AFE. TI’s BQ76920 supports up to 5-series cells / typical 18 V packs, while the larger BQ76930/BQ76940 handle higher cell counts. ([Texas Instruments][8])

For the first LCPCB version, I would not integrate full cell balancing/protection unless you are ready to validate battery safety thoroughly. Use:

```text
PMU board:
  charger
  power path
  rail generation
  current/voltage monitoring
  fuses/eFuses/load switches

Battery pack:
  built-in BMS/protection/balancing
```

This is much safer and faster.

---

## 4.4 Power rails

### Rail map

| Rail        | Source                 | Use                    | Notes                |
| ----------- | ---------------------- | ---------------------- | -------------------- |
| `VBAT_RAW`  | battery pack           | input to PMU           | protected/fused      |
| `VBAT_SW`   | high-side switch/eFuse | wheel boards           | can be emergency cut |
| `VSERVO`    | buck/BEC from VBAT     | servos                 | high current, noisy  |
| `5V_SYS`    | buck from VBAT/SYS     | LicheeRV Nano, sensors | 3–5 A recommended    |
| `5V_AUX`    | filtered 5 V           | CAN nodes/encoders     | current limited      |
| `3V3_LOGIC` | buck/LDO               | AT32F435, transceivers | clean digital        |
| `3V3_IMU`   | low-noise LDO          | ICM-42688-P only       | quiet local rail     |

### PMU output connector

```text
1  VBAT_SW
2  GND_POWER
3  VSERVO
4  GND_SERVO
5  5V_SYS
6  GND_SYS
7  3V3_LOGIC
8  PWR_GOOD
9  CHG_STATUS
10 PMU_FAULT
11 I2C_SCL
12 I2C_SDA
13 BAT_NTC
14 E_STOP_IN
```

---

# 5. Safety circuits to include

For a jumpable robot, this part is not optional.

## Hardware e-stop chain

```text
E_STOP button
  → PMU latch / supervisor
  → disables VBAT_SW high-side switch
  → disables VSERVO switch
  → pulls wheel board ENABLE low
  → reports fault to AT32F435 and LicheeRV
```

The e-stop must not depend only on Linux software.

The validation model treats the e-stop rail switches as safety-critical
selected parts, not generic policy boxes. The servo rail has a first-pass
TPS25948 model with current-limit, thermal, always-on reverse-current blocking,
and dVdt/inrush evidence. The wheel rail has a first-pass TPS24751 plus
CSD17501Q5A path with current-limit, thermal, off-state reverse-current
isolation, and inrush evidence.
`POWER_SWITCH_BUDGET_VALID`, `POWER_SWITCH_REVERSE_CURRENT_VALID`, and
`POWER_SWITCH_INRUSH_VALID` now check those selected paths. They still do not
replace layout thermal extraction, measured fault/retry waveforms, or final
downstream switched-capacitance evidence.

## Wheel driver enable chain

```text
Motion Core MCU ENABLE
AND PMU_POWER_GOOD
AND no E_STOP
AND no wheel fault
  → WHEEL_ENABLE
```

## Charging interlock

```text
USB-C plugged / CHG_ACTIVE
  → default disable wheel power
  → allow only if debug jumper fitted
```

## Brownout behavior

```text
VBAT low:
  warn LicheeRV
  reduce servo/motor power
  land/sit down
  disable jump
  then controlled shutdown
```

---

# 6. Updated reusable pin map

## LicheeRV Nano connector

```text
GPIOA16 / UART0_TX  → AT32F435 USART_RX
GPIOA17 / UART0_RX  ← AT32F435 USART_TX
GPIOA14             → AT32F435 LRV_ENABLE input
GPIOA15             ← AT32F435 FAULT_IRQ output
5V                  ← PMU 5V_SYS
GND                 ↔ GND
3V3                 → reference only
```

## AT32F435 Motion Core functional mapping

| Function | Net                                                         |
| -------- | ----------------------------------------------------------- |
| USART1   | `LRV_UART_TX/RX`                                            |
| SPI1     | `ICM42688_SPI_SCK/MOSI/MISO/CS`                             |
| EXTI     | `ICM42688_INT1`, `ICM42688_INT2`                            |
| CAN1     | `ROBOT_CAN_TX/RX`                                           |
| I2C1     | `PMU_I2C_SCL/SDA`, `INA226`, `BQ25798`, optional `HUSB238A` |
| I2C2     | `PCA9685_SCL/SDA`, auxiliary sensors                        |
| USART2   | `RS485_SERVO_TX/RX/DE`                                      |
| USART3   | `DEBUG_UART_TX/RX`                                          |
| ADC      | `VBAT_DIV`, `VSERVO_DIV`, `5V_DIV`, `BOARD_TEMP`            |
| GPIO OUT | `WHEEL_EN`, `SERVO_PWR_EN`, `LRV_RESET`, `PMU_WAKE`         |
| GPIO IN  | `PMU_FAULT`, `CHG_STATUS`, `E_STOP`, `WHEEL_FAULT_L/R`      |

## AT32M416 Wheel Board functional mapping

| Function            | Net                                    |
| ------------------- | -------------------------------------- |
| Advanced PWM CH1/1N | `PWM_UH/UL`                            |
| Advanced PWM CH2/2N | `PWM_VH/VL`                            |
| Advanced PWM CH3/3N | `PWM_WH/WL`                            |
| Brake input         | `PWM_BKIN_FAULT`                       |
| ADC injected        | `I_U`, `I_V`, `I_W`                    |
| ADC regular         | `VBAT_WHEEL`, `TEMP_MOS`, `TEMP_MOTOR` |
| Comparator          | `HW_OCP`                               |
| CAN-FD              | `ROBOT_CAN_TX/RX`                      |
| Timer encoder       | `ENC_A/B/Z`                            |
| GPIO/EXTI           | `HALL_A/B/C`, `DRV_NFAULT`             |
| SPI/GPIO            | `DRV_SPI_*` if using smart gate driver |
| GPIO OUT            | `DRV_EN`, `SERVO_PWR_EN`, `FAULT_OUT`  |
| PWM/UART            | `JUMP_SERVO_PWM` or `RS485_JUMP_SERVO` |

---

# 7. EasyEDA project structure

Create four reusable PCB projects, not one giant one:

```text
robot-pmu-usbc-2s3s
  01_usb_c_pd
  02_bq25798_charger
  03_battery_bms_interface
  04_power_rails
  05_power_monitor
  06_estop_safety
  07_connectors

robot-motion-core-at32f435
  01_at32f435
  02_icm42688p
  03_can_rs485
  04_pmu_interface
  05_lrv_nano_interface
  06_servo_i2c
  07_debug

robot-wheel-at32m416
  01_at32m416
  02_gate_driver
  03_mosfet_bridge
  04_current_sense
  05_encoder_hall
  06_can_interface
  07_jump_servo
  08_power_fault

robot-servo-payload-hub
  01_pca9685
  02_rs485_servo
  03_servo_power_distribution
  04_connectors
```

---

# 8. My recommended first fabrication version

For the first LC EasyEDA fabrication, build:

```text
1× PMU board:
  USB-C PD input
  BQ25798 charger footprint
  battery-pack/BMS connector
  5V_SYS buck
  VSERVO buck/BEC connector
  source-backed VBAT_SW high-side switch/eFuse
  source-backed VSERVO switch/eFuse
  INA226 current monitor
  e-stop chain

1× Motion Core board:
  AT32F435
  ICM-42688-P
  CAN
  RS485
  PCA9685 optional
  LicheeRV Nano connector

2× Wheel board:
  AT32M416
  gate driver
  MOSFET bridge
  encoder/Hall
  local jump actuator connector
```

For battery safety, I would use a **protected 2S pack** or **external 2S BMS pack** in the first revision, while still placing the BQ25798/charger section on the PMU board. After the robot works, make the integrated 3S BMS version.

This gives you a reusable stack: future robots can reuse the **PMU**, **motion core**, and **wheel actuator modules**, and only change the brain carrier or mechanical payload.

[1]: https://www.arterytek.com/cn/product/AT32F435.jsp?utm_source=chatgpt.com "AT32F435 系列"
[2]: https://product.tdk.com/system/files/dam/doc/product/sensor/mortion-inertial/imu/data_sheet/ds-000347-icm-42688-p-v1.6.pdf?utm_source=chatgpt.com "ICM-42688-P Datasheet - TDK Product Center"
[3]: https://www.arterytek.com/cn/product/AT32M416.jsp?utm_source=chatgpt.com "AT32M416 系列"
[4]: https://www.ti.com/lit/ds/symlink/bq25798.pdf?utm_source=chatgpt.com "BQ25798 I2C Controlled, 1- to 4-Cell, 5-A Buck-Boost ..."
[5]: https://www.hynetek.com/uploadfiles/site/219/news/aabbbbdb-48c9-4a44-a6dc-2c15f53282e6.pdf?utm_source=chatgpt.com "HUSB238"
[6]: https://www.zrsc-ic.com/Data/zrsc-ic/upload/file/INJOINIC-PRO/IP2721%20Datasheet%20V2.0.pdf?utm_source=chatgpt.com "IP2721"
[7]: https://www.ti.com/lit/gpn/INA226?utm_source=chatgpt.com "INA226 36V, 16-Bit, Ultra-Precise I2C Output Current, ..."
[8]: https://www.ti.com/lit/ds/symlink/bq76940.pdf?utm_source=chatgpt.com "BQ769x0 3-Series to 15-Series Cell Battery Monitor Family ..."
