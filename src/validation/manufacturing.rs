mod geometry;

use crate::board_ir::{
    LayoutCopperFeature, LayoutCopperSegment, LayoutDrill, LayoutSegment, Scenario,
};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use self::geometry::{
    CopperFeatureEdgeClearance, CopperObjectRef, CopperSegmentEdgeClearance, DrillEdgeClearance,
    annular_ring_for_feature, copper_object_spacing_mm, nearest_copper_feature_edge_clearance,
    nearest_copper_segment_edge_clearance, nearest_drill_edge_clearance, point_distance_mm,
    usable_outline_segment, validate_copper_feature_geometry, validate_copper_segment_geometry,
    validate_drill_geometry,
};
use super::COPPER_SPACING_VALID;
use super::COPPER_TO_BOARD_EDGE_CLEARANCE_VALID;
use super::DRILL_ANNULAR_RING_VALID;
use super::DRILL_TO_BOARD_EDGE_CLEARANCE_VALID;
use super::common::validation_input_missing;

pub(super) fn validate_drill_to_board_edge_clearance(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(min_clearance_mm) =
        required_numeric_parameter(scenario, "min_drill_edge_clearance_mm", findings)
    else {
        return;
    };
    if min_clearance_mm < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.min_drill_edge_clearance_mm must be greater than or equal to zero.",
        );
        return;
    }
    let drills = &bound.project.board.layout.drills;
    if drills.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "DRILL_TO_BOARD_EDGE_CLEARANCE_VALID requires board.layout.drills evidence.",
        );
        return;
    }
    let board_edges = bound
        .project
        .board
        .layout
        .outline
        .segments
        .iter()
        .filter(|segment| usable_outline_segment(segment))
        .collect::<Vec<_>>();
    if board_edges.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "DRILL_TO_BOARD_EDGE_CLEARANCE_VALID requires usable board.layout.outline.segments evidence.",
        );
        return;
    }
    for (drill_index, drill) in drills.iter().enumerate() {
        if let Err(message) = validate_drill_geometry(drill, drill_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        let Some(nearest) = nearest_drill_edge_clearance(drill, &board_edges) else {
            validation_input_missing(
                findings,
                scenario,
                "DRILL_TO_BOARD_EDGE_CLEARANCE_VALID could not compute finite drill-to-board-edge clearance.",
            );
            continue;
        };
        if nearest.clearance_mm + f64::EPSILON < min_clearance_mm {
            findings.push(drill_edge_clearance_finding(
                scenario,
                drill,
                drill_index,
                nearest,
                min_clearance_mm,
            ));
        }
    }
}

pub(super) fn validate_drill_annular_ring(
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
        let mut best_candidate = None;
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
            if best_candidate
                .as_ref()
                .is_none_or(|best: &DrillAnnularRingCandidate<'_>| {
                    candidate.annular_ring_mm > best.annular_ring_mm
                })
            {
                best_candidate = Some(candidate);
            }
        }
        let Some(best_candidate) = best_candidate else {
            findings.push(drill_annular_ring_missing_finding(
                scenario,
                drill,
                drill_index,
                min_annular_ring_mm,
                max_center_offset_mm,
            ));
            continue;
        };
        if best_candidate.annular_ring_mm + f64::EPSILON < min_annular_ring_mm {
            findings.push(drill_annular_ring_finding(
                scenario,
                drill,
                drill_index,
                best_candidate,
                min_annular_ring_mm,
                max_center_offset_mm,
            ));
        }
    }
}

pub(super) fn validate_copper_to_board_edge_clearance(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(min_clearance_mm) =
        required_numeric_parameter(scenario, "min_copper_edge_clearance_mm", findings)
    else {
        return;
    };
    if min_clearance_mm < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.min_copper_edge_clearance_mm must be greater than or equal to zero.",
        );
        return;
    }
    let copper = &bound.project.board.layout.copper;
    if copper.features.is_empty() && copper.segments.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "COPPER_TO_BOARD_EDGE_CLEARANCE_VALID requires board.layout.copper.features or board.layout.copper.segments evidence.",
        );
        return;
    }
    let board_edges = bound
        .project
        .board
        .layout
        .outline
        .segments
        .iter()
        .filter(|segment| usable_outline_segment(segment))
        .collect::<Vec<_>>();
    if board_edges.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "COPPER_TO_BOARD_EDGE_CLEARANCE_VALID requires usable board.layout.outline.segments evidence.",
        );
        return;
    }
    for (feature_index, feature) in copper.features.iter().enumerate() {
        if let Err(message) = validate_copper_feature_geometry(feature, feature_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        let Some(nearest) = nearest_copper_feature_edge_clearance(feature, &board_edges) else {
            validation_input_missing(
                findings,
                scenario,
                "COPPER_TO_BOARD_EDGE_CLEARANCE_VALID could not compute finite copper feature-to-board-edge clearance.",
            );
            continue;
        };
        if nearest.clearance_mm + f64::EPSILON < min_clearance_mm {
            findings.push(copper_feature_edge_clearance_finding(
                scenario,
                feature,
                feature_index,
                nearest,
                min_clearance_mm,
            ));
        }
    }
    for (segment_index, segment) in copper.segments.iter().enumerate() {
        if let Err(message) = validate_copper_segment_geometry(segment, segment_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        let Some(nearest) = nearest_copper_segment_edge_clearance(segment, &board_edges) else {
            validation_input_missing(
                findings,
                scenario,
                "COPPER_TO_BOARD_EDGE_CLEARANCE_VALID could not compute finite copper segment-to-board-edge clearance.",
            );
            continue;
        };
        if nearest.clearance_mm + f64::EPSILON < min_clearance_mm {
            findings.push(copper_segment_edge_clearance_finding(
                scenario,
                segment,
                segment_index,
                nearest,
                min_clearance_mm,
            ));
        }
    }
}

pub(super) fn validate_copper_spacing(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(min_spacing_mm) =
        required_numeric_parameter(scenario, "min_copper_spacing_mm", findings)
    else {
        return;
    };
    if min_spacing_mm < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.min_copper_spacing_mm must be greater than or equal to zero.",
        );
        return;
    }
    let copper = &bound.project.board.layout.copper;
    if copper.features.len() + copper.segments.len() < 2 {
        validation_input_missing(
            findings,
            scenario,
            "COPPER_SPACING_VALID requires at least two board.layout.copper features or segments.",
        );
        return;
    }
    for (first_index, first_feature) in copper.features.iter().enumerate() {
        if let Err(message) = validate_copper_feature_geometry(first_feature, first_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        for (second_index, second_feature) in
            copper.features.iter().enumerate().skip(first_index + 1)
        {
            if let Err(message) = validate_copper_feature_geometry(second_feature, second_index) {
                validation_input_missing(findings, scenario, message);
                continue;
            }
            maybe_report_copper_spacing(
                scenario,
                findings,
                CopperObjectRef::Feature {
                    feature: first_feature,
                    index: first_index,
                },
                CopperObjectRef::Feature {
                    feature: second_feature,
                    index: second_index,
                },
                min_spacing_mm,
            );
        }
        for (second_index, second_segment) in copper.segments.iter().enumerate() {
            if let Err(message) = validate_copper_segment_geometry(second_segment, second_index) {
                validation_input_missing(findings, scenario, message);
                continue;
            }
            maybe_report_copper_spacing(
                scenario,
                findings,
                CopperObjectRef::Feature {
                    feature: first_feature,
                    index: first_index,
                },
                CopperObjectRef::Segment {
                    segment: second_segment,
                    index: second_index,
                },
                min_spacing_mm,
            );
        }
    }
    for (first_index, first_segment) in copper.segments.iter().enumerate() {
        if let Err(message) = validate_copper_segment_geometry(first_segment, first_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        for (second_index, second_segment) in
            copper.segments.iter().enumerate().skip(first_index + 1)
        {
            if let Err(message) = validate_copper_segment_geometry(second_segment, second_index) {
                validation_input_missing(findings, scenario, message);
                continue;
            }
            maybe_report_copper_spacing(
                scenario,
                findings,
                CopperObjectRef::Segment {
                    segment: first_segment,
                    index: first_index,
                },
                CopperObjectRef::Segment {
                    segment: second_segment,
                    index: second_index,
                },
                min_spacing_mm,
            );
        }
    }
}

fn required_numeric_parameter(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    if !scenario.parameters.contains_key(name) {
        validation_input_missing(
            findings,
            scenario,
            format!("manufacturing parameters.{name} is required."),
        );
        return None;
    }
    let Some(value) = scenario
        .parameters
        .get(name)
        .and_then(serde_yaml_ng::Value::as_f64)
    else {
        validation_input_missing(
            findings,
            scenario,
            format!("manufacturing parameters.{name} must be numeric."),
        );
        return None;
    };
    if !value.is_finite() {
        validation_input_missing(
            findings,
            scenario,
            format!("manufacturing parameters.{name} must be finite."),
        );
        return None;
    }
    Some(value)
}

fn optional_numeric_parameter(
    scenario: &Scenario,
    name: &str,
    default: f64,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    if !scenario.parameters.contains_key(name) {
        return Some(default);
    }
    let Some(value) = scenario
        .parameters
        .get(name)
        .and_then(serde_yaml_ng::Value::as_f64)
    else {
        validation_input_missing(
            findings,
            scenario,
            format!("manufacturing parameters.{name} must be numeric."),
        );
        return None;
    };
    if !value.is_finite() {
        validation_input_missing(
            findings,
            scenario,
            format!("manufacturing parameters.{name} must be finite."),
        );
        return None;
    }
    Some(value)
}

#[derive(Debug, Clone, Copy)]
struct DrillAnnularRingCandidate<'a> {
    feature: &'a LayoutCopperFeature,
    feature_index: usize,
    center_offset_mm: f64,
    annular_ring_mm: f64,
}

fn maybe_report_copper_spacing(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    first: CopperObjectRef<'_>,
    second: CopperObjectRef<'_>,
    min_spacing_mm: f64,
) {
    if first.layer() != second.layer() {
        return;
    }
    let Some(clearance_mm) = copper_object_spacing_mm(first, second) else {
        validation_input_missing(
            findings,
            scenario,
            "COPPER_SPACING_VALID could not compute finite copper-to-copper spacing for supported Gerber copper geometry.",
        );
        return;
    };
    if clearance_mm <= f64::EPSILON {
        return;
    }
    if clearance_mm + f64::EPSILON < min_spacing_mm {
        findings.push(copper_spacing_finding(
            scenario,
            first,
            second,
            clearance_mm,
            min_spacing_mm,
        ));
    }
}

fn drill_edge_clearance_finding(
    scenario: &Scenario,
    drill: &LayoutDrill,
    drill_index: usize,
    nearest: DrillEdgeClearance<'_>,
    min_clearance_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        DRILL_TO_BOARD_EDGE_CLEARANCE_VALID,
        &scenario.name,
        format!(
            "Drill hit {} has {:.3} mm edge-to-board clearance, below {:.3} mm minimum.",
            drill_index, nearest.clearance_mm, min_clearance_mm
        ),
    );
    finding
        .measured
        .insert("drill_index".to_string(), json!(drill_index));
    finding
        .measured
        .insert("drill_x_mm".to_string(), json!(drill.at.x_mm));
    finding
        .measured
        .insert("drill_y_mm".to_string(), json!(drill.at.y_mm));
    finding
        .measured
        .insert("drill_mm".to_string(), json!(drill.drill_mm));
    finding
        .measured
        .insert("drill_radius_mm".to_string(), json!(drill.drill_mm / 2.0));
    finding
        .measured
        .insert("clearance_mm".to_string(), json!(nearest.clearance_mm));
    finding.measured.insert(
        "center_to_board_edge_distance_mm".to_string(),
        json!(nearest.center_distance_mm),
    );
    finding
        .measured
        .insert("drill_plating".to_string(), json!(drill.plating));
    if let Some(layer) = &drill.layer {
        finding
            .measured
            .insert("drill_layer".to_string(), json!(layer));
    }
    if let Some(tool) = &drill.tool {
        finding
            .measured
            .insert("drill_tool".to_string(), json!(tool));
    }
    if let Some(source_hit_index) = drill.source_hit_index {
        finding
            .measured
            .insert("source_hit_index".to_string(), json!(source_hit_index));
    }
    finding.measured.insert(
        "board_edge_start".to_string(),
        json!({
            "x_mm": nearest.edge.start.x_mm,
            "y_mm": nearest.edge.start.y_mm,
        }),
    );
    finding.measured.insert(
        "board_edge_end".to_string(),
        json!({
            "x_mm": nearest.edge.end.x_mm,
            "y_mm": nearest.edge.end.y_mm,
        }),
    );
    if let Some(layer) = &nearest.edge.layer {
        finding
            .measured
            .insert("board_edge_layer".to_string(), json!(layer));
    }
    if let Some(source_primitive) = &nearest.edge.source_primitive {
        finding.measured.insert(
            "board_edge_source_primitive".to_string(),
            json!(source_primitive),
        );
    }
    if let Some(source_primitive_index) = nearest.edge.source_primitive_index {
        finding.measured.insert(
            "board_edge_source_primitive_index".to_string(),
            json!(source_primitive_index),
        );
    }
    if let Some(contour_index) = nearest.edge.contour_index {
        finding
            .measured
            .insert("board_edge_contour_index".to_string(), json!(contour_index));
    }
    if let Some(boundary_role) = &nearest.edge.boundary_role {
        finding
            .measured
            .insert("board_edge_boundary_role".to_string(), json!(boundary_role));
    }
    finding.limit.insert(
        "min_drill_edge_clearance_mm".to_string(),
        json!(min_clearance_mm),
    );
    finding.suggested_fixes = vec![
        "Move the drilled hole farther from the nearest board outline or cutout edge.".to_string(),
        "Reduce the drill diameter only if the mechanical/electrical requirement allows it."
            .to_string(),
        "Adjust the board outline or slot geometry if the fabrication drawing is incorrect."
            .to_string(),
    ];
    finding
}

fn drill_annular_ring_missing_finding(
    scenario: &Scenario,
    drill: &LayoutDrill,
    drill_index: usize,
    min_annular_ring_mm: f64,
    max_center_offset_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        DRILL_ANNULAR_RING_VALID,
        &scenario.name,
        format!(
            "Plated/unknown drill hit {} has no co-located Gerber copper flash evidence within {:.3} mm.",
            drill_index, max_center_offset_mm
        ),
    );
    insert_drill_measurements(&mut finding, drill, drill_index);
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

fn drill_annular_ring_finding(
    scenario: &Scenario,
    drill: &LayoutDrill,
    drill_index: usize,
    candidate: DrillAnnularRingCandidate<'_>,
    min_annular_ring_mm: f64,
    max_center_offset_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        DRILL_ANNULAR_RING_VALID,
        &scenario.name,
        format!(
            "Drill hit {} has {:.3} mm annular ring, below {:.3} mm minimum.",
            drill_index, candidate.annular_ring_mm, min_annular_ring_mm
        ),
    );
    insert_drill_measurements(&mut finding, drill, drill_index);
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

fn copper_feature_edge_clearance_finding(
    scenario: &Scenario,
    feature: &LayoutCopperFeature,
    feature_index: usize,
    nearest: CopperFeatureEdgeClearance<'_>,
    min_clearance_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        COPPER_TO_BOARD_EDGE_CLEARANCE_VALID,
        &scenario.name,
        format!(
            "Gerber copper feature {} has {:.3} mm board-edge clearance, below {:.3} mm minimum.",
            feature_index, nearest.clearance_mm, min_clearance_mm
        ),
    );
    finding
        .measured
        .insert("copper_kind".to_string(), json!("feature"));
    finding
        .measured
        .insert("copper_feature_index".to_string(), json!(feature_index));
    insert_copper_feature_edge_measurements(&mut finding, feature);
    finding
        .measured
        .insert("clearance_mm".to_string(), json!(nearest.clearance_mm));
    insert_board_edge_measurements(&mut finding, nearest.edge);
    finding.limit.insert(
        "min_copper_edge_clearance_mm".to_string(),
        json!(min_clearance_mm),
    );
    finding.suggested_fixes = vec![
        "Move the copper feature farther from the board outline or cutout edge.".to_string(),
        "Reduce copper flash size only if the pad/land requirement allows it.".to_string(),
        "Adjust the board outline or copper Gerber origin if fabrication layers are misregistered."
            .to_string(),
    ];
    finding
}

fn copper_segment_edge_clearance_finding(
    scenario: &Scenario,
    segment: &LayoutCopperSegment,
    segment_index: usize,
    nearest: CopperSegmentEdgeClearance<'_>,
    min_clearance_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        COPPER_TO_BOARD_EDGE_CLEARANCE_VALID,
        &scenario.name,
        format!(
            "Gerber copper segment {} has {:.3} mm board-edge clearance, below {:.3} mm minimum.",
            segment_index, nearest.clearance_mm, min_clearance_mm
        ),
    );
    finding
        .measured
        .insert("copper_kind".to_string(), json!("segment"));
    finding
        .measured
        .insert("copper_segment_index".to_string(), json!(segment_index));
    insert_copper_segment_measurements(&mut finding, segment);
    finding
        .measured
        .insert("clearance_mm".to_string(), json!(nearest.clearance_mm));
    finding.measured.insert(
        "trace_centerline_to_board_edge_distance_mm".to_string(),
        json!(nearest.centerline_distance_mm),
    );
    insert_board_edge_measurements(&mut finding, nearest.edge);
    finding.limit.insert(
        "min_copper_edge_clearance_mm".to_string(),
        json!(min_clearance_mm),
    );
    finding.suggested_fixes = vec![
        "Move or reroute the copper segment farther from the board outline or cutout edge."
            .to_string(),
        "Reduce trace width only if current capacity, impedance, and fabrication rules allow it."
            .to_string(),
        "Adjust the board outline or copper Gerber origin if fabrication layers are misregistered."
            .to_string(),
    ];
    finding
}

fn copper_spacing_finding(
    scenario: &Scenario,
    first: CopperObjectRef<'_>,
    second: CopperObjectRef<'_>,
    clearance_mm: f64,
    min_spacing_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        COPPER_SPACING_VALID,
        &scenario.name,
        format!(
            "Gerber copper {} and {} have {:.3} mm same-layer spacing, below {:.3} mm minimum.",
            first.kind(),
            second.kind(),
            clearance_mm,
            min_spacing_mm
        ),
    );
    insert_copper_object_measurements(&mut finding, "first", first);
    insert_copper_object_measurements(&mut finding, "second", second);
    finding
        .measured
        .insert("clearance_mm".to_string(), json!(clearance_mm));
    finding
        .measured
        .insert("copper_layer".to_string(), json!(first.layer()));
    finding
        .limit
        .insert("min_copper_spacing_mm".to_string(), json!(min_spacing_mm));
    finding.suggested_fixes = vec![
        "Increase spacing between the same-layer copper objects in the Gerber output."
            .to_string(),
        "Move pads or reroute traces to satisfy the fabrication copper-spacing rule.".to_string(),
        "If the copper objects are intentionally connected, use net-aware PCB evidence instead of anonymous Gerber spacing for sign-off.".to_string(),
    ];
    finding
}

fn insert_drill_measurements(finding: &mut Finding, drill: &LayoutDrill, drill_index: usize) {
    finding
        .measured
        .insert("drill_index".to_string(), json!(drill_index));
    finding
        .measured
        .insert("drill_x_mm".to_string(), json!(drill.at.x_mm));
    finding
        .measured
        .insert("drill_y_mm".to_string(), json!(drill.at.y_mm));
    finding
        .measured
        .insert("drill_mm".to_string(), json!(drill.drill_mm));
    finding
        .measured
        .insert("drill_radius_mm".to_string(), json!(drill.drill_mm / 2.0));
    finding
        .measured
        .insert("drill_plating".to_string(), json!(drill.plating));
    if let Some(layer) = &drill.layer {
        finding
            .measured
            .insert("drill_layer".to_string(), json!(layer));
    }
    if let Some(tool) = &drill.tool {
        finding
            .measured
            .insert("drill_tool".to_string(), json!(tool));
    }
    if let Some(source_hit_index) = drill.source_hit_index {
        finding
            .measured
            .insert("source_hit_index".to_string(), json!(source_hit_index));
    }
}

fn insert_copper_feature_edge_measurements(finding: &mut Finding, feature: &LayoutCopperFeature) {
    finding
        .measured
        .insert("copper_feature_x_mm".to_string(), json!(feature.at.x_mm));
    finding
        .measured
        .insert("copper_feature_y_mm".to_string(), json!(feature.at.y_mm));
    finding
        .measured
        .insert("copper_feature_layer".to_string(), json!(feature.layer));
    finding.measured.insert(
        "copper_feature_aperture".to_string(),
        json!(feature.aperture),
    );
    finding
        .measured
        .insert("copper_feature_shape".to_string(), json!(feature.shape));
    finding.measured.insert(
        "copper_feature_size_x_mm".to_string(),
        json!(feature.size.x_mm),
    );
    finding.measured.insert(
        "copper_feature_size_y_mm".to_string(),
        json!(feature.size.y_mm),
    );
    finding.measured.insert(
        "copper_feature_source_primitive".to_string(),
        json!(feature.source_primitive),
    );
    finding.measured.insert(
        "copper_feature_source_primitive_index".to_string(),
        json!(feature.source_primitive_index),
    );
}

fn insert_copper_segment_measurements(finding: &mut Finding, segment: &LayoutCopperSegment) {
    finding.measured.insert(
        "copper_segment_start".to_string(),
        json!({
            "x_mm": segment.start.x_mm,
            "y_mm": segment.start.y_mm,
        }),
    );
    finding.measured.insert(
        "copper_segment_end".to_string(),
        json!({
            "x_mm": segment.end.x_mm,
            "y_mm": segment.end.y_mm,
        }),
    );
    finding
        .measured
        .insert("copper_segment_layer".to_string(), json!(segment.layer));
    finding.measured.insert(
        "copper_segment_aperture".to_string(),
        json!(segment.aperture),
    );
    finding.measured.insert(
        "copper_segment_width_mm".to_string(),
        json!(segment.width_mm),
    );
    finding.measured.insert(
        "copper_segment_source_primitive".to_string(),
        json!(segment.source_primitive),
    );
    finding.measured.insert(
        "copper_segment_source_primitive_index".to_string(),
        json!(segment.source_primitive_index),
    );
}

fn insert_copper_object_measurements(
    finding: &mut Finding,
    prefix: &str,
    object: CopperObjectRef<'_>,
) {
    finding
        .measured
        .insert(format!("{prefix}_copper_kind"), json!(object.kind()));
    match object {
        CopperObjectRef::Feature { feature, index } => {
            finding
                .measured
                .insert(format!("{prefix}_copper_feature_index"), json!(index));
            finding.measured.insert(
                format!("{prefix}_copper_feature_x_mm"),
                json!(feature.at.x_mm),
            );
            finding.measured.insert(
                format!("{prefix}_copper_feature_y_mm"),
                json!(feature.at.y_mm),
            );
            finding.measured.insert(
                format!("{prefix}_copper_feature_layer"),
                json!(feature.layer),
            );
            finding.measured.insert(
                format!("{prefix}_copper_feature_aperture"),
                json!(feature.aperture),
            );
            finding.measured.insert(
                format!("{prefix}_copper_feature_shape"),
                json!(feature.shape),
            );
            finding.measured.insert(
                format!("{prefix}_copper_feature_size_x_mm"),
                json!(feature.size.x_mm),
            );
            finding.measured.insert(
                format!("{prefix}_copper_feature_size_y_mm"),
                json!(feature.size.y_mm),
            );
            finding.measured.insert(
                format!("{prefix}_copper_feature_source_primitive"),
                json!(feature.source_primitive),
            );
            finding.measured.insert(
                format!("{prefix}_copper_feature_source_primitive_index"),
                json!(feature.source_primitive_index),
            );
        }
        CopperObjectRef::Segment { segment, index } => {
            finding
                .measured
                .insert(format!("{prefix}_copper_segment_index"), json!(index));
            finding.measured.insert(
                format!("{prefix}_copper_segment_start"),
                json!({
                    "x_mm": segment.start.x_mm,
                    "y_mm": segment.start.y_mm,
                }),
            );
            finding.measured.insert(
                format!("{prefix}_copper_segment_end"),
                json!({
                    "x_mm": segment.end.x_mm,
                    "y_mm": segment.end.y_mm,
                }),
            );
            finding.measured.insert(
                format!("{prefix}_copper_segment_layer"),
                json!(segment.layer),
            );
            finding.measured.insert(
                format!("{prefix}_copper_segment_aperture"),
                json!(segment.aperture),
            );
            finding.measured.insert(
                format!("{prefix}_copper_segment_width_mm"),
                json!(segment.width_mm),
            );
            finding.measured.insert(
                format!("{prefix}_copper_segment_source_primitive"),
                json!(segment.source_primitive),
            );
            finding.measured.insert(
                format!("{prefix}_copper_segment_source_primitive_index"),
                json!(segment.source_primitive_index),
            );
        }
    }
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

fn insert_board_edge_measurements(finding: &mut Finding, edge: &LayoutSegment) {
    finding.measured.insert(
        "board_edge_start".to_string(),
        json!({
            "x_mm": edge.start.x_mm,
            "y_mm": edge.start.y_mm,
        }),
    );
    finding.measured.insert(
        "board_edge_end".to_string(),
        json!({
            "x_mm": edge.end.x_mm,
            "y_mm": edge.end.y_mm,
        }),
    );
    if let Some(layer) = &edge.layer {
        finding
            .measured
            .insert("board_edge_layer".to_string(), json!(layer));
    }
    if let Some(source_primitive) = &edge.source_primitive {
        finding.measured.insert(
            "board_edge_source_primitive".to_string(),
            json!(source_primitive),
        );
    }
    if let Some(source_primitive_index) = edge.source_primitive_index {
        finding.measured.insert(
            "board_edge_source_primitive_index".to_string(),
            json!(source_primitive_index),
        );
    }
    if let Some(contour_index) = edge.contour_index {
        finding
            .measured
            .insert("board_edge_contour_index".to_string(), json!(contour_index));
    }
    if let Some(boundary_role) = &edge.boundary_role {
        finding
            .measured
            .insert("board_edge_boundary_role".to_string(), json!(boundary_role));
    }
}
