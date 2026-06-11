# KiCad Group Bus Syntax Research

Source persisted locally:

- `docs/research/kicad/kicad_9_eeschema.html`
- Original URL: <https://docs.kicad.org/9.0/en/eeschema/eeschema.html>

KiCad documents group bus labels as members listed inside curly braces,
separated by spaces. A group name may appear before the opening brace. When a
group name is present, KiCad prefixes each emitted member with the group name
and a period.

Examples from the documented semantics:

- `{DP DM VBUS}` produces `DP`, `DM`, and `VBUS`.
- `USB1{DP DM}` produces `USB1.DP` and `USB1.DM`.

CircuitCI's importer applies the same expansion only inside explicit
`bus_alias` member strings. It does not infer bus-entry nets from graphics
alone. A bus entry still needs either a scalar wire label or exactly one
resolvable bus label on the attached bus segment.

Conservative implementation boundaries:

- one balanced group per member string,
- group body terms are whitespace-separated,
- scalar terms may use the existing decimal index/range parser,
- alias references are expanded recursively,
- nested group syntax remains fail-closed,
- suffix text after a group remains fail-closed,
- alias-reference cycles remain fail-closed.
