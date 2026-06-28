use crate::board_ir::{
    BoardProject, ComponentSourceSpec, LayoutFootprint, PlacementSide, Scenario,
};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use super::super::ASSEMBLY_FOOTPRINT_ALIGNMENT_VALID;
use super::super::common::validation_input_missing;

const DEFAULT_ROTATION_TOLERANCE_DEG: f64 = 0.01;

pub(in crate::validation) fn validate_assembly_footprint_alignment(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(components) = target_components(bound.project, scenario, findings) else {
        return;
    };
    let rotation_tolerance_deg = scenario
        .parameters
        .get("rotation_tolerance_deg")
        .and_then(serde_yaml_ng::Value::as_f64)
        .unwrap_or(DEFAULT_ROTATION_TOLERANCE_DEG);
    if !rotation_tolerance_deg.is_finite() || rotation_tolerance_deg < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "ASSEMBLY_FOOTPRINT_ALIGNMENT_VALID parameters.rotation_tolerance_deg must be a finite non-negative number.",
        );
        return;
    }

    let mut comparable_evidence = 0usize;
    for component_id in components {
        let Some(component) = bound.project.board.components.get(component_id.as_str()) else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "ASSEMBLY_FOOTPRINT_ALIGNMENT_VALID references unknown component {component_id}."
                ),
            );
            continue;
        };
        let Some(source) = component.source.as_ref() else {
            continue;
        };
        let Some(footprint) = bound
            .project
            .board
            .layout
            .footprints
            .get(component_id.as_str())
        else {
            continue;
        };
        comparable_evidence += validate_footprint_name_evidence(
            scenario,
            component_id.as_str(),
            source,
            footprint,
            findings,
        );
        comparable_evidence += validate_part_property_evidence(
            scenario,
            component_id.as_str(),
            source,
            footprint,
            findings,
        );
        comparable_evidence += validate_side_evidence(
            bound.project,
            scenario,
            component_id.as_str(),
            source,
            findings,
        );
        comparable_evidence += validate_rotation_evidence(
            bound.project,
            scenario,
            component_id.as_str(),
            source,
            rotation_tolerance_deg,
            findings,
        );
    }

    if comparable_evidence == 0 {
        validation_input_missing(
            findings,
            scenario,
            "ASSEMBLY_FOOTPRINT_ALIGNMENT_VALID requires at least one component with comparable JLC/EasyEDA assembly source evidence and KiCad PCB footprint or placement evidence.",
        );
    }
}

fn target_components(
    project: &BoardProject,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> Option<Vec<String>> {
    if let Some(value) = scenario.parameters.get("components") {
        let Some(items) = value.as_sequence() else {
            validation_input_missing(
                findings,
                scenario,
                "ASSEMBLY_FOOTPRINT_ALIGNMENT_VALID parameters.components must be a list of component ids.",
            );
            return None;
        };
        if items.is_empty() {
            validation_input_missing(
                findings,
                scenario,
                "ASSEMBLY_FOOTPRINT_ALIGNMENT_VALID parameters.components must not be empty.",
            );
            return None;
        }
        let mut components = Vec::with_capacity(items.len());
        for item in items {
            let Some(component) = item.as_str().map(str::trim).filter(|item| !item.is_empty())
            else {
                validation_input_missing(
                    findings,
                    scenario,
                    "ASSEMBLY_FOOTPRINT_ALIGNMENT_VALID parameters.components entries must be non-empty strings.",
                );
                return None;
            };
            components.push(component.to_string());
        }
        return Some(components);
    }
    if let Some(target) = &scenario.target {
        return Some(vec![target.component.clone()]);
    }
    Some(
        project
            .board
            .components
            .iter()
            .filter_map(|(component_id, component)| {
                component
                    .source
                    .as_ref()
                    .and_then(|source| source.format.as_deref())
                    .filter(|format| *format == "jlc_assembly")
                    .map(|_| component_id.clone())
            })
            .collect(),
    )
}

fn validate_footprint_name_evidence(
    scenario: &Scenario,
    component_id: &str,
    source: &ComponentSourceSpec,
    footprint: &LayoutFootprint,
    findings: &mut Vec<Finding>,
) -> usize {
    let mut comparisons = 0usize;
    for (source_field, assembly_footprint) in [
        ("source.footprint", source.footprint.as_deref()),
        (
            "source.placement_footprint",
            source.placement_footprint.as_deref(),
        ),
    ] {
        let Some(assembly_footprint) = assembly_footprint
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let Some(layout_property) = footprint.properties.iter().find(|property| {
            matches!(
                property.source.as_deref(),
                Some("kicad_footprint_identifier" | "kicad_footprint_property")
            ) && normalize_footprint(&property.value) == normalize_footprint(assembly_footprint)
        }) else {
            comparisons += 1;
            findings.push(alignment_finding(
                scenario,
                component_id,
                "footprint_name_mismatch",
                format!(
                    "Component {component_id} assembly {source_field} '{assembly_footprint}' does not match any imported KiCad footprint property value."
                ),
                [
                    ("component", json!(component_id)),
                    ("assembly_field", json!(source_field)),
                    ("assembly_footprint", json!(assembly_footprint)),
                    (
                        "kicad_footprint_values",
                        json!(
                            footprint
                                .properties
                                .iter()
                                .map(|property| property.value.as_str())
                                .collect::<Vec<_>>()
                        ),
                    ),
                ],
            ));
            continue;
        };
        comparisons += 1;
        if layout_property.name.trim().is_empty() {
            findings.push(alignment_finding(
                scenario,
                component_id,
                "footprint_property_name_empty",
                format!(
                    "Component {component_id} matching KiCad footprint property has an empty name."
                ),
                [
                    ("component", json!(component_id)),
                    ("assembly_field", json!(source_field)),
                    ("assembly_footprint", json!(assembly_footprint)),
                ],
            ));
        }
    }
    comparisons
}

fn validate_part_property_evidence(
    scenario: &Scenario,
    component_id: &str,
    source: &ComponentSourceSpec,
    footprint: &LayoutFootprint,
    findings: &mut Vec<Finding>,
) -> usize {
    let mut comparisons = 0usize;
    comparisons += compare_named_part_property(
        scenario,
        component_id,
        "source.supplier_part",
        source.supplier_part.as_deref(),
        footprint,
        &["jlcpcbpart", "lcscpart", "supplierpart", "supplierpn"],
        findings,
    );
    comparisons += compare_named_part_property(
        scenario,
        component_id,
        "source.manufacturer_part",
        source.manufacturer_part.as_deref(),
        footprint,
        &[
            "mpn",
            "manufacturerpart",
            "manufacturerpartnumber",
            "partnumber",
        ],
        findings,
    );
    comparisons
}

fn compare_named_part_property(
    scenario: &Scenario,
    component_id: &str,
    source_field: &str,
    expected: Option<&str>,
    footprint: &LayoutFootprint,
    property_names: &[&str],
    findings: &mut Vec<Finding>,
) -> usize {
    let Some(expected) = expected.map(str::trim).filter(|value| !value.is_empty()) else {
        return 0;
    };
    let comparable_properties = footprint
        .properties
        .iter()
        .filter(|property| property_names.contains(&normalize_key(&property.name).as_str()))
        .collect::<Vec<_>>();
    if comparable_properties.is_empty() {
        return 0;
    }
    for property in comparable_properties {
        if normalize_exact(&property.value) != normalize_exact(expected) {
            findings.push(alignment_finding(
                scenario,
                component_id,
                "part_property_mismatch",
                format!(
                    "Component {component_id} {source_field} '{expected}' does not match KiCad footprint property {}='{}'.",
                    property.name, property.value
                ),
                [
                    ("component", json!(component_id)),
                    ("assembly_field", json!(source_field)),
                    ("assembly_part", json!(expected)),
                    ("kicad_property_name", json!(property.name)),
                    ("kicad_property_value", json!(property.value)),
                ],
            ));
        }
    }
    1
}

fn validate_side_evidence(
    project: &BoardProject,
    scenario: &Scenario,
    component_id: &str,
    source: &ComponentSourceSpec,
    findings: &mut Vec<Finding>,
) -> usize {
    if source.placement_side_confidence.as_deref() != Some("source_explicit") {
        return 0;
    }
    let Some(assembly_side) = source.placement_side.as_ref() else {
        return 0;
    };
    let Some(layout_side) = project
        .board
        .layout
        .placements
        .get(component_id)
        .and_then(|placement| placement.side.as_ref())
    else {
        return 0;
    };
    if assembly_side != layout_side {
        findings.push(alignment_finding(
            scenario,
            component_id,
            "placement_side_mismatch",
            format!(
                "Component {component_id} assembly placement side {} does not match imported layout side {}.",
                side_name(assembly_side),
                side_name(layout_side)
            ),
            [
                ("component", json!(component_id)),
                ("assembly_side", json!(side_name(assembly_side))),
                ("layout_side", json!(side_name(layout_side))),
            ],
        ));
    }
    1
}

fn validate_rotation_evidence(
    project: &BoardProject,
    scenario: &Scenario,
    component_id: &str,
    source: &ComponentSourceSpec,
    tolerance_deg: f64,
    findings: &mut Vec<Finding>,
) -> usize {
    if source.placement_orientation_confidence.as_deref() != Some("source_explicit") {
        return 0;
    }
    let Some(assembly_rotation_deg) = source.placement_rotation_deg else {
        return 0;
    };
    let Some(layout_rotation_deg) = project
        .board
        .layout
        .placements
        .get(component_id)
        .and_then(|placement| placement.rotation_deg)
    else {
        return 0;
    };
    if !assembly_rotation_deg.is_finite() || !layout_rotation_deg.is_finite() {
        return 0;
    }
    let delta_deg = rotation_delta_deg(assembly_rotation_deg, layout_rotation_deg);
    if delta_deg > tolerance_deg + f64::EPSILON {
        let mut finding = alignment_finding(
            scenario,
            component_id,
            "placement_rotation_mismatch",
            format!(
                "Component {component_id} assembly rotation {assembly_rotation_deg} deg differs from imported layout rotation {layout_rotation_deg} deg by {delta_deg} deg."
            ),
            [
                ("component", json!(component_id)),
                ("assembly_rotation_deg", json!(assembly_rotation_deg)),
                ("layout_rotation_deg", json!(layout_rotation_deg)),
                ("rotation_delta_deg", json!(delta_deg)),
            ],
        );
        finding
            .limit
            .insert("rotation_tolerance_deg".to_string(), json!(tolerance_deg));
        findings.push(finding);
    }
    1
}

fn alignment_finding(
    scenario: &Scenario,
    component_id: &str,
    reason: &str,
    message: String,
    measured: impl IntoIterator<Item = (&'static str, serde_json::Value)>,
) -> Finding {
    let mut finding =
        Finding::critical(ASSEMBLY_FOOTPRINT_ALIGNMENT_VALID, &scenario.name, message);
    finding.component = Some(component_id.to_string());
    finding.measured.extend(
        measured
            .into_iter()
            .map(|(key, value)| (key.to_string(), value)),
    );
    finding.measured.insert("reason".to_string(), json!(reason));
    finding.suggested_fixes = vec![
        "Review BOM/CPL and PCB footprint evidence for the named component.".to_string(),
        "Correct the assembly source, KiCad footprint assignment, or component mapping only after confirming the intended package and orientation.".to_string(),
    ];
    finding
}

fn normalize_key(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn normalize_exact(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn normalize_footprint(value: &str) -> String {
    value
        .rsplit_once(':')
        .map_or(value, |(_, tail)| tail)
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn side_name(side: &PlacementSide) -> &'static str {
    match side {
        PlacementSide::Top => "top",
        PlacementSide::Bottom => "bottom",
    }
}

fn rotation_delta_deg(a: f64, b: f64) -> f64 {
    let raw = (a - b).rem_euclid(360.0).abs();
    raw.min(360.0 - raw)
}
