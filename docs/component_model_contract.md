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
  load_capacitance_F: 18.0e-12
```

`CLOCK_SOURCE_VALID` checks that a crystal model is connected between the two
clock-source pins and that each pin has a Board IR capacitor to ground. It
computes effective load capacitance as `C1*C2/(C1+C2) + stray_capacitance_F`
and compares that to the crystal load target. This is a static support-network
screen, not oscillator startup or gain-margin sign-off.

Motor and actuator load-envelope models can declare current evidence used by
`motor_drive` scenarios:

```yaml
motor_load:
  supply_voltage_min_V: 6.0
  supply_voltage_nominal_V: 12.0
  supply_voltage_max_V: 12.6
  phase_peak_current_A: 10.0
  phase_rms_current_A: 6.0
  max_regen_current_A: 6.0
  source: docs/research/smart_robot/design_review.md
```

These fields are load-budget evidence, not a dynamic motor model. Supply
voltage fields bound the motor's allowed bus range for
`MOTOR_LOAD_SUPPLY_VALID`; current fields feed bridge, route, current-sense,
SOA, and regeneration screens. Use datasheet- or measurement-backed values for
a selected motor before treating the actuator bridge as fabrication-ready.

Regeneration clamp or absorber models can declare first-pass single-event
current and energy evidence used by `MOTOR_REGEN_CLAMP_VALID`:

```yaml
regen_absorber:
  clamp_current_rating_A: 10.0
  clamp_energy_rating_J: 1.5
  source: docs/research/smart_robot/sources/vishay_rh_nh_aluminum_housed_resistors.pdf
```

These fields are selected-part evidence for the static clamp screen. They are
not repeated-pulse thermal, enclosure heat-sinking, or firmware regeneration
control proof unless the source explicitly covers those conditions.

Motor bridge models can declare first-pass loss and rating evidence used by
`MOTOR_BRIDGE_LOSS_THERMAL_VALID`:

```yaml
motor_bridge:
  voltage_rating_V: 60.0
  current_rating_A: 40.0
  reference_loss_W: 3.0
  reference_current_A: 30.0
  reference_loss_scope: per_half_bridge
  switching_devices: 3
  gate_charge_total_C: 0.000000056
  gate_charge_voltage_V: 10.0
  rise_time_s: 0.000000020
  fall_time_s: 0.000000003
  source: docs/research/smart_robot/sources/csd88599q5dc_product.html
```

`reference_loss_scope` is either `per_half_bridge` or `three_phase_bridge`.
When it is `per_half_bridge`, `switching_devices` is required so the validator
can scale the total bridge loss. This is a static source-backed reference-loss
screen. Optional `gate_charge_total_C`, `gate_charge_voltage_V`,
`rise_time_s`, and `fall_time_s` feed static switching-budget checks when they
are source-backed. These fields are not a replacement for MOSFET SOA curves,
switching waveform simulation, peak gate-current timing, thermal impedance, or
measured board temperature.

Motor bridge SOA checks can consume either
`motor_bridge.system_soa.output_current_temperature_curves` for power-block
datasheets that publish output-current versus board/case temperature limits, or
`datasheet.safe_operating_area.vds_id_curves`, the same source-backed curve
metadata used by analog transient SOA checks. A bridge model without either
curve family can still support reference-loss and switching screens, but any
`MOTOR_BRIDGE_SOA_VALID` scenario must fail closed until sourced SOA points or
measured waveform/thermal evidence is available.

Connector models can declare static electrical ratings used by `load_budget`
scenarios:

```yaml
connector:
  current_rating_A: 3.0
  voltage_rating_V: 250.0
  source: docs/research/smart_robot/sources/jst_xh_connector_datasheet.pdf
```

`LOAD_CONNECTOR_CURRENT_VALID` uses this metadata when a scenario declares
`parameters.connector_component`. Scenario parameters
`connector_current_rating_A` and `connector_voltage_rating_V` can override the
model for a board-specific cable or derated assembly. This is static connector
budget evidence; temperature rise, crimp quality, wire gauge, vibration, and
pulsed-load behavior need separate validation evidence.

Connector models can also declare a reduced generated-SPICE face when the model
has explicit pass-through pins. The JST XH/VH connector packs use board-side
pins plus matching load-side pins, for example `VCC`/`VCC_LOAD` or
`VBAT`/`VBAT_LOAD`, and model each mated contact as the datasheet 20 mOhm
post-test/environment maximum. This supports contact voltage-drop observation
only; it does not replace cable resistance, crimp, temperature-rise, retention,
vibration, signal-integrity, or harness qualification evidence.

Cable or harness assemblies can declare separate static ratings:

```yaml
cable_assembly:
  current_rating_A: 8.0
  voltage_rating_V: 30.0
  loop_resistance_ohm: 0.02
  max_voltage_drop_V: 0.3
  max_power_loss_W: 2.0
  temperature_rise_test_current_A: 8.0
  temperature_rise_at_test_current_C: 18.0
  max_temperature_rise_C: 30.0
  source: cable assembly datasheet or measured harness qualification
```

`LOAD_CABLE_CURRENT_VALID` uses this metadata when a scenario declares
`parameters.cable_component`. Scenario parameters `cable_current_rating_A` and
`cable_voltage_rating_V` can also provide explicit board-specific evidence.
`LOAD_CABLE_THERMAL_DERATING_VALID` uses the temperature-rise test point and
maximum allowed rise when a scenario declares `parameters.cable_component`, or
equivalent explicit scenario parameters. The validator scales temperature rise
by I^2 from the declared test current; it does not derive wire ampacity from
AWG tables or generic installation assumptions.
`LOAD_CABLE_VOLTAGE_DROP_VALID` uses loop resistance and optional voltage-drop
or power-loss limits from the cable model when present, or equivalent explicit
scenario parameters. The validator computes DC drop and loss only; it does not
model PWM ripple, contact aging, temperature-dependent resistance, or shared
return paths.
This is intentionally separate from connector metadata because a connector
header rating does not prove wire gauge, crimp, harness routing, or cable
temperature rise.

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
  current_limit_A: 0.08
  on_resistance_ohm: 0.050
  thermal_resistance_junction_to_ambient_C_per_W: 80.0
  max_junction_temperature_C: 150.0
  reverse_current_blocking: true
  reverse_current_blocking_mode: always
  max_inrush_current_A: 1.0
  soft_start_time_us: 1000.0
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
- `current_limit_A`, `on_resistance_ohm`,
  `thermal_resistance_junction_to_ambient_C_per_W`, and
  `max_junction_temperature_C` are used by `POWER_SWITCH_BUDGET_VALID` for
  selected eFuse/load-switch/MOSFET-path sign-off gates.
- `reverse_current_blocking_mode` is used by
  `POWER_SWITCH_REVERSE_CURRENT_VALID` and can be `always`,
  `when_disabled`, or `none`. Legacy `reverse_current_blocking: true` is
  interpreted as `always`; `false` is interpreted as `none`.
- `max_inrush_current_A` and `soft_start_time_us` are used by
  `POWER_SWITCH_INRUSH_VALID` together with scenario-declared switched
  capacitance.

This is a static topology and conduction-budget check. It does not sign off
inrush, turn-on ramp, reverse current, switch SOA, current-limit waveform,
repeated surge, or PCB copper temperature; those require SPICE, measurement,
or a datasheet-backed transient/power-path model.

Component models can declare source-backed package thermal screening metadata
for manufacturing scenarios:

```yaml
thermal_package:
  thermal_resistance_junction_to_ambient_C_per_W: 38.0
  max_junction_temperature_C: 125.0
  source: datasheet_package_table_rev_a
```

`THERMAL_PACKAGE_TEMPERATURE_VALID` combines this metadata with reviewed
`board.manufacturing.thermal_copper[].power_loss_w` and scenario-declared
ambient/temperature-rise limits to run a static package temperature screen. The
metadata must identify the source and package/board assumption well enough for
review. It is not a transient thermal model, board spreading solver, airflow
model, enclosure model, or substitute for measured temperature evidence.

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

`suggest-scenarios` may use `reset_release_delay_us` to make
`RESET_RELEASE_AFTER_POWER_VALID` or UART bootloader timing suggestions runnable
only when the supervisor model is datasheet-backed, not low-confidence, uniquely
matches the target reset net, and monitors the same rail as the target power
pin. Generic reset-supervisor models still document topology and threshold
evidence for power-tree suggestions, but do not become standalone reset timing
proof.

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
- Single-line power-entry clamps use the same contract. For example,
  `vendor.nexperia.pesd5v0s1ul` declares cathode `K` on protected VBUS and
  anode `A` on ground, with `working_voltage_max_V: 5.0` and
  `line_capacitance_F: 200.0e-12`. This proves static schematic coverage,
  standoff, and capacitance only; surge heating, USB inrush, and ESD waveform
  sign-off require separate evidence.

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

## Generic Behavioral Analog Macro-Models

Reusable preliminary analog models should be normal component models with
`simulation.spice` metadata, not special-case simulator code. The generic
analog library includes low-confidence behavioral subcircuits for:

- `generic.analog.ideal_opamp`
- `generic.analog.ideal_comparator`
- `generic.analog.ideal_ldo_3v3`

Each declares an explicit `pin_order` and points at
`models/spice/generic/analog_behavioral.lib`. They also reference the built-in
compact-model package registry entry `generic_analog_behavioral_spice`, backed
by `models/packages/generic/analog_behavioral.lock.json` and
`models/packages/compact_model_registry.json`. Generated Board IR scenarios must
still list that file in `analog.model_files` with its SHA-256 hash and package
pins, so reports show exactly which reusable macro-model artifact was used.
These models are useful for
topology, waveform, sweep, and GUI workflow checks. They are not valid for
vendor-part sign-off, op-amp stability/noise/slew/current analysis,
comparator propagation-delay/hysteresis/open-drain sign-off, or regulator
loop-stability/current-limit/thermal/PSRR analysis.

Analog function models can also declare workflow semantics used by GUI
observation presets:

```yaml
analog_function:
  kind: op_amp
  positive_input_pin: INP
  negative_input_pin: INN
  output_pin: OUT
  positive_supply_pin: VCC
  negative_supply_pin: VEE
  default_output_tolerance_V: 0.05
```

`kind` is currently `op_amp` or `comparator`. The pin fields identify analog
function roles independently from SPICE subcircuit pin order. Observation
presets use this metadata only when the surrounding circuit is inferable. For
example, an op-amp whose inverting input is tied to the output and whose
non-inverting input is driven by a Board IR pulse source receives output
tracking sample checks. A comparator whose one input is pulse-driven and the
other is a fixed DC/reference net receives output low/high state checks against
the positive supply rail. These checks are starter observations, not vendor
dynamic-performance sign-off.

Vendor component models may also point at these shared behavioral subcircuits
when the datasheet-backed package/pin/limit metadata is still explicit and the
simulation notes state the reduced fidelity. For example,
`vendor.diodes.ap2112k_3v3` uses AP2112K datasheet voltage, dropout, current,
enable, and capacitor metadata while its generated-SPICE face points at
`CIRCUITCI_IDEAL_LDO_3V3`. That lets users place and observe the real part in
Sketch/Scopes without pretending the generic subcircuit is a vendor transient,
thermal, stability, or PSRR model.
`vendor.ti.tps54331_5v` follows the same pattern for 5 V buck-regulator rail
observation: its TI source-pinned input range, output-current class,
switching-frequency class, configured output-voltage metadata, and static
power-conversion metadata remain source-backed, while its generated-SPICE face
uses a reduced enabled output source. Its `simulation.spice.instance_parameters`
map an optional `observation_output_voltage_V` Board IR component parameter into
the SPICE `VOUT_V` instance parameter with a visible 5 V default. That model can
exercise VIN/EN/VSENSE wiring and preliminary output/load-current observation,
but not PH/BOOT switching, compensation, inductor ripple/current, output ripple,
current limit, Eco-mode, startup timing, thermal behavior, layout, EMI, or
loop-stability sign-off.
`vendor.ti.tps62162_3v3`, `vendor.ti.tps63802_3v3`, and
`vendor.ti.tps61023_5v` follow the same pattern for common switch-mode rail
observation. Their source-pinned input ranges, output classes or current-limit
classes, support-component constraints, and static power-conversion metadata
remain source-backed, while their generated-SPICE faces use reduced enabled
output sources with optional `VOUT_V` instance parameters. These models can
exercise VIN/EN/VOS or VIN/EN/VOUT wiring and preliminary output/load-current
observation, but not SW/L1/L2 switching, feedback-loop dynamics, ripple, current
limit, PG/MODE behavior, thermal behavior, layout, EMI, or loop-stability
sign-off.
`vendor.wch.ch340c` follows the same pattern for USB-UART output-state
observation: its WCH source-pinned supply range, 3.3 V-mode logic thresholds,
VOH/source-impedance metadata, active-low modem-output notes, and integrated
clock note remain source-backed, while its generated-SPICE face uses reduced
voltage-driver outputs for TXD, DTR#, and RTS#. Its
`simulation.spice.instance_parameters` map optional Board IR component
parameters `observation_txd_state`, `observation_dtr_n_state`, and
`observation_rts_n_state` into explicit SPICE output-state parameters. That
model can exercise rail and host-control-line output-state observations, but
not USB PHY behavior, enumeration, baud-rate timing, oscillator accuracy,
transistor-level auto-download circuits, or final I/O injection-current
sign-off.

`vendor.silabs.cp2102n` extends that USB-UART generated-observation pattern to
parts with a VREGIN-fed internal regulator. Its Silicon Labs source-pinned
VREGIN/VDD/VIO rail limits, regulator output-current class, reset pull-up note,
and UART logic threshold metadata remain source-backed, while its
generated-SPICE face uses a reduced VREGIN-to-VDD rail source plus VIO-referenced
TXD, RTS, and DTR voltage-driver outputs. Its
`simulation.spice.instance_parameters` map optional Board IR component
parameters `observation_txd_state`, `observation_rts_state`, and
`observation_dtr_state` into explicit SPICE output-state parameters. That model
can exercise regulator-rail and host-control-line output-state observations,
but not USB PHY behavior, enumeration, baud-rate timing, oscillator accuracy,
suspend behavior, regulator stability, transistor-level modem-line circuitry,
or final I/O injection-current sign-off.

`vendor.ftdi.ft232r` follows the same USB-UART generated-observation pattern
for FTDI bridges with a `3V3OUT` regulator and separate `VCCIO` logic rail. Its
source-backed VCC/VCCIO/3V3OUT rail limits, regulator output-current class,
reset-pin note, CBUS configuration note, and UART input/output threshold
metadata remain source-backed, while its generated-SPICE face uses a reduced
VCC-to-3V3OUT rail source plus VCCIO-referenced TXD, RTS#, and DTR# voltage
drivers. Its `simulation.spice.instance_parameters` map optional Board IR
component parameters `observation_txd_state`, `observation_rts_n_state`, and
`observation_dtr_n_state` into explicit SPICE output-state parameters. That
model can exercise regulator-rail and host-control-line output-state
observations, but not USB PHY behavior, enumeration, EEPROM/CBUS programming,
baud-rate timing, oscillator accuracy, suspend behavior, regulator stability,
transistor-level modem-line circuitry, or final I/O injection-current and
thermal sign-off.

`vendor.ti.tps2121` follows the same pattern for selected-source power-mux
observation: its TI source-pinned input range, output-current class,
reverse-blocking metadata, string `selected_input` static contract, and
`power_mux` metadata remain source-backed, while its generated-SPICE face uses a
reduced ideal selected-source output. Its `simulation.spice.instance_parameters`
map optional numeric `observation_selected_input_index` into the SPICE
`SELECT_INPUT` parameter so generated observations can choose IN1 or IN2 without
changing the static power-tree parameter. That model can exercise IN1/IN2/OUT
wiring and preliminary output/load-current observation, but not priority
threshold comparators, switchover droop, reverse-current magnitude,
ILIM-derived current limit, soft-start timing, thermal behavior, status output,
layout, or final mux sign-off.
`vendor.ti.tps2115a` follows the same pattern for autoswitching power-mux
observation: its TI source-pinned 2.8 V to 5.5 V input range, 1 A output-current
class, reverse/cross-conduction blocking metadata, string `selected_input`
static contract, and `power_mux` metadata remain source-backed, while its
generated-SPICE face uses a reduced ideal selected-source output. Its
`simulation.spice.instance_parameters` map optional numeric
`observation_selected_input_index` into the SPICE `SELECT_INPUT` parameter so
generated observations can choose IN1 or IN2 without changing the static
power-tree parameter. That model can exercise IN1/IN2/OUT wiring and
preliminary output/load-current observation, but not EN/D0/D1/VSNS autoswitch
truth-table behavior, switchover droop, reverse-current magnitude, ILIM-derived
current limit, thermal behavior, package limits, layout, or final mux sign-off.
`vendor.ti.tlv803ea29` follows the same pattern for reset-supervisor threshold
observation: its datasheet threshold, delay, active-low open-drain topology, and
pin metadata remain source-backed, while its generated-SPICE face points at a
reduced nominal-threshold open-drain behavioral subcircuit. That model can
exercise reset pull-up wiring and assertions, but not reset-delay, hysteresis,
glitch-immunity, propagation-delay, or leakage sign-off.
`vendor.microchip.mcp1316t_29le_ot` uses the same static reset-supervisor
contract for an active-low push-pull 2.90 V supervisor with optional MR_N and
WDI board-boundary pins. Its `reset_release_delay_us` uses the conservative
standard timeout maximum; watchdog behavior and manual-reset debounce remain
outside the static contract.
`vendor.ti.tps22918` follows the same pattern for active-high load-switch
observation: its datasheet voltage, ON threshold, current, pinout, and static
power-switch metadata remain source-backed, while its generated-SPICE face uses
a reduced smooth VIN-to-VOUT conductance near the typical on-resistance. That
model can exercise enabled-load wiring and preliminary voltage/current
observations, but not CT slew-rate shaping, QOD discharge, reverse current,
current limiting, inrush, leakage, or thermal sign-off.
`vendor.ti.tps25948_8a_rcb_dvdt` follows the same pattern for eFuse/load-switch
observation: its TI source-backed voltage range, EN/UVLO threshold,
current-limit class, maximum on-resistance, always-on reverse-current-blocking
metadata, and static power-switch limits remain source-backed, while its
generated-SPICE face uses a reduced smooth VIN-to-VOUT conductance. That model
can exercise enabled protected-rail wiring and preliminary voltage/current
observations, but not dVdt slew, ILM/ITIMER current-limit and fault timing,
FLT/SPLYGD outputs, OVLO, RCBCTRL, reverse-current dynamics, thermal shutdown,
inrush, or final eFuse/protection sign-off.
`vendor.ti.tps24751_csd17501q5a_12a_reverse_blocking` follows the same pattern
for hot-swap/reverse-blocking observation: its TI source-backed voltage range,
EN threshold, 12 A current class, 11 A current-limit design point, 9.7 mOhm
effective path resistance, disabled-state reverse-current-blocking metadata,
and static power-switch limits remain source-backed, while its generated-SPICE
face uses a reduced smooth VIN-to-VOUT conductance. That model can exercise
enabled protected-rail wiring and preliminary voltage/current observations, but
not TIMER/PROG/SET current-limit and fault-timer behavior, FLTb/PGb outputs,
external MOSFET gate-drive dynamics, disabled-state reverse-current dynamics,
thermal shutdown, inrush accuracy, or final hot-swap/protection sign-off.
`vendor.ti.drv8323` follows the same pattern for three-phase smart gate-driver
observation: its TI source-backed VM/DVDD ranges, logic threshold, nFAULT/SDO
output metadata, and three current-sense amplifier metadata remain
source-backed, while its generated-SPICE face uses reduced voltage-driver and
observation-node behavior. Its `simulation.spice.instance_parameters` map
optional Board IR component parameters `observation_nfault_state`,
`observation_sdo_state`, `observation_soa_v`, `observation_sob_v`, and
`observation_soc_v` into explicit SPICE instance parameters. That model can
exercise supply, enable, nFAULT/SDO output-state, and current-sense output
presence observations, but not MOSFET gate-drive strength, half-bridge
switching, charge-pump/bootstrap behavior, dead time, SPI register/protection
behavior, shunt gain/offset/noise, motor dynamics, layout, EMI, thermal
behavior, or final motor-driver sign-off.
`vendor.nxp.pca9685` follows the same pattern for PWM-driver observation: its
NXP source-backed 2.3 V to 5.5 V VDD range, Fast-mode Plus I2C role, 12-bit PWM
controller role, and low-load output class remain source-backed, while its
generated-SPICE face uses reduced voltage-driver and pulse-source behavior. Its
`simulation.spice.instance_parameters` map optional Board IR component
parameters `observation_pwm_high_v`, `observation_pwm_frequency_hz`,
`observation_pwm0_duty_percent`, `observation_pwm1_duty_percent`,
`observation_pwm2_duty_percent`, `observation_pwm3_duty_percent`,
`observation_scl_state`, and `observation_sda_state` into explicit SPICE
instance parameters. That model can exercise VDD/OE, I2C idle-line, and
representative low-load PWM high/low sample observations, but not I2C protocol,
register behavior, oscillator tolerance, phase staggering, LED/servo output
current, pull-up rise time, servo position/stall/regeneration, disabled-output
high-Z behavior, thermal behavior, or final PWM timing sign-off.
`vendor.tdk.icm42688p` follows the same pattern for IMU board-boundary
observation: its TDK source-backed 1.71 V to 3.6 V VDD/VDDIO ranges, SPI input
thresholds, and SDO/INT1 output metadata remain source-backed, while its
generated-SPICE face uses reduced high-impedance input pins and static output
drivers. Its `simulation.spice.instance_parameters` map optional Board IR
component parameters `observation_sdo_state` and `observation_int1_state` into
explicit SPICE output-state parameters. That model can exercise VDD/VDDIO, SPI
line-state, SDO, and INT1 observations, but not sensor dynamics, register
protocol, FIFO behavior, sampling timing, noise, bias stability, vibration,
package stress, layout coupling, or final SPI timing sign-off.
`vendor.sipeed.licheerv_nano_w` follows the same pattern for Linux module
board-boundary observation: its Sipeed source-backed 5 V module supply range,
1 A current-budget class, UART/GPIO threshold metadata, and project-facing
low-speed pin roles remain source-backed, while its generated-SPICE face uses
reduced high-impedance input pins and static output drivers. Its
`simulation.spice.instance_parameters` map optional Board IR component
parameters `observation_uart0_tx_a16_state` and
`observation_gpioa14_motion_en_state` into explicit SPICE output-state
parameters. That model can exercise 5 V module power, UART0 TX/RX,
motion-enable, and fault-IRQ observations, but not Linux boot power transients,
internal SoC rails, firmware behavior, USB/MIPI/high-speed interfaces,
RF/Wi-Fi behavior, thermal behavior, exact header-numbering sign-off, or final
signal-integrity sign-off.
`vendor.st.stm32l431vct6` and `vendor.um.um_stm32l4_resident` follow the same
pattern for STM32L431 board-boundary observation: saved ST source documents
back their VDD range, reset/BOOT0 boot metadata, USART1 PA9/PA10 role, and
SWD PA13/PA14 role, while their generated-SPICE face uses reduced
high-impedance input pins and static PA9/PA13 output drivers. Their
`simulation.spice.instance_parameters` map optional Board IR component
parameters `observation_pa9_state` and `observation_pa13_state` into explicit
SPICE output-state parameters. That model can exercise VDD, NRST, BOOT0,
USART1 TX/RX, and SWDIO/SWCLK line-state observations, but not firmware
execution, oscillator accuracy, reset timing, UART protocol timing, SWD
transactions, flash programming side effects, exhaustive package-pin mapping,
layout, thermal behavior, EMC, or final signal-integrity sign-off.
`vendor.raspberrypi.rp2040` adds a source-backed static RP2040 pack without a
generated-SPICE face yet. Official Raspberry Pi source documents back its
IOVDD, VREG_VIN, DVDD, USB_VDD, and ADC_AVDD supply limits, internal
VREG_VOUT-to-DVDD `100 mA` regulator budget, RUN active-low reset metadata,
3.3 V GPIO thresholds, and QSPI_SS BOOTSEL strap contract. That model can
exercise board-level power-tree and BOOTSEL strap checks, but not USB signal
integrity, crystal startup/accuracy, QSPI flash protocol timing, BOOTROM USB
protocol behavior, firmware execution, thermal behavior, or transient current
waveforms.
`vendor.nordic.nrf52840` adds a source-backed static nRF52-class pack without
a generated-SPICE face yet. Official Nordic documentation backs its `VDD`
normal-voltage range, optional `VDDH` high-voltage range, optional USB `VBUS`
range, configurable `P0.18` reset boundary, SWD pins, USB pins, and antenna
pin identity. That model can exercise board-level supply voltage checks, but
not high-voltage-mode regulator sequencing, GPIO threshold or drive-strength
sign-off, DCDC support networks, USB signal integrity, antenna matching, NFC,
RF protocol behavior, UICR reset programming, firmware execution, thermal
behavior, or transient current waveforms.
`vendor.st.stm8s003f3p6` adds a source-backed static STM8S003F3P6 pack without
a generated-SPICE face yet. Official ST documentation backs its `VDD` supply
range, required `VCAP` board pin, active-low `NRST`, `PD1/SWIM`, and
`PD5`/`PD6` UART1 pin boundaries. That model can exercise board-level supply
voltage checks and required pin binding, but not `VCAP` capacitance/ESR/ESL,
formula-based GPIO thresholds, oscillator startup/accuracy, SWIM protocol
timing, UART bootloader behavior, flash/EEPROM programming behavior, firmware
execution, thermal behavior, or transient current waveforms.
`vendor.stc.stc15w408as` adds a source-backed static STC 1T 8051-family pack
without a generated-SPICE face yet. Official STC documentation backs its `VCC`
operating range, active-high reset boundary, primary ISP/UART pins
`P3.0/RxD` and `P3.1/TxD`, and alternate UART pin pairs. That model can
exercise board-level supply voltage checks and reset/UART board-boundary
review, but not exact STC ISP sync/ACK bytes, ISP monitor entry timing,
package-variant exhaustive pin mapping, oscillator startup/accuracy, flash or
EEPROM programming behavior, firmware execution, thermal behavior, or
transient current waveforms.
`vendor.ti.ne555` adds a source-backed static NE555 timer pack without a
generated-SPICE face yet. Official TI documentation backs its 4.5 V to 16 V
NE555 supply range, 8-pin timer pin roles, no-load supply-current class,
output-current class, and 0.1 uF VCC bypass recommendation. That model can
exercise board-level supply voltage checks and timer pin-boundary review, but
not RC timing equations, threshold-spread sign-off, output-drive/load sign-off,
discharge-transistor saturation, reset pulse timing, control-voltage
modulation, thermal behavior, or generated timer waveforms.
`vendor.abracon.abm3_8mhz_18pf` adds a source-backed static ABM3 8 MHz crystal
pack. Official Abracon documentation backs its 8 MHz nominal frequency, 18 pF
standard load capacitance, 140 ohm maximum ESR for the 8 MHz to below-9 MHz
fundamental range, 7 pF shunt capacitance, 10 uW to 100 uW drive range, and
standard ppm tolerance/stability metadata. The validator currently uses only
the crystal terminals and load-capacitance target for `CLOCK_SOURCE_VALID`; it
does not sign off oscillator startup, negative resistance, drive-level stress,
ppm accuracy, layout parasitics, motional behavior, or phase noise.
`vendor.winbond.w25q64jv` adds a source-backed static SPI/QSPI NOR flash pack.
Official Winbond documentation backs its 2.7 V to 3.6 V VCC rail range, 25 mA
program/erase/write-current class, 8-pin SPI/QSPI pin roles, 64 Mbit density,
256-byte page size, 4 KiB sector size, 100k minimum program-erase endurance,
and 20-year minimum retention metadata. The validator currently uses the power
pin contract for `POWER_TREE_VALID` and the explicit port list for
board-boundary review; it does not emulate SPI commands, JEDEC ID, SFDP tables,
erase/program sequencing, flash contents, XIP behavior, write-protection
policy, or high-speed signal integrity.
`vendor.bosch.bme280` adds a source-backed static environmental sensor pack.
Official Bosch documentation backs its 1.71 V to 3.6 V `VDD` rail range,
1.2 V to 3.6 V `VDDIO` rail range, separate digital-interface supply, I2C/SPI
pin roles, 2.5 mm x 2.5 mm x 0.93 mm package, low-power use-current metadata,
I2C/SPI clock-class metadata, and SDO-selected I2C address metadata. The
validator currently uses the two power-pin contracts for `POWER_TREE_VALID`
and the explicit port list for board-boundary review; it does not validate
humidity, pressure, or temperature accuracy, compensation algorithms, register
protocols, bus timing, response time, noise, drift, self-heating, or
calibration behavior.
`vendor.kingbright.apt1608surck` adds a source-backed common indicator LED
pack. Official Kingbright documentation backs its 30 mA DC forward-current
limit, 5 V reverse-voltage limit, 75 mW power limit, 1.95 V typical and 2.5 V
maximum forward voltage at 20 mA, capacitance, wavelength, viewing-angle, and
package metadata. Generated SPICE uses the same diode operating-limit probes as
small-signal diodes, but the bundled electrical card is a reduced preliminary
fit and does not validate optical output, lifetime, thermal board coupling,
pulse-current derating, reflow-process exposure, or production hardware
behavior.
`vendor.onsemi.npn_2n3904` adds a source-backed common NPN transistor pack.
Official onsemi documentation backs its 40 V `VCEO`, 60 V `VCBO`, 6 V `VEBO`,
200 mA continuous collector-current limit, 625 mW ambient power limit with
5 mW/C derating, TO-92 pinout, gain/saturation, capacitance, and transition
frequency metadata. Generated SPICE uses the same BJT operating-limit probes
as the existing SS8050/SS8550 packs, but the bundled electrical card is a
reduced preliminary fit and does not validate gain spread, storage time, noise,
package thermal coupling, or production hardware behavior.
`vendor.onsemi.pnp_2n3906` adds the matching source-backed common PNP
transistor pack. Official onsemi documentation backs its 40 V `VCEO`, 40 V
`VCBO`, 5 V `VEBO`, 200 mA continuous collector-current magnitude, 625 mW
ambient power limit with 5 mW/C derating, TO-92 pinout, gain/saturation,
capacitance, and transition-frequency metadata. The model stores PNP terminal
voltage and collector-current ratings with negative signs so reports preserve
polarity while operating-limit probes evaluate absolute magnitude.
`vendor.onsemi.1n5819` adds a source-backed common Schottky rectifier pack.
Official onsemi documentation backs its 40 V repetitive reverse-voltage limit,
1 A average rectified forward-current rating, 25 A one-cycle surge-current
metadata, low forward-voltage rows, leakage rows, ESD metadata, axial
polarity-band package evidence, and enough thermal evidence to derive a
conservative 0.875 W ambient power screen. Generated SPICE uses the existing
diode operating-limit probes for reverse voltage, forward current, and power,
but the bundled card is a reduced preliminary fit and does not validate thermal
runaway, repetitive surge stress, leakage over temperature, rectifier waveform
heating, or production hardware behavior.
`vendor.artery.at32f435_motion_core` follows the same pattern for MCU
board-boundary observation: its Artery source-backed MCU class, project VDD
range, current-budget class, UART/CAN/RS-485/control GPIO threshold metadata,
and project-facing pin roles remain source-backed, while its generated-SPICE
face uses reduced high-impedance input pins and static output drivers. Its
`simulation.spice.instance_parameters` map optional Board IR component
parameters `observation_lrv_uart_tx_state`,
`observation_motion_fault_irq_state`, `observation_can_tx_state`,
`observation_rs485_tx_state`, `observation_rs485_de_state`, and
`observation_servo_pwm_oe_state` into explicit SPICE output-state parameters.
That model can exercise VDD, LicheeRV UART, motion-enable/fault, CAN, RS-485,
and servo PWM enable observations, but not firmware execution, reset/clock
timing, CAN/RS-485 protocol timing, ADC behavior, motor-control loops, exact
package pin assignment, layout, thermal behavior, EMC, or final
signal-integrity sign-off.
`vendor.artery.at32m416_motor_control` follows the same pattern for wheel
actuator motor-control MCU board-boundary observation: its Artery source-backed
motor-control class, project VDD range, current-budget class, CAN, six PWM
outputs, driver-interface, current-sense, encoder, enable, and fault-line port
metadata remain source-backed, while its generated-SPICE face uses reduced
high-impedance input pins and static output drivers. Its
`simulation.spice.instance_parameters` map optional Board IR component
parameters `observation_can_tx_state`, `observation_pwm_uh_state`,
`observation_pwm_ul_state`, `observation_pwm_vh_state`,
`observation_pwm_vl_state`, `observation_pwm_wh_state`,
`observation_pwm_wl_state`, `observation_drv_en_state`,
`observation_drv_spi_sck_state`, `observation_drv_spi_mosi_state`,
`observation_drv_spi_cs_state`, and `observation_fault_out_state` into
explicit SPICE output-state parameters. That model can exercise VDD, CAN,
six-PWM, driver enable/fault/SPI, current-sense, encoder, board enable, and
fault-output observations, but not firmware execution, reset/clock timing, PWM
timer waveform generation, ADC conversion or current reconstruction, FOC
loops, dead-time, exact package pin assignment, gate-drive physics, layout,
thermal behavior, EMC, or final signal-integrity sign-off.
`vendor.microchip.mcp73831_4v2` follows the same pattern for Li-Ion charger
observation: its datasheet pinout, input range, battery regulation target, PROG
resistor charge-current equation, and static charger metadata remain
source-backed, while its generated-SPICE face uses a reduced smooth
constant-current/constant-voltage behavioral source. That model can exercise
charger wiring, PROG resistor behavior, battery-node voltage, and preliminary
charge-current observations, but not preconditioning, termination, STAT,
thermal regulation, timer behavior, battery chemistry, cell safety, package
dissipation, or final charger sign-off.
`vendor.ti.bq24075` follows the same pattern for power-path charger
observation: its datasheet pinout, IN/BAT/OUT ranges, battery regulation
target, ISET resistor charge-current equation, and static charger metadata
remain source-backed, while its generated-SPICE face uses a reduced OUT rail
source plus constant-current/constant-voltage BAT charger. That model can
exercise power-path charger wiring, OUT rail observation, ISET resistor
behavior, battery-node voltage, and preliminary charge-current observations,
but not DPPM, battery supplement mode, ILIM/EN current-limit derivation, status
pins, termination, thermal regulation, timer behavior, battery chemistry, cell
safety, package dissipation, or final charger sign-off.
`vendor.ti.bq25798` follows the same pattern for buck-boost/NVDC charger
observation: its TI source-pinned input range, 1- to 4-cell/5 A class charger
metadata, NVDC power-path notes, and static charge-current parameter remain
source-backed, while its generated-SPICE face uses a reduced SYS rail source
and BAT charge-current source. Its `simulation.spice.instance_parameters` map
`programmed_charge_current_A` and the fixture-level
`observation_system_voltage_V` into SPICE instance parameters, with a visible
12 V default for the preliminary SYS observation target. That model can
exercise adapter/SYS/BAT wiring and preliminary configured-current observation,
but not buck-boost switching, DPM/MPPT, BATFET supplement mode, register
sequencing, thermal regulation, safety timers, battery chemistry, or final
charger sign-off.

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

Projects may also declare `MODEL_QUALITY_REQUIRED` scenarios for selected
components. Those scenarios compare each named component model's
`model_quality.source` and `model_quality.confidence` against an explicit
sign-off policy, turning placeholder envelopes into critical findings when a
board is being prepared for fabrication.
