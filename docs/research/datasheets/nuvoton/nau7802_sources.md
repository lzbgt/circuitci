# Nuvoton NAU7802 Source Notes

## Retained Sources

| Source | URL | Local copy | SHA-256 |
| --- | --- | --- | --- |
| NAU7802 Precision ADC with 2-wire Control Interface Data Sheet Rev 2.6 | <https://www.nuvoton.com/export/resource-files/en-us--DS_NAU7802_DataSheet_EN_Rev2.6.pdf> | `docs/research/datasheets/nuvoton/nau7802_datasheet_rev2_6.pdf` | `1d52edc0267ea1c529003af5b29b1088cee01c64a511079258359ede91633a7d` |

The PDF was retrieved from Nuvoton and retained verbatim. Text extraction is
kept at `docs/research/datasheets/nuvoton/nau7802_datasheet_rev2_6.txt` for
agent-side review.

## Board-Level Facts Used

- NAU7802 is a precision low-power 24-bit ADC for bridge/sensor measurement
  with an onboard low-noise PGA, sigma-delta ADC, and 2-wire control
  interface.
- The supply voltage feature row lists `2.7V~5.5V`.
- The on-chip AVDD regulator can be off or programmed from 2.4 V to 4.5 V;
  the LDO load current at AVDD should not exceed 10 mA, and DVDD must be at
  least 0.3 V above the desired AVDD output voltage.
- Pin descriptions define `DVDD` as digital power supply `2.7V ~ 5.5V` and
  `AVDD/LDO` as the analog supply or LDO output, with external supply mode
  `2.7V ~ 5.5V`.
- Pin descriptions define `SCLK` as serial data clock input, `SDIO` as data
  input/output for serial communication, and `DRDY` as the data-ready output.
- The serial interface is a 2-wire bidirectional read/write interface similar
  to and typically compatible with standard I2C; the NAU7802 can function only
  as a slave device.
- The datasheet states standard mode `SCLK` 0-100 kHz and fast mode `SCLK`
  0-400 kHz, with serial bus capacitance load required to be 400 pF or less.
- Digital serial-interface thresholds list input high voltage at `0.7 VDD`
  and input low voltage at `0.3 VDD`; `SCLK`, `SDIO`, and `DRDY` low output is
  specified at `0.2 DVDD` with 1 mA sink.
- The analog input pairs are `VIN1P/VIN1N` and `VIN2P/VIN2N`; the device can
  measure differential inputs and can also measure single-ended signals by
  grounding the negative input.

## Model Boundary

The CircuitCI model is deliberately reduced to a high-impedance board-observation
face for rail, reference, input-bias, I2C idle, and data-ready line-state
checks. It does not model ADC conversion codes, PGA gain, calibration, noise,
register protocol, I2C timing, streaming mode, bridge excitation accuracy,
crystal oscillator behavior, temperature sensor behavior, or final metrology
accuracy.
