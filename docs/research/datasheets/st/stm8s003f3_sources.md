# STM8S003F3P6 Source Notes

Retrieved on 2026-07-05 from the official STMicroelectronics datasheet
endpoint.

## Original Documents

| Document | Source URL | Local file | SHA-256 |
| --- | --- | --- | --- |
| STM8S003F3/K3 datasheet | <https://www.st.com/resource/en/datasheet/stm8s003f3.pdf> | `docs/research/datasheets/st/stm8s003f3_datasheet.pdf` | `1a0775fefcda1553ffea5216b60acdb9566e487f60aad8afa5c859a32acaa8f2` |

## Modeled Facts

- `VDD` standard operating voltage is `2.95 V` to `5.5 V` at `fCPU <= 16 MHz`.
- TSSOP20 pin 9 is `VDD`; pin 7 is `VSS`; pin 8 is `VCAP`.
- TSSOP20 pin 4 is `NRST`.
- TSSOP20 pin 18 is `PD1/SWIM`.
- TSSOP20 pin 2 is `PD5/UART1_TX`; pin 3 is `PD6/UART1_RX`.
- The external `VCAP` capacitor range is `470 nF` to `3300 nF`.
- Ambient operating temperature for suffix 6 devices is `-40 C` to `85 C`.
- NRST low and high thresholds are specified as `0.3 x VDD` and `0.7 x VDD`.

## Non-Modeled Facts

The first model does not validate `VCAP` capacitance, ESR, or ESL because the
current passive-component rule set does not bind generic capacitors to MCU
support-pin requirements. It also does not encode GPIO or NRST thresholds as
fixed port thresholds, because the component-model schema currently represents
fixed voltages while the STM8S003F3 thresholds scale with `VDD`. Oscillator
startup, SWIM protocol timing, UART bootloader/protocol behavior, flash/EEPROM
programming behavior, firmware execution, thermal limits, and peak-current
waveforms are outside this static model.
