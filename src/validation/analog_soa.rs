use crate::board_ir::Scenario;
use crate::library::{ComponentModel, SoaCurve, SoaPoint};
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;

use super::SPICE_OPERATING_LIMIT;
use super::analog_operating_limits::{
    OperatingLimitProbes, ValidatedSoaCurve, duration_above_limit_us, transient_duration_us,
};
use super::analog_runner::NgspiceRun;

pub(super) fn validated_soa_curves(
    model: &ComponentModel,
) -> Result<Vec<ValidatedSoaCurve>, String> {
    let curves = &model
        .datasheet
        .as_ref()
        .ok_or_else(|| "missing datasheet metadata".to_string())?
        .safe_operating_area
        .vds_id_curves;
    if curves.is_empty() {
        return Err("missing safe_operating_area.vds_id_curves".to_string());
    }
    let mut names = BTreeSet::new();
    let mut validated = Vec::with_capacity(curves.len());
    for curve in curves {
        validate_soa_curve(curve, &mut names)?;
        validated.push(ValidatedSoaCurve {
            name: curve.name.clone(),
            pulse_width_us: curve.pulse_width_us,
            duty_cycle_max: curve.duty_cycle_max,
            source_document: curve.source_document.clone(),
            source_figure: curve.source_figure.clone(),
            digitization_method: curve.digitization.method.clone(),
            digitization_confidence: curve.digitization.confidence.clone(),
            digitization_note: curve.digitization.note.clone(),
            points: curve.points.clone(),
        });
    }
    validated.sort_by(|left, right| {
        left.pulse_width_us
            .partial_cmp(&right.pulse_width_us)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(validated)
}

fn validate_soa_curve(curve: &SoaCurve, names: &mut BTreeSet<String>) -> Result<(), String> {
    if curve.name.trim().is_empty() || !names.insert(curve.name.clone()) {
        return Err(format!("duplicate or empty SOA curve name {}", curve.name));
    }
    if !curve.pulse_width_us.is_finite() || curve.pulse_width_us <= 0.0 {
        return Err(format!(
            "SOA curve {} has invalid pulse_width_us",
            curve.name
        ));
    }
    if !curve.duty_cycle_max.is_finite()
        || curve.duty_cycle_max <= 0.0
        || curve.duty_cycle_max > 1.0
    {
        return Err(format!(
            "SOA curve {} has invalid duty_cycle_max",
            curve.name
        ));
    }
    if curve.source_document.trim().is_empty()
        || curve.source_figure.trim().is_empty()
        || curve.digitization.method.trim().is_empty()
        || curve.digitization.confidence.trim().is_empty()
    {
        return Err(format!("SOA curve {} lacks source metadata", curve.name));
    }
    if curve.points.len() < 2 {
        return Err(format!("SOA curve {} has fewer than 2 points", curve.name));
    }
    let mut previous_vds = 0.0;
    for point in &curve.points {
        if !point.vds_v.is_finite()
            || !point.id_a.is_finite()
            || point.vds_v <= 0.0
            || point.id_a <= 0.0
        {
            return Err(format!(
                "SOA curve {} has a nonpositive or nonfinite point",
                curve.name
            ));
        }
        if point.vds_v <= previous_vds {
            return Err(format!(
                "SOA curve {} points are not strictly increasing by VDS",
                curve.name
            ));
        }
        previous_vds = point.vds_v;
    }
    Ok(())
}

pub(super) fn invalid_soa_metadata_finding(
    component_id: &str,
    model: &ComponentModel,
    scenario_name: &str,
    message: String,
) -> Finding {
    let mut finding = Finding::critical(
        SPICE_OPERATING_LIMIT,
        scenario_name,
        format!(
            "Component {component_id} model {} has invalid safe-operating-area metadata: {message}.",
            model.component_id
        ),
    );
    finding
        .measured
        .insert("component".to_string(), json!(component_id));
    finding
        .measured
        .insert("model".to_string(), json!(model.component_id));
    finding
        .measured
        .insert("soa_metadata_error".to_string(), json!(message));
    finding
        .limit
        .insert("valid_soa_curve_required".to_string(), json!(true));
    finding.suggested_fixes.push(
        "Add strictly increasing positive VDS/ID SOA points with source document, source figure, digitization method, pulse width, and duty cycle metadata."
            .to_string(),
    );
    finding
}

pub(super) fn evaluate_soa_limits(
    scenario: &Scenario,
    run: &NgspiceRun,
    operating_limits: &OperatingLimitProbes,
    findings: &mut Vec<Finding>,
) {
    let soa_base = run.user_probe_count + operating_limits.probes.len();
    for (check_index, check) in operating_limits.soa_checks.iter().enumerate() {
        let vds_index = soa_base + check_index * 2;
        let id_index = vds_index + 1;
        let (Some(vds_values), Some(id_values)) = (
            run.series.values_by_probe.get(vds_index),
            run.series.values_by_probe.get(id_index),
        ) else {
            continue;
        };
        let duration_us = max_contiguous_duration_above_limit_us(
            &run.series.time_s,
            id_values,
            check.continuous_limit_a,
        );
        if duration_us <= 0.0 {
            continue;
        }
        let duty_cycle = {
            let total =
                duration_above_limit_us(&run.series.time_s, id_values, check.continuous_limit_a);
            let duration = transient_duration_us(&run.series.time_s);
            if duration > 0.0 {
                total / duration
            } else {
                1.0
            }
        };
        let (curve, duration_covered) = select_soa_curve(&check.curves, duration_us);
        let mut worst: Option<SoaSample> = None;
        for (index, ((vds, id), time_s)) in vds_values
            .iter()
            .copied()
            .zip(id_values.iter().copied())
            .zip(run.series.time_s.iter().copied())
            .enumerate()
        {
            if id <= check.continuous_limit_a {
                continue;
            }
            let limit = soa_id_limit(curve, vds);
            let (allowed_id_a, out_of_range) = match limit {
                SoaLimitAtVds::Allowed(allowed) => (allowed, false),
                SoaLimitAtVds::AboveRange(endpoint) => (endpoint, true),
            };
            let ratio = if allowed_id_a > 0.0 {
                id / allowed_id_a
            } else {
                f64::INFINITY
            };
            let violates = !duration_covered
                || duty_cycle > curve.duty_cycle_max
                || out_of_range
                || ratio > 1.0;
            if !violates {
                continue;
            }
            let sample = SoaSample {
                index,
                time_us: time_s * 1_000_000.0,
                vds_v: vds,
                id_a: id,
                allowed_id_a,
                ratio,
                vds_above_curve_range: out_of_range,
                duration_covered,
                duty_cycle,
                contiguous_duration_us: duration_us,
            };
            if worst
                .as_ref()
                .is_none_or(|existing| sample.ratio > existing.ratio)
            {
                worst = Some(sample);
            }
        }
        let Some(sample) = worst else {
            continue;
        };
        let mut finding = Finding::critical(
            SPICE_OPERATING_LIMIT,
            &scenario.name,
            format!(
                "Component {} exceeded digitized SOA screening curve {}: ID {:.6} A at VDS {:.6} V, allowed {:.6} A.",
                check.component_id, curve.name, sample.id_a, sample.vds_v, sample.allowed_id_a
            ),
        );
        finding
            .measured
            .insert("component".to_string(), json!(check.component_id));
        finding.measured.insert("rating".to_string(), json!("SOA"));
        finding
            .measured
            .insert("time_us".to_string(), json!(sample.time_us));
        finding
            .measured
            .insert("sample_index".to_string(), json!(sample.index));
        finding
            .measured
            .insert("vds_v".to_string(), json!(sample.vds_v));
        finding
            .measured
            .insert("id_a".to_string(), json!(sample.id_a));
        finding
            .measured
            .insert("soa_margin_ratio".to_string(), json!(sample.ratio));
        finding.measured.insert(
            "pulse_duration_us".to_string(),
            json!(sample.contiguous_duration_us),
        );
        finding
            .measured
            .insert("pulse_duty_cycle".to_string(), json!(sample.duty_cycle));
        finding.measured.insert(
            "vds_above_curve_range".to_string(),
            json!(sample.vds_above_curve_range),
        );
        finding.measured.insert(
            "duration_covered_by_curve".to_string(),
            json!(sample.duration_covered),
        );
        finding
            .limit
            .insert("id_limit_a".to_string(), json!(sample.allowed_id_a));
        finding
            .limit
            .insert("soa_curve".to_string(), json!(curve.name));
        finding.limit.insert(
            "curve_pulse_width_us".to_string(),
            json!(curve.pulse_width_us),
        );
        finding.limit.insert(
            "curve_duty_cycle_max".to_string(),
            json!(curve.duty_cycle_max),
        );
        finding
            .limit
            .insert("interpolation".to_string(), json!("log_log"));
        finding
            .limit
            .insert("source_document".to_string(), json!(curve.source_document));
        finding
            .limit
            .insert("source_figure".to_string(), json!(curve.source_figure));
        finding.limit.insert(
            "digitization_method".to_string(),
            json!(curve.digitization_method),
        );
        finding.limit.insert(
            "digitization_confidence".to_string(),
            json!(curve.digitization_confidence),
        );
        if let Some(note) = &curve.digitization_note {
            finding
                .limit
                .insert("digitization_warning".to_string(), json!(note));
        }
        finding.suggested_fixes.push(
            "Reduce VDS/ID stress, shorten the pulse, lower duty cycle, choose a larger MOSFET, or replace hand-digitized SOA metadata with vendor/bench-validated curve points.".to_string(),
        );
        findings.push(finding);
    }
}

struct SoaSample {
    index: usize,
    time_us: f64,
    vds_v: f64,
    id_a: f64,
    allowed_id_a: f64,
    ratio: f64,
    vds_above_curve_range: bool,
    duration_covered: bool,
    duty_cycle: f64,
    contiguous_duration_us: f64,
}

fn select_soa_curve(curves: &[ValidatedSoaCurve], duration_us: f64) -> (&ValidatedSoaCurve, bool) {
    if let Some(curve) = curves
        .iter()
        .find(|curve| curve.pulse_width_us >= duration_us)
    {
        return (curve, true);
    }
    (
        curves
            .last()
            .expect("SOA curves are validated as nonempty before evaluation"),
        false,
    )
}

enum SoaLimitAtVds {
    Allowed(f64),
    AboveRange(f64),
}

fn soa_id_limit(curve: &ValidatedSoaCurve, vds_v: f64) -> SoaLimitAtVds {
    let first = curve
        .points
        .first()
        .expect("SOA curves are validated with at least two points");
    let last = curve
        .points
        .last()
        .expect("SOA curves are validated with at least two points");
    if vds_v <= first.vds_v {
        return SoaLimitAtVds::Allowed(first.id_a);
    }
    if vds_v > last.vds_v {
        return SoaLimitAtVds::AboveRange(last.id_a);
    }
    for pair in curve.points.windows(2) {
        let left = &pair[0];
        let right = &pair[1];
        if vds_v <= right.vds_v {
            return SoaLimitAtVds::Allowed(log_log_interpolate_id(left, right, vds_v));
        }
    }
    SoaLimitAtVds::Allowed(last.id_a)
}

fn log_log_interpolate_id(left: &SoaPoint, right: &SoaPoint, vds_v: f64) -> f64 {
    let left_v = left.vds_v.log10();
    let right_v = right.vds_v.log10();
    let t = if right_v > left_v {
        (vds_v.log10() - left_v) / (right_v - left_v)
    } else {
        0.0
    };
    10_f64.powf(left.id_a.log10() + t * (right.id_a.log10() - left.id_a.log10()))
}

fn max_contiguous_duration_above_limit_us(time_s: &[f64], values: &[f64], limit: f64) -> f64 {
    if time_s.len() < 2 || values.len() < 2 {
        return 0.0;
    }
    let mut current = 0.0;
    let mut max_duration = 0.0;
    for (time_pair, value_pair) in time_s.windows(2).zip(values.windows(2)) {
        let interval_s = time_pair[1] - time_pair[0];
        if interval_s <= 0.0 {
            continue;
        }
        if value_pair[0] > limit && value_pair[1] > limit {
            current += interval_s * 1_000_000.0;
            if current > max_duration {
                max_duration = current;
            }
        } else {
            current = 0.0;
        }
    }
    max_duration
}

#[cfg(test)]
mod tests {
    use super::{SoaPoint, log_log_interpolate_id, max_contiguous_duration_above_limit_us};

    #[test]
    fn soa_log_log_interpolation_uses_vds_id_curve_points() {
        let left = SoaPoint {
            vds_v: 10.0,
            id_a: 100.0,
        };
        let right = SoaPoint {
            vds_v: 100.0,
            id_a: 10.0,
        };
        let interpolated = log_log_interpolate_id(&left, &right, 31.6227766017);
        assert!((interpolated - 31.6227766017).abs() < 1e-9);
    }

    #[test]
    fn soa_duration_uses_max_contiguous_overstress_window() {
        let time_s = [0.0, 10e-6, 20e-6, 30e-6, 40e-6, 50e-6];
        let values = [0.0, 13.0, 14.0, 0.0, 15.0, 0.0];
        let duration = max_contiguous_duration_above_limit_us(&time_s, &values, 12.0);
        assert!((duration - 10.0).abs() < 1e-9);
    }
}
