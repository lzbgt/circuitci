# ESP32-WROOM-32E Source Notes

Retrieved on 2026-06-13 from official Espressif documentation endpoints.

## Original Documents

| Document | Source URL | Local file | SHA-256 |
| --- | --- | --- | --- |
| ESP32-WROOM-32E ESP32-WROOM-32UE Datasheet v2.0 | <https://www.espressif.com/sites/default/files/documentation/esp32-wroom-32e_esp32-wroom-32ue_datasheet_en.pdf> | `docs/research/datasheets/espressif/esp32-wroom-32e_esp32-wroom-32ue_datasheet_en.pdf` | `1dabd3a7eecee58e2852aaacd6af6f811f4f5d01235211de5ab8486cc001cf40` |
| ESP32 Hardware Design Guidelines | <https://docs.espressif.com/projects/esp-hardware-design-guidelines/en/latest/esp32/esp-hardware-design-guidelines-en-master-esp32.pdf> | `docs/research/datasheets/espressif/esp32_hardware_design_guidelines_en.pdf` | `c37ebba953555065e0c5cb884cb6ce61a9cf7509bc6b7ec6514b501aebef8424` |

## Modeled Facts

- ESP32-WROOM-32E operating supply is `3.0 V` to `3.6 V`; recommended
  operation is `3.3 V`.
- The datasheet states the external power supply should deliver at least
  `0.5 A`; the model uses this as a static worst-case source-current budget
  requirement for board screening.
- GPIO high/low thresholds at 3.3 V are modeled from the DC characteristic
  equations `VIH = 0.75 x VDD` and `VIL = 0.25 x VDD`, producing `2.475 V`
  and `0.825 V` at a 3.3 V board rail.
- EN is modeled as active-low reset/shutdown using the CHIP_PU low-level
  shutdown threshold `0.6 V` and reset-release threshold `0.75 x VDD`.
- SPI flash boot requires GPIO0 high. Joint download boot requires GPIO0 low
  and GPIO2 low. UART download is one of the supported joint download methods.
- The hardware design guidelines recommend a pull-up resistor on GPIO0 and warn
  against high-value capacitors on GPIO0 because they can force download mode.

## Non-Modeled Facts

The first model does not sign off RF matching, antenna placement, flash/PSRAM
pin reuse, boot-ROM serial packet timing, firmware execution, thermal behavior,
or transient current waveform shape.
