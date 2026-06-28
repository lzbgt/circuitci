# Agent Board-Validation Usefulness Backlog

CircuitCI is already useful for modeled, evidence-backed board checks. An agent
can import common artifacts, bind component models, request scenario
suggestions, run validation, inspect stable JSON findings, and track
limitations. The remaining work is what moves the tool from useful on covered
risks to broadly useful on arbitrary IoT boards.

## Priority 1: Datasheet-Backed Component Packs

This is the highest-leverage gap because every validation rule depends on
trustworthy component metadata. Generic models should keep producing
limitations; they must not imply full sign-off.

Useful next packs:

- MCU and wireless modules beyond the current ESP32 and STM32 acceptance paths.
- USB-UART/debug bridges such as CH340 variants, CP210x variants, FT232, and
  CMSIS-DAP/debug probe circuits.
- Common regulators, chargers, power muxes, reset supervisors, level shifters,
  ESD arrays, sensors, flash memories, crystals, LEDs, and small-signal
  discretes seen on real boards.

Done means:

- source documents are saved under `docs/`,
- model metadata is source-pinned and conservative,
- passing and failing fixtures prove the modeled limits execute,
- low-confidence or incomplete packs still emit limitations.

## Priority 2: Agent Repair Loop

Reports already carry suggested fixes, and suites can compare bad and fixed
cases. The first concrete patch-and-rerun workflow is now available for Board
IR YAML `INVALID_POWER_DOMAIN`, `NET_NOT_FOUND`, `PIN_NOT_DECLARED`, and
`REQUIRED_PIN_FLOATING` binding findings through `circuitci repair-yaml`.

Implemented slice:

- select Board IR YAML as the first artifact family,
- generate a machine-readable repair proposal for `INVALID_POWER_DOMAIN`,
  `NET_NOT_FOUND`, `PIN_NOT_DECLARED`, and `REQUIRED_PIN_FLOATING`,
- apply the patch to a copied `project.yaml`,
- rerun validation,
- report whether the original finding disappeared across failures, warnings,
  and infos without new critical findings,
- report guarded no-op and ambiguous cases in `repair_report.json`/`.md` via
  `messages[]`, `reason_codes[]`, `summary.blocked`, `summary.skipped`, and
  proposal-level `reason_code` values,
- support `repair-yaml --dry-run`, which writes the original validation report
  and repair proposals without writing a repaired copy or rerunning validation,
- support `repair-yaml --apply-report`, which consumes a previous dry-run
  `repair_report.json`, verifies the project, profile, finding, original
  matching findings, and regenerated proposal list still match, then applies
  exactly those proposed edits to a copied project and reruns validation,
- support selective report-driven apply with repeated `--proposal-id` values,
  preserving the stale-report guards while marking non-selected proposals as
  skipped in the apply report.

Do not start with arbitrary schematic or PCB editing. That remains too broad
until several narrow YAML repair loops are proven end to end.

## Priority 3: Import More Real Evidence

KiCad import is strong, and JLC/EasyEDA fabrication import is useful for
manufacturing checks. Arbitrary board validation still needs more source
adapters and richer evidence normalization.

Useful next slices:

- EasyEDA Pro `.eprj2` evidence normalization beyond the current SQLite
  envelope/table/object-hash/payload-hash/plaintext-shape manifest,
- broader EasyEDA schematic/PCB import,
- BOM and pick-and-place normalization beyond the current
  source/row/component/side-confidence/orientation-confidence assembly
  manifest,
- vendor reference-design adapters,
- package/footprint semantics beyond the current KiCad pin-1,
  body/courtyard-bounds, connector-entry, and raw pad-level mask/paste
  fabrication override evidence,
- board/order metadata extraction workflows beyond the current reviewed
  manufacturing metadata CSV import manifest.

Done means imported evidence is represented in Board IR with provenance and
ambiguous constructs fail closed.

## Priority 4: Runtime And Firmware Evidence

Static checks cover reset, boot straps, UART bootloader sync, GPIO backdrive,
and control-line sequence evidence when the required observations are present.
Broad board validation needs stronger execution and trace paths.

Useful next slices:

- more QEMU or Renode board machines,
- deterministic firmware pin traces,
- protocol decoders for UART/I2C/SPI/CAN/USB transactions,
- bootloader command coverage,
- update, rollback, and interrupted-update scenarios,
- brownout and reset fault-injection traces.

Done means runtime observations become Board IR or scenario evidence that can
make suggestions runnable without inventing behavior.

## Priority 5: Layout Physics Boundaries

CircuitCI now has executable USB connector, route, VBUS, same-layer return-path,
filled-zone, stitching-via, and manufacturing geometry screens when explicit
policy exists. It is not a field solver.

Useful next slices:

- controlled impedance metadata and static rule checks,
- stackup-aware adjacent-plane return-path evidence,
- RF antenna keepout and matching evidence,
- thermal copper and dissipation evidence,
- creepage and clearance screens beyond current fabrication checks,
- footprint, pin-1, BOM, and PNP alignment checks.

Done means each rule has explicit imported geometry or metadata, not guessed
layout intent.

## Priority 6: Complex Geometry

Gerber disjoint multi-contour regions import as separate simple polygons.
Nested or overlapping contours still fail closed because Board IR region
polygons do not represent holes.

Do not add hole-bearing region import alone. It is useful only if the Board IR,
schemas, ownership matching, measurement helpers, and affected validators all
consume hole-bearing geometry correctly.

## Practical Agent Readiness Bar

A board is a strong CircuitCI target when it has:

- imported schematic and layout evidence,
- high-confidence component packs for critical active parts,
- runnable scenario suggestions or hand-authored scenarios for core risks,
- explicit board/order metadata for manufacturing and mechanical policies,
- no critical findings,
- limitations that are understood and allowed,
- repair evidence for known-bad acceptance cases.
