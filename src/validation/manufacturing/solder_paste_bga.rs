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

    let mut selected_features = Vec::new();
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
        selected_features.push((feature_index, feature));
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

    if selected_features.is_empty() {
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
        return;
    }

    if selected_features.len() < 4 {
        validation_input_missing(
            findings,
            scenario,
            "SOLDER_PASTE_BGA_APERTURE_VALID requires at least four pad-owned solder-paste flash features to prove a two-axis BGA pitch grid.",
        );
        return;
    }
    let (horizontal_gaps, vertical_gaps) =
        count_axis_aligned_pitch_gaps(&selected_features, pin_pitch_mm);
    if horizontal_gaps < 2 || vertical_gaps < 2 {
        findings.push(solder_paste_bga_pitch_grid_finding(
            scenario,
            selected_features.len(),
            horizontal_gaps,
            vertical_gaps,
            pin_pitch_mm,
            aperture,
        ));
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

fn count_axis_aligned_pitch_gaps(
    features: &[(usize, &crate::board_ir::LayoutCopperFeature)],
    pin_pitch_mm: f64,
) -> (usize, usize) {
    const TOLERANCE_MM: f64 = 0.01;
    let mut horizontal_gaps = 0usize;
    let mut vertical_gaps = 0usize;
    for (index, (_, first)) in features.iter().enumerate() {
        for (_, second) in features.iter().skip(index + 1) {
            let dx = first.at.x_mm - second.at.x_mm;
            let dy = first.at.y_mm - second.at.y_mm;
            if dy.abs() <= TOLERANCE_MM && (dx.abs() - pin_pitch_mm).abs() <= TOLERANCE_MM {
                horizontal_gaps += 1;
            }
            if dx.abs() <= TOLERANCE_MM && (dy.abs() - pin_pitch_mm).abs() <= TOLERANCE_MM {
                vertical_gaps += 1;
            }
        }
    }
    (horizontal_gaps, vertical_gaps)
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

fn solder_paste_bga_pitch_grid_finding(
    scenario: &Scenario,
    feature_count: usize,
    horizontal_gaps: usize,
    vertical_gaps: usize,
    pin_pitch_mm: f64,
    aperture: BgaApertureSize,
) -> Finding {
    let mut finding = Finding::critical(
        SOLDER_PASTE_BGA_APERTURE_VALID,
        scenario.name.clone(),
        format!(
            "Pad-owned solder-paste flashes do not prove a two-axis BGA grid at {:.3} mm pitch; found {} horizontal and {} vertical matching gaps.",
            pin_pitch_mm, horizontal_gaps, vertical_gaps
        ),
    );
    finding.suggested_fixes = vec![
        "Use the BGA aperture rule only for the component whose pad-owned paste flashes prove the declared BGA pitch in both axes.".to_string(),
        "Correct parameters.pin_pitch_mm if the BGA package pitch was entered incorrectly.".to_string(),
        "If only partial paste evidence was imported, import the full paste layer and pad ownership evidence before applying this package-specific check.".to_string(),
    ];
    finding.measured.insert(
        "solder_paste_bga_feature_count".to_string(),
        json!(feature_count),
    );
    finding.measured.insert(
        "solder_paste_bga_horizontal_pitch_gap_count".to_string(),
        json!(horizontal_gaps),
    );
    finding.measured.insert(
        "solder_paste_bga_vertical_pitch_gap_count".to_string(),
        json!(vertical_gaps),
    );
    finding
        .measured
        .insert("pin_pitch_mm".to_string(), json!(pin_pitch_mm));
    finding.measured.insert(
        "source_condition".to_string(),
        json!(aperture.source_condition),
    );
    finding.limit.insert(
        "min_solder_paste_bga_horizontal_pitch_gap_count".to_string(),
        json!(2),
    );
    finding.limit.insert(
        "min_solder_paste_bga_vertical_pitch_gap_count".to_string(),
        json!(2),
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
