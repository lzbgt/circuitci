use crate::board_ir::{ComponentSpec, Scenario};
use crate::library::{BoundBoard, ComponentModel, PortKind};
use crate::reports::Finding;
use serde_json::json;

use super::resolve_power_net;
use crate::validation::POWER_TREE_VALID;

pub(super) fn validate_reset_supervisor(
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(supervisor) = &model.reset_supervisor else {
        return;
    };
    if !validate_reset_supervisor_metadata(component_id, model, scenario, findings) {
        return;
    }
    let Some(monitored_net_name) = resolve_power_net(component, &supervisor.monitored_pin) else {
        reset_supervisor_pin_finding(
            component_id,
            &supervisor.monitored_pin,
            "monitored",
            scenario,
            findings,
        );
        return;
    };
    let Some(output_net_name) = component
        .pins
        .get(&supervisor.reset_output_pin)
        .map(String::as_str)
    else {
        reset_supervisor_pin_finding(
            component_id,
            &supervisor.reset_output_pin,
            "reset output",
            scenario,
            findings,
        );
        return;
    };
    let Some(monitored_net) = bound.project.board.nets.get(monitored_net_name) else {
        return;
    };
    let Some(nominal_v) = monitored_net.nominal_voltage else {
        reset_supervisor_missing_voltage_finding(
            component_id,
            monitored_net_name,
            scenario,
            findings,
        );
        return;
    };
    if nominal_v.is_finite() && nominal_v <= supervisor.threshold_max_v {
        let mut finding = Finding::critical(
            POWER_TREE_VALID,
            &scenario.name,
            format!(
                "Reset supervisor {component_id} monitored rail {monitored_net_name} nominal voltage {:.6} V is not above worst-case release threshold {:.6} V.",
                nominal_v, supervisor.threshold_max_v
            ),
        );
        finding.component = Some(component_id.to_string());
        finding.net = Some(monitored_net_name.to_string());
        finding
            .measured
            .insert("monitored_nominal_voltage_V".to_string(), json!(nominal_v));
        finding.limit.insert(
            "reset_supervisor_threshold_max_V".to_string(),
            json!(supervisor.threshold_max_v),
        );
        finding.suggested_fixes = vec![
            "Select a reset supervisor threshold option below the monitored rail nominal voltage with tolerance margin.".to_string(),
            "Correct the monitored rail nominal_voltage if this scenario uses a different powered state.".to_string(),
        ];
        findings.push(finding);
    }

    let required_min_v = monitored_rail_required_min_voltage(
        bound,
        component_id,
        &supervisor.monitored_pin,
        monitored_net_name,
    );
    if let Some((load_component, load_pin, required_min_v)) = required_min_v
        && supervisor.threshold_min_v < required_min_v
    {
        let mut finding = Finding::critical(
            POWER_TREE_VALID,
            &scenario.name,
            format!(
                "Reset supervisor {component_id} can release {output_net_name} at {:.6} V, below {}.{} minimum operating voltage {:.6} V.",
                supervisor.threshold_min_v, load_component, load_pin, required_min_v
            ),
        );
        finding.component = Some(component_id.to_string());
        finding.net = Some(output_net_name.to_string());
        finding.measured.insert(
            "reset_supervisor_threshold_min_V".to_string(),
            json!(supervisor.threshold_min_v),
        );
        finding.measured.insert(
            "monitored_load_component".to_string(),
            json!(load_component),
        );
        finding
            .measured
            .insert("monitored_load_pin".to_string(), json!(load_pin));
        finding.limit.insert(
            "load_operating_voltage_min_V".to_string(),
            json!(required_min_v),
        );
        finding.suggested_fixes = vec![
            "Use a reset supervisor threshold option whose minimum release threshold is at or above the monitored device minimum operating voltage.".to_string(),
            "Hold reset longer with a supervisor delay only after voltage threshold margin is correct; delay alone cannot fix too-low release voltage.".to_string(),
            "Move the supervised load to a rail whose operating range matches the supervisor threshold.".to_string(),
        ];
        findings.push(finding);
    }
}

fn validate_reset_supervisor_metadata(
    component_id: &str,
    model: &ComponentModel,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> bool {
    let Some(supervisor) = &model.reset_supervisor else {
        return true;
    };
    let mut valid = true;
    if supervisor.monitored_pin == supervisor.reset_output_pin {
        reset_supervisor_metadata_finding(
            component_id,
            "monitored_pin",
            "reset_supervisor monitored_pin and reset_output_pin must be distinct.",
            scenario,
            findings,
        );
        valid = false;
    }
    match model.ports.get(&supervisor.monitored_pin) {
        Some(port) if port.kind == PortKind::ElectricalPower => {}
        Some(_) => {
            reset_supervisor_metadata_finding(
                component_id,
                "monitored_pin",
                &format!(
                    "reset_supervisor monitored_pin {} is not an electrical_power port.",
                    supervisor.monitored_pin
                ),
                scenario,
                findings,
            );
            valid = false;
        }
        None => {
            reset_supervisor_metadata_finding(
                component_id,
                "monitored_pin",
                &format!(
                    "reset_supervisor monitored_pin {} is not declared in model ports.",
                    supervisor.monitored_pin
                ),
                scenario,
                findings,
            );
            valid = false;
        }
    }
    match model.ports.get(&supervisor.reset_output_pin) {
        Some(port)
            if matches!(
                port.kind,
                PortKind::DigitalElectricalOutput | PortKind::DigitalElectricalIo
            ) => {}
        Some(_) => {
            reset_supervisor_metadata_finding(
                component_id,
                "reset_output_pin",
                &format!(
                    "reset_supervisor reset_output_pin {} is not a digital output or IO port.",
                    supervisor.reset_output_pin
                ),
                scenario,
                findings,
            );
            valid = false;
        }
        None => {
            reset_supervisor_metadata_finding(
                component_id,
                "reset_output_pin",
                &format!(
                    "reset_supervisor reset_output_pin {} is not declared in model ports.",
                    supervisor.reset_output_pin
                ),
                scenario,
                findings,
            );
            valid = false;
        }
    }
    if !supervisor.threshold_min_v.is_finite() || supervisor.threshold_min_v <= 0.0 {
        reset_supervisor_metadata_finding(
            component_id,
            "threshold_min_V",
            "reset_supervisor threshold_min_V must be finite and positive.",
            scenario,
            findings,
        );
        valid = false;
    }
    if !supervisor.threshold_max_v.is_finite() || supervisor.threshold_max_v <= 0.0 {
        reset_supervisor_metadata_finding(
            component_id,
            "threshold_max_V",
            "reset_supervisor threshold_max_V must be finite and positive.",
            scenario,
            findings,
        );
        valid = false;
    }
    if supervisor.threshold_min_v > supervisor.threshold_max_v {
        reset_supervisor_metadata_finding(
            component_id,
            "threshold_min_V",
            "reset_supervisor threshold_min_V must not exceed threshold_max_V.",
            scenario,
            findings,
        );
        valid = false;
    }
    if let Some(delay_us) = supervisor.reset_release_delay_us
        && (!delay_us.is_finite() || delay_us < 0.0)
    {
        reset_supervisor_metadata_finding(
            component_id,
            "reset_release_delay_us",
            "reset_supervisor reset_release_delay_us must be finite and non-negative.",
            scenario,
            findings,
        );
        valid = false;
    }
    valid
}

fn monitored_rail_required_min_voltage(
    bound: &BoundBoard<'_>,
    supervisor_component_id: &str,
    supervisor_monitored_pin: &str,
    monitored_net_name: &str,
) -> Option<(String, String, f64)> {
    bound
        .project
        .board
        .components
        .iter()
        .flat_map(|(component_id, component)| {
            let Some(model) = bound.library.get(&component.model) else {
                return Vec::new();
            };
            model
                .ports
                .iter()
                .filter_map(move |(pin_name, port)| {
                    if port.kind != PortKind::ElectricalPower {
                        return None;
                    }
                    if component_id == supervisor_component_id
                        && pin_name == supervisor_monitored_pin
                    {
                        return None;
                    }
                    let net_name = resolve_power_net(component, pin_name)?;
                    if net_name != monitored_net_name {
                        return None;
                    }
                    let min_v = port.electrical.operating_voltage_min_v?;
                    if min_v.is_finite() && min_v > 0.0 {
                        Some((component_id.clone(), pin_name.clone(), min_v))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        })
        .max_by(|left, right| left.2.total_cmp(&right.2))
}

fn reset_supervisor_metadata_finding(
    component_id: &str,
    field: &str,
    message: &str,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(POWER_TREE_VALID, &scenario.name, message.to_string());
    finding.component = Some(component_id.to_string());
    finding
        .limit
        .insert("reset_supervisor_field".to_string(), json!(field));
    finding.suggested_fixes = vec![
        "Correct the component model reset_supervisor metadata before using it for power-tree validation.".to_string(),
        "Use analog_transient or reset timing scenarios when reset behavior depends on waveform shape or open-drain pull-up dynamics.".to_string(),
    ];
    findings.push(finding);
}

fn reset_supervisor_pin_finding(
    component_id: &str,
    pin: &str,
    role: &str,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        POWER_TREE_VALID,
        &scenario.name,
        format!(
            "Reset supervisor {component_id} reset_supervisor {role}_pin {pin} is not connected."
        ),
    );
    finding.component = Some(component_id.to_string());
    finding.limit.insert(format!("{role}_pin"), json!(pin));
    finding.suggested_fixes = vec![
        "Connect every declared reset_supervisor monitored and reset output pin to explicit nets."
            .to_string(),
        "Correct the component model reset_supervisor pin names if they do not match the model ports."
            .to_string(),
    ];
    findings.push(finding);
}

fn reset_supervisor_missing_voltage_finding(
    component_id: &str,
    monitored_net_name: &str,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        POWER_TREE_VALID,
        &scenario.name,
        format!(
            "Reset supervisor {component_id} monitored rail {monitored_net_name} is missing nominal_voltage."
        ),
    );
    finding.component = Some(component_id.to_string());
    finding.net = Some(monitored_net_name.to_string());
    finding
        .limit
        .insert("required_nominal_voltage".to_string(), json!(true));
    finding.suggested_fixes = vec![
        "Declare nominal_voltage for every rail monitored by a reset supervisor.".to_string(),
        "Use analog_transient if reset release must be derived from a waveform threshold crossing."
            .to_string(),
    ];
    findings.push(finding);
}
