# Gerber Copper Ownership Fixture

This fixture proves `import-gerber-copper` can annotate anonymous Gerber copper
with net ownership when the input Board IR already contains PCB evidence:

- the flash at `(1.0, 1.0)` overlaps pad `J1.1` on net `GND`,
- the circular-aperture draw from `(2.0, 1.0)` to `(4.0, 1.0)` overlaps route
  evidence for net `USB_DP`,
- the region centered around `(6.5, 1.0)` is contained by the `VBAT` zone.

The importer does not invent island IDs. It writes `net` only when exactly one
matching owner is found.
