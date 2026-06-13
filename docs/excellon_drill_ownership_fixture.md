# Excellon Drill Ownership Fixture

`examples/import_excellon_drill_ownership/` proves that
`import-excellon-drill` can annotate fabrication drill hits with existing PCB
layout ownership evidence.

- The first imported hit matches drilled pad `J1.1` on net `GND`.
- The second imported hit matches route via index `0` on net `USB_DP`.
- The annular-ring scenario intentionally fails the `J1.1` pad hit because the
  Gerber copper flash leaves only `0.1 mm` annular ring against a `0.15 mm`
  limit.

This fixture is intentionally importer-backed: the drill file is plain
Excellon evidence, while pad/via ownership comes from the input Board IR layout
evidence. Ambiguous or missing matches remain anonymous.
