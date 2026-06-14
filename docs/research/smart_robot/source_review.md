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
| TI BQ25798 datasheet | `docs/research/smart_robot/sources/bq25798_datasheet.pdf` | `631d541679512116b5cf7ade0516000e78bc95f060b8b958e726c1b6eb74c27f` |

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
