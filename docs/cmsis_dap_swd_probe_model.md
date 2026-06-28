# CMSIS-DAP SWD Probe Model

## Sources

- Saved Arm CMSIS-DAP source notes:
  `docs/research/debug/cmsis_dap/cmsis_dap_sources.md`
- CMSIS-DAP firmware configuration template:
  `docs/research/debug/cmsis_dap/cmsis_dap_config_template.h`
- CMSIS-DAP command reference source:
  `docs/research/debug/cmsis_dap/cmsis_dap_ref_dap.txt`

## Modeled Facts

`generic.debug.cmsis_dap_swd_probe` is a source-backed generic interface model,
not a vendor probe IC. It captures only the CMSIS-DAP SWD pin contract:

- SWD mode uses `SWCLK`, `SWDIO`, and `nRESET`.
- `SWCLK` is represented as a push-pull output.
- `SWDIO` is represented as a push-pull output for idle-state observation, with
  source notes documenting that it is also an input during SWD receive phases.
- `nRESET` is represented as an open-drain-style output that either pulls low
  or releases to the target pull-up network.
- `SWO` is represented as a high-impedance input.
- `VTREF` is the target reference node used by the reduced model to scale line
  drivers. The generic model deliberately does not assign target-voltage
  operating limits because those depend on the specific probe and target board.

## Validation Use

The generated-SPICE face `CIRCUITCI_CMSIS_DAP_SWD_PROBE` enables executable
line-state observations for common SWD bring-up checks. Board IR component
parameters control the observable state:

- `observation_swclk_state`
- `observation_swdio_state`
- `observation_nreset_assert`

`examples/good_cmsis_dap_swd_probe_observation` is registered as the GUI
`CMSIS-DAP SWD Probe` example. It opens with routed schematic metadata, can run
the generated transient observation, and can regenerate model-aware probes and
checks for the placed `UPROBE` component through `Create Checks`.

The model is not valid for probe-vendor electrical sign-off, target voltage
compatibility sign-off, USB protocol or enumeration, SWD turnaround timing,
JTAG/SWD protocol transfer correctness, SWO bandwidth, ESD behavior, or
connector mechanical pinout sign-off.
