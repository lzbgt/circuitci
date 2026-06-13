# Common IoT Board Readiness Gaps

CircuitCI can already validate specific modeled risks on IoT-class boards:
schematic import, Board IR binding, generated SPICE transient checks,
semiconductor operating limits, functional MCU pin-observation scenarios, and
machine-readable repair findings. It is not yet a universal sign-off tool for
any arbitrary common IoT board.

This document records the missing feature groups that block broad validation.

## 1. Datasheet-Backed Component Packs

The largest gap is library depth. Broad IoT validation needs high-confidence
models for common parts:

- MCU and wireless modules such as ESP32, STM32, RP2040, nRF52, STM8, and C51
  variants.
- USB-UART bridges such as CH340, CP210x, FT232, and CMSIS-DAP/debug probes.
- Regulators, load switches, battery chargers, ideal diodes, reset supervisors,
  level shifters, ESD arrays, sensors, flash memories, crystals, LEDs, and
  small-signal discrete parts.

Without those packs, reports correctly emit `LOW_CONFIDENCE_MODEL` limitations.
A passing rule on generic metadata is useful evidence, but not full board
sign-off.

Initial datasheet-backed packs now exist for selected parts used by common IoT
bring-up paths, including WCH CH340C and Silicon Labs CP2102N board-level
USB-UART metadata, TI TXS0108E interface-protection metadata, and TI TPS22918
load-switch metadata plus Microchip MCP73831 charger metadata. TI TPS2115A and
TI TPS2121 now cover datasheet-backed power-mux source-selection,
reverse-blocking, rail-range, and output-current packs, while TI TLV803EA29
covers a first datasheet-backed reset supervisor
threshold/delay pack, and Diodes AP2112K-3.3 covers a common fixed 3.3 V LDO.
Advanced Monolithic Systems AMS1117-3.3 now covers a common 1117-style 3.3 V
LDO with its larger dropout and output-capacitance requirements. TI TPD2EUSB30
now covers a first datasheet-backed clamp-only USB ESD array for static
standoff-voltage and capacitance screening. Nexperia PRTR5V0U2X now covers a
rail-to-rail two-line USB ESD array with VCC reference validation. TI BQ24075
now covers a peer-board 1-cell linear charger with power-path pack for static
input-range, charge-current range, and input-source current-budget screening. TI
TPS62162-3.3 now covers a common 3 V to 17 V, 1 A fixed 3.3 V synchronous buck
regulator for static input-range, output-current, support-capacitance, and
direct output-inductance screening. TI TPS61023 now covers the peer-board 5 V
boost regulator path for static input/output range, support capacitance, and
direct input-inductance screening. TI TPS63802 now covers the peer-board
3.3 V buck-boost path for static input/output range, support capacitance,
output-current budget, and direct L1-L2 switch-inductor screening. Espressif
ESP32-WROOM-32E and
ESP32-S3-WROOM-1U-N16R8 now cover common Wi-Fi/Bluetooth MCU-module packs for
3.3 V rail budgeting and boot-strap screening. The gap remains broad library
depth across other MCU/wireless modules, USB-UART bridges, debug probes,
radios, sensors, regulators, advanced power muxes, reset supervisors, and protection
devices.

## 2. Power-Tree Validation

Common boards need explicit power checks beyond declared powered/unpowered
states:

- regulator dropout and load-current margin,
- startup sequencing and reset release relative to power-good,
- inrush and soft-start behavior,
- reverse current and backfeed paths,
- battery charger and USB power-path behavior,
- brownout and low-power wake behavior,
- load-transient stability and output capacitor ESR/range constraints.

Some of these can be SPICE scenarios. Others need first-class power-tree rules
that derive required checks from regulator and load metadata.

Executable slices now covered by `POWER_TREE_VALID`:

- declared powered rails,
- nominal voltage against component model power-port ranges,
- static `supply_current_limit_A` budget against declared
  `max_supply_current_A` loads,
- explicit regulator `power_conversion` dropout margin,
- explicit regulator minimum output-load requirement,
- explicit regulator maximum output-current budget,
- explicit regulator startup-delay sequencing against `power_valid_at_us` rail
  metadata,
- explicit regulator input/output support-capacitance requirements against
  Board IR capacitors to ground,
- explicit regulator direct input/output inductance requirements against Board
  IR inductors between input/switch or switch/output nets,
- explicit regulator direct switch-pin inductance requirements against Board IR
  inductors between two declared converter switch-pin nets,
- explicit load-switch `power_switch` enable-state evidence and maximum
  switched-output current budget,
- explicit or source-backed resistor-derived battery-charger
  `battery_charger` programmed-current range and input-source current budget,
- explicit power-mux `power_mux` source-selection evidence and inactive-input
  reverse-blocking checks,
- explicit reset-supervisor `reset_supervisor` threshold checks against rail
  nominal voltage and monitored-load minimum operating voltage,
- reset release checked against target rail `power_valid_at_us` plus optional
  reset-supervisor or power-good delay metadata.

The remaining gap is waveform-dependent power behavior: real ramp shape,
load-dependent dropout under waveform load, inrush, soft-start,
load-switch turn-on/reverse-current behavior, charger thermal foldback,
charge termination, battery chemistry, power-mux switchover/reverse-current
magnitude, thermal margin, and stability.

## 3. Functional MCU And Peripheral Models

MCU internals should remain functional black boxes, not transistor-level
netlists. The missing work is board-facing behavior:

- more QEMU/Renode machine coverage,
- firmware-visible peripherals and alternate-function pin modes,
- boot ROM and boot-strap behavior,
- watchdog, reset-cause, flash-layout, and low-power state behavior,
- deterministic pin-trace instrumentation,
- board-level assertions that connect firmware behavior to external circuitry.

The current QEMU path requires explicit pin observations. It does not infer
correct package-pin behavior from MCU internals.

## 4. Automatic Scenario Generation

Physical proof currently requires explicit scenarios, probes, and assertions.
For broad IoT boards, CircuitCI should suggest or generate checks for
recognized circuits:

- reset and boot straps,
- UART/SWD/JTAG/debug paths,
- USB-UART auto-download circuits,
- regulator input/output networks,
- battery charger and power mux circuits,
- GPIO protection, level shifting, and backdrive paths,
- oscillator/crystal support networks,
- MOSFET load switches and high-side/low-side drivers.

Generated scenarios must remain conservative: no assertion means no sign-off.

Executable slices: `circuitci suggest-scenarios` now emits runnable
`POWER_TREE_VALID` scenarios for projects with declared power nets and runnable
`BOOT_STRAP_BIAS_VALID` scenarios when boot straps have explicit resistor bias
evidence to power or ground, including mapped KiCad pull resistors with parsed
schematic values. Standalone `BOOT_STRAP_DEFINED` suggestions become runnable
when every required strap is directly tied to a declared powered rail or
ground, so the required state is proven without observing firmware behavior. It
also emits runnable `RESET_RELEASE_AFTER_POWER_VALID` scenarios when active-low
reset nets have explicit pull-up resistor and reset-to-ground capacitor
evidence, including mapped KiCad RC networks with parsed schematic values, or
when exactly one matching `board.runtime.reset_release[]` record supplies
explicit measured/simulated release timing. It emits runnable
`UART_BOOTLOADER_SYNC` scenarios when the target RX net has an output-capable
sender, reset/boot timing is derived from explicit RC or runtime timing
evidence, and the required boot mode is proven by a direct rail/ground strap.
Complete `board.runtime.control_line_sequences[]` evidence also emits runnable
`CONTROL_LINE_RELEASE_SEQUENCE` scenarios for reviewed host-control traces. It
emits non-runnable templates for reset release without explicit timing
evidence, observed boot-strap states that are not direct rail/ground ties, UART
bootloader sync without complete timing/strap/sender proof, and first-slice
GPIO backdrive hot-plug risks when model/connectivity evidence is present but
observations still need real evidence. Matching
`board.runtime.gpio_backdrive[]` evidence makes those GPIO backdrive templates
runnable by supplying the runtime driver state, victim mode, and schematic
series resistance. It also emits
interface-protection review templates when component models declare explicit
`signal_conditioning.channels`, and includes regulator input/output rail,
dropout/current/startup/capacitance requirements plus measured support-capacitor
evidence, reset-supervisor monitored rail, reset output, and threshold evidence
in power-tree suggestions. It marks
power-tree templates non-runnable when load-switch enable evidence is missing
and not hard-tied to a declared powered rail or ground, when charger
programmed-current evidence is missing, or when power-mux selected-source
evidence is missing and cannot be derived from an explicit board state with
exactly one powered mux input.
`INTERFACE_PROTECTION_REVIEW` now also has an executable clamp-only path for
USB ESD/protection arrays, covering reference-net kind, standoff-voltage limits,
and line-capacitance budgets when component metadata and scenario limits are
declared. `USB_CONNECTOR_PROTECTION_VALID` adds connector-level schematic
coverage for declared USB D+/D- and optional VBUS protection.
`USB_PROTECTION_PLACEMENT_VALID` adds the first explicit layout-evidence guard:
connector-to-protection center distance from `board.layout.placements`.
`USB_CONNECTOR_ORIENTATION_VALID` adds a static footprint-orientation guard
from imported connector placement `rotation_deg` evidence and explicit
mechanical/layout rotation limits. KiCad PCB import now preserves straight
`Edge.Cuts` outline segments so scenario suggestions can prefill the expected
rotation from nearest-edge outward-normal evidence when available.
`USB_CONNECTOR_EDGE_PROXIMITY_VALID` adds the corresponding executable
connector-to-board-edge distance guard from the same straight outline evidence.
It uses imported `fabrication`/`courtyard` footprint drawing extents when
available and falls back to component placement-center distance otherwise.
`USB_CONNECTOR_BODY_OVERHANG_VALID` adds an explicit 2D body/courtyard
protrusion guard from the same board-outline and footprint drawing evidence, so
connector overhang can be checked against a connector/enclosure mechanical
limit instead of being conflated with connector-to-edge proximity.
`USB_CONNECTOR_COMPONENT_CLEARANCE_VALID` adds a static connector keepout guard
from supported connector footprint graphics and other component footprint or
placement evidence, so nearby component intrusion can be screened separately
from edge proximity and overhang.
`USB_CONNECTOR_ENTRY_CLEARANCE_VALID` adds the first static cable-entry
corridor guard by projecting a 2D clearance corridor forward from the connector
body using imported placement rotation or explicit entry-direction evidence.
`USB_ROUTE_GEOMETRY_VALID` adds the first routed-geometry guard for USB data
nets: imported route length, via count, and connector-to-protection route
distance from `board.layout.routes`. It can now require imported
`board.layout.pads` evidence so connector-to-protection route distance is
measured with same-net pad evidence instead of component placement centers;
supported pad geometry is checked against imported pad copper extent.
`USB_VBUS_ROUTE_VALID` adds the matching static VBUS power-entry route guard
for route length, optional via count, optional minimum segment width, and
optional connector-to-VBUS-protection route distance.
KiCad PCB import now also carries mapped copper-zone outlines into
`board.layout.zones`. `USB_RETURN_PATH_VALID` adds the first static
return-path guard by requiring D+/D- route segment midpoints to sit inside
same-layer ground-zone outlines within the declared unreferenced-length budget.
It can also require nearby ground-net stitching vias for USB data route vias
when `max_data_via_to_ground_stitch_distance_mm` is declared, and can use saved
`filled_polygons` when `require_filled_zone_coverage` is true. It can also
screen route-midpoint margin to filled-copper polygon edges with
`min_data_line_filled_zone_edge_clearance_mm`. It can now require imported
same-net pad/via contact evidence with `require_ground_zone_contact_evidence`.
When supported pad geometry exists, pad contact is checked against imported pad
copper extent, and when filled-zone coverage is required, that pad copper or
via contact must overlap the same saved filled polygon as the route midpoint.
It still does not prove unmodeled filled-zone island continuity, adjacent-plane
return paths, stitching-via inductance, or USB eye margin.
`suggest-scenarios` now emits connector-level schematic templates automatically
from `usb_connector` metadata and connected clamp evidence. It can emit runnable
USB data-route templates when imported KiCad routing constraints provide both
route-length and pair-mismatch limits. It can also emit runnable VBUS-route
templates when imported KiCad routing constraints provide the VBUS
route-length limit. It can emit runnable USB return-path templates when
`board.layout.constraints.usb_return_path.max_data_line_unreferenced_length_mm`
is supplied. USB connector placement, orientation tolerance, edge proximity,
body-overhang, and component-clearance templates can become runnable when
explicit `board.layout.constraints.usb_connector` mechanical policy is supplied.
These
checks do not invent placement or unreferenced-return-path limits, timing,
observed strap
states, protocol events, GPIO pin-state observations, protection-path
resistance, datasheet isolation behavior, untied load-switch enable evidence,
power-mux selected-source evidence, oscillator startup margin, or analog
assertions. Broader automatic recognition for device-specific protection
behavior, VBUS current capacity and transient fuse behavior, trace-order proof,
additional datasheet-backed USB ESD arrays, ESD pulse behavior, and USB signal
integrity remain component-pack and physics gaps.

Executable clock slice: `CLOCK_SOURCE_VALID` now statically checks declared
external crystal support networks: crystal between oscillator pins, load
capacitors from both oscillator pins to ground, and effective load capacitance
against the crystal model target. `suggest-scenarios` now emits runnable clock
templates when the component model declares `clock_sources[]` and board
connectivity provides distinct oscillator nets. It does not prove oscillator
startup, ESR, drive level, ppm accuracy, or layout parasitics.

## 5. Layout-Dependent Physics

The current tool validates schematic/netlist behavior and a bounded amount of
explicit layout evidence. KiCad `.kicad_pcb` import can now populate component
center placements, matched footprint drawing evidence, routed segment/via
geometry, route constraints, and copper-zone outlines plus saved filled
polygons for matching Board IR nets, but the tool does not yet solve full PCB
layout physics. Missing layout checks
include:

- USB connector orientation, routed trace order, adjacent-plane return paths,
  stitching-via continuity, and filled-zone continuity,
- RF antenna matching, keepout, and 2.4 GHz layout,
- impedance, return path, and high-speed signal integrity,
- thermal copper and power dissipation from layout,
- creepage/clearance and manufacturing constraints,
- footprint/pin-1/BOM/PNP alignment.

These require PCB import and geometry-aware rule engines, not only schematic
connectivity.

## 6. Import Coverage Beyond KiCad

KiCad XML and native `.kicad_sch` import are covered conservatively. Common IoT
projects also need adapters for:

- EasyEDA,
- Altium,
- JITX,
- raw PCB/layout formats,
- full KiCad routed geometry beyond component placement,
- BOM and pick-and-place files,
- vendor reference-design formats.

Each adapter should fail closed on ambiguous constructs instead of guessing.

## 7. Protocol And Firmware Update Validation

Declared protocol traces are useful, but broad validation needs stronger
execution paths:

- raw UART/I2C/SPI/CAN/USB transaction decoding,
- CRC and frame validation,
- flash/storage emulation,
- bootloader command coverage,
- application update, rollback, and confirmation flows,
- fault injection around reset, brownout, and interrupted updates.

This should connect to functional firmware execution where possible.

## 8. Repair Synthesis

Reports can emit suggested fixes. Missing general repair capabilities include:

- schematic patch generation,
- value solving from electrical constraints,
- model-backed alternative-part selection,
- rerun-to-pass repair loops,
- proof that a concrete modified design artifact removes the original finding
  without introducing new critical findings.

## Practical Readiness Bar

For a common IoT board to be considered broadly verified, the project should
have:

- imported schematic and, where layout matters, imported PCB evidence,
- high-confidence component packs for every critical active part,
- explicit power-tree checks,
- generated or hand-authored SPICE scenarios for analog risk circuits,
- firmware-in-loop pin behavior checks for MCU-driven behavior,
- protocol/update checks for bootloader or field-update paths,
- no unallowed blocking limitations,
- no critical findings,
- repair evidence for each known-bad acceptance case.
