# Smart Robot KiCad CAD Bridge

This folder contains KiCad-source artifacts that are meant to be imported back
through CircuitCI during schematic/layout iteration.

## Motion Core

`motion_core/root.kicad_sch` is the first KiCad schematic skeleton for the
validated motion-core logical model. It intentionally uses local `CircuitCI:*`
symbols with named pins so agents can edit connectivity without depending on a
complete production symbol library yet.

`motion_core/circuitci.kicad-map.yaml` binds those schematic references and
pins back to the same source-backed models used by
`../circuitci/motion_core/project.yaml`.

Current scope:

- LicheeRV Nano purchased module interface.
- AT32F435 motion MCU.
- ICM-42688-P IMU SPI/interrupt link.
- TCAN3413 CAN transceiver.
- THVD1450 RS485 transceiver.
- ESD2CAN24-Q1 and ESDS552 static bus TVS devices.
- 120 ohm CAN and RS485 endpoint termination options.

Import check:

```sh
circuitci import-kicad-schematic \
  demos/smart_robot/kicad/motion_core/root.kicad_sch \
  --mapping demos/smart_robot/kicad/motion_core/circuitci.kicad-map.yaml \
  --output out/smart_robot_motion_core_imported.project.yaml
```

The imported Board IR is a connectivity source artifact. It does not yet carry
all fields from the richer logical validation model, such as `power_domains`,
validation scenarios, pre-layout placement contracts, or routed PCB evidence.
Keep using `../circuitci/motion_core/project.yaml` for sign-off checks until
the KiCad importer round-trips those fields or explicit overlay steps are
added.
