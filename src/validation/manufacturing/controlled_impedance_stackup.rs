use crate::board_ir::{NetRoute, Scenario, StackupLayer, StackupLayerKind};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use super::super::CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID;
use super::super::common::validation_input_missing;

pub(in crate::validation) fn validate_controlled_impedance_stackup_evidence(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(rules) = stackup_rules(bound, scenario, findings) else {
        return;
    };
    for rule in rules {
        validate_rule(bound, scenario, findings, rule);
    }
}

#[derive(Debug)]
struct StackupRule {
    net: String,
    route_layer: String,
    reference_layer: String,
    dielectric_layer: String,
}

fn stackup_rules(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> Option<Vec<StackupRule>> {
    let Some(value) = scenario.parameters.get("routes") else {
        validation_input_missing(
            findings,
            scenario,
            "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID requires parameters.routes.",
        );
        return None;
    };
    let Some(items) = value.as_sequence() else {
        validation_input_missing(
            findings,
            scenario,
            "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID parameters.routes must be a list.",
        );
        return None;
    };
    if items.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID parameters.routes must not be empty.",
        );
        return None;
    }

    let mut rules = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let Some(mapping) = item.as_mapping() else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID parameters.routes[{index}] must be an object."
                ),
            );
            return None;
        };
        let net = required_string(scenario, findings, mapping, index, "net")?;
        require_declared_net(bound, scenario, findings, index, &net)?;
        rules.push(StackupRule {
            net,
            route_layer: required_string(scenario, findings, mapping, index, "route_layer")?,
            reference_layer: required_string(
                scenario,
                findings,
                mapping,
                index,
                "reference_layer",
            )?,
            dielectric_layer: required_string(
                scenario,
                findings,
                mapping,
                index,
                "dielectric_layer",
            )?,
        });
    }
    Some(rules)
}

fn validate_rule(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    rule: StackupRule,
) {
    let Some(route) = route_for_net(bound, scenario, findings, &rule.net) else {
        return;
    };
    if !route_has_layer_evidence(route, &rule.route_layer) {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID net {} has no finite route segment evidence on route_layer {}.",
                rule.net, rule.route_layer
            ),
        );
        return;
    }

    let layers = &bound.project.board.layout.stackup.layers;
    if layers.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID requires board.layout.stackup.layers evidence.",
        );
        return;
    }

    let Some((route_index, route_layer)) = named_stackup_layer(layers, &rule.route_layer) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID route_layer {} is absent from board.layout.stackup.layers.",
                rule.route_layer
            ),
        );
        return;
    };
    let Some((reference_index, reference_layer)) =
        named_stackup_layer(layers, &rule.reference_layer)
    else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID reference_layer {} is absent from board.layout.stackup.layers.",
                rule.reference_layer
            ),
        );
        return;
    };
    let Some((dielectric_index, dielectric_layer)) =
        named_stackup_layer(layers, &rule.dielectric_layer)
    else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID dielectric_layer {} is absent from board.layout.stackup.layers.",
                rule.dielectric_layer
            ),
        );
        return;
    };

    if route_layer.kind != StackupLayerKind::Signal {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID route_layer {} must be kind: signal.",
                route_layer.name
            ),
        );
        return;
    }
    if reference_layer.kind != StackupLayerKind::Plane {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID reference_layer {} must be kind: plane.",
                reference_layer.name
            ),
        );
        return;
    }
    if dielectric_layer.kind != StackupLayerKind::Dielectric {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID dielectric_layer {} must be kind: dielectric.",
                dielectric_layer.name
            ),
        );
        return;
    }

    let Some(route_copper_thickness_um) = positive_layer_number(route_layer.copper_thickness_um)
    else {
        missing_positive_layer_field(findings, scenario, &route_layer.name, "copper_thickness_um");
        return;
    };
    let Some(reference_copper_thickness_um) =
        positive_layer_number(reference_layer.copper_thickness_um)
    else {
        missing_positive_layer_field(
            findings,
            scenario,
            &reference_layer.name,
            "copper_thickness_um",
        );
        return;
    };
    let Some(dielectric_thickness_mm) = positive_layer_number(dielectric_layer.thickness_mm) else {
        missing_positive_layer_field(findings, scenario, &dielectric_layer.name, "thickness_mm");
        return;
    };
    let Some(dielectric_constant) = positive_layer_number(dielectric_layer.dielectric_constant)
    else {
        missing_positive_layer_field(
            findings,
            scenario,
            &dielectric_layer.name,
            "dielectric_constant",
        );
        return;
    };
    let Some(dielectric_material) = non_empty_layer_string(dielectric_layer.material.as_deref())
    else {
        missing_text_layer_field(findings, scenario, &dielectric_layer.name, "material");
        return;
    };
    let Some(route_source) = non_empty_layer_string(route_layer.source.as_deref()) else {
        missing_text_layer_field(findings, scenario, &route_layer.name, "source");
        return;
    };
    let Some(reference_source) = non_empty_layer_string(reference_layer.source.as_deref()) else {
        missing_text_layer_field(findings, scenario, &reference_layer.name, "source");
        return;
    };
    let Some(dielectric_source) = non_empty_layer_string(dielectric_layer.source.as_deref()) else {
        missing_text_layer_field(findings, scenario, &dielectric_layer.name, "source");
        return;
    };
    let Some(reference_net) = non_empty_layer_string(reference_layer.reference_net.as_deref())
    else {
        missing_text_layer_field(findings, scenario, &reference_layer.name, "reference_net");
        return;
    };

    if !index_between(dielectric_index, route_index, reference_index) {
        findings.push(stackup_topology_finding(
            scenario,
            &rule,
            StackupEvidence {
                route_index,
                reference_index,
                dielectric_index,
                route_copper_thickness_um,
                reference_copper_thickness_um,
                dielectric_thickness_mm,
                dielectric_constant,
                dielectric_material,
                route_source,
                reference_source,
                dielectric_source,
                reference_net,
            },
        ));
    }
}

#[derive(Debug)]
struct StackupEvidence {
    route_index: usize,
    reference_index: usize,
    dielectric_index: usize,
    route_copper_thickness_um: f64,
    reference_copper_thickness_um: f64,
    dielectric_thickness_mm: f64,
    dielectric_constant: f64,
    dielectric_material: String,
    route_source: String,
    reference_source: String,
    dielectric_source: String,
    reference_net: String,
}

fn required_string(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    mapping: &serde_yaml_ng::Mapping,
    index: usize,
    key: &str,
) -> Option<String> {
    let value = mapping
        .get(serde_yaml_ng::Value::String(key.to_string()))
        .and_then(serde_yaml_ng::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    if value.is_none() {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID parameters.routes[{index}].{key} must be a non-empty string."
            ),
        );
    }
    value
}

fn require_declared_net(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    index: usize,
    net: &str,
) -> Option<()> {
    if bound.project.board.nets.contains_key(net) {
        return Some(());
    }
    validation_input_missing(
        findings,
        scenario,
        format!(
            "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID parameters.routes[{index}] references undeclared net {net}."
        ),
    );
    None
}

fn route_for_net<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    net: &str,
) -> Option<&'a NetRoute> {
    let Some(route) = bound.project.board.layout.routes.get(net) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID net {net} has no board.layout.routes entry."
            ),
        );
        return None;
    };
    Some(route)
}

fn route_has_layer_evidence(route: &NetRoute, route_layer: &str) -> bool {
    route.segments.iter().any(|segment| {
        segment.layer == route_layer
            && segment.start.x_mm.is_finite()
            && segment.start.y_mm.is_finite()
            && segment.end.x_mm.is_finite()
            && segment.end.y_mm.is_finite()
            && segment.width_mm.is_finite()
            && segment.width_mm > 0.0
    })
}

fn named_stackup_layer<'a>(
    layers: &'a [StackupLayer],
    name: &str,
) -> Option<(usize, &'a StackupLayer)> {
    layers
        .iter()
        .enumerate()
        .find(|(_, layer)| layer.name == name)
}

fn positive_layer_number(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value > 0.0)
}

fn non_empty_layer_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn missing_positive_layer_field(
    findings: &mut Vec<Finding>,
    scenario: &Scenario,
    layer_name: &str,
    field_name: &str,
) {
    validation_input_missing(
        findings,
        scenario,
        format!(
            "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID stackup layer {layer_name} must declare finite positive {field_name} evidence."
        ),
    );
}

fn missing_text_layer_field(
    findings: &mut Vec<Finding>,
    scenario: &Scenario,
    layer_name: &str,
    field_name: &str,
) {
    validation_input_missing(
        findings,
        scenario,
        format!(
            "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID stackup layer {layer_name} must declare non-empty {field_name} evidence."
        ),
    );
}

fn index_between(candidate: usize, first: usize, second: usize) -> bool {
    let low = first.min(second);
    let high = first.max(second);
    low < candidate && candidate < high
}

fn stackup_topology_finding(
    scenario: &Scenario,
    rule: &StackupRule,
    evidence: StackupEvidence,
) -> Finding {
    let mut finding = Finding::critical(
        CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID,
        &scenario.name,
        format!(
            "Controlled-impedance stackup evidence for net {} names dielectric layer {} outside the route/reference layer interval.",
            rule.net, rule.dielectric_layer
        ),
    );
    finding.measured.insert("net".to_string(), json!(rule.net));
    finding
        .measured
        .insert("route_layer".to_string(), json!(rule.route_layer));
    finding
        .measured
        .insert("reference_layer".to_string(), json!(rule.reference_layer));
    finding
        .measured
        .insert("dielectric_layer".to_string(), json!(rule.dielectric_layer));
    finding
        .measured
        .insert("route_layer_index".to_string(), json!(evidence.route_index));
    finding.measured.insert(
        "reference_layer_index".to_string(),
        json!(evidence.reference_index),
    );
    finding.measured.insert(
        "dielectric_layer_index".to_string(),
        json!(evidence.dielectric_index),
    );
    finding.measured.insert(
        "route_copper_thickness_um".to_string(),
        json!(evidence.route_copper_thickness_um),
    );
    finding.measured.insert(
        "reference_copper_thickness_um".to_string(),
        json!(evidence.reference_copper_thickness_um),
    );
    finding.measured.insert(
        "dielectric_thickness_mm".to_string(),
        json!(evidence.dielectric_thickness_mm),
    );
    finding.measured.insert(
        "dielectric_constant".to_string(),
        json!(evidence.dielectric_constant),
    );
    finding.measured.insert(
        "dielectric_material".to_string(),
        json!(evidence.dielectric_material),
    );
    finding.measured.insert(
        "route_layer_source".to_string(),
        json!(evidence.route_source),
    );
    finding.measured.insert(
        "reference_layer_source".to_string(),
        json!(evidence.reference_source),
    );
    finding.measured.insert(
        "dielectric_layer_source".to_string(),
        json!(evidence.dielectric_source),
    );
    finding
        .measured
        .insert("reference_net".to_string(), json!(evidence.reference_net));
    finding.limit.insert(
        "dielectric_layer_must_be_between_route_and_reference".to_string(),
        json!(true),
    );
    finding.suggested_fixes = vec![
        "Correct the declared dielectric_layer only after reviewing the board stackup table."
            .to_string(),
        "Use this check as stackup evidence validation only; it does not calculate impedance."
            .to_string(),
    ];
    finding
}
