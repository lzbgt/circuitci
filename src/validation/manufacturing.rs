mod annular_ring;
mod geometry;
mod process;
mod solder_mask;
mod solder_paste_bga;
mod solder_paste_ic;

use crate::board_ir::{
    LayoutCopperFeature, LayoutCopperRegion, LayoutCopperSegment, LayoutDrill, LayoutSegment,
    LayoutSlot, Scenario,
};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use self::geometry::{
    CopperFeatureEdgeClearance, CopperObjectRef, CopperRegionEdgeClearance,
    CopperSegmentEdgeClearance, DrillEdgeClearance, SlotEdgeClearance, copper_object_spacing_mm,
    nearest_copper_feature_edge_clearance, nearest_copper_region_edge_clearance,
    nearest_copper_segment_edge_clearance, nearest_drill_edge_clearance,
    nearest_slot_edge_clearance, usable_outline_segment, validate_copper_feature_geometry,
    validate_copper_region_geometry, validate_copper_segment_geometry, validate_drill_geometry,
    validate_slot_geometry,
};
use self::process::{
    explicit_numeric_parameter, optional_numeric_parameter, required_numeric_parameter,
    required_numeric_parameter_with_board_default,
};
use super::CASTELLATED_HOLE_VALID;
use super::COPPER_SPACING_VALID;
use super::COPPER_TO_BOARD_EDGE_CLEARANCE_VALID;
use super::DRILL_DIAMETER_VALID;
use super::DRILL_TO_BOARD_EDGE_CLEARANCE_VALID;
use super::SLOT_ASPECT_RATIO_VALID;
use super::SLOT_TO_BOARD_EDGE_CLEARANCE_VALID;
use super::SLOT_WIDTH_VALID;
use super::common::validation_input_missing;

pub(super) use annular_ring::validate_drill_annular_ring;
pub(super) use solder_mask::{
    validate_solder_mask_dam, validate_solder_mask_opening,
    validate_solder_paste_aperture_area_ratio, validate_solder_paste_aperture_size,
    validate_solder_paste_opening, validate_solder_paste_spacing,
};
pub(super) use solder_paste_bga::validate_solder_paste_bga_aperture;
pub(super) use solder_paste_ic::validate_solder_paste_ic_pin_aperture;

pub(super) fn validate_drill_diameter(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(min_drill_diameter_mm) =
        required_numeric_parameter(scenario, "min_drill_diameter_mm", findings)
    else {
        return;
    };
    let Some(max_drill_diameter_mm) =
        required_numeric_parameter(scenario, "max_drill_diameter_mm", findings)
    else {
        return;
    };
    if min_drill_diameter_mm < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.min_drill_diameter_mm must be greater than or equal to zero.",
        );
        return;
    }
    if max_drill_diameter_mm < min_drill_diameter_mm {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.max_drill_diameter_mm must be greater than or equal to parameters.min_drill_diameter_mm.",
        );
        return;
    }
    let drills = &bound.project.board.layout.drills;
    if drills.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "DRILL_DIAMETER_VALID requires board.layout.drills evidence.",
        );
        return;
    }
    for (drill_index, drill) in drills.iter().enumerate() {
        if let Err(message) = validate_drill_geometry(drill, drill_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        if drill.drill_mm + f64::EPSILON < min_drill_diameter_mm
            || drill.drill_mm > max_drill_diameter_mm + f64::EPSILON
        {
            findings.push(drill_diameter_finding(
                scenario,
                drill,
                drill_index,
                min_drill_diameter_mm,
                max_drill_diameter_mm,
            ));
        }
    }
}

pub(super) fn validate_drill_to_board_edge_clearance(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(min_clearance_mm) = required_numeric_parameter_with_board_default(
        scenario,
        "min_drill_edge_clearance_mm",
        bound
            .project
            .board
            .manufacturing
            .min_drill_edge_clearance_mm,
        "min_drill_edge_clearance_mm",
        findings,
    ) else {
        return;
    };
    if min_clearance_mm < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "min_drill_edge_clearance_mm must be greater than or equal to zero.",
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

pub(super) fn validate_slot_to_board_edge_clearance(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(min_clearance_mm) = required_numeric_parameter_with_board_default(
        scenario,
        "min_slot_edge_clearance_mm",
        bound.project.board.manufacturing.min_slot_edge_clearance_mm,
        "min_slot_edge_clearance_mm",
        findings,
    ) else {
        return;
    };
    if min_clearance_mm < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "min_slot_edge_clearance_mm must be greater than or equal to zero.",
        );
        return;
    }
    let slots = &bound.project.board.layout.slots;
    if slots.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "SLOT_TO_BOARD_EDGE_CLEARANCE_VALID requires board.layout.slots evidence.",
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
            "SLOT_TO_BOARD_EDGE_CLEARANCE_VALID requires usable board.layout.outline.segments evidence.",
        );
        return;
    }
    for (slot_index, slot) in slots.iter().enumerate() {
        if let Err(message) = validate_slot_geometry(slot, slot_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        let Some(nearest) = nearest_slot_edge_clearance(slot, &board_edges) else {
            validation_input_missing(
                findings,
                scenario,
                "SLOT_TO_BOARD_EDGE_CLEARANCE_VALID could not compute finite slot-to-board-edge clearance.",
            );
            continue;
        };
        if nearest.clearance_mm + f64::EPSILON < min_clearance_mm {
            findings.push(slot_edge_clearance_finding(
                scenario,
                slot,
                slot_index,
                nearest,
                min_clearance_mm,
            ));
        }
    }
}

pub(super) fn validate_slot_width(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(min_plated_width_mm) =
        required_numeric_parameter(scenario, "min_plated_slot_width_mm", findings)
    else {
        return;
    };
    let Some(min_non_plated_width_mm) =
        required_numeric_parameter(scenario, "min_non_plated_slot_width_mm", findings)
    else {
        return;
    };
    if min_plated_width_mm < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.min_plated_slot_width_mm must be greater than or equal to zero.",
        );
        return;
    }
    if min_non_plated_width_mm < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.min_non_plated_slot_width_mm must be greater than or equal to zero.",
        );
        return;
    }
    let slots = &bound.project.board.layout.slots;
    if slots.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "SLOT_WIDTH_VALID requires board.layout.slots evidence.",
        );
        return;
    }
    for (slot_index, slot) in slots.iter().enumerate() {
        if let Err(message) = validate_slot_geometry(slot, slot_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        let (required_width_mm, slot_process) = match slot.plating.as_str() {
            "plated" => (min_plated_width_mm, "plated"),
            "non_plated" => (min_non_plated_width_mm, "non_plated"),
            _ => (
                min_plated_width_mm.max(min_non_plated_width_mm),
                "unknown_plating",
            ),
        };
        if slot.width_mm + f64::EPSILON < required_width_mm {
            findings.push(slot_width_finding(
                scenario,
                slot,
                slot_index,
                slot_process,
                required_width_mm,
            ));
        }
    }
}

pub(super) fn validate_slot_aspect_ratio(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(min_aspect_ratio) =
        required_numeric_parameter(scenario, "min_slot_aspect_ratio", findings)
    else {
        return;
    };
    if min_aspect_ratio < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.min_slot_aspect_ratio must be greater than or equal to zero.",
        );
        return;
    }
    let slots = &bound.project.board.layout.slots;
    if slots.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "SLOT_ASPECT_RATIO_VALID requires board.layout.slots evidence.",
        );
        return;
    }
    for (slot_index, slot) in slots.iter().enumerate() {
        if let Err(message) = validate_slot_geometry(slot, slot_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        let length_mm = slot_centerline_length_mm(slot);
        let aspect_ratio = length_mm / slot.width_mm;
        if aspect_ratio + f64::EPSILON < min_aspect_ratio {
            findings.push(slot_aspect_ratio_finding(
                scenario,
                slot,
                slot_index,
                length_mm,
                aspect_ratio,
                min_aspect_ratio,
            ));
        }
    }
}

pub(super) fn validate_castellated_hole(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(min_diameter_mm) =
        required_numeric_parameter(scenario, "min_castellated_hole_diameter_mm", findings)
    else {
        return;
    };
    let Some(min_edge_clearance_mm) =
        required_numeric_parameter(scenario, "min_castellated_hole_edge_clearance_mm", findings)
    else {
        return;
    };
    let Some(min_hole_spacing_mm) = required_numeric_parameter(
        scenario,
        "min_castellated_hole_to_hole_spacing_mm",
        findings,
    ) else {
        return;
    };
    if min_diameter_mm < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.min_castellated_hole_diameter_mm must be greater than or equal to zero.",
        );
        return;
    }
    if min_edge_clearance_mm < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.min_castellated_hole_edge_clearance_mm must be greater than or equal to zero.",
        );
        return;
    }
    if min_hole_spacing_mm < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.min_castellated_hole_to_hole_spacing_mm must be greater than or equal to zero.",
        );
        return;
    }
    let drills = &bound.project.board.layout.drills;
    if drills.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "CASTELLATED_HOLE_VALID requires board.layout.drills evidence.",
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
            "CASTELLATED_HOLE_VALID requires usable board.layout.outline.segments evidence.",
        );
        return;
    }

    let mut castellated_drills = Vec::new();
    for (drill_index, drill) in drills.iter().enumerate() {
        if let Err(message) = validate_drill_geometry(drill, drill_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        if !drill.castellated {
            continue;
        }
        castellated_drills.push((drill_index, drill));
        if drill.drill_mm + f64::EPSILON < min_diameter_mm {
            findings.push(castellated_hole_diameter_finding(
                scenario,
                drill,
                drill_index,
                min_diameter_mm,
            ));
        }
        let Some(nearest) = nearest_drill_edge_clearance(drill, &board_edges) else {
            validation_input_missing(
                findings,
                scenario,
                "CASTELLATED_HOLE_VALID could not compute finite castellated-hole-to-board-edge clearance.",
            );
            continue;
        };
        if nearest.clearance_mm + f64::EPSILON < min_edge_clearance_mm {
            findings.push(castellated_hole_edge_finding(
                scenario,
                drill,
                drill_index,
                nearest,
                min_edge_clearance_mm,
            ));
        }
    }
    for first_index in 0..castellated_drills.len() {
        for second_index in (first_index + 1)..castellated_drills.len() {
            let (first_drill_index, first) = castellated_drills[first_index];
            let (second_drill_index, second) = castellated_drills[second_index];
            let spacing_mm = castellated_hole_spacing_mm(first, second);
            if spacing_mm + f64::EPSILON < min_hole_spacing_mm {
                findings.push(castellated_hole_spacing_finding(
                    scenario,
                    first,
                    first_drill_index,
                    second,
                    second_drill_index,
                    spacing_mm,
                    min_hole_spacing_mm,
                ));
            }
        }
    }
    if castellated_drills.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "CASTELLATED_HOLE_VALID requires at least one board.layout.drills[] entry with castellated: true.",
        );
    }
}

fn castellated_hole_spacing_mm(first: &LayoutDrill, second: &LayoutDrill) -> f64 {
    let center_distance_mm = ((first.at.x_mm - second.at.x_mm).powi(2)
        + (first.at.y_mm - second.at.y_mm).powi(2))
    .sqrt();
    center_distance_mm - first.drill_mm / 2.0 - second.drill_mm / 2.0
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

fn drill_diameter_finding(
    scenario: &Scenario,
    drill: &LayoutDrill,
    drill_index: usize,
    min_drill_diameter_mm: f64,
    max_drill_diameter_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        DRILL_DIAMETER_VALID,
        &scenario.name,
        format!(
            "Drill hit {} is {:.3} mm; selected fabrication process supports {:.3} mm to {:.3} mm circular drills.",
            drill_index, drill.drill_mm, min_drill_diameter_mm, max_drill_diameter_mm
        ),
    );
    insert_drill_measurements(&mut finding, drill, drill_index);
    finding.limit.insert(
        "min_drill_diameter_mm".to_string(),
        json!(min_drill_diameter_mm),
    );
    finding.limit.insert(
        "max_drill_diameter_mm".to_string(),
        json!(max_drill_diameter_mm),
    );
    finding.suggested_fixes = vec![
        "Choose a circular drill diameter inside the selected fabrication process range."
            .to_string(),
        "Use a routed slot rule instead if this geometry is a routed slot rather than a circular drill hit.".to_string(),
        "Move the board to a process option that explicitly supports this drill diameter."
            .to_string(),
    ];
    finding
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
    if drill.castellated {
        finding
            .measured
            .insert("drill_castellated".to_string(), json!(true));
    }
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

fn slot_edge_clearance_finding(
    scenario: &Scenario,
    slot: &LayoutSlot,
    slot_index: usize,
    nearest: SlotEdgeClearance<'_>,
    min_clearance_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        SLOT_TO_BOARD_EDGE_CLEARANCE_VALID,
        &scenario.name,
        format!(
            "Routed slot {} has {:.3} mm edge-to-board clearance, below {:.3} mm minimum.",
            slot_index, nearest.clearance_mm, min_clearance_mm
        ),
    );
    insert_slot_measurements(&mut finding, slot, slot_index);
    finding
        .measured
        .insert("clearance_mm".to_string(), json!(nearest.clearance_mm));
    finding.measured.insert(
        "slot_centerline_to_board_edge_distance_mm".to_string(),
        json!(nearest.centerline_distance_mm),
    );
    insert_board_edge_measurements(&mut finding, nearest.edge);
    finding.limit.insert(
        "min_slot_edge_clearance_mm".to_string(),
        json!(min_clearance_mm),
    );
    finding.suggested_fixes = vec![
        "Move the routed slot farther from the nearest board outline or cutout edge.".to_string(),
        "Reduce slot width only if the mechanical requirement and fabricator minimums allow it."
            .to_string(),
        "Adjust the board outline or slot geometry if the fabrication drawing is incorrect."
            .to_string(),
    ];
    finding
}

fn slot_width_finding(
    scenario: &Scenario,
    slot: &LayoutSlot,
    slot_index: usize,
    slot_process: &str,
    min_width_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        SLOT_WIDTH_VALID,
        &scenario.name,
        format!(
            "Routed slot {} is {:.3} mm wide for {} process evidence; required at least {:.3} mm.",
            slot_index, slot.width_mm, slot_process, min_width_mm
        ),
    );
    insert_slot_measurements(&mut finding, slot, slot_index);
    finding
        .measured
        .insert("slot_process".to_string(), json!(slot_process));
    finding
        .limit
        .insert("min_slot_width_mm".to_string(), json!(min_width_mm));
    finding.suggested_fixes = vec![
        "Increase the routed slot width to meet the selected fabrication process minimum."
            .to_string(),
        "Use the correct plated/non-plated slot export if the drill file plating class is wrong."
            .to_string(),
        "Move this feature to a process option that explicitly supports the smaller slot width."
            .to_string(),
    ];
    finding
}

fn slot_aspect_ratio_finding(
    scenario: &Scenario,
    slot: &LayoutSlot,
    slot_index: usize,
    length_mm: f64,
    aspect_ratio: f64,
    min_aspect_ratio: f64,
) -> Finding {
    let mut finding = Finding::critical(
        SLOT_ASPECT_RATIO_VALID,
        &scenario.name,
        format!(
            "Routed slot {} has length-to-width ratio {:.3}; selected process requires at least {:.3}.",
            slot_index, aspect_ratio, min_aspect_ratio
        ),
    );
    insert_slot_measurements(&mut finding, slot, slot_index);
    finding
        .measured
        .insert("slot_length_mm".to_string(), json!(length_mm));
    finding
        .measured
        .insert("slot_aspect_ratio".to_string(), json!(aspect_ratio));
    finding
        .limit
        .insert("min_slot_aspect_ratio".to_string(), json!(min_aspect_ratio));
    finding.suggested_fixes = vec![
        "Increase the routed slot length or reduce slot width until the length-to-width ratio meets the selected process rule.".to_string(),
        "Replace very short routed slots with circular drill hits when the mechanical requirement allows it.".to_string(),
        "Move this feature to a fabrication process option that explicitly supports shorter routed slots.".to_string(),
    ];
    finding
}

fn castellated_hole_diameter_finding(
    scenario: &Scenario,
    drill: &LayoutDrill,
    drill_index: usize,
    min_diameter_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        CASTELLATED_HOLE_VALID,
        &scenario.name,
        format!(
            "Castellated drill hit {} is {:.3} mm; selected castellated-hole process requires at least {:.3} mm.",
            drill_index, drill.drill_mm, min_diameter_mm
        ),
    );
    insert_drill_measurements(&mut finding, drill, drill_index);
    finding.limit.insert(
        "min_castellated_hole_diameter_mm".to_string(),
        json!(min_diameter_mm),
    );
    finding.suggested_fixes = vec![
        "Increase the castellated hole diameter to meet the selected fabrication rule."
            .to_string(),
        "Remove the castellated marker if this drill hit is not actually a castellated hole."
            .to_string(),
        "Move this feature to a fabrication process option that explicitly supports the smaller castellated hole diameter.".to_string(),
    ];
    finding
}

fn castellated_hole_edge_finding(
    scenario: &Scenario,
    drill: &LayoutDrill,
    drill_index: usize,
    nearest: DrillEdgeClearance<'_>,
    min_edge_clearance_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        CASTELLATED_HOLE_VALID,
        &scenario.name,
        format!(
            "Castellated drill hit {} has {:.3} mm hole-edge-to-board-edge clearance, below {:.3} mm minimum.",
            drill_index, nearest.clearance_mm, min_edge_clearance_mm
        ),
    );
    insert_drill_measurements(&mut finding, drill, drill_index);
    finding
        .measured
        .insert("clearance_mm".to_string(), json!(nearest.clearance_mm));
    finding.measured.insert(
        "center_to_board_edge_distance_mm".to_string(),
        json!(nearest.center_distance_mm),
    );
    insert_board_edge_measurements(&mut finding, nearest.edge);
    finding.limit.insert(
        "min_castellated_hole_edge_clearance_mm".to_string(),
        json!(min_edge_clearance_mm),
    );
    finding.suggested_fixes = vec![
        "Move the castellated hole farther from the board edge or revise the castellated board outline.".to_string(),
        "Use a non-castellated drill-edge scenario if this is an ordinary circular drill hit.".to_string(),
        "Document a fabricator-approved castellated-hole exception if the board is intentionally below the default JLCPCB source condition.".to_string(),
    ];
    finding
}

fn castellated_hole_spacing_finding(
    scenario: &Scenario,
    first: &LayoutDrill,
    first_index: usize,
    second: &LayoutDrill,
    second_index: usize,
    spacing_mm: f64,
    min_spacing_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        CASTELLATED_HOLE_VALID,
        &scenario.name,
        format!(
            "Castellated drill hits {} and {} have {:.3} mm hole-to-hole spacing, below {:.3} mm minimum.",
            first_index, second_index, spacing_mm, min_spacing_mm
        ),
    );
    finding
        .measured
        .insert("first_drill_index".to_string(), json!(first_index));
    finding.measured.insert(
        "first_drill_at".to_string(),
        json!({
            "x_mm": first.at.x_mm,
            "y_mm": first.at.y_mm,
        }),
    );
    finding
        .measured
        .insert("first_drill_mm".to_string(), json!(first.drill_mm));
    finding
        .measured
        .insert("second_drill_index".to_string(), json!(second_index));
    finding.measured.insert(
        "second_drill_at".to_string(),
        json!({
            "x_mm": second.at.x_mm,
            "y_mm": second.at.y_mm,
        }),
    );
    finding
        .measured
        .insert("second_drill_mm".to_string(), json!(second.drill_mm));
    finding.measured.insert(
        "castellated_hole_to_hole_spacing_mm".to_string(),
        json!(spacing_mm),
    );
    finding.limit.insert(
        "min_castellated_hole_to_hole_spacing_mm".to_string(),
        json!(min_spacing_mm),
    );
    finding.suggested_fixes = vec![
        "Increase spacing between adjacent castellated holes to meet the selected fabrication rule.".to_string(),
        "Reduce castellated hole diameter only if the plated-edge requirement still permits it.".to_string(),
        "Document a fabricator-approved castellated-hole spacing exception if the board is intentionally below the default JLCPCB source condition.".to_string(),
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
    if drill.castellated {
        finding
            .measured
            .insert("drill_castellated".to_string(), json!(true));
    }
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

fn insert_slot_measurements(finding: &mut Finding, slot: &LayoutSlot, slot_index: usize) {
    finding
        .measured
        .insert("slot_index".to_string(), json!(slot_index));
    finding.measured.insert(
        "slot_start".to_string(),
        json!({
            "x_mm": slot.start.x_mm,
            "y_mm": slot.start.y_mm,
        }),
    );
    finding.measured.insert(
        "slot_end".to_string(),
        json!({
            "x_mm": slot.end.x_mm,
            "y_mm": slot.end.y_mm,
        }),
    );
    finding
        .measured
        .insert("slot_width_mm".to_string(), json!(slot.width_mm));
    finding
        .measured
        .insert("slot_radius_mm".to_string(), json!(slot.width_mm / 2.0));
    finding
        .measured
        .insert("slot_plating".to_string(), json!(slot.plating));
    if let Some(layer) = &slot.layer {
        finding
            .measured
            .insert("slot_layer".to_string(), json!(layer));
    }
    if let Some(tool) = &slot.tool {
        finding
            .measured
            .insert("slot_tool".to_string(), json!(tool));
    }
    if let Some(source_slot_index) = slot.source_slot_index {
        finding
            .measured
            .insert("source_slot_index".to_string(), json!(source_slot_index));
    }
}

fn slot_centerline_length_mm(slot: &LayoutSlot) -> f64 {
    let dx = slot.end.x_mm - slot.start.x_mm;
    let dy = slot.end.y_mm - slot.start.y_mm;
    (dx * dx + dy * dy).sqrt()
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
