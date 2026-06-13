use crate::board_ir::{LayoutCopperFeature, LayoutDrill, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use super::super::DRILL_ANNULAR_RING_VALID;
use super::super::common::validation_input_missing;
use super::geometry::{
    annular_ring_for_feature, point_distance_mm, validate_copper_feature_geometry,
    validate_drill_geometry,
};
use super::{
    insert_drill_measurements, insert_optional_copper_feature_owner_measurements,
    optional_numeric_parameter, required_numeric_parameter,
};

pub(in crate::validation) fn validate_drill_annular_ring(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(min_annular_ring_mm) =
        required_numeric_parameter(scenario, "min_annular_ring_mm", findings)
    else {
        return;
    };
    if min_annular_ring_mm < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.min_annular_ring_mm must be greater than or equal to zero.",
        );
        return;
    }
    let Some(max_center_offset_mm) = optional_numeric_parameter(
        scenario,
        "max_drill_to_copper_center_offset_mm",
        0.1,
        findings,
    ) else {
        return;
    };
    if max_center_offset_mm < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.max_drill_to_copper_center_offset_mm must be greater than or equal to zero.",
        );
        return;
    }
    let Some(required_copper_layers) =
        optional_string_list_parameter(scenario, "required_copper_layers", findings)
    else {
        return;
    };
    let drills = &bound.project.board.layout.drills;
    if drills.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "DRILL_ANNULAR_RING_VALID requires board.layout.drills evidence.",
        );
        return;
    }
    let copper_features = &bound.project.board.layout.copper.features;
    if copper_features.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "DRILL_ANNULAR_RING_VALID requires board.layout.copper.features evidence.",
        );
        return;
    }
    for (drill_index, drill) in drills.iter().enumerate() {
        if drill.plating == "non_plated" {
            continue;
        }
        if let Err(message) = validate_drill_geometry(drill, drill_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        let mut candidates = Vec::new();
        let mut mismatched_candidates = Vec::new();
        for (feature_index, feature) in copper_features.iter().enumerate() {
            if let Err(message) = validate_copper_feature_geometry(feature, feature_index) {
                validation_input_missing(findings, scenario, message);
                continue;
            }
            let center_offset_mm = point_distance_mm(&drill.at, &feature.at);
            if center_offset_mm > max_center_offset_mm {
                continue;
            }
            let Some(annular_ring_mm) = annular_ring_for_feature(drill, feature) else {
                continue;
            };
            let candidate = DrillAnnularRingCandidate {
                feature,
                feature_index,
                center_offset_mm,
                annular_ring_mm,
            };
            if drill_and_copper_owners_conflict(drill, feature) {
                mismatched_candidates.push(candidate);
            } else {
                candidates.push(candidate);
            }
        }
        if required_copper_layers.is_empty() {
            let evidence = DrillAnnularRingEvidence {
                candidates: &candidates,
                mismatched_candidates: &mismatched_candidates,
            };
            let limits = DrillAnnularRingLimits {
                min_annular_ring_mm,
                max_center_offset_mm,
            };
            report_annular_ring_candidate(
                scenario,
                findings,
                drill,
                drill_index,
                evidence,
                None,
                limits,
            );
        } else {
            let evidence = DrillAnnularRingEvidence {
                candidates: &candidates,
                mismatched_candidates: &mismatched_candidates,
            };
            let limits = DrillAnnularRingLimits {
                min_annular_ring_mm,
                max_center_offset_mm,
            };
            for required_layer in &required_copper_layers {
                report_annular_ring_candidate(
                    scenario,
                    findings,
                    drill,
                    drill_index,
                    evidence,
                    Some(required_layer),
                    limits,
                );
            }
        }
    }
}

fn optional_string_list_parameter(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<Vec<String>> {
    let Some(value) = scenario.parameters.get(name) else {
        return Some(Vec::new());
    };
    let Some(sequence) = value.as_sequence() else {
        validation_input_missing(
            findings,
            scenario,
            format!("manufacturing parameters.{name} must be a list of strings."),
        );
        return None;
    };
    if sequence.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            format!("manufacturing parameters.{name} must not be empty when provided."),
        );
        return None;
    }
    let mut strings = Vec::new();
    for (index, item) in sequence.iter().enumerate() {
        let Some(item) = item.as_str() else {
            validation_input_missing(
                findings,
                scenario,
                format!("manufacturing parameters.{name}[{index}] must be a string."),
            );
            return None;
        };
        let item = item.trim();
        if item.is_empty() {
            validation_input_missing(
                findings,
                scenario,
                format!("manufacturing parameters.{name}[{index}] must not be empty."),
            );
            return None;
        }
        if strings.iter().any(|existing| existing == item) {
            validation_input_missing(
                findings,
                scenario,
                format!("manufacturing parameters.{name} contains duplicate layer {item}."),
            );
            return None;
        }
        strings.push(item.to_string());
    }
    Some(strings)
}

fn drill_and_copper_owners_conflict(drill: &LayoutDrill, feature: &LayoutCopperFeature) -> bool {
    if matches!(
        (drill.net.as_deref(), feature.net.as_deref()),
        (Some(drill_net), Some(feature_net)) if drill_net != feature_net
    ) {
        return true;
    }
    drill_and_copper_rich_owners_conflict(drill, feature)
}

fn drill_and_copper_rich_owners_conflict(
    drill: &LayoutDrill,
    feature: &LayoutCopperFeature,
) -> bool {
    match (drill.owner_kind.as_deref(), feature.owner_kind.as_deref()) {
        (Some(drill_kind), Some(feature_kind)) if drill_kind != feature_kind => true,
        (Some("pad"), Some("pad")) => {
            match (
                drill.component.as_deref(),
                drill.pin.as_deref(),
                feature.component.as_deref(),
                feature.pin.as_deref(),
            ) {
                (
                    Some(drill_component),
                    Some(drill_pin),
                    Some(feature_component),
                    Some(feature_pin),
                ) => (drill_component, drill_pin) != (feature_component, feature_pin),
                _ => false,
            }
        }
        (Some("via"), Some("via")) => match (drill.via_index, feature.via_index) {
            (Some(drill_via), Some(feature_via)) => drill_via != feature_via,
            _ => false,
        },
        _ => false,
    }
}

fn report_annular_ring_candidate(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    drill: &LayoutDrill,
    drill_index: usize,
    evidence: DrillAnnularRingEvidence<'_, '_>,
    required_layer: Option<&str>,
    limits: DrillAnnularRingLimits,
) {
    let Some(best_candidate) = best_annular_ring_candidate(evidence.candidates, required_layer)
    else {
        if let Some(mismatched_candidate) =
            best_annular_ring_candidate(evidence.mismatched_candidates, required_layer)
        {
            findings.push(drill_annular_ring_owner_mismatch_finding(
                scenario,
                drill,
                drill_index,
                mismatched_candidate,
                required_layer,
                limits.min_annular_ring_mm,
                limits.max_center_offset_mm,
            ));
            return;
        }
        findings.push(drill_annular_ring_missing_finding(
            scenario,
            drill,
            drill_index,
            required_layer,
            limits.min_annular_ring_mm,
            limits.max_center_offset_mm,
        ));
        return;
    };
    if best_candidate.annular_ring_mm + f64::EPSILON < limits.min_annular_ring_mm {
        findings.push(drill_annular_ring_finding(
            scenario,
            drill,
            drill_index,
            best_candidate,
            required_layer,
            limits.min_annular_ring_mm,
            limits.max_center_offset_mm,
        ));
    }
}

fn best_annular_ring_candidate<'a>(
    candidates: &[DrillAnnularRingCandidate<'a>],
    required_layer: Option<&str>,
) -> Option<DrillAnnularRingCandidate<'a>> {
    candidates
        .iter()
        .filter(|candidate| {
            required_layer.is_none_or(|required_layer| candidate.feature.layer == required_layer)
        })
        .copied()
        .max_by(|left, right| left.annular_ring_mm.total_cmp(&right.annular_ring_mm))
}

#[derive(Debug, Clone, Copy)]
struct DrillAnnularRingCandidate<'a> {
    feature: &'a LayoutCopperFeature,
    feature_index: usize,
    center_offset_mm: f64,
    annular_ring_mm: f64,
}

#[derive(Debug, Clone, Copy)]
struct DrillAnnularRingEvidence<'a, 'b> {
    candidates: &'b [DrillAnnularRingCandidate<'a>],
    mismatched_candidates: &'b [DrillAnnularRingCandidate<'a>],
}

#[derive(Debug, Clone, Copy)]
struct DrillAnnularRingLimits {
    min_annular_ring_mm: f64,
    max_center_offset_mm: f64,
}

fn drill_annular_ring_missing_finding(
    scenario: &Scenario,
    drill: &LayoutDrill,
    drill_index: usize,
    required_layer: Option<&str>,
    min_annular_ring_mm: f64,
    max_center_offset_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        DRILL_ANNULAR_RING_VALID,
        &scenario.name,
        if let Some(required_layer) = required_layer {
            format!(
                "Plated/unknown drill hit {} has no co-located Gerber copper flash evidence on required layer {} within {:.3} mm.",
                drill_index, required_layer, max_center_offset_mm
            )
        } else {
            format!(
                "Plated/unknown drill hit {} has no co-located Gerber copper flash evidence within {:.3} mm.",
                drill_index, max_center_offset_mm
            )
        },
    );
    insert_drill_measurements(&mut finding, drill, drill_index);
    insert_required_copper_layer_measurement(&mut finding, required_layer);
    finding.limit.insert(
        "min_annular_ring_mm".to_string(),
        json!(min_annular_ring_mm),
    );
    finding.limit.insert(
        "max_drill_to_copper_center_offset_mm".to_string(),
        json!(max_center_offset_mm),
    );
    finding.suggested_fixes = vec![
        "Import the matching copper Gerber layer if copper flash evidence is missing.".to_string(),
        "Increase the pad copper size around the plated drill or reduce drill diameter if allowed."
            .to_string(),
        "Check that the drill and copper Gerbers share the same origin and units.".to_string(),
    ];
    finding
}

fn drill_annular_ring_owner_mismatch_finding(
    scenario: &Scenario,
    drill: &LayoutDrill,
    drill_index: usize,
    candidate: DrillAnnularRingCandidate<'_>,
    required_layer: Option<&str>,
    min_annular_ring_mm: f64,
    max_center_offset_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        DRILL_ANNULAR_RING_VALID,
        &scenario.name,
        drill_copper_owner_mismatch_message(drill_index, drill, candidate.feature),
    );
    insert_drill_measurements(&mut finding, drill, drill_index);
    insert_required_copper_layer_measurement(&mut finding, required_layer);
    finding.measured.insert(
        "annular_ring_mm".to_string(),
        json!(candidate.annular_ring_mm),
    );
    finding.measured.insert(
        "drill_to_copper_center_offset_mm".to_string(),
        json!(candidate.center_offset_mm),
    );
    insert_copper_feature_measurements(&mut finding, candidate);
    finding
        .measured
        .insert("drill_copper_owner_mismatch".to_string(), json!(true));
    finding.limit.insert(
        "min_annular_ring_mm".to_string(),
        json!(min_annular_ring_mm),
    );
    finding.limit.insert(
        "max_drill_to_copper_center_offset_mm".to_string(),
        json!(max_center_offset_mm),
    );
    finding.suggested_fixes = vec![
        "Check that the drill hit is associated with the intended pad or via net.".to_string(),
        "Check the copper Gerber layer and PCB net ownership evidence for origin or net mapping errors."
            .to_string(),
        "Move or correct the copper pad/flash so the drilled pad or via has copper on the same owner net."
            .to_string(),
    ];
    finding
}

fn drill_copper_owner_mismatch_message(
    drill_index: usize,
    drill: &LayoutDrill,
    feature: &LayoutCopperFeature,
) -> String {
    if matches!(
        (drill.net.as_deref(), feature.net.as_deref()),
        (Some(drill_net), Some(feature_net)) if drill_net != feature_net
    ) {
        return format!(
            "Drill hit {} owner net {} is co-located with Gerber copper flash owner net {}, so annular-ring evidence is on the wrong owner.",
            drill_index,
            drill.net.as_deref().unwrap_or("unknown"),
            feature.net.as_deref().unwrap_or("unknown")
        );
    }
    format!(
        "Drill hit {} owner {} is co-located with Gerber copper flash owner {}, so annular-ring evidence is on a different pad/via owner.",
        drill_index,
        drill_owner_label(drill),
        copper_feature_owner_label(feature)
    )
}

fn drill_owner_label(drill: &LayoutDrill) -> String {
    match drill.owner_kind.as_deref() {
        Some("pad") => format!(
            "pad {}/{} on net {}",
            drill.component.as_deref().unwrap_or("unknown"),
            drill.pin.as_deref().unwrap_or("unknown"),
            drill.net.as_deref().unwrap_or("unknown")
        ),
        Some("via") => format!(
            "via {} on net {}",
            drill
                .via_index
                .map(|index| index.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            drill.net.as_deref().unwrap_or("unknown")
        ),
        Some(kind) => format!(
            "{} on net {}",
            kind,
            drill.net.as_deref().unwrap_or("unknown")
        ),
        None => format!("net {}", drill.net.as_deref().unwrap_or("unknown")),
    }
}

fn copper_feature_owner_label(feature: &LayoutCopperFeature) -> String {
    match feature.owner_kind.as_deref() {
        Some("pad") => format!(
            "pad {}/{} on net {}",
            feature.component.as_deref().unwrap_or("unknown"),
            feature.pin.as_deref().unwrap_or("unknown"),
            feature.net.as_deref().unwrap_or("unknown")
        ),
        Some("via") => format!(
            "via {} on net {}",
            feature
                .via_index
                .map(|index| index.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            feature.net.as_deref().unwrap_or("unknown")
        ),
        Some(kind) => format!(
            "{} on net {}",
            kind,
            feature.net.as_deref().unwrap_or("unknown")
        ),
        None => format!("net {}", feature.net.as_deref().unwrap_or("unknown")),
    }
}

fn drill_annular_ring_finding(
    scenario: &Scenario,
    drill: &LayoutDrill,
    drill_index: usize,
    candidate: DrillAnnularRingCandidate<'_>,
    required_layer: Option<&str>,
    min_annular_ring_mm: f64,
    max_center_offset_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        DRILL_ANNULAR_RING_VALID,
        &scenario.name,
        if let Some(required_layer) = required_layer {
            format!(
                "Drill hit {} has {:.3} mm annular ring on required layer {}, below {:.3} mm minimum.",
                drill_index, candidate.annular_ring_mm, required_layer, min_annular_ring_mm
            )
        } else {
            format!(
                "Drill hit {} has {:.3} mm annular ring, below {:.3} mm minimum.",
                drill_index, candidate.annular_ring_mm, min_annular_ring_mm
            )
        },
    );
    insert_drill_measurements(&mut finding, drill, drill_index);
    insert_required_copper_layer_measurement(&mut finding, required_layer);
    finding.measured.insert(
        "annular_ring_mm".to_string(),
        json!(candidate.annular_ring_mm),
    );
    finding.measured.insert(
        "drill_to_copper_center_offset_mm".to_string(),
        json!(candidate.center_offset_mm),
    );
    insert_copper_feature_measurements(&mut finding, candidate);
    finding.limit.insert(
        "min_annular_ring_mm".to_string(),
        json!(min_annular_ring_mm),
    );
    finding.limit.insert(
        "max_drill_to_copper_center_offset_mm".to_string(),
        json!(max_center_offset_mm),
    );
    finding.suggested_fixes = vec![
        "Increase the pad copper diameter or pad dimensions around the plated drill.".to_string(),
        "Reduce drill diameter only if the mechanical and plating requirements allow it."
            .to_string(),
        "Check drill-to-copper registration if the drill is off-center in the Gerber evidence."
            .to_string(),
    ];
    finding
}

fn insert_copper_feature_measurements(
    finding: &mut Finding,
    candidate: DrillAnnularRingCandidate<'_>,
) {
    finding.measured.insert(
        "copper_feature_index".to_string(),
        json!(candidate.feature_index),
    );
    finding.measured.insert(
        "copper_feature_x_mm".to_string(),
        json!(candidate.feature.at.x_mm),
    );
    finding.measured.insert(
        "copper_feature_y_mm".to_string(),
        json!(candidate.feature.at.y_mm),
    );
    finding.measured.insert(
        "copper_feature_layer".to_string(),
        json!(candidate.feature.layer),
    );
    insert_optional_copper_feature_owner_measurements(finding, "copper_feature", candidate.feature);
    finding.measured.insert(
        "copper_feature_aperture".to_string(),
        json!(candidate.feature.aperture),
    );
    finding.measured.insert(
        "copper_feature_shape".to_string(),
        json!(candidate.feature.shape),
    );
    finding.measured.insert(
        "copper_feature_size_x_mm".to_string(),
        json!(candidate.feature.size.x_mm),
    );
    finding.measured.insert(
        "copper_feature_size_y_mm".to_string(),
        json!(candidate.feature.size.y_mm),
    );
    finding.measured.insert(
        "copper_feature_source_primitive".to_string(),
        json!(candidate.feature.source_primitive),
    );
    finding.measured.insert(
        "copper_feature_source_primitive_index".to_string(),
        json!(candidate.feature.source_primitive_index),
    );
}

fn insert_required_copper_layer_measurement(finding: &mut Finding, required_layer: Option<&str>) {
    if let Some(required_layer) = required_layer {
        finding
            .measured
            .insert("required_copper_layer".to_string(), json!(required_layer));
    }
}
