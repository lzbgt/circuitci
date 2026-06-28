# TI ESDS552 RS-485 ESD Observation

This direct-open GUI example validates a reduced generated-SPICE face for the
source-backed TI ESDS552 two-channel bidirectional RS-485/RS-422 ESD and surge
protection diode.

The observation checks normal RS-485 A/B line voltages against the sourced
`+/-12 V` standoff rating while loading each line with the datasheet `11 pF`
maximum I/O-to-ground capacitance. It is a preliminary normal-operation
protection check, not an IEC 61000-4-2, IEC 61000-4-5, surge-response,
common-mode, termination, cable-harness, signal-integrity, or final layout
sign-off model.
