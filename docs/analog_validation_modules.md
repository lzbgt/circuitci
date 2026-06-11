# Analog Validation Module Split

CircuitCI keeps SPICE-class analog validation in focused Rust modules so the
engine can grow without returning to a monolithic validator.

## Boundaries

- `src/validation/analog_spice.rs` owns orchestration and validation order:
  scenario preflight, model artifact checks, Board IR netlist preparation,
  backend selection, solver invocation, and report assembly.
- `src/validation/analog_runner.rs` owns external solver execution details:
  ngspice wrapper generation, include rewriting, timeout handling, solver log
  failure detection, and waveform CSV parsing.
- `src/validation/analog_assertions.rs` owns user-declared waveform contracts:
  probe expression validation, assertion contract validation, interpolation,
  window aggregation, and `SPICE_TRANSIENT_ANALYSIS` assertion findings.
- `src/validation/analog_operating_limits.rs` owns automatic datasheet-derived
  semiconductor stress checks for MOSFETs, BJTs, diodes, temperature derating,
  scalar pulse qualifier checks, shared operating-limit probe metadata, and
  scalar `SPICE_OPERATING_LIMIT` findings.
- `src/validation/analog_soa.rs` owns digitized MOSFET safe-operating-area
  metadata validation, VDS/ID envelope evaluation, log-log interpolation, and
  SOA-shaped `SPICE_OPERATING_LIMIT` findings.
- `src/validation/analog_util.rs` owns shared filesystem and artifact helpers.

## Preserved Contracts

Generated Board IR decks are still prepared before backend selection. Missing
MOSFET/BJT/diode absolute-maximum, required derating, or requested pulse
metadata still fails with
`SPICE_OPERATING_LIMIT` after `generated_board.cir` is produced but before
wrapper/log/waveform solver artifacts exist.

Wrapper waveform columns remain ordered as user probes first, then automatic
operating-limit probes. `NgspiceRun.user_probe_count` is the boundary that lets
the assertion evaluator and operating-limit evaluator read their respective
columns without changing report behavior.

Generated semiconductor current probes use the same `current_sense_name`
function as generated netlist emission. This keeps operating-limit expressions
such as `I(VCCI_M1)` aligned with the zero-volt current-sense sources inserted
into the generated SPICE deck.
