use crate::board_ir::{LayoutCopperFeature, LayoutDrill, LayoutPoint, LayoutSegment, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

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

fn validate_drill_geometry(drill: &LayoutDrill, drill_index: usize) -> Result<(), String> {
    if !finite_point(&drill.at) {
        return Err(format!(
            "board.layout.drills[{drill_index}].at must contain finite coordinates."
        ));
    }
    if !drill.drill_mm.is_finite() || drill.drill_mm <= 0.0 {
        return Err(format!(
            "board.layout.drills[{drill_index}].drill_mm must be finite and positive."
        ));
    }
    Ok(())
}

fn validate_copper_feature_geometry(
    feature: &LayoutCopperFeature,
    feature_index: usize,
) -> Result<(), String> {
    if !finite_point(&feature.at) {
        return Err(format!(
            "board.layout.copper.features[{feature_index}].at must contain finite coordinates."
        ));
    }
    if !feature.size.x_mm.is_finite()
        || !feature.size.y_mm.is_finite()
        || feature.size.x_mm <= 0.0
        || feature.size.y_mm <= 0.0
    {
        return Err(format!(
            "board.layout.copper.features[{feature_index}].size must contain finite positive dimensions."
        ));
    }
    Ok(())
}

fn usable_outline_segment(segment: &LayoutSegment) -> bool {
    finite_point(&segment.start)
        && finite_point(&segment.end)
        && point_distance_mm(&segment.start, &segment.end) > f64::EPSILON
}

fn finite_point(point: &LayoutPoint) -> bool {
    point.x_mm.is_finite() && point.y_mm.is_finite()
}

#[derive(Debug, Clone, Copy)]
struct DrillEdgeClearance<'a> {
    edge: &'a LayoutSegment,
    center_distance_mm: f64,
    clearance_mm: f64,
}

#[derive(Debug, Clone, Copy)]
struct DrillAnnularRingCandidate<'a> {
    feature: &'a LayoutCopperFeature,
    feature_index: usize,
    center_offset_mm: f64,
    annular_ring_mm: f64,
}

fn nearest_drill_edge_clearance<'a>(
    drill: &LayoutDrill,
    board_edges: &'a [&LayoutSegment],
) -> Option<DrillEdgeClearance<'a>> {
    let radius_mm = drill.drill_mm / 2.0;
    board_edges
        .iter()
        .filter_map(|edge| {
            let center_distance_mm =
                point_to_segment_distance_mm(&drill.at, &edge.start, &edge.end);
            center_distance_mm
                .is_finite()
                .then_some(DrillEdgeClearance {
                    edge,
                    center_distance_mm,
                    clearance_mm: center_distance_mm - radius_mm,
                })
        })
        .min_by(|first, second| first.clearance_mm.total_cmp(&second.clearance_mm))
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

fn annular_ring_for_feature(drill: &LayoutDrill, feature: &LayoutCopperFeature) -> Option<f64> {
    let drill_radius_mm = drill.drill_mm / 2.0;
    let dx = drill.at.x_mm - feature.at.x_mm;
    let dy = drill.at.y_mm - feature.at.y_mm;
    let copper_boundary_distance_mm = match feature.shape.as_str() {
        "circle" => feature.size.x_mm.min(feature.size.y_mm) / 2.0 - dx.hypot(dy),
        "rect" => {
            let half_x = feature.size.x_mm / 2.0;
            let half_y = feature.size.y_mm / 2.0;
            (half_x - dx.abs()).min(half_y - dy.abs())
        }
        "oval" => oval_boundary_distance_mm(dx, dy, feature.size.x_mm, feature.size.y_mm),
        _ => return None,
    };
    Some(copper_boundary_distance_mm - drill_radius_mm)
}

fn oval_boundary_distance_mm(dx: f64, dy: f64, width_mm: f64, height_mm: f64) -> f64 {
    if width_mm >= height_mm {
        let radius = height_mm / 2.0;
        let segment_half = (width_mm - height_mm) / 2.0;
        if dx.abs() <= segment_half {
            radius - dy.abs()
        } else {
            radius - (dx.abs() - segment_half).hypot(dy)
        }
    } else {
        let radius = width_mm / 2.0;
        let segment_half = (height_mm - width_mm) / 2.0;
        if dy.abs() <= segment_half {
            radius - dx.abs()
        } else {
            radius - dx.hypot(dy.abs() - segment_half)
        }
    }
}

fn point_to_segment_distance_mm(
    point: &LayoutPoint,
    start: &LayoutPoint,
    end: &LayoutPoint,
) -> f64 {
    let dx = end.x_mm - start.x_mm;
    let dy = end.y_mm - start.y_mm;
    let length_squared = dx * dx + dy * dy;
    if length_squared <= f64::EPSILON {
        return point_distance_mm(point, start);
    }
    let t = (((point.x_mm - start.x_mm) * dx + (point.y_mm - start.y_mm) * dy) / length_squared)
        .clamp(0.0, 1.0);
    let projection = LayoutPoint {
        x_mm: start.x_mm + t * dx,
        y_mm: start.y_mm + t * dy,
    };
    point_distance_mm(point, &projection)
}

fn point_distance_mm(first: &LayoutPoint, second: &LayoutPoint) -> f64 {
    (second.x_mm - first.x_mm).hypot(second.y_mm - first.y_mm)
}
