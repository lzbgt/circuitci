use crate::board_ir::{LayoutFootprintBounds, LayoutPinMarker, LayoutPoint, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use super::super::PIN_1_ORIENTATION_VALID;
use super::super::common::validation_input_missing;

const MIN_PIN1_VECTOR_MM: f64 = 0.001;

pub(in crate::validation) fn validate_pin_1_orientation(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(target) = scenario.target.as_ref() else {
        validation_input_missing(
            findings,
            scenario,
            "PIN_1_ORIENTATION_VALID requires scenario.target.component.",
        );
        return;
    };
    let Some(expected_direction_deg) =
        required_direction_parameter(scenario, "expected_pin_1_direction_deg", findings)
    else {
        return;
    };
    let Some(max_error_deg) =
        required_direction_parameter(scenario, "max_pin_1_direction_error_deg", findings)
    else {
        return;
    };
    if max_error_deg < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "PIN_1_ORIENTATION_VALID parameters.max_pin_1_direction_error_deg must be non-negative.",
        );
        return;
    }
    let Some(footprint) = bound
        .project
        .board
        .layout
        .footprints
        .get(target.component.as_str())
    else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "PIN_1_ORIENTATION_VALID requires board.layout.footprints.{} evidence.",
                target.component
            ),
        );
        return;
    };
    let Some(semantics) = footprint.semantics.as_ref() else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "PIN_1_ORIENTATION_VALID requires board.layout.footprints.{}.semantics evidence.",
                target.component
            ),
        );
        return;
    };
    let Some(body_bounds) = semantics.body_bounds.as_ref() else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "PIN_1_ORIENTATION_VALID requires board.layout.footprints.{}.semantics.body_bounds evidence.",
                target.component
            ),
        );
        return;
    };
    let Some(pin_1) = semantics.pin_1.as_ref() else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "PIN_1_ORIENTATION_VALID requires board.layout.footprints.{}.semantics.pin_1 evidence.",
                target.component
            ),
        );
        return;
    };
    let Some(body_center) = body_center(body_bounds) else {
        validation_input_missing(
            findings,
            scenario,
            "PIN_1_ORIENTATION_VALID requires finite body_bounds coordinates.",
        );
        return;
    };
    let Some(measured_direction_deg) = pin_1_direction_deg(pin_1, &body_center) else {
        validation_input_missing(
            findings,
            scenario,
            "PIN_1_ORIENTATION_VALID requires pin_1 to be offset from the imported body center.",
        );
        return;
    };
    let direction_error_deg = rotation_delta_deg(expected_direction_deg, measured_direction_deg);
    if direction_error_deg > max_error_deg + f64::EPSILON {
        let mut finding = Finding::critical(
            PIN_1_ORIENTATION_VALID,
            &scenario.name,
            format!(
                "Component {} imported pin-1 marker direction {:.3} deg differs from expected {:.3} deg by {:.3} deg.",
                target.component,
                measured_direction_deg,
                expected_direction_deg,
                direction_error_deg
            ),
        );
        finding.component = Some(target.component.clone());
        finding.measured.extend([
            ("component".to_string(), json!(target.component)),
            ("pin_1_x_mm".to_string(), json!(pin_1.at.x_mm)),
            ("pin_1_y_mm".to_string(), json!(pin_1.at.y_mm)),
            ("pin_1_source".to_string(), json!(pin_1.source.as_deref())),
            ("body_center_x_mm".to_string(), json!(body_center.x_mm)),
            ("body_center_y_mm".to_string(), json!(body_center.y_mm)),
            (
                "body_bounds_source".to_string(),
                json!(body_bounds.source.as_deref()),
            ),
            (
                "measured_pin_1_direction_deg".to_string(),
                json!(measured_direction_deg),
            ),
            (
                "expected_pin_1_direction_deg".to_string(),
                json!(expected_direction_deg),
            ),
            (
                "pin_1_direction_error_deg".to_string(),
                json!(direction_error_deg),
            ),
        ]);
        finding.limit.insert(
            "max_pin_1_direction_error_deg".to_string(),
            json!(max_error_deg),
        );
        finding.suggested_fixes = vec![
            "Review the imported KiCad footprint pad-1 marker, placement rotation, and package pin-1 convention.".to_string(),
            "Correct the footprint, placement, or explicit expected_pin_1_direction_deg only after confirming the assembly drawing.".to_string(),
        ];
        findings.push(finding);
    }
}

fn required_direction_parameter(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    let value = scenario
        .parameters
        .get(name)
        .and_then(serde_yaml_ng::Value::as_f64);
    let Some(value) = value else {
        validation_input_missing(
            findings,
            scenario,
            format!("PIN_1_ORIENTATION_VALID parameters.{name} must be a finite number."),
        );
        return None;
    };
    if !value.is_finite() {
        validation_input_missing(
            findings,
            scenario,
            format!("PIN_1_ORIENTATION_VALID parameters.{name} must be finite."),
        );
        return None;
    }
    Some(value)
}

fn body_center(bounds: &LayoutFootprintBounds) -> Option<LayoutPoint> {
    if !bounds.min.x_mm.is_finite()
        || !bounds.min.y_mm.is_finite()
        || !bounds.max.x_mm.is_finite()
        || !bounds.max.y_mm.is_finite()
    {
        return None;
    }
    Some(LayoutPoint {
        x_mm: (bounds.min.x_mm + bounds.max.x_mm) / 2.0,
        y_mm: (bounds.min.y_mm + bounds.max.y_mm) / 2.0,
    })
}

fn pin_1_direction_deg(pin_1: &LayoutPinMarker, center: &LayoutPoint) -> Option<f64> {
    if !pin_1.at.x_mm.is_finite() || !pin_1.at.y_mm.is_finite() {
        return None;
    }
    let dx = pin_1.at.x_mm - center.x_mm;
    let dy = pin_1.at.y_mm - center.y_mm;
    if dx.hypot(dy) < MIN_PIN1_VECTOR_MM {
        return None;
    }
    Some(normalize_degrees(dy.atan2(dx).to_degrees()))
}

fn rotation_delta_deg(expected_deg: f64, measured_deg: f64) -> f64 {
    let delta = (normalize_degrees(measured_deg) - normalize_degrees(expected_deg)).abs();
    delta.min(360.0 - delta)
}

fn normalize_degrees(value: f64) -> f64 {
    value.rem_euclid(360.0)
}
