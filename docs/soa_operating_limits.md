# SOA Operating Limits

Generated Board IR SPICE validation supports datasheet safe-operating-area
checks for MOSFETs when the component model provides explicit digitized SOA
curve points.

## Scope

This is not graph OCR and not a substitute for vendor sign-off. CircuitCI only
evaluates SOA points that are present in model metadata and traceable to a
datasheet source. Missing, malformed, non-positive, or unsorted points are a
metadata failure when a model claims SOA support.

The first target is MOSFET forward-bias SOA for generated Board IR decks. The
validator samples simulated `VDS` and `ID` across the transient, computes the
maximum `ID` allowed at the corresponding `VDS`, and emits
`SPICE_OPERATING_LIMIT` when the waveform leaves the digitized envelope.

## Metadata Contract

Models may declare SOA curves under `datasheet.safe_operating_area`:

```yaml
safe_operating_area:
  vds_id_curves:
    - name: forward_bias_100us
      pulse_width_us: 100
      duty_cycle_max: 0.02
      temperature_c: 25
      source_document: onsemi_fdmc86184-d.pdf
      source_figure: Figure 11 Forward Bias Safe Operating Area
      digitization:
        method: manual
        confidence: low
        note: Hand-digitized from datasheet graph; screening evidence only.
      points:
        - {vds_v: 1, id_a: 100}
        - {vds_v: 10, id_a: 40}
        - {vds_v: 100, id_a: 1}
```

Required constraints:

- curve names must be unique
- `pulse_width_us` must be finite and positive
- `duty_cycle_max` must be finite and in `(0, 1]`
- at least two points are required
- every point must have finite positive `vds_v` and `id_a`
- points must be strictly increasing by `vds_v`

`source_document`, `source_figure`, and `digitization` must identify how the
points were derived. For hand-digitized points, the model quality and docs must
say this is preliminary screening evidence, not final sign-off.

## Evaluation

The evaluator chooses the SOA curve with the smallest `pulse_width_us` that is
greater than or equal to the measured contiguous duration above continuous
current. If no curve covers the duration, it chooses the longest available
curve and fails if the waveform exceeds that curve.

For a sampled point between two SOA points, the allowable current is
interpolated in log-log space:

```text
log10(I_allowed) = lerp(log10(I1), log10(I2), t)
t = (log10(VDS) - log10(VDS1)) / (log10(VDS2) - log10(VDS1))
```

For `VDS` below the digitized point range, the first endpoint current limit is
used. For `VDS` above the maximum digitized point, the check fails closed
because passing would require extrapolation.

SOA evidence is report-only when the scalar current is below the continuous
rating. When current exceeds the continuous rating and pulse ratings are
enabled, SOA evidence must also pass before a pulse-current waiver can pass.

## Report Evidence

SOA findings include:

- `component`
- `rating: SOA`
- `vds_v`
- `id_a`
- `time_us`
- `soa_margin_ratio`
- `limit.id_limit_a`
- `limit.curve_pulse_width_us`
- `limit.interpolation: log_log`
- `limit.source_figure`
- `limit.digitization_warning`
