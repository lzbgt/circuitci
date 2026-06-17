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

## Wheel Actuator

`wheel_actuator/root.kicad_sch` is the matching KiCad connectivity skeleton for
the reusable left/right wheel actuator board. It uses local `CircuitCI:*`
symbols for the motor-control MCU, DRV8323 gate driver, CAN transceiver and TVS,
endpoint termination option, encoder/Hall input, preliminary CSD88599Q5DC
three-phase bridge candidate, motor-load envelope, and JST VH actuator-bus
connector.

`wheel_actuator/circuitci.kicad-map.yaml` binds those schematic references and
pins back to the source-backed models used by
`../circuitci/wheel_actuator/project.yaml`.

Import check:

```sh
circuitci import-kicad-schematic \
  demos/smart_robot/kicad/wheel_actuator/root.kicad_sch \
  --mapping demos/smart_robot/kicad/wheel_actuator/circuitci.kicad-map.yaml \
  --output out/smart_robot_wheel_actuator_imported.project.yaml
```

As with the motion-core bridge, the imported Board IR proves connectivity and
model binding only. It does not yet round-trip the motor-drive validation
scenario, current/load budgets, connector-current policy, layout placement
contracts, or routed PCB evidence.

## PMU

`pmu/root.kicad_sch` is the KiCad connectivity skeleton for the reusable PMU
board. It uses local `CircuitCI:*` symbols for the USB-C PD source placeholder,
protected 2S battery source, BQ25798 charger, TPS54331 5 V buck, TPS62162
3.3 V buck, required 3.3 V support passives, e-stop switched rail placeholders,
and the first-pass rail load budgets.

`pmu/circuitci.kicad-map.yaml` binds those schematic references and pins back
to the source-backed models used by `../circuitci/pmu/project.yaml`.

Import check:

```sh
circuitci import-kicad-schematic \
  demos/smart_robot/kicad/pmu/root.kicad_sch \
  --mapping demos/smart_robot/kicad/pmu/circuitci.kicad-map.yaml \
  --output out/smart_robot_pmu_imported.project.yaml
```

The imported Board IR proves PMU connectivity and passive values only. It does
not yet round-trip the BQ25798 configured charge current, power-domain metadata,
e-stop pin states, validation scenarios, regulator layout contracts, or routed
PCB evidence.

## Servo Payload

`servo_payload/root.kicad_sch` is the KiCad connectivity skeleton for the
low-load PWM servo/payload board. It uses local `CircuitCI:*` symbols for the
AT32F435 I2C/OE host pins, PCA9685 PWM driver, four low-load servo design
loads, and four JST XH servo headers.

`servo_payload/circuitci.kicad-map.yaml` binds those schematic references and
pins back to the source-backed models used by
`../circuitci/servo_payload/project.yaml`.

Import check:

```sh
circuitci import-kicad-schematic \
  demos/smart_robot/kicad/servo_payload/root.kicad_sch \
  --mapping demos/smart_robot/kicad/servo_payload/circuitci.kicad-map.yaml \
  --output out/smart_robot_servo_payload_imported.project.yaml
```

The imported Board IR proves I2C, OE, PWM fanout, servo power, and connector
signal connectivity only. It does not yet round-trip connector-current budget
scenarios, servo stall/regeneration assumptions, cable derating, or routed PCB
evidence.
