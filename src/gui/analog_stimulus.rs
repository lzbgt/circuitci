use anyhow::{Context, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AnalogStimulusKind {
    DcVoltage,
    PulseVoltage,
    DcCurrent,
    PulseCurrent,
}

impl AnalogStimulusKind {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::DcVoltage => "DC voltage source",
            Self::PulseVoltage => "Pulse voltage source",
            Self::DcCurrent => "DC current source",
            Self::PulseCurrent => "Pulse current source",
        }
    }

    pub(super) fn primitive_name(self) -> &'static str {
        match self {
            Self::DcVoltage => "dc_voltage_source",
            Self::PulseVoltage => "pulse_voltage_source",
            Self::DcCurrent => "dc_current_source",
            Self::PulseCurrent => "pulse_current_source",
        }
    }

    pub(super) fn value_unit(self) -> &'static str {
        match self {
            Self::DcVoltage | Self::PulseVoltage => " V",
            Self::DcCurrent | Self::PulseCurrent => " A",
        }
    }

    pub(super) fn is_pulse(self) -> bool {
        matches!(self, Self::PulseVoltage | Self::PulseCurrent)
    }
}

#[derive(Debug, Clone)]
pub(super) struct AnalogStimulusPulseDraft {
    pub(super) initial: f64,
    pub(super) pulsed: f64,
    pub(super) delay_us: f64,
    pub(super) rise_us: f64,
    pub(super) fall_us: f64,
    pub(super) width_us: f64,
    pub(super) period_us: f64,
}

impl AnalogStimulusPulseDraft {
    fn default_for(kind: AnalogStimulusKind) -> Self {
        let (initial, pulsed) = match kind {
            AnalogStimulusKind::PulseVoltage => (0.0, 3.3),
            AnalogStimulusKind::PulseCurrent => (0.0, 0.1),
            AnalogStimulusKind::DcVoltage | AnalogStimulusKind::DcCurrent => (0.0, 0.0),
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
}

#[derive(Debug, Clone)]
pub(super) struct AnalogStimulusChoice {
    pub(super) scenario_name: String,
    pub(super) component_id: String,
    pub(super) kind: AnalogStimulusKind,
    pub(super) dc_value: f64,
    pub(super) pulse: AnalogStimulusPulseDraft,
}

impl AnalogStimulusChoice {
    pub(super) fn label(&self) -> String {
        format!(
            "{} / {} ({})",
            self.scenario_name,
            self.component_id,
            self.kind.label()
        )
    }
}

#[derive(Debug, Clone)]
pub(super) struct AnalogStimulusDraft {
    pub(super) scenario_name: String,
    pub(super) component_id: String,
    pub(super) kind: AnalogStimulusKind,
    pub(super) dc_value: f64,
    pub(super) pulse: AnalogStimulusPulseDraft,
}

pub(super) fn analog_stimulus_choices(text: &str) -> Result<Vec<AnalogStimulusChoice>> {
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let mut choices = Vec::new();
    for scenario in &project.scenarios {
        let Some(analog) = &scenario.analog else {
            continue;
        };
        if analog.netlist_source != crate::board_ir::AnalogNetlistSource::GeneratedFromBoard {
            continue;
        }
        let Some(generated) = &analog.generated else {
            continue;
        };
        for component_id in &generated.components {
            let Some(component) = project.board.components.get(component_id) else {
                continue;
            };
            let Some(spice) = &component.spice else {
                continue;
            };
            let Some(kind) = stimulus_kind(&spice.primitive) else {
                continue;
            };
            choices.push(AnalogStimulusChoice {
                scenario_name: scenario.name.clone(),
                component_id: component_id.clone(),
                kind,
                dc_value: match kind {
                    AnalogStimulusKind::DcVoltage => spice.dc_v.unwrap_or(0.0),
                    AnalogStimulusKind::DcCurrent => spice.dc_a.unwrap_or(0.0),
                    AnalogStimulusKind::PulseVoltage | AnalogStimulusKind::PulseCurrent => 0.0,
                },
                pulse: match kind {
                    AnalogStimulusKind::PulseVoltage => spice
                        .pulse
                        .as_ref()
                        .map(voltage_pulse_to_draft)
                        .unwrap_or_else(|| AnalogStimulusPulseDraft::default_for(kind)),
                    AnalogStimulusKind::PulseCurrent => spice
                        .current_pulse
                        .as_ref()
                        .map(current_pulse_to_draft)
                        .unwrap_or_else(|| AnalogStimulusPulseDraft::default_for(kind)),
                    AnalogStimulusKind::DcVoltage | AnalogStimulusKind::DcCurrent => {
                        AnalogStimulusPulseDraft::default_for(kind)
                    }
                },
            });
        }
    }
    choices.sort_by(|left, right| {
        left.scenario_name
            .cmp(&right.scenario_name)
            .then_with(|| left.component_id.cmp(&right.component_id))
    });
    Ok(choices)
}

pub(super) fn replace_analog_stimulus(text: &str, draft: &AnalogStimulusDraft) -> Result<String> {
    validate_stimulus_draft(draft)?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == draft.scenario_name)
        .with_context(|| format!("Scenario {} was not found.", draft.scenario_name))?;
    let analog = scenario
        .analog
        .as_ref()
        .with_context(|| format!("Scenario {} is not an analog scenario.", scenario.name))?;
    if analog.netlist_source != crate::board_ir::AnalogNetlistSource::GeneratedFromBoard {
        anyhow::bail!(
            "Stimulus editing requires a generated_from_board analog scenario; scenario {} uses a file-backed deck.",
            scenario.name
        );
    }
    let generated = analog.generated.as_ref().with_context(|| {
        format!(
            "Scenario {} must declare analog.generated before source stimulus can be edited.",
            scenario.name
        )
    })?;
    if !generated
        .components
        .iter()
        .any(|component_id| component_id == &draft.component_id)
    {
        anyhow::bail!(
            "Scenario {} does not include generated component {}.",
            scenario.name,
            draft.component_id
        );
    }
    let component = project
        .board
        .components
        .get(&draft.component_id)
        .with_context(|| format!("Component {} was not found.", draft.component_id))?;
    let spice = component
        .spice
        .as_ref()
        .with_context(|| format!("Component {} has no spice primitive.", draft.component_id))?;
    let actual_kind = stimulus_kind(&spice.primitive).with_context(|| {
        format!(
            "Component {} is not an editable source stimulus primitive.",
            draft.component_id
        )
    })?;
    if actual_kind != draft.kind {
        anyhow::bail!(
            "Component {} is {}, but the editor was loaded for {}.",
            draft.component_id,
            actual_kind.label(),
            draft.kind.label()
        );
    }

    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    let components = board_components_mapping_mut(&mut yaml)?;
    let component_mapping = components
        .get_mut(key(&draft.component_id))
        .with_context(|| format!("Component {} was not found in YAML.", draft.component_id))?
        .as_mapping_mut()
        .with_context(|| format!("Component {} must be a YAML object.", draft.component_id))?;
    let spice_mapping = component_mapping
        .get_mut(key("spice"))
        .with_context(|| format!("Component {} must declare spice.", draft.component_id))?
        .as_mapping_mut()
        .with_context(|| {
            format!(
                "Component {} spice must be a YAML object.",
                draft.component_id
            )
        })?;
    insert_string(spice_mapping, "primitive", draft.kind.primitive_name());
    match draft.kind {
        AnalogStimulusKind::DcVoltage => {
            insert_number(spice_mapping, "dc_v", draft.dc_value)?;
            remove_keys(spice_mapping, &["dc_a", "pulse", "current_pulse"]);
        }
        AnalogStimulusKind::DcCurrent => {
            insert_number(spice_mapping, "dc_a", draft.dc_value)?;
            remove_keys(spice_mapping, &["dc_v", "pulse", "current_pulse"]);
        }
        AnalogStimulusKind::PulseVoltage => {
            spice_mapping.insert(key("pulse"), pulse_mapping_value(&draft.pulse, true)?);
            remove_keys(spice_mapping, &["dc_v", "dc_a", "current_pulse"]);
        }
        AnalogStimulusKind::PulseCurrent => {
            spice_mapping.insert(
                key("current_pulse"),
                pulse_mapping_value(&draft.pulse, false)?,
            );
            remove_keys(spice_mapping, &["dc_v", "dc_a", "pulse"]);
        }
    }
    let updated = serde_yaml_ng::to_string(&yaml).context("Failed to serialize edited YAML.")?;
    let _: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&updated)
        .context("Edited source stimulus YAML is not valid Board IR.")?;
    Ok(updated)
}

fn stimulus_kind(primitive: &crate::board_ir::SpicePrimitive) -> Option<AnalogStimulusKind> {
    Some(match primitive {
        crate::board_ir::SpicePrimitive::DcVoltageSource => AnalogStimulusKind::DcVoltage,
        crate::board_ir::SpicePrimitive::PulseVoltageSource => AnalogStimulusKind::PulseVoltage,
        crate::board_ir::SpicePrimitive::DcCurrentSource => AnalogStimulusKind::DcCurrent,
        crate::board_ir::SpicePrimitive::PulseCurrentSource => AnalogStimulusKind::PulseCurrent,
        crate::board_ir::SpicePrimitive::Resistor
        | crate::board_ir::SpicePrimitive::Capacitor
        | crate::board_ir::SpicePrimitive::Inductor => return None,
    })
}

fn voltage_pulse_to_draft(pulse: &crate::board_ir::SpicePulseSpec) -> AnalogStimulusPulseDraft {
    AnalogStimulusPulseDraft {
        initial: pulse.initial_v,
        pulsed: pulse.pulsed_v,
        delay_us: pulse.delay_us,
        rise_us: pulse.rise_us,
        fall_us: pulse.fall_us,
        width_us: pulse.width_us,
        period_us: pulse.period_us,
    }
}

fn current_pulse_to_draft(
    pulse: &crate::board_ir::SpiceCurrentPulseSpec,
) -> AnalogStimulusPulseDraft {
    AnalogStimulusPulseDraft {
        initial: pulse.initial_a,
        pulsed: pulse.pulsed_a,
        delay_us: pulse.delay_us,
        rise_us: pulse.rise_us,
        fall_us: pulse.fall_us,
        width_us: pulse.width_us,
        period_us: pulse.period_us,
    }
}

fn validate_stimulus_draft(draft: &AnalogStimulusDraft) -> Result<()> {
    validated_id(&draft.scenario_name, "scenario name")?;
    validated_id(&draft.component_id, "component id")?;
    match draft.kind {
        AnalogStimulusKind::DcVoltage | AnalogStimulusKind::DcCurrent => {
            if !draft.dc_value.is_finite() {
                anyhow::bail!("DC source value must be finite.");
            }
        }
        AnalogStimulusKind::PulseVoltage | AnalogStimulusKind::PulseCurrent => {
            validate_pulse(&draft.pulse)?;
        }
    }
    Ok(())
}

fn validate_pulse(pulse: &AnalogStimulusPulseDraft) -> Result<()> {
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

fn board_components_mapping_mut(
    yaml: &mut serde_yaml_ng::Value,
) -> Result<&mut serde_yaml_ng::Mapping> {
    yaml.as_mapping_mut()
        .context("Board IR project must be a YAML object.")?
        .get_mut(key("board"))
        .context("Board IR project must declare board.")?
        .as_mapping_mut()
        .context("Board IR board must be a YAML object.")?
        .get_mut(key("components"))
        .context("Board IR board must declare components.")?
        .as_mapping_mut()
        .context("Board IR board.components must be a YAML object.")
}

fn pulse_mapping_value(
    pulse: &AnalogStimulusPulseDraft,
    voltage: bool,
) -> Result<serde_yaml_ng::Value> {
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

fn remove_keys(mapping: &mut serde_yaml_ng::Mapping, fields: &[&str]) {
    for field in fields {
        mapping.remove(key(field));
    }
}

fn insert_string(mapping: &mut serde_yaml_ng::Mapping, name: &str, value: &str) {
    mapping.insert(key(name), serde_yaml_ng::Value::String(value.to_string()));
}

fn insert_number(mapping: &mut serde_yaml_ng::Mapping, name: &str, value: f64) -> Result<()> {
    mapping.insert(
        key(name),
        serde_yaml_ng::to_value(value).context("Failed to encode source stimulus number.")?,
    );
    Ok(())
}

fn key(name: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::Value::String(name.to_string())
}

fn validated_id<'a>(value: &'a str, label: &str) -> Result<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        anyhow::bail!("{label} must not be blank.");
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
    {
        anyhow::bail!("{label} {value} contains unsupported characters.");
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::{
        AnalogStimulusDraft, AnalogStimulusKind, AnalogStimulusPulseDraft, analog_stimulus_choices,
        replace_analog_stimulus,
    };

    fn project_yaml() -> &'static str {
        "project:
  name: gui_stimulus_test
  version: 0.1.0
board:
  components:
    V1:
      model: generic.analog.dc_voltage_source
      spice:
        primitive: dc_voltage_source
        dc_v: 5.0
      pins: {P: rail_5v, N: gnd}
    ILOAD:
      model: generic.analog.pulse_current_source
      spice:
        primitive: pulse_current_source
        current_pulse:
          initial_a: 0.0
          pulsed_a: 0.1
          delay_us: 1.0
          rise_us: 0.5
          fall_us: 0.5
          width_us: 10.0
          period_us: 20.0
      pins: {P: rail_5v, N: gnd}
    R1:
      model: generic.analog.resistor
      spice:
        primitive: resistor
        value_ohm: 1000.0
      pins: {A: rail_5v, B: gnd}
  nets:
    rail_5v: {kind: power}
    gnd: {kind: ground}
scenarios:
  - name: generated_transient
    type: analog_transient
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated:
        components: [V1, ILOAD, R1]
        ground_net: gnd
      model_files: []
      node_bindings:
        - {node: '0', net: gnd}
        - {node: rail_5v, net: rail_5v}
      pin_bindings:
        - {node: rail_5v, endpoint: {component: V1, pin: P}}
        - {node: '0', endpoint: {component: V1, pin: N}}
        - {node: rail_5v, endpoint: {component: ILOAD, pin: P}}
        - {node: '0', endpoint: {component: ILOAD, pin: N}}
      analysis: {type: tran, stop_time_us: 100.0, max_step_us: 1.0}
      stimuli: []
      probes: []
      assertions: []
"
    }

    #[test]
    fn analog_stimulus_choices_lists_generated_source_primitives() {
        let choices = analog_stimulus_choices(project_yaml()).unwrap();
        assert_eq!(choices.len(), 2);
        assert_eq!(choices[0].component_id, "ILOAD");
        assert_eq!(choices[0].kind, AnalogStimulusKind::PulseCurrent);
        assert_eq!(choices[1].component_id, "V1");
        assert_eq!(choices[1].kind, AnalogStimulusKind::DcVoltage);
        assert_eq!(choices[1].dc_value, 5.0);
    }

    #[test]
    fn replace_analog_stimulus_updates_dc_voltage() {
        let edited = replace_analog_stimulus(
            project_yaml(),
            &AnalogStimulusDraft {
                scenario_name: "generated_transient".to_string(),
                component_id: "V1".to_string(),
                kind: AnalogStimulusKind::DcVoltage,
                dc_value: 3.3,
                pulse: AnalogStimulusPulseDraft::default_for(AnalogStimulusKind::DcVoltage),
            },
        )
        .unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let spice = project.board.components["V1"].spice.as_ref().unwrap();
        assert_eq!(spice.dc_v, Some(3.3));
        assert!(spice.dc_a.is_none());
        assert!(spice.pulse.is_none());
    }

    #[test]
    fn replace_analog_stimulus_updates_current_pulse() {
        let edited = replace_analog_stimulus(
            project_yaml(),
            &AnalogStimulusDraft {
                scenario_name: "generated_transient".to_string(),
                component_id: "ILOAD".to_string(),
                kind: AnalogStimulusKind::PulseCurrent,
                dc_value: 0.0,
                pulse: AnalogStimulusPulseDraft {
                    initial: 0.01,
                    pulsed: 0.25,
                    delay_us: 2.0,
                    rise_us: 0.2,
                    fall_us: 0.3,
                    width_us: 5.0,
                    period_us: 12.0,
                },
            },
        )
        .unwrap();
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
        let pulse = project.board.components["ILOAD"]
            .spice
            .as_ref()
            .unwrap()
            .current_pulse
            .as_ref()
            .unwrap();
        assert_eq!(pulse.initial_a, 0.01);
        assert_eq!(pulse.pulsed_a, 0.25);
        assert_eq!(pulse.width_us, 5.0);
    }

    #[test]
    fn replace_analog_stimulus_rejects_stale_kind() {
        let error = replace_analog_stimulus(
            project_yaml(),
            &AnalogStimulusDraft {
                scenario_name: "generated_transient".to_string(),
                component_id: "V1".to_string(),
                kind: AnalogStimulusKind::DcCurrent,
                dc_value: 0.1,
                pulse: AnalogStimulusPulseDraft::default_for(AnalogStimulusKind::DcCurrent),
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("editor was loaded"));
    }
}
