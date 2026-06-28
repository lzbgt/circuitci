# Sipeed LicheeRV-Nano-W Model

`vendor.sipeed.licheerv_nano_w` is a source-backed Sipeed LicheeRV-Nano-W
module model for preliminary module-rail and low-speed control-link screening.

## Sources

- Introduction HTML:
  `docs/research/smart_robot/sources/licheerv_nano_intro.html`
- Introduction URL:
  `https://wiki.sipeed.com/hardware/en/lichee/RV_Nano/1_intro.html`
- Introduction SHA-256:
  `7b8a90dbe05c8f03c0b9036ad9204ef851fcdd4256b982870592b00e2bccecf3`
- Schematic PDF:
  `docs/research/smart_robot/sources/licheerv_nano_70405_schematic.pdf`
- Schematic URL:
  `https://cn.dl.sipeed.com/fileList/LICHEE/LicheeRV_Nano/02_Schematic/LicheeRV_Nano-70405_Schematic.pdf`
- Schematic SHA-256:
  `b09ec99069e7f696498b3501785f5296fd0ecaed6d1895d16de2c2e057c2fd19`
- Retrieved: `2026-06-14`

## Modeled Facts

- Module supply rail: `5 V`, with a source-backed preliminary operating window
  of `4.75 V` to `5.25 V`.
- Module current budget: `1 A` source-backed preliminary supply-current class.
- UART0 RX and fault IRQ are represented as high-impedance digital inputs with
  `2.0 V` high and `0.8 V` low thresholds.
- UART0 TX and motion enable are represented as board-facing digital outputs
  with `3.3 V` high-state metadata and `50 ohm` source impedance.

## Generated-SPICE Face

`CIRCUITCI_LICHEERV_NANO_W_MODULE` is a reduced observation model for:

- 5 V module rail checks.
- UART0 TX idle-output line-state checks.
- Host-driven UART0 RX input line-state checks.
- Motion-enable GPIO output checks.
- Fault IRQ input-state checks.

The output states are explicit Board IR component parameters:

- `observation_uart0_tx_a16_state`
- `observation_gpioa14_motion_en_state`

The direct-open GUI fixture is:

- `examples/good_sipeed_licheerv_nano_w_observation/project.yaml`

Its `Create Checks` action regenerates 5 V rail, UART0 TX/RX, motion-enable,
and fault-IRQ checks for the placed module without editing YAML.

## Limits

This model is not valid for Linux boot power transients, internal SoC rails,
firmware behavior, USB/MIPI/high-speed interfaces, RF/Wi-Fi behavior, thermal
behavior, exact header numbering beyond the reviewed project-facing nets, or
final signal-integrity sign-off. Those require separate measurement, firmware,
schematic-symbol, layout, and SI evidence.
