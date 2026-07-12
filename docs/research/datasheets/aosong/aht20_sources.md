# Aosong AHT20 Source Notes

## Retained Sources

| Source | URL | Local copy | SHA-256 |
| --- | --- | --- | --- |
| AHT20 Humidity and Temperature Sensor Data Sheet V1.0 | <https://www.aosong.com/userfiles/files/media/Data%20Sheet%20AHT20.pdf> | `docs/research/datasheets/aosong/aht20_datasheet_v1_0_2021.pdf` | `1e460b777986e711f301eb468265603ea6242ab44735b2686801d78d9779079e` |

The PDF was retrieved from Aosong and retained verbatim. Text extraction is
kept at `docs/research/datasheets/aosong/aht20_datasheet_v1_0_2021.txt` for
agent-side review.

## Board-Level Facts Used

- AHT20 is a calibrated humidity and temperature sensor with standard I2C
  digital output.
- The sensor package is a 3 mm x 3 mm bottom-face SMD package with 1.0 mm
  height.
- The host-to-sensor power supply voltage range is 2.2 V to 5.5 V.
- The power pin section states the sensor supply range is 2.2 V to 5.5 V and
  requires a 10 uF decoupling capacitor between `VDD` and `GND`.
- Table 3 lists 980 uA typical measure current and 250 nA maximum dormant
  current at VCC = 3.3 V and T < 60 C, but it does not provide a maximum
  measurement current. The model therefore does not claim
  `max_supply_current_A`.
- The interface pins are `SCL` serial clock and `SDA` serial data. `SCL` has no
  minimum frequency because the interface contains static logic.
- The typical application notes say `SCL` and `SDA` pull-up voltage must be
  powered by `VDD`, and external pull-ups such as 2.0 kOhm to 4.7 kOhm lift
  the signals high.
- The datasheet recommends I2C frequency between 10 kHz and 400 kHz during
  measurement.
- Digital I/O thresholds are `VIH` at 70% of `VDD` to `VDD` and `VIL` at 0 to
  30% of `VDD`; `VOL` is up to 0.4 V under the listed sink-current condition.
- After power-on, the sensor needs at least 100 ms with `SCL` high before it is
  ready to receive host commands.
- The I2C device address is `0x38`.

## Model Boundary

The CircuitCI model is deliberately reduced to a high-impedance board-observation
face for VDD and idle I2C pull-up checks. It does not model humidity or
temperature conversion accuracy, calibration state, command protocol,
measurement timing, power-on readiness timing, self-heating, contamination,
recovery processing, solder/reflow drift, or final I2C signal-integrity timing.
