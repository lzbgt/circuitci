# KiCad Pin Electrical Type Metadata

## Purpose

KiCad symbols and generic XML netlists carry pin electrical types such as
`input`, `output`, `bidirectional`, `passive`, and `power_in`. CircuitCI imports
must preserve those facts because they are useful evidence for later
directionality checks, especially when imported schematics feed control-line,
protocol, or backdrive validation.

## Source Basis

The saved KiCad documentation under `docs/research/kicad/` documents:

- native schematic library pins as `(pin PIN_ELECTRICAL_TYPE ...)`,
- generic XML netlist nodes with optional `pintype` attributes.

## Import Contract

Imported components now serialize two source metadata maps when electrical type
evidence exists:

- `source.kicad_pin_electrical_types`: raw KiCad pin number to KiCad electrical
  type,
- `source.board_pin_electrical_types`: mapped Board IR pin name to KiCad
  electrical type.

Native `.kicad_sch` imports populate the raw map from `lib_symbols` pin
geometry and the Board IR map after KiCad mapping resolves pin aliases.
KiCad XML imports populate both maps from `node pintype="..."` attributes.

Conflicting electrical types for the same raw KiCad pin fail closed.

## Validation Use

Several validators consume `source.board_pin_electrical_types` when it is
present:

- control-line sources, UART/protocol senders, and backdrive drivers must be
  KiCad `output`, `bidirectional`, `tri_state`,
  `power_out`, `open_collector`, or `open_emitter`,
- control-line targets, UART target RX pins, and backdrive victims must be
  KiCad `input`, `bidirectional`, or `tri_state`,
- `IO_VOLTAGE_COMPATIBLE` scans only model output/input pairs whose imported
  KiCad pin types also allow the scanned direction.

The check is additive. Component models still have to declare output-capable
source/driver/sender ports and input-capable target/victim ports. KiCad
metadata cannot upgrade a bad component model; it can only fail closed when
imported schematic direction evidence contradicts the required electrical
direction. These contradictions are critical validation failures for
control-line, UART/protocol, and backdrive checks because those scenarios
declare explicit endpoints. For scan-based I/O voltage compatibility, imported
metadata constrains candidate driver/receiver roles so an input-only schematic
pin is not treated as a possible output driver just because the generic model
port is bidirectional or otherwise output-capable.
