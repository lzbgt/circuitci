# Good Noise Observation

Generated-from-board ngspice `.noise` example for a 10 kOhm / 10 kOhm divider.

The observation exports `noise_spectrum.csv` with output and input-referred
spectral density, plus `noise_total.csv` with integrated RMS noise across the
declared 10 Hz to 100 kHz band. The assertions check both output density at
1 kHz and integrated output/input-referred RMS noise.

The fixture is registered in the GUI Examples picker as `Noise Observation`.
Open it from Examples, run observations, then inspect output/input noise density
in Scopes and integrated RMS totals in the noise table. New generated noise run
setups can use the GUI noise check presets to recreate the same output and
input-referred density/RMS checks without editing YAML.
