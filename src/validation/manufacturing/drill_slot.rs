use crate::board_ir::{LayoutDrill, LayoutSlot, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use super::super::{
    CASTELLATED_HOLE_VALID, DRILL_DIAMETER_VALID, DRILL_TO_BOARD_EDGE_CLEARANCE_VALID,
    SLOT_ASPECT_RATIO_VALID, SLOT_TO_BOARD_EDGE_CLEARANCE_VALID, SLOT_WIDTH_VALID,
};
use super::geometry::{
    DrillEdgeClearance, SlotEdgeClearance, nearest_drill_edge_clearance,
    nearest_slot_edge_clearance, usable_outline_segment, validate_drill_geometry,
    validate_slot_geometry,
};
use super::{
    insert_board_edge_measurements, insert_drill_measurements, required_numeric_parameter,
    required_numeric_parameter_with_board_default, validation_input_missing,
};

pub(in crate::validation) fn validate_drill_diameter(
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

pub(in crate::validation) fn validate_drill_to_board_edge_clearance(
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

pub(in crate::validation) fn validate_slot_to_board_edge_clearance(
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

pub(in crate::validation) fn validate_slot_width(
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

pub(in crate::validation) fn validate_slot_aspect_ratio(
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

pub(in crate::validation) fn validate_castellated_hole(
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
