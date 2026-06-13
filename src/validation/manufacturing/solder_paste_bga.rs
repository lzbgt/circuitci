use crate::board_ir::Scenario;
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use super::super::SOLDER_PASTE_BGA_APERTURE_VALID;
use super::super::common::validation_input_missing;
use super::geometry::{CopperObjectRef, validate_copper_feature_geometry};
use super::{insert_optional_copper_feature_owner_measurements, required_numeric_parameter};

pub(in crate::validation) fn validate_solder_paste_bga_aperture(
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
    let Some(aperture) = jlc_bga_aperture_size(pin_pitch_mm) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "SOLDER_PASTE_BGA_APERTURE_VALID has no source-backed JLCPCB BGA aperture size for pin_pitch_mm={pin_pitch_mm:.6}."
            ),
        );
        return;
    };
    let paste = &bound.project.board.layout.solder_paste;
    if paste.features.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "SOLDER_PASTE_BGA_APERTURE_VALID requires board.layout.solder_paste feature evidence.",
        );
        return;
    }

    let mut checked_pad_owned_features = 0usize;
    for (feature_index, feature) in paste.features.iter().enumerate() {
        if let Err(message) = validate_copper_feature_geometry(feature, feature_index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        if feature.owner_kind.as_deref() != Some("pad") {
            continue;
        }
        if !feature_matches_target(feature.component.as_deref(), scenario) {
            continue;
        }
        checked_pad_owned_features += 1;
        let aperture_size_mm = feature.size.x_mm.min(feature.size.y_mm);
        if (aperture_size_mm - aperture.size_mm).abs() > f64::EPSILON {
            findings.push(solder_paste_bga_aperture_finding(
                scenario,
                CopperObjectRef::Feature {
                    feature,
                    index: feature_index,
                },
                aperture_size_mm,
                pin_pitch_mm,
                aperture,
            ));
        }
    }

    if checked_pad_owned_features == 0 {
        let message = scenario.target.as_ref().map_or_else(
            || {
                "SOLDER_PASTE_BGA_APERTURE_VALID requires pad-owned board.layout.solder_paste feature evidence.".to_string()
            },
            |target| {
                format!(
                    "SOLDER_PASTE_BGA_APERTURE_VALID requires pad-owned board.layout.solder_paste feature evidence for target component {}.",
                    target.component
                )
            },
        );
        validation_input_missing(findings, scenario, message);
    }
}

#[derive(Clone, Copy)]
struct BgaApertureSize {
    size_mm: f64,
    source_condition: &'static str,
}

fn jlc_bga_aperture_size(pin_pitch_mm: f64) -> Option<BgaApertureSize> {
    let exact = [
        (
            0.40,
            0.23,
            "JLCPCB BGA stencil pitch 0.4 mm: open 0.23 mm square with rounded corners",
        ),
        (0.45, 0.26, "JLCPCB BGA stencil pitch 0.45 mm: open 0.26 mm"),
        (0.50, 0.30, "JLCPCB BGA stencil pitch 0.5 mm: open 0.30 mm"),
        (0.65, 0.35, "JLCPCB BGA stencil pitch 0.65 mm: open 0.35 mm"),
        (0.80, 0.45, "JLCPCB BGA stencil pitch 0.8 mm: open 0.45 mm"),
        (1.00, 0.55, "JLCPCB BGA stencil pitch 1.0 mm: open 0.55 mm"),
        (1.27, 0.65, "JLCPCB BGA stencil pitch 1.27 mm: open 0.65 mm"),
    ];
    exact
        .iter()
        .find(|(pitch, _, _)| (pin_pitch_mm - *pitch).abs() <= 1.0e-9)
        .map(|(_, size, source_condition)| BgaApertureSize {
            size_mm: *size,
            source_condition,
        })
}

fn feature_matches_target(component: Option<&str>, scenario: &Scenario) -> bool {
    let Some(target) = &scenario.target else {
        return true;
    };
    component == Some(target.component.as_str())
}

fn solder_paste_bga_aperture_finding(
    scenario: &Scenario,
    paste_object: CopperObjectRef<'_>,
    aperture_size_mm: f64,
    pin_pitch_mm: f64,
    aperture: BgaApertureSize,
) -> Finding {
    let mut finding = Finding::critical(
        SOLDER_PASTE_BGA_APERTURE_VALID,
        scenario.name.clone(),
        format!(
            "Solder-paste {} opening on {} has BGA aperture size {:.6} mm; JLCPCB source size for {:.3} mm BGA pitch is {:.6} mm.",
            paste_object.kind(),
            paste_object.layer(),
            aperture_size_mm,
            pin_pitch_mm,
            aperture.size_mm
        ),
    );
    finding.suggested_fixes = vec![
        "Adjust the BGA stencil aperture size to match the selected JLCPCB pitch-conditioned stencil opening standard.".to_string(),
        "Use a package-specific scenario with the correct pin_pitch_mm instead of applying this BGA rule to unrelated pads.".to_string(),
        "If the paste layer must remain as exported, document the order remark or assembly-process override and do not use this JLC default optimization check.".to_string(),
    ];
    insert_solder_paste_feature_measurements(&mut finding, paste_object);
    finding.measured.insert(
        "solder_paste_bga_aperture_size_mm".to_string(),
        json!(aperture_size_mm),
    );
    finding
        .measured
        .insert("pin_pitch_mm".to_string(), json!(pin_pitch_mm));
    finding.measured.insert(
        "source_condition".to_string(),
        json!(aperture.source_condition),
    );
    finding.limit.insert(
        "solder_paste_bga_aperture_size_mm".to_string(),
        json!(aperture.size_mm),
    );
    finding
}

fn insert_solder_paste_feature_measurements(finding: &mut Finding, object: CopperObjectRef<'_>) {
    let CopperObjectRef::Feature { feature, index } = object else {
        return;
    };
    finding
        .measured
        .insert("solder_paste_kind".to_string(), json!("feature"));
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
    insert_optional_copper_feature_owner_measurements(finding, "solder_paste_feature", feature);
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
