# CMSIS-DAP SWD Probe Observation

This fixture opens directly in the GUI as `CMSIS-DAP SWD Probe`. It exercises
the reduced `CIRCUITCI_CMSIS_DAP_SWD_PROBE` generated-SPICE face using the
source-backed generic CMSIS-DAP SWD interface model.

The observation is intentionally limited to board-level SWD line states:

- `VTREF` is a target-provided 3.3 V reference.
- `SWCLK` and `SWDIO` are observed high in a default idle snapshot.
- `nRESET` is released by the probe and pulled high by the target-side pull-up.
- `SWO` is represented as a high-impedance input with a target-side pull-up.

This is not a probe-vendor electrical model, USB transport model, SWD protocol
transfer model, turnaround timing model, or connector mechanical pinout
sign-off. It is a quick executable check that a CMSIS-DAP-style SWD probe
interface has plausible target-reference, clock/data, reset, and trace-input
line behavior before deeper protocol validation.
