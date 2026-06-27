use super::analog::{
    AnalogAssertionDraft, AnalogProbeDraft, AnalogScenarioDraft, append_analog_assertion,
    append_analog_transient_scenario_with_project_path, append_analog_voltage_probe,
};
use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Clone)]
pub(super) struct ObservationPresetResult {
    pub(super) project_yaml: String,
    pub(super) scenario_name: String,
    pub(super) probe_count: usize,
}

pub(super) fn create_component_observation_preset(
    text: &str,
    project_path: &Path,
    component_id: &str,
) -> Result<ObservationPresetResult> {
    let component_id = component_id.trim();
    if component_id.is_empty() {
        anyhow::bail!("Select a component before creating an observation preset.");
    }
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let component = project
        .board
        .components
        .get(component_id)
        .with_context(|| format!("Component {component_id} was not found."))?;
    let (library, findings) = crate::library::load_library(project_path, &project);
    let hard_failures: Vec<_> = findings
        .iter()
        .filter(|finding| finding.id == "LIBRARY_NOT_FOUND" || finding.id == "MODEL_LOAD_FAILED")
        .map(|finding| finding.message.clone())
        .collect();
    if !hard_failures.is_empty() {
        anyhow::bail!("{}", hard_failures.join("; "));
    }
    let model = library
        .get(&component.model)
        .with_context(|| format!("Model {} was not found.", component.model))?;
    if model.simulation.spice.is_none() {
        anyhow::bail!(
            "Component {component_id} model {} has no generated-SPICE simulation face.",
            component.model
        );
    }
    let ground_net = observation_ground_net(&project, component, model)?;
    let probe_specs = observation_voltage_probe_specs(component_id, component, model, &ground_net);
    let first = probe_specs.first().with_context(|| {
        format!("Component {component_id} has no non-ground pin nets to probe.")
    })?;
    let scenario_name = unique_observation_scenario_name(&project, component_id);
    let mut updated = append_analog_transient_scenario_with_project_path(
        text,
        project_path,
        &AnalogScenarioDraft {
            name: scenario_name.clone(),
            ground_net,
            probe_net: first.net.clone(),
            probe_name: first.probe_name.clone(),
            stop_time_us: 100.0,
            max_step_us: 0.2,
        },
    )?;
    for probe in probe_specs.iter().skip(1) {
        updated = append_analog_voltage_probe(
            &updated,
            &AnalogProbeDraft {
                scenario_name: scenario_name.clone(),
                net_id: probe.net.clone(),
                probe_name: probe.probe_name.clone(),
            },
        )?;
    }
    for assertion in observation_default_assertions(
        &project,
        component,
        model,
        &probe_specs,
        &scenario_name,
        100.0,
    ) {
        updated = append_analog_assertion(&updated, &assertion)?;
    }
    Ok(ObservationPresetResult {
        project_yaml: updated,
        scenario_name,
        probe_count: probe_specs.len(),
    })
}

#[derive(Debug, Clone)]
struct ObservationProbeSpec {
    pin: String,
    probe_name: String,
    net: String,
}

fn observation_ground_net(
    project: &crate::board_ir::BoardProject,
    component: &crate::board_ir::ComponentSpec,
    model: &crate::library::ComponentModel,
) -> Result<String> {
    for (pin_id, port) in &model.ports {
        if port.kind == crate::library::PortKind::ElectricalGround
            && let Some(net) = component.pins.get(pin_id)
        {
            return Ok(net.clone());
        }
    }
    project
        .board
        .nets
        .iter()
        .find(|(_, net)| net.kind == crate::board_ir::NetKind::Ground)
        .map(|(net_id, _)| net_id.clone())
        .context("Observation presets require a ground net or an electrical-ground component pin.")
}

fn observation_voltage_probe_specs(
    component_id: &str,
    component: &crate::board_ir::ComponentSpec,
    model: &crate::library::ComponentModel,
    ground_net: &str,
) -> Vec<ObservationProbeSpec> {
    let mut probes = Vec::new();
    let mut seen_nets = BTreeSet::new();
    let pin_order = model
        .simulation
        .spice
        .as_ref()
        .map(|spice| spice.pin_order.as_slice())
        .unwrap_or(&[]);
    let mut pins = pin_order
        .iter()
        .filter(|pin| component.pins.contains_key(pin.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    for pin in component.pins.keys() {
        if !pins.iter().any(|ordered| ordered == pin) {
            pins.push(pin.clone());
        }
    }
    for pin in pins {
        let Some(net) = component.pins.get(&pin) else {
            continue;
        };
        if net == ground_net {
            continue;
        }
        if let Some(port) = model.ports.get(&pin)
            && port.kind == crate::library::PortKind::ElectricalGround
        {
            continue;
        }
        if !seen_nets.insert(net.clone()) {
            continue;
        }
        probes.push(ObservationProbeSpec {
            pin: pin.clone(),
            probe_name: sanitize_observation_id(&format!("v_{component_id}_{pin}")),
            net: net.clone(),
        });
    }
    probes
}

fn observation_default_assertions(
    project: &crate::board_ir::BoardProject,
    component: &crate::board_ir::ComponentSpec,
    model: &crate::library::ComponentModel,
    probes: &[ObservationProbeSpec],
    scenario_name: &str,
    stop_time_us: f64,
) -> Vec<AnalogAssertionDraft> {
    let mut assertions = Vec::new();
    if let Some(power) = &model.power_conversion
        && let Some(output_probe) = probe_for_pin(probes, &power.output_pin)
        && let Some(output_port) = model.ports.get(&power.output_pin)
    {
        if let Some(min_v) = output_port.electrical.operating_voltage_min_v {
            assertions.push(default_voltage_assertion(
                scenario_name,
                &format!("{}_min_voltage", output_probe.probe_name),
                &output_probe.probe_name,
                "min",
                "above",
                min_v,
                (0.0, stop_time_us),
            ));
        }
        if let Some(max_v) = output_port.electrical.operating_voltage_max_v {
            assertions.push(default_voltage_assertion(
                scenario_name,
                &format!("{}_max_voltage", output_probe.probe_name),
                &output_probe.probe_name,
                "max",
                "below",
                max_v,
                (0.0, stop_time_us),
            ));
        }
    }
    if let Some(reset) = &model.reset_supervisor
        && let Some(output_probe) = probe_for_pin(probes, &reset.reset_output_pin)
        && let Some(monitored_net) = component.pins.get(&reset.monitored_pin)
        && let Some(pulse) = pulse_source_for_net(project, monitored_net)
    {
        let low_time = (pulse.delay_us * 0.5).clamp(0.0, stop_time_us);
        let high_time =
            (pulse.delay_us + pulse.rise_us + stop_time_us * 0.1).clamp(0.0, stop_time_us);
        let low_threshold = (pulse.pulsed_v.abs() * 0.1).max(0.1);
        let high_threshold = pulse.pulsed_v * 0.9;
        match reset.active {
            crate::library::ResetSupervisorActive::Low => {
                assertions.push(default_sample_assertion(
                    scenario_name,
                    &format!("{}_asserted_low", output_probe.probe_name),
                    &output_probe.probe_name,
                    "below",
                    low_threshold,
                    low_time,
                ));
                assertions.push(default_sample_assertion(
                    scenario_name,
                    &format!("{}_released_high", output_probe.probe_name),
                    &output_probe.probe_name,
                    "above",
                    high_threshold,
                    high_time,
                ));
            }
            crate::library::ResetSupervisorActive::High => {
                assertions.push(default_sample_assertion(
                    scenario_name,
                    &format!("{}_asserted_high", output_probe.probe_name),
                    &output_probe.probe_name,
                    "above",
                    high_threshold,
                    low_time,
                ));
                assertions.push(default_sample_assertion(
                    scenario_name,
                    &format!("{}_released_low", output_probe.probe_name),
                    &output_probe.probe_name,
                    "below",
                    low_threshold,
                    high_time,
                ));
            }
        }
    }
    assertions
}

fn probe_for_pin<'a>(
    probes: &'a [ObservationProbeSpec],
    pin: &str,
) -> Option<&'a ObservationProbeSpec> {
    probes.iter().find(|probe| probe.pin == pin)
}

fn pulse_source_for_net<'a>(
    project: &'a crate::board_ir::BoardProject,
    net_id: &str,
) -> Option<&'a crate::board_ir::SpicePulseSpec> {
    project.board.components.values().find_map(|component| {
        let spice = component.spice.as_ref()?;
        if spice.primitive != crate::board_ir::SpicePrimitive::PulseVoltageSource {
            return None;
        }
        if component.pins.get("P").is_some_and(|net| net == net_id) {
            spice.pulse.as_ref()
        } else {
            None
        }
    })
}

fn default_voltage_assertion(
    scenario_name: &str,
    assertion_name: &str,
    probe_name: &str,
    aggregation: &str,
    relation: &str,
    threshold: f64,
    window_us: (f64, f64),
) -> AnalogAssertionDraft {
    let (start_us, end_us) = window_us;
    AnalogAssertionDraft {
        scenario_name: scenario_name.to_string(),
        assertion_name: sanitize_observation_id(assertion_name),
        probe_name: probe_name.to_string(),
        reference_probe: String::new(),
        aggregation: aggregation.to_string(),
        relation: relation.to_string(),
        threshold,
        reference_threshold: 0.0,
        target: 0.0,
        tolerance: 0.0,
        at_us: 0.0,
        start_us,
        end_us,
        time_limit_us: 0.0,
        duty_limit_percent: 0.0,
        count_limit: 0.0,
        overshoot_limit_percent: 0.0,
    }
}

fn default_sample_assertion(
    scenario_name: &str,
    assertion_name: &str,
    probe_name: &str,
    relation: &str,
    threshold: f64,
    at_us: f64,
) -> AnalogAssertionDraft {
    AnalogAssertionDraft {
        scenario_name: scenario_name.to_string(),
        assertion_name: sanitize_observation_id(assertion_name),
        probe_name: probe_name.to_string(),
        reference_probe: String::new(),
        aggregation: "sample".to_string(),
        relation: relation.to_string(),
        threshold,
        reference_threshold: 0.0,
        target: 0.0,
        tolerance: 0.0,
        at_us,
        start_us: 0.0,
        end_us: 0.0,
        time_limit_us: 0.0,
        duty_limit_percent: 0.0,
        count_limit: 0.0,
        overshoot_limit_percent: 0.0,
    }
}

fn unique_observation_scenario_name(
    project: &crate::board_ir::BoardProject,
    component_id: &str,
) -> String {
    let base = sanitize_observation_id(&format!("{component_id}_observation"));
    let existing = project
        .scenarios
        .iter()
        .map(|scenario| scenario.name.as_str())
        .collect::<BTreeSet<_>>();
    if !existing.contains(base.as_str()) {
        return base;
    }
    for suffix in 2.. {
        let candidate = format!("{base}_{suffix}");
        if !existing.contains(candidate.as_str()) {
            return candidate;
        }
    }
    unreachable!("unbounded suffix search must return")
}

fn sanitize_observation_id(value: &str) -> String {
    let mut out = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            out.push(character.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let out = out.trim_matches('_');
    if out.is_empty() {
        "observation".to_string()
    } else if out
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        format!("obs_{out}")
    } else {
        out.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::create_component_observation_preset;
    use std::path::Path;

    fn ap2112_project_yaml() -> &'static str {
        "project:
  name: ap2112_observation_preset_test
  version: 0.1.0
libraries:
  - libs/generic/analog
  - libs/vendor/diodes/regulators
board:
  components:
    VUSB:
      model: generic.analog.dc_voltage_source
      pins: { P: usb_5v, N: gnd }
      spice: { primitive: dc_voltage_source, dc_v: 5.0 }
    VEN:
      model: generic.analog.dc_voltage_source
      pins: { P: regulator_en, N: gnd }
      spice: { primitive: dc_voltage_source, dc_v: 5.0 }
    UREG:
      model: vendor.diodes.ap2112k_3v3
      pins: { VIN: usb_5v, EN: regulator_en, GND: gnd, VOUT: rail_3v3 }
    RLOAD:
      model: generic.analog.resistor
      pins: { A: rail_3v3, B: gnd }
      spice: { primitive: resistor, value_ohm: 3300.0 }
  nets:
    usb_5v: { kind: power, nominal_voltage: 5.0, powered: true }
    regulator_en: { kind: power }
    rail_3v3: { kind: power, nominal_voltage: 3.3, powered: true }
    gnd: { kind: ground }
"
    }

    fn tlv803_project_yaml() -> &'static str {
        "project:
  name: tlv803_observation_preset_test
  version: 0.1.0
libraries:
  - libs/generic/analog
  - libs/vendor/ti/reset_supervisors
board:
  components:
    VDD:
      model: generic.analog.pulse_voltage_source
      pins: { P: rail_3v3, N: gnd }
      spice:
        primitive: pulse_voltage_source
        pulse:
          initial_v: 0.0
          pulsed_v: 3.3
          delay_us: 10.0
          rise_us: 1.0
          fall_us: 1.0
          width_us: 80.0
          period_us: 160.0
    URESET:
      model: vendor.ti.tlv803ea29
      pins: { VDD: rail_3v3, GND: gnd, RESET: reset_n }
    RPU:
      model: generic.analog.resistor
      pins: { A: rail_3v3, B: reset_n }
      spice: { primitive: resistor, value_ohm: 10000.0 }
  nets:
    rail_3v3: { kind: power, nominal_voltage: 3.3, powered: true }
    reset_n: { kind: digital_or_analog }
    gnd: { kind: ground }
"
    }

    #[test]
    fn observation_preset_creates_generated_spice_setup_for_component_pins() {
        let result =
            create_component_observation_preset(ap2112_project_yaml(), Path::new("."), "UREG")
                .unwrap();
        let project: crate::board_ir::BoardProject =
            serde_yaml_ng::from_str(&result.project_yaml).unwrap();
        let scenario = project
            .scenarios
            .iter()
            .find(|scenario| scenario.name == "ureg_observation")
            .unwrap();
        let analog = scenario.analog.as_ref().unwrap();
        let generated = analog.generated.as_ref().unwrap();

        assert_eq!(result.probe_count, 3);
        assert_eq!(generated.ground_net, "gnd");
        assert!(
            generated
                .components
                .iter()
                .any(|component| component == "UREG")
        );
        assert_eq!(
            analog
                .probes
                .iter()
                .map(|probe| probe.name.as_str())
                .collect::<Vec<_>>(),
            vec!["v_ureg_vin", "v_ureg_en", "v_ureg_vout"]
        );
        assert_eq!(analog.model_files.len(), 1);
        assert_eq!(
            analog.model_files[0].path,
            "models/spice/generic/analog_behavioral.lib"
        );
        assert!(analog.model_files[0].sha256.is_some());
        assert_eq!(
            analog
                .assertions
                .iter()
                .map(|assertion| assertion.name.as_str())
                .collect::<Vec<_>>(),
            vec!["v_ureg_vout_min_voltage", "v_ureg_vout_max_voltage"]
        );
    }

    #[test]
    fn observation_preset_adds_reset_supervisor_threshold_checks_for_pulsed_rail() {
        let result =
            create_component_observation_preset(tlv803_project_yaml(), Path::new("."), "URESET")
                .unwrap();
        let project: crate::board_ir::BoardProject =
            serde_yaml_ng::from_str(&result.project_yaml).unwrap();
        let scenario = project
            .scenarios
            .iter()
            .find(|scenario| scenario.name == "ureset_observation")
            .unwrap();
        let analog = scenario.analog.as_ref().unwrap();

        assert_eq!(
            analog
                .probes
                .iter()
                .map(|probe| probe.name.as_str())
                .collect::<Vec<_>>(),
            vec!["v_ureset_vdd", "v_ureset_reset"]
        );
        assert_eq!(
            analog
                .assertions
                .iter()
                .map(|assertion| {
                    (
                        assertion.name.as_str(),
                        assertion.probe.as_str(),
                        assertion.relation.clone(),
                        assertion.at_us,
                        assertion.threshold_v,
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                (
                    "v_ureset_reset_asserted_low",
                    "v_ureset_reset",
                    crate::board_ir::AnalogRelation::Below,
                    Some(5.0),
                    Some(0.33)
                ),
                (
                    "v_ureset_reset_released_high",
                    "v_ureset_reset",
                    crate::board_ir::AnalogRelation::Above,
                    Some(21.0),
                    Some(2.9699999999999998)
                )
            ]
        );
    }
}
