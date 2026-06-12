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
- explicit regulator maximum output-current budget,
- explicit regulator startup-delay sequencing against `power_valid_at_us` rail
  metadata.

The remaining gap is waveform-dependent power behavior: real ramp shape,
load-dependent dropout under waveform load, inrush, soft-start,
charger/power-mux behavior, thermal margin, and stability.

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

## 5. Layout-Dependent Physics

The current tool validates schematic/netlist behavior, not PCB layout physics.
Missing layout checks include:

- USB differential pair constraints and connector orientation,
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
