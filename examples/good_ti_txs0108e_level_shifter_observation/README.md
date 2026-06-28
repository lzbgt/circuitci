# TXS0108E Level-Shifter Observation

This fixture opens directly in the GUI as `TXS0108E Level Shifter`. It uses the
source-backed `vendor.ti.txs0108e` model and the reduced
`CIRCUITCI_TXS0108E_A_TO_B_LEVEL_SHIFTER` generated-SPICE face.

The observation drives `A1` high from a 1.8 V domain with `OE` enabled and
checks that `B1` translates high into a 3.3 V domain under a light load. It also
checks both supply rails and the A-side input level.

This is not TXS0108E timing or signal-integrity sign-off. The model is a
conservative workflow/topology observation for channel binding, supply
compatibility, OE state, and first-order translated output checks.
