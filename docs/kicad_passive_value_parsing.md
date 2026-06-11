# KiCad Passive Value Parsing

CircuitCI can derive resistor and capacitor SPICE primitive values from KiCad
schematic values only when a mapping file explicitly requests it. This keeps
the importer deterministic while reducing duplicate mapping boilerplate for
ordinary passive parts.

## Contract

Mapping-file `spice` metadata may use:

```yaml
spice:
  primitive: resistor
  value_ohm_from: schematic_value
```

or:

```yaml
spice:
  primitive: capacitor
  value_f_from: schematic_value
```

The importer resolves those fields before writing Board IR. Generated Board IR
still contains numeric `value_ohm` or `value_f`; it never stores the source
selector.

This is intentionally opt-in:

- `value_ohm_from` is valid only for `primitive: resistor`.
- `value_f_from` is valid only for `primitive: capacitor`.
- A mapping cannot provide both an explicit numeric value and a source selector
  for the same quantity.
- The selected component must have a non-empty KiCad `Value`.
- The parsed result must be finite and greater than zero.
- Ambiguous values fail the import instead of being guessed.

## Accepted Forms

Resistance values accept strict ohm notation:

- `10k`, `4.7k`, `1M`, `2.2M`, `100R`
- embedded decimal designator notation such as `4k7`, `1R0`, `0R05`
- plain numeric values, interpreted as ohms

Capacitance values accept strict farad notation:

- `100n`, `100nF`, `4.7u`, `4u7`, `10p`, `1mF`, `1F`

Plain numeric capacitance values are rejected. A KiCad value like `100` is too
ambiguous to treat as `100 F`; capacitance values must carry an explicit
capacitance suffix.

Suffix case is significant. For resistance, `M` means megaohms and `m` means
milliohms. For capacitance, `m` means millifarads.

The parser intentionally rejects values with tolerance, voltage rating, package,
or parallel/series annotations such as `10k 1%`, `100n/50V`, `DNP`, `NC`, and
comma-formatted numbers. Those annotations are useful schematic metadata, but
they are not a single unambiguous SPICE primitive value.

## Importer Scope

The feature is shared by `import-kicad-netlist` and
`import-kicad-schematic`, because both paths converge on the same mapped KiCad
Board IR builder.
