# KiCad Hierarchy Net Aliases

Native KiCad hierarchy import now treats root sheet pins as aliases of the
root net they touch instead of blindly injecting every sheet-pin name as a root
label.

The importer builds root connectivity from wires, junctions, symbol pins,
explicit labels, and sheet-pin coordinates. For each root net that touches one
or more sheet pins:

- if the root net has one explicit label, that label is the canonical net name
  and every touched sheet pin aliases to it;
- if the root net has no explicit label and touches one unique sheet-pin name,
  that sheet-pin name is the canonical net name;
- if the root net has no explicit label and touches multiple distinct
  non-ground sheet-pin names, import fails closed because the flattened Board IR
  would otherwise need to guess the canonical interface name;
- if disconnected root nets resolve to the same non-ground canonical name,
  import fails closed rather than silently shorting unrelated child interfaces;
- if the root net has conflicting explicit labels, import fails closed through
  the existing label-conflict rule.

This allows common root-mediated wiring such as child sheet pin `FILTER_OUT`
and child sheet pin `ADC_IN` both touching a root label `SENSE_NODE`. Both child
nets flatten to `SENSE_NODE` and can then participate in generated SPICE
scenarios without manual YAML net rewrites.

Aliases are keyed by sheet instance and pin name. This matters for duplicated
pin names such as `GND`: one sheet's `GND` pin may alias to a labelled test node
while another sheet's `GND` pin aliases to actual ground, and the importer must
not let one overwrite the other. Ground aliases remain special only for
duplicate-name and canonical-collision checks because Board IR already
normalizes recognized ground names to `gnd`.
