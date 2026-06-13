# EasyEDA/JLC Flying-Probe Pad Import Fixture

This fixture is a small plaintext extract shaped like the JLC/EasyEDA
`FlyingProbeTesting.json` artifact from the peer `urine_monitor` release.

It proves that `import-easyeda-flying-probe` can add connected pad/net evidence
to an existing Board IR project without claiming route or Gerber artwork
ownership by itself. Duplicate identical rows are deduplicated, empty-net rows
are skipped, and missing component references become pad-only generic
placeholders so later fabrication imports can still associate artwork with a
component pin.
