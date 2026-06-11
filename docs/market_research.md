# Market Research Notes

These notes preserve the product evidence captured in `project.md` for future reuse.

## Existing Category Proof

- Proteus VSM demonstrates that firmware-aware mixed-mode circuit simulation is a real product category.
- Renode demonstrates that embedded simulation can be CI-friendly.
- ngspice and Xyce demonstrate that open analog simulation engines exist as integration backends.
- JITX demonstrates that circuit and PCB design can be represented as code, which strengthens the need for hardware CI.

## Product Wedge

CircuitCI should not clone a GUI-first simulator. Its wedge is:

```text
agent/API/CLI -> import design -> run validation profiles -> emit JSON failures -> agent repairs design -> repeat
```

## Source Links From Project Brief

- Labcenter Proteus simulation: https://www.labcenter.com/simulation/
- Labcenter Proteus education/product material: https://www.labcenter.com/education/
- Labcenter Proteus main site: https://www.labcenter.com/
- Renode: https://renode.io/
- JITX: https://www.jitx.com/
- JITX skills: https://github.com/JITx-Inc/jitx-skills
- HWE-Bench paper: https://arxiv.org/abs/2603.18102

