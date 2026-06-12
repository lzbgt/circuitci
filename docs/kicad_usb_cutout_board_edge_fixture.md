# KiCad USB Cutout Board Edge Fixture

`examples/import_kicad_usb_cutout_board_edge_suggestions/` proves that USB
mechanical/layout checks prefer external board-edge contours over enclosed
Edge.Cuts cutouts.

The PCB fixture contains:

- an external rectangular `Edge.Cuts` contour,
- an enclosed circular `Edge.Cuts` cutout near the USB connector,
- a mapped USB connector with finite placement, pad geometry, and fabrication
  body polygon evidence.

The regression imports the PCB, confirms Board IR classifies the rectangle as
`boundary_role: external` and the enclosed circle as `boundary_role: cutout`,
then confirms USB orientation, edge-proximity, and body-overhang suggestions
select the external contour. It also runs an executable edge-proximity scenario
with a tight limit that fails against the external edge; this would incorrectly
pass if the nearby cutout were used as the connector entry edge.
