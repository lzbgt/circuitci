# KiCad Hierarchy Alias SPICE Fixture

`examples/import_kicad_hierarchy_alias_spice` proves that hierarchy root-net
aliases can drive generated SPICE scenarios.

The fixture has one root schematic and two child sheets:

- root `V1` drives child sheet pin `VIN`,
- child sheet `Filter` contains `R1` between `VIN` and `FILTER_OUT`,
- root net `SENSE_NODE` connects `FILTER_OUT` to another child sheet pin
  `ADC_IN`,
- child sheet `Load` contains `C1` between `ADC_IN` and `GND`.

`FILTER_OUT` and `ADC_IN` are distinct child interface names. The explicit root
label `SENSE_NODE` is the canonical flattened net name, so both child nodes
become Board IR net `net_sense_node`. The mapping file then generates a SPICE
transient scenario and asserts that `V(net_sense_node)` charges above `2.5 V`
after `2 ms`.

This fixture is intentionally small but not a toy path: it exercises native
schematic import, one-level hierarchy flattening, root-net aliasing, schematic
passive value parsing, generated Board IR SPICE deck emission, solver waveform
parsing, and quantitative assertion evaluation.
