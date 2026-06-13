# Component Model Contract

Each component model is a YAML document with a stable `component_id`, version, pin declarations, optional model faces, rules, and quality metadata.

## Minimal Model

```yaml
component_id: generic.mcu.basic
version: 0.1.0
category: mcu

ports:
  VDD:
    kind: electrical_power
    required: true
    electrical:
      operating_voltage_min_V: 2.7
      operating_voltage_max_V: 3.6
      max_supply_current_A: 0.03
  GND:
    kind: electrical_ground
    required: true
  RX:
    kind: digital_electrical_input
    required: false
    electrical:
      vih_min_V: 2.0
      vil_max_V: 0.8
      injection_current_limit_A: 0.0001

model_faces:
  electrical_pins:
    status: simple_behavioral

rules:
  - GPIO_BACKDRIVE

model_quality:
  source: generic
  confidence: low
  intended_use:
    - power_sequence
    - leakage
  not_valid_for:
    - rf
    - high_speed_signal_integrity
    - transistor_level_mcu_behavior
```

## Port Kinds

| Kind | Meaning |
| --- | --- |
| `electrical_power` | Supply input or source rail. |
| `electrical_ground` | Ground reference. |
| `digital_electrical_input` | Digital input with electrical limits. |
| `digital_electrical_output` | Driven output with voltage/current metadata. |
| `digital_electrical_io` | Bidirectional GPIO. |
| `passive` | Passive two-terminal behavior. |

## Electrical Metadata

Inputs should declare:

- `vih_min_V`
- `vil_max_V`
- `injection_current_limit_A`

Outputs should declare:

- `drive_high_voltage_V`
- `source_impedance_ohm`
- optional `powered_behavior`

Power ports should declare when known:

- `operating_voltage_min_V`
- `operating_voltage_max_V`
- `min_supply_current_A`
- `max_supply_current_A`

MCU, radio, and clock-consumer models can declare external crystal pins:

```yaml
clock_sources:
  - name: hse
    input_pin: OSC_IN
    output_pin: OSC_OUT
    stray_capacitance_F: 2.0e-12
```

Crystal and resonator models can declare the static load target:

```yaml
crystal:
  frequency_Hz: 8000000
  load_capacitance_F: 12.5e-12
  load_capacitance_tolerance_F: 2.5e-12
```

`CLOCK_SOURCE_VALID` checks that a crystal model is connected between the two
clock-source pins and that each pin has a Board IR capacitor to ground. It
computes effective load capacitance as `C1*C2/(C1+C2) + stray_capacitance_F`
and compares that to the crystal load target. This is a static support-network
screen, not oscillator startup or gain-margin sign-off.

Regulator and power-converter models may also declare static conversion
metadata:

```yaml
power_conversion:
  input_pin: VIN
  output_pin: VOUT
  switch_pin: SW
  dropout_voltage_V: 0.3
  min_output_current_A: 0.01
  max_output_current_A: 0.1
  startup_delay_us: 1000
  input_capacitance_min_F: 0.000001
  output_capacitance_min_F: 0.000001
  input_inductance_min_H: 0.00000037
  output_inductance_min_H: 0.0000022
  switch_inductor_pin_a: L1
  switch_inductor_pin_b: L2
  switch_inductance_min_H: 0.00000037
  switch_inductance_max_H: 0.00000057
```

- `input_pin` and `output_pin` must name model ports connected to Board IR power
  rails. They must be distinct `electrical_power` ports.
- `switch_pin` is optional unless input or output inductance limits are declared. When
  present, it must name a model port distinct from the input/output rails and
  be connected to the converter switch net in Board IR.
- `dropout_voltage_V` is a static nominal-voltage margin check:
  `V(input) - V(output)` must be at least this value.
- `min_output_current_A` checks the sum of declared `min_supply_current_A`
  always-on loads on the output rail.
- `max_output_current_A` checks the sum of declared `max_supply_current_A`
  loads on the output rail.
- `startup_delay_us` checks declared rail timing:
  `output.power_valid_at_us` must be no earlier than
  `input.power_valid_at_us + startup_delay_us`.
- `input_capacitance_min_F` and `output_capacitance_min_F` require explicit
  Board IR capacitor primitives from the corresponding rail to ground. The
  validator sums those capacitances. This is a schematic support-component
  screen, not an ESR/ESL/DC-bias or regulator stability sign-off.
- `input_inductance_min_H` and `input_inductance_max_H` require explicit
  Board IR inductor primitives directly between `input_pin` and `switch_pin`.
  This models boost-style energy-storage inductors.
- `output_inductance_min_H` and `output_inductance_max_H` require explicit
  Board IR inductor primitives directly between `switch_pin` and `output_pin`.
  This models buck-style output inductors. The validator sums direct inductors
  on each declared path. This is a static support-component screen, not
  saturation-current, DCR, ripple, or loop stability sign-off.
- `switch_inductor_pin_a`, `switch_inductor_pin_b`,
  `switch_inductance_min_H`, and `switch_inductance_max_H` require explicit
  Board IR inductor primitives directly between the two declared converter
  switch pins. This models buck-boost topologies such as TPS63802, where the
  energy-storage inductor is between two converter switch pins rather than
  input-to-switch or switch-to-output.

`POWER_TREE_VALID` uses these values to check that a component is connected to
a powered rail inside its allowed operating range, that declared rail current
budgets are not exceeded, and that explicitly modeled regulator dropout/output
current/startup timing/support-capacitance/support-inductance margins are plausible. Invalid
`power_conversion` metadata fails closed at validation time. Generic models may
use conservative screening values; datasheet-backed packs should cite their
source documents.

Load-switch and high-side/low-side switch models can declare static switch
metadata:

```yaml
power_switch:
  input_pin: VIN
  output_pin: VOUT
  control_pin: EN
  enabled_state: high
  max_output_current_A: 0.05
```

- `input_pin` and `output_pin` must name distinct `electrical_power` model
  ports.
- `control_pin` must name a `digital_electrical_input` or
  `digital_electrical_io` model port.
- `enabled_state` is `high` or `low` and must be proven by scenario
  `pin_states` when the output rail is declared powered. Scenario suggestions
  can fill that pin state from a direct rail/ground tie, or from exactly one
  positive-valued pull resistor from the control net to a direct rail/ground
  state matching `enabled_state`. Ambiguous dividers and active control nets
  remain explicit evidence inputs.
- `max_output_current_A` checks the sum of declared `max_supply_current_A`
  loads on the switched output rail.

This is a static topology/evidence check. It does not sign off inrush,
turn-on ramp, reverse current, switch SOA, or thermal behavior; those require
SPICE or a datasheet-backed transient/power-path model.

Reset supervisor models can declare static threshold metadata:

```yaml
reset_supervisor:
  monitored_pin: VDD
  reset_output_pin: RESET
  active: low
  threshold_min_V: 2.93
  threshold_max_V: 3.08
  reset_release_delay_us: 200000
```

- `monitored_pin` must name an `electrical_power` port connected to the
  supervised rail.
- `reset_output_pin` must name a `digital_electrical_output` or
  `digital_electrical_io` port connected to the reset net.
- `active` is `low` or `high`.
- `threshold_min_V` and `threshold_max_V` bound the worst-case supervisor
  release/assert threshold tolerance.
- `reset_release_delay_us` is optional static delay metadata for reset timing
  scenarios.

`POWER_TREE_VALID` checks that the monitored rail nominal voltage is above the
worst-case threshold maximum, and that the worst-case threshold minimum is not
below the minimum operating voltage of powered loads on the monitored rail.
This is a static threshold-screening rule; it does not model output topology,
pull-up RC shape, noise immunity, comparator hysteresis, or transient
oscillation around threshold.

Battery charger models can declare static charge-current metadata:

```yaml
battery_charger:
  input_pin: VDD
  battery_pin: VBAT
  charge_current_parameter: programmed_charge_current_A
  charge_current_programming:
    programming_pin: PROG
    reference_pin: VSS
    current_gain_V: 1000.0
    source: MCP73831 DS20001984H section 5.1.2
  min_charge_current_A: 0.015
  max_charge_current_A: 0.5
  regulation_voltage_V: 4.2
```

- `input_pin` and `battery_pin` must name distinct `electrical_power` model
  ports.
- `charge_current_parameter` names a Board IR component instance parameter.
  For resistor-programmed chargers, agents should derive this value from the
  schematic programming resistor or board configuration.
- `charge_current_programming`, when present, allows static inference from one
  positive resistor between `programming_pin` and `reference_pin` using
  `current_A = current_gain_V / resistor_ohm`. This is only for source-backed
  linear programming equations such as `PROG`/`ISET` resistor chargers; multiple
  matching resistors or missing resistor evidence fail closed.
- `min_charge_current_A` and `max_charge_current_A` bound the programmed charge
  current.
- `regulation_voltage_V` bounds the battery net nominal voltage for the modeled
  charger option.

Example component instance:

```yaml
components:
  UCHG:
    model: vendor.microchip.mcp73831_4v2
    parameters:
      programmed_charge_current_A: 0.1
```

This is a static input-budget/range check. It does not sign off battery
chemistry, thermal foldback, charge termination, USB negotiation, or transient
load sharing.

Power mux and ideal-diode models can declare static source-selection metadata:

```yaml
power_mux:
  output_pin: SYS
  selected_input_parameter: selected_input
  max_output_current_A: 1.0
  inputs:
    - name: usb
      input_pin: USB_IN
      reverse_blocking: true
    - name: battery
      input_pin: BAT_IN
      reverse_blocking: true
```

- `output_pin` and each `input_pin` must name distinct `electrical_power`
  model ports.
- `selected_input_parameter` names a Board IR component instance parameter that
  identifies the active source in this scenario. When that parameter is absent,
  CircuitCI may derive the selected source only for a static board state where
  the mux output rail is declared powered and every declared mux input rail has
  explicit powered/unpowered metadata with exactly one powered input.
- `inputs[].name` values are the allowed source-selection strings.
- `reverse_blocking: true` means the model claims that a powered output rail
  will not backfeed that input when the input is inactive and unpowered.
- `max_output_current_A` checks the sum of declared `max_supply_current_A`
  loads on the mux output rail.

Example component instance:

```yaml
components:
  UMUX:
    model: generic.analog.power_mux_basic
    parameters:
      selected_input: usb
```

This is a static topology/evidence check. It does not quantify reverse current,
switchover timing, inrush, body-diode conduction, thermal margin, or transient
source sharing.

## Signal Conditioning Metadata

Interface, protection, and level-shifter models can declare explicit
board-facing channels:

```yaml
signal_conditioning:
  supply_constraints:
    - name: vcca_lte_vccb
      relation: less_than_or_equal
      lower_supply_pin: VCCA
      upper_supply_pin: VCCB
  channels:
    - name: ch1
      kind: level_shifter
      side_a_pin: A1
      side_b_pin: B1
      side_a_supply_pin: VCCA
      side_b_supply_pin: VCCB
      direction: bidirectional
      unpowered_isolation: false
      enable_pin: OE
      disabled_state: low
  protection_clamps:
    - name: dp
      protected_pin: DP
      reference_pin: GND
      reference: ground
      working_voltage_max_V: 5.5
      line_capacitance_F: 1.0e-12
```

- `kind` is one of `level_shifter`, `protection`, `series_resistor`, or
  `bus_switch`.
- `side_a_pin` and `side_b_pin` name the protected or translated signal pins.
- `side_a_supply_pin` and `side_b_supply_pin` identify the rails that define
  each side's voltage domain when applicable.
- `direction` is `a_to_b`, `b_to_a`, or `bidirectional`.
- `unpowered_isolation` records whether the datasheet guarantees isolation when
  one side's supply is absent.
- `enable_pin` and `disabled_state` optionally record the channel-control pin
  and the logic state that disables the channel.
- `supply_constraints` records datasheet supply-order rules. The first
  supported relation is `less_than_or_equal`, which requires the powered rail on
  `lower_supply_pin` to have nominal voltage no greater than the powered rail on
  `upper_supply_pin`.
- `protection_clamps` records clamp-only protection paths such as USB ESD
  arrays. `protected_pin` names the signal being protected. `reference_pin`
  must connect to the declared `reference` kind, currently `ground` or `power`.
  `working_voltage_max_V` is the maximum normal protected-net voltage the clamp
  may see. `line_capacitance_F` records the modeled capacitance added to that
  interface line.

## USB Connector Metadata

USB connector models can declare board-facing connector pins so validation can
check whether common protection coverage exists:

```yaml
usb_connector:
  standard: usb2
  vbus_pin: VBUS
  dp_pin: D+
  dm_pin: D-
  gnd_pin: GND
  shield_pin: SHIELD
  entry_direction_offset_deg: 0.0
  entry_clearance_depth_mm: 8.0
  entry_clearance_width_mm: 6.0
  entry_aperture_front_offset_mm: 0.0
  entry_aperture_lateral_offset_mm: 0.0
  entry_aperture_width_mm: 6.0
```

`USB_CONNECTOR_PROTECTION_VALID` uses this metadata to locate the connector's
D+, D-, and optional VBUS nets, then searches connected clamp-only protection
models for matching protection paths. This is connector-level schematic
coverage; it does not prove placement, trace routing, differential impedance,
ESD pulse energy handling, or USB signal integrity.

`entry_direction_offset_deg` is optional mechanical metadata for cable-entry
checks. When declared, `USB_CONNECTOR_ENTRY_CLEARANCE_VALID` and
`suggest-scenarios` compute the default cable insertion direction as imported
placement `rotation_deg + entry_direction_offset_deg`, normalized into
`[0, 360)`. KiCad schematic mapping metadata can override this model default
per footprint/library convention through `layout.entry_direction_offset_deg`;
that path is reported as `kicad_mapping_offset`. Explicit KiCad PCB footprint
property `CircuitCI_EntryDirectionOffsetDeg` can override both model and
mapping defaults and is reported as `footprint_property_offset`. Omit the model
value only when the footprint's zero-degree convention already points in the
cable insertion direction or when every supported KiCad footprint or mapping
supplies the offset.
See [usb_connector_entry_offset_fixture.md](usb_connector_entry_offset_fixture.md)
for a validation fixture that proves a nonzero offset changes the checked entry
direction.

`entry_clearance_depth_mm` and `entry_clearance_width_mm` are optional 2D
cable-entry corridor policy hints. When present, `suggest-scenarios` uses them
to prefill `parameters.min_cable_entry_clearance_depth_mm` and
`parameters.cable_entry_clearance_width_mm`. When both values are present, the
entry-clearance suggestion is runnable; when either value is missing, the
template remains non-runnable and records the missing mechanical policy input.
KiCad schematic mapping `layout.entry_clearance_depth_mm` and
`layout.entry_clearance_width_mm` are reported as `kicad_mapping_depth` and
`kicad_mapping_width`. Explicit KiCad PCB footprint properties
`CircuitCI_EntryClearanceDepthMM` and `CircuitCI_EntryClearanceWidthMM` are
reported as `footprint_property_depth` and `footprint_property_width`.
Footprint properties take precedence over mapping metadata, and both take
precedence over component-model defaults. Values are millimeters and must be
greater than zero.

`entry_aperture_front_offset_mm`, `entry_aperture_lateral_offset_mm`, and
`entry_aperture_width_mm` are optional 2D cable-entry aperture metadata. The
front offset shifts the checked corridor start forward from the imported
footprint body front in the cable-entry direction. The lateral offset shifts
the corridor centerline along the axis perpendicular to cable entry. The
aperture width is used as a model-derived minimum checked corridor width when
it is larger than the scenario's `cable_entry_clearance_width_mm`. Omit these
fields when the connector placement center and footprint front are the best
available 2D entry approximation.
Design-specific aperture metadata can override these model defaults. KiCad
schematic mapping `layout.entry_aperture` is reported as
`kicad_mapping_aperture`, and explicit KiCad PCB footprint properties named
`CircuitCI_EntryAperture*` are reported as `footprint_property_aperture`.
Footprint properties take precedence over mapping metadata, and both take
precedence over component-model defaults.
See [usb_connector_entry_aperture_fixture.md](usb_connector_entry_aperture_fixture.md)
for a validation fixture that proves aperture metadata changes the checked
entry corridor.

`circuitci suggest-scenarios` uses `signal_conditioning` metadata to emit
`interface_protection` review templates. Generic or incomplete channel metadata
stays non-runnable. A channel template becomes runnable only when a non-generic
datasheet-backed model supplies complete direction, supply pin, rail powered
state, supply-constraint, and unpowered-isolation metadata. A runnable template
still does not prove that a level shifter prevents backdrive; it makes the
static `INTERFACE_PROTECTION_REVIEW` executable so the rule can pass or fail
from the modeled datasheet facts and any direct OE/reset pin-state evidence.
`INTERFACE_PROTECTION_REVIEW` accepts a powered-to-unpowered channel only when
the model declares `unpowered_isolation: true`, or when the scenario observes
the declared `enable_pin` in its `disabled_state`. It also checks declared
`supply_constraints` whenever both constrained rails are powered. For
`protection_clamps`, `INTERFACE_PROTECTION_REVIEW` checks the reference-net
kind, optional reverse-standoff limit, and optional line-capacitance budget.

The first back-drive approximation computes injection current as:

```text
max(0, driver_high_voltage - victim_power_voltage - diode_drop) / source_resistance
```

The behavioral `GPIO_BACKDRIVE` rule defaults diode drop to `0.3 V` and
combines the output source impedance with any scenario-declared series
resistance. Physical voltage/current proof belongs in `analog_transient`
scenarios, where generated or file-backed SPICE decks provide waveform evidence
and generated semiconductor models can be checked against datasheet operating
limits.

`IO_VOLTAGE_COMPATIBLE` uses the same model fields without requiring explicit
scenario `paths`. On a `power_tree` scenario, it scans same-net digital
output/input pairs and:

- fails when `drive_high_voltage_V < vih_min_V`,
- estimates receiver clamp current as
  `max(0, drive_high_voltage_V - receiver_rail_voltage_V - diode_drop_V) /
  source_impedance_ohm`,
- fails when that estimate exceeds `injection_current_limit_A`.

The rule skips pairs that lack the relevant metadata; it is a static board-level
screen, not a replacement for analog waveform proof. When imported KiCad
`source.board_pin_electrical_types` exists, the scan also requires the imported
pin type to allow the candidate driver or receiver direction.

## GPIO_BACKDRIVE Rule

Normative first-slice behavior:

- Rule ID: `GPIO_BACKDRIVE`.
- Severity: `critical` when measured current is greater than the victim limit.
- Comparison: fail iff `injection_current_A > injection_current_limit_A`.
- Default diode drop: `0.3 V`, overridable by scenario `parameters.diode_drop_V`.
- Missing output `source_impedance_ohm`: binding warning and skip that path.
- Missing output `drive_high_voltage_V`: binding warning and skip that path.
- Missing input `injection_current_limit_A`: binding warning and skip that path.
- `digital_electrical_io` direction comes from scenario `pin_states`.
- Victim rail voltage follows Board IR power semantics.

Formula:

```text
effective_resistance = driver.source_impedance_ohm + path.series_resistance_ohm
injection_current_A =
  max(0, driver.drive_high_voltage_V - victim_rail_voltage_V - diode_drop_V)
  / effective_resistance
```

`effective_resistance <= 0` is invalid model/scenario data and must produce a warning finding instead of division by zero.

## Reset/Boot Model Metadata

MCU-like models can declare reset and boot behavior without making the engine chip-specific:

```yaml
behavior:
  reset:
    pin: NRST
    active: low
    min_assert_us: 20
  boot:
    sample_time_after_reset_release_us: 100
    modes:
      bootloader:
        straps:
          - pin: BOOT0
            required_state: high
      application:
        straps:
          - pin: BOOT0
            required_state: low
  bootloader:
    interfaces:
      uart:
        rx_pin: RX
        tx_pin: TX
        sync_byte: 0x7F
        ack_byte: 0x79
```

This metadata can represent STM32-like boot flows, ESP32-like EN/IO0 flows, STM8/C51/STC serial entry flows, or simpler generic boot selectors. Vendor packs provide concrete values; the validation engine reads only the generic contract.

MCU models should remain functional black boxes. A stronger MCU model may run
firmware and expose correct peripheral/pin behavior, reset causes, boot-ROM
entry, pin modes, thresholds, clamp/leakage limits, and timing at the board
boundary. It should not attempt transistor-level modeling of the MCU core or
internal silicon implementation.

## Resident Protocol Metadata

Firmware-specific models can declare resident update protocols without changing the engine:

```yaml
behavior:
  protocols:
    resident_update:
      transport_interface: uart
      frame:
        magic: [85, 77, 66, 76]
        version: 1
        request_type: 1
        response_type: 2
        crc: crc32_ieee
        max_payload_len: 1030
        ok_result: 0
      operations:
        begin:
          opcode: 2
          role: start_transfer
          payload:
            min_len: 36
            max_len: 37
        data:
          opcode: 3
          role: data_chunk
          payload:
            overhead_len: 6
        finish:
          opcode: 4
          role: finish_transfer
          payload:
            len: 36
      flows:
        upload:
          phases:
            - operation: begin
            - operation: data
              repeat: one_or_more
            - operation: finish
```

Operation names are model-local. Generic validation keys off operation metadata such as `role`, payload limits, and flow phases, not chip or protocol names.

## Reset/Boot Rules

`RESET_RELEASE_AFTER_POWER_VALID`:

- Severity: `critical`.
- Prefer target rail `power_valid_at_us` over duplicated scenario timing when
  the target power rail declares it.
- Fail if duplicated scenario `power_valid_at_us` conflicts with target rail
  timing.
- Fail iff `reset_release_at_us < power_valid_at_us + reset_release_delay_us`.
- Missing target or timing fields produce critical `VALIDATION_INPUT_MISSING` findings.

`BOOT_STRAP_DEFINED`:

- Severity: `critical`.
- Resolve required straps from `behavior.boot.modes[scenario.required_boot_mode]`.
- Fail iff a required strap observation is missing, `floating`, `undefined`, or does not equal the model-required state.
- Required and actual values are compared as lowercase symbolic states.

`BOOT_STRAP_BIAS_VALID`:

- Severity: `critical`.
- Resolve required straps from `behavior.boot.modes[scenario.required_boot_mode]`.
- Resolve each strap pin to a board net and compute its DC voltage from explicit
  `spice.primitive: resistor` components connected to declared power or ground
  nets.
- Fail iff the network is floating, a required high strap is below `vih_min_V`,
  a required low strap is above `vil_max_V`, or the optional
  `parameters.max_strap_bias_current_A` limit is exceeded.
- Missing strap pin thresholds, resistor values, or power-net metadata produce
  critical `VALIDATION_INPUT_MISSING` findings.

`UART_BOOTLOADER_SYNC`:

- Severity: `critical`.
- Fail if the model lacks the requested bootloader interface.
- Fail if scenario-declared sync/ACK bytes conflict with the model.
- Fail if no scenario event sends exactly `[sync_byte]` to the model's bootloader RX pin.
- Fail if the event sender is missing, unresolved, not output-capable, or not connected to the target RX net.
- Fail if the event target is not the target component and model RX pin.
- Fail if event time is before `boot_sample_at_us` when that timing is declared.
- Pass/fail is abstract protocol behavior, not full firmware execution.

`RESIDENT_BOOTLOADER_UPDATE_SEQUENCE`:

- Severity: `critical`.
- Resolve the named protocol from `behavior.protocols`.
- Fail if the protocol sender does not resolve to an output-capable pin on the target RX net.
- Fail if operation order does not match the named flow phases.
- Fail if any event result code differs from `frame.ok_result`.
- Fail if payload lengths exceed `frame.max_payload_len` or operation payload limits.
- Fail if `data_chunk` roles do not cover the declared package size exactly.
- Pass/fail is abstract trace validation, not full firmware execution or flash emulation.

`CONTROL_LINE_RELEASE_SEQUENCE`:

- Severity: `critical`.
- Uses scenario `control_effects`; no component-model protocol extension is required.
- Fail if a control source is unresolved, unconnected, or not output-capable.
- Fail if a control target is unresolved, unconnected, not input-capable, or not on the target component.
- Fail if an evaluated effect has no explicit prior `control_line` event.
- Fail if derived reset state is not released at reset release or boot sample time.
- Fail if derived boot strap states do not match the required boot mode.
- Pass/fail is abstract line-effect timing, not transistor-level or RC waveform simulation.

## Quality Policy

Every model must declare model quality. Reports emit `LOW_CONFIDENCE_MODEL` limitations for `generic`, `estimated`, or `low` confidence models so users do not over-trust behavioral library metadata.
