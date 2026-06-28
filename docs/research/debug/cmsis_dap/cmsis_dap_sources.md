# CMSIS-DAP / SWD Source Notes

Retrieved: 2026-06-28

## Saved Sources

- CMSIS-DAP README:
  `docs/research/debug/cmsis_dap/cmsis_dap_readme.md`
- CMSIS-DAP documentation source:
  `docs/research/debug/cmsis_dap/cmsis_dap_mainpage.md`
- CMSIS-DAP command reference source:
  `docs/research/debug/cmsis_dap/cmsis_dap_ref_dap.txt`
- CMSIS-DAP firmware configuration template:
  `docs/research/debug/cmsis_dap/cmsis_dap_config_template.h`
- CMSIS-DAP DAP header:
  `docs/research/debug/cmsis_dap/cmsis_dap_dap_h.h`
## Extracted Facts Used

- CMSIS-DAP is a protocol specification and firmware implementation for host
  debug tools communicating with Arm Cortex target processors through a debug
  unit.
- CMSIS-DAP target connection can use a two-pin Serial Wire Debug interface or
  a five-pin JTAG interface.
- The CMSIS-DAP firmware configuration template declares SWD and JTAG support
  flags and defaults to SWD as the default JTAG/SWJ port mode.
- SWD mode uses `SWCLK`, `SWDIO`, and `nRESET`; the DAP header defines
  `DAP_PORT_SWD` as `SWCLK, SWDIO + nRESET`.
- CMSIS-DAP maps `SWCLK/TCK`, `SWDIO/TMS`, `TDI`, `TDO`, `nTRST`, and
  `nRESET` as common SWJ pins.
- The firmware configuration template states that `SWCLK/TCK` is output
  push-pull, `SWDIO/TMS` is output push-pull and input for receiving data,
  `TDO` is input, and `nRESET` is output open drain with a pull-up resistor.
- SWD setup configures `SWCLK`, `SWDIO`, and `nRESET` to output mode and their
  default high level.
- The `DAP_SWJ_Pins` command can read/write SWD/JTAG pins including `nRESET`.
- The `DAP_SWD_Sequence` command can drive or capture `SWDIO` data while using
  `SWCLK` cycles, which confirms SWDIO is bidirectional at the protocol level.

The reduced CircuitCI model intentionally does not claim probe-vendor drive
strength, target voltage limits, connector pin numbering, USB behavior, or SWD
protocol timing from these generic sources.
