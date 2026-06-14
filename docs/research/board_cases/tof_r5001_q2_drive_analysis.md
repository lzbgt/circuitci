# TOF-R5001 Q2 Drive Analysis

Date: 2026-06-14

Local input evidence:

- `/Users/zongbaolu/Downloads/TOF-R5001.SchDoc`
- `/Users/zongbaolu/Downloads/TOF-R5001.PCBDOC`
- `/Users/zongbaolu/Downloads/TOF-R5001.PrjPCB`

CircuitCI status:

- CircuitCI does not currently include a native Altium SchDoc/PcbDoc importer.
- The analysis used extracted OLE text plus a reduced local Board/Project evidence model under `out/tof_r5001_analysis/`.
- `out/` is intentionally ignored and may contain user-specific board artifacts; this tracked note keeps only reusable findings.

Known bench/user facts:

- `VLD` is actually `21.8 V`.
- `Q9` is not mounted.
- The original `Q2` drive path had a real drive issue.
- The circuit works after replacing `Q2` with a different MOSFET. The working
  replacement MOSFET part number is currently unknown.
- The trigger signal is a pulse from about `7 kHz` to `30 kHz`, with about
  `30 ns` pulse width and an upward triangular edge shape. At the `30 kHz`
  worst case, `30 ns` corresponds to `0.09%` duty cycle.
- The VCSEL is powered by the C27/C28 pulse-storage network. That means the
  original Q2 path must be analyzed as a precharged-capacitor pulse-discharge
  switch, not as a DC or steady resistive load.

Extracted design evidence:

- `Q2` is labeled `CSD17484F4` in the schematic and PCB text, with schematic library reference `MOSFET-N`.
- `Q2` is in the `LED-TX-` low-side switch path.
- `Q9` appears in the PCB document as `AO3400A`, but the project variation file contains no mounted/DNP variation data. The user-supplied not-mounted fact must therefore override the PCB component listing.
- The design text contains stale `VLD-28.5V` labeling, while the measured/known rail for validation is `VLD = 21.8 V`.
- The LD driver section includes `C27`, documented in the extracted schematic
  text as `0402 22pF 1% 50V`, and `C28`, documented as
  `NC/0402 56pF 1% 50V`. The user-supplied circuit fact says the VCSEL is
  powered by C27/C28, but C28's mounted state must still be confirmed from BOM
  or assembly evidence because the schematic comment marks it `NC`.
- The supplied schematic text contains `U5 = NL27WZ17DFT2G`. This identifies a
  dual Schmitt-trigger buffer in the design, not the `Q2` MOSFET replacement.

Source-backed component models added:

- `libs/vendor/ti/discrete/csd17484f4.model.yaml`
- `libs/vendor/onsemi/logic/nl27wz17.model.yaml`

Datasheet snapshots:

- `docs/research/datasheets/ti/csd17484f4.pdf`
- `docs/research/datasheets/onsemi/nl27wz17.pdf`

Tool result on the reduced local evidence:

```text
CircuitCI tof_r5001_original_q2_drive_evidence: fail (critical=1, warning=0, info=0)
```

The first tool run emitted `ANALOG_MODEL_UNAVAILABLE`: a real `CSD17484F4`
SPICE model required for physical transient validation was not available. That
was the correct behavior before a CSD17484F4 model face existed. Using the
Altium generic `MOSFET-N` symbol as a physical model would hide the actual Q2
drive problem.

The library now includes `models/spice/ti/csd17484f4.lib`, a coarse Level-1
datasheet-fit card. CircuitCI also gained generated-deck support for capacitor
initial conditions, so the C27 pulse-storage topology can be represented
directly instead of approximating the VCSEL path as a steady load. This improves
preliminary generated-deck verification but does not make `CSD17484F4` a final
validated Q2 candidate for the laser switch. Final root-cause work still needs
the confirmed mounted C27/C28 values and topology, a sourced vendor or
bench-calibrated Q2 model, measured gate/drain/current waveforms, or the
confirmed working replacement MOSFET identity.

Interpretation:

- `CSD17484F4` is the original `Q2` identity extracted from the design files.
- CircuitCI did not validate `CSD17484F4` as an electrically sufficient Q2
  candidate for the laser switch.
- The tool first reported that the original Q2 path cannot be physically
  trusted with the generic Altium `NMOS` model and needs sourced model or
  waveform evidence.
- The added datasheet-fit CSD17484F4 model can support preliminary screening,
  but it does not close the original root-cause question by itself.
- Because the VCSEL pulse is powered from C27/C28, the key dynamic stress is
  pulsed capacitor discharge. With C27 alone, the ideal stored energy at
  `21.8 V` is `0.5 * 22 pF * 21.8^2 = 5.23 nJ`. If C28 is also mounted, the
  documented nominal capacitance would be `78 pF`, or `18.54 nJ`. The C28
  contribution should not be used for sign-off until its mounted state is
  proven.

Replacement MOSFET lookup:

- User-provided memory: possible replacement text `VBQF1308`.
- Direct web search for exact `VBQF1308` plus `LCSC`, `datasheet`, and
  `VBsemi Elec` returned no exact downloadable source in this session.
- A nearby VBsemi family part, `VBQF1310`, is listed by LCSC as an N-channel
  `30 V`, `30 A`, `DFN3x3-8` MOSFET, which shows the `VBQF13xx` family is
  plausible, but it is not evidence that `VBQF1308` was the installed Q2.
- Do not add a `VBQF1308` component model or treat it as the working Q2
  replacement until the exact part number is confirmed from a package marking,
  invoice/BOM, photo, or datasheet PDF.

Issues identified for the original circuit review:

1. `Q2` must be validated as TI `CSD17484F4`, not as a generic NMOS.
2. `Q9` must be excluded from the mounted circuit because the board fact says it is not mounted.
3. `VLD` must be validated at `21.8 V`; the stale `VLD-28.5V` label should not drive analysis.
4. The supplied design includes `NL27WZ17DFT2G`, which is a dual non-inverting Schmitt-trigger buffer, not a MOSFET and not the replacement `Q2` part.
5. The working replacement `Q2` MOSFET part number is not yet known.
6. The exact pre-fix Q2 drive failure still needs either the older schematic, the replacement MOSFET identity, a vendor/bench-calibrated CSD17484F4 SPICE model, mounted C27/C28 evidence, or measured gate/drain/current waveforms to prove the dynamic failure mode.

Follow-up evidence that would make the diagnosis stronger:

- The pre-fix schematic/layout revision and the working replacement `Q2`
  MOSFET part number.
- Laser/LED pulse current, pulse width, and duty cycle.
- C27/C28 mounted state and exact connectivity around VCSEL anode/cathode and
  `LED-TX-`.
- Q2 gate waveform, drain waveform, and load current waveform from the failing original circuit.
- A sourced vendor or bench-calibrated SPICE model for `CSD17484F4`.
