mod annular_ring;
mod geometry;

use crate::board_ir::{
    LayoutCopperFeature, LayoutCopperRegion, LayoutCopperSegment, LayoutDrill, LayoutSegment,
    Scenario,
};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use self::geometry::{
    CopperFeatureEdgeClearance, CopperObjectRef, CopperRegionEdgeClearance,
    CopperSegmentEdgeClearance, DrillEdgeClearance, copper_object_spacing_mm,
    nearest_copper_feature_edge_clearance, nearest_copper_region_edge_clearance,
    nearest_copper_segment_edge_clearance, nearest_drill_edge_clearance, point_distance_mm,
    usable_outline_segment, validate_copper_feature_geometry, validate_copper_region_geometry,
    validate_copper_segment_geometry, validate_drill_geometry,
};
use super::COPPER_SPACING_VALID;
use super::COPPER_TO_BOARD_EDGE_CLEARANCE_VALID;
use super::DRILL_TO_BOARD_EDGE_CLEARANCE_VALID;
use super::SOLDER_MASK_DAM_VALID;
use super::SOLDER_MASK_OPENING_VALID;
use super::common::validation_input_missing;

pub(super) use annular_ring::validate_drill_annular_ring;

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
    if copper.features.is_empty() && copper.segments.is_empty() && copper.regions.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "COPPER_TO_BOARD_EDGE_CLEARANCE_VALID requires board.layout.copper.features, board.layout.copper.segments, or board.layout.copper.regions evidence.",
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
    for (region_index, region) in copper.regions.iter().enumerate() {
        if let Err(message) = validate_copper_region_geometry(region, region_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        let Some(nearest) = nearest_copper_region_edge_clearance(region, &board_edges) else {
            validation_input_missing(
                findings,
                scenario,
                "COPPER_TO_BOARD_EDGE_CLEARANCE_VALID could not compute finite copper region-to-board-edge clearance.",
            );
            continue;
        };
        if nearest.clearance_mm + f64::EPSILON < min_clearance_mm {
            findings.push(copper_region_edge_clearance_finding(
                scenario,
                region,
                region_index,
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
    if copper.features.len() + copper.segments.len() + copper.regions.len() < 2 {
        validation_input_missing(
            findings,
            scenario,
            "COPPER_SPACING_VALID requires at least two board.layout.copper features, segments, or regions.",
        );
        return;
    }
    let mut copper_objects = Vec::new();
    for (first_index, first_feature) in copper.features.iter().enumerate() {
        if let Err(message) = validate_copper_feature_geometry(first_feature, first_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        copper_objects.push(CopperObjectRef::Feature {
            feature: first_feature,
            index: first_index,
        });
    }
    for (segment_index, segment) in copper.segments.iter().enumerate() {
        if let Err(message) = validate_copper_segment_geometry(segment, segment_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        copper_objects.push(CopperObjectRef::Segment {
            segment,
            index: segment_index,
        });
    }
    for (region_index, region) in copper.regions.iter().enumerate() {
        if let Err(message) = validate_copper_region_geometry(region, region_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        copper_objects.push(CopperObjectRef::Region {
            region,
            index: region_index,
        });
    }
    for (first_index, first_object) in copper_objects.iter().enumerate() {
        for second_object in copper_objects.iter().skip(first_index + 1) {
            maybe_report_copper_spacing(
                scenario,
                findings,
                *first_object,
                *second_object,
                min_spacing_mm,
            );
        }
    }
}

pub(super) fn validate_solder_mask_opening(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(min_expansion_mm) =
        required_numeric_parameter(scenario, "min_mask_expansion_mm", findings)
    else {
        return;
    };
    if min_expansion_mm < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.min_mask_expansion_mm must be greater than or equal to zero.",
        );
        return;
    }
    let Some(max_center_offset_mm) = optional_numeric_parameter(
        scenario,
        "max_copper_to_mask_center_offset_mm",
        0.1,
        findings,
    ) else {
        return;
    };
    if max_center_offset_mm < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.max_copper_to_mask_center_offset_mm must be greater than or equal to zero.",
        );
        return;
    }
    let copper = &bound.project.board.layout.copper;
    if copper.features.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "SOLDER_MASK_OPENING_VALID requires board.layout.copper.features evidence.",
        );
        return;
    }
    let mask = &bound.project.board.layout.solder_mask;
    if mask.features.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "SOLDER_MASK_OPENING_VALID requires board.layout.solder_mask.features evidence.",
        );
        return;
    }
    for (copper_index, copper_feature) in copper.features.iter().enumerate() {
        if let Err(message) = validate_copper_feature_geometry(copper_feature, copper_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        let Some(mask_layer) = solder_mask_layer_for_copper_layer(&copper_feature.layer) else {
            continue;
        };
        let mut best_candidate: Option<SolderMaskOpeningCandidate<'_>> = None;
        for (mask_index, mask_feature) in mask.features.iter().enumerate() {
            if let Err(message) = validate_copper_feature_geometry(mask_feature, mask_index) {
                validation_input_missing(findings, scenario, message);
                continue;
            }
            if mask_feature.layer != mask_layer {
                continue;
            }
            let center_offset_mm = point_distance_mm(&copper_feature.at, &mask_feature.at);
            if center_offset_mm > max_center_offset_mm {
                continue;
            }
            let expansion_x_mm = (mask_feature.size.x_mm - copper_feature.size.x_mm) / 2.0;
            let expansion_y_mm = (mask_feature.size.y_mm - copper_feature.size.y_mm) / 2.0;
            let min_expansion_found_mm = expansion_x_mm.min(expansion_y_mm);
            let candidate = SolderMaskOpeningCandidate {
                mask_feature,
                mask_index,
                center_offset_mm,
                expansion_x_mm,
                expansion_y_mm,
                min_expansion_found_mm,
            };
            if best_candidate.is_none_or(|best| {
                candidate.min_expansion_found_mm > best.min_expansion_found_mm
                    || (candidate.min_expansion_found_mm == best.min_expansion_found_mm
                        && candidate.center_offset_mm < best.center_offset_mm)
            }) {
                best_candidate = Some(candidate);
            }
        }
        match best_candidate {
            Some(candidate)
                if candidate.min_expansion_found_mm + f64::EPSILON < min_expansion_mm =>
            {
                findings.push(solder_mask_opening_undersized_finding(
                    scenario,
                    copper_feature,
                    copper_index,
                    candidate,
                    min_expansion_mm,
                    max_center_offset_mm,
                ));
            }
            None => findings.push(solder_mask_opening_missing_finding(
                scenario,
                copper_feature,
                copper_index,
                mask_layer,
                min_expansion_mm,
                max_center_offset_mm,
            )),
            _ => {}
        }
    }
}

pub(super) fn validate_solder_mask_dam(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(min_dam_mm) = required_numeric_parameter(scenario, "min_solder_mask_dam_mm", findings)
    else {
        return;
    };
    if min_dam_mm < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.min_solder_mask_dam_mm must be greater than or equal to zero.",
        );
        return;
    }
    let mask = &bound.project.board.layout.solder_mask;
    if mask.features.len() < 2 {
        validation_input_missing(
            findings,
            scenario,
            "SOLDER_MASK_DAM_VALID requires at least two board.layout.solder_mask.features entries.",
        );
        return;
    }
    let mut mask_features = Vec::new();
    for (feature_index, feature) in mask.features.iter().enumerate() {
        if let Err(message) = validate_copper_feature_geometry(feature, feature_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        mask_features.push((feature_index, feature));
    }
    for (first_offset, (first_index, first_feature)) in mask_features.iter().enumerate() {
        for (second_index, second_feature) in mask_features.iter().skip(first_offset + 1) {
            if first_feature.layer != second_feature.layer {
                continue;
            }
            let Some(dam_width_mm) = copper_object_spacing_mm(
                CopperObjectRef::Feature {
                    feature: first_feature,
                    index: *first_index,
                },
                CopperObjectRef::Feature {
                    feature: second_feature,
                    index: *second_index,
                },
            ) else {
                validation_input_missing(
                    findings,
                    scenario,
                    "SOLDER_MASK_DAM_VALID could not compute finite solder-mask opening spacing for supported Gerber mask geometry.",
                );
                return;
            };
            if dam_width_mm + f64::EPSILON < min_dam_mm {
                findings.push(solder_mask_dam_finding(
                    scenario,
                    first_feature,
                    *first_index,
                    second_feature,
                    *second_index,
                    dam_width_mm.max(0.0),
                    min_dam_mm,
                ));
            }
        }
    }
}

#[derive(Clone, Copy)]
struct SolderMaskOpeningCandidate<'a> {
    mask_feature: &'a LayoutCopperFeature,
    mask_index: usize,
    center_offset_mm: f64,
    expansion_x_mm: f64,
    expansion_y_mm: f64,
    min_expansion_found_mm: f64,
}

fn solder_mask_layer_for_copper_layer(copper_layer: &str) -> Option<&'static str> {
    match copper_layer {
        "F.Cu" => Some("F.Mask"),
        "B.Cu" => Some("B.Mask"),
        _ => None,
    }
}

fn solder_mask_opening_missing_finding(
    scenario: &Scenario,
    copper_feature: &LayoutCopperFeature,
    copper_index: usize,
    expected_mask_layer: &str,
    min_expansion_mm: f64,
    max_center_offset_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        SOLDER_MASK_OPENING_VALID,
        scenario.name.clone(),
        format!(
            "Copper flash {copper_index} on {} has no co-located solder-mask opening on {expected_mask_layer}.",
            copper_feature.layer
        ),
    );
    finding.suggested_fixes = vec![
        "Add or restore a solder-mask opening over this copper pad/via, or verify the fabrication export did not omit the mask aperture.".to_string(),
    ];
    finding
        .measured
        .insert("copper_feature_index".to_string(), json!(copper_index));
    insert_copper_feature_edge_measurements(&mut finding, copper_feature);
    finding.measured.insert(
        "expected_solder_mask_layer".to_string(),
        json!(expected_mask_layer),
    );
    finding
        .limit
        .insert("min_mask_expansion_mm".to_string(), json!(min_expansion_mm));
    finding.limit.insert(
        "max_copper_to_mask_center_offset_mm".to_string(),
        json!(max_center_offset_mm),
    );
    finding
}

fn solder_mask_opening_undersized_finding(
    scenario: &Scenario,
    copper_feature: &LayoutCopperFeature,
    copper_index: usize,
    candidate: SolderMaskOpeningCandidate<'_>,
    min_expansion_mm: f64,
    max_center_offset_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        SOLDER_MASK_OPENING_VALID,
        scenario.name.clone(),
        format!(
            "Solder-mask opening {} on {} expands copper flash {copper_index} by only {:.6} mm; required at least {:.6} mm.",
            candidate.mask_index,
            candidate.mask_feature.layer,
            candidate.min_expansion_found_mm,
            min_expansion_mm
        ),
    );
    finding.suggested_fixes = vec![
        "Increase the solder-mask aperture expansion or restore the PCB tool's intended solder-mask clearance for this pad/via.".to_string(),
    ];
    finding
        .measured
        .insert("copper_feature_index".to_string(), json!(copper_index));
    insert_copper_feature_edge_measurements(&mut finding, copper_feature);
    insert_solder_mask_feature_measurements(&mut finding, candidate);
    finding
        .limit
        .insert("min_mask_expansion_mm".to_string(), json!(min_expansion_mm));
    finding.limit.insert(
        "max_copper_to_mask_center_offset_mm".to_string(),
        json!(max_center_offset_mm),
    );
    finding.measured.insert(
        "measured_mask_expansion_x_mm".to_string(),
        json!(candidate.expansion_x_mm),
    );
    finding.measured.insert(
        "measured_mask_expansion_y_mm".to_string(),
        json!(candidate.expansion_y_mm),
    );
    finding.measured.insert(
        "measured_min_mask_expansion_mm".to_string(),
        json!(candidate.min_expansion_found_mm),
    );
    finding.measured.insert(
        "copper_to_mask_center_offset_mm".to_string(),
        json!(candidate.center_offset_mm),
    );
    finding
}

fn solder_mask_dam_finding(
    scenario: &Scenario,
    first_feature: &LayoutCopperFeature,
    first_index: usize,
    second_feature: &LayoutCopperFeature,
    second_index: usize,
    dam_width_mm: f64,
    min_dam_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        SOLDER_MASK_DAM_VALID,
        scenario.name.clone(),
        format!(
            "Solder-mask openings {first_index} and {second_index} on {} leave only {:.6} mm mask dam; required at least {:.6} mm.",
            first_feature.layer, dam_width_mm, min_dam_mm
        ),
    );
    finding.suggested_fixes = vec![
        "Increase the solder-mask dam by reducing mask expansion, increasing pad spacing, or using a package/fabrication process that supports the smaller mask web.".to_string(),
        "If the mask bridge is intentionally removed for fine-pitch pads, record that fabrication rule explicitly and adjust this scenario threshold.".to_string(),
    ];
    finding
        .measured
        .insert("solder_mask_layer".to_string(), json!(first_feature.layer));
    insert_prefixed_solder_mask_feature_measurements(
        &mut finding,
        "first",
        first_feature,
        first_index,
    );
    insert_prefixed_solder_mask_feature_measurements(
        &mut finding,
        "second",
        second_feature,
        second_index,
    );
    finding
        .measured
        .insert("solder_mask_dam_width_mm".to_string(), json!(dam_width_mm));
    finding
        .limit
        .insert("min_solder_mask_dam_mm".to_string(), json!(min_dam_mm));
    finding
}

fn insert_solder_mask_feature_measurements(
    finding: &mut Finding,
    candidate: SolderMaskOpeningCandidate<'_>,
) {
    let feature = candidate.mask_feature;
    finding.measured.insert(
        "solder_mask_feature_index".to_string(),
        json!(candidate.mask_index),
    );
    finding.measured.insert(
        "solder_mask_feature_x_mm".to_string(),
        json!(feature.at.x_mm),
    );
    finding.measured.insert(
        "solder_mask_feature_y_mm".to_string(),
        json!(feature.at.y_mm),
    );
    finding.measured.insert(
        "solder_mask_feature_layer".to_string(),
        json!(feature.layer),
    );
    finding.measured.insert(
        "solder_mask_feature_aperture".to_string(),
        json!(feature.aperture),
    );
    finding.measured.insert(
        "solder_mask_feature_shape".to_string(),
        json!(feature.shape),
    );
    finding.measured.insert(
        "solder_mask_feature_size_x_mm".to_string(),
        json!(feature.size.x_mm),
    );
    finding.measured.insert(
        "solder_mask_feature_size_y_mm".to_string(),
        json!(feature.size.y_mm),
    );
    finding.measured.insert(
        "solder_mask_feature_source_primitive".to_string(),
        json!(feature.source_primitive),
    );
    finding.measured.insert(
        "solder_mask_feature_source_primitive_index".to_string(),
        json!(feature.source_primitive_index),
    );
}

fn insert_prefixed_solder_mask_feature_measurements(
    finding: &mut Finding,
    prefix: &str,
    feature: &LayoutCopperFeature,
    feature_index: usize,
) {
    finding.measured.insert(
        format!("{prefix}_solder_mask_feature_index"),
        json!(feature_index),
    );
    finding.measured.insert(
        format!("{prefix}_solder_mask_feature_x_mm"),
        json!(feature.at.x_mm),
    );
    finding.measured.insert(
        format!("{prefix}_solder_mask_feature_y_mm"),
        json!(feature.at.y_mm),
    );
    finding.measured.insert(
        format!("{prefix}_solder_mask_feature_layer"),
        json!(feature.layer),
    );
    finding.measured.insert(
        format!("{prefix}_solder_mask_feature_aperture"),
        json!(feature.aperture),
    );
    finding.measured.insert(
        format!("{prefix}_solder_mask_feature_shape"),
        json!(feature.shape),
    );
    finding.measured.insert(
        format!("{prefix}_solder_mask_feature_size_x_mm"),
        json!(feature.size.x_mm),
    );
    finding.measured.insert(
        format!("{prefix}_solder_mask_feature_size_y_mm"),
        json!(feature.size.y_mm),
    );
    finding.measured.insert(
        format!("{prefix}_solder_mask_feature_source_primitive"),
        json!(feature.source_primitive),
    );
    finding.measured.insert(
        format!("{prefix}_solder_mask_feature_source_primitive_index"),
        json!(feature.source_primitive_index),
    );
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
    if copper_objects_share_owner(first, second) {
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
        if copper_objects_conflict(first, second) {
            findings.push(copper_spacing_finding(
                scenario,
                first,
                second,
                0.0,
                min_spacing_mm,
            ));
        }
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

fn copper_objects_share_owner(first: CopperObjectRef<'_>, second: CopperObjectRef<'_>) -> bool {
    match (first.net(), second.net()) {
        (Some(first_net), Some(second_net)) => first_net == second_net,
        (Some(_), None) | (None, Some(_)) => false,
        (None, None) => match (first.island_id(), second.island_id()) {
            (Some(first_island), Some(second_island)) => first_island == second_island,
            _ => false,
        },
    }
}

fn copper_objects_conflict(first: CopperObjectRef<'_>, second: CopperObjectRef<'_>) -> bool {
    match (first.net(), second.net()) {
        (Some(first_net), Some(second_net)) => first_net != second_net,
        (Some(_), None) | (None, Some(_)) => false,
        (None, None) => match (first.island_id(), second.island_id()) {
            (Some(first_island), Some(second_island)) => first_island != second_island,
            _ => false,
        },
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
    if let Some(owner_kind) = &drill.owner_kind {
        finding
            .measured
            .insert("drill_owner_kind".to_string(), json!(owner_kind));
    }
    if let Some(net) = &drill.net {
        finding.measured.insert("drill_net".to_string(), json!(net));
    }
    if let Some(component) = &drill.component {
        finding
            .measured
            .insert("drill_component".to_string(), json!(component));
    }
    if let Some(pin) = &drill.pin {
        finding.measured.insert("drill_pin".to_string(), json!(pin));
    }
    if let Some(via_index) = drill.via_index {
        finding
            .measured
            .insert("drill_via_index".to_string(), json!(via_index));
    }
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

fn copper_region_edge_clearance_finding(
    scenario: &Scenario,
    region: &LayoutCopperRegion,
    region_index: usize,
    nearest: CopperRegionEdgeClearance<'_>,
    min_clearance_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        COPPER_TO_BOARD_EDGE_CLEARANCE_VALID,
        &scenario.name,
        format!(
            "Gerber copper region {} has {:.3} mm board-edge clearance, below {:.3} mm minimum.",
            region_index, nearest.clearance_mm, min_clearance_mm
        ),
    );
    finding
        .measured
        .insert("copper_kind".to_string(), json!("region"));
    finding
        .measured
        .insert("copper_region_index".to_string(), json!(region_index));
    insert_copper_region_measurements(&mut finding, region);
    finding
        .measured
        .insert("clearance_mm".to_string(), json!(nearest.clearance_mm));
    insert_board_edge_measurements(&mut finding, nearest.edge);
    finding.limit.insert(
        "min_copper_edge_clearance_mm".to_string(),
        json!(min_clearance_mm),
    );
    finding.suggested_fixes = vec![
        "Move or reshape the copper region farther from the board outline or cutout edge."
            .to_string(),
        "Reduce the polygon pour boundary only if the copper-pour requirement allows it."
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
    if let Some(owner_kind) = &drill.owner_kind {
        finding
            .measured
            .insert("drill_owner_kind".to_string(), json!(owner_kind));
    }
    if let Some(net) = &drill.net {
        finding.measured.insert("drill_net".to_string(), json!(net));
    }
    if let Some(component) = &drill.component {
        finding
            .measured
            .insert("drill_component".to_string(), json!(component));
    }
    if let Some(pin) = &drill.pin {
        finding.measured.insert("drill_pin".to_string(), json!(pin));
    }
    if let Some(via_index) = drill.via_index {
        finding
            .measured
            .insert("drill_via_index".to_string(), json!(via_index));
    }
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
    insert_optional_copper_feature_owner_measurements(finding, "copper_feature", feature);
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
    insert_optional_copper_owner_measurements(
        finding,
        "copper_segment",
        segment.net.as_deref(),
        segment.island_id.as_deref(),
    );
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
            insert_optional_copper_feature_owner_measurements(
                finding,
                &format!("{prefix}_copper_feature"),
                feature,
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
            insert_optional_copper_owner_measurements(
                finding,
                &format!("{prefix}_copper_segment"),
                segment.net.as_deref(),
                segment.island_id.as_deref(),
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
        CopperObjectRef::Region { region, index } => {
            finding
                .measured
                .insert(format!("{prefix}_copper_region_index"), json!(index));
            insert_prefixed_copper_region_measurements(finding, prefix, region);
        }
    }
}

fn insert_optional_copper_owner_measurements(
    finding: &mut Finding,
    prefix: &str,
    net: Option<&str>,
    island_id: Option<&str>,
) {
    if let Some(net) = net {
        finding.measured.insert(format!("{prefix}_net"), json!(net));
    }
    if let Some(island_id) = island_id {
        finding
            .measured
            .insert(format!("{prefix}_island_id"), json!(island_id));
    }
}

fn insert_optional_copper_feature_owner_measurements(
    finding: &mut Finding,
    prefix: &str,
    feature: &LayoutCopperFeature,
) {
    insert_optional_copper_owner_measurements(
        finding,
        prefix,
        feature.net.as_deref(),
        feature.island_id.as_deref(),
    );
    if let Some(owner_kind) = &feature.owner_kind {
        finding
            .measured
            .insert(format!("{prefix}_owner_kind"), json!(owner_kind));
    }
    if let Some(component) = &feature.component {
        finding
            .measured
            .insert(format!("{prefix}_component"), json!(component));
    }
    if let Some(pin) = &feature.pin {
        finding.measured.insert(format!("{prefix}_pin"), json!(pin));
    }
    if let Some(via_index) = feature.via_index {
        finding
            .measured
            .insert(format!("{prefix}_via_index"), json!(via_index));
    }
}

fn insert_copper_region_measurements(finding: &mut Finding, region: &LayoutCopperRegion) {
    finding
        .measured
        .insert("copper_region_layer".to_string(), json!(region.layer));
    insert_optional_copper_owner_measurements(
        finding,
        "copper_region",
        region.net.as_deref(),
        region.island_id.as_deref(),
    );
    finding
        .measured
        .insert("copper_region_polarity".to_string(), json!(region.polarity));
    finding.measured.insert(
        "copper_region_source_primitive".to_string(),
        json!(region.source_primitive),
    );
    finding.measured.insert(
        "copper_region_source_primitive_index".to_string(),
        json!(region.source_primitive_index),
    );
    finding.measured.insert(
        "copper_region_point_count".to_string(),
        json!(region.points.len()),
    );
}

fn insert_prefixed_copper_region_measurements(
    finding: &mut Finding,
    prefix: &str,
    region: &LayoutCopperRegion,
) {
    finding
        .measured
        .insert(format!("{prefix}_copper_region_layer"), json!(region.layer));
    insert_optional_copper_owner_measurements(
        finding,
        &format!("{prefix}_copper_region"),
        region.net.as_deref(),
        region.island_id.as_deref(),
    );
    finding.measured.insert(
        format!("{prefix}_copper_region_polarity"),
        json!(region.polarity),
    );
    finding.measured.insert(
        format!("{prefix}_copper_region_source_primitive"),
        json!(region.source_primitive),
    );
    finding.measured.insert(
        format!("{prefix}_copper_region_source_primitive_index"),
        json!(region.source_primitive_index),
    );
    finding.measured.insert(
        format!("{prefix}_copper_region_point_count"),
        json!(region.points.len()),
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
