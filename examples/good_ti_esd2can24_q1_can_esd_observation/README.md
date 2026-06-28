# TI ESD2CAN24-Q1 CAN ESD Observation

This direct-open GUI example validates a reduced generated-SPICE face for the
source-backed TI ESD2CAN24-Q1 two-channel CAN ESD protection diode.

The observation checks normal CANH/CANL line voltages against the sourced
`+/-24 V` standoff rating while loading each line with the datasheet `3 pF`
typical capacitance. It is a preliminary normal-operation protection check, not
an ISO 7637, ISO 10605, IEC ESD, surge-energy, CAN signal-integrity, cable
harness, route-placement, stub-length, or final layout sign-off model.
