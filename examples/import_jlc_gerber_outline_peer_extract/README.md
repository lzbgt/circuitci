# JLC/EasyEDA Gerber Outline Peer Extract

This fixture mirrors the `Gerber_BoardOutlineLayer.GKO` structure observed in
the `urine_monitor` JLC/EasyEDA fabricated release. It contains one external
rectangular board outline and three enclosed rectangular slots/cutouts. The
fixture is intentionally limited to the outline layer so the importer test can
prove fabricated Gerber outline evidence becomes `board.layout.outline`
segments without depending on the full peer-board archive.
