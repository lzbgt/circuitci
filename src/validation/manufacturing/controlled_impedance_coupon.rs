use crate::board_ir::{
    ControlledImpedanceCoupon, ControlledImpedanceCouponSample, ControlledImpedanceCouponType,
    NetRoute, RouteSegment, Scenario,
};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;

use super::super::common::validation_input_missing;
use super::super::{
    CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID, CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID,
    CONTROLLED_IMPEDANCE_COUPON_VALID,
};

const IMPEDANCE_MATCH_EPSILON_OHM: f64 = 1.0e-9;

pub(in crate::validation) fn validate_controlled_impedance_coupon(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(names) = coupon_names(scenario, findings) else {
        return;
    };
    let coupons = &bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .coupons;
    if coupons.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "CONTROLLED_IMPEDANCE_COUPON_VALID requires board.manufacturing.controlled_impedance.coupons evidence.",
        );
        return;
    }
    for name in names {
        let Some(coupon) = named_coupon(coupons, scenario, findings, &name) else {
            return;
        };
        if !coupon_has_valid_metadata(bound, scenario, findings, coupon) {
            return;
        }
        let impedance_error_ohm =
            (coupon.measured_impedance_ohm - coupon.target_impedance_ohm).abs();
        if impedance_error_ohm > coupon.max_impedance_error_ohm {
            findings.push(coupon_finding(scenario, coupon, impedance_error_ohm));
        }
    }
}

pub(in crate::validation) fn validate_controlled_impedance_coupon_batch(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(names) =
        coupon_names_for_check(scenario, findings, CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID)
    else {
        return;
    };
    let coupons = &bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .coupons;
    if coupons.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID requires board.manufacturing.controlled_impedance.coupons evidence.",
        );
        return;
    }
    for name in names {
        let Some(coupon) = named_coupon_for_check(
            coupons,
            scenario,
            findings,
            &name,
            CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID,
        ) else {
            return;
        };
        if !coupon_has_valid_metadata(bound, scenario, findings, coupon) {
            return;
        }
        let Some(metrics) = batch_metrics(scenario, findings, coupon) else {
            return;
        };
        if metrics.sample_count < coupon.min_batch_sample_count.unwrap_or_default()
            || metrics.mean_impedance_error_ohm
                > coupon
                    .max_batch_mean_impedance_error_ohm
                    .unwrap_or(f64::INFINITY)
            || metrics.max_sample_impedance_error_ohm
                > coupon
                    .max_batch_sample_impedance_error_ohm
                    .unwrap_or(f64::INFINITY)
            || metrics.stddev_impedance_ohm > coupon.max_batch_stddev_ohm.unwrap_or(f64::INFINITY)
        {
            findings.push(coupon_batch_finding(scenario, coupon, &metrics));
        }
    }
}

pub(in crate::validation) fn validate_controlled_impedance_coupon_trace_correlation(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(names) = coupon_names_for_check(
        scenario,
        findings,
        CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID,
    ) else {
        return;
    };
    let coupons = &bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .coupons;
    if coupons.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID requires board.manufacturing.controlled_impedance.coupons evidence.",
        );
        return;
    }
    for name in names {
        let Some(coupon) = named_coupon_for_check(
            coupons,
            scenario,
            findings,
            &name,
            CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID,
        ) else {
            return;
        };
        if !coupon_has_valid_metadata(bound, scenario, findings, coupon) {
            return;
        }
        let Some(policy) = trace_correlation_policy(scenario, findings, coupon) else {
            return;
        };
        let Some(metrics) = trace_correlation_metrics(bound, scenario, findings, coupon, &policy)
        else {
            return;
        };
        if metrics.layer_mismatch
            || metrics.max_width_delta_mm > policy.max_trace_width_delta_mm + f64::EPSILON
            || metrics.max_gap_delta_mm.is_some_and(|gap| {
                gap > policy.max_trace_gap_delta_mm.unwrap_or(f64::INFINITY) + f64::EPSILON
            })
        {
            findings.push(trace_correlation_finding(
                scenario, coupon, &policy, &metrics,
            ));
        }
    }
}

fn coupon_names(scenario: &Scenario, findings: &mut Vec<Finding>) -> Option<Vec<String>> {
    coupon_names_for_check(scenario, findings, CONTROLLED_IMPEDANCE_COUPON_VALID)
}

fn coupon_names_for_check(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    check_id: &str,
) -> Option<Vec<String>> {
    let Some(value) = scenario.parameters.get("coupons") else {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} requires parameters.coupons."),
        );
        return None;
    };
    let Some(items) = value.as_sequence() else {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} parameters.coupons must be a list."),
        );
        return None;
    };
    if items.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} parameters.coupons must not be empty."),
        );
        return None;
    }
    let mut names = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, item) in items.iter().enumerate() {
        let Some(mapping) = item.as_mapping() else {
            validation_input_missing(
                findings,
                scenario,
                format!("{check_id} parameters.coupons[{index}] must be an object."),
            );
            return None;
        };
        let Some(value) = mapping.get("name") else {
            validation_input_missing(
                findings,
                scenario,
                format!("{check_id} parameters.coupons[{index}].name is required."),
            );
            return None;
        };
        let Some(name) = value
            .as_str()
            .map(str::trim)
            .filter(|name| !name.is_empty())
        else {
            validation_input_missing(
                findings,
                scenario,
                format!("{check_id} parameters.coupons[{index}].name must be a non-empty string."),
            );
            return None;
        };
        if !seen.insert(name.to_string()) {
            validation_input_missing(
                findings,
                scenario,
                format!("{check_id} parameters.coupons repeats coupon name {name}."),
            );
            return None;
        }
        names.push(name.to_string());
    }
    Some(names)
}

fn named_coupon<'a>(
    coupons: &'a [ControlledImpedanceCoupon],
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    name: &str,
) -> Option<&'a ControlledImpedanceCoupon> {
    named_coupon_for_check(
        coupons,
        scenario,
        findings,
        name,
        CONTROLLED_IMPEDANCE_COUPON_VALID,
    )
}

fn named_coupon_for_check<'a>(
    coupons: &'a [ControlledImpedanceCoupon],
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    name: &str,
    check_id: &str,
) -> Option<&'a ControlledImpedanceCoupon> {
    let mut matches = coupons.iter().filter(|coupon| coupon.name == name);
    let Some(coupon) = matches.next() else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "{check_id} coupon {name} is absent from board.manufacturing.controlled_impedance.coupons."
            ),
        );
        return None;
    };
    if matches.next().is_some() {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "{check_id} coupon name {name} is duplicated in board.manufacturing.controlled_impedance.coupons."
            ),
        );
        return None;
    }
    Some(coupon)
}

fn coupon_has_valid_metadata(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    coupon: &ControlledImpedanceCoupon,
) -> bool {
    if coupon.name.trim().is_empty() || coupon.source.trim().is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "CONTROLLED_IMPEDANCE_COUPON_VALID coupons must declare non-empty name and source.",
        );
        return false;
    }
    if !coupon.target_impedance_ohm.is_finite()
        || coupon.target_impedance_ohm <= 0.0
        || !coupon.measured_impedance_ohm.is_finite()
        || coupon.measured_impedance_ohm <= 0.0
        || !coupon.max_impedance_error_ohm.is_finite()
        || coupon.max_impedance_error_ohm < 0.0
    {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_COUPON_VALID coupon {} must declare finite positive target/measured impedance and non-negative max_impedance_error_ohm.",
                coupon.name
            ),
        );
        return false;
    }
    match coupon.coupon_type {
        ControlledImpedanceCouponType::SingleEnded => {
            let Some(net) = coupon
                .net
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "CONTROLLED_IMPEDANCE_COUPON_VALID single-ended coupon {} must declare net.",
                        coupon.name
                    ),
                );
                return false;
            };
            if coupon.first_net.is_some() || coupon.second_net.is_some() {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "CONTROLLED_IMPEDANCE_COUPON_VALID single-ended coupon {} must not declare first_net or second_net.",
                        coupon.name
                    ),
                );
                return false;
            }
            if !bound.project.board.nets.contains_key(net) {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "CONTROLLED_IMPEDANCE_COUPON_VALID coupon {} references undeclared net {net}.",
                        coupon.name
                    ),
                );
                return false;
            }
            if !coupon_matches_single_ended_target(bound, scenario, findings, coupon, net) {
                return false;
            }
        }
        ControlledImpedanceCouponType::Differential => {
            if coupon.net.is_some() {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "CONTROLLED_IMPEDANCE_COUPON_VALID differential coupon {} must not declare net.",
                        coupon.name
                    ),
                );
                return false;
            }
            let Some(first_net) = coupon
                .first_net
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "CONTROLLED_IMPEDANCE_COUPON_VALID differential coupon {} must declare first_net.",
                        coupon.name
                    ),
                );
                return false;
            };
            let Some(second_net) = coupon
                .second_net
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "CONTROLLED_IMPEDANCE_COUPON_VALID differential coupon {} must declare second_net.",
                        coupon.name
                    ),
                );
                return false;
            };
            if first_net == second_net {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "CONTROLLED_IMPEDANCE_COUPON_VALID differential coupon {} must declare two distinct nets.",
                        coupon.name
                    ),
                );
                return false;
            }
            for net in [first_net, second_net] {
                if !bound.project.board.nets.contains_key(net) {
                    validation_input_missing(
                        findings,
                        scenario,
                        format!(
                            "CONTROLLED_IMPEDANCE_COUPON_VALID coupon {} references undeclared net {net}.",
                            coupon.name
                        ),
                    );
                    return false;
                }
            }
            if !coupon_matches_differential_target(
                bound, scenario, findings, coupon, first_net, second_net,
            ) {
                return false;
            }
        }
    }
    true
}

fn coupon_matches_single_ended_target(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    coupon: &ControlledImpedanceCoupon,
    net: &str,
) -> bool {
    let targets = bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .nets
        .iter()
        .filter(|target| target.net == net)
        .collect::<Vec<_>>();
    let Some(target) = unique_target(scenario, findings, coupon, net, targets.len()) else {
        return false;
    };
    let target = targets[target];
    if !target.target_impedance_ohm.is_finite() || target.target_impedance_ohm <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_COUPON_VALID board target for coupon {} net {net} must declare finite positive target_impedance_ohm.",
                coupon.name
            ),
        );
        return false;
    }
    if (coupon.target_impedance_ohm - target.target_impedance_ohm).abs()
        > IMPEDANCE_MATCH_EPSILON_OHM
    {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_COUPON_VALID coupon {} target {:.3} ohm conflicts with reviewed board target {:.3} ohm for net {net}.",
                coupon.name, coupon.target_impedance_ohm, target.target_impedance_ohm
            ),
        );
        return false;
    }
    true
}

fn coupon_matches_differential_target(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    coupon: &ControlledImpedanceCoupon,
    first_net: &str,
    second_net: &str,
) -> bool {
    let targets = bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .differential_pairs
        .iter()
        .filter(|target| {
            unordered_pair_matches(&target.first_net, &target.second_net, first_net, second_net)
        })
        .collect::<Vec<_>>();
    let label = format!("{first_net}/{second_net}");
    let Some(target) = unique_target(scenario, findings, coupon, &label, targets.len()) else {
        return false;
    };
    let target = targets[target];
    if !target.target_differential_impedance_ohm.is_finite()
        || target.target_differential_impedance_ohm <= 0.0
    {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_COUPON_VALID board target for coupon {} pair {label} must declare finite positive target_differential_impedance_ohm.",
                coupon.name
            ),
        );
        return false;
    }
    if (coupon.target_impedance_ohm - target.target_differential_impedance_ohm).abs()
        > IMPEDANCE_MATCH_EPSILON_OHM
    {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_COUPON_VALID coupon {} target {:.3} ohm conflicts with reviewed board differential target {:.3} ohm for pair {label}.",
                coupon.name, coupon.target_impedance_ohm, target.target_differential_impedance_ohm
            ),
        );
        return false;
    }
    true
}

fn unique_target(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    coupon: &ControlledImpedanceCoupon,
    label: &str,
    count: usize,
) -> Option<usize> {
    match count {
        0 => {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_COUPON_VALID coupon {} requires exactly one reviewed board controlled-impedance target for {label}.",
                    coupon.name
                ),
            );
            None
        }
        1 => Some(0),
        _ => {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_COUPON_VALID coupon {} found duplicate reviewed board controlled-impedance targets for {label}.",
                    coupon.name
                ),
            );
            None
        }
    }
}

fn unordered_pair_matches(
    first: &str,
    second: &str,
    expected_first: &str,
    expected_second: &str,
) -> bool {
    (first == expected_first && second == expected_second)
        || (first == expected_second && second == expected_first)
}

fn coupon_finding(
    scenario: &Scenario,
    coupon: &ControlledImpedanceCoupon,
    impedance_error_ohm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        CONTROLLED_IMPEDANCE_COUPON_VALID,
        &scenario.name,
        format!(
            "Controlled-impedance coupon {} measured {:.2} ohm against {:.2} ohm target, exceeding {:.2} ohm reviewed tolerance.",
            coupon.name,
            coupon.measured_impedance_ohm,
            coupon.target_impedance_ohm,
            coupon.max_impedance_error_ohm
        ),
    );
    finding
        .measured
        .insert("coupon_name".to_string(), json!(coupon.name));
    finding
        .measured
        .insert("coupon_type".to_string(), json!(coupon_type_label(coupon)));
    finding
        .measured
        .insert("source".to_string(), json!(coupon.source));
    if let Some(net) = &coupon.net {
        finding.measured.insert("net".to_string(), json!(net));
    }
    if let Some(first_net) = &coupon.first_net {
        finding
            .measured
            .insert("first_net".to_string(), json!(first_net));
    }
    if let Some(second_net) = &coupon.second_net {
        finding
            .measured
            .insert("second_net".to_string(), json!(second_net));
    }
    finding.measured.insert(
        "target_impedance_ohm".to_string(),
        json!(coupon.target_impedance_ohm),
    );
    finding.measured.insert(
        "measured_impedance_ohm".to_string(),
        json!(coupon.measured_impedance_ohm),
    );
    finding.measured.insert(
        "impedance_error_ohm".to_string(),
        json!(impedance_error_ohm),
    );
    finding.limit.insert(
        "max_impedance_error_ohm".to_string(),
        json!(coupon.max_impedance_error_ohm),
    );
    finding.suggested_fixes = vec![
        "Review the fabricator coupon report against the controlled-impedance requirement.".to_string(),
        "Update stackup, trace geometry, or fabrication notes before accepting a coupon outside tolerance.".to_string(),
        "Do not treat this check as a field solver; it verifies explicit coupon measurement evidence and reviewed board-target consistency only.".to_string(),
    ];
    finding
}

#[derive(Debug)]
struct CouponBatchMetrics {
    sample_count: usize,
    mean_impedance_ohm: f64,
    mean_impedance_error_ohm: f64,
    max_sample_impedance_error_ohm: f64,
    stddev_impedance_ohm: f64,
}

fn batch_metrics(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    coupon: &ControlledImpedanceCoupon,
) -> Option<CouponBatchMetrics> {
    let Some(min_sample_count) = coupon.min_batch_sample_count else {
        missing_batch_field(scenario, findings, coupon, "min_batch_sample_count");
        return None;
    };
    let Some(max_mean_error) = coupon.max_batch_mean_impedance_error_ohm else {
        missing_batch_field(
            scenario,
            findings,
            coupon,
            "max_batch_mean_impedance_error_ohm",
        );
        return None;
    };
    let Some(max_sample_error) = coupon.max_batch_sample_impedance_error_ohm else {
        missing_batch_field(
            scenario,
            findings,
            coupon,
            "max_batch_sample_impedance_error_ohm",
        );
        return None;
    };
    let Some(max_stddev) = coupon.max_batch_stddev_ohm else {
        missing_batch_field(scenario, findings, coupon, "max_batch_stddev_ohm");
        return None;
    };
    if min_sample_count == 0
        || !max_mean_error.is_finite()
        || max_mean_error < 0.0
        || !max_sample_error.is_finite()
        || max_sample_error < 0.0
        || !max_stddev.is_finite()
        || max_stddev < 0.0
    {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID coupon {} must declare positive min_batch_sample_count and finite non-negative batch limits.",
                coupon.name
            ),
        );
        return None;
    }
    if coupon.samples.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID coupon {} requires reviewed samples.",
                coupon.name
            ),
        );
        return None;
    }
    let mut seen = BTreeSet::new();
    for sample in &coupon.samples {
        if !sample_has_valid_metadata(scenario, findings, coupon, sample, &mut seen) {
            return None;
        }
    }
    let sample_count = coupon.samples.len();
    let mean_impedance_ohm = coupon
        .samples
        .iter()
        .map(|sample| sample.measured_impedance_ohm)
        .sum::<f64>()
        / sample_count as f64;
    let mean_impedance_error_ohm = (mean_impedance_ohm - coupon.target_impedance_ohm).abs();
    let max_sample_impedance_error_ohm = coupon
        .samples
        .iter()
        .map(|sample| (sample.measured_impedance_ohm - coupon.target_impedance_ohm).abs())
        .fold(0.0, f64::max);
    let variance = coupon
        .samples
        .iter()
        .map(|sample| (sample.measured_impedance_ohm - mean_impedance_ohm).powi(2))
        .sum::<f64>()
        / sample_count as f64;
    Some(CouponBatchMetrics {
        sample_count,
        mean_impedance_ohm,
        mean_impedance_error_ohm,
        max_sample_impedance_error_ohm,
        stddev_impedance_ohm: variance.sqrt(),
    })
}

fn missing_batch_field(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    coupon: &ControlledImpedanceCoupon,
    field: &str,
) {
    validation_input_missing(
        findings,
        scenario,
        format!(
            "CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID coupon {} requires reviewed {field}.",
            coupon.name
        ),
    );
}

fn sample_has_valid_metadata(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    coupon: &ControlledImpedanceCoupon,
    sample: &ControlledImpedanceCouponSample,
    seen: &mut BTreeSet<String>,
) -> bool {
    if sample.name.trim().is_empty() || sample.source.trim().is_empty() {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID coupon {} samples must declare non-empty name and source.",
                coupon.name
            ),
        );
        return false;
    }
    if !seen.insert(sample.name.clone()) {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID coupon {} repeats sample name {}.",
                coupon.name, sample.name
            ),
        );
        return false;
    }
    if !sample.measured_impedance_ohm.is_finite() || sample.measured_impedance_ohm <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID coupon {} sample {} must declare finite positive measured_impedance_ohm.",
                coupon.name, sample.name
            ),
        );
        return false;
    }
    true
}

fn coupon_batch_finding(
    scenario: &Scenario,
    coupon: &ControlledImpedanceCoupon,
    metrics: &CouponBatchMetrics,
) -> Finding {
    let mut finding = Finding::critical(
        CONTROLLED_IMPEDANCE_COUPON_BATCH_VALID,
        &scenario.name,
        format!(
            "Controlled-impedance coupon {} batch statistics exceeded reviewed acceptance limits.",
            coupon.name
        ),
    );
    finding
        .measured
        .insert("coupon_name".to_string(), json!(coupon.name));
    finding
        .measured
        .insert("coupon_type".to_string(), json!(coupon_type_label(coupon)));
    finding
        .measured
        .insert("source".to_string(), json!(coupon.source));
    if let Some(net) = &coupon.net {
        finding.measured.insert("net".to_string(), json!(net));
    }
    if let Some(first_net) = &coupon.first_net {
        finding
            .measured
            .insert("first_net".to_string(), json!(first_net));
    }
    if let Some(second_net) = &coupon.second_net {
        finding
            .measured
            .insert("second_net".to_string(), json!(second_net));
    }
    finding.measured.insert(
        "target_impedance_ohm".to_string(),
        json!(coupon.target_impedance_ohm),
    );
    finding
        .measured
        .insert("sample_count".to_string(), json!(metrics.sample_count));
    finding.measured.insert(
        "mean_impedance_ohm".to_string(),
        json!(metrics.mean_impedance_ohm),
    );
    finding.measured.insert(
        "mean_impedance_error_ohm".to_string(),
        json!(metrics.mean_impedance_error_ohm),
    );
    finding.measured.insert(
        "max_sample_impedance_error_ohm".to_string(),
        json!(metrics.max_sample_impedance_error_ohm),
    );
    finding.measured.insert(
        "stddev_impedance_ohm".to_string(),
        json!(metrics.stddev_impedance_ohm),
    );
    finding.limit.insert(
        "min_batch_sample_count".to_string(),
        json!(coupon.min_batch_sample_count),
    );
    finding.limit.insert(
        "max_batch_mean_impedance_error_ohm".to_string(),
        json!(coupon.max_batch_mean_impedance_error_ohm),
    );
    finding.limit.insert(
        "max_batch_sample_impedance_error_ohm".to_string(),
        json!(coupon.max_batch_sample_impedance_error_ohm),
    );
    finding.limit.insert(
        "max_batch_stddev_ohm".to_string(),
        json!(coupon.max_batch_stddev_ohm),
    );
    finding.suggested_fixes = vec![
        "Review the complete fabricator coupon batch report against the controlled-impedance acceptance policy.".to_string(),
        "Check whether stackup, trace geometry, solder-mask loading, or fabrication process controls changed across the batch.".to_string(),
        "Do not treat this check as a field solver; it verifies explicit coupon sample evidence and reviewed batch limits only.".to_string(),
    ];
    finding
}

#[derive(Debug)]
struct TraceCorrelationPolicy {
    process_lot: String,
    panel_id: String,
    stackup_revision: String,
    coupon_trace_layer: String,
    coupon_trace_width_mm: f64,
    max_trace_width_delta_mm: f64,
    coupon_trace_gap_mm: Option<f64>,
    max_trace_gap_delta_mm: Option<f64>,
}

#[derive(Debug)]
struct TraceCorrelationMetrics {
    layer_mismatch: bool,
    observed_route_layers: Vec<String>,
    max_width_delta_mm: f64,
    width_segment_net: String,
    width_segment_index: usize,
    measured_width_mm: f64,
    max_gap_delta_mm: Option<f64>,
    measured_gap_mm: Option<f64>,
    gap_first_segment_index: Option<usize>,
    gap_second_segment_index: Option<usize>,
}

fn trace_correlation_policy(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    coupon: &ControlledImpedanceCoupon,
) -> Option<TraceCorrelationPolicy> {
    let process_lot = required_coupon_string(scenario, findings, coupon, "process_lot")?;
    let panel_id = required_coupon_string(scenario, findings, coupon, "panel_id")?;
    let stackup_revision = required_coupon_string(scenario, findings, coupon, "stackup_revision")?;
    let coupon_trace_layer =
        required_coupon_string(scenario, findings, coupon, "coupon_trace_layer")?;
    let coupon_trace_width_mm =
        required_coupon_positive(scenario, findings, coupon, "coupon_trace_width_mm")?;
    let max_trace_width_delta_mm =
        required_coupon_nonnegative(scenario, findings, coupon, "max_trace_width_delta_mm")?;
    let (coupon_trace_gap_mm, max_trace_gap_delta_mm) = match coupon.coupon_type {
        ControlledImpedanceCouponType::SingleEnded => {
            if coupon.coupon_trace_gap_mm.is_some() || coupon.max_trace_gap_delta_mm.is_some() {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID single-ended coupon {} must not declare coupon_trace_gap_mm or max_trace_gap_delta_mm.",
                        coupon.name
                    ),
                );
                return None;
            }
            (None, None)
        }
        ControlledImpedanceCouponType::Differential => (
            Some(required_coupon_positive(
                scenario,
                findings,
                coupon,
                "coupon_trace_gap_mm",
            )?),
            Some(required_coupon_nonnegative(
                scenario,
                findings,
                coupon,
                "max_trace_gap_delta_mm",
            )?),
        ),
    };
    Some(TraceCorrelationPolicy {
        process_lot,
        panel_id,
        stackup_revision,
        coupon_trace_layer,
        coupon_trace_width_mm,
        max_trace_width_delta_mm,
        coupon_trace_gap_mm,
        max_trace_gap_delta_mm,
    })
}

fn required_coupon_string(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    coupon: &ControlledImpedanceCoupon,
    field: &str,
) -> Option<String> {
    let value = match field {
        "process_lot" => coupon.process_lot.as_deref(),
        "panel_id" => coupon.panel_id.as_deref(),
        "stackup_revision" => coupon.stackup_revision.as_deref(),
        "coupon_trace_layer" => coupon.coupon_trace_layer.as_deref(),
        _ => None,
    }
    .map(str::trim)
    .filter(|value| !value.is_empty())
    .map(str::to_string);
    if value.is_none() {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID coupon {} requires reviewed {field}.",
                coupon.name
            ),
        );
    }
    value
}

fn required_coupon_positive(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    coupon: &ControlledImpedanceCoupon,
    field: &str,
) -> Option<f64> {
    let value = match field {
        "coupon_trace_width_mm" => coupon.coupon_trace_width_mm,
        "coupon_trace_gap_mm" => coupon.coupon_trace_gap_mm,
        _ => None,
    };
    required_coupon_number(scenario, findings, coupon, field, value, true)
}

fn required_coupon_nonnegative(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    coupon: &ControlledImpedanceCoupon,
    field: &str,
) -> Option<f64> {
    let value = match field {
        "max_trace_width_delta_mm" => coupon.max_trace_width_delta_mm,
        "max_trace_gap_delta_mm" => coupon.max_trace_gap_delta_mm,
        _ => None,
    };
    required_coupon_number(scenario, findings, coupon, field, value, false)
}

fn required_coupon_number(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    coupon: &ControlledImpedanceCoupon,
    field: &str,
    value: Option<f64>,
    positive: bool,
) -> Option<f64> {
    let Some(value) = value.filter(|value| value.is_finite()) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID coupon {} requires finite reviewed {field}.",
                coupon.name
            ),
        );
        return None;
    };
    if positive && value <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID coupon {} {field} must be positive.",
                coupon.name
            ),
        );
        return None;
    }
    if !positive && value < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID coupon {} {field} must be non-negative.",
                coupon.name
            ),
        );
        return None;
    }
    Some(value)
}

fn trace_correlation_metrics(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    coupon: &ControlledImpedanceCoupon,
    policy: &TraceCorrelationPolicy,
) -> Option<TraceCorrelationMetrics> {
    match coupon.coupon_type {
        ControlledImpedanceCouponType::SingleEnded => {
            let net = coupon.net.as_deref()?;
            let route = route_for_trace_correlation(bound, scenario, findings, net)?;
            trace_correlation_single_metrics(net, route, policy)
        }
        ControlledImpedanceCouponType::Differential => {
            let first_net = coupon.first_net.as_deref()?;
            let second_net = coupon.second_net.as_deref()?;
            let first_route = route_for_trace_correlation(bound, scenario, findings, first_net)?;
            let second_route = route_for_trace_correlation(bound, scenario, findings, second_net)?;
            trace_correlation_pair_metrics(
                scenario,
                findings,
                coupon,
                first_net,
                first_route,
                second_net,
                second_route,
                policy,
            )
        }
    }
}

fn route_for_trace_correlation<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    net: &str,
) -> Option<&'a NetRoute> {
    let Some(route) = bound.project.board.layout.routes.get(net) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID net {net} has no board.layout.routes entry."
            ),
        );
        return None;
    };
    if route.segments.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID net {net} route must include at least one segment."
            ),
        );
        return None;
    }
    for segment in &route.segments {
        if !usable_route_segment(segment) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID net {net} route segments must have finite endpoints, positive width, non-empty layer, and non-zero length."
                ),
            );
            return None;
        }
    }
    Some(route)
}

fn trace_correlation_single_metrics(
    net: &str,
    route: &NetRoute,
    policy: &TraceCorrelationPolicy,
) -> Option<TraceCorrelationMetrics> {
    let width = worst_trace_width_delta(route, net, policy.coupon_trace_width_mm)?;
    let observed_route_layers = route_layers(route);
    Some(TraceCorrelationMetrics {
        layer_mismatch: !observed_route_layers
            .iter()
            .all(|layer| layer == &policy.coupon_trace_layer),
        observed_route_layers,
        max_width_delta_mm: width.delta_mm,
        width_segment_net: width.net,
        width_segment_index: width.segment_index,
        measured_width_mm: width.measured_width_mm,
        max_gap_delta_mm: None,
        measured_gap_mm: None,
        gap_first_segment_index: None,
        gap_second_segment_index: None,
    })
}

#[allow(clippy::too_many_arguments)]
fn trace_correlation_pair_metrics(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    coupon: &ControlledImpedanceCoupon,
    first_net: &str,
    first_route: &NetRoute,
    second_net: &str,
    second_route: &NetRoute,
    policy: &TraceCorrelationPolicy,
) -> Option<TraceCorrelationMetrics> {
    let first_width = worst_trace_width_delta(first_route, first_net, policy.coupon_trace_width_mm);
    let second_width =
        worst_trace_width_delta(second_route, second_net, policy.coupon_trace_width_mm);
    let width = [first_width, second_width]
        .into_iter()
        .flatten()
        .max_by(|left, right| left.delta_mm.total_cmp(&right.delta_mm))?;
    let expected_gap = policy.coupon_trace_gap_mm?;
    let Some(gap) = worst_trace_gap_delta(first_route, second_route, expected_gap) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID coupon {} differential route has no parallel same-layer gap evidence.",
                coupon.name
            ),
        );
        return None;
    };
    let mut observed_route_layers = route_layers(first_route);
    observed_route_layers.extend(route_layers(second_route));
    observed_route_layers.sort();
    observed_route_layers.dedup();
    Some(TraceCorrelationMetrics {
        layer_mismatch: !observed_route_layers
            .iter()
            .all(|layer| layer == &policy.coupon_trace_layer),
        observed_route_layers,
        max_width_delta_mm: width.delta_mm,
        width_segment_net: width.net,
        width_segment_index: width.segment_index,
        measured_width_mm: width.measured_width_mm,
        max_gap_delta_mm: Some(gap.delta_mm),
        measured_gap_mm: Some(gap.measured_gap_mm),
        gap_first_segment_index: Some(gap.first_segment_index),
        gap_second_segment_index: Some(gap.second_segment_index),
    })
}

#[derive(Debug)]
struct TraceWidthDelta {
    net: String,
    segment_index: usize,
    measured_width_mm: f64,
    delta_mm: f64,
}

#[derive(Debug)]
struct TraceGapDelta {
    first_segment_index: usize,
    second_segment_index: usize,
    measured_gap_mm: f64,
    delta_mm: f64,
}

fn worst_trace_width_delta(
    route: &NetRoute,
    net: &str,
    expected_width_mm: f64,
) -> Option<TraceWidthDelta> {
    route
        .segments
        .iter()
        .enumerate()
        .map(|(segment_index, segment)| TraceWidthDelta {
            net: net.to_string(),
            segment_index,
            measured_width_mm: segment.width_mm,
            delta_mm: (segment.width_mm - expected_width_mm).abs(),
        })
        .max_by(|left, right| left.delta_mm.total_cmp(&right.delta_mm))
}

fn worst_trace_gap_delta(
    first_route: &NetRoute,
    second_route: &NetRoute,
    expected_gap_mm: f64,
) -> Option<TraceGapDelta> {
    let mut worst = None;
    for (first_segment_index, first_segment) in first_route.segments.iter().enumerate() {
        for (second_segment_index, second_segment) in second_route.segments.iter().enumerate() {
            if first_segment.layer != second_segment.layer {
                continue;
            }
            let Some(measured_gap_mm) = parallel_overlap_gap_mm(first_segment, second_segment)
            else {
                continue;
            };
            let delta_mm = (measured_gap_mm - expected_gap_mm).abs();
            let gap = TraceGapDelta {
                first_segment_index,
                second_segment_index,
                measured_gap_mm,
                delta_mm,
            };
            if worst
                .as_ref()
                .is_none_or(|current: &TraceGapDelta| delta_mm > current.delta_mm)
            {
                worst = Some(gap);
            }
        }
    }
    worst
}

fn route_layers(route: &NetRoute) -> Vec<String> {
    let mut layers = route
        .segments
        .iter()
        .map(|segment| segment.layer.clone())
        .collect::<Vec<_>>();
    layers.sort();
    layers.dedup();
    layers
}

fn usable_route_segment(segment: &RouteSegment) -> bool {
    segment.start.x_mm.is_finite()
        && segment.start.y_mm.is_finite()
        && segment.end.x_mm.is_finite()
        && segment.end.y_mm.is_finite()
        && segment.width_mm.is_finite()
        && segment.width_mm > 0.0
        && !segment.layer.trim().is_empty()
        && segment_length_mm(segment) > f64::EPSILON
}

fn segment_length_mm(segment: &RouteSegment) -> f64 {
    (segment.end.x_mm - segment.start.x_mm).hypot(segment.end.y_mm - segment.start.y_mm)
}

fn parallel_overlap_gap_mm(first: &RouteSegment, second: &RouteSegment) -> Option<f64> {
    let first_dx = first.end.x_mm - first.start.x_mm;
    let first_dy = first.end.y_mm - first.start.y_mm;
    let second_dx = second.end.x_mm - second.start.x_mm;
    let second_dy = second.end.y_mm - second.start.y_mm;
    let first_len = first_dx.hypot(first_dy);
    let second_len = second_dx.hypot(second_dy);
    if first_len <= f64::EPSILON || second_len <= f64::EPSILON {
        return None;
    }
    let first_unit_x = first_dx / first_len;
    let first_unit_y = first_dy / first_len;
    let second_unit_x = second_dx / second_len;
    let second_unit_y = second_dy / second_len;
    let cross = (first_unit_x * second_unit_y - first_unit_y * second_unit_x).abs();
    if cross > 1.0e-6 {
        return None;
    }
    let projection_a = (second.start.x_mm - first.start.x_mm) * first_unit_x
        + (second.start.y_mm - first.start.y_mm) * first_unit_y;
    let projection_b = (second.end.x_mm - first.start.x_mm) * first_unit_x
        + (second.end.y_mm - first.start.y_mm) * first_unit_y;
    let overlap_start = projection_a.min(projection_b).max(0.0);
    let overlap_end = projection_a.max(projection_b).min(first_len);
    if overlap_end - overlap_start <= f64::EPSILON {
        return None;
    }
    let centerline_distance_mm = ((second.start.x_mm - first.start.x_mm) * first_unit_y
        - (second.start.y_mm - first.start.y_mm) * first_unit_x)
        .abs();
    Some(centerline_distance_mm - (first.width_mm + second.width_mm) / 2.0)
}

fn trace_correlation_finding(
    scenario: &Scenario,
    coupon: &ControlledImpedanceCoupon,
    policy: &TraceCorrelationPolicy,
    metrics: &TraceCorrelationMetrics,
) -> Finding {
    let mut finding = Finding::critical(
        CONTROLLED_IMPEDANCE_COUPON_TRACE_CORRELATION_VALID,
        &scenario.name,
        format!(
            "Controlled-impedance coupon {} trace/process metadata does not correlate with imported board route evidence.",
            coupon.name
        ),
    );
    finding
        .measured
        .insert("coupon_name".to_string(), json!(coupon.name));
    finding
        .measured
        .insert("coupon_type".to_string(), json!(coupon_type_label(coupon)));
    finding
        .measured
        .insert("source".to_string(), json!(coupon.source));
    if let Some(net) = &coupon.net {
        finding.measured.insert("net".to_string(), json!(net));
    }
    if let Some(first_net) = &coupon.first_net {
        finding
            .measured
            .insert("first_net".to_string(), json!(first_net));
    }
    if let Some(second_net) = &coupon.second_net {
        finding
            .measured
            .insert("second_net".to_string(), json!(second_net));
    }
    finding
        .measured
        .insert("process_lot".to_string(), json!(policy.process_lot));
    finding
        .measured
        .insert("panel_id".to_string(), json!(policy.panel_id));
    finding.measured.insert(
        "stackup_revision".to_string(),
        json!(policy.stackup_revision),
    );
    finding.measured.insert(
        "coupon_trace_layer".to_string(),
        json!(policy.coupon_trace_layer),
    );
    finding.measured.insert(
        "observed_route_layers".to_string(),
        json!(metrics.observed_route_layers),
    );
    finding
        .measured
        .insert("layer_mismatch".to_string(), json!(metrics.layer_mismatch));
    finding.measured.insert(
        "width_segment_net".to_string(),
        json!(metrics.width_segment_net),
    );
    finding.measured.insert(
        "width_segment_index".to_string(),
        json!(metrics.width_segment_index),
    );
    finding.measured.insert(
        "measured_width_mm".to_string(),
        json!(metrics.measured_width_mm),
    );
    finding.measured.insert(
        "max_width_delta_mm".to_string(),
        json!(metrics.max_width_delta_mm),
    );
    if let Some(value) = metrics.max_gap_delta_mm {
        finding
            .measured
            .insert("max_gap_delta_mm".to_string(), json!(value));
    }
    if let Some(value) = metrics.measured_gap_mm {
        finding
            .measured
            .insert("measured_gap_mm".to_string(), json!(value));
    }
    if let Some(value) = metrics.gap_first_segment_index {
        finding
            .measured
            .insert("gap_first_segment_index".to_string(), json!(value));
    }
    if let Some(value) = metrics.gap_second_segment_index {
        finding
            .measured
            .insert("gap_second_segment_index".to_string(), json!(value));
    }
    finding.limit.insert(
        "coupon_trace_width_mm".to_string(),
        json!(policy.coupon_trace_width_mm),
    );
    finding.limit.insert(
        "max_trace_width_delta_mm".to_string(),
        json!(policy.max_trace_width_delta_mm),
    );
    if let Some(value) = policy.coupon_trace_gap_mm {
        finding
            .limit
            .insert("coupon_trace_gap_mm".to_string(), json!(value));
    }
    if let Some(value) = policy.max_trace_gap_delta_mm {
        finding
            .limit
            .insert("max_trace_gap_delta_mm".to_string(), json!(value));
    }
    finding.suggested_fixes = vec![
        "Review whether the coupon report belongs to the same fabrication lot, panel, stackup revision, and routed trace geometry as the board target.".to_string(),
        "Update coupon trace-layer/width/gap metadata only from the reviewed fabricator coupon or process report.".to_string(),
        "Do not treat this check as a field solver; it verifies explicit coupon-to-route correlation evidence only.".to_string(),
    ];
    finding
}

fn coupon_type_label(coupon: &ControlledImpedanceCoupon) -> &'static str {
    match coupon.coupon_type {
        ControlledImpedanceCouponType::SingleEnded => "single_ended",
        ControlledImpedanceCouponType::Differential => "differential",
    }
}
