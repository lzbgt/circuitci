# KiCad Hierarchy SPICE Fixture

The hierarchy importer is now covered by an end-to-end generated-SPICE fixture
within the conservative native KiCad schematic subset that CircuitCI currently
accepts. The fixture keeps hierarchy strict but still proves that flattened
sheet connectivity can drive physical validation:

- root schematic: voltage source and one child sheet instance,
- child schematic: resistor/capacitor network,
- parent sheet pins: `VIN`, `OUT`, `GND`,
- child hierarchical labels: exactly `VIN`, `OUT`, `GND`,
- mapping file: explicit models, pin maps, passive value parsing, and one
  generated transient scenario.

The generated Board IR uses the same analog simulation path as flat KiCad
imports. The `OUT` net is produced by the child schematic through a
hierarchical label and is asserted in the transient scenario as `V(net_out)`.
The root schematic intentionally does not add separate local labels for `VIN`
or `GND`; those net names come from the parent sheet pins so the fixture proves
sheet-pin connectivity rather than same-name net merging.

This fixture intentionally avoids nested sheets and repeated sheet instances.
Those remain fail-closed until instance-specific reference and local-net
namespacing are implemented.
