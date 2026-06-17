use crate::board_ir::{Scenario, SpicePrimitive};
use crate::library::BoundBoard;
use crate::reports::Finding;
use crate::validation::BUS_TERMINATION_VALID;
use serde_json::json;

use super::super::common::validation_input_missing;
use super::required_scenario_numeric_parameter;

pub(super) fn validate_bus_termination(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(line_a_net) = required_string_parameter(scenario, "line_a_net", findings) else {
        return;
    };
    let Some(line_b_net) = required_string_parameter(scenario, "line_b_net", findings) else {
        return;
    };
    let Some(board_is_endpoint) =
        required_bool_parameter(scenario, "board_is_bus_endpoint", findings)
    else {
        return;
    };
    let Some(expected_ohm) =
        required_positive_parameter(scenario, "expected_termination_ohm", findings)
    else {
        return;
    };
    let Some(tolerance_percent) =
        required_nonnegative_parameter(scenario, "termination_tolerance_percent", findings)
    else {
        return;
    };

    if !bound.project.board.nets.contains_key(line_a_net) {
        findings.push(input_finding(
            scenario,
            format!("Bus termination line_a_net {line_a_net} is not declared."),
        ));
        return;
    }
    if !bound.project.board.nets.contains_key(line_b_net) {
        findings.push(input_finding(
            scenario,
            format!("Bus termination line_b_net {line_b_net} is not declared."),
        ));
        return;
    }
    if line_a_net == line_b_net {
        findings.push(input_finding(
            scenario,
            "Bus termination line_a_net and line_b_net must be different.".to_string(),
        ));
        return;
    }

    let termination_component =
        scenario
            .parameters
            .get("termination_component")
            .and_then(|value| {
                let value = value.as_str()?;
                let trimmed = value.trim();
                (!trimmed.is_empty()).then_some(trimmed)
            });

    if !board_is_endpoint {
        if let Some(component_id) = termination_component
            && let Some(evidence) = termination_resistor_evidence(
                bound,
                TerminationContext {
                    scenario,
                    component_id,
                    line_a_net,
                    line_b_net,
                    expected_ohm,
                    tolerance_percent,
                },
                findings,
            )
        {
            findings.push(non_endpoint_termination_finding(
                scenario,
                component_id,
                line_a_net,
                line_b_net,
                evidence.value_ohm,
            ));
        }
        return;
    }

    let Some(component_id) = termination_component else {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection parameters.termination_component is required when board_is_bus_endpoint is true.",
        );
        return;
    };
    let Some(evidence) = termination_resistor_evidence(
        bound,
        TerminationContext {
            scenario,
            component_id,
            line_a_net,
            line_b_net,
            expected_ohm,
            tolerance_percent,
        },
        findings,
    ) else {
        return;
    };

    let allowed_error_ohm = expected_ohm * tolerance_percent / 100.0;
    let error_ohm = (evidence.value_ohm - expected_ohm).abs();
    if error_ohm > allowed_error_ohm {
        findings.push(termination_value_finding(
            scenario,
            component_id,
            line_a_net,
            line_b_net,
            evidence.value_ohm,
            expected_ohm,
            tolerance_percent,
        ));
    }
}

struct TerminationEvidence {
    value_ohm: f64,
}

#[derive(Clone, Copy)]
struct TerminationContext<'a> {
    scenario: &'a Scenario,
    component_id: &'a str,
    line_a_net: &'a str,
    line_b_net: &'a str,
    expected_ohm: f64,
    tolerance_percent: f64,
}

fn termination_resistor_evidence(
    bound: &BoundBoard<'_>,
    context: TerminationContext<'_>,
    findings: &mut Vec<Finding>,
) -> Option<TerminationEvidence> {
    let component_id = context.component_id;
    let line_a_net = context.line_a_net;
    let line_b_net = context.line_b_net;
    let Some(component) = bound.project.board.components.get(component_id) else {
        findings.push(termination_input_finding(
            context.scenario,
            component_id,
            line_a_net,
            line_b_net,
            context.expected_ohm,
            context.tolerance_percent,
            format!("Bus termination component {component_id} is not declared."),
        ));
        return None;
    };
    let Some(spice) = &component.spice else {
        findings.push(termination_input_finding(
            context.scenario,
            component_id,
            line_a_net,
            line_b_net,
            context.expected_ohm,
            context.tolerance_percent,
            format!("Bus termination component {component_id} has no spice resistor evidence."),
        ));
        return None;
    };
    if spice.primitive != SpicePrimitive::Resistor {
        findings.push(termination_input_finding(
            context.scenario,
            component_id,
            line_a_net,
            line_b_net,
            context.expected_ohm,
            context.tolerance_percent,
            format!("Bus termination component {component_id} must be a resistor."),
        ));
        return None;
    }
    let Some(value_ohm) = spice.value_ohm else {
        findings.push(termination_input_finding(
            context.scenario,
            component_id,
            line_a_net,
            line_b_net,
            context.expected_ohm,
            context.tolerance_percent,
            format!("Bus termination component {component_id} is missing spice.value_ohm."),
        ));
        return None;
    };
    if !value_ohm.is_finite() || value_ohm <= 0.0 {
        findings.push(termination_input_finding(
            context.scenario,
            component_id,
            line_a_net,
            line_b_net,
            context.expected_ohm,
            context.tolerance_percent,
            format!("Bus termination component {component_id} spice.value_ohm must be positive."),
        ));
        return None;
    }

    let connects_line_a = component.pins.values().any(|net| net == line_a_net);
    let connects_line_b = component.pins.values().any(|net| net == line_b_net);
    if !(connects_line_a && connects_line_b) {
        findings.push(termination_input_finding(
            context.scenario,
            component_id,
            line_a_net,
            line_b_net,
            context.expected_ohm,
            context.tolerance_percent,
            format!(
                "Bus termination component {component_id} must connect directly across {line_a_net} and {line_b_net}."
            ),
        ));
        return None;
    }

    Some(TerminationEvidence { value_ohm })
}

fn required_string_parameter<'a>(
    scenario: &'a Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<&'a str> {
    let Some(raw) = scenario.parameters.get(name) else {
        validation_input_missing(
            findings,
            scenario,
            format!("interface_protection parameters.{name} is required."),
        );
        return None;
    };
    let Some(value) = raw.as_str() else {
        validation_input_missing(
            findings,
            scenario,
            format!("interface_protection parameters.{name} must be a string."),
        );
        return None;
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            format!("interface_protection parameters.{name} must not be blank."),
        );
        return None;
    }
    Some(trimmed)
}

fn required_bool_parameter(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<bool> {
    let Some(raw) = scenario.parameters.get(name) else {
        validation_input_missing(
            findings,
            scenario,
            format!("interface_protection parameters.{name} is required."),
        );
        return None;
    };
    let Some(value) = raw.as_bool() else {
        validation_input_missing(
            findings,
            scenario,
            format!("interface_protection parameters.{name} must be a boolean."),
        );
        return None;
    };
    Some(value)
}

fn required_positive_parameter(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    let value = required_scenario_numeric_parameter(scenario, name, findings)?;
    if value <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            format!("interface_protection parameters.{name} must be greater than zero."),
        );
        return None;
    }
    Some(value)
}

fn required_nonnegative_parameter(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    required_scenario_numeric_parameter(scenario, name, findings)
}

fn input_finding(scenario: &Scenario, message: String) -> Finding {
    let mut finding = Finding::critical(BUS_TERMINATION_VALID, &scenario.name, message);
    finding.suggested_fixes = vec![
        "Declare the bus endpoint role, the two bus nets, expected termination resistance, tolerance, and the actual termination resistor evidence.".to_string(),
        "Do not use BUS_TERMINATION_VALID until the board topology and resistor population option are explicit.".to_string(),
    ];
    finding
}

fn termination_input_finding(
    scenario: &Scenario,
    component_id: &str,
    line_a_net: &str,
    line_b_net: &str,
    expected_ohm: f64,
    tolerance_percent: f64,
    message: String,
) -> Finding {
    let mut finding = input_finding(scenario, message);
    finding.component = Some(component_id.to_string());
    finding
        .measured
        .insert("line_a_net".to_string(), json!(line_a_net));
    finding
        .measured
        .insert("line_b_net".to_string(), json!(line_b_net));
    finding
        .limit
        .insert("expected_termination_ohm".to_string(), json!(expected_ohm));
    finding.limit.insert(
        "termination_tolerance_percent".to_string(),
        json!(tolerance_percent),
    );
    finding
}

fn termination_value_finding(
    scenario: &Scenario,
    component_id: &str,
    line_a_net: &str,
    line_b_net: &str,
    actual_ohm: f64,
    expected_ohm: f64,
    tolerance_percent: f64,
) -> Finding {
    let mut finding = Finding::critical(
        BUS_TERMINATION_VALID,
        &scenario.name,
        format!(
            "Bus termination component {component_id} is {actual_ohm:.3} ohm, outside {expected_ohm:.3} ohm +/- {tolerance_percent:.3}%."
        ),
    );
    finding.component = Some(component_id.to_string());
    finding
        .measured
        .insert("line_a_net".to_string(), json!(line_a_net));
    finding
        .measured
        .insert("line_b_net".to_string(), json!(line_b_net));
    finding
        .measured
        .insert("termination_ohm".to_string(), json!(actual_ohm));
    finding
        .limit
        .insert("expected_termination_ohm".to_string(), json!(expected_ohm));
    finding.limit.insert(
        "termination_tolerance_percent".to_string(),
        json!(tolerance_percent),
    );
    finding.suggested_fixes = vec![
        "Populate the endpoint termination resistor value required by the explicit bus topology policy.".to_string(),
        "If this board is not a bus endpoint, mark board_is_bus_endpoint false and do not populate local termination.".to_string(),
    ];
    finding
}

fn non_endpoint_termination_finding(
    scenario: &Scenario,
    component_id: &str,
    line_a_net: &str,
    line_b_net: &str,
    actual_ohm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        BUS_TERMINATION_VALID,
        &scenario.name,
        format!(
            "Bus termination component {component_id} is populated across {line_a_net}/{line_b_net}, but this scenario declares the board is not a bus endpoint."
        ),
    );
    finding.component = Some(component_id.to_string());
    finding
        .measured
        .insert("line_a_net".to_string(), json!(line_a_net));
    finding
        .measured
        .insert("line_b_net".to_string(), json!(line_b_net));
    finding
        .measured
        .insert("termination_ohm".to_string(), json!(actual_ohm));
    finding.suggested_fixes = vec![
        "Do not populate local termination on non-endpoint nodes.".to_string(),
        "If this board is an endpoint in the selected harness topology, set board_is_bus_endpoint true and validate the termination value.".to_string(),
    ];
    finding
}
