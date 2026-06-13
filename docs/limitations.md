# Limitations

CircuitCI is a validation runtime, not a full EDA suite.

The runtime backbone is Rust. Python is not part of the production engine path.

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
- `DRILL_TO_BOARD_EDGE_CLEARANCE_VALID` uses imported drill centers, drill
  diameters, and board-outline centerline segments for a static 2D
  edge-clearance screen. `SLOT_TO_BOARD_EDGE_CLEARANCE_VALID` similarly uses
  imported Excellon `G85` routed-slot centerlines and widths under
  `board.layout.slots[]`. `SLOT_WIDTH_VALID` checks routed-slot width against
  process thresholds and can use the dedicated JLCPCB metallized/non-metallized
  slot preset. These rules do not model drill wander, route-tool runout/
  overcut, plating tolerances, panel tabs, fab-specific stackup rules,
  copper-to-hole clearance, minimum slot length, or 3D mechanical fit.
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
  solder-mask flash, circular-aperture draw, and single-contour region openings
  for a static 2D mask-aperture screen. It checks same-side `F.Cu` -> `F.Mask`
  and `B.Cu` -> `B.Mask` co-located openings and minimum mask expansion.
  Source-backed process presets can provide selected defaults, currently
  `jlcpcb_standard_2026_06` for minimum mask expansion. It does not yet
  evaluate multi-contour mask regions, full fab-specific mask swell,
  package-specific solder-mask-defined pad rules, or 3D solderability effects.
- `SOLDER_MASK_DAM_VALID` uses imported Gerber solder-mask flash, sampled draw,
  and region openings for a static same-layer 2D mask-web screen. It can detect
  thin or missing dams between supported circle, rectangle, axis-aligned oval,
  observed EasyEDA `RoundRect`, circular-aperture linear/arc draw, and
  single-contour region openings. Source-backed process presets can provide
  selected defaults, currently `jlcpcb_standard_2026_06` for minimum
  solder-mask dam width. It does not yet evaluate multi-contour
  solder-mask regions, package-specific no-dam exceptions, manufacturer-specific
  bridge rules, paste stencil behavior, or 3D solderability effects.
- `SOLDER_PASTE_OPENING_VALID` uses imported Gerber copper flashes and Gerber
  solder-paste flash, circular-aperture draw, and single-contour region
  openings for a static 2D stencil-aperture screen. It checks same-side `F.Cu`
  -> `F.Paste` and `B.Cu` -> `B.Paste` co-located openings and min/max
  paste-to-copper area ratio, aggregating multiple co-located paste openings
  for windowed stencil patterns. It skips copper features explicitly owned by
  vias. It does not yet evaluate multi-contour paste regions, step-stencil
  thickness, paste volume, package-specific paste reductions, or 3D
  solderability effects.
- `SOLDER_PASTE_SPACING_VALID` uses imported Gerber solder-paste flash,
  circular-aperture linear/arc draw, and single-contour region openings for a
  static same-layer 2D stencil-web screen. It can detect merged or too-close
  paste openings between supported paste objects. It does not evaluate stencil
  thickness, paste release, paste volume, multi-contour paste regions, or
  package-specific intentional aperture merging.
- Gerber copper import currently records dark `D03` flash features for circle,
  rectangle, oval, and observed EasyEDA `RoundRect` apertures, dark linear
  `D01` traces and sampled `G02`/`G03` arc traces for circular apertures, and
  dark single-contour `G36`/`G37` region polygons. When the input Board IR
  already has exactly one matching pad, route, or zone owner, it can annotate
  imported copper with `net`. It ignores non-circular aperture draws, skips
  clear-polarity copper primitives, and does not infer component ownership, pad
  names, copper island connectivity, mask expansion, or electrical
  connectivity.
- Component models are low-confidence generic behavioral models unless a vendor
  or datasheet-backed pack says otherwise.
- Reports include `LOW_CONFIDENCE_MODEL` limitations for `generic`, `estimated`, or `low` confidence models used by a project.
- `RESIDENT_BOOTLOADER_UPDATE_SEQUENCE` validates declared transaction traces and does not execute firmware, decode raw serial frames, recompute CRCs, emulate flash, or prove HIL behavior.
- `CONTROL_LINE_RELEASE_SEQUENCE` validates declared line effects and release delays and does not solve transistor storage, hidden RC networks, or physical modem-pin voltage truth tables.
- `analog_transient` scenarios are the only path intended for quantitative
  voltage/current waveform proof. If no SPICE-class backend is available, or if
  the solver cannot produce parseable waveform data, these scenarios fail with
  critical analog findings rather than producing fake passes.

Reports must include these limitations so automated agents and human users know when a pass does not imply full physical coverage.

For the broader gap list between the current tool and "verify any common IoT
board" readiness, see [common_iot_board_readiness_gaps.md](common_iot_board_readiness_gaps.md).
