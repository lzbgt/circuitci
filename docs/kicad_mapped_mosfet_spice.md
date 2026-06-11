# KiCad-Mapped MOSFET SPICE Scenarios

## Purpose

KiCad XML import can already preserve schematic connectivity and generate
Board-IR SPICE scenarios from explicit mapping files. MOSFET scenarios need one
more contract: importer mappings must be able to pass through solver operating
conditions and datasheet-backed model files so imported schematic connectivity
can exercise the same operating-limit and SOA validators as hand-authored
Board IR.

## Contract

The importer remains fail-closed and does not infer physics from KiCad symbol
names or displayed values.

- A mapped MOSFET scenario must list every generated component explicitly.
- Components with primitive behavior, such as resistors or voltage sources,
  must declare mapping-file `spice` metadata.
- Components whose selected model has `simulation.spice` metadata must declare
  a matching SHA-pinned `model_files` entry in the scenario.
- MOSFET body behavior is still owned by the component model. If the board does
  not bind a `B` pin, generated SPICE requires
  `simulation.spice.body_pin_policy: tie_to_source_when_absent`.
- Scenario `operating_conditions` is copied into generated Board IR only when
  explicitly declared in the mapping file. This is required for pulse/SOA
  ratings and keeps pulse use auditable.
- Runtime still emits `SCHEMATIC_IMPORT_ONLY` for KiCad XML imports. A passing
  explicit scenario is scenario evidence, not whole-board sign-off.

## Review Notes

The main risk is accidental sign-off by omission. The implementation must not:

- auto-enable pulse ratings,
- infer MOSFET body ties from missing symbol pins,
- accept unpinned model files,
- drop operating-limit/SOA evidence from imported scenarios,
- or remove schematic-import limitations when a scenario passes.

The fixture for this slice imports a KiCad XML low-side MOSFET stress circuit,
maps it to the datasheet-backed FDMC86184 model, enables pulse ratings
explicitly, and validates that generated SPICE reports a MOSFET SOA violation
with waveform/model artifacts.
