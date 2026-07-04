# onsemi SS8050 and SS8550 Models

`vendor.onsemi.ss8050` and `vendor.onsemi.ss8550` are source-backed
generated-SPICE BJT models for onsemi's TO-92 SS8050 NPN and SS8550 PNP
epitaxial silicon transistors.

The SS8050 model records 40 V collector-base, 25 V collector-emitter, 6 V
emitter-base, 1.5 A continuous collector-current, and 1 W power-dissipation
limits. The SS8550 model records the matching signed PNP limits: -40 V
collector-base, -25 V collector-emitter, -6 V emitter-base, -1.5 A continuous
collector current, and 1 W power dissipation. Both models retain TO-92 pinout,
saturation, gain, capacitance, and transition-frequency metadata.

Generated SPICE uses reduced preliminary BJT fits and the shared BJT
operating-limit probes for terminal voltages, collector current, and power
dissipation. The pack is intended for generated-deck plumbing and preliminary
switching/operating-limit screening. It does not prove gain spread, saturation
margin across process and temperature, switching storage time, noise, package
thermal coupling, or final production hardware behavior.

## Evidence

The official onsemi PDFs are retained at
`docs/research/datasheets/onsemi/ss8050.pdf` and
`docs/research/datasheets/onsemi/ss8550.pdf`. Source notes and hashes are
recorded in `docs/research/datasheets/onsemi/ss8050_sources.md` and
`docs/research/datasheets/onsemi/ss8550_sources.md`.
