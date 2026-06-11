# PROJECT.md — CircuitCI / BoardSim

## 0. Project identity

**Working product name:** CircuitCI
**Internal engine name:** BoardSim
**Project type:** generic, library-driven circuit validation and mixed-domain simulation platform
**Primary purpose:** allow AI agents and engineers to validate embedded/IoT circuit designs and firmware before PCB fabrication.

CircuitCI is not a schematic editor, PCB layout tool, or replacement for KiCad/EasyEDA/Altium. It is a **pre-fabrication validation runtime** that imports designs, attaches component models, runs simulation scenarios, and produces machine-readable and human-readable pass/fail reports.

---

## 1. Product thesis

AI agents can now generate schematics, firmware, BOMs, and board variants quickly. The missing layer is a trusted way to verify whether those generated designs are electrically and logically sane before fabrication.

CircuitCI provides that missing layer:

```text
requirement
  -> agent generates circuit + firmware
  -> CircuitCI imports design artifacts
  -> CircuitCI attaches component models
  -> CircuitCI runs validation profiles
  -> CircuitCI reports failures with waveforms and root causes
  -> agent patches design
  -> repeat until validation passes
  -> only then fabricate PCB
```

The product is best understood as:

```text
CI/CD for embedded circuit designs.
```

---

## 2. Non-negotiable scope

CircuitCI must be generic.

STM32, ESP32, CH340, capacitors, MOSFETs, LDOs, sensors, connectors, and batteries are all **library elements**. The core platform must not be hardcoded around STM32, CH340, ESP32, or any single board family.

The platform must support:

1. Generic board graph representation.
2. Multi-fidelity component model library.
3. Multi-domain simulation kernel.
4. Scenario/fault-injection engine.
5. Validation rule engine.
6. Agent-readable JSON reports.
7. Human-readable HTML/Markdown reports.
8. Waveform export.

The first demos may use STM32/CH340/ESP32 because they are practical and easy to validate, but they are not the product boundary.

---

## 3. Primary users

### 3.1 AI hardware agents

Agents that generate circuit designs and firmware need a validation target. CircuitCI gives them pass/fail feedback, waveform evidence, and machine-readable repair hints.

### 3.2 Embedded engineers

Engineers use the tool before PCB fabrication to catch common mistakes:

* wrong boot strap values
* reset timing failures
* GPIO back-power paths
* unstable or overloaded regulators
* MOSFET under-drive
* wrong pull-up domains
* bad I2C pull-up sizing
* unsafe power sequencing
* USB-UART auto-download failure
* deep-sleep leakage mistakes

### 3.3 Hardware startups and freelancers

Small teams use the tool as a low-cost virtual bring-up lab before ordering boards.

---

## 4. Explicit non-goals

Do not build these in the MVP:

1. Schematic editor.
2. PCB layout editor.
3. Full EDA suite.
4. Full transistor-level MCU simulation.
5. Full USB PHY analog simulation.
6. GHz RF/antenna solver.
7. DDR/high-speed signal-integrity solver.
8. Complete SMPS compensation design tool.
9. Perfect vendor-silicon replication.
10. Fully automatic datasheet-to-perfect-model generator.

These may be future integrations, but the MVP must stay focused on embedded board validation.

---

## 5. Core concept

A design is converted into a board-level intermediate representation:

```text
EDA schematic/netlist/BOM
        ↓
Board Graph IR
        ↓
Component Model Binding
        ↓
Scenario Execution
        ↓
Simulation + Rule Checks
        ↓
Report + Waveforms + Suggested Fixes
```

CircuitCI answers two questions:

### Simulation question

```text
What happens over time?
```

Examples:

* VDD ramp waveform
* reset release timing
* GPIO voltage over time
* MOSFET gate voltage
* I2C rise time
* UART bytes
* back-drive current

### Validation question

```text
Is this design likely to work safely and reliably?
```

Examples:

* PASS/FAIL boot sequence
* PASS/FAIL power-domain safety
* PASS/FAIL programming interface
* PASS/FAIL GPIO voltage limits
* PASS/FAIL sleep current budget
* PASS/FAIL protocol timing

---

## 6. Architecture overview

```text
CircuitCI
│
├── importers/
│   ├── kicad/
│   ├── easyeda/
│   ├── altium/
│   ├── spice/
│   └── yaml/
│
├── board_ir/
│   ├── components
│   ├── nets
│   ├── pins
│   ├── power_domains
│   ├── signal_domains
│   └── constraints
│
├── component_library/
│   ├── passives/
│   ├── semiconductors/
│   ├── power_ics/
│   ├── logic_ics/
│   ├── mcu_soc/
│   ├── sensors/
│   ├── comms/
│   ├── connectors/
│   └── modules/
│
├── simulation_kernel/
│   ├── scheduler
│   ├── analog_solver_adapter
│   ├── digital_event_solver
│   ├── protocol_simulator
│   ├── firmware_emulator_adapter
│   └── analog_digital_bridge
│
├── scenario_engine/
│   ├── power_up
│   ├── power_down
│   ├── hot_plug
│   ├── reset_boot
│   ├── communication
│   ├── sleep_wakeup
│   ├── fault_injection
│   └── tolerance_sweep
│
├── validation_engine/
│   ├── electrical_limits
│   ├── timing_checks
│   ├── protocol_checks
│   ├── power_sequence_checks
│   ├── leakage_checks
│   ├── thermal_checks
│   └── design_rule_checks
│
├── reports/
│   ├── json
│   ├── html
│   ├── markdown
│   └── waveform_export
│
└── cli/
    └── circuitci
```

---

## 7. Implementation strategy

### 7.1 MVP language choice

Start with a Rust-first engine and CLI:

* Rust CLI
* Rust board IR parser
* Rust scenario runner
* Rust validation rules
* Rust report generator
* replaceable C/C++/Rust analog backend adapters

Python may be used for investigation scripts, data conversion experiments, or disposable research tooling only. It must not become the production engine backbone.

Do not over-engineer the first scheduler. Make it clear, testable, and replaceable.

### 7.2 Future production direction

When the core interfaces stabilize:

* keep the event scheduler and analog/digital bridge in Rust unless a backend requires C/C++
* keep Python limited to optional scripting and investigation tools
* keep backends replaceable
* keep component libraries data-driven

### 7.3 Design principle

Always prefer this:

```text
small verified vertical slice
```

over this:

```text
large unverified architecture skeleton
```

---

## 8. Board Graph IR

The Board Graph IR is the normalized internal form of a circuit design.

Minimum required concepts:

```yaml
project:
  name: example_iot_board
  version: 0.1.0

components:
  U1:
    model: generic.mcu
    part_number: STM32L431
    pins:
      VDD: net_3v3
      GND: gnd
      NRST: net_nrst
      BOOT0: net_boot0
      PA9: net_uart_tx
      PA10: net_uart_rx

  U2:
    model: generic.usb_uart
    part_number: CH340C
    pins:
      VCC: net_3v3
      GND: gnd
      TXD: net_uart_rx
      RXD: net_uart_tx
      DTR_N: net_dtr
      RTS_N: net_rts

  R1:
    model: generic.resistor
    value: 10k
    pins:
      "1": net_boot0
      "2": gnd

nets:
  net_3v3:
    kind: power
    nominal_voltage: 3.3

  gnd:
    kind: ground

  net_boot0:
    kind: digital_or_analog

  net_nrst:
    kind: digital_or_analog
```

The IR must support mixed nets. A reset net may be an analog RC waveform and also a digital reset signal after threshold crossing.

---

## 9. Component model contract

Every library element must use a common contract.

```yaml
component_id: vendor.family.part
version: 0.1.0
category: mcu

ports:
  VDD:
    kind: electrical_power
  GND:
    kind: electrical_ground
  NRST:
    kind: digital_electrical_input
  BOOT0:
    kind: digital_electrical_input
  PA9:
    kind: digital_electrical_io
  PA10:
    kind: digital_electrical_io

parameters:
  vdd_min:
    value: 1.71
    unit: V
  vdd_max:
    value: 3.6
    unit: V

model_faces:
  electrical_pins:
    file: pins.yaml

  power_reset_boot:
    file: reset_boot.yaml

  protocol:
    files:
      - uart.yaml
      - i2c.yaml
      - spi.yaml

  firmware_emulation:
    adapter: renode_or_qemu
    status: future

  spice:
    file: optional_spice_model.lib

rules:
  - gpio_voltage_limit
  - injection_current_limit
  - reset_pulse_width
  - boot_pin_sampling
  - power_domain_validity

tests:
  - tests/component/vendor.family.part/pin_thresholds.yaml
  - tests/component/vendor.family.part/reset_boot.yaml
```

### 9.1 Model faces

A component may expose one or more faces:

| Face                 | Purpose                                              |
| -------------------- | ---------------------------------------------------- |
| `spice`              | analog electrical simulation                         |
| `electrical_pins`    | pin thresholds, leakage, drive strength, capacitance |
| `digital_behavior`   | state-machine behavior                               |
| `protocol`           | UART/I2C/SPI/CAN/USB-UART/etc.                       |
| `power_model`        | supply current, sleep current, burst current         |
| `firmware_emulation` | run actual firmware or abstract firmware model       |
| `thermal`            | power-to-temperature approximation                   |
| `rules`              | datasheet/design-rule validation                     |
| `fault_model`        | open/short/leakage/wrong-value tests                 |

### 9.2 Fidelity levels

Each component should support multiple fidelity levels where useful:

```yaml
fidelity_levels:
  - ideal
  - simple_behavioral
  - simple_spice
  - vendor_spice
  - electrothermal
  - measured_calibrated
```

Agents must choose the lowest fidelity that answers the validation question.

---

## 10. Initial component library

The first library must be small but useful.

### 10.1 Passives

* resistor
* capacitor
* inductor
* ferrite bead
* jumper
* test point

Capacitor model should support:

* nominal capacitance
* tolerance
* ESR
* ESL
* leakage
* voltage rating
* DC-bias derating metadata
* temperature derating metadata

### 10.2 Semiconductors

* diode
* Schottky diode
* TVS diode
* BJT NPN
* BJT PNP
* NMOS
* PMOS

MOSFET validation must include:

* VGS max
* VDS max
* gate-drive margin
* RDS(on) validity at actual gate voltage
* power dissipation estimate
* body diode conduction warning

### 10.3 Power

* ideal voltage source
* battery source
* USB 5 V source
* LDO simplified model
* buck simplified model
* load switch
* ideal current load
* pulsed current load

Power checks must include:

* voltage margin
* startup sequencing
* dropout
* overload/current limit
* output capacitor stability hints
* burst-load droop

### 10.4 Digital/electrical primitives

* push-pull output pin
* open-drain output pin
* digital input pin
* bidirectional GPIO pin
* Schmitt input
* weak pull-up
* weak pull-down
* ESD/injection approximation

### 10.5 Communication ICs

Start with behavioral models:

* generic USB-UART bridge
* CH340-family pack
* CP210x-family pack
* FT232-family pack

These should model:

* TXD/RXD
* DTR/RTS
* baud rate
* serial open/close events
* output drive levels
* powered/unpowered behavior
* basic UART timing

Do not model full USB PHY in MVP.

### 10.6 MCU/SOC

Start with generic models:

* generic MCU power/reset/GPIO model
* generic STM32-like boot model
* generic ESP32-like EN/IO0 boot model

Then add concrete packs:

* STM32L4 basic pack
* STM32F/G basic packs
* ESP32 basic pack

MVP MCU model must support:

* VDD valid threshold
* reset threshold
* reset pulse width
* boot pin sampling
* GPIO reset state
* UART bootloader abstract handshake
* optional firmware initial-pin-state file

Do not emulate all peripherals in the MVP.

---

## 11. Simulation kernel

The simulation kernel coordinates multiple domains.

### 11.1 Analog solver adapter

The analog solver adapter owns:

* netlist generation
* analog simulation execution
* source updates
* waveform collection
* threshold-crossing callbacks
* convergence/failure diagnostics

Backends must be swappable:

```text
analog_solver:
  backend: ngspice
```

Later:

```text
analog_solver:
  backend: xyce
```

### 11.2 Digital event solver

The digital solver owns:

* scheduled events
* logic state
* protocol state machines
* pin mode changes
* UART/I2C/SPI transactions
* firmware abstract events

### 11.3 Analog/digital bridge

The bridge converts:

```text
digital output event -> analog voltage/current source
analog voltage crossing -> digital input event
power rail crossing -> component power-state event
protocol event -> digital/pin events
```

Each digital input must have:

* VIL
* VIH
* optional hysteresis
* undefined region
* minimum pulse width
* optional debounce/glitch filter

CircuitCI must warn when an input spends meaningful time in the undefined region.

---

## 12. Scenario engine

A scenario describes what happens to the board.

Example:

```yaml
scenario: usb_programming_attempt

events:
  - at: 0ms
    action: set_source
    source: usb_vbus
    voltage: 5.0

  - at: 10ms
    action: serial_open
    device: U2
    baud: 115200

  - at: 12ms
    action: set_modem
    device: U2
    dtr: low
    rts: high

  - at: 20ms
    action: set_modem
    device: U2
    dtr: high
    rts: low

  - at: 25ms
    action: uart_send
    device: U2
    bytes: [0x7F]

checks:
  - boot_pin_valid_during_reset_release
  - reset_pulse_width
  - uart_bootloader_sync
  - no_backdrive_current_above_limit
```

Scenario types required for MVP:

1. `power_up`
2. `power_down`
3. `usb_hot_plug`
4. `reset_boot`
5. `serial_programming`
6. `gpio_backdrive`
7. `i2c_bus`
8. `sleep_current`
9. `brownout`
10. `tolerance_sweep`

---

## 13. Validation profiles

A validation profile is a reusable set of scenarios and pass criteria.

### 13.1 `iot_basic_v0`

This is the first product profile.

```yaml
profile: iot_basic_v0

required_inputs:
  - board_ir
  - component_bindings
  - power_domains

optional_inputs:
  - bom
  - firmware_elf
  - firmware_pin_init
  - physical_load_profile

scenarios:
  - power_up
  - power_down
  - usb_hot_plug
  - reset_boot
  - programming_interface
  - gpio_backdrive
  - i2c_bus_if_present
  - spi_bus_if_present
  - uart_if_present
  - sleep_current_if_declared
  - battery_brownout_if_battery_present

pass_criteria:
  no_critical_electrical_limit_violation: true
  no_unknown_power_domain: true
  no_unresolved_component_model_for_critical_path: true
  reset_release_after_vdd_valid: true
  boot_straps_defined_during_sampling: true
  no_gpio_backdrive_above_default_limit: true
  programming_interface_valid_if_declared: true
```

### 13.2 Future profiles

* `motor_control_basic_v0`
* `battery_low_power_v0`
* `usb_powered_device_v0`
* `sensor_hub_v0`
* `industrial_24v_io_v0`
* `lora_node_v0`
* `esp32_auto_download_v0`
* `stm32_uart_boot_v0`

---

## 14. Validation rule examples

### 14.1 Back-power rule

Detect when a powered component drives a signal into an unpowered component through an input pin.

Output example:

```json
{
  "id": "GPIO_BACKDRIVE",
  "severity": "critical",
  "net": "I2C_SDA",
  "message": "Powered MCU drives SDA high while sensor VDD is 0 V.",
  "measured": {
    "injection_current_A": 0.0012
  },
  "limit": {
    "injection_current_A": 0.0001
  },
  "suggested_fixes": [
    "Move pull-up to the same power domain as the sensor.",
    "Add a bus switch.",
    "Add a series resistor.",
    "Configure MCU pin as high impedance before sensor power is removed."
  ]
}
```

### 14.2 Reset rule

Detect invalid reset release.

```json
{
  "id": "RESET_RELEASE_BEFORE_VDD_VALID",
  "severity": "critical",
  "net": "NRST",
  "message": "Reset releases before MCU VDD is above the valid operating threshold.",
  "suggested_fixes": [
    "Increase reset RC delay.",
    "Use supervisor IC.",
    "Tie reset release to regulator power-good.",
    "Review brownout threshold configuration."
  ]
}
```

### 14.3 MOSFET gate-drive rule

```json
{
  "id": "MOSFET_GATE_UNDERDRIVE",
  "severity": "warning",
  "component": "Q1",
  "message": "MOSFET gate voltage is below the voltage where RDS(on) is guaranteed.",
  "suggested_fixes": [
    "Select logic-level MOSFET specified at the available gate voltage.",
    "Use a gate driver.",
    "Reduce load current.",
    "Change topology."
  ]
}
```

---

## 15. Report outputs

CircuitCI must produce both machine-readable and human-readable reports.

### 15.1 Machine-readable JSON

Required fields:

```json
{
  "project": "example_iot_board",
  "profile": "iot_basic_v0",
  "result": "fail",
  "summary": {
    "critical": 2,
    "warning": 5,
    "info": 9
  },
  "failures": [],
  "warnings": [],
  "waveforms": [],
  "artifacts": [],
  "suggested_next_actions": []
}
```

### 15.2 Human-readable report

Required sections:

1. Executive summary.
2. Pass/fail table.
3. Critical failures.
4. Warnings.
5. Waveform snapshots.
6. Root-cause explanations.
7. Suggested fixes.
8. Unmodeled/low-confidence areas.
9. Reproduction command.
10. Version and model metadata.

### 15.3 Waveform export

Support at least:

* CSV
* VCD
* JSON time-series

Future:

* FST
* interactive HTML waveform viewer

---

## 16. CLI design

The first CLI should be simple.

```bash
circuitci init
circuitci import --format kicad --input board.kicad_sch --output board.ir.yaml
circuitci bind --board board.ir.yaml --library libs/default --output bound_board.yaml
circuitci validate --board bound_board.yaml --profile iot_basic_v0 --output out/
circuitci report --input out/report.json --format html --output out/report.html
```

Shortcut:

```bash
circuitci validate board.kicad_sch --profile iot_basic_v0 --output out/
```

Agent-friendly command:

```bash
circuitci validate project.yaml --json out/report.json --no-open-ui
```

---

## 17. Agent workflow

Agents working on hardware designs must follow this loop:

```text
1. Read requirement.
2. Generate or modify schematic/firmware.
3. Export netlist/BOM/firmware artifacts.
4. Run CircuitCI validation profile.
5. Read JSON report.
6. Fix critical failures first.
7. Re-run validation.
8. Do not declare design fabrication-ready until validation passes or limitations are explicitly documented.
```

### 17.1 Agent must not

Agents must not:

* claim a design is fabrication-ready without validation output
* ignore critical validation failures
* hide unmodeled assumptions
* convert warnings into passes without justification
* hardcode board-specific behavior in the generic engine
* add component models without tests
* add validation rules without at least one passing and one failing fixture
* silently change pass criteria to make tests pass

### 17.2 Agent must

Agents must:

* keep every feature testable
* add small fixtures for every rule
* document model fidelity and limitations
* prefer deterministic tests
* preserve machine-readable reports
* include reproduction commands
* label uncertain results as low confidence
* separate generic engine code from component-library code

---

## 18. Repository structure

Recommended repository:

```text
circuitci/
├── PROJECT.md
├── README.md
├── LICENSE
├── pyproject.toml
├── docs/
│   ├── architecture.md
│   ├── board_ir.md
│   ├── component_model_contract.md
│   ├── validation_profiles.md
│   ├── scenario_language.md
│   ├── agent_workflow.md
│   └── limitations.md
│
├── circuitci/
│   ├── __init__.py
│   ├── cli/
│   ├── board_ir/
│   ├── importers/
│   ├── library/
│   ├── simulation/
│   ├── scenarios/
│   ├── validation/
│   ├── reports/
│   └── utils/
│
├── libs/
│   ├── generic/
│   │   ├── passives/
│   │   ├── semiconductors/
│   │   ├── power/
│   │   ├── digital_pins/
│   │   └── protocols/
│   ├── vendor/
│   │   ├── st/
│   │   ├── espressif/
│   │   ├── wch/
│   │   ├── silicon_labs/
│   │   └── ftdi/
│   └── profiles/
│       └── iot_basic_v0.yaml
│
├── examples/
│   ├── good_iot_board/
│   ├── bad_bootstrap_board/
│   ├── bad_backdrive_board/
│   ├── bad_mosfet_gate_drive/
│   └── bad_i2c_pullup/
│
├── tests/
│   ├── unit/
│   ├── integration/
│   ├── fixtures/
│   └── golden_reports/
│
├── tools/
│   ├── model_lint/
│   ├── waveform_compare/
│   └── fixture_generator/
│
└── out/
    └── .gitkeep
```

---

## 19. First vertical slice

The first end-to-end demo must be generic but concrete.

### 19.1 Demo name

```text
examples/bad_backdrive_board
```

### 19.2 Board behavior

A USB-UART bridge is powered from USB 5 V / 3.3 V, while the MCU power domain is off. The USB-UART TXD pin drives the MCU RX pin high through a direct connection, creating injection/back-power current.

### 19.3 Expected result

CircuitCI must report:

```text
Result: FAIL
Critical: powered component drives unpowered component input above allowed injection current.
```

### 19.4 Why this demo matters

It proves:

* board IR works
* component binding works
* power domains work
* analog/digital pin model works
* scenario engine works
* validation rule works
* JSON report works
* human report works
* agent can fix the design and re-run

This is better than starting with a full MCU emulator.

---

## 20. Milestones

### M0 — Project skeleton

Deliverables:

* repository structure
* CLI stub
* board IR schema draft
* component model schema draft
* validation report schema draft
* example project fixture

Acceptance:

```bash
circuitci --help
circuitci validate examples/minimal_project/project.yaml --output out/
```

must run and produce a placeholder report.

---

### M1 — Board IR + model binding

Deliverables:

* YAML board IR parser
* component-library loader
* netlist consistency checker
* unresolved model reporting
* power-domain extraction

Acceptance:

* load at least three example boards
* detect missing component model
* detect floating required power pin
* produce valid report JSON

---

### M2 — Electrical pin model + rule engine

Deliverables:

* digital input model
* push-pull output model
* open-drain output model
* weak pull-up/down model
* ESD/injection approximation
* back-drive rule
* voltage-domain mismatch rule
* undefined digital input rule

Acceptance:

* `bad_backdrive_board` fails
* `good_backdrive_fixed_board` passes
* generated JSON identifies exact net and components

---

### M3 — Analog backend integration

Deliverables:

* simple analog netlist generation
* backend adapter
* transient simulation runner
* waveform capture
* threshold crossing extraction
* CSV/VCD export

Acceptance:

* simulate RC reset circuit
* detect reset crossing time
* report reset low pulse duration
* export waveform file

---

### M4 — Scenario engine

Deliverables:

* scenario YAML parser
* power-up scenario
* USB hot-plug scenario
* reset/boot scenario
* UART event abstraction
* tolerance sweep primitive

Acceptance:

* run `power_up`
* run `usb_hot_plug`
* run `serial_programming`
* produce per-scenario pass/fail report

---

### M5 — `iot_basic_v0` validation profile

Deliverables:

* reusable profile file
* power validity checks
* reset/boot checks
* programming interface checks
* GPIO back-drive checks
* I2C pull-up checks
* MOSFET gate-drive checks

Acceptance:

* one good IoT board passes
* at least five intentionally bad boards fail for the expected reason
* report includes fix suggestions

---

### M6 — Agent-ready loop

Deliverables:

* stable JSON report
* stable CLI
* example agent prompt
* docs for agent workflow
* failing/fixed design examples
* regression suite

Acceptance:

An agent can:

1. run validation
2. parse JSON report
3. identify the failed rule
4. patch example design
5. re-run validation
6. get PASS

without manual interpretation.

---

## 21. Testing strategy

Every rule requires:

1. one passing fixture
2. one failing fixture
3. expected JSON report
4. expected severity
5. expected suggested fix class

Test types:

```text
unit tests:
  parser, schema, rule functions

integration tests:
  board fixture -> validation report

golden tests:
  report output stable across versions

simulation tests:
  waveform numerical tolerance

agent-loop tests:
  failure report can guide deterministic fix
```

Do not merge a validation rule without fixtures.

---

## 22. Model quality policy

Every component model must declare:

```yaml
model_quality:
  source: datasheet | measured | estimated | generic
  confidence: high | medium | low
  intended_use:
    - power_sequence
    - timing
    - leakage
    - protocol
  not_valid_for:
    - rf
    - high_speed_signal_integrity
    - thermal_runaway
```

CircuitCI must never hide model limitations.

Reports must include:

```text
Unmodeled or low-confidence areas
```

This protects users from over-trusting simulations.

---

## 23. Open-source integration policy

CircuitCI should integrate existing tools instead of rewriting them.

Preferred integration pattern:

```text
stable adapter interface
  -> backend implementation
  -> backend-specific tests
  -> clear fallback behavior
```

Backend failure must not crash silently. It must produce an actionable error:

```text
Analog backend failed to converge in scenario power_up.
Try smaller timestep, simplified model, or inspect generated netlist.
```

Third-party licenses must be tracked in a dedicated file. Vendor model redistribution must be handled carefully.

---

## 24. Documentation requirements

Every major feature must update docs.

Required docs:

1. `docs/architecture.md`
2. `docs/board_ir.md`
3. `docs/component_model_contract.md`
4. `docs/scenario_language.md`
5. `docs/validation_profiles.md`
6. `docs/report_schema.md`
7. `docs/agent_workflow.md`
8. `docs/limitations.md`

Docs must include runnable examples.

---

## 25. Definition of done

A feature is done only when:

1. code is implemented
2. tests are added
3. example fixture exists if applicable
4. JSON report behavior is documented
5. human report behavior is documented
6. limitations are documented
7. CLI command is reproducible
8. agent can consume the output

A component model is done only when:

1. schema validates
2. pins are declared
3. model faces are declared
4. model quality is declared
5. at least one test fixture uses it
6. unsupported use cases are documented

A validation rule is done only when:

1. rule has ID
2. severity is defined
3. pass/fail condition is deterministic
4. at least one pass fixture exists
5. at least one fail fixture exists
6. report includes suggested fixes
7. documentation explains the rule

---

## 26. First task list for agents

Agents should execute in this order.

### Task A — Create repository skeleton

Create the directory structure in section 18.

### Task B — Define schemas

Create:

* `docs/board_ir.md`
* `docs/component_model_contract.md`
* `docs/report_schema.md`
* JSON schema files if practical

### Task C — Implement CLI stub

Commands:

```bash
circuitci --help
circuitci validate <project.yaml> --output <dir>
```

### Task D — Implement Board IR parser

Support YAML first.

### Task E — Implement component loader

Load generic passives and digital pin primitives first.

### Task F — Implement validation report

Output JSON and Markdown initially.

### Task G — Implement first rule

Implement `GPIO_BACKDRIVE`.

### Task H — Add first example

Add:

* `examples/bad_backdrive_board`
* `examples/good_backdrive_fixed_board`

### Task I — Add regression tests

Ensure the bad board fails and the fixed board passes.

### Task J — Add physics-complete analog simulation

The IR, reports, and rule pipeline now exist. The next core requirement is a
generic SPICE-class analog path for quantitative voltage/current analysis. The
engine must not fake analog physics with circuit-specific behavioral delays:
physical acceptance needs backend execution, device models parameterized from
datasheets or measurements, board-to-netlist bindings, waveform artifacts, and
numeric assertions.

---

## 27. Strategic warning

The project will fail if it tries to become a full EDA suite immediately.

The project will succeed if it becomes:

```text
the validation runtime that agents must pass before fabricating embedded boards.
```

Keep the core generic. Keep the first examples practical. Keep the reports machine-readable. Make every rule testable. Make every model honest about fidelity.



Yes — this is exactly the evidence that the project is **technically possible** and commercially meaningful.

Proteus proves that **MCU firmware + mixed-mode circuit simulation + virtual instruments** is a real category, not fantasy. Labcenter describes Proteus VSM as firmware running on a supported microcontroller inside a schematic while co-simulating with mixed-mode SPICE and MCU peripherals. ([Labcenter][1]) It also supports many embedded peripherals/protocols, including I2C, SPI, Ethernet, and USB in its VSM education/product materials. ([Labcenter][2])

But the opportunity is not “clone Proteus.”

The opportunity is:

```text
Proteus-like embedded simulation,
but redesigned as an agent-native validation runtime.
```

One important nuance: Proteus is not standing still. Proteus 9 introduced a new 64-bit application framework and new simulation/schematic features, and Proteus 9.1 introduced EDAi, an in-product AI assistant that understands schematic, simulation, and firmware context. ([Labcenter][3]) So the market direction is clearly real: even incumbents know AI + EDA is coming.

But your angle can still be sharply different.

## Why Proteus validates your project

Proteus proves these things are feasible:

| Feasibility claim                                                | Proteus evidence   |
| ---------------------------------------------------------------- | ------------------ |
| MCU firmware can be co-simulated with circuit behavior           | Yes, VSM does this |
| Analog + digital + firmware simulation has engineering value     | Yes                |
| Virtual instruments and waveform-based debugging are useful      | Yes                |
| Component libraries are central                                  | Yes                |
| Education/hobby/prototype users already understand this workflow | Yes                |

So the physics/engineering side is not impossible. The hard part is **product architecture, model library, and workflow design**.

## Where your tool should be different

Proteus is mainly:

```text
human GUI -> schematic -> simulate/debug inside desktop EDA
```

Your tool should be:

```text
agent/API/CLI -> import design -> run validation profiles -> emit JSON failures -> agent repairs design -> repeat
```

That difference is huge.

## Proteus-style simulator vs agent-native CircuitCI

| Area              | Proteus-like tool                | Your tool                                   |
| ----------------- | -------------------------------- | ------------------------------------------- |
| Primary user      | Human engineer/student           | AI agent + engineer                         |
| Interface         | GUI-first                        | CLI/API/CI-first                            |
| Output            | Waveforms, instruments, debug UI | Machine-readable pass/fail + repair hints   |
| Core purpose      | Interactive simulation           | Pre-fabrication validation loop             |
| Component models  | Internal product library         | Open/extensible model packs                 |
| Workflow          | Manual simulate/debug            | Automated scenario generation               |
| Integration       | Desktop EDA environment          | GitHub/Codex/CI/design-agent loop           |
| Success condition | “Simulation runs”                | “Design passes declared validation profile” |

This is the real wedge.

## The product should not say “new Proteus”

Better positioning:

```text
CircuitCI is not a circuit simulator UI.
It is a validation runtime for AI-generated embedded hardware.
```

Or:

```text
Hardware CI for agent-designed IoT boards.
```

The agent-native features are the moat:

1. **Headless validation**

   ```bash
   circuitci validate project.yaml --profile iot_basic_v0 --json out/report.json
   ```

2. **Machine-readable failures**

   ```json
   {
     "result": "fail",
     "rule": "GPIO_BACKDRIVE",
     "net": "UART_RX",
     "suggested_fixes": [
       "add series resistor",
       "add isolation",
       "move pull-up to same power domain"
     ]
   }
   ```

3. **Scenario generation**
   The tool automatically tests USB hot-plug, power sequencing, reset, boot, I2C pull-ups, sleep leakage, back-power, brownout, etc.

4. **Agent repair loop**
   The circuit-design agent reads the report, patches schematic/firmware, and reruns validation.

5. **Open model ecosystem**
   Component packs for STM32, ESP32, CH340, CP2102, FT232, LDOs, MOSFETs, sensors, batteries, motor drivers, etc.

## Strong conclusion

Your instinct is right.

Proteus proves the **category exists**.
Renode proves **embedded simulation can be CI-friendly**; its own site emphasizes CI workflows for testing embedded software. ([renode.io][4])
ngspice/Xyce prove there are open analog simulation engines available as building blocks.

The gap is:

```text
No widely adopted open, agent-native, validation-profile-driven
Circuit CI system for embedded/IoT board design.
```

That is the product.

The right strategy is not to beat Proteus on GUI polish. That is hard and unnecessary.

The right strategy is to beat traditional tools on:

```text
automation
agent feedback
machine-readable reports
validation profiles
open component packs
GitHub/CI integration
pre-fab pass/fail gates
```

That is a much more modern and winnable direction.

[1]: https://www.labcenter.com/simulation/?utm_source=chatgpt.com "Circuit Simulation Software with SPICE"
[2]: https://www.labcenter.com/education/?utm_source=chatgpt.com "A Global Standard in Education"
[3]: https://www.labcenter.com/?utm_source=chatgpt.com "Proteus: PCB Design and Circuit Simulator Software"
[4]: https://renode.io/?utm_source=chatgpt.com "Renode"


Yes — **JITX is very strong proof** for your thesis.

JITX proves that **circuit/PCB design can be represented as code**, not only as manual schematic drawing. Their own positioning is Python-based board design where requirements, stackups, SI targets, and manufacturing rules become inspectable design code; AI edits the code while JITX generates schematics/board structures and dispatches checks or simulations such as HFSS. ([jitx.com][1])

That means your project should not compete with JITX directly. It should become the **validation/runtime companion** to tools like JITX.

```text
JITX / KiCad / EasyEDA / Altium
        = design authoring / circuit generation

CircuitCI / BoardSim
        = validation, simulation, failure diagnosis, agent repair loop
```

The best architecture is therefore:

```text
Agent writes JITX/Python hardware design
        ↓
JITX generates schematic / board / netlist / constraints
        ↓
CircuitCI imports the generated artifacts
        ↓
CircuitCI runs validation profiles:
    - power-up
    - boot/reset
    - GPIO back-power
    - firmware pin init
    - I2C/SPI/UART behavior
    - brownout
    - sleep current
    - tolerance sweep
        ↓
CircuitCI emits JSON failure report
        ↓
Agent modifies JITX code
        ↓
Repeat until pass
```

This is an even stronger product wedge because JITX already validates the **code-defined hardware design workflow**. JITX even has a public `jitx-skills` repo for Claude Code workflows, including skills for JITX circuit building, component modeling, substrate modeling, pin assignment, and interconnect constraints. ([GitHub][2])

So the market signal is clear:

```text
Hardware design is becoming code.
Code-designed hardware needs CI.
Your tool can be the CI.
```

The 2026 HWE-Bench paper is also direct evidence of the pain: it benchmarks LLMs generating board-level schematics from requirements and datasheets, checks them against static rules, then simulates dynamic behavior; the authors report that current models still lack physical intuition and the top model only reached an 8.15% overall pass rate. ([arXiv][3])

That makes your positioning sharper:

```text
JITX is “hardware as code.”
CircuitCI is “testbench + simulator + judge for hardware as code.”
```

For your `PROJECT.md`, I would add a section like this:

```md
## JITX Integration Direction

CircuitCI must treat code-defined hardware tools such as JITX as first-class upstream design generators.

JITX, KiCad, EasyEDA, and Altium are design authoring tools. CircuitCI is not intended to replace them. CircuitCI consumes generated design artifacts and validates whether the resulting embedded board is electrically and behaviorally safe before fabrication.

Expected flow:

1. Agent writes or modifies JITX hardware design code.
2. JITX generates schematic, board structures, netlist, BOM, and constraints.
3. CircuitCI imports generated artifacts.
4. CircuitCI binds component models.
5. CircuitCI runs validation profiles.
6. CircuitCI produces JSON and human-readable reports.
7. Agent patches the JITX design code based on failures.
8. The loop repeats until validation passes.

CircuitCI should eventually provide:
- JITX artifact importer
- JITX model-binding helpers
- JITX validation-profile templates
- agent-readable feedback for automatic repair
- examples showing failing JITX designs and fixed JITX designs
```

This makes the product more realistic, not less. You do not need to build the whole design-generation layer first. Let JITX and other EDA tools generate designs; your tool becomes the **verification gate**.

[1]: https://www.jitx.com/ "JITX - Software-defined electronics"
[2]: https://github.com/JITx-Inc/jitx-skills "GitHub - JITx-Inc/jitx-skills · GitHub"
[3]: https://arxiv.org/abs/2603.18102 "[2603.18102] HWE-Bench: Can Language Models Perform Board-level Schematic Designs?"


Yes — that is a strong architecture.

But I would separate it carefully:

```text
Rust GUI
    = human-facing design/inspection/debug tool

Rust engine
    = deterministic validation runtime

Chip/component library
    = data + model plugins + regression tests

CLI/API
    = agent-facing interface
```

The **agent path must remain headless**. The GUI is valuable, but the project should not become GUI-first like Proteus.

## Recommended architecture

```text
CircuitCI
├── circuitci-core        # Rust simulation/validation kernel
├── circuitci-cli         # agent/CI command line interface
├── circuitci-gui         # Rust desktop GUI
├── circuitci-libs        # chip/component/model libraries
├── circuitci-sim         # ngspice/Xyce/Renode/QEMU adapters
├── circuitci-report      # JSON/HTML/Markdown reports
└── circuitci-agent-api   # machine-readable repair loop API
```

The key rule:

```text
The GUI calls the same engine as the CLI.
The CLI is not a wrapper around the GUI.
The chip library is not embedded inside the GUI.
```

That keeps the tool useful for agents, GitHub CI, and human engineers.

## Rust GUI choice

There are three practical options:

| Option    | Fit                                                                                                                                                                                      |
| --------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **egui**  | Best for fast engineering tools, inspectors, waveform panels, debug dashboards. It is an immediate-mode Rust GUI that runs natively and on web. ([GitHub][1])                            |
| **Slint** | Good for a more polished native GUI. Slint targets Rust/C++/JS/Python and supports embedded, desktop, and web use cases. ([slint.dev][2])                                                |
| **Tauri** | Good if you want a modern web-style frontend with Rust backend; Tauri v2 supports desktop and mobile targets from one codebase and lets the frontend use any web framework. ([Tauri][3]) |

My recommendation:

```text
MVP GUI: egui
Future polished GUI: Slint or Tauri
Core engine: Rust
Agent interface: CLI + JSON + gRPC/HTTP later
```

For this type of engineering/debug tool, **egui is probably the fastest path** because you need tree views, tables, waveform panels, logs, model inspectors, and scenario controls more than fancy consumer UI.

## How the GUI should look

The GUI should be a **validation workbench**:

```text
Left panel:
  project tree
  board nets
  components
  power domains
  scenarios
  validation profiles

Center:
  schematic/net graph view
  waveform viewer
  scenario timeline
  rule results

Right panel:
  selected component model
  pin states
  electrical limits
  failure explanation
  suggested fixes

Bottom:
  simulation log
  agent JSON report
  backend messages
```

The important GUI screens:

1. **Board graph viewer**
   See components, nets, power domains, unresolved models.

2. **Component model inspector**
   Inspect chip pins, voltage limits, behavior models, fidelity level, confidence.

3. **Scenario runner**
   Run `power_up`, `usb_hot_plug`, `reset_boot`, `gpio_backdrive`, etc.

4. **Waveform viewer**
   Show `VDD`, `NRST`, `BOOT0`, `GPIO`, current injection, UART events.

5. **Validation report view**
   Critical failures, warnings, suggested fixes, confidence level.

6. **Agent loop view**
   Show what the design agent should patch next.

## Chip library design

The chip library should be treated like package management.

```text
circuitci-libs/
├── generic/
│   ├── resistor
│   ├── capacitor
│   ├── diode
│   ├── bjt
│   ├── mosfet
│   ├── ldo
│   └── digital_pin
│
├── vendor/
│   ├── st/
│   │   ├── stm32l4/
│   │   ├── stm32f4/
│   │   └── stm32g0/
│   ├── espressif/
│   │   ├── esp32/
│   │   ├── esp32s3/
│   │   └── esp32p4/
│   ├── wch/
│   │   ├── ch340/
│   │   └── ch343/
│   ├── silicon_labs/
│   │   └── cp210x/
│   └── ftdi/
│       └── ft232/
│
└── profiles/
    ├── iot_basic_v0
    ├── stm32_uart_boot_v0
    ├── esp32_auto_download_v0
    └── battery_low_power_v0
```

Each chip pack should include:

```text
chip.yaml              # pins, power domains, limits
electrical.yaml        # thresholds, leakage, drive strength
behavior.rhai/wasm     # optional behavior model
rules.yaml             # validation rules
spice.lib              # optional SPICE model
tests/                 # passing/failing fixtures
docs.md                # model limitations
```

For complex chips, use multiple fidelity levels:

```text
STM32 model levels:
  L0: power pins + GPIO limits
  L1: reset/boot behavior
  L2: UART/I2C/SPI behavioral peripherals
  L3: firmware-in-loop through Renode/QEMU
  L4: measured/calibrated board model
```

This is much more scalable than trying to build one “perfect chip model.”

## Rust engine crate layout

I would structure the Rust workspace like this:

```text
circuitci/
├── crates/
│   ├── circuitci-core/
│   │   ├── board_ir
│   │   ├── component_model
│   │   ├── signal
│   │   ├── scheduler
│   │   └── diagnostics
│   │
│   ├── circuitci-validation/
│   │   ├── rule_engine
│   │   ├── profiles
│   │   └── assertions
│   │
│   ├── circuitci-sim/
│   │   ├── spice_adapter
│   │   ├── renode_adapter
│   │   ├── qemu_adapter
│   │   └── waveform
│   │
│   ├── circuitci-libs/
│   │   ├── library_loader
│   │   ├── model_schema
│   │   └── model_registry
│   │
│   ├── circuitci-cli/
│   │   └── main.rs
│   │
│   ├── circuitci-gui/
│   │   └── main.rs
│   │
│   └── circuitci-report/
│       ├── json
│       ├── markdown
│       └── html
```

## Strong product rule

The GUI should make the tool understandable, but the **engine + libraries + reports** are the real product.

The winning loop is still:

```bash
circuitci validate project.yaml \
  --profile iot_basic_v0 \
  --json out/report.json \
  --wave out/waves.vcd
```

Then:

```text
agent reads report.json
agent patches JITX/KiCad design
agent reruns CircuitCI
agent repeats until PASS
```

## Best direction

So yes:

```text
Rust engine: yes.
Rust GUI: yes.
Chip/component library: yes.
Agent-native CLI/API: mandatory.
```

The key is to build it like this:

```text
Proteus has GUI-first simulation.
JITX has hardware-as-code generation.
CircuitCI should be engine-first validation for agents,
with a Rust GUI as the human inspection/debug workbench.
```

That is a very coherent and buildable product architecture.

[1]: https://github.com/emilk/egui?utm_source=chatgpt.com "egui: an easy-to-use immediate mode GUI in Rust that runs ..."
[2]: https://slint.dev/?utm_source=chatgpt.com "Slint | Declarative GUI for Rust, C++, JavaScript & Python"
[3]: https://v2.tauri.app/?utm_source=chatgpt.com "Tauri 2.0 | Tauri"
