# TI TCAN3413 CAN Transceiver Observation

This direct-open GUI example validates a reduced generated-SPICE face for the
source-backed TI TCAN3413 3.3 V CAN FD transceiver.

The observation checks a 3.3 V normal-mode dominant-state snapshot: VCC and VIO
are in range, TXD is low, STB is low, RXD is low, CANH is high, and CANL is
low. It uses sourced TCAN3413 supply, VIO, pinout, logic-threshold, CAN FD
data-rate, light-bus data-rate, and bus-fault metadata, but it is only a
preliminary normal-operation line-state model. It is not a termination,
stub-length, common-mode, cable, EMC, bus-fault-energy, CAN FD timing, or final
signal-integrity sign-off model.
