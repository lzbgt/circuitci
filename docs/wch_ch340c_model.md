# WCH CH340C Model

## Sources

- Official WCH English metadata:
  `docs/research/datasheets/wch/ch340ds1_wch_ic_metadata.json`
- Official WCH Chinese metadata:
  `docs/research/datasheets/wch/ch340ds1_wch_cn_metadata.json`
- Local text-extraction copy:
  `docs/research/datasheets/wch/ch340ds1_sparkfun_mirror.pdf`

The official WCH metadata identifies file `CH340DS1.PDF`, version `3.4`, file
id `79`, and scope `CH340`. The official binary download endpoint requires a
browser refresh/session token, so the repo keeps the official metadata JSON and
an inspectable public PDF mirror for local text extraction.

## Modeled Facts

The `vendor.wch.ch340c` model now captures board-level facts needed by common
IoT validation:

- CH340C supports 5 V and 3.3 V supply modes.
- The model accepts `VCC` from `3.1 V` to `5.3 V`, matching the datasheet's
  CH340C 3.3 V mode minimum and 5 V mode maximum.
- `max_supply_current_A` is `0.02 A`, from the 5 V operating current maximum.
- 3.3 V-mode input thresholds are `VIH >= 1.9 V` and `VIL <= 0.8 V`.
- 3.3 V-mode output high is conservatively modeled as `2.7 V` from
  `VCC - 0.6 V` at 2 mA source current.
- `source_impedance_ohm` is conservatively set to `300 ohm` from the same
  `0.6 V / 2 mA` output-high condition.
- `DTR_N` and `RTS_N` are modeled as active-low MODEM outputs.
- CH340C has an integrated clock generator, so the model does not require an
  external crystal for board validation.

## Validation Use

`POWER_TREE_VALID` can now reject rails outside the datasheet-backed CH340C VCC
range. `UART_BOOTLOADER_SYNC` and control-line scenarios can continue to use
the CH340C as a USB-UART bridge without embedding WCH-specific logic in the
engine.

The model is not valid for USB PHY sign-off, transistor-level modem-line
behavior, or final I/O injection-current sign-off. Use `GPIO_BACKDRIVE` or
`analog_transient` scenarios when CH340C and the target MCU can be powered from
separate rails.
