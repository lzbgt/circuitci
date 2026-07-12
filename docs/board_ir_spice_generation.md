# Board IR to SPICE Generation

CircuitCI must not depend on hand-written fixture decks for every board issue.
The analog backend still delegates nonlinear device physics to mature SPICE
engines such as ngspice, but CircuitCI should be able to generate the SPICE deck
from Board IR and component model metadata.

## Scope

This slice adds generated transient and AC/Bode decks for board-local analog subcircuits.
It is not a new simulator and must not implement SPICE numerics in Rust. Rust
only translates audited Board IR into a solver deck, records artifacts, invokes
the mature backend, and evaluates waveform assertions.

Initial primitive coverage is intentionally small in resource usage, not a toy
scope:

- resistor,
- capacitor,
- inductor,
- independent DC voltage source,
- independent pulse voltage source,
- independent DC current source,
- independent pulse current source,
- diode backed by `simulation.spice`,
- BJT NPN/PNP backed by `simulation.spice`,
- N-channel and P-channel MOSFETs backed by `simulation.spice`,
- subcircuits backed by `simulation.spice` with explicit `pin_order`.

Unsupported components in a generated deck are critical validation-input
failures. They must not be silently omitted.

## Project Contract

An `analog_transient`, `analog_ac`, or `analog_dc` scenario can use either a
hand-authored deck or generated Board IR source:

```yaml
analog:
  backend: auto
  netlist_source: generated_from_board
  generated:
    components: [VDTR, VRTS, R1, R26, R27, R8, D13, Q2, Q3, CBOOT, CNRST]
    ground_net: gnd
  model_files:
    - path: ../../models/spice/onsemi/ss8050_ss8550.lib
      sha256: ...
  node_bindings:
    - node: "0"
      net: gnd
    - node: nrst
      net: nrst
```

`netlist_source` defaults to `file` for compatibility with existing projects.
For `file`, `netlist` remains required and points to a SPICE-compatible deck.
For `generated_from_board`, `generated.components` is required and every listed
component must resolve through Board IR and component models.

Board components may include a `spice` object for primitive parameters:

```yaml
R8:
  model: generic.analog.resistor
  pins: {A: nrst, B: vdd_3v3}
  spice: {primitive: resistor, value_ohm: 10000}
```

Current-source primitives use SPICE's positive-current convention from `P` to
`N`. Use `dc_current_source` with `dc_a` for static loads and
`pulse_current_source` with `current_pulse` for pulsed loads or stress
stimuli:

```yaml
ILOAD:
  model: generic.analog.dc_current_source
  pins: {P: rail_3v3, N: gnd}
  spice: {primitive: dc_current_source, dc_a: 0.1}
```

For generated `analog_ac` scenarios, independent voltage and current source
primitives emit a unity small-signal source suffix (`AC 1`) in addition to
their declared DC or pulse operating point. This makes GUI-created Bode
observations executable without requiring users to hand-author source cards.

Capacitors may optionally declare `initial_v`. Generated ngspice wrappers add
`UIC` automatically when any selected generated capacitor has an initial
condition, so precharged storage-capacitor pulse circuits can be represented
without a hand-authored raw netlist.

Discrete semiconductors should derive their SPICE model name/type/path from the
component model's `simulation.spice` metadata. Generated Board IR scenarios may
still declare `model_files` explicitly, but validation also builds an effective
model-file set by resolving active generated components'
`simulation.spice.model_path` values from the project directory or an ancestor,
hashing the artifacts, and adding missing entries. If a scenario explicitly
declares the matching model file, that declaration must still carry its own
SHA-256 pin; the resolver does not silently repair unpinned authored evidence.
File-backed decks still depend on authored `analog.model_files` because
CircuitCI cannot know which includes are required without parsing the user deck
contract.

Compiled Verilog-A compact models should be declared as OpenVAF/OSDI artifacts
instead of opaque binary blobs. An `analog.model_files[]` entry for a compiled
OSDI shared object uses the compiled artifact path as `path`, pins it with
`sha256`, declares `artifact_format: osdi_shared_object`, and records
`source_path`, `source_sha256`, `compiler: openvaf`, `compiler_version`, and a
reproducible `compiler_command`. CircuitCI validates the source file and source
hash before solver planning, adds the Verilog-A source to report artifacts, and
fails closed if any compiler provenance field is missing. It also checks that
the compiler command invokes `openvaf`, references the declared source and OSDI
output path, and reports a build plan when the compiled artifact is missing or
its hash is stale. CI may opt into controlled rebuilds with
`CIRCUITCI_RUN_OPENVAF_BUILDS=1`; in that mode CircuitCI executes the declared
`openvaf` command directly from the project directory, never through a shell,
then revalidates the compiled artifact hash before allowing solver planning.
Generated Board IR netlists do not `.include` compiled OSDI binaries as text.
When an external ngspice backend is selected, CircuitCI emits `pre_osdi`
commands in the generated wrapper deck and records the source, wrapper, OSDI
artifact, and solver log as report artifacts. If ngspice was built without OSDI
command support, the run fails closed with the wrapper/log artifacts preserved.

Xyce compact-model plugins use a separate `analog.model_files[]` contract:
`artifact_format: xyce_adms_plugin`, `source_path`, `source_sha256`,
`compiler: xyce_adms`, `compiler_version`, a `buildxyceplugin`
`compiler_command`, `plugin_load_command` containing `-plugin`,
`xyce_version`, `xyce_adms_template_revision`, `xyce_configure_options`
including `--enable-shared` and `--enable-xyce-shareable`,
`conformance_artifact`, and `conformance_sha256`. CircuitCI verifies the
source, plugin, and conformance artifact hashes and then fails closed with
`ANALOG_MODEL_COMPILER_XYCE_PLUGIN_UNSUPPORTED`; generated netlists still do
not execute Xyce plugin loading until a real-Xyce adapter and conformance
fixture are available.

Both OpenVAF/OSDI and Xyce/ADMS compact-model artifacts may also reference a
reusable package lock with `model_package_name`, `model_package_version`,
`model_package_artifact_id`, `model_package_lock_path`, and
`model_package_lock_sha256`. The lock file can be JSON or YAML and follows
`schemas/model_package_lock.schema.json`. To avoid repeating the lock pointer
in every scenario, `model_files[]` may instead reference a pinned registry entry
with `model_package_registry_path`, `model_package_registry_sha256`, and
`model_package_registry_entry`; the registry entry supplies the package identity,
artifact id, lock path, and lock SHA-256. CircuitCI rejects partial registries,
stale registry hashes, missing entries, and registry values that conflict with
explicit scenario metadata. Valid lock and registry files are added to report
artifacts and projected into solver manifests plus report-level
`model_file_provenance[]`.

Generated analog scenarios may also declare `operating_conditions`. An ambient
temperature enables datasheet power derating when the model provides linear
derating metadata. `allow_pulse_ratings` only permits pulse-current waivers
when the pulse rating declares both pulse width and duty cycle.

The first qualified pulse-current example is
`examples/good_mosfet_qualified_pulse_current`, which uses onsemi FDMC86184
metadata. Its companion `examples/bad_mosfet_pulse_duty` proves that current
below the pulsed-current scalar still fails when pulse width or duty exceeds
the encoded datasheet limits.

Digitized MOSFET SOA curve checks are documented in
`docs/soa_operating_limits.md`; `examples/bad_mosfet_soa_violation` exercises
paired `VDS`/`ID` envelope checking against hand-digitized screening points.

## Generation Rules

1. Map Board IR nets to SPICE nodes using `node_bindings`.
2. Map the declared `ground_net` to node `0`; reject missing or conflicting
   ground bindings.
3. Emit exactly the components listed in `generated.components`, in that order.
4. Reject unknown components, unknown pins, missing pin nets, and nets without
   node bindings.
5. Reject unsupported primitives and missing required primitive parameters.
6. Include declared model files with absolute paths in the generated deck.
7. Emit MOSFETs as SPICE `M` devices with required `D`, `G`, and `S` pins.
   If a body `B` pin is declared on the board component, bind it explicitly.
   If no `B` pin is declared, tie body to source only when the component model
   declares `simulation.spice.body_pin_policy: tie_to_source_when_absent`;
   otherwise fail before solver execution.
8. Emit subcircuits as SPICE `X` devices only when the component model declares
   `simulation.spice.pin_order`; a `.subckt` without deterministic pin mapping
   is a validation-input failure. Subcircuit models may also declare
   `simulation.spice.instance_parameters` to map numeric Board IR component
   parameters into SPICE instance assignments such as `ICHG_A=2`; a mapping may
   declare `default_value` for observation defaults that are still visible in
   component-model metadata.
9. Require every generated semiconductor or subcircuit model file to appear in
   `analog.model_files` with a SHA-256 pin. If the model file is a compiled
   OpenVAF/OSDI artifact, also require source path/hash, compiler identity,
   compiler version, and compiler command provenance.
10. Resolve model metadata paths from the Board IR project directory and its
    ancestors so CLI launch location does not change the physical model.
11. Prepare generated source decks before solver backend selection so Board IR,
    body-pin, subcircuit pin-order, and model-provenance contract errors are
    visible even on hosts without `ngspice` or `Xyce` installed.
12. Emit generated deck, wrapper, solver log, and waveform as report artifacts.
13. Keep all solver execution, convergence checks, waveform parsing, and
   assertion evaluation in the existing ngspice runner path.
14. Evaluate generated semiconductor operating limits with any declared
   scenario `operating_conditions`; fail closed when temperature or pulse
   metadata is incomplete.

## Review Notes

- Schema compatibility: `netlist_source` must be additive and default to `file`.
  Existing projects that declare `netlist` continue to work.
- Schema enforcement: file-backed scenarios require `netlist`; generated
  scenarios require `generated`. Runtime validation repeats this and fails
  closed so malformed projects cannot reach the solver as partial decks.
- Rust model access: component-library loading must deserialize
  `simulation.spice`; the generator must not reparse model YAML or hardcode
  semiconductor model names.
- Board topology: generated physical decks require explicit Board IR components
  and per-instance values for passives, sources, and device pins. Missing R/C/D
  or stimulus components are validation failures, not inferred shortcuts.
- Evidence quality: generated netlists are artifacts, not temporary invisible
  implementation details. A report must be reproducible from the emitted deck
  and model files.
- Model provenance: generation must not pass if a semiconductor component lacks
  `simulation.spice` metadata or a declared model file hash fails.
- Physical honesty: if a component model is low confidence or estimated, the
  existing limitation mechanism remains visible in the report.

## Contract Fixtures

- `examples/good_mosfet_low_side_switch` proves generated N-channel MOSFET `M`
  device emission with a SHA-pinned datasheet-fit NDS7002A model.
- `examples/good_csd17484f4_low_side_switch` proves generated N-channel MOSFET
  `M` device emission with the SHA-pinned TI CSD17484F4 datasheet-fit model
  under a TOF-style `21.8 V`, `30 ns`, `30 kHz` trigger condition.
- `examples/good_csd17484f4_vcsel_capacitor_discharge` proves generated
  capacitor `IC=` emission and `tran ... uic` execution for a precharged
  C27-style VCSEL pulse-discharge path through the same Q2 model.
- `examples/good_pmos_high_side_switch` proves generated P-channel MOSFET `M`
  device emission with a SHA-pinned datasheet-fit BSS84 model.
- `examples/good_subckt_rc_delay` proves generated subcircuit `X` device
  emission from explicit `simulation.spice.pin_order` metadata.
- `examples/good_bq25798_nvdc_observation` proves generated subcircuit `X`
  device emission can append model-declared instance parameters from Board IR
  component parameters for a reduced BQ25798 NVDC charger observation.
- `examples/good_ideal_opamp_buffer` proves reusable generic behavioral
  macro-model packs can drive generated Board IR decks through the same
  subcircuit, model-file, and SHA-pinned artifact path used by vendor models.
  The same fixture is registered as a direct-open GUI scope example with routed
  schematic metadata for op-amp buffer observation.
- `examples/good_wch_ch340c_usb_uart_observation` proves the WCH CH340C
  datasheet-backed USB-UART bridge model can use a reduced generated-SPICE face
  for VCC, TXD, DTR#, and RTS# output-state observations. The model-state
  inputs are explicit Board IR component parameters, not inferred USB protocol
  behavior.
- `examples/good_wch_ch340n_usb_uart_observation` proves the WCH CH340N SOP-8
  variant can use its own reduced generated-SPICE face for VCC, TXD, and RTS#
  output-state observations without exposing DTR#.
- `examples/good_silabs_cp2102n_usb_uart_observation` proves the Silicon Labs
  CP2102N datasheet-backed USB-UART bridge model can use a reduced
  generated-SPICE face for VREGIN, generated VDD/VIO, TXD, RTS, and DTR
  output-state observations. The model-state inputs are explicit Board IR
  component parameters, not inferred USB protocol behavior.
- `examples/good_ftdi_ft232r_usb_uart_observation` proves the FTDI FT232R
  source-backed USB-UART bridge model can use a reduced generated-SPICE face
  for VCC, generated 3V3OUT/VCCIO, TXD, RTS#, and DTR# output-state
  observations. The model-state inputs are explicit Board IR component
  parameters, not inferred USB protocol or EEPROM/CBUS configuration behavior.
- `examples/good_wch_ch347_usb_jtag_observation` proves the WCH CH347
  source-backed USB-JTAG/debug bridge model can use a reduced generated-SPICE
  face for VCC, UART1 TXD, and JTAG TMS/TCK/TDI/TRST line-state observations.
  The model-state inputs are explicit Board IR component parameters, not
  inferred USB enumeration, JTAG TAP state, or driver-mode behavior.
- `examples/good_cmsis_dap_swd_probe_observation` proves the generic
  CMSIS-DAP SWD probe model can use a reduced generated-SPICE face for
  VTREF-referenced SWCLK, SWDIO, nRESET, and SWO line-state observations. The
  model-state inputs are explicit Board IR component parameters, not inferred
  USB transport, SWD protocol transfer, or probe-vendor electrical behavior.
- `examples/good_stm32l431_boot_uart_swd_observation` proves the ST
  STM32L431 MCU model can use a reduced generated-SPICE face for preliminary
  VDD, NRST, BOOT0, USART1 PA9/PA10, and SWD PA13/PA14 line-state
  observations. The model-state inputs are explicit Board IR component
  parameters, not inferred firmware execution, boot ROM timing, SWD
  transactions, flash programming effects, package mapping, layout, thermal, or
  EMC behavior.
- `examples/good_nordic_nrf52840_board_observation` proves the Nordic
  nRF52840 MCU model can use a reduced high-impedance generated-SPICE face for
  preliminary VDD, VDDH, VBUS, reset, SWD, UART/GPIO idle, USB boundary, and
  antenna feed-state observations. The macro observes explicit board sources,
  pull resistors, and loads, not firmware execution, reset UICR programming,
  GPIO thresholds or drive strength, USB protocol, RF behavior, antenna
  matching, DCDC support networks, thermal behavior, or transient current
  waveforms.
- `examples/good_jst_xh_servo_connector_observation` and
  `examples/good_jst_vh_actuator_bus_connector_observation` prove the
  source-backed JST XH/VH connector models can use reduced generated-SPICE
  pass-through contact faces. The examples bind explicit load-side pins and
  observe voltage drop from the datasheet 20 mOhm post-test/environment contact
  resistance; cable resistance, crimp quality, temperature rise, retention,
  vibration, CAN signal integrity, and harness qualification stay outside the
  model.
- `examples/good_esp32_s3_wroom_boot_usb_observation` proves the Espressif
  ESP32-S3-WROOM-1U-N16R8 module model can use a reduced generated-SPICE face
  for 3.3 V supply, EN release, GPIO0/GPIO46 boot straps, USB D-/D+ line-state,
  and TXD0 idle-state observations. The model-state inputs are explicit Board
  IR component parameters, not inferred firmware, ROM boot protocol, USB PHY,
  RF, or peak-current behavior.
- `examples/good_esp32_wroom_32e_boot_uart_observation` proves the Espressif
  ESP32-WROOM-32E module model can use a reduced generated-SPICE face for
  3.3 V supply, EN release, GPIO0/GPIO2 boot straps, TXD0 idle state, and RXD0
  high-impedance connectivity observations. The model-state inputs are
  explicit Board IR component parameters, not inferred firmware, ROM serial
  protocol, RF, peak-current, or flash/PSRAM mux behavior.
- `examples/good_sipeed_licheerv_nano_w_observation` proves the Sipeed
  LicheeRV-Nano-W module model can use a reduced generated-SPICE face for
  preliminary 5 V module power, UART0 TX/RX line-state, motion-enable output,
  and fault-IRQ input observations. The model-state inputs are explicit Board
  IR component parameters, not inferred Linux boot, firmware, USB/MIPI, RF,
  high-speed, or thermal behavior.
- `examples/good_artery_at32f435_motion_core_observation` proves the Artery
  AT32F435 motion-core MCU model can use a reduced generated-SPICE face for
  preliminary VDD, LicheeRV UART, motion-enable/fault, CAN, RS-485, and servo
  PWM enable line-state observations. The model-state inputs are explicit
  Board IR component parameters, not inferred firmware, protocol timing, ADC,
  motor-control, package, layout, or thermal behavior.
- `examples/good_artery_at32m416_motor_control_observation` proves the Artery
  AT32M416 motor-control MCU model can use a reduced generated-SPICE face for
  preliminary VDD, CAN, six PWM outputs, DRV8323-style enable/fault/SPI lines,
  current-sense nodes, encoder inputs, board enable, and fault-output
  observations. The model-state inputs are explicit Board IR component
  parameters, not inferred firmware, PWM timing, ADC conversion/current
  reconstruction, FOC loops, dead-time, package, layout, or thermal behavior.
- `examples/good_ti_txs0108e_level_shifter_observation` proves the TI TXS0108E
  datasheet-backed level-shifter model can use a reduced generated-SPICE face
  for an enabled A1-to-B1 mixed-voltage observation with rail, OE, input, and
  translated-output checks.
- `examples/good_onsemi_nl27wz17_logic_buffer_observation` proves the onsemi
  NL27WZ17 datasheet-backed logic-buffer model can use a reduced
  generated-SPICE face for VCC, 1A/2A input-state, and non-inverted 1Y/2Y
  output-state checks. The model-state inputs are explicit Board IR component
  parameters, not inferred Schmitt-trigger threshold, delay, loading, or
  signal-integrity behavior.
- `examples/good_tpd2eusb30_usb_esd_observation` proves the TI TPD2EUSB30
  datasheet-backed USB ESD model can use a reduced generated-SPICE face for
  normal-operation D+/D- standoff checks with the source-backed 0.7 pF
  line-capacitance load.
- `examples/good_nexperia_prtr5v0u2x_usb_esd_observation` proves the Nexperia
  PRTR5V0U2X datasheet-backed rail-to-rail USB ESD model can use a reduced
  generated-SPICE face for normal-operation VBUS, IO1, and IO2 standoff checks
  with source-backed IO/VCC capacitance loads.
- `examples/good_ti_esd2can24_q1_can_esd_observation` proves the TI
  ESD2CAN24-Q1 datasheet-backed CAN ESD model can use a reduced generated-SPICE
  face for normal-operation CANH/CANL standoff checks with the source-backed
  3 pF line-capacitance load.
- `examples/good_ti_tcan3413_can_transceiver_observation` proves the TI
  TCAN3413 datasheet-backed CAN transceiver model can use a reduced
  generated-SPICE face for VCC, VIO, TXD, STB, RXD, and CANH/CANL line-state
  checks. The model-state inputs are explicit Board IR component parameters,
  not inferred CAN protocol, termination, or cable behavior.
- `examples/good_drv8323_gate_driver_observation` proves the TI DRV8323
  source-backed three-phase gate-driver model can use a reduced
  generated-SPICE face for VM, DVDD, ENABLE, nFAULT, SDO, and SOA/SOB/SOC
  current-sense output-presence checks. The model-state inputs are explicit
  Board IR component parameters, not inferred SPI/protection behavior,
  half-bridge gate-drive dynamics, motor behavior, or current-sense accuracy.
- `examples/good_pca9685_pwm_driver_observation` proves the NXP PCA9685
  source-backed PWM-driver model can use a reduced generated-SPICE face for
  VDD/OE, idle SCL/SDA, and representative low-load PWM output checks. The
  frequency, duty, and I2C idle-state inputs are explicit Board IR component
  parameters, not inferred I2C protocol/register behavior, oscillator
  tolerance, load-current behavior, servo dynamics, or final PWM timing signoff.
- `examples/good_tdk_icm42688p_imu_observation` proves the TDK InvenSense
  ICM-42688-P source-backed IMU model can use a reduced generated-SPICE face
  for VDD/VDDIO, host-driven SPI line-state, SDO, and INT1 observations. The
  output-state inputs are explicit Board IR component parameters, and the host
  SPI input states are explicit source components, not inferred register
  protocol, sampling, FIFO, sensor-dynamics, noise, or final SPI timing
  behavior.
- `examples/good_bosch_bme280_i2c_observation` proves the Bosch BME280
  source-backed environmental sensor model can use a reduced high-impedance
  generated-SPICE face for VDD/VDDIO, I2C pull-up, `CSB` interface-select, and
  `SDO` address-select observations. The macro observes board wiring and does
  not emulate measurements, registers, compensation formulas, bus transactions,
  timing, noise, or calibration.
- `examples/good_sensirion_sht31_i2c_observation` proves the Sensirion
  SHT31-DIS source-backed humidity/temperature sensor model can use a reduced
  high-impedance generated-SPICE face for VDD, I2C pull-ups, `ADDR` address
  select, `nRESET`, and `ALERT` idle-state observations. The macro observes
  board wiring and does not emulate measurements, registers, compensation
  formulas, bus transactions, clock stretching, alert thresholds, heater
  behavior, drift, or calibration.
- `examples/good_aosong_aht20_i2c_observation` proves the Aosong AHT20
  source-backed humidity/temperature sensor model can use the generated-SPICE
  resolver path to find and SHA-pin a reduced high-impedance model file from
  `simulation.spice.model_path`, bind pins in declared order, and observe VDD
  plus idle SDA/SCL pull-up line states. The macro observes board wiring and
  does not emulate measurements, calibration, command protocol, conversion
  timing, power-on readiness, bus timing, self-heating, contamination/recovery,
  reflow drift, or environmental-chamber behavior.
- `examples/good_winbond_w25q64jv_spi_flash_observation` proves the Winbond
  W25Q64JV source-backed SPI/QSPI NOR flash model can use a reduced
  high-impedance generated-SPICE face for VCC, standby `/CS`, `/WP`,
  `/HOLD or /RESET`, `CLK`, `DI/IO0`, and `DO/IO1` line-state observations.
  The macro observes board biasing and does not emulate commands, JEDEC ID,
  SFDP, memory contents, erase/program state, XIP, retention, endurance, or
  signal integrity.
- `examples/good_microchip_at24c02c_i2c_eeprom_observation` proves the
  Microchip AT24C02C source-backed I2C EEPROM model can use a reduced
  high-impedance generated-SPICE face for VCC, idle `SDA`/`SCL`, `A0`/`A1`/`A2`
  address-select, and `WP` write-protect observations. The macro observes
  board biasing and does not emulate I2C transactions, acknowledge polling,
  EEPROM contents, write-cycle timing, retention, endurance, or signal
  integrity.
- `examples/good_microchip_mcp23017_i2c_gpio_expander_observation` proves the
  Microchip MCP23017 source-backed I2C GPIO expander model can use a reduced
  high-impedance generated-SPICE face for VDD, idle `SDA`/`SCL`, `A0`/`A1`/`A2`
  address-select, `RESET`, `INTA`/`INTB`, and representative `GPA0`/`GPB0`
  line-state observations. The macro observes board biasing and does not
  emulate I2C transactions, register configuration, GPIO direction,
  interrupt-on-change logic, weak pull-up behavior, output-load sign-off, or
  signal integrity.
- `examples/good_microchip_atecc608a_i2c_secure_element_observation` proves
  the Microchip ATECC608A source-backed I2C secure-element model can use a
  reduced high-impedance generated-SPICE face for VCC and idle `SDA`/`SCL`
  pull-up observations. The macro observes board biasing and does not emulate
  cryptographic commands, key storage, provisioning state, RNG behavior,
  secure boot policy, wake/sleep timing, single-wire operation, I2C transaction
  content, firmware, or signal integrity.
- `examples/good_nuvoton_nau7802_bridge_adc_observation` proves the Nuvoton
  NAU7802 source-backed bridge ADC model can use a reduced high-impedance
  generated-SPICE face for DVDD/AVDD rails, reference, bridge-input, idle
  `SCLK`/`SDIO`, and `DRDY` line-state observations. The macro observes board
  biasing and does not emulate ADC conversion codes, PGA gain, calibration,
  register protocol, I2C timing, streaming data mode, bridge excitation
  accuracy, oscillator behavior, temperature sensing, or metrology accuracy.
- `examples/good_ti_esds552_rs485_esd_observation` proves the TI ESDS552
  datasheet-backed RS-485/RS-422 ESD/surge model can use a reduced
  generated-SPICE face for normal-operation A/B standoff checks with the
  source-backed 11 pF maximum line-capacitance load.
- `examples/good_ti_thvd1450_rs485_transceiver_observation` proves the TI
  THVD1450 datasheet-backed RS-485 transceiver model can use a reduced
  generated-SPICE face for VCC, DI, DE, RE_N, RO, and A/B line-state checks.
  The model-state inputs are explicit Board IR component parameters, not
  inferred RS-485 protocol, termination, or cable behavior.
- `examples/good_tps54331_5v_buck_observation` proves the TI TPS54331
  datasheet-backed buck-regulator model can use a reduced generated-SPICE face
  in a direct-open GUI example with routed schematic metadata, VIN/EN/VSENSE
  voltage probes, load-current probes, and executable preliminary rail checks.
- `examples/good_tps62162_3v3_buck_observation` proves the TI TPS62162
  datasheet-backed fixed 3.3 V buck-regulator model can use the same reduced
  generated-SPICE pattern with VIN/EN/VOS probes, load-current probes, and
  executable preliminary rail checks.
- `examples/good_tps63802_3v3_buck_boost_observation` proves the TI TPS63802
  datasheet-backed 3.3 V buck-boost model can use a reduced generated-SPICE
  face with VIN/EN/VOUT probes, load-current probes, and executable preliminary
  rail checks.
- `examples/good_tps61023_5v_boost_observation` proves the TI TPS61023
  datasheet-backed 5 V boost model can use a reduced generated-SPICE face with
  VIN/EN/VOUT probes, load-current probes, and executable preliminary rail
  checks.
- `examples/good_ams1117_3v3_ldo_observation` proves the AMS1117-3.3
  datasheet-backed fixed LDO model can use a reduced generated-SPICE face with
  VIN/VOUT probes, a 22 uF output capacitor, load-current probes, and executable
  preliminary rail/minimum-load checks.
- `examples/good_tps2121_power_mux_observation` proves the TI TPS2121
  datasheet-backed power-mux model can use a reduced generated-SPICE face with
  IN1/IN2/OUT probes, load-current probes, and executable preliminary
  selected-source rail checks.
- `examples/good_tps2115a_power_mux_observation` proves the TI TPS2115A
  datasheet-backed autoswitching power-mux model can use a reduced
  generated-SPICE face with IN1/IN2/OUT probes, load-current probes, and
  executable preliminary selected-source rail checks.
- `examples/comparator_threshold_scope` proves the generic comparator
  macro-model in a direct-open GUI example with routed schematic metadata,
  named scope probes, and executable threshold/output-state waveform checks.
- `examples/good_tps22918_load_switch_observation` proves the TI TPS22918
  datasheet-backed load-switch model can use a reduced generic generated-SPICE
  face in a direct-open GUI example with routed schematic metadata, switched
  rail voltage probes, branch-current probes, and executable load-path checks.
- `examples/good_tps25948_efuse_observation` proves the TI TPS25948
  source-backed eFuse/load-switch model can use a reduced generic
  generated-SPICE face in a direct-open GUI example with routed schematic
  metadata, protected-rail voltage probes, branch-current probes, and
  executable load-path checks.
- `examples/good_tps24751_hot_swap_observation` proves the TI TPS24751 +
  CSD17501Q5A source-backed hot-swap/reverse-blocking model can use a reduced
  generic generated-SPICE face in a direct-open GUI example with routed
  schematic metadata, protected-rail voltage probes, branch-current probes, and
  executable load-path checks.
- `examples/good_mcp73831_charger_observation` proves the Microchip MCP73831-2
  datasheet-backed charger model can use a reduced generic generated-SPICE face
  in a direct-open GUI example with routed schematic metadata, PROG resistor,
  battery-node voltage probes, charge-current probes, and executable charger
  checks.
- `examples/good_bq24075_power_path_observation` proves the TI BQ24075
  datasheet-backed power-path charger model can use a reduced generic
  generated-SPICE face in a direct-open GUI example with routed schematic
  metadata, ISET resistor, OUT/BAT voltage probes, charge-current probes, and
  executable power-path charger checks.
- `examples/good_bq25798_nvdc_observation` proves the TI BQ25798
  datasheet-backed buck-boost/NVDC charger model can map Board IR component
  parameters into a reduced generated-SPICE face in a direct-open GUI example
  with routed schematic metadata, SYS/BAT voltage probes, charge-current
  probes, and executable preliminary charger observation checks.
- `examples/loop_stability_bode_scope` proves file-backed AC/Bode loop-gain
  observation in a direct-open GUI example with routed schematic metadata,
  Bode artifact export, and executable phase/gain margin checks.
- `examples/bad_mosfet_missing_body_policy` proves a three-pin MOSFET fails
  closed when the model does not explicitly allow body-to-source tying.
- `examples/bad_mosfet_model_missing_sha` proves generated device models must
  be SHA-pinned in `analog.model_files`.
- OpenVAF/OSDI analog model compiler fixtures prove compiled Verilog-A compact
  model artifacts must carry source and compiler provenance before any analog
  analysis can use them as physical evidence.
- `examples/bad_mosfet_missing_operating_ratings` proves generated MOSFET/BJT
  semiconductor models must carry usable absolute-maximum ratings before their
  simulations can be accepted as physical evidence.
- `examples/bad_subckt_wrong_pin_order` proves wrong subcircuit pin ordering can
  be detected by quantitative waveform assertions.
- `examples/bad_mosfet_overcurrent` proves generated MOSFET drain current and
  power can be checked automatically against datasheet absolute maximum ratings
  without a hand-authored current-limit assertion.
- `examples/bad_pmos_overcurrent` proves signed negative P-channel datasheet
  current ratings are preserved in the report while evaluated by absolute
  magnitude.
- `examples/bad_bjt_overcurrent` proves generated BJT collector current can be
  checked automatically against datasheet absolute maximum ratings without a
  hand-authored transistor-limit assertion.
- `examples/good_onsemi_2n3904_low_side_switch` and
  `examples/bad_onsemi_2n3904_collector_overcurrent` prove a source-backed
  common NPN transistor model can pass normal generated-SPICE switching and
  fail closed on datasheet collector-current stress.
- `examples/good_onsemi_2n3906_high_side_switch` and
  `examples/bad_onsemi_2n3906_collector_overcurrent` prove the matching PNP
  polarity path can pass generated-SPICE high-side switching and fail closed
  while preserving signed collector-current ratings in the report.
- `examples/bad_diode_overcurrent`, `examples/bad_diode_reverse_voltage`, and
  `examples/bad_kingbright_apt1608surck_led_overcurrent` prove generated diode
  and LED forward-current, reverse-voltage, and power stress can be checked
  automatically against datasheet absolute maximum ratings.
- `examples/good_onsemi_1n5819_schottky_rectifier` and
  `examples/bad_onsemi_1n5819_overcurrent` prove the same diode operating-limit
  path works for a source-backed Schottky rectifier whose datasheet average
  rectified current is mapped into the Board IR `IF_AV` rating key.
- `examples/good_nexperia_pesd5v0s1ul_vbus_esd` and
  `examples/bad_nexperia_pesd5v0s1ul_vbus_capacitance` prove a source-backed
  single-line VBUS ESD clamp through the static interface-protection path. The
  reduced generated-SPICE metadata is intentionally limited to normal-operation
  standoff/capacitance evidence, not ESD pulse simulation.

## Datasheet Operating Limits

For generated Board IR decks, CircuitCI augments the ngspice waveform export
with automatic probes derived from component-model
`datasheet.absolute_maximum_ratings`:

- MOSFET `VDSS`, `VGSS`/`VGSS_continuous`, `ID`/`ID_continuous`, and `PD`.
- BJT `VCEO`, `VCBO`, `VEBO`, `IC`, and `PD`.
- Diode `VRRM`/`VR`, `IF`/`IF_AV`, and `PD`/`Ptot`.

Generated MOSFET/BJT/diode models fail closed if these rating groups are absent
or use the wrong unit, because a missing datasheet limit is not pass evidence.
The operating-limit probes are evaluated over the full transient using maximum
stress magnitude. Exceeding a rating emits `SPICE_OPERATING_LIMIT` with the
component id, datasheet rating key, expression, measured maximum, time of
maximum, unit, signed datasheet rating value, and absolute comparison limit.
These checks are supplemental to scenario assertions: a circuit can pass its
functional voltage/current assertions and still fail because the selected part
is overstressed.
