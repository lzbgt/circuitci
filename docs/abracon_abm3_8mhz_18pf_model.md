# Abracon ABM3 8 MHz Crystal Model

`vendor.abracon.abm3_8mhz_18pf` is a source-backed static crystal model for
Abracon's ABM3 8.000 MHz ceramic SMD crystal with the standard 18 pF load
capacitance option.

The retained source is the official Abracon ABM3 datasheet:
`docs/research/datasheets/abracon/abm3.pdf`.

## Encoded Facts

- Two passive crystal terminals: `A` and `B`.
- Nominal frequency: `8 MHz`.
- Load capacitance target: `18 pF`.
- Datasheet metadata records 140 ohm maximum ESR for the 8 MHz to below-9 MHz
  range, 7 pF shunt capacitance, 10 uW to 100 uW drive level, +/-50 ppm
  standard frequency tolerance and stability, and +/-5 ppm first-year aging.

## Validation Use

`CLOCK_SOURCE_VALID` uses the model's `crystal.load_capacitance_F` value to
screen a board-level oscillator support network. For the generic MCU clock
fixture, 32 pF load capacitors on each crystal leg plus the MCU model's 2 pF
stray capacitance produce an 18 pF effective load:

```text
Ceff = (32 pF * 32 pF) / (32 pF + 32 pF) + 2 pF = 18 pF
```

The failing fixture keeps 8 pF capacitors, producing only 6 pF effective load.

## Boundary

This is not an oscillator sign-off model. It does not prove oscillator startup,
negative resistance, drive-level stress, ppm accuracy over temperature,
motional behavior, layout parasitics, or phase noise.
