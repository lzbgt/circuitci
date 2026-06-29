use crate::board_ir::{ControlledImpedanceCoupon, ControlledImpedanceCouponType, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;

use super::super::CONTROLLED_IMPEDANCE_COUPON_VALID;
use super::super::common::validation_input_missing;

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

fn coupon_names(scenario: &Scenario, findings: &mut Vec<Finding>) -> Option<Vec<String>> {
    let Some(value) = scenario.parameters.get("coupons") else {
        validation_input_missing(
            findings,
            scenario,
            "CONTROLLED_IMPEDANCE_COUPON_VALID requires parameters.coupons.",
        );
        return None;
    };
    let Some(items) = value.as_sequence() else {
        validation_input_missing(
            findings,
            scenario,
            "CONTROLLED_IMPEDANCE_COUPON_VALID parameters.coupons must be a list.",
        );
        return None;
    };
    if items.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "CONTROLLED_IMPEDANCE_COUPON_VALID parameters.coupons must not be empty.",
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
                format!(
                    "CONTROLLED_IMPEDANCE_COUPON_VALID parameters.coupons[{index}] must be an object."
                ),
            );
            return None;
        };
        let Some(value) = mapping.get("name") else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_COUPON_VALID parameters.coupons[{index}].name is required."
                ),
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
                format!(
                    "CONTROLLED_IMPEDANCE_COUPON_VALID parameters.coupons[{index}].name must be a non-empty string."
                ),
            );
            return None;
        };
        if !seen.insert(name.to_string()) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_COUPON_VALID parameters.coupons repeats coupon name {name}."
                ),
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
    let mut matches = coupons.iter().filter(|coupon| coupon.name == name);
    let Some(coupon) = matches.next() else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_COUPON_VALID coupon {name} is absent from board.manufacturing.controlled_impedance.coupons."
            ),
        );
        return None;
    };
    if matches.next().is_some() {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_COUPON_VALID coupon name {name} is duplicated in board.manufacturing.controlled_impedance.coupons."
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
        "Do not treat this check as a field solver; it verifies explicit coupon measurement evidence only.".to_string(),
    ];
    finding
}

fn coupon_type_label(coupon: &ControlledImpedanceCoupon) -> &'static str {
    match coupon.coupon_type {
        ControlledImpedanceCouponType::SingleEnded => "single_ended",
        ControlledImpedanceCouponType::Differential => "differential",
    }
}
