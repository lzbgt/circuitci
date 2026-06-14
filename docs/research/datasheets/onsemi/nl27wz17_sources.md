# onsemi NL27WZ17 Sources

Source snapshot:

- `docs/research/datasheets/onsemi/nl27wz17.pdf`
- URL: https://www.onsemi.com/download/data-sheet/pdf/nl27wz17-d.pdf
- SHA256: `64e38515090ece6b098f6e7f8ec9790e8cc447a4d165a9c2732312f43e736dd5`
- Retrieved: 2026-06-14

CircuitCI model:

- `libs/vendor/onsemi/logic/nl27wz17.model.yaml`

Use notes:

- `NL27WZ17` is modeled as a dual non-inverting Schmitt-trigger buffer.
- The model is suitable for exact part binding and static connectivity evidence.
- It is not a MOSFET model and is not sufficient by itself for output-drive waveform, MOSFET gate-drive, or laser pulse switching sign-off.

TOF-R5001 relevance:

- The supplied TOF-R5001 schematic text contains `U5 = NL27WZ17DFT2G`.
- That indicates the supplied design files include a buffer-based drive revision, not only the unfixed generic MOSFET path.
