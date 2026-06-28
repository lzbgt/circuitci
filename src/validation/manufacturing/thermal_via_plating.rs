use crate::board_ir::{LayoutDrill, RouteVia, Scenario, ThermalCopperRule};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use super::super::THERMAL_VIA_PLATING_VALID;
use super::super::common::validation_input_missing;
use super::thermal_copper::{
    named_thermal_rule_parameters, positive_number, thermal_rule, thermal_via_spans_layers,
    valid_route_via, validate_rule_metadata,
};

pub(in crate::validation) fn validate_thermal_via_plating(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(names) = named_thermal_rule_parameters(
        scenario,
        findings,
        THERMAL_VIA_PLATING_VALID,
        "thermal_copper",
    ) else {
        return;
    };
    for name in names {
        let Some(rule) = thermal_rule(bound, scenario, findings, THERMAL_VIA_PLATING_VALID, &name)
        else {
            return;
        };
        validate_via_plating_rule(bound, scenario, findings, rule);
    }
}

fn validate_via_plating_rule(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    rule: &ThermalCopperRule,
) {
    if let Err(message) = validate_rule_metadata(bound, rule) {
        validation_input_missing(findings, scenario, message);
        return;
    }
    let Some(min_plated_thermal_via_count) = rule.min_plated_thermal_via_count else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "THERMAL_VIA_PLATING_VALID thermal copper rule {} must declare min_plated_thermal_via_count.",
                rule.name
            ),
        );
        return;
    };
    if min_plated_thermal_via_count == 0 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "THERMAL_VIA_PLATING_VALID thermal copper rule {} min_plated_thermal_via_count must be positive.",
                rule.name
            ),
        );
        return;
    }
    let Some(min_thermal_via_drill_mm) = positive_number(rule.min_thermal_via_drill_mm) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "THERMAL_VIA_PLATING_VALID thermal copper rule {} must declare finite positive min_thermal_via_drill_mm.",
                rule.name
            ),
        );
        return;
    };
    let min_thermal_via_plating_thickness_um =
        positive_number(rule.min_thermal_via_plating_thickness_um);
    if rule.nets.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "THERMAL_VIA_PLATING_VALID thermal copper rule {} requires at least one reviewed net in board.manufacturing.thermal_copper[].nets.",
                rule.name
            ),
        );
        return;
    }
    if rule.layers.len() < 2 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "THERMAL_VIA_PLATING_VALID thermal copper rule {} requires at least two reviewed copper layers.",
                rule.name
            ),
        );
        return;
    }
    if bound.project.board.layout.drills.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "THERMAL_VIA_PLATING_VALID requires board.layout.drills evidence with plating metadata.",
        );
        return;
    }
    let Some(evidence) = thermal_via_plating_evidence(bound, scenario, findings, rule) else {
        return;
    };
    if evidence.matched_drill_count == 0 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "THERMAL_VIA_PLATING_VALID thermal copper rule {} has no matching board.layout.drills evidence for reviewed route vias.",
                rule.name
            ),
        );
        return;
    }
    if min_thermal_via_plating_thickness_um.is_some()
        && evidence.plating_thickness_evidence_count < evidence.plated_thermal_via_count
    {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "THERMAL_VIA_PLATING_VALID thermal copper rule {} requires plating_thickness_um on every matched plated thermal via drill when min_thermal_via_plating_thickness_um is declared.",
                rule.name
            ),
        );
        return;
    }
    if evidence.plated_thermal_via_count < min_plated_thermal_via_count {
        findings.push(thermal_via_plated_count_finding(
            scenario,
            rule,
            &evidence,
            min_plated_thermal_via_count,
            min_thermal_via_drill_mm,
        ));
    }
    if evidence.observed_min_thermal_via_drill_mm + f64::EPSILON < min_thermal_via_drill_mm {
        findings.push(thermal_via_drill_diameter_finding(
            scenario,
            rule,
            &evidence,
            min_plated_thermal_via_count,
            min_thermal_via_drill_mm,
        ));
    }
    if let Some(min_thickness_um) = min_thermal_via_plating_thickness_um
        && evidence.observed_min_thermal_via_plating_thickness_um + f64::EPSILON < min_thickness_um
    {
        findings.push(thermal_via_plating_thickness_finding(
            scenario,
            rule,
            &evidence,
            min_plated_thermal_via_count,
            min_thermal_via_drill_mm,
            min_thickness_um,
        ));
    }
}

fn thermal_via_plating_evidence(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    rule: &ThermalCopperRule,
) -> Option<ThermalViaPlatingEvidence> {
    let mut evidence = ThermalViaPlatingEvidence {
        observed_min_thermal_via_drill_mm: f64::INFINITY,
        observed_min_thermal_via_plating_thickness_um: f64::INFINITY,
        ..ThermalViaPlatingEvidence::default()
    };
    for net in &rule.nets {
        let Some(route) = bound.project.board.layout.routes.get(net) else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "THERMAL_VIA_PLATING_VALID thermal copper rule {} net {net} has no board.layout.routes evidence.",
                    rule.name
                ),
            );
            return None;
        };
        evidence.route_via_count += route.vias.len();
        for (via_index, via) in route.vias.iter().enumerate() {
            if !valid_route_via(via) {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "THERMAL_VIA_PLATING_VALID net {net} via {via_index} must have finite coordinates, positive size/drill, and at least two explicit layers."
                    ),
                );
                return None;
            }
            if !thermal_via_spans_layers(via, &rule.layers) {
                continue;
            }
            evidence.thermal_via_count += 1;
            let Some(drill) = matching_thermal_via_drill(bound, net, via_index, via) else {
                continue;
            };
            if !valid_thermal_drill(drill) {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "THERMAL_VIA_PLATING_VALID matching drill evidence for net {net} via {via_index} must have finite coordinates, positive drill_mm, optional positive plating_thickness_um, and plating of plated/non_plated/unknown."
                    ),
                );
                return None;
            }
            evidence.matched_drill_count += 1;
            if drill.plating == "plated" {
                evidence.plated_thermal_via_count += 1;
                evidence.observed_min_thermal_via_drill_mm = evidence
                    .observed_min_thermal_via_drill_mm
                    .min(drill.drill_mm);
                if let Some(thickness_um) = drill.plating_thickness_um {
                    evidence.plating_thickness_evidence_count += 1;
                    evidence.observed_min_thermal_via_plating_thickness_um = evidence
                        .observed_min_thermal_via_plating_thickness_um
                        .min(thickness_um);
                }
            } else {
                evidence.non_plated_or_unknown_drill_count += 1;
            }
        }
    }
    if !evidence.observed_min_thermal_via_drill_mm.is_finite() {
        evidence.observed_min_thermal_via_drill_mm = 0.0;
    }
    if !evidence
        .observed_min_thermal_via_plating_thickness_um
        .is_finite()
    {
        evidence.observed_min_thermal_via_plating_thickness_um = 0.0;
    }
    Some(evidence)
}

fn matching_thermal_via_drill<'a>(
    bound: &'a BoundBoard<'_>,
    net: &str,
    via_index: usize,
    via: &RouteVia,
) -> Option<&'a LayoutDrill> {
    bound
        .project
        .board
        .layout
        .drills
        .iter()
        .find(|drill| {
            drill.via_index == Some(via_index)
                && drill.net.as_deref().is_none_or(|value| value == net)
        })
        .or_else(|| {
            bound.project.board.layout.drills.iter().find(|drill| {
                drill.net.as_deref().is_none_or(|value| value == net)
                    && (drill.at.x_mm - via.at.x_mm).abs() <= 1.0e-6
                    && (drill.at.y_mm - via.at.y_mm).abs() <= 1.0e-6
                    && (drill.drill_mm - via.drill_mm).abs() <= 1.0e-6
            })
        })
}

fn valid_thermal_drill(drill: &LayoutDrill) -> bool {
    drill.at.x_mm.is_finite()
        && drill.at.y_mm.is_finite()
        && drill.drill_mm.is_finite()
        && drill.drill_mm > 0.0
        && drill
            .plating_thickness_um
            .is_none_or(|value| value.is_finite() && value > 0.0)
        && matches!(drill.plating.as_str(), "plated" | "non_plated" | "unknown")
}

#[derive(Debug, Default)]
struct ThermalViaPlatingEvidence {
    route_via_count: usize,
    thermal_via_count: usize,
    matched_drill_count: usize,
    plated_thermal_via_count: usize,
    non_plated_or_unknown_drill_count: usize,
    plating_thickness_evidence_count: usize,
    observed_min_thermal_via_drill_mm: f64,
    observed_min_thermal_via_plating_thickness_um: f64,
}

fn thermal_via_plated_count_finding(
    scenario: &Scenario,
    rule: &ThermalCopperRule,
    evidence: &ThermalViaPlatingEvidence,
    min_plated_thermal_via_count: usize,
    min_thermal_via_drill_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        THERMAL_VIA_PLATING_VALID,
        &scenario.name,
        format!(
            "Thermal copper rule {} has {} plated thermal via(s), below the reviewed {} minimum.",
            rule.name, evidence.plated_thermal_via_count, min_plated_thermal_via_count
        ),
    );
    populate_via_plating_finding(
        &mut finding,
        rule,
        evidence,
        min_plated_thermal_via_count,
        min_thermal_via_drill_mm,
        None,
    );
    finding.suggested_fixes = vec![
        "Add explicit plated drill evidence for the reviewed thermal route vias, then re-import the drill/layout evidence.".to_string(),
        "If non-plated or unknown drills are intentional, update the reviewed thermal via policy before using this check for sign-off.".to_string(),
    ];
    finding
}

fn thermal_via_drill_diameter_finding(
    scenario: &Scenario,
    rule: &ThermalCopperRule,
    evidence: &ThermalViaPlatingEvidence,
    min_plated_thermal_via_count: usize,
    min_thermal_via_drill_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        THERMAL_VIA_PLATING_VALID,
        &scenario.name,
        format!(
            "Thermal copper rule {} has minimum plated thermal via drill {:.3} mm, below the reviewed {:.3} mm minimum.",
            rule.name, evidence.observed_min_thermal_via_drill_mm, min_thermal_via_drill_mm
        ),
    );
    populate_via_plating_finding(
        &mut finding,
        rule,
        evidence,
        min_plated_thermal_via_count,
        min_thermal_via_drill_mm,
        None,
    );
    finding.suggested_fixes = vec![
        "Increase reviewed thermal via drill diameter or add larger plated thermal vias tied to the reviewed thermal net/layers.".to_string(),
        "If the drill requirement changed, update board.manufacturing.thermal_copper from the reviewed thermal policy.".to_string(),
    ];
    finding
}

fn thermal_via_plating_thickness_finding(
    scenario: &Scenario,
    rule: &ThermalCopperRule,
    evidence: &ThermalViaPlatingEvidence,
    min_plated_thermal_via_count: usize,
    min_thermal_via_drill_mm: f64,
    min_thermal_via_plating_thickness_um: f64,
) -> Finding {
    let mut finding = Finding::critical(
        THERMAL_VIA_PLATING_VALID,
        &scenario.name,
        format!(
            "Thermal copper rule {} has minimum thermal via plating thickness {:.3} um, below the reviewed {:.3} um minimum.",
            rule.name,
            evidence.observed_min_thermal_via_plating_thickness_um,
            min_thermal_via_plating_thickness_um
        ),
    );
    populate_via_plating_finding(
        &mut finding,
        rule,
        evidence,
        min_plated_thermal_via_count,
        min_thermal_via_drill_mm,
        Some(min_thermal_via_plating_thickness_um),
    );
    finding.suggested_fixes = vec![
        "Use fabrication drill evidence with reviewed plating thickness that satisfies the thermal-via policy.".to_string(),
        "If via plating thickness is not controlled by the process, remove or revise the reviewed thermal via plating-thickness requirement before sign-off.".to_string(),
    ];
    finding
}

fn populate_via_plating_finding(
    finding: &mut Finding,
    rule: &ThermalCopperRule,
    evidence: &ThermalViaPlatingEvidence,
    min_plated_thermal_via_count: usize,
    min_thermal_via_drill_mm: f64,
    min_thermal_via_plating_thickness_um: Option<f64>,
) {
    finding.component = Some(rule.component.clone());
    finding
        .measured
        .insert("thermal_copper_name".to_string(), json!(rule.name));
    finding
        .measured
        .insert("thermal_copper_source".to_string(), json!(rule.source));
    finding
        .measured
        .insert("component".to_string(), json!(rule.component));
    finding
        .measured
        .insert("power_loss_w".to_string(), json!(rule.power_loss_w));
    finding
        .measured
        .insert("nets".to_string(), json!(rule.nets));
    finding
        .measured
        .insert("layers".to_string(), json!(rule.layers));
    finding.measured.insert(
        "route_via_count".to_string(),
        json!(evidence.route_via_count),
    );
    finding.measured.insert(
        "thermal_via_count".to_string(),
        json!(evidence.thermal_via_count),
    );
    finding.measured.insert(
        "matched_drill_count".to_string(),
        json!(evidence.matched_drill_count),
    );
    finding.measured.insert(
        "plated_thermal_via_count".to_string(),
        json!(evidence.plated_thermal_via_count),
    );
    finding.measured.insert(
        "non_plated_or_unknown_drill_count".to_string(),
        json!(evidence.non_plated_or_unknown_drill_count),
    );
    finding.measured.insert(
        "plating_thickness_evidence_count".to_string(),
        json!(evidence.plating_thickness_evidence_count),
    );
    finding.measured.insert(
        "observed_min_thermal_via_drill_mm".to_string(),
        json!(evidence.observed_min_thermal_via_drill_mm),
    );
    finding.measured.insert(
        "observed_min_thermal_via_plating_thickness_um".to_string(),
        json!(evidence.observed_min_thermal_via_plating_thickness_um),
    );
    finding.limit.insert(
        "min_plated_thermal_via_count".to_string(),
        json!(min_plated_thermal_via_count),
    );
    finding.limit.insert(
        "min_thermal_via_drill_mm".to_string(),
        json!(min_thermal_via_drill_mm),
    );
    if let Some(value) = min_thermal_via_plating_thickness_um {
        finding.limit.insert(
            "min_thermal_via_plating_thickness_um".to_string(),
            json!(value),
        );
    }
}
