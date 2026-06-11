# KiCad Package Pin Aliases

## Purpose

KiCad imports see package or symbol pins such as `"1"`, `"2"`, `"7"`, or
`"PA9"`. CircuitCI component models use semantic port names such as `A`, `B`,
`NRST`, or `BOOT0`. Direct `pin_map` entries already provide this mapping, but
large designs often need the same package pin mapping in multiple component
rules.

## Mapping Contract

Mapping files may declare named package pin aliases:

```yaml
pin_aliases:
  two_terminal_ab:
    "1": A
    "2": B
libsource_rules:
  - lib: Device
    part: R
    model: generic.analog.resistor
    pin_alias: two_terminal_ab
```

Rules:

- `components.<ref>.pin_alias` and `libsource_rules[].pin_alias` select one
  named alias table.
- A mapping entry may declare either `pin_alias` or direct `pin_map`, not both.
- Alias names must exist and alias tables must be non-empty.
- Alias keys and values follow the same validation as direct `pin_map`.
- After alias resolution, every existing fail-closed check still applies:
  connected pins must be mapped for real models, target model ports must exist,
  and multiple package pins cannot map to one model pin.

This is a mapping convenience only. It does not infer package behavior,
electrical equivalence, or schematic connectivity.
