# Diode Operating Limits

Generated Board IR SPICE decks include automatic operating-limit checks for
datasheet-backed diode models.

## Scope

This slice covers two-pin SPICE diode models emitted by
`src/validation/spice_netlist.rs`. It does not change hand-authored netlist
behavior and does not infer ratings for generic estimated diode models.

Generated diode models must provide usable `datasheet.absolute_maximum_ratings`
for:

- reverse voltage: `VRRM` or `VR`, unit `V`
- forward current: `IF` or `IF_AV`, unit `A`
- power: `PD` or `Ptot`, unit `W`

Missing required rating groups emit `SPICE_OPERATING_LIMIT` before solver
execution. A missing rating is not pass evidence.

## SPICE Emission

Generated diode emission inserts a zero-volt current-sense source in series
with the anode:

```spice
VCCI_D1 anode cci_d1_a 0
D1 cci_d1_a cathode MODEL_NAME
```

The operating-limit evaluator uses the same current-sense source name as the
netlist generator.

## Probe Semantics

For a diode with anode `A` and cathode `K`:

- reverse-voltage stress is `max(0, V(K,A))`
- forward-current stress is `max(0, I(VCCI_Dx))`
- power stress is `max(0, V(A,K) * I(VCCI_Dx))`

The checks are evaluated across the full transient. Exceeding a rating emits
`SPICE_OPERATING_LIMIT` with component id, rating key, expression, measured
maximum, time of maximum, unit, signed datasheet value, and absolute comparison
limit.

`IF_AV` is treated as a conservative continuous/average current limit in this
slice. That is intentionally fail-closed for board-level agent review, but it
can false-fail short pulse cases that should instead be checked against
datasheet pulse/surge limits such as `IFSM`. Time-qualified pulse and
temperature derating metadata use the contract in
`docs/temperature_pulse_operating_limits.md`; unqualified pulse ratings must
not waive continuous current findings.

## Datasheet Source

The first vendor diode fixture uses the onsemi 1N4148WS datasheet:

- Source URL: `https://www.onsemi.com/download/data-sheet/pdf/1n4148ws-d.pdf`
- Local copy: `docs/research/datasheets/onsemi_1n4148ws-d.pdf`
- SHA-256:
  `11f014f05f4ab6ba5eddb0bd8fc0c27f49f9fc25433800d0a327595d4031f148`

The metadata records `VRRM = 100 V`, `IF_AV = 0.15 A`, and `PD = 0.2 W` from
the absolute maximum / thermal ratings in the downloaded datasheet at
`TA = 25 C` / `TC = 25 C` as applicable.
