# KiCad USB Sampled Board Edge Fixture

`examples/import_kicad_usb_curved_board_edge_suggestions/` proves that sampled
KiCad `Edge.Cuts` rectangles and curves feed USB mechanical/layout checks, not
just raw import storage. The directory name is retained for continuity with the
first curved-edge fixture, but the fixture now covers multiple sampled outline
primitive types.

The fixture has three PCB variants:

- `board_rect.kicad_pcb` uses an `Edge.Cuts` `gr_rect`.
- `board_circle.kicad_pcb` uses an `Edge.Cuts` `gr_circle`.
- `board_arc.kicad_pcb` uses an `Edge.Cuts` `gr_arc`.

Both boards contain a mapped USB connector with finite placement, connected pad
geometry, and a fabrication footprint polygon near the curved edge.

The regression flow is:

1. Import each PCB into `project.yaml`.
2. Confirm the Board IR contains sampled outline segments from the board edge.
3. Run `suggest-scenarios` and assert the USB connector orientation,
   edge-proximity, and body-overhang suggestions select a sampled outline
   segment as `nearest_board_edge`.
4. Import each PCB into `project_checks.yaml` and run the generated executable
   checks to prove the same sampled segment evidence is usable by validators.
   The rectangle variant intentionally tightens the body-overhang limit in a
   temporary checks file so the validation report must expose
   `board_edge_source_primitive: gr_rect`.

This fixture intentionally checks exact sampled segment coordinates. If the
sampling policy changes, update the fixture expectations with the new bounded
sampling contract.
