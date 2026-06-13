use crate::board_ir::{LayoutCopperRegion, LayoutCopperSegment, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use super::super::SOLDER_PASTE_IC_PIN_APERTURE_VALID;
use super::super::common::validation_input_missing;
use super::geometry::{
    CopperObjectRef, validate_copper_feature_geometry, validate_copper_region_geometry,
    validate_copper_segment_geometry,
};
use super::{insert_optional_copper_feature_owner_measurements, required_numeric_parameter};

pub(in crate::validation) fn validate_solder_paste_ic_pin_aperture(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(pin_pitch_mm) = required_numeric_parameter(scenario, "pin_pitch_mm", findings) else {
        return;
    };
    if pin_pitch_mm <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.pin_pitch_mm must be greater than zero.",
        );
        return;
    }
    let Some(aperture_spec) = jlc_ic_pin_aperture_spec(pin_pitch_mm) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "SOLDER_PASTE_IC_PIN_APERTURE_VALID has no source-backed JLCPCB IC aperture guidance for pin_pitch_mm={pin_pitch_mm:.6}."
            ),
        );
        return;
    };
    let paste = &bound.project.board.layout.solder_paste;
    if paste.features.is_empty() && paste.segments.is_empty() && paste.regions.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "SOLDER_PASTE_IC_PIN_APERTURE_VALID requires board.layout.solder_paste evidence.",
        );
        return;
    }

    let mut checked_pad_owned_openings = 0usize;
    for (feature_index, feature) in paste.features.iter().enumerate() {
        if let Err(message) = validate_copper_feature_geometry(feature, feature_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        let object = CopperObjectRef::Feature {
            feature,
            index: feature_index,
        };
        if !paste_object_is_pad_owned(object) {
            continue;
        }
        if !paste_object_matches_target(object, scenario) {
            continue;
        }
        checked_pad_owned_openings += 1;
        let aperture_width_mm = feature.size.x_mm.min(feature.size.y_mm);
        let aperture_length_mm = feature.size.x_mm.max(feature.size.y_mm);
        if aperture_width_out_of_range(aperture_width_mm, aperture_spec) {
            findings.push(solder_paste_ic_pin_aperture_width_finding(
                scenario,
                object,
                aperture_width_mm,
                pin_pitch_mm,
                aperture_spec,
            ));
        }
        if aperture_length_out_of_range(aperture_length_mm, aperture_spec) {
            findings.push(solder_paste_ic_pin_aperture_length_finding(
                scenario,
                object,
                aperture_length_mm,
                pin_pitch_mm,
                aperture_spec,
            ));
        }
    }
    for (segment_index, segment) in paste.segments.iter().enumerate() {
        if let Err(message) = validate_copper_segment_geometry(segment, segment_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        let object = CopperObjectRef::Segment {
            segment,
            index: segment_index,
        };
        if !paste_object_is_pad_owned(object) {
            continue;
        }
        if !paste_object_matches_target(object, scenario) {
            continue;
        }
        checked_pad_owned_openings += 1;
        if aperture_width_out_of_range(segment.width_mm, aperture_spec) {
            findings.push(solder_paste_ic_pin_aperture_width_finding(
                scenario,
                object,
                segment.width_mm,
                pin_pitch_mm,
                aperture_spec,
            ));
        }
        let aperture_length_mm = segment_aperture_length_mm(segment);
        if aperture_length_out_of_range(aperture_length_mm, aperture_spec) {
            findings.push(solder_paste_ic_pin_aperture_length_finding(
                scenario,
                object,
                aperture_length_mm,
                pin_pitch_mm,
                aperture_spec,
            ));
        }
    }
    for (region_index, region) in paste.regions.iter().enumerate() {
        if let Err(message) = validate_copper_region_geometry(region, region_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        let object = CopperObjectRef::Region {
            region,
            index: region_index,
        };
        if !paste_object_is_pad_owned(object) {
            continue;
        }
        if !paste_object_matches_target(object, scenario) {
            continue;
        }
        let Some(aperture_width_mm) = region_bounding_box_min_dimension_mm(region) else {
            validation_input_missing(
                findings,
                scenario,
                "SOLDER_PASTE_IC_PIN_APERTURE_VALID could not compute finite solder-paste region aperture width.",
            );
            continue;
        };
        let Some(aperture_length_mm) = region_bounding_box_max_dimension_mm(region) else {
            validation_input_missing(
                findings,
                scenario,
                "SOLDER_PASTE_IC_PIN_APERTURE_VALID could not compute finite solder-paste region aperture length.",
            );
            continue;
        };
        checked_pad_owned_openings += 1;
        if aperture_width_out_of_range(aperture_width_mm, aperture_spec) {
            findings.push(solder_paste_ic_pin_aperture_width_finding(
                scenario,
                object,
                aperture_width_mm,
                pin_pitch_mm,
                aperture_spec,
            ));
        }
        if aperture_length_out_of_range(aperture_length_mm, aperture_spec) {
            findings.push(solder_paste_ic_pin_aperture_length_finding(
                scenario,
                object,
                aperture_length_mm,
                pin_pitch_mm,
                aperture_spec,
            ));
        }
    }

    if checked_pad_owned_openings == 0 {
        let message = scenario.target.as_ref().map_or_else(
            || {
                "SOLDER_PASTE_IC_PIN_APERTURE_VALID requires pad-owned board.layout.solder_paste feature, segment, or region evidence.".to_string()
            },
            |target| {
                format!(
                    "SOLDER_PASTE_IC_PIN_APERTURE_VALID requires pad-owned board.layout.solder_paste feature, segment, or region evidence for target component {}.",
                    target.component
                )
            },
        );
        validation_input_missing(findings, scenario, message);
    }
}

#[derive(Clone, Copy)]
struct IcPinApertureSpec {
    min_mm: f64,
    max_mm: f64,
    length_mm: Option<f64>,
    source_condition: &'static str,
}

fn jlc_ic_pin_aperture_spec(pin_pitch_mm: f64) -> Option<IcPinApertureSpec> {
    if (0.8..=1.27).contains(&pin_pitch_mm) {
        return Some(IcPinApertureSpec {
            min_mm: pin_pitch_mm * 0.45,
            max_mm: pin_pitch_mm * 0.60,
            length_mm: None,
            source_condition: "JLCPCB IC stencil pitch 0.8-1.27 mm: width 45%-60% of pitch",
        });
    }
    if (0.635..=0.65).contains(&pin_pitch_mm) {
        return Some(IcPinApertureSpec {
            min_mm: 0.30,
            max_mm: 0.33,
            length_mm: Some(1.00),
            source_condition: "JLCPCB IC stencil pitch 0.635-0.65 mm: W=0.30-0.33 mm, L=1.00 mm",
        });
    }
    let exact = [
        (0.50, 0.24, "JLCPCB IC stencil pitch 0.5 mm: W=0.24 mm"),
        (0.40, 0.19, "JLCPCB IC stencil pitch 0.4 mm: W=0.19 mm"),
        (0.35, 0.17, "JLCPCB IC stencil pitch 0.35 mm: W=0.17 mm"),
        (0.30, 0.16, "JLCPCB IC stencil pitch 0.3 mm: W=0.16 mm"),
    ];
    exact
        .iter()
        .find(|(pitch, _, _)| (pin_pitch_mm - *pitch).abs() <= 1.0e-9)
        .map(|(_, width, source_condition)| IcPinApertureSpec {
            min_mm: *width,
            max_mm: *width,
            length_mm: None,
            source_condition,
        })
}

fn aperture_width_out_of_range(aperture_width_mm: f64, range: IcPinApertureSpec) -> bool {
    aperture_width_mm + f64::EPSILON < range.min_mm
        || aperture_width_mm > range.max_mm + f64::EPSILON
}

fn aperture_length_out_of_range(aperture_length_mm: f64, spec: IcPinApertureSpec) -> bool {
    let Some(expected_length_mm) = spec.length_mm else {
        return false;
    };
    (aperture_length_mm - expected_length_mm).abs() > f64::EPSILON
}

fn segment_aperture_length_mm(segment: &LayoutCopperSegment) -> f64 {
    let dx_mm = segment.end.x_mm - segment.start.x_mm;
    let dy_mm = segment.end.y_mm - segment.start.y_mm;
    dx_mm.hypot(dy_mm) + segment.width_mm
}

fn paste_object_is_pad_owned(object: CopperObjectRef<'_>) -> bool {
    match object {
        CopperObjectRef::Feature { feature, .. } => feature.owner_kind.as_deref() == Some("pad"),
        CopperObjectRef::Segment { segment, .. } => segment.owner_kind.as_deref() == Some("pad"),
        CopperObjectRef::Region { region, .. } => region.owner_kind.as_deref() == Some("pad"),
    }
}

fn paste_object_matches_target(object: CopperObjectRef<'_>, scenario: &Scenario) -> bool {
    let Some(target) = &scenario.target else {
        return true;
    };
    match object {
        CopperObjectRef::Feature { feature, .. } => {
            feature.component.as_deref() == Some(target.component.as_str())
        }
        CopperObjectRef::Segment { segment, .. } => {
            segment.component.as_deref() == Some(target.component.as_str())
        }
        CopperObjectRef::Region { region, .. } => {
            region.component.as_deref() == Some(target.component.as_str())
        }
    }
}

fn region_bounding_box_min_dimension_mm(region: &LayoutCopperRegion) -> Option<f64> {
    region_bounding_box_dimensions_mm(region).map(|(width_mm, height_mm)| width_mm.min(height_mm))
}

fn region_bounding_box_max_dimension_mm(region: &LayoutCopperRegion) -> Option<f64> {
    region_bounding_box_dimensions_mm(region).map(|(width_mm, height_mm)| width_mm.max(height_mm))
}

fn region_bounding_box_dimensions_mm(region: &LayoutCopperRegion) -> Option<(f64, f64)> {
    let first = region.points.first()?;
    let mut min_x = first.x_mm;
    let mut max_x = first.x_mm;
    let mut min_y = first.y_mm;
    let mut max_y = first.y_mm;
    for point in &region.points {
        if !point.x_mm.is_finite() || !point.y_mm.is_finite() {
            return None;
        }
        min_x = min_x.min(point.x_mm);
        max_x = max_x.max(point.x_mm);
        min_y = min_y.min(point.y_mm);
        max_y = max_y.max(point.y_mm);
    }
    let width_mm = max_x - min_x;
    let height_mm = max_y - min_y;
    (width_mm > 0.0 && height_mm > 0.0).then_some((width_mm, height_mm))
}

fn solder_paste_ic_pin_aperture_width_finding(
    scenario: &Scenario,
    paste_object: CopperObjectRef<'_>,
    aperture_width_mm: f64,
    pin_pitch_mm: f64,
    aperture_spec: IcPinApertureSpec,
) -> Finding {
    let mut finding = Finding::critical(
        SOLDER_PASTE_IC_PIN_APERTURE_VALID,
        scenario.name.clone(),
        format!(
            "Solder-paste {} opening on {} has IC pin aperture width {:.6} mm; JLCPCB pitch-conditioned range for {:.3} mm pitch is {:.6}..={:.6} mm.",
            paste_object.kind(),
            paste_object.layer(),
            aperture_width_mm,
            pin_pitch_mm,
            aperture_spec.min_mm,
            aperture_spec.max_mm
        ),
    );
    finding.suggested_fixes = vec![
        "Adjust the IC pin stencil aperture width to match the selected JLCPCB pitch-conditioned stencil opening standard.".to_string(),
        "Use a package-specific scenario with the correct pin_pitch_mm instead of applying this IC rule to unrelated pads.".to_string(),
        "If the paste layer must remain as exported, document the order remark or assembly-process override and do not use this JLC default optimization check.".to_string(),
    ];
    insert_solder_paste_object_measurements(&mut finding, paste_object);
    finding.measured.insert(
        "solder_paste_ic_pin_aperture_width_mm".to_string(),
        json!(aperture_width_mm),
    );
    finding
        .measured
        .insert("pin_pitch_mm".to_string(), json!(pin_pitch_mm));
    finding.measured.insert(
        "source_condition".to_string(),
        json!(aperture_spec.source_condition),
    );
    finding.limit.insert(
        "min_solder_paste_ic_pin_aperture_width_mm".to_string(),
        json!(aperture_spec.min_mm),
    );
    finding.limit.insert(
        "max_solder_paste_ic_pin_aperture_width_mm".to_string(),
        json!(aperture_spec.max_mm),
    );
    finding
}

fn solder_paste_ic_pin_aperture_length_finding(
    scenario: &Scenario,
    paste_object: CopperObjectRef<'_>,
    aperture_length_mm: f64,
    pin_pitch_mm: f64,
    aperture_spec: IcPinApertureSpec,
) -> Finding {
    let expected_length_mm = aperture_spec
        .length_mm
        .expect("length finding requires length-constrained IC aperture spec");
    let mut finding = Finding::critical(
        SOLDER_PASTE_IC_PIN_APERTURE_VALID,
        scenario.name.clone(),
        format!(
            "Solder-paste {} opening on {} has IC pin aperture length {:.6} mm; JLCPCB pitch-conditioned length for {:.3} mm pitch is {:.6} mm.",
            paste_object.kind(),
            paste_object.layer(),
            aperture_length_mm,
            pin_pitch_mm,
            expected_length_mm
        ),
    );
    finding.suggested_fixes = vec![
        "Adjust the IC pin stencil aperture length to match the selected JLCPCB pitch-conditioned stencil opening standard.".to_string(),
        "Use a package-specific scenario with the correct pin_pitch_mm instead of applying this IC rule to unrelated pads.".to_string(),
        "If the paste layer must remain as exported, document the order remark or assembly-process override and do not use this JLC default optimization check.".to_string(),
    ];
    insert_solder_paste_object_measurements(&mut finding, paste_object);
    finding.measured.insert(
        "solder_paste_ic_pin_aperture_length_mm".to_string(),
        json!(aperture_length_mm),
    );
    finding
        .measured
        .insert("pin_pitch_mm".to_string(), json!(pin_pitch_mm));
    finding.measured.insert(
        "source_condition".to_string(),
        json!(aperture_spec.source_condition),
    );
    finding.limit.insert(
        "solder_paste_ic_pin_aperture_length_mm".to_string(),
        json!(expected_length_mm),
    );
    finding
}

fn insert_solder_paste_object_measurements(finding: &mut Finding, object: CopperObjectRef<'_>) {
    finding
        .measured
        .insert("solder_paste_kind".to_string(), json!(object.kind()));
    match object {
        CopperObjectRef::Feature { feature, index } => {
            finding
                .measured
                .insert("solder_paste_feature_index".to_string(), json!(index));
            finding.measured.insert(
                "solder_paste_feature_x_mm".to_string(),
                json!(feature.at.x_mm),
            );
            finding.measured.insert(
                "solder_paste_feature_y_mm".to_string(),
                json!(feature.at.y_mm),
            );
            finding.measured.insert(
                "solder_paste_feature_layer".to_string(),
                json!(feature.layer),
            );
            insert_optional_copper_feature_owner_measurements(
                finding,
                "solder_paste_feature",
                feature,
            );
            finding.measured.insert(
                "solder_paste_feature_aperture".to_string(),
                json!(feature.aperture),
            );
            finding.measured.insert(
                "solder_paste_feature_shape".to_string(),
                json!(feature.shape),
            );
            finding.measured.insert(
                "solder_paste_feature_size_x_mm".to_string(),
                json!(feature.size.x_mm),
            );
            finding.measured.insert(
                "solder_paste_feature_size_y_mm".to_string(),
                json!(feature.size.y_mm),
            );
            finding.measured.insert(
                "solder_paste_feature_source_primitive".to_string(),
                json!(feature.source_primitive),
            );
            finding.measured.insert(
                "solder_paste_feature_source_primitive_index".to_string(),
                json!(feature.source_primitive_index),
            );
        }
        CopperObjectRef::Segment { segment, index } => {
            finding
                .measured
                .insert("solder_paste_segment_index".to_string(), json!(index));
            finding.measured.insert(
                "solder_paste_segment_start".to_string(),
                json!({
                    "x_mm": segment.start.x_mm,
                    "y_mm": segment.start.y_mm,
                }),
            );
            finding.measured.insert(
                "solder_paste_segment_end".to_string(),
                json!({
                    "x_mm": segment.end.x_mm,
                    "y_mm": segment.end.y_mm,
                }),
            );
            finding.measured.insert(
                "solder_paste_segment_layer".to_string(),
                json!(segment.layer),
            );
            insert_optional_artwork_segment_owner_measurements(
                finding,
                "solder_paste_segment",
                segment,
            );
            finding.measured.insert(
                "solder_paste_segment_aperture".to_string(),
                json!(segment.aperture),
            );
            finding.measured.insert(
                "solder_paste_segment_width_mm".to_string(),
                json!(segment.width_mm),
            );
            finding.measured.insert(
                "solder_paste_segment_source_primitive".to_string(),
                json!(segment.source_primitive),
            );
            finding.measured.insert(
                "solder_paste_segment_source_primitive_index".to_string(),
                json!(segment.source_primitive_index),
            );
        }
        CopperObjectRef::Region { region, index } => {
            finding
                .measured
                .insert("solder_paste_region_index".to_string(), json!(index));
            finding
                .measured
                .insert("solder_paste_region_layer".to_string(), json!(region.layer));
            insert_optional_artwork_region_owner_measurements(
                finding,
                "solder_paste_region",
                region,
            );
            finding.measured.insert(
                "solder_paste_region_source_primitive".to_string(),
                json!(region.source_primitive),
            );
            finding.measured.insert(
                "solder_paste_region_source_primitive_index".to_string(),
                json!(region.source_primitive_index),
            );
            finding.measured.insert(
                "solder_paste_region_point_count".to_string(),
                json!(region.points.len()),
            );
        }
    }
}

fn insert_optional_artwork_segment_owner_measurements(
    finding: &mut Finding,
    prefix: &str,
    segment: &LayoutCopperSegment,
) {
    insert_optional_artwork_owner_measurements(
        finding,
        prefix,
        ArtworkOwnerMeasurements {
            net: segment.net.as_deref(),
            island_id: segment.island_id.as_deref(),
            owner_kind: segment.owner_kind.as_deref(),
            component: segment.component.as_deref(),
            pin: segment.pin.as_deref(),
            via_index: segment.via_index,
        },
    );
}

fn insert_optional_artwork_region_owner_measurements(
    finding: &mut Finding,
    prefix: &str,
    region: &LayoutCopperRegion,
) {
    insert_optional_artwork_owner_measurements(
        finding,
        prefix,
        ArtworkOwnerMeasurements {
            net: region.net.as_deref(),
            island_id: region.island_id.as_deref(),
            owner_kind: region.owner_kind.as_deref(),
            component: region.component.as_deref(),
            pin: region.pin.as_deref(),
            via_index: region.via_index,
        },
    );
}

struct ArtworkOwnerMeasurements<'a> {
    net: Option<&'a str>,
    island_id: Option<&'a str>,
    owner_kind: Option<&'a str>,
    component: Option<&'a str>,
    pin: Option<&'a str>,
    via_index: Option<usize>,
}

fn insert_optional_artwork_owner_measurements(
    finding: &mut Finding,
    prefix: &str,
    owner: ArtworkOwnerMeasurements<'_>,
) {
    if let Some(net) = owner.net {
        finding.measured.insert(format!("{prefix}_net"), json!(net));
    }
    if let Some(island_id) = owner.island_id {
        finding
            .measured
            .insert(format!("{prefix}_island_id"), json!(island_id));
    }
    if let Some(owner_kind) = owner.owner_kind {
        finding
            .measured
            .insert(format!("{prefix}_owner_kind"), json!(owner_kind));
    }
    if let Some(component) = owner.component {
        finding
            .measured
            .insert(format!("{prefix}_component"), json!(component));
    }
    if let Some(pin) = owner.pin {
        finding.measured.insert(format!("{prefix}_pin"), json!(pin));
    }
    if let Some(via_index) = owner.via_index {
        finding
            .measured
            .insert(format!("{prefix}_via_index"), json!(via_index));
    }
}
