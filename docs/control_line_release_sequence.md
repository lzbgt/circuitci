# Control-Line Release Sequence Design

This slice lets CircuitCI derive reset/boot strap outcomes from modeled host control-line effects instead of only consuming observed strap states. It is still generic: CH340, STM32, ESP32 auto-download, STC ISP, and other boot-entry circuits are represented by scenario/model data, not engine branches.

## Product Boundary

The runtime may validate:

- a host control-line sequence,
- source endpoint connectivity for each host-driven line,
- modeled effects from a host line onto a target reset or strap pin,
- release delays and target sample timing,
- whether the derived target pin states match a required boot mode.

The runtime must not:

- solve transistor storage charge,
- infer hidden RC networks from a schematic crop,
- hardcode DTR, RTS, BOOT0, NRST, CH340, STM32, Q2, or Q3,
- claim physical HIL proof for host-controlled flash boot.

## Peer Evidence

Persistent source notes are in [um_stm32l4_acceptance_sources.md](research/um_stm32l4_acceptance_sources.md).

The relevant facts are:

- the fabricated UM board has a transistor/diode reset/BOOT0 network rather than direct DTR/RTS-to-target-pin wiring,
- runtime-safe application idle is host modem-control `DTR=0` for BOOT0 disabled and `RTS=1` for NRST released,
- runtime reset is `RTS: 1 -> 0 -> 1` with DTR held low,
- the release edge is a race between BOOT0 falling and NRST rising,
- host-issued flash-boot reset is not proven equivalent to manual reset,
- a Q3 base-emitter bleed is the recommended first rework to make BOOT0 release more deterministic.

## Scenario Contract

Add `control_line_sequence` scenarios with `CONTROL_LINE_RELEASE_SEQUENCE`:

```yaml
scenarios:
  - name: derived_app_boot_release
    type: control_line_sequence
    target:
      component: U1
      reset_pin: NRST
    checks:
      - CONTROL_LINE_RELEASE_SEQUENCE
    required_boot_mode: application
    timing:
      power_valid_at_us: 1200
      reset_release_at_us: 5000
      boot_sample_at_us: 5100
    control_effects:
      - name: boot_select
        source:
          component: U5
          pin: DTR_N
        target:
          component: U1
          pin: BOOT0
        asserted_state: high
        released_state: low
        release_delay_us: 400
      - name: reset
        source:
          component: U5
          pin: RTS_N
        target:
          component: U1
          pin: NRST
        asserted_state: low
        released_state: high
        release_delay_us: 0
    events:
      - at_us: 0
        action: control_line
        line: boot_select
        asserted: true
      - at_us: 1000
        action: control_line
        line: reset
        asserted: true
      - at_us: 4900
        action: control_line
        line: boot_select
        asserted: false
      - at_us: 5000
        action: control_line
        line: reset
        asserted: false
```

`control_effects` describe abstract line-to-target behavior after all transistor, diode, inversion, and RC details have been reduced to:

- target pin state while the line is asserted,
- target pin state after release,
- release delay from the host event to the target pin reaching released state.

`events` describe the host-visible sequence. `asserted` is semantic and local to the named effect; it is not required to equal a physical high or low voltage. It is a reduced behavioral model of the circuit after inversion, transistor, diode, and storage effects.

Every `control_effect` must have at least one explicit `control_line` event before each evaluated sample time. The runtime must not infer initial line states.

`suggest-scenarios` can generate a runnable `CONTROL_LINE_RELEASE_SEQUENCE`
template from `board.runtime.control_line_sequences[]` when that runtime
evidence already includes the target, required boot mode, timing, complete
control effects, and explicit events. The suggestion path is only a projection
from reviewed runtime evidence into scenario YAML; it does not infer a
transistor-level control network.

## Validation Algorithm

`CONTROL_LINE_RELEASE_SEQUENCE` should:

1. Resolve `target.component`.
2. Resolve required straps from `component.behavior.boot.modes[required_boot_mode]`.
3. Verify each `control_effect.source` resolves to an output-capable model pin.
4. If imported KiCad pin electrical metadata is present, verify the source pin
   is KiCad-output-capable.
5. Verify each `control_effect.target` resolves to the target component and an input-capable model pin.
6. If imported KiCad pin electrical metadata is present, verify the target pin
   is KiCad-input-capable.
7. Use `reset_release_at_us` to evaluate reset release and `boot_sample_at_us` to evaluate boot straps.
8. For each effect and sample time, find the last `control_line` event at or before that sample time. Missing events are critical validation-input errors.
9. If the last event is asserted, derive `asserted_state`.
10. If the last event is released and `sample_time - event.at_us >= release_delay_us`, derive `released_state`; otherwise derive `asserted_state` because release has not settled.
11. Require reset to be released at `reset_release_at_us` and still released at `boot_sample_at_us`.
12. Compare derived strap states at `boot_sample_at_us` with required boot-mode straps.

## Initial Fixtures

Add:

- `examples/um_stm32l4_control_line_app_release_bad`: fail because BOOT0 release delay keeps derived BOOT0 high at sample time.
- `examples/um_stm32l4_control_line_app_release_fixed`: pass because the fixture assumes a reduced BOOT0 release delay consistent with the Q3 bleed rework intent.
- `examples/um_stm32l4_control_line_rom_entry`: pass because BOOT0 remains high and reset releases after power valid for ROM entry.

## Definition Of Done

- Design is reviewed before implementation.
- Schemas cover `control_effects` and control-line event fields.
- Rust validation remains generic and model/fixture driven.
- CLI tests cover bad release, fixed release, and ROM entry.
- Reports include a non-blocking limitation for abstract control-line modeling.
- `cargo clippy --all-targets -- -D warnings` and `cargo test` pass.
