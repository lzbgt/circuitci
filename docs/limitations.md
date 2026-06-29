# Limitations

CircuitCI is a validation runtime, not a full EDA suite.

The runtime backbone is Rust. Python is not part of the production engine path.

For a current feature matrix and the analog-simulation boundary, see
[docs/current_capabilities.md](current_capabilities.md).

## Not Implemented

- schematic editing
- PCB layout editing
- GHz RF or antenna solving
- DDR or high-speed signal integrity solving
- full USB PHY simulation
- full SMPS compensation design
- automatic datasheet-to-perfect-model generation
- broad firmware-in-loop MCU/peripheral machine coverage beyond the explicit
  QEMU pin-observation path

## Limitation Triage

Not every limitation below should become a feature. CircuitCI should remain a
validation and evidence runtime, not a replacement for schematic capture,
layout editing, RF/SI field solving, or vendor simulation tools. Those are
hard non-goals unless the project changes scope.

The limitations that do provide high leverage for this tool are the ones that
increase automated board-assessment coverage while preserving fail-closed
evidence semantics:

- More datasheet-backed component packs for parts seen on real boards.
- More importer evidence from KiCad, JLC/EasyEDA, Gerber, Excellon, and
  eventually other EDA exports.
- More scenario suggestions from imported evidence, especially when a check can
  become runnable without hand-authored YAML.
- Static manufacturability checks backed by fabrication outputs, such as
  copper/edge/drill/annular-ring/spacing screens.
- SPICE-backed waveform checks for board-boundary analog failures that static
  topology cannot prove, such as transistor storage, reset release, inrush,
  brownout, and power-mux switchover.
- Better model metadata for common packages and connector mechanical evidence.

The limitations that are useful to record but should not be chased blindly are
full-physics or tool-replacement claims: full USB PHY/eye solving, GHz antenna
or RF solving, DDR SI, full transistor-level MCU simulation, automatic
datasheet-to-perfect-model generation, and general 3D enclosure/plug fit
without explicit imported mechanical evidence.

## Current Technical Limits

- Full transistor-level MCU simulation is intentionally not a CircuitCI goal.
  MCU models should be functional black boxes at the board boundary: firmware
  execution, reset/boot behavior, peripheral state, pin modes, electrical pin
  limits, thresholds, clamps, leakage, and timing that matter to the surrounding
  circuit.
- `firmware_in_loop` supports QEMU functional execution when the scenario
  declares a machine, firmware image, and expected board-facing pin states, and
  when the QEMU run emits explicit `CIRCUITCI_PIN` observations. It does not
  infer pins from MCU internals. Renode remains fail-closed until a Renode
  adapter is integrated.
- `POWER_TREE_VALID` checks declared rail power state, nominal voltage ranges,
  static current budgets, and explicit regulator
  dropout/output-current/startup/capacitance metadata plus reset-supervisor
  threshold metadata. It does not infer a
  whole-board analog power tree or solve regulator ramp waveform shape,
  load-dependent dropout, inrush, charger/power-mux behavior, reset-output
  waveform shape, inductor saturation current/DCR/ripple, thermal behavior, or
  load-transient stability.
- `RESET_RELEASE_AFTER_POWER_VALID` can consume target rail `power_valid_at_us`
  and reset-supervisor delay metadata. It does not derive reset release from an
  analog RC/supervisor waveform unless an explicit `analog_transient` scenario
  is provided.
- `GPIO_BACKDRIVE` uses a simple diode/source-resistance approximation.
- `INTERFACE_PROTECTION_REVIEW` checks declared signal-conditioning channel
  metadata, unpowered-isolation claims, observed disabled-state evidence,
  declared static supply-order constraints, and clamp-only protection metadata
  such as reference net kind, standoff voltage, and line capacitance. It does
  not prove analog leakage, dynamic clamp current, ESD pulse performance,
  propagation delay, edge rate, USB eye margin, or signal integrity.
- `USB_CONNECTOR_PROTECTION_VALID` checks that declared USB connector D+/D- and,
  when requested, VBUS nets have connected clamp-only protection with compatible
  reference wiring and optional standoff-voltage evidence. When requested, it
  can also require the connector shield pin to connect to a declared ground net.
  It does not prove ESD pulse robustness, connector placement, RC/ferrite/chassis
  shield-bonding strategy, differential routing, return-path quality, USB eye
  margin, or layout-level protection effectiveness.
- `USB_PROTECTION_PLACEMENT_VALID` checks explicit component placement
  coordinates and center-to-center connector-to-protection distance for USB
  clamp coverage. It does not prove trace order, trace length, via count,
  parasitic inductance, shield strategy, return-path continuity, differential
  impedance, ESD pulse survival, or USB signal integrity.
- `BUS_PROTECTION_PLACEMENT_VALID` checks explicit CAN/RS485-style bus line
  route evidence and component placements against declared route-distance and
  off-route limits. It requires ordered continuous route polylines and does not
  prove surge-current sharing, stub inductance, cable EMC, common-mode behavior,
  or differential signal integrity.
- `USB_CONNECTOR_ORIENTATION_VALID` checks imported connector placement
  `rotation_deg` against an explicit expected rotation and tolerance. It does
  not prove enclosure entry direction, connector keepout, cable clearance, or
  mechanical insertion robustness. Suggestions may infer an expected rotation
  from imported `Edge.Cuts` outline segment evidence. KiCad outline arcs and
  circles are sampled into segments with source provenance. Closed contour
  classification can identify enclosed cutouts for nearest-edge filtering, but
  exact curve geometry, slots, and footprint-specific connector-entry
  conventions still require review.
- `USB_CONNECTOR_EDGE_PROXIMITY_VALID` checks the nearest imported board-edge
  segment against supported connector `fabrication`/`courtyard` footprint
  drawing evidence when available, falling back to connector-center distance
  otherwise. It ignores imported board-outline segments marked as interior
  `cutout` contours. It does not prove connector body overhang, panel
  alignment, shell clearance, cable insertion clearance, slots, complex cutout
  geometry, or full enclosure fit. Imported `fp_rect` evidence is treated as a
  rectangular extent from its transformed endpoints, not a full mechanical body
  model; imported `fp_poly` evidence is a 2D drawing outline, and imported
  `fp_circle`/`fp_arc` evidence is sampled into bounded 2D polylines for static
  measurements, not a 3D connector envelope.
- `USB_CONNECTOR_BODY_OVERHANG_VALID` measures supported 2D connector
  `fabrication`/`courtyard` footprint drawing protrusion past the nearest
  board-edge segment. It does not model 3D connector shell volume,
  panel cutouts, board slots, enclosure interference, cable insertion clearance,
  or assembly tolerances.
- `USB_CONNECTOR_COMPONENT_CLEARANCE_VALID` measures supported 2D connector
  `fabrication`/`courtyard` footprint evidence against other component
  footprint evidence, falling back to other component placement centers only
  when no usable footprint graphics are present. It is a static keepout screen;
  it does not prove 3D connector shell, cable insertion, panel, enclosure, or
  assembly stack-up clearance.
- `USB_CONNECTOR_ENTRY_CLEARANCE_VALID` checks a static 2D cable-entry corridor
  projected forward from the connector footprint body using imported placement
  rotation plus optional KiCad footprint-property, KiCad mapping, or
  component-model
  `entry_direction_offset_deg`, or explicit scenario `entry_direction_deg`.
  Optional KiCad footprint-property, KiCad mapping, or component-model
  entry-clearance depth and width can prefill suggestion parameters,
  but that depth is still only 2D corridor evidence.
  Optional imported footprint-property, KiCad mapping, or component-model
  aperture offsets and width can move and widen that 2D corridor, but they
  still do not model plug geometry, connector shell volume,
  cable bend radius, panel cutouts, enclosure interference, or assembly
  stack-up.
- `USB_ROUTE_GEOMETRY_VALID` and `USB_VBUS_ROUTE_VALID` check imported static
  route geometry for USB data nets and VBUS respectively. VBUS route checks are
  limited to route length, via count, optional minimum segment width, and
  connector-to-protection route distance. They do not prove VBUS current
  capacity, fuse trip behavior, inrush current, voltage drop under load,
  temperature rise, or ESD pulse survival.
- `USB_ROUTE_GEOMETRY_VALID` can use imported pad evidence to make USB data-line
  connector-to-protection route checks pad-aware. For supported KiCad pad
  shapes (`rect`, `circle`, and `oval`) it screens route contact against the
  imported pad copper extent; incomplete or unsupported pad geometry falls back
  to pad-center projection. It still does not prove solder-joint geometry,
  exact pad-edge trace entry, or high-frequency discontinuity.
- `USB_RETURN_PATH_VALID` checks whether USB D+/D- route segment midpoints are
  inside same-layer ground-zone outlines, and can optionally check that USB
  data vias have nearby ground stitching vias spanning the same layer
  transition. It can use imported `filled_polygons` when
  `require_filled_zone_coverage` is true and can screen midpoint distance to
  filled-copper polygon edges when
  `min_data_line_filled_zone_edge_clearance_mm` is declared. When
  `require_ground_zone_contact_evidence` is true, the same-layer ground zone
  must also contain imported same-net pad or via contact evidence; in
  filled-zone mode, pad copper or via contact evidence must overlap the same
  saved filled polygon as the route midpoint. Supported KiCad pad shapes use
  imported pad copper extent; incomplete or unsupported pad geometry falls back
  to pad-center containment. This still does not prove unmodeled filled-zone
  island connectivity, adjacent-plane coupling, impedance, eye margin,
  stitching-via inductance, common-mode radiation, or return-current behavior
  under signal transitions.
- `CLOCK_SOURCE_VALID` checks declared external crystal support-network
  connectivity and load capacitance. It does not prove oscillator startup,
  negative resistance, ESR margin, drive level, ppm accuracy, temperature
  drift, or layout parasitics.
- Quantitative waveform proof is available only through `analog_transient`
  scenarios with a SPICE-class backend and explicit assertions.
- Imported SPICE decks can produce solver and waveform evidence, but an
  assertion-free imported deck reports `ANALOG_ASSERTIONS_ABSENT`; waveform
  evidence alone is not design sign-off.
- KiCad XML, native `.kicad_sch`, and `.kicad_pcb` layout-evidence import are
  conservative. Unsupported or ambiguous constructs fail closed instead of being
  guessed. PCB import currently extracts component center placements,
  connected pad center/kind/shape/size/rotation/net/layer evidence,
  `Edge.Cuts` outline segment evidence, segment/via route geometry, copper-zone
  outlines/fill polygons, and a bounded subset of
  net-class/custom-rule route constraints for mapped nets, not arbitrary DRC
  rule semantics, filled-copper connectivity, thermal relief behavior, solder
  mask expansion, return paths, or signal-integrity constraints.
- `DRILL_DIAMETER_VALID` checks imported circular Excellon drill diameters
  against explicit or preset process ranges and can use the dedicated JLCPCB
  0.15-6.30 mm circular drill preset. `DRILL_TO_BOARD_EDGE_CLEARANCE_VALID`
  uses imported drill centers, drill diameters, and board-outline centerline
  segments for a static 2D edge-clearance screen and can use
  `board.manufacturing.min_drill_edge_clearance_mm` when the scenario omits the
  parameter. `SLOT_TO_BOARD_EDGE_CLEARANCE_VALID` similarly uses imported
  Excellon `G85` routed-slot centerlines and widths under `board.layout.slots[]`
  and can use `board.manufacturing.min_slot_edge_clearance_mm`. These edge
  metadata fields are order-specific board facts, not generic JLCPCB defaults.
  `SLOT_WIDTH_VALID` checks routed-slot width
  against process thresholds and can use the dedicated JLCPCB
  metallized/non-metallized slot preset. `SLOT_ASPECT_RATIO_VALID` checks
  routed-slot length-to-width ratio and can use the same JLCPCB slot preset.
  `CASTELLATED_HOLE_VALID` is a
  separate opt-in rule for drill evidence explicitly marked `castellated: true`;
  it can use the dedicated JLCPCB castellated-hole diameter,
  hole-to-board-edge, and hole-to-hole spacing preset without changing the
  generic drill-edge rule.
  These rules do not model drill
  wander, route-tool runout/overcut, plating tolerances, panel tabs,
  fab-specific stackup rules, copper-to-hole clearance, minimum slot length, or
  3D mechanical fit.
- `DRILL_ANNULAR_RING_VALID` uses imported drill centers, drill diameters, and
  Gerber flash copper geometry for a static 2D annular-ring screen. It checks
  circle, rectangle, and axis-aligned oval flashes, skips non-plated drills,
  rejects co-located copper as annular-ring evidence when drill and flash carry
  conflicting net or pad/via owner evidence, and can require explicit copper
  flash evidence on scenario-listed layers such as `F.Cu` and `B.Cu`. Drill
  hits may carry pad or via ownership when an importer can correlate them with
  existing layout evidence. Source-backed process presets can provide selected
  defaults, currently a dedicated JLCPCB double-sided/multilayer via minimum
  annular ring. It does not use copper draw traces, thermal reliefs,
  plated-barrel thickness, drill wander distributions, solder mask, fab
  compensation, component-hole annular requirements, or solve full electrical
  continuity.
- `COPPER_TO_BOARD_EDGE_CLEARANCE_VALID` uses imported Gerber copper
  flashes, trace segments, and region polygons with board-outline centerline
  segments for a static 2D copper-edge screen. It does not model solder mask,
  etch compensation, panelization tabs, fab-specific clearance expansion,
  or copper island connectivity. Imported copper may carry `net`, `island_id`,
  or flash-level pad/via owner fields when correlated with separate layout
  evidence, but plain Gerber copper remains anonymous.
- `COPPER_SPACING_VALID` uses imported Gerber copper flashes, trace
  segments, and region polygons for a static same-layer 2D copper-spacing
  screen. If copper objects carry explicit `net` or `island_id` ownership
  evidence, the rule can skip same-owner copper and report touching
  conflicting-owner copper. It still ignores different-layer copper and
  overlapping/touching anonymous copper because plain Gerber evidence has no
  net or island ownership. It does not prove solder-mask behavior, etch
  compensation, fab-specific spacing expansion, or copper connectivity.
- `SOLDER_MASK_OPENING_VALID` uses imported Gerber copper flashes and Gerber
  solder-mask flash, circular-aperture draw, and simple region openings for a
  static 2D mask-aperture screen. It checks same-side `F.Cu` -> `F.Mask`
  and `B.Cu` -> `B.Mask` co-located openings and minimum mask expansion.
  Source-backed process presets can provide selected defaults, currently
  `jlcpcb_standard_2026_06` for minimum mask expansion. It does not yet
  evaluate nested or overlapping mask-region holes, full fab-specific mask swell,
  package-specific solder-mask-defined pad rules, or 3D solderability effects.
- `CONTROLLED_IMPEDANCE_GEOMETRY_VALID` compares imported route width and
  same-layer parallel differential-pair gap evidence against explicit reviewed
  target geometry declared by the scenario. It does not calculate
  characteristic or differential impedance, model dielectric stackup,
  solder-mask loading, copper thickness, etch compensation, or fabricator
  coupon results. Passing this rule only means the imported geometry matches
  the declared target dimensions within tolerance.
- `CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID` verifies that explicit
  route/reference/dielectric layer names have reviewed material,
  dielectric-constant, dielectric-thickness, copper-thickness, source, and
  layer-order evidence. It does not solve impedance, account for solder mask,
  model roughness or plating tolerance, or replace field-solver/coupon signoff.
- `CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID` compares sampled imported
  route points against imported dark solder-mask opening artwork and reviewed
  `covered`/`opened` loading targets. It does not calculate the impedance
  effect of solder mask material, model registration tolerance, infer mask
  intent from copper names, or replace field-solver/coupon signoff.
- `ADJACENT_PLANE_RETURN_PATH_VALID` uses explicit stackup layer order,
  declared plane `reference_net`, route segments, and sampled zone polygons to
  screen for adjacent-plane coverage. It does not infer reference planes from
  layer names, model return-current distribution, account for via transitions,
  copper roughness, dielectric fields, or EMI/SI behavior.
  Passing this rule only means sampled imported route points stay over declared
  adjacent reference-plane zone evidence within the reviewed length limit.
- `REFERENCE_PLANE_SLOT_CROSSING_VALID` computes route-centerline coverage
  intervals from explicit adjacent reference-plane zone polygons and counts
  internal gaps between covered intervals. It does not model return-current
  density, fringing fields, stitching-via effectiveness, plane impedance, or
  EMI/SI risk. Passing this rule only means the imported
  route centerline did not cross more declared reference-plane zone gaps than
  the scenario allowed.
- `RETURN_PATH_STITCHING_VIA_VALID` compares explicit signal route vias against
  explicit reference-net route vias with matching layer spans and a reviewed
  maximum distance. It does not prove via inductance, plane cavity behavior,
  stitching density, connector launch quality, or solver-backed return-current
  continuity. Passing this rule only means each imported signal transition via
  has a nearby declared reference-net via within the reviewed distance limit.
- `RF_ANTENNA_KEEPOUT_VALID` compares reviewed same-layer RF antenna keepout
  polygons against imported copper features, segments, and regions while
  optionally excluding a declared antenna net. It does not prove antenna
  matching, tuning, radiation pattern, efficiency, enclosure/cable effects,
  solder-mask loading, 3D structures, or RF field behavior. Passing this rule
  only means comparable imported copper is outside the reviewed 2D keepout
  clearance.
- `RF_ANTENNA_FEED_PATH_VALID` compares reviewed antenna feed-path metadata
  against explicit antenna-net route length, feed component/pin pad evidence,
  matching component placement, and matching component antenna-net pad evidence.
  It does not infer a pi/L/T matching topology, verify component values, solve
  feed impedance, model launch discontinuities, prove radiation performance, or
  replace RF simulation and VNA/S-parameter measurement. Passing this rule only
  means the imported layout evidence satisfies the reviewed static feed-path
  length and proximity limits.
- `RF_ANTENNA_MATCHING_TOPOLOGY_VALID` compares reviewed RF matching-network
  topology metadata against explicit component pin and layout pad evidence for
  declared series/shunt elements. It does not verify L/C values, solve
  impedance, model parasitics, infer topology from designators or net names,
  prove return loss, or replace RF simulation and S-parameter measurement.
  Passing this rule only means the source-backed topology declaration is
  consistent with the explicit reviewed role and pin/pad evidence.
- `RF_ANTENNA_MEASURED_PERFORMANCE_VALID` compares reviewed RF measurement rows
  against explicit return-loss, optional frequency-band, and optional sampled
  sweep-coverage and measurement-condition limits. It does not interpolate
  S-parameter sweeps between sampled points, solve impedance, infer antenna
  tuning quality from layout or net names, model enclosure/cable/fixture
  effects, or replace RF simulation, chamber testing, or final antenna tuning.
  Passing this rule only means the selected source-backed measurement points
  satisfy the reviewed numeric, sampling, and condition-reference limits.
- `THERMAL_COPPER_AREA_VALID` compares reviewed component thermal-copper
  minimum-area metadata against explicit imported copper area tied to the
  reviewed component or reviewed nets/layers. It does not model copper
  thickness, thermal vias, spreading resistance, convection, enclosure,
  component package thermal resistance, derating, or measured temperature rise.
  Passing this rule only means the imported 2D copper-area evidence satisfies
  the reviewed static minimum.
- `THERMAL_VIA_STACKUP_VALID` compares reviewed thermal via-count and
  copper-thickness metadata against explicit imported route vias and stackup
  layer copper thickness. It does not model via barrel resistance, spreading
  resistance, convection, enclosure, package thermal resistance, derating, or
  measured temperature rise. Passing this rule only means the imported via layer
  spans and stackup copper-thickness evidence meet the reviewed static
  minimums.
- `THERMAL_VIA_PLATING_VALID` compares reviewed plated thermal-via count and
  drill-diameter metadata, plus optional reviewed via plating-thickness
  metadata, against explicit imported route-via and drill plating evidence. It
  does not model via barrel resistance, spreading resistance, convection,
  enclosure, package thermal resistance, derating, or measured temperature rise.
  Passing this rule only means the imported drill plating, drill-diameter, and
  optional plating-thickness evidence meet the reviewed static minimums.
- `THERMAL_VIA_BARREL_CROSS_SECTION_VALID` sums explicit annular barrel copper
  cross-section from imported plated drill diameter and plating-thickness
  evidence for reviewed thermal route vias. It does not model via barrel
  thermal resistance, spreading resistance, convection, enclosure, package
  thermal resistance, derating, or measured temperature rise. Passing this rule
  only means the imported static barrel geometry evidence meets the reviewed
  minimum.
- `THERMAL_DERATING_ENVIRONMENT_VALID` compares reviewed thermal-copper
  environment assumptions against explicit scenario ambient, airflow, and
  enclosure-profile inputs. It does not model airflow distribution, convection,
  enclosure thermal impedance, fan curves, heatsinks, component derating curves,
  or measured temperature behavior. Passing this rule only means the scenario
  environment inputs do not contradict the reviewed static derating metadata.
- `THERMAL_PACKAGE_TEMPERATURE_VALID` compares reviewed static power loss and
  source-backed component package Rja metadata against reviewed ambient and
  temperature-rise limits. It does not model transient thermal impedance, board
  spreading resistance, copper/via effectiveness, airflow, enclosure effects,
  heatsinking, package mounting variation, derating curves, or measured
  temperature behavior. Passing this rule only means the reviewed static
  package estimate is within the declared limits.
- `THERMAL_MEASURED_TEMPERATURE_VALID` compares reviewed thermal measurement
  rows against reviewed temperature limits. When explicitly requested, it adds
  reviewed `measurement_uncertainty_C` to the measured absolute temperature and
  optional rise-over-ambient value before comparison. It does not infer
  measurement uncertainty, probe emissivity, sensor placement, transient
  warm-up, environmental repeatability, airflow/enclosure effects, or derating
  curves. Passing this rule only means the explicit measurement rows are within
  the declared static limits.
- `SOLDER_MASK_DAM_VALID` uses imported Gerber solder-mask flash, sampled draw,
  and region openings for a static same-layer 2D mask-web screen. It can detect
  thin or missing dams between supported circle, rectangle, axis-aligned oval,
  observed EasyEDA `RoundRect`, circular-aperture linear/arc draw, and simple
  region openings. Source-backed process presets can provide
  selected defaults, currently `jlcpcb_standard_2026_06` for minimum
  solder-mask dam width. It does not yet evaluate nested or overlapping
  solder-mask region holes, package-specific no-dam exceptions, manufacturer-specific
  bridge rules, paste stencil behavior, or 3D solderability effects.
- `SOLDER_PASTE_OPENING_VALID` uses imported Gerber copper flashes and Gerber
  solder-paste flash, circular-aperture draw, and simple region openings for a
  static 2D stencil-aperture screen. It checks same-side `F.Cu`
  -> `F.Paste` and `B.Cu` -> `B.Paste` co-located openings and min/max
  paste-to-copper area ratio, aggregating multiple co-located paste openings
  for windowed stencil patterns. It skips copper features explicitly owned by
  vias. The min/max ratio may come from scenario parameters or explicit
  `board.manufacturing` metadata; CircuitCI does not invent global defaults for
  package-specific paste coverage. It does not yet evaluate nested or
  overlapping paste-region holes, step-stencil thickness, paste volume, package-specific paste
  reductions, or 3D solderability effects.
- `SOLDER_PASTE_APERTURE_SIZE_VALID` uses imported Gerber solder-paste flash
  and circular-aperture draw evidence for a static stencil minimum aperture-size
  screen. Source-backed process presets can provide selected defaults,
  currently `jlcpcb_stencil_aperture_min_2026_06` for JLCPCB's greater-than
  0.08 mm minimum aperture size. It does not evaluate arbitrary region minimum
  width, stencil thickness, paste release, or package-specific paste reductions.
- `SOLDER_PASTE_APERTURE_AREA_RATIO_VALID` uses imported Gerber solder-paste
  flash, circular-aperture draw, and simple region evidence for a
  static stencil release area-ratio screen. Source-backed process presets can
  provide the selected minimum, currently `jlcpcb_stencil_area_ratio_2026_06`
  for the JLCPCB/IPC-7525 `0.66` aperture area-ratio floor. The scenario or
  board manufacturing metadata must still provide `stencil_thickness_mm`;
  CircuitCI does not infer stencil thickness from Gerbers or reuse this metric
  as paste-to-copper coverage.
- `SOLDER_PASTE_IC_PIN_APERTURE_VALID` uses pad-owned Gerber solder-paste
  feature, draw, and region evidence for an opt-in IC pin aperture screen
  against the saved JLCPCB stencil opening table. It only runs for an explicit
  `pin_pitch_mm` whose pitch has source-backed JLCPCB guidance, checks the exact
  1.00 mm length requirement for the 0.635-0.65 mm IC pitch row, applies the
  0.5 mm row length-extension text only when unique owner-matched copper pad
  geometry proves the pad-length condition, and honors `target.component` when
  present. `suggest-scenarios` can infer selected
  target-scoped discrete pitch rows from repeated pad-owned paste flashes, but
  CircuitCI does not infer arbitrary package pitch or package class
  automatically. This is not a generic paste area-ratio, paste spacing,
  paste-volume, or stencil-thickness rule.
- `SOLDER_PASTE_BGA_APERTURE_VALID` uses pad-owned Gerber solder-paste flash
  evidence for an opt-in BGA aperture-size screen against the saved JLCPCB BGA
  stencil opening table. It only runs for an explicit source-backed
  `pin_pitch_mm`, honors `target.component` when present, and `suggest-scenarios`
  only infers it from repeated two-axis grid evidence. The validator also
  requires same-pitch horizontal and vertical grid evidence for the declared
  pitch, so a hand-authored BGA scenario with the wrong pitch fails closed. It
  does not infer arbitrary package class, paste volume, or BGA stencil rules
  beyond the encoded JLC pitch rows.
- `SOLDER_PASTE_SPACING_VALID` uses imported Gerber solder-paste flash,
  circular-aperture linear/arc draw, and simple region openings for a
  static same-layer 2D stencil-web screen. It can detect merged or too-close
  paste openings between supported paste objects. The spacing limit may come
  from scenario parameters or explicit `board.manufacturing` metadata; CircuitCI
  does not invent a global paste-spacing preset for package-specific stencil
  behavior. It does not evaluate stencil thickness, paste release, paste volume,
  nested or overlapping paste-region holes, or package-specific intentional
  aperture merging.
- Gerber copper import currently records dark `D03` flash features for circle,
  rectangle, oval, and observed EasyEDA `RoundRect` apertures, dark linear
  `D01` traces and sampled `G02`/`G03` arc traces for circular apertures, and
  dark simple `G36`/`G37` region polygons. When the input Board IR
  already has exactly one matching pad, route, or zone owner, it can annotate
  imported copper with `net`. It ignores non-circular aperture draws, skips
  clear-polarity copper primitives, and does not infer component ownership, pad
  names, copper island connectivity, mask expansion, or electrical
  connectivity.
- Component models are low-confidence generic behavioral models unless a vendor
  or datasheet-backed pack says otherwise.
- Reports include `LOW_CONFIDENCE_MODEL` limitations for `generic`, `estimated`, or `low` confidence models used by a project.
- `MODEL_QUALITY_REQUIRED` can make selected low-confidence or wrong-source
  models critical for fabrication sign-off, but it only validates declared
  model provenance and confidence. It does not prove that the datasheet model
  contains every physical rating needed by the board.
- `LOAD_CONNECTOR_CURRENT_VALID` compares static load current and optional
  nominal rail voltage against declared connector ratings. It does not prove
  crimp quality, wire gauge, contact temperature rise, vibration retention,
  pulsed current, hot-plug behavior, or regeneration into the load rail.
- `POWER_SWITCH_BUDGET_VALID` compares a switched load's static current against
  selected switch current rating/current-limit metadata and estimates
  conduction junction temperature from one on-resistance and thermal-resistance
  point. It does not model inrush, turn-on ramp, current-limit transient
  waveform, reverse current, short-circuit SOA, repeated surge, or PCB copper
  temperature.
- `POWER_SWITCH_REVERSE_CURRENT_VALID` only checks a declared
  reverse-current-blocking mode (`always`, `when_disabled`, or `none`). It does
  not model body-diode timing, back-to-back FET gate behavior, upstream clamp
  energy, or measured backfeed waveforms.
- `POWER_SWITCH_INRUSH_VALID` estimates capacitive turn-on current from a
  declared capacitance and soft-start time. It does not model nonlinear
  capacitance, load startup behavior, upstream source droop, eFuse retry
  behavior, or repeated thermal pulses.
- `LOAD_CABLE_CURRENT_VALID` compares static load current and optional nominal
  rail voltage against declared cable or harness assembly ratings. It does not
  prove crimp process quality, bundle derating, flex life, enclosure airflow,
  contact temperature rise, or pulsed-current heating.
- `LOAD_CABLE_THERMAL_DERATING_VALID` estimates cable temperature rise by
  I^2 scaling from one declared temperature-rise test point. It does not model
  bundle derating, airflow, enclosure contact, intermittent duty cycle, flex
  aging, crimp heating, connector heating, or measured routed-harness
  temperature.
- `LOAD_CABLE_VOLTAGE_DROP_VALID` estimates voltage drop and harness power
  loss from one declared loop resistance. It does not model connector contact
  resistance drift, temperature-dependent copper resistance, PWM ripple,
  intermittent load duty cycle, return-path sharing, or measured in-system
  voltage transients.
- `MOTOR_BRIDGE_LOSS_THERMAL_VALID` scales a source-backed bridge reference
  loss point against declared motor RMS current and explicit board thermal
  budget. It does not compute MOSFET SOA, switching transition loss,
  gate-charge timing, transient thermal impedance, regeneration energy, or
  measured PCB temperature.
- `MOTOR_BRIDGE_SWITCHING_VALID` estimates transition loss from source-backed
  rise/fall timing and checks average gate-charge current from source-backed
  total gate charge. It does not prove MOSFET SOA, peak gate source/sink
  current, Miller plateau behavior, switch-node ringing, dead-time, diode
  reverse recovery, transient thermal impedance, or measured waveforms.
- `MOTOR_BRIDGE_SOA_VALID` compares one explicit static bridge stress point to
  source-backed datasheet SOA curves. For power-block system SOA it checks
  current versus board/case temperature; for discrete-style curves it checks
  VDS/ID pulse limits. It fails closed when SOA metadata is missing, but it
  does not infer motor inertia, actual switch-node waveforms, avalanche energy,
  repeated-pulse thermal impedance, current sharing, heatsinking, or measured
  board temperature.
- `MOTOR_REGEN_CLAMP_VALID` checks explicit single-event regeneration current,
  energy, bus capacitance, voltage window, and clamp/absorber current-energy
  limits. It does not infer motor inertia or speed, prove repeated-pulse
  thermal behavior, brake-resistor temperature rise, active clamp stability,
  firmware regeneration control, MOSFET SOA, or real battery acceptance of
  regenerated energy.
- `MOTOR_ROUTE_CURRENT_VALID` compares imported route segment widths against an
  explicit A/mm policy. It does not compute copper temperature rise, current
  sharing across pours, thermal-via effectiveness, MOSFET SOA, switching loss,
  or regeneration transient energy.
- `MOTOR_CURRENT_SENSE_ACCURACY_VALID` sums declared shunt tolerance, gain
  error, input offset, and ADC quantization into a static worst-case current
  error budget, and checks peak ADC range plus minimum-current ADC counts. It
  does not prove PWM sample timing, common-mode rejection, amplifier bandwidth,
  ADC aperture behavior, calibration firmware, or thermal drift.
- `MOTOR_CURRENT_SENSE_PLACEMENT_VALID` compares phase-shunt placement and
  current-sense route distances against explicit layout policy. It does not
  compute shunt parasitics, amplifier gain/offset/noise, ADC resolution, PWM
  common-mode rejection, or current-sense thermal drift.
- `RESIDENT_BOOTLOADER_UPDATE_SEQUENCE` validates declared transaction traces and does not execute firmware, decode raw serial frames, recompute CRCs, emulate flash, or prove HIL behavior.
- `CONTROL_LINE_RELEASE_SEQUENCE` validates declared line effects and release delays and does not solve transistor storage, hidden RC networks, or physical modem-pin voltage truth tables.
- `analog_transient` scenarios are the only path intended for quantitative
  voltage/current waveform proof. If no SPICE-class backend is available, or if
  the solver cannot produce parseable waveform data, these scenarios fail with
  critical analog findings rather than producing fake passes.

Reports must include these limitations so automated agents and human users know when a pass does not imply full physical coverage.

For the broader gap list between the current tool and "verify any common IoT
board" readiness, see [common_iot_board_readiness_gaps.md](common_iot_board_readiness_gaps.md).
