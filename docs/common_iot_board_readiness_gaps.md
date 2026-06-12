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
load-switch metadata plus Microchip MCP73831 charger metadata. TI TPS2115A now
covers a first datasheet-backed power-mux source-selection and reverse-blocking
pack, TI TLV803EA29 covers a first datasheet-backed reset supervisor
threshold/delay pack, and Diodes AP2112K-3.3 covers a common fixed 3.3 V LDO.
Advanced Monolithic Systems AMS1117-3.3 now covers a common 1117-style 3.3 V
LDO with its larger dropout and output-capacitance requirements. TI TPD2EUSB30
now covers a first datasheet-backed clamp-only USB ESD array for static
standoff-voltage and capacitance screening. Nexperia PRTR5V0U2X now covers a
rail-to-rail two-line USB ESD array with VCC reference validation. The gap
remains broad library depth across other USB-UART bridges, debug probes,
radios, sensors, regulators, power muxes, reset supervisors, and protection
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
- explicit load-switch `power_switch` enable-state evidence and maximum
  switched-output current budget,
- explicit battery-charger `battery_charger` programmed-current range and
  input-source current budget,
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
schematic values. It also emits runnable `RESET_RELEASE_AFTER_POWER_VALID`
scenarios when active-low reset nets have explicit pull-up resistor and
reset-to-ground capacitor evidence, including mapped KiCad RC networks with
parsed schematic values. It emits non-runnable templates for reset release
without RC evidence, observed boot-strap states, UART bootloader sync, and
first-slice GPIO backdrive hot-plug risks when model/connectivity evidence is
present but observations still need real evidence. It also emits
interface-protection review templates when component models declare explicit
`signal_conditioning.channels`, and includes regulator input/output rail,
dropout/current/startup/capacitance requirements plus measured support-capacitor
evidence, reset-supervisor monitored rail, reset output, and threshold evidence
in power-tree suggestions. It marks
power-tree templates non-runnable when load-switch enable, charger
programmed-current, or power-mux selected-source evidence is missing.
`INTERFACE_PROTECTION_REVIEW` now also has an executable clamp-only path for
USB ESD/protection arrays, covering reference-net kind, standoff-voltage limits,
and line-capacitance budgets when component metadata and scenario limits are
declared. `USB_CONNECTOR_PROTECTION_VALID` adds connector-level schematic
coverage for declared USB D+/D- and optional VBUS protection.
`USB_PROTECTION_PLACEMENT_VALID` adds the first explicit layout-evidence guard:
connector-to-protection center distance from `board.layout.placements`.
`USB_ROUTE_GEOMETRY_VALID` adds the first routed-geometry guard for USB data
nets: imported route length, via count, and connector-to-protection route
distance from `board.layout.routes`. `USB_VBUS_ROUTE_VALID` adds the matching
static VBUS power-entry route guard for route length, via count, optional
minimum segment width, and connector-to-VBUS-protection route distance.
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
When filled-zone coverage is required, the contact must be in the same saved
filled polygon as the route midpoint. It still does not prove unmodeled
filled-zone island continuity, adjacent-plane return paths, stitching-via
inductance, or USB eye margin.
`suggest-scenarios` now emits connector-level schematic templates automatically
from `usb_connector` metadata and connected clamp evidence, and emits
non-runnable placement, route-geometry, VBUS-route, and return-path templates
when the connector, required protection components, imported routes, and
ground-zone outlines provide enough evidence. These checks do not invent
placement or unreferenced-return-path limits, timing, observed strap
states, protocol events, GPIO pin-state observations, protection-path
resistance, datasheet isolation behavior, load-switch enable evidence,
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
center placements, routed segment/via geometry, route constraints, and
copper-zone outlines plus saved filled polygons for matching Board IR nets, but
the tool does not yet solve full PCB layout physics. Missing layout checks
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
