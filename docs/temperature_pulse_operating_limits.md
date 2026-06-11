# Temperature and Pulse Operating Limits

Generated Board IR SPICE operating-limit checks support two optional
datasheet-qualified refinements:

- temperature derating for ratings such as package power dissipation
- pulse-qualified current ratings for short transient events

The default remains the conservative absolute-maximum check. A scenario must
explicitly opt in before pulse ratings can relax a continuous current limit.

## Scenario Contract

Analog scenarios may declare operating conditions:

```yaml
operating_conditions:
  ambient_temperature_c: 100
  allow_pulse_ratings: true
```

`ambient_temperature_c` asks the validator to derate any rating that provides
linear derating metadata. For package power ratings, missing derating metadata
is a critical `SPICE_OPERATING_LIMIT` finding before solver execution. A
temperature-aware physical report is not valid if the model has only the 25 C
rating.

`allow_pulse_ratings` permits a short current excursion above the continuous
current rating only when the component model provides a pulse current rating
with both:

- `pulse_width_us`
- `duty_cycle_max`

Without those qualifiers, the continuous current rating remains the limit.

## Datasheet Metadata

Each absolute-maximum rating keeps the existing required `value` and `unit`.
Optional fields add context:

```yaml
PD:
  value: 0.3
  unit: W
  reference_temperature_c: 25
  derate_above_c: 25
  derating_per_c: 0.0024
  derating_basis: Derived from PD at TA=25C and TJ(max)=150C.
ID_pulsed:
  value: 1.5
  unit: A
  pulse_width_us: 300
  duty_cycle_max: 0.02
```

Linear derating is evaluated as:

```text
effective_limit = max(0, abs(value) - max(0, ambient - derate_above_c) * abs(derating_per_c))
```

Pulse allowance is evaluated from the simulated waveform. The validator
measures the total transient duration above the continuous current limit and
divides it by the transient duration to estimate duty cycle. The pulse rating
can waive the continuous-current failure only if:

- maximum current is below the pulse current limit
- duration above the continuous limit is no longer than `pulse_width_us`
- estimated duty cycle is no larger than `duty_cycle_max`

This is still not a full safe-operating-area model. It is a conservative,
machine-checkable contract for short-pulse board simulation, and it keeps
unqualified pulse ratings from hiding real overstress.

`vendor.onsemi.fdmc86184` is the first qualified pulse-current fixture. Its
metadata records `ID_continuous = 12 A`, `ID_pulsed = 266 A`,
`pulse_width_us = 300`, and `duty_cycle_max = 0.02` from the downloaded onsemi
datasheet. The same datasheet says pulsed `ID` should refer to the SOA graph;
CircuitCI does not yet digitize that graph, so this fixture proves scalar
qualified pulse-current handling rather than complete SOA sign-off.

## Report Evidence

Operating-limit findings include the original datasheet rating and the
effective comparison limit. Derated findings also include ambient temperature
and derating parameters. Current findings that considered a pulse rating include
the measured pulse duration and duty evidence.
