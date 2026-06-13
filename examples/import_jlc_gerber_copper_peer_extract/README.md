# JLC/EasyEDA Gerber Copper Peer-Shape Extract

This fixture mirrors the coordinate format and board coordinate frame used by
the `urine_monitor` JLC/EasyEDA fabricated release extracts. It is intentionally
small because the peer release currently checked into this repository contains
outline and drill extracts, not full raw copper layers.

The fixture exercises the supported `import-gerber-copper` subset:

- millimeter absolute RS-274X coordinates,
- `G04 Layer: F.Cu` layer metadata,
- circle, rectangle, and oval aperture definitions,
- dark `D03` copper flashes,
- one linear draw record that is counted but not imported yet,
- one clear-polarity flash that is skipped as non-conductive clearance.

The imported evidence is anonymous fabrication copper. It does not assign nets,
component ownership, pad names, or electrical connectivity.
