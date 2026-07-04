# onsemi 2N3904 Model

`vendor.onsemi.npn_2n3904` is a source-backed NPN general-purpose transistor
model for generated Board IR SPICE checks.

The model records onsemi's static operating limits:

- 40 V collector-emitter voltage.
- 60 V collector-base voltage.
- 6 V emitter-base voltage.
- 200 mA continuous collector current.
- 625 mW total device dissipation at 25 C ambient with 5 mW/C derating.
- TO-92 pinout with emitter on pin 1, base on pin 2, and collector on pin 3.

The bundled SPICE card is a reduced electrical fit for generated transient
plumbing and operating-limit probes. It is not a gain, noise, switching-time,
thermal, or final production sign-off model.

## Evidence

The official onsemi PDF is retained at
`docs/research/datasheets/onsemi/2n3903_2n3904.pdf`. Source notes and hashes
are recorded in `docs/research/datasheets/onsemi/2n3904_sources.md`.
