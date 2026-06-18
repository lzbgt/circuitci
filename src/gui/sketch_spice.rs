use super::sketch::{board_child_mapping_mut, encode_edited_project_yaml, validated_graph_id};
use anyhow::{Context, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SketchSpiceKind {
    Resistor,
    Capacitor,
    Inductor,
    DcVoltageSource,
    PulseVoltageSource,
    DcCurrentSource,
    PulseCurrentSource,
}

impl SketchSpiceKind {
    pub(super) const ALL: [Self; 7] = [
        Self::Resistor,
        Self::Capacitor,
        Self::Inductor,
        Self::DcVoltageSource,
        Self::PulseVoltageSource,
        Self::DcCurrentSource,
        Self::PulseCurrentSource,
    ];

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Resistor => "resistor",
            Self::Capacitor => "capacitor",
            Self::Inductor => "inductor",
            Self::DcVoltageSource => "dc_voltage_source",
            Self::PulseVoltageSource => "pulse_voltage_source",
            Self::DcCurrentSource => "dc_current_source",
            Self::PulseCurrentSource => "pulse_current_source",
        }
    }

    pub(super) fn device_prefix(self) -> &'static str {
        match self {
            Self::Resistor => "R",
            Self::Capacitor => "C",
            Self::Inductor => "L",
            Self::DcVoltageSource | Self::PulseVoltageSource => "V",
            Self::DcCurrentSource | Self::PulseCurrentSource => "I",
        }
    }

    pub(super) fn is_passive(self) -> bool {
        matches!(self, Self::Resistor | Self::Capacitor | Self::Inductor)
    }

    pub(super) fn requires_pins(self) -> (&'static str, &'static str) {
        if self.is_passive() {
            ("A", "B")
        } else {
            ("P", "N")
        }
    }
}

impl From<&crate::board_ir::SpicePrimitive> for SketchSpiceKind {
    fn from(value: &crate::board_ir::SpicePrimitive) -> Self {
        match value {
            crate::board_ir::SpicePrimitive::Resistor => Self::Resistor,
            crate::board_ir::SpicePrimitive::Capacitor => Self::Capacitor,
            crate::board_ir::SpicePrimitive::Inductor => Self::Inductor,
            crate::board_ir::SpicePrimitive::DcVoltageSource => Self::DcVoltageSource,
            crate::board_ir::SpicePrimitive::PulseVoltageSource => Self::PulseVoltageSource,
            crate::board_ir::SpicePrimitive::DcCurrentSource => Self::DcCurrentSource,
            crate::board_ir::SpicePrimitive::PulseCurrentSource => Self::PulseCurrentSource,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct SketchComponentSpice {
    pub(super) kind: SketchSpiceKind,
    pub(super) value: f64,
    pub(super) initial_v: Option<f64>,
    pub(super) pulse: SketchSpicePulse,
}

impl SketchComponentSpice {
    pub(super) fn from_board(spice: &crate::board_ir::ComponentSpiceSpec) -> Self {
        let kind = SketchSpiceKind::from(&spice.primitive);
        Self {
            kind,
            value: match kind {
                SketchSpiceKind::Resistor => spice.value_ohm.unwrap_or(1000.0),
                SketchSpiceKind::Capacitor => spice.value_f.unwrap_or(1e-6),
                SketchSpiceKind::Inductor => spice.value_h.unwrap_or(1e-6),
                SketchSpiceKind::DcVoltageSource => spice.dc_v.unwrap_or(5.0),
                SketchSpiceKind::DcCurrentSource => spice.dc_a.unwrap_or(0.1),
                SketchSpiceKind::PulseVoltageSource | SketchSpiceKind::PulseCurrentSource => 0.0,
            },
            initial_v: spice.initial_v,
            pulse: match kind {
                SketchSpiceKind::PulseVoltageSource => spice
                    .pulse
                    .as_ref()
                    .map(SketchSpicePulse::from_voltage)
                    .unwrap_or_else(|| SketchSpicePulse::default_for(kind)),
                SketchSpiceKind::PulseCurrentSource => spice
                    .current_pulse
                    .as_ref()
                    .map(SketchSpicePulse::from_current)
                    .unwrap_or_else(|| SketchSpicePulse::default_for(kind)),
                _ => SketchSpicePulse::default_for(kind),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct SketchSpicePulse {
    pub(super) initial: f64,
    pub(super) pulsed: f64,
    pub(super) delay_us: f64,
    pub(super) rise_us: f64,
    pub(super) fall_us: f64,
    pub(super) width_us: f64,
    pub(super) period_us: f64,
}

impl SketchSpicePulse {
    pub(super) fn default_for(kind: SketchSpiceKind) -> Self {
        let (initial, pulsed) = match kind {
            SketchSpiceKind::PulseVoltageSource => (0.0, 3.3),
            SketchSpiceKind::PulseCurrentSource => (0.0, 0.1),
            _ => (0.0, 0.0),
        };
        Self {
            initial,
            pulsed,
            delay_us: 0.0,
            rise_us: 1.0,
            fall_us: 1.0,
            width_us: 10.0,
            period_us: 20.0,
        }
    }

    fn from_voltage(pulse: &crate::board_ir::SpicePulseSpec) -> Self {
        Self {
            initial: pulse.initial_v,
            pulsed: pulse.pulsed_v,
            delay_us: pulse.delay_us,
            rise_us: pulse.rise_us,
            fall_us: pulse.fall_us,
            width_us: pulse.width_us,
            period_us: pulse.period_us,
        }
    }

    fn from_current(pulse: &crate::board_ir::SpiceCurrentPulseSpec) -> Self {
        Self {
            initial: pulse.initial_a,
            pulsed: pulse.pulsed_a,
            delay_us: pulse.delay_us,
            rise_us: pulse.rise_us,
            fall_us: pulse.fall_us,
            width_us: pulse.width_us,
            period_us: pulse.period_us,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct SketchSpiceDraft {
    pub(super) component_id: String,
    pub(super) kind: SketchSpiceKind,
    pub(super) value: f64,
    pub(super) initial_v: Option<f64>,
    pub(super) pulse: SketchSpicePulse,
}

pub(super) fn draft_from_existing(
    component_id: &str,
    existing: Option<&SketchComponentSpice>,
    kind: SketchSpiceKind,
) -> SketchSpiceDraft {
    let default = SketchComponentSpice {
        kind,
        value: default_value(kind),
        initial_v: None,
        pulse: SketchSpicePulse::default_for(kind),
    };
    let existing = existing.unwrap_or(&default);
    SketchSpiceDraft {
        component_id: component_id.to_string(),
        kind,
        value: if existing.kind == kind {
            existing.value
        } else {
            default_value(kind)
        },
        initial_v: if existing.kind == kind {
            existing.initial_v
        } else {
            None
        },
        pulse: if existing.kind == kind {
            existing.pulse.clone()
        } else {
            SketchSpicePulse::default_for(kind)
        },
    }
}

pub(super) fn replace_component_spice(text: &str, draft: &SketchSpiceDraft) -> Result<String> {
    validate_draft(draft)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let component = project
        .board
        .components
        .get(&draft.component_id)
        .with_context(|| format!("Component {} was not found.", draft.component_id))?;
    let (first_pin, second_pin) = draft.kind.requires_pins();
    if !component.pins.contains_key(first_pin) || !component.pins.contains_key(second_pin) {
        anyhow::bail!(
            "Component {} must have pins {first_pin}/{second_pin} before assigning {} SPICE evidence.",
            draft.component_id,
            draft.kind.label()
        );
    }
    let previous_kind = component
        .spice
        .as_ref()
        .map(|spice| SketchSpiceKind::from(&spice.primitive));

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let components = board_child_mapping_mut(&mut yaml, "components")?;
    let component_mapping = components
        .get_mut(key(&draft.component_id))
        .with_context(|| format!("Component {} was not found in YAML.", draft.component_id))?
        .as_mapping_mut()
        .with_context(|| format!("Component {} must be a YAML object.", draft.component_id))?;
    component_mapping.insert(
        key("spice"),
        serde_yaml_ng::Value::Mapping(spice_mapping(draft)?),
    );
    if let Some(previous_kind) = previous_kind
        && previous_kind != draft.kind
    {
        rewrite_branch_probe_expressions(&mut yaml, &draft.component_id, previous_kind, draft.kind);
    }
    encode_edited_project_yaml(yaml)
}

fn spice_mapping(draft: &SketchSpiceDraft) -> Result<serde_yaml_ng::Mapping> {
    let mut mapping = serde_yaml_ng::Mapping::new();
    insert_string(&mut mapping, "primitive", draft.kind.label());
    match draft.kind {
        SketchSpiceKind::Resistor => insert_number(&mut mapping, "value_ohm", draft.value)?,
        SketchSpiceKind::Capacitor => {
            insert_number(&mut mapping, "value_f", draft.value)?;
            if let Some(initial_v) = draft.initial_v {
                insert_number(&mut mapping, "initial_v", initial_v)?;
            }
        }
        SketchSpiceKind::Inductor => insert_number(&mut mapping, "value_h", draft.value)?,
        SketchSpiceKind::DcVoltageSource => insert_number(&mut mapping, "dc_v", draft.value)?,
        SketchSpiceKind::DcCurrentSource => insert_number(&mut mapping, "dc_a", draft.value)?,
        SketchSpiceKind::PulseVoltageSource => {
            mapping.insert(key("pulse"), pulse_mapping_value(&draft.pulse, true)?);
        }
        SketchSpiceKind::PulseCurrentSource => {
            mapping.insert(
                key("current_pulse"),
                pulse_mapping_value(&draft.pulse, false)?,
            );
        }
    }
    Ok(mapping)
}

fn rewrite_branch_probe_expressions(
    yaml: &mut serde_yaml_ng::Value,
    component_id: &str,
    previous_kind: SketchSpiceKind,
    next_kind: SketchSpiceKind,
) {
    let old_expression = current_expression(previous_kind, component_id);
    let new_expression = current_expression(next_kind, component_id);
    if old_expression == new_expression {
        return;
    }
    let Some(scenarios) = yaml
        .as_mapping_mut()
        .and_then(|project| project.get_mut(key("scenarios")))
        .and_then(serde_yaml_ng::Value::as_sequence_mut)
    else {
        return;
    };
    for scenario in scenarios {
        let Some(probes) = scenario
            .as_mapping_mut()
            .and_then(|scenario| scenario.get_mut(key("analog")))
            .and_then(serde_yaml_ng::Value::as_mapping_mut)
            .and_then(|analog| analog.get_mut(key("probes")))
            .and_then(serde_yaml_ng::Value::as_sequence_mut)
        else {
            continue;
        };
        for probe in probes {
            let Some(mapping) = probe.as_mapping_mut() else {
                continue;
            };
            let Some(expression) = mapping
                .get(key("expression"))
                .and_then(serde_yaml_ng::Value::as_str)
            else {
                continue;
            };
            let renamed = expression.replace(&old_expression, &new_expression);
            if renamed != expression {
                mapping.insert(key("expression"), serde_yaml_ng::Value::String(renamed));
            }
        }
    }
}

fn current_expression(kind: SketchSpiceKind, component_id: &str) -> String {
    if kind.is_passive() {
        format!(
            "I(VCCI_{})",
            spice_element_name(kind.device_prefix(), component_id)
        )
    } else {
        format!(
            "I({})",
            spice_element_name(kind.device_prefix(), component_id)
        )
    }
}

fn spice_element_name(prefix: &str, component_id: &str) -> String {
    let suffix = spice_element_suffix(component_id);
    if suffix.starts_with(prefix) {
        suffix
    } else {
        format!("{prefix}{suffix}")
    }
}

fn spice_element_suffix(component_id: &str) -> String {
    let mut suffix = String::new();
    for character in component_id.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            suffix.push(character);
        } else {
            suffix.push('_');
        }
    }
    if suffix.is_empty() {
        suffix.push('X');
    }
    suffix
}

fn validate_draft(draft: &SketchSpiceDraft) -> Result<()> {
    validated_graph_id(&draft.component_id, "component")?;
    match draft.kind {
        SketchSpiceKind::Resistor | SketchSpiceKind::Capacitor | SketchSpiceKind::Inductor => {
            if !draft.value.is_finite() || draft.value <= 0.0 {
                anyhow::bail!("Passive SPICE value must be finite and positive.");
            }
            if let Some(initial_v) = draft.initial_v
                && (!initial_v.is_finite() || draft.kind != SketchSpiceKind::Capacitor)
            {
                anyhow::bail!("Capacitor initial voltage must be finite.");
            }
        }
        SketchSpiceKind::DcVoltageSource | SketchSpiceKind::DcCurrentSource => {
            if !draft.value.is_finite() {
                anyhow::bail!("DC source value must be finite.");
            }
            if draft.initial_v.is_some() {
                anyhow::bail!("Initial voltage only applies to capacitors.");
            }
        }
        SketchSpiceKind::PulseVoltageSource | SketchSpiceKind::PulseCurrentSource => {
            if draft.initial_v.is_some() {
                anyhow::bail!("Initial voltage only applies to capacitors.");
            }
            validate_pulse(&draft.pulse)?;
        }
    }
    Ok(())
}

fn validate_pulse(pulse: &SketchSpicePulse) -> Result<()> {
    for (field, value) in [
        ("initial", pulse.initial),
        ("pulsed", pulse.pulsed),
        ("delay_us", pulse.delay_us),
        ("rise_us", pulse.rise_us),
        ("fall_us", pulse.fall_us),
        ("width_us", pulse.width_us),
        ("period_us", pulse.period_us),
    ] {
        if !value.is_finite()
            || (field.ends_with("_us") && value < 0.0)
            || matches!(field, "width_us" | "period_us") && value <= 0.0
        {
            anyhow::bail!("Pulse field {field} must be finite and in range.");
        }
    }
    Ok(())
}

fn pulse_mapping_value(pulse: &SketchSpicePulse, voltage: bool) -> Result<serde_yaml_ng::Value> {
    validate_pulse(pulse)?;
    let mut mapping = serde_yaml_ng::Mapping::new();
    if voltage {
        insert_number(&mut mapping, "initial_v", pulse.initial)?;
        insert_number(&mut mapping, "pulsed_v", pulse.pulsed)?;
    } else {
        insert_number(&mut mapping, "initial_a", pulse.initial)?;
        insert_number(&mut mapping, "pulsed_a", pulse.pulsed)?;
    }
    insert_number(&mut mapping, "delay_us", pulse.delay_us)?;
    insert_number(&mut mapping, "rise_us", pulse.rise_us)?;
    insert_number(&mut mapping, "fall_us", pulse.fall_us)?;
    insert_number(&mut mapping, "width_us", pulse.width_us)?;
    insert_number(&mut mapping, "period_us", pulse.period_us)?;
    Ok(serde_yaml_ng::Value::Mapping(mapping))
}

pub(super) fn default_value(kind: SketchSpiceKind) -> f64 {
    match kind {
        SketchSpiceKind::Resistor => 1000.0,
        SketchSpiceKind::Capacitor => 1e-6,
        SketchSpiceKind::Inductor => 1e-6,
        SketchSpiceKind::DcVoltageSource => 5.0,
        SketchSpiceKind::DcCurrentSource => 0.1,
        SketchSpiceKind::PulseVoltageSource | SketchSpiceKind::PulseCurrentSource => 0.0,
    }
}

fn insert_string(mapping: &mut serde_yaml_ng::Mapping, name: &str, value: &str) {
    mapping.insert(key(name), serde_yaml_ng::Value::String(value.to_string()));
}

fn insert_number(mapping: &mut serde_yaml_ng::Mapping, name: &str, value: f64) -> Result<()> {
    mapping.insert(
        key(name),
        serde_yaml_ng::to_value(value).context("Failed to encode SPICE primitive number.")?,
    );
    Ok(())
}

fn key(name: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::Value::String(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::{SketchSpiceDraft, SketchSpiceKind, SketchSpicePulse, replace_component_spice};

    fn project_yaml() -> &'static str {
        "project:
  name: sketch_spice_test
  version: 0.1.0
board:
  components:
    R1:
      model: generic.analog.resistor
      spice: { primitive: resistor, value_ohm: 1000 }
      pins: { A: rail, B: out }
    V1:
      model: generic.analog.dc_voltage_source
      spice: { primitive: dc_voltage_source, dc_v: 5.0 }
      pins: { P: rail, N: gnd }
  nets:
    rail: { kind: power }
    out: { kind: digital_or_analog }
    gnd: { kind: ground }
scenarios:
  - name: tran
    type: analog_transient
    checks: [SPICE_TRANSIENT_ANALYSIS]
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated: { ground_net: gnd, components: [R1, V1] }
      model_files: []
      node_bindings:
        - { net: rail, node: rail }
        - { net: out, node: out }
        - { net: gnd, node: '0' }
      pin_bindings:
        - { endpoint: { component: R1, pin: A }, node: rail }
        - { endpoint: { component: R1, pin: B }, node: out }
        - { endpoint: { component: V1, pin: P }, node: rail }
        - { endpoint: { component: V1, pin: N }, node: '0' }
      analysis: { type: tran, stop_time_us: 10, max_step_us: 1 }
      stimuli: []
      probes:
        - { name: r1_current, expression: 'I(VCCI_R1)', quantity: current }
        - { name: r1_power, expression: 'V(rail,out)*I(VCCI_R1)', quantity: power }
        - { name: v1_current, expression: 'I(V1)', quantity: current }
      assertions: []
"
    }

    #[test]
    fn replaces_passive_value() {
        let edited = replace_component_spice(
            project_yaml(),
            &SketchSpiceDraft {
                component_id: "R1".to_string(),
                kind: SketchSpiceKind::Resistor,
                value: 2200.0,
                initial_v: None,
                pulse: SketchSpicePulse::default_for(SketchSpiceKind::Resistor),
            },
        )
        .unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();

        assert_eq!(
            project
                .board
                .components
                .get("R1")
                .unwrap()
                .spice
                .as_ref()
                .unwrap()
                .value_ohm,
            Some(2200.0)
        );
    }

    #[test]
    fn changing_passive_kind_updates_current_probe_branches() {
        let edited = replace_component_spice(
            project_yaml(),
            &SketchSpiceDraft {
                component_id: "R1".to_string(),
                kind: SketchSpiceKind::Capacitor,
                value: 1e-6,
                initial_v: Some(0.25),
                pulse: SketchSpicePulse::default_for(SketchSpiceKind::Capacitor),
            },
        )
        .unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();

        assert!(
            analog
                .probes
                .iter()
                .any(|probe| { probe.name == "r1_current" && probe.expression == "I(VCCI_CR1)" })
        );
        assert!(analog.probes.iter().any(|probe| {
            probe.name == "r1_power" && probe.expression == "V(rail,out)*I(VCCI_CR1)"
        }));
    }

    #[test]
    fn changing_source_kind_updates_native_branch_probe() {
        let edited = replace_component_spice(
            project_yaml(),
            &SketchSpiceDraft {
                component_id: "V1".to_string(),
                kind: SketchSpiceKind::DcCurrentSource,
                value: 0.05,
                initial_v: None,
                pulse: SketchSpicePulse::default_for(SketchSpiceKind::DcCurrentSource),
            },
        )
        .unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();

        assert!(
            analog
                .probes
                .iter()
                .any(|probe| { probe.name == "v1_current" && probe.expression == "I(IV1)" })
        );
    }

    #[test]
    fn rejects_source_kind_without_source_pins() {
        let error = replace_component_spice(
            project_yaml(),
            &SketchSpiceDraft {
                component_id: "R1".to_string(),
                kind: SketchSpiceKind::DcVoltageSource,
                value: 5.0,
                initial_v: None,
                pulse: SketchSpicePulse::default_for(SketchSpiceKind::DcVoltageSource),
            },
        )
        .unwrap_err();

        assert!(error.to_string().contains("must have pins P/N"));
    }
}
