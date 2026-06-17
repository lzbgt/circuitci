use crate::board_ir::{ComponentPlacement, LayoutPoint, NetRoute, RouteSegment, Scenario};
use crate::library::{BoundBoard, MotorBridge, MotorLoad};
use crate::reports::Finding;
use serde_json::json;

use super::{
    MOTOR_BRIDGE_BUDGET_VALID, MOTOR_BRIDGE_LOSS_THERMAL_VALID, MOTOR_CURRENT_SENSE_ACCURACY_VALID,
    MOTOR_CURRENT_SENSE_PLACEMENT_VALID, MOTOR_REGEN_CLAMP_VALID,
};

const VALIDATION_INPUT_MISSING: &str = "VALIDATION_INPUT_MISSING";

pub(super) fn required_route_nets(scenario: &Scenario, findings: &mut Vec<Finding>) -> Vec<String> {
    let Some(raw) = scenario.parameters.get("route_nets") else {
        missing_input(
            scenario,
            "route_nets",
            "Add motor_drive parameters.route_nets as a non-empty list of routed motor/power nets.",
            findings,
        );
        return Vec::new();
    };
    let Some(values) = raw.as_sequence() else {
        missing_input(
            scenario,
            "route_nets",
            "Set motor_drive parameters.route_nets to a list of net names.",
            findings,
        );
        return Vec::new();
    };
    let mut nets = Vec::new();
    for value in values {
        let Some(net) = value.as_str() else {
            missing_input(
                scenario,
                "route_nets",
                "Each motor_drive parameters.route_nets entry must be a string.",
                findings,
            );
            return Vec::new();
        };
        let trimmed = net.trim();
        if trimmed.is_empty() {
            missing_input(
                scenario,
                "route_nets",
                "Motor-drive route net names must not be blank.",
                findings,
            );
            return Vec::new();
        }
        if !nets.iter().any(|existing| existing == trimmed) {
            nets.push(trimmed.to_string());
        }
    }
    if nets.is_empty() {
        missing_input(
            scenario,
            "route_nets",
            "Add at least one motor_drive parameters.route_nets entry.",
            findings,
        );
    }
    nets
}

pub(super) fn required_string(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<String> {
    let Some(raw) = scenario.parameters.get(name) else {
        missing_input(
            scenario,
            name,
            &format!("Add motor_drive parameters.{name}."),
            findings,
        );
        return None;
    };
    let Some(value) = raw.as_str() else {
        missing_input(
            scenario,
            name,
            &format!("Set motor_drive parameters.{name} to a string."),
            findings,
        );
        return None;
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        missing_input(
            scenario,
            name,
            &format!("Set motor_drive parameters.{name} to a non-blank string."),
            findings,
        );
        return None;
    }
    Some(trimmed.to_string())
}

pub(super) fn required_string_list(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Vec<String> {
    let Some(raw) = scenario.parameters.get(name) else {
        missing_input(
            scenario,
            name,
            &format!("Add motor_drive parameters.{name} as a non-empty string list."),
            findings,
        );
        return Vec::new();
    };
    let Some(values) = raw.as_sequence() else {
        missing_input(
            scenario,
            name,
            &format!("Set motor_drive parameters.{name} to a list of strings."),
            findings,
        );
        return Vec::new();
    };
    let mut strings = Vec::new();
    for value in values {
        let Some(raw_string) = value.as_str() else {
            missing_input(
                scenario,
                name,
                &format!("Each motor_drive parameters.{name} entry must be a string."),
                findings,
            );
            return Vec::new();
        };
        let trimmed = raw_string.trim();
        if trimmed.is_empty() {
            missing_input(
                scenario,
                name,
                &format!("Motor-drive parameters.{name} entries must not be blank."),
                findings,
            );
            return Vec::new();
        }
        strings.push(trimmed.to_string());
    }
    if strings.is_empty() {
        missing_input(
            scenario,
            name,
            &format!("Add at least one motor_drive parameters.{name} entry."),
            findings,
        );
    }
    strings
}

pub(super) struct RouteCurrentEvidence {
    pub(super) current_a: f64,
    pub(super) source: &'static str,
    pub(super) motor_component: Option<String>,
}

pub(super) fn route_current_evidence(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> Option<RouteCurrentEvidence> {
    if scenario.parameters.contains_key("route_current_A") {
        return required_positive(scenario, "route_current_A", findings).map(|current_a| {
            RouteCurrentEvidence {
                current_a,
                source: "route_current_A",
                motor_component: None,
            }
        });
    }
    let source = required_current_source(scenario, findings)?;
    let motor_load = motor_load_evidence(bound, scenario, findings)?;
    let current = match source {
        "phase_rms" => motor_load.load.phase_rms_current_a,
        "phase_peak" => motor_load.load.phase_peak_current_a,
        "max_regen" => motor_load.load.max_regen_current_a,
        _ => None,
    };
    let Some(current_a) = current else {
        missing_input(
            scenario,
            "current_source",
            &format!("Motor load component does not declare current evidence for {source}."),
            findings,
        );
        return None;
    };
    if !current_a.is_finite() || current_a <= 0.0 {
        missing_input(
            scenario,
            "current_source",
            &format!("Motor load {source} current must be finite and greater than zero."),
            findings,
        );
        return None;
    }
    Some(RouteCurrentEvidence {
        current_a,
        source,
        motor_component: Some(motor_load.component_id),
    })
}

fn required_current_source(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> Option<&'static str> {
    let Some(raw) = scenario.parameters.get("current_source") else {
        missing_input(
            scenario,
            "current_source",
            "Add motor_drive parameters.current_source as phase_rms, phase_peak, or max_regen when route_current_A is omitted.",
            findings,
        );
        return None;
    };
    let Some(value) = raw.as_str() else {
        missing_input(
            scenario,
            "current_source",
            "Set motor_drive parameters.current_source to phase_rms, phase_peak, or max_regen.",
            findings,
        );
        return None;
    };
    match value.trim() {
        "phase_rms" => Some("phase_rms"),
        "phase_peak" => Some("phase_peak"),
        "max_regen" => Some("max_regen"),
        _ => {
            missing_input(
                scenario,
                "current_source",
                "Use current_source: phase_rms, phase_peak, or max_regen.",
                findings,
            );
            None
        }
    }
}

pub(super) fn route_evidence<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    net: &str,
    findings: &mut Vec<Finding>,
) -> Option<&'a NetRoute> {
    if !bound.project.board.nets.contains_key(net) {
        missing_input(
            scenario,
            "route_nets",
            &format!("Declare motor-drive route net {net} in board.nets."),
            findings,
        );
        return None;
    }
    let Some(route) = bound.project.board.layout.routes.get(net) else {
        missing_input(
            scenario,
            "board.layout.routes",
            &format!(
                "Add imported or explicit board.layout.routes evidence for motor-drive net {net}."
            ),
            findings,
        );
        return None;
    };
    Some(route)
}

pub(super) fn placement_evidence<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    component_id: &str,
    findings: &mut Vec<Finding>,
) -> Option<&'a ComponentPlacement> {
    if !bound.project.board.components.contains_key(component_id) {
        missing_input(
            scenario,
            "component",
            &format!("Declare motor-drive component {component_id} in board.components."),
            findings,
        );
        return None;
    }
    let Some(placement) = bound.project.board.layout.placements.get(component_id) else {
        missing_input(
            scenario,
            "board.layout.placements",
            &format!(
                "Add board.layout.placements evidence for motor-drive component {component_id}."
            ),
            findings,
        );
        return None;
    };
    if !placement.x_mm.is_finite() || !placement.y_mm.is_finite() {
        missing_input(
            scenario,
            "board.layout.placements",
            &format!("Use finite placement coordinates for motor-drive component {component_id}."),
            findings,
        );
        return None;
    }
    Some(placement)
}

pub(super) fn min_route_width_mm(route: &NetRoute) -> Option<f64> {
    route
        .segments
        .iter()
        .filter_map(|segment| {
            if segment.width_mm.is_finite() && segment.width_mm > 0.0 {
                Some(segment.width_mm)
            } else {
                None
            }
        })
        .min_by(|left, right| left.total_cmp(right))
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PlacementPoint {
    x_mm: f64,
    y_mm: f64,
}

impl From<&ComponentPlacement> for PlacementPoint {
    fn from(placement: &ComponentPlacement) -> Self {
        Self {
            x_mm: placement.x_mm,
            y_mm: placement.y_mm,
        }
    }
}

impl From<&LayoutPoint> for PlacementPoint {
    fn from(point: &LayoutPoint) -> Self {
        Self {
            x_mm: point.x_mm,
            y_mm: point.y_mm,
        }
    }
}

pub(super) fn distance_to_route_mm(route: &NetRoute, point: PlacementPoint) -> Option<f64> {
    route
        .segments
        .iter()
        .filter(|segment| segment.width_mm.is_finite() && segment.width_mm > 0.0)
        .filter_map(|segment| distance_to_segment_mm(segment, point))
        .min_by(|left, right| left.total_cmp(right))
}

pub(super) fn route_length_mm(route: &NetRoute) -> f64 {
    route
        .segments
        .iter()
        .filter(|segment| segment.width_mm.is_finite() && segment.width_mm > 0.0)
        .map(segment_length_mm)
        .sum()
}

fn distance_to_segment_mm(segment: &RouteSegment, point: PlacementPoint) -> Option<f64> {
    let start = PlacementPoint::from(&segment.start);
    let end = PlacementPoint::from(&segment.end);
    let dx = end.x_mm - start.x_mm;
    let dy = end.y_mm - start.y_mm;
    let length_squared = dx * dx + dy * dy;
    if !length_squared.is_finite() || length_squared <= f64::EPSILON {
        return None;
    }
    let raw_t = ((point.x_mm - start.x_mm) * dx + (point.y_mm - start.y_mm) * dy) / length_squared;
    let t = raw_t.clamp(0.0, 1.0);
    let projected = PlacementPoint {
        x_mm: start.x_mm + t * dx,
        y_mm: start.y_mm + t * dy,
    };
    Some(point_distance_mm(point, projected))
}

fn segment_length_mm(segment: &RouteSegment) -> f64 {
    point_distance_mm(
        PlacementPoint::from(&segment.start),
        PlacementPoint::from(&segment.end),
    )
}

pub(super) fn point_distance_mm(left: PlacementPoint, right: PlacementPoint) -> f64 {
    (left.x_mm - right.x_mm).hypot(left.y_mm - right.y_mm)
}

pub(super) fn current_sense_distance_finding(
    scenario: &Scenario,
    component_id: &str,
    measured_name: &str,
    measured: f64,
    limit_name: &str,
    limit: f64,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        MOTOR_CURRENT_SENSE_PLACEMENT_VALID,
        &scenario.name,
        format!(
            "Motor current-sense component {component_id} has {measured_name} {measured:.6} mm, limit {limit:.6} mm."
        ),
    );
    finding.component = Some(component_id.to_string());
    finding
        .measured
        .insert(measured_name.to_string(), json!(measured));
    finding.limit.insert(limit_name.to_string(), json!(limit));
    finding.suggested_fixes = vec![
        "Move the phase shunt closer to the bridge and routed phase copper, or update the scenario limit with sourced layout policy.".to_string(),
        "Keep the Kelvin/current-sense route short and explicitly represented in board.layout.routes before fabrication review.".to_string(),
    ];
    findings.push(finding);
}

pub(super) struct CurrentSenseAccuracyComparison<'a> {
    pub(super) measured_name: &'a str,
    pub(super) limit_name: &'a str,
    pub(super) measured: f64,
    pub(super) limit: f64,
    pub(super) message: &'a str,
    pub(super) fix: &'a str,
}

pub(super) fn current_sense_accuracy_finding(
    scenario: &Scenario,
    component: &str,
    comparison: CurrentSenseAccuracyComparison<'_>,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        MOTOR_CURRENT_SENSE_ACCURACY_VALID,
        &scenario.name,
        comparison.message,
    );
    finding.component = Some(component.to_string());
    finding.measured.insert(
        comparison.measured_name.to_string(),
        json!(comparison.measured),
    );
    finding
        .limit
        .insert(comparison.limit_name.to_string(), json!(comparison.limit));
    finding.suggested_fixes = vec![comparison.fix.to_string()];
    findings.push(finding);
}

pub(super) fn positive_bridge_value(
    scenario: &Scenario,
    name: &str,
    value: Option<f64>,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    let Some(value) = value else {
        missing_input(
            scenario,
            name,
            &format!("Add source-backed {name} metadata to the motor bridge component model."),
            findings,
        );
        return None;
    };
    if value.is_finite() && value > 0.0 {
        Some(value)
    } else {
        missing_input(
            scenario,
            name,
            &format!("Set {name} to a finite value greater than zero."),
            findings,
        );
        None
    }
}

pub(super) fn required_resolution_bits(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> Option<u32> {
    let Some(raw) = scenario.parameters.get("adc_resolution_bits") else {
        missing_input(
            scenario,
            "adc_resolution_bits",
            "Add motor_drive parameters.adc_resolution_bits.",
            findings,
        );
        return None;
    };
    let Some(bits) = raw.as_u64() else {
        missing_input(
            scenario,
            "adc_resolution_bits",
            "Set motor_drive parameters.adc_resolution_bits to an integer.",
            findings,
        );
        return None;
    };
    if (1..=30).contains(&bits) {
        Some(bits as u32)
    } else {
        missing_input(
            scenario,
            "adc_resolution_bits",
            "Set motor_drive parameters.adc_resolution_bits between 1 and 30.",
            findings,
        );
        None
    }
}

pub(super) fn bridge_loss_multiplier(
    scenario: &Scenario,
    bridge: &MotorBridge,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    let scope = bridge.reference_loss_scope.as_deref().unwrap_or("");
    match scope {
        "three_phase_bridge" => Some(1.0),
        "per_half_bridge" => {
            let Some(devices) = bridge.switching_devices else {
                missing_input(
                    scenario,
                    "motor_bridge.switching_devices",
                    "Declare motor_bridge.switching_devices when reference_loss_scope is per_half_bridge.",
                    findings,
                );
                return None;
            };
            if devices > 0 {
                Some(devices as f64)
            } else {
                missing_input(
                    scenario,
                    "motor_bridge.switching_devices",
                    "Set motor_bridge.switching_devices to at least one.",
                    findings,
                );
                None
            }
        }
        _ => {
            missing_input(
                scenario,
                "motor_bridge.reference_loss_scope",
                "Use motor_bridge.reference_loss_scope: per_half_bridge or three_phase_bridge.",
                findings,
            );
            None
        }
    }
}

pub(super) struct BridgeLossComparison<'a> {
    pub(super) measured_name: &'a str,
    pub(super) limit_name: &'a str,
    pub(super) measured: f64,
    pub(super) limit: f64,
    pub(super) message: &'a str,
    pub(super) fix: &'a str,
}

pub(super) fn bridge_loss_finding(
    scenario: &Scenario,
    component: &str,
    comparison: BridgeLossComparison<'_>,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        MOTOR_BRIDGE_LOSS_THERMAL_VALID,
        &scenario.name,
        comparison.message,
    );
    finding.component = Some(component.to_string());
    finding.measured.insert(
        comparison.measured_name.to_string(),
        json!(comparison.measured),
    );
    finding
        .limit
        .insert(comparison.limit_name.to_string(), json!(comparison.limit));
    finding.suggested_fixes = vec![comparison.fix.to_string()];
    findings.push(finding);
}

pub(super) struct RegenClampComparison<'a> {
    pub(super) measured_name: &'a str,
    pub(super) limit_name: &'a str,
    pub(super) measured: f64,
    pub(super) limit: f64,
    pub(super) message: &'a str,
    pub(super) fix: &'a str,
}

pub(super) fn regen_clamp_finding(
    scenario: &Scenario,
    component: &str,
    comparison: RegenClampComparison<'_>,
    findings: &mut Vec<Finding>,
) {
    let mut finding =
        Finding::critical(MOTOR_REGEN_CLAMP_VALID, &scenario.name, comparison.message);
    finding.component = Some(component.to_string());
    finding.measured.insert(
        comparison.measured_name.to_string(),
        json!(comparison.measured),
    );
    finding
        .limit
        .insert(comparison.limit_name.to_string(), json!(comparison.limit));
    finding.suggested_fixes = vec![comparison.fix.to_string()];
    findings.push(finding);
}

pub(super) fn required_positive(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    let value = required_number(scenario, name, findings)?;
    if value > 0.0 {
        Some(value)
    } else {
        missing_input(
            scenario,
            name,
            &format!("Set motor_drive parameters.{name} to a finite value greater than zero."),
            findings,
        );
        None
    }
}

pub(super) fn required_positive_with_fallback(
    scenario: &Scenario,
    name: &str,
    fallback: Option<f64>,
    fallback_name: &str,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    if scenario.parameters.contains_key(name) {
        return required_positive(scenario, name, findings);
    }
    let Some(value) = fallback else {
        missing_input(
            scenario,
            name,
            &format!(
                "Add motor_drive parameters.{name}, or set parameters.motor_component to a component model with {fallback_name}."
            ),
            findings,
        );
        return None;
    };
    if value.is_finite() && value > 0.0 {
        Some(value)
    } else {
        missing_input(
            scenario,
            fallback_name,
            &format!("Set {fallback_name} to a finite value greater than zero."),
            findings,
        );
        None
    }
}

pub(super) fn required_non_negative(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    let value = required_number(scenario, name, findings)?;
    if value >= 0.0 {
        Some(value)
    } else {
        missing_input(
            scenario,
            name,
            &format!("Set motor_drive parameters.{name} to a finite non-negative value."),
            findings,
        );
        None
    }
}

pub(super) fn required_non_negative_with_fallback(
    scenario: &Scenario,
    name: &str,
    fallback: Option<f64>,
    fallback_name: &str,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    if scenario.parameters.contains_key(name) {
        return required_non_negative(scenario, name, findings);
    }
    let Some(value) = fallback else {
        missing_input(
            scenario,
            name,
            &format!(
                "Add motor_drive parameters.{name}, or set parameters.motor_component to a component model with {fallback_name}."
            ),
            findings,
        );
        return None;
    };
    if value.is_finite() && value >= 0.0 {
        Some(value)
    } else {
        missing_input(
            scenario,
            fallback_name,
            &format!("Set {fallback_name} to a finite non-negative value."),
            findings,
        );
        None
    }
}

pub(super) fn required_at_least(
    scenario: &Scenario,
    name: &str,
    min: f64,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    let value = required_number(scenario, name, findings)?;
    if value >= min {
        Some(value)
    } else {
        missing_input(
            scenario,
            name,
            &format!("Set motor_drive parameters.{name} to at least {min}."),
            findings,
        );
        None
    }
}

pub(super) fn optional_positive(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    let raw = scenario.parameters.get(name)?;
    let Some(value) = raw.as_f64() else {
        missing_input(
            scenario,
            name,
            &format!("Set motor_drive parameters.{name} to a number."),
            findings,
        );
        return None;
    };
    if value.is_finite() && value > 0.0 {
        Some(value)
    } else {
        missing_input(
            scenario,
            name,
            &format!("Set motor_drive parameters.{name} to a finite value greater than zero."),
            findings,
        );
        None
    }
}

pub(super) fn required_number(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    let Some(raw) = scenario.parameters.get(name) else {
        missing_input(
            scenario,
            name,
            &format!("Add motor_drive parameters.{name}."),
            findings,
        );
        return None;
    };
    let Some(value) = raw.as_f64() else {
        missing_input(
            scenario,
            name,
            &format!("Set motor_drive parameters.{name} to a number."),
            findings,
        );
        return None;
    };
    if value.is_finite() {
        Some(value)
    } else {
        missing_input(
            scenario,
            name,
            &format!("Set motor_drive parameters.{name} to a finite number."),
            findings,
        );
        None
    }
}

pub(super) struct MotorLoadEvidence<'a> {
    pub(super) component_id: String,
    pub(super) load: &'a MotorLoad,
}

pub(super) fn motor_load_evidence<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> Option<MotorLoadEvidence<'a>> {
    let raw = scenario.parameters.get("motor_component")?;
    let Some(component_id) = raw.as_str() else {
        missing_input(
            scenario,
            "motor_component",
            "Set motor_drive parameters.motor_component to a component id string.",
            findings,
        );
        return None;
    };
    let Some(component) = bound.project.board.components.get(component_id) else {
        missing_input(
            scenario,
            "motor_component",
            "Set motor_drive parameters.motor_component to an existing motor/load component.",
            findings,
        );
        return None;
    };
    let Some(model) = bound.library.get(&component.model) else {
        missing_input(
            scenario,
            "motor_component.model",
            "Bind the motor/load component to a component model.",
            findings,
        );
        return None;
    };
    let Some(load) = model.motor_load.as_ref() else {
        missing_input(
            scenario,
            "motor_component.motor_load",
            "Use a motor/load component model that declares motor_load current evidence.",
            findings,
        );
        return None;
    };
    Some(MotorLoadEvidence {
        component_id: component_id.to_string(),
        load,
    })
}

pub(super) fn missing_input(
    scenario: &Scenario,
    input: &str,
    fix: &str,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        VALIDATION_INPUT_MISSING,
        &scenario.name,
        format!("Motor-drive bridge-budget validation requires {input}."),
    );
    finding
        .limit
        .insert("required_input".to_string(), json!(input));
    finding.suggested_fixes = vec![fix.to_string()];
    findings.push(finding);
}

pub(super) struct BudgetComparison<'a> {
    pub(super) measured_name: &'a str,
    pub(super) limit_name: &'a str,
    pub(super) measured: f64,
    pub(super) limit: f64,
    pub(super) message: &'a str,
    pub(super) fix: &'a str,
}

pub(super) fn budget_finding(
    scenario: &Scenario,
    component: &str,
    comparison: BudgetComparison<'_>,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        MOTOR_BRIDGE_BUDGET_VALID,
        &scenario.name,
        comparison.message,
    );
    finding.component = Some(component.to_string());
    finding.measured.insert(
        comparison.measured_name.to_string(),
        json!(comparison.measured),
    );
    finding
        .limit
        .insert(comparison.limit_name.to_string(), json!(comparison.limit));
    finding.suggested_fixes = vec![comparison.fix.to_string()];
    findings.push(finding);
}
