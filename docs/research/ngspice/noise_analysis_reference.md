# Ngspice Noise Analysis Reference

Source: <https://nmg.gitlab.io/ngspice-manual/analysesandoutputcontrol_batchmode/analyses/noise_noiseanalysis.html>

Saved local copy: `docs/research/ngspice/noise_noiseanalysis.html`

Key implementation facts from the ngspice manual:

- `.noise output ref src (dec|lin|oct) pts fstart fstop [pts_per_summary]` runs small-signal noise analysis.
- `output` is the node whose total output noise is desired; optional `ref` makes the output `v(output) - v(ref)`, otherwise ground is implied.
- `src` is the independent source used for input-referred noise.
- Noise analysis produces a spectral-density plot containing `onoise_spectrum` and `inoise_spectrum` over frequency.
- It also produces an integrated-total plot containing scalar `onoise_total` and `inoise_total` over the specified frequency range.

CircuitCI should therefore model noise as a frequency-domain observation with output node, optional reference node, input source, sweep type, points per sweep interval, start frequency, and stop frequency. GUI/report artifacts should preserve both spectral-density curves and integrated totals.
