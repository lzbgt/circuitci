# NL27WZ17 Logic Buffer Observation

This direct-open GUI example proves the source-backed onsemi NL27WZ17 dual
non-inverting Schmitt-trigger buffer can be used in generated SPICE
observations.

The fixture drives a 3.3 V `VCC`, sets `1A` high and `2A` low through explicit
Board IR component parameters, then checks that `1Y` and `2Y` mirror those
states. It is intended for board-level line-state evidence and GUI workflow
coverage, not Schmitt hysteresis, propagation delay, output-drive strength,
loading, signal-integrity, or timing sign-off.
