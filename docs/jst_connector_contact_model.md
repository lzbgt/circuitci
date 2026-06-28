# JST XH/VH Connector Contact-Drop Models

CircuitCI includes reduced generated-SPICE faces for:

- `vendor.jst.xh_3pin_servo_header`
- `vendor.jst.vh_8pin_actuator_bus_header`

The saved JST datasheets are in `docs/research/smart_robot/sources/`:

- `jst_xh_connector_datasheet.pdf`
- `jst_vh_connector_datasheet.pdf`

Both datasheets list 10 mOhm maximum initial contact resistance and 20 mOhm
maximum contact resistance after the relevant environmental/test sequence. The
generated-SPICE faces use 20 mOhm per mated contact so contact-drop examples
are conservative for a datasheet-limited smoke test.

The SPICE faces are explicit pass-through models:

- XH: `VCC` to `VCC_LOAD`, `GND` to `GND_LOAD`, `SIG` to `SIG_LOAD`
- VH: each board-side bus/control pin to the matching `*_LOAD` pin

Existing one-sided connector symbols remain useful for static current and
voltage rating checks. Contact-drop simulation requires binding the load-side
pins so the series contact resistance has a real circuit path.

These faces do not model cable resistance, crimp quality, contact aging beyond
the cited datasheet test value, temperature rise, current sharing, retention,
vibration, CAN signal integrity, EMC, or harness qualification.
