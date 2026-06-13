use crate::board_ir::{LayoutCopperFeature, LayoutPoint, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use super::super::common::validation_input_missing;
use super::super::{
    SOLDER_MASK_DAM_VALID, SOLDER_MASK_OPENING_VALID, SOLDER_PASTE_OPENING_VALID,
    SOLDER_PASTE_SPACING_VALID,
};
use super::geometry::{
    CopperObjectRef, copper_object_spacing_mm, point_distance_mm, validate_copper_feature_geometry,
    validate_copper_region_geometry, validate_copper_segment_geometry,
};
use super::{
    insert_copper_feature_edge_measurements, insert_optional_copper_feature_owner_measurements,
    optional_numeric_parameter, required_numeric_parameter,
};

pub(in crate::validation) fn validate_solder_mask_opening(
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

pub(in crate::validation) fn validate_solder_mask_dam(
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
    if mask.features.len() + mask.segments.len() + mask.regions.len() < 2 {
        validation_input_missing(
            findings,
            scenario,
            "SOLDER_MASK_DAM_VALID requires at least two board.layout.solder_mask features, segments, or regions.",
        );
        return;
    }
    let mut mask_objects = Vec::new();
    for (feature_index, feature) in mask.features.iter().enumerate() {
        if let Err(message) = validate_copper_feature_geometry(feature, feature_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        mask_objects.push(CopperObjectRef::Feature {
            feature,
            index: feature_index,
        });
    }
    for (segment_index, segment) in mask.segments.iter().enumerate() {
        if let Err(message) = validate_copper_segment_geometry(segment, segment_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        mask_objects.push(CopperObjectRef::Segment {
            segment,
            index: segment_index,
        });
    }
    for (region_index, region) in mask.regions.iter().enumerate() {
        if let Err(message) = validate_copper_region_geometry(region, region_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        mask_objects.push(CopperObjectRef::Region {
            region,
            index: region_index,
        });
    }
    for (first_offset, first_object) in mask_objects.iter().enumerate() {
        for second_object in mask_objects.iter().skip(first_offset + 1) {
            if first_object.layer() != second_object.layer() {
                continue;
            }
            let Some(dam_width_mm) = copper_object_spacing_mm(*first_object, *second_object) else {
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
                    *first_object,
                    *second_object,
                    dam_width_mm.max(0.0),
                    min_dam_mm,
                ));
            }
        }
    }
}

pub(in crate::validation) fn validate_solder_paste_opening(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(min_area_ratio) =
        required_numeric_parameter(scenario, "min_paste_area_ratio", findings)
    else {
        return;
    };
    let Some(max_area_ratio) =
        required_numeric_parameter(scenario, "max_paste_area_ratio", findings)
    else {
        return;
    };
    if min_area_ratio < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.min_paste_area_ratio must be greater than or equal to zero.",
        );
        return;
    }
    if max_area_ratio < min_area_ratio {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.max_paste_area_ratio must be greater than or equal to parameters.min_paste_area_ratio.",
        );
        return;
    }
    let Some(max_center_offset_mm) = optional_numeric_parameter(
        scenario,
        "max_copper_to_paste_center_offset_mm",
        0.1,
        findings,
    ) else {
        return;
    };
    if max_center_offset_mm < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.max_copper_to_paste_center_offset_mm must be greater than or equal to zero.",
        );
        return;
    }
    let copper = &bound.project.board.layout.copper;
    if copper.features.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "SOLDER_PASTE_OPENING_VALID requires board.layout.copper.features evidence.",
        );
        return;
    }
    let paste = &bound.project.board.layout.solder_paste;
    if paste.features.len() + paste.segments.len() + paste.regions.len() == 0 {
        validation_input_missing(
            findings,
            scenario,
            "SOLDER_PASTE_OPENING_VALID requires board.layout.solder_paste features, segments, or regions evidence.",
        );
        return;
    }
    let mut paste_objects = Vec::new();
    for (paste_index, paste_feature) in paste.features.iter().enumerate() {
        if let Err(message) = validate_copper_feature_geometry(paste_feature, paste_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        paste_objects.push(CopperObjectRef::Feature {
            feature: paste_feature,
            index: paste_index,
        });
    }
    for (paste_index, paste_segment) in paste.segments.iter().enumerate() {
        if let Err(message) = validate_copper_segment_geometry(paste_segment, paste_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        paste_objects.push(CopperObjectRef::Segment {
            segment: paste_segment,
            index: paste_index,
        });
    }
    for (paste_index, paste_region) in paste.regions.iter().enumerate() {
        if let Err(message) = validate_copper_region_geometry(paste_region, paste_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        paste_objects.push(CopperObjectRef::Region {
            region: paste_region,
            index: paste_index,
        });
    }
    for (copper_index, copper_feature) in copper.features.iter().enumerate() {
        if copper_feature.owner_kind.as_deref() == Some("via") {
            continue;
        }
        if let Err(message) = validate_copper_feature_geometry(copper_feature, copper_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        let Some(paste_layer) = solder_paste_layer_for_copper_layer(&copper_feature.layer) else {
            continue;
        };
        let Some(copper_area_mm2) = feature_area_mm2(copper_feature) else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "SOLDER_PASTE_OPENING_VALID does not support copper feature {copper_index} shape {} for area-ratio validation.",
                    copper_feature.shape
                ),
            );
            continue;
        };
        let mut best_candidate: Option<SolderPasteOpeningCandidate<'_>> = None;
        for paste_object in &paste_objects {
            if paste_object.layer() != paste_layer {
                continue;
            }
            let Some(paste_center) = paste_object_center(*paste_object) else {
                validation_input_missing(
                    findings,
                    scenario,
                    "SOLDER_PASTE_OPENING_VALID could not compute finite solder-paste opening center for supported Gerber paste geometry.",
                );
                continue;
            };
            let center_offset_mm = point_distance_mm(&copper_feature.at, &paste_center);
            if center_offset_mm > max_center_offset_mm {
                continue;
            }
            let Some(paste_area_mm2) = paste_object_area_mm2(*paste_object) else {
                validation_input_missing(
                    findings,
                    scenario,
                    "SOLDER_PASTE_OPENING_VALID could not compute finite positive solder-paste opening area for supported Gerber paste geometry.",
                );
                continue;
            };
            let area_ratio = paste_area_mm2 / copper_area_mm2;
            let candidate = SolderPasteOpeningCandidate {
                paste_object: *paste_object,
                center_offset_mm,
                copper_area_mm2,
                paste_area_mm2,
                area_ratio,
            };
            if best_candidate.is_none_or(|best| {
                candidate.center_offset_mm < best.center_offset_mm
                    || (candidate.center_offset_mm == best.center_offset_mm
                        && (candidate.area_ratio - 1.0).abs() < (best.area_ratio - 1.0).abs())
            }) {
                best_candidate = Some(candidate);
            }
        }
        match best_candidate {
            Some(candidate)
                if candidate.area_ratio + f64::EPSILON < min_area_ratio
                    || candidate.area_ratio > max_area_ratio + f64::EPSILON =>
            {
                findings.push(solder_paste_opening_area_finding(
                    scenario,
                    copper_feature,
                    copper_index,
                    candidate,
                    min_area_ratio,
                    max_area_ratio,
                    max_center_offset_mm,
                ));
            }
            None => findings.push(solder_paste_opening_missing_finding(
                scenario,
                copper_feature,
                copper_index,
                paste_layer,
                min_area_ratio,
                max_area_ratio,
                max_center_offset_mm,
            )),
            _ => {}
        }
    }
}

pub(in crate::validation) fn validate_solder_paste_spacing(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(min_spacing_mm) =
        required_numeric_parameter(scenario, "min_solder_paste_spacing_mm", findings)
    else {
        return;
    };
    if min_spacing_mm < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.min_solder_paste_spacing_mm must be greater than or equal to zero.",
        );
        return;
    }
    let paste = &bound.project.board.layout.solder_paste;
    if paste.features.len() + paste.segments.len() + paste.regions.len() < 2 {
        validation_input_missing(
            findings,
            scenario,
            "SOLDER_PASTE_SPACING_VALID requires at least two board.layout.solder_paste features, segments, or regions.",
        );
        return;
    }
    let mut paste_objects = Vec::new();
    for (feature_index, feature) in paste.features.iter().enumerate() {
        if let Err(message) = validate_copper_feature_geometry(feature, feature_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        paste_objects.push(CopperObjectRef::Feature {
            feature,
            index: feature_index,
        });
    }
    for (segment_index, segment) in paste.segments.iter().enumerate() {
        if let Err(message) = validate_copper_segment_geometry(segment, segment_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        paste_objects.push(CopperObjectRef::Segment {
            segment,
            index: segment_index,
        });
    }
    for (region_index, region) in paste.regions.iter().enumerate() {
        if let Err(message) = validate_copper_region_geometry(region, region_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        paste_objects.push(CopperObjectRef::Region {
            region,
            index: region_index,
        });
    }
    for (first_offset, first_object) in paste_objects.iter().enumerate() {
        for second_object in paste_objects.iter().skip(first_offset + 1) {
            if first_object.layer() != second_object.layer() {
                continue;
            }
            let Some(spacing_mm) = copper_object_spacing_mm(*first_object, *second_object) else {
                validation_input_missing(
                    findings,
                    scenario,
                    "SOLDER_PASTE_SPACING_VALID could not compute finite solder-paste opening spacing for supported Gerber paste geometry.",
                );
                return;
            };
            if spacing_mm + f64::EPSILON < min_spacing_mm {
                findings.push(solder_paste_spacing_finding(
                    scenario,
                    *first_object,
                    *second_object,
                    spacing_mm.max(0.0),
                    min_spacing_mm,
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

#[derive(Clone, Copy)]
struct SolderPasteOpeningCandidate<'a> {
    paste_object: CopperObjectRef<'a>,
    center_offset_mm: f64,
    copper_area_mm2: f64,
    paste_area_mm2: f64,
    area_ratio: f64,
}

fn solder_mask_layer_for_copper_layer(copper_layer: &str) -> Option<&'static str> {
    match copper_layer {
        "F.Cu" => Some("F.Mask"),
        "B.Cu" => Some("B.Mask"),
        _ => None,
    }
}

fn solder_paste_layer_for_copper_layer(copper_layer: &str) -> Option<&'static str> {
    match copper_layer {
        "F.Cu" => Some("F.Paste"),
        "B.Cu" => Some("B.Paste"),
        _ => None,
    }
}

fn feature_area_mm2(feature: &LayoutCopperFeature) -> Option<f64> {
    match feature.shape.as_str() {
        "rect" => Some(feature.size.x_mm * feature.size.y_mm),
        "circle" | "oval" => {
            Some(std::f64::consts::PI * feature.size.x_mm * feature.size.y_mm / 4.0)
        }
        _ => None,
    }
}

fn paste_object_center(object: CopperObjectRef<'_>) -> Option<LayoutPoint> {
    match object {
        CopperObjectRef::Feature { feature, .. } => Some(feature.at.clone()),
        CopperObjectRef::Segment { segment, .. } => Some(LayoutPoint {
            x_mm: (segment.start.x_mm + segment.end.x_mm) / 2.0,
            y_mm: (segment.start.y_mm + segment.end.y_mm) / 2.0,
        }),
        CopperObjectRef::Region { region, .. } => polygon_centroid(&region.points),
    }
    .filter(|point| point.x_mm.is_finite() && point.y_mm.is_finite())
}

fn paste_object_area_mm2(object: CopperObjectRef<'_>) -> Option<f64> {
    match object {
        CopperObjectRef::Feature { feature, .. } => feature_area_mm2(feature),
        CopperObjectRef::Segment { segment, .. } => {
            let length_mm = point_distance_mm(&segment.start, &segment.end);
            let radius_mm = segment.width_mm / 2.0;
            Some(length_mm * segment.width_mm + std::f64::consts::PI * radius_mm * radius_mm)
        }
        CopperObjectRef::Region { region, .. } => Some(polygon_area_mm2(&region.points).abs()),
    }
    .filter(|area| area.is_finite() && *area > 0.0)
}

fn polygon_area_mm2(points: &[LayoutPoint]) -> f64 {
    closed_edges(points)
        .map(|(first, second)| first.x_mm * second.y_mm - second.x_mm * first.y_mm)
        .sum::<f64>()
        / 2.0
}

fn polygon_centroid(points: &[LayoutPoint]) -> Option<LayoutPoint> {
    let signed_area = polygon_area_mm2(points);
    if signed_area.abs() <= f64::EPSILON {
        return None;
    }
    let mut cx = 0.0;
    let mut cy = 0.0;
    for (first, second) in closed_edges(points) {
        let cross = first.x_mm * second.y_mm - second.x_mm * first.y_mm;
        cx += (first.x_mm + second.x_mm) * cross;
        cy += (first.y_mm + second.y_mm) * cross;
    }
    Some(LayoutPoint {
        x_mm: cx / (6.0 * signed_area),
        y_mm: cy / (6.0 * signed_area),
    })
}

fn closed_edges(points: &[LayoutPoint]) -> impl Iterator<Item = (&LayoutPoint, &LayoutPoint)> {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
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
    first_object: CopperObjectRef<'_>,
    second_object: CopperObjectRef<'_>,
    dam_width_mm: f64,
    min_dam_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        SOLDER_MASK_DAM_VALID,
        scenario.name.clone(),
        format!(
            "Solder-mask {} and {} openings on {} leave only {:.6} mm mask dam; required at least {:.6} mm.",
            first_object.kind(),
            second_object.kind(),
            first_object.layer(),
            dam_width_mm,
            min_dam_mm
        ),
    );
    finding.suggested_fixes = vec![
        "Increase the solder-mask dam by reducing mask expansion, increasing pad spacing, or using a package/fabrication process that supports the smaller mask web.".to_string(),
        "If the mask bridge is intentionally removed for fine-pitch pads, record that fabrication rule explicitly and adjust this scenario threshold.".to_string(),
    ];
    finding
        .measured
        .insert("solder_mask_layer".to_string(), json!(first_object.layer()));
    insert_solder_mask_object_measurements(&mut finding, "first", first_object);
    insert_solder_mask_object_measurements(&mut finding, "second", second_object);
    finding
        .measured
        .insert("solder_mask_dam_width_mm".to_string(), json!(dam_width_mm));
    finding
        .limit
        .insert("min_solder_mask_dam_mm".to_string(), json!(min_dam_mm));
    finding
}

fn solder_paste_opening_missing_finding(
    scenario: &Scenario,
    copper_feature: &LayoutCopperFeature,
    copper_index: usize,
    expected_paste_layer: &str,
    min_area_ratio: f64,
    max_area_ratio: f64,
    max_center_offset_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        SOLDER_PASTE_OPENING_VALID,
        scenario.name.clone(),
        format!(
            "Copper flash {copper_index} on {} has no co-located solder-paste opening on {expected_paste_layer}.",
            copper_feature.layer
        ),
    );
    finding.suggested_fixes = vec![
        "Add or restore a solder-paste stencil aperture over this paste-bearing SMT pad, or verify the paste Gerber was exported for the correct board side.".to_string(),
    ];
    finding
        .measured
        .insert("copper_feature_index".to_string(), json!(copper_index));
    insert_copper_feature_edge_measurements(&mut finding, copper_feature);
    finding.measured.insert(
        "expected_solder_paste_layer".to_string(),
        json!(expected_paste_layer),
    );
    insert_solder_paste_limits(
        &mut finding,
        min_area_ratio,
        max_area_ratio,
        max_center_offset_mm,
    );
    finding
}

fn solder_paste_opening_area_finding(
    scenario: &Scenario,
    copper_feature: &LayoutCopperFeature,
    copper_index: usize,
    candidate: SolderPasteOpeningCandidate<'_>,
    min_area_ratio: f64,
    max_area_ratio: f64,
    max_center_offset_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        SOLDER_PASTE_OPENING_VALID,
        scenario.name.clone(),
        format!(
            "Solder-paste {} opening on {} has area ratio {:.6} against copper flash {copper_index}; allowed range is {:.6}..={:.6}.",
            candidate.paste_object.kind(),
            candidate.paste_object.layer(),
            candidate.area_ratio,
            min_area_ratio,
            max_area_ratio
        ),
    );
    finding.suggested_fixes = vec![
        "Adjust the stencil aperture size or paste margin/ratio for this pad so paste area matches the package and fabrication process requirements.".to_string(),
        "Verify the paste Gerber layer is registered to the copper Gerber and that paste apertures were not globally over-expanded or suppressed.".to_string(),
    ];
    finding
        .measured
        .insert("copper_feature_index".to_string(), json!(copper_index));
    insert_copper_feature_edge_measurements(&mut finding, copper_feature);
    insert_solder_paste_feature_measurements(&mut finding, candidate);
    insert_solder_paste_limits(
        &mut finding,
        min_area_ratio,
        max_area_ratio,
        max_center_offset_mm,
    );
    finding
}

fn solder_paste_spacing_finding(
    scenario: &Scenario,
    first_object: CopperObjectRef<'_>,
    second_object: CopperObjectRef<'_>,
    spacing_mm: f64,
    min_spacing_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        SOLDER_PASTE_SPACING_VALID,
        scenario.name.clone(),
        format!(
            "Solder-paste {} and {} openings on {} leave only {:.6} mm spacing; required at least {:.6} mm.",
            first_object.kind(),
            second_object.kind(),
            first_object.layer(),
            spacing_mm,
            min_spacing_mm
        ),
    );
    finding.suggested_fixes = vec![
        "Increase paste aperture spacing by reducing stencil aperture size, increasing pad spacing, or applying package-specific paste reductions.".to_string(),
        "If adjacent paste openings are intentionally merged, document the stencil process rule and adjust this scenario threshold.".to_string(),
    ];
    finding.measured.insert(
        "solder_paste_layer".to_string(),
        json!(first_object.layer()),
    );
    insert_prefixed_solder_paste_object_measurements(&mut finding, "first", first_object);
    insert_prefixed_solder_paste_object_measurements(&mut finding, "second", second_object);
    finding
        .measured
        .insert("solder_paste_spacing_mm".to_string(), json!(spacing_mm));
    finding.limit.insert(
        "min_solder_paste_spacing_mm".to_string(),
        json!(min_spacing_mm),
    );
    finding
}

fn insert_solder_paste_limits(
    finding: &mut Finding,
    min_area_ratio: f64,
    max_area_ratio: f64,
    max_center_offset_mm: f64,
) {
    finding
        .limit
        .insert("min_paste_area_ratio".to_string(), json!(min_area_ratio));
    finding
        .limit
        .insert("max_paste_area_ratio".to_string(), json!(max_area_ratio));
    finding.limit.insert(
        "max_copper_to_paste_center_offset_mm".to_string(),
        json!(max_center_offset_mm),
    );
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
    insert_optional_copper_feature_owner_measurements(finding, "solder_mask_feature", feature);
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

fn insert_solder_paste_feature_measurements(
    finding: &mut Finding,
    candidate: SolderPasteOpeningCandidate<'_>,
) {
    insert_solder_paste_object_measurements(finding, candidate.paste_object);
    finding.measured.insert(
        "copper_feature_area_mm2".to_string(),
        json!(candidate.copper_area_mm2),
    );
    finding.measured.insert(
        "solder_paste_opening_area_mm2".to_string(),
        json!(candidate.paste_area_mm2),
    );
    finding.measured.insert(
        "solder_paste_area_ratio".to_string(),
        json!(candidate.area_ratio),
    );
    finding.measured.insert(
        "copper_to_paste_center_offset_mm".to_string(),
        json!(candidate.center_offset_mm),
    );
}

fn insert_solder_paste_object_measurements(finding: &mut Finding, object: CopperObjectRef<'_>) {
    insert_prefixed_solder_paste_object_measurements(finding, "", object);
}

fn insert_prefixed_solder_paste_object_measurements(
    finding: &mut Finding,
    prefix: &str,
    object: CopperObjectRef<'_>,
) {
    let key = |field: &str| {
        if prefix.is_empty() {
            format!("solder_paste_{field}")
        } else {
            format!("{prefix}_solder_paste_{field}")
        }
    };
    finding.measured.insert(key("kind"), json!(object.kind()));
    match object {
        CopperObjectRef::Feature { feature, index } => {
            finding.measured.insert(key("feature_index"), json!(index));
            finding
                .measured
                .insert(key("feature_x_mm"), json!(feature.at.x_mm));
            finding
                .measured
                .insert(key("feature_y_mm"), json!(feature.at.y_mm));
            finding
                .measured
                .insert(key("feature_layer"), json!(feature.layer));
            insert_optional_copper_feature_owner_measurements(finding, &key("feature"), feature);
            finding
                .measured
                .insert(key("feature_aperture"), json!(feature.aperture));
            finding
                .measured
                .insert(key("feature_shape"), json!(feature.shape));
            finding
                .measured
                .insert(key("feature_size_x_mm"), json!(feature.size.x_mm));
            finding
                .measured
                .insert(key("feature_size_y_mm"), json!(feature.size.y_mm));
            finding.measured.insert(
                key("feature_source_primitive"),
                json!(feature.source_primitive),
            );
            finding.measured.insert(
                key("feature_source_primitive_index"),
                json!(feature.source_primitive_index),
            );
        }
        CopperObjectRef::Segment { segment, index } => {
            finding.measured.insert(key("segment_index"), json!(index));
            finding.measured.insert(
                key("segment_start"),
                json!({
                    "x_mm": segment.start.x_mm,
                    "y_mm": segment.start.y_mm,
                }),
            );
            finding.measured.insert(
                key("segment_end"),
                json!({
                    "x_mm": segment.end.x_mm,
                    "y_mm": segment.end.y_mm,
                }),
            );
            finding
                .measured
                .insert(key("segment_layer"), json!(segment.layer));
            finding
                .measured
                .insert(key("segment_aperture"), json!(segment.aperture));
            finding
                .measured
                .insert(key("segment_width_mm"), json!(segment.width_mm));
            finding.measured.insert(
                key("segment_source_primitive"),
                json!(segment.source_primitive),
            );
            finding.measured.insert(
                key("segment_source_primitive_index"),
                json!(segment.source_primitive_index),
            );
        }
        CopperObjectRef::Region { region, index } => {
            finding.measured.insert(key("region_index"), json!(index));
            finding
                .measured
                .insert(key("region_layer"), json!(region.layer));
            finding.measured.insert(
                key("region_source_primitive"),
                json!(region.source_primitive),
            );
            finding.measured.insert(
                key("region_source_primitive_index"),
                json!(region.source_primitive_index),
            );
            finding
                .measured
                .insert(key("region_point_count"), json!(region.points.len()));
        }
    }
}

fn insert_solder_mask_object_measurements(
    finding: &mut Finding,
    prefix: &str,
    object: CopperObjectRef<'_>,
) {
    finding
        .measured
        .insert(format!("{prefix}_solder_mask_kind"), json!(object.kind()));
    match object {
        CopperObjectRef::Feature { feature, index } => {
            finding
                .measured
                .insert(format!("{prefix}_solder_mask_feature_index"), json!(index));
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
            insert_optional_copper_feature_owner_measurements(
                finding,
                &format!("{prefix}_solder_mask_feature"),
                feature,
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
        CopperObjectRef::Segment { segment, index } => {
            finding
                .measured
                .insert(format!("{prefix}_solder_mask_segment_index"), json!(index));
            finding.measured.insert(
                format!("{prefix}_solder_mask_segment_start"),
                json!({
                    "x_mm": segment.start.x_mm,
                    "y_mm": segment.start.y_mm,
                }),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_segment_end"),
                json!({
                    "x_mm": segment.end.x_mm,
                    "y_mm": segment.end.y_mm,
                }),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_segment_layer"),
                json!(segment.layer),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_segment_aperture"),
                json!(segment.aperture),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_segment_width_mm"),
                json!(segment.width_mm),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_segment_source_primitive"),
                json!(segment.source_primitive),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_segment_source_primitive_index"),
                json!(segment.source_primitive_index),
            );
        }
        CopperObjectRef::Region { region, index } => {
            finding
                .measured
                .insert(format!("{prefix}_solder_mask_region_index"), json!(index));
            finding.measured.insert(
                format!("{prefix}_solder_mask_region_layer"),
                json!(region.layer),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_region_source_primitive"),
                json!(region.source_primitive),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_region_source_primitive_index"),
                json!(region.source_primitive_index),
            );
            finding.measured.insert(
                format!("{prefix}_solder_mask_region_point_count"),
                json!(region.points.len()),
            );
        }
    }
}
