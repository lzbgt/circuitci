use super::manufacturing_suggestion;
use crate::board_ir::{
    LayoutPoint, NetRoute, RfAntennaFeedPathRule, RfAntennaKeepoutRule, RfAntennaMatchingElement,
    RfAntennaMatchingNetworkRule, RfAntennaMeasurement, RfAntennaMeasurementCondition,
    RfAntennaPerformanceLimit, RouteSegment,
};
use crate::library::BoundBoard;
use crate::scenario_suggestions::{ScenarioSuggestion, sanitized_name};
use serde_json::json;
use std::collections::BTreeMap;

const RF_ANTENNA_KEEPOUT_VALID: &str = "RF_ANTENNA_KEEPOUT_VALID";
const RF_ANTENNA_FEED_PATH_VALID: &str = "RF_ANTENNA_FEED_PATH_VALID";
const RF_ANTENNA_MATCHING_TOPOLOGY_VALID: &str = "RF_ANTENNA_MATCHING_TOPOLOGY_VALID";
const RF_ANTENNA_MEASURED_PERFORMANCE_VALID: &str = "RF_ANTENNA_MEASURED_PERFORMANCE_VALID";

pub(super) fn rf_antenna_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    suggestions.extend(rf_antenna_keepout_suggestions(bound, project_name));
    suggestions.extend(rf_antenna_feed_path_suggestions(bound, project_name));
    suggestions.extend(rf_antenna_matching_topology_suggestions(
        bound,
        project_name,
    ));
    suggestions.extend(rf_antenna_measured_performance_suggestions(
        bound,
        project_name,
    ));
    suggestions
}

fn rf_antenna_keepout_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for keepout in &bound.project.board.layout.constraints.rf_antenna.keepouts {
        if !rf_antenna_keepout_has_evidence(bound, keepout)
            || rf_antenna_keepout_check_declared(bound, &keepout.name)
        {
            continue;
        }
        suggestions.push(manufacturing_suggestion(
            &format!("rf_antenna_keepout_{}", sanitized_name(&keepout.name)),
            true,
            &format!(
                "RF antenna keepout {} has reviewed polygon/source metadata and imported same-layer copper evidence.",
                keepout.name
            ),
            &format!(
                "{}_{}_rf_antenna_keepout",
                project_name,
                sanitized_name(&keepout.name)
            ),
            RF_ANTENNA_KEEPOUT_VALID,
            Some(BTreeMap::from([(
                "keepouts".to_string(),
                json!([{ "name": keepout.name }]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

fn rf_antenna_keepout_has_evidence(bound: &BoundBoard<'_>, keepout: &RfAntennaKeepoutRule) -> bool {
    let metadata_valid = !keepout.name.trim().is_empty()
        && !keepout.layer.trim().is_empty()
        && !keepout.source.trim().is_empty()
        && keepout.min_copper_clearance_mm.is_finite()
        && keepout.min_copper_clearance_mm >= 0.0
        && keepout.polygon.len() >= 3
        && keepout
            .polygon
            .iter()
            .all(|point| point.x_mm.is_finite() && point.y_mm.is_finite())
        && keepout_polygon_area_mm2(&keepout.polygon) > f64::EPSILON
        && keepout
            .antenna_net
            .as_deref()
            .is_none_or(|net| bound.project.board.nets.contains_key(net));
    metadata_valid
        && (bound
            .project
            .board
            .layout
            .copper
            .features
            .iter()
            .any(|feature| {
                feature.layer == keepout.layer && !same_antenna_net(keepout, feature.net.as_deref())
            })
            || bound
                .project
                .board
                .layout
                .copper
                .segments
                .iter()
                .any(|segment| {
                    segment.layer == keepout.layer
                        && !same_antenna_net(keepout, segment.net.as_deref())
                })
            || bound
                .project
                .board
                .layout
                .copper
                .regions
                .iter()
                .any(|region| {
                    region.layer == keepout.layer
                        && !same_antenna_net(keepout, region.net.as_deref())
                }))
}

fn keepout_polygon_area_mm2(points: &[LayoutPoint]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(left, right)| left.x_mm * right.y_mm - right.x_mm * left.y_mm)
        .sum::<f64>()
        .abs()
        / 2.0
}

fn same_antenna_net(keepout: &RfAntennaKeepoutRule, net: Option<&str>) -> bool {
    matches!((keepout.antenna_net.as_deref(), net), (Some(antenna), Some(candidate)) if antenna == candidate)
}

fn rf_antenna_keepout_check_declared(bound: &BoundBoard<'_>, keepout_name: &str) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario
                .checks
                .iter()
                .any(|declared| declared == RF_ANTENNA_KEEPOUT_VALID)
            && scenario
                .parameters
                .get("keepouts")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .is_some_and(|keepouts| {
                    keepouts.iter().any(|item| {
                        item.as_mapping().and_then(|mapping| {
                            mapping
                                .get(serde_yaml_ng::Value::String("name".to_string()))
                                .and_then(serde_yaml_ng::Value::as_str)
                        }) == Some(keepout_name)
                    })
                })
    })
}

fn rf_antenna_feed_path_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for feed_path in &bound.project.board.layout.constraints.rf_antenna.feed_paths {
        if !rf_antenna_feed_path_has_evidence(bound, feed_path)
            || rf_antenna_feed_path_check_declared(bound, &feed_path.name)
        {
            continue;
        }
        suggestions.push(manufacturing_suggestion(
            &format!("rf_antenna_feed_path_{}", sanitized_name(&feed_path.name)),
            true,
            &format!(
                "RF antenna feed path {} has reviewed source metadata plus imported route, pad, placement, and matching-component evidence.",
                feed_path.name
            ),
            &format!(
                "{}_{}_rf_antenna_feed_path",
                project_name,
                sanitized_name(&feed_path.name)
            ),
            RF_ANTENNA_FEED_PATH_VALID,
            Some(BTreeMap::from([(
                "feed_paths".to_string(),
                json!([{ "name": feed_path.name }]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

fn rf_antenna_feed_path_has_evidence(
    bound: &BoundBoard<'_>,
    feed_path: &RfAntennaFeedPathRule,
) -> bool {
    !feed_path.name.trim().is_empty()
        && !feed_path.source.trim().is_empty()
        && bound
            .project
            .board
            .nets
            .contains_key(&feed_path.antenna_net)
        && feed_path.max_feed_route_length_mm.is_finite()
        && feed_path.max_feed_route_length_mm >= 0.0
        && feed_path.max_matching_component_distance_mm.is_finite()
        && feed_path.max_matching_component_distance_mm >= 0.0
        && !feed_path.matching_components.is_empty()
        && bound
            .project
            .board
            .components
            .get(&feed_path.feed_component)
            .is_some_and(|component| {
                component.pins.get(&feed_path.feed_pin) == Some(&feed_path.antenna_net)
            })
        && bound
            .project
            .board
            .layout
            .pads
            .get(&feed_path.feed_component)
            .and_then(|pads| pads.get(&feed_path.feed_pin))
            .is_some_and(|pad| {
                pad.net == feed_path.antenna_net
                    && pad.at.x_mm.is_finite()
                    && pad.at.y_mm.is_finite()
            })
        && bound
            .project
            .board
            .layout
            .routes
            .get(&feed_path.antenna_net)
            .is_some_and(route_has_finite_segments)
        && feed_path.matching_components.iter().all(|component| {
            bound.project.board.components.contains_key(component)
                && component_has_antenna_pin(bound, component, &feed_path.antenna_net)
                && bound
                    .project
                    .board
                    .layout
                    .placements
                    .get(component)
                    .is_some_and(|placement| {
                        placement.x_mm.is_finite() && placement.y_mm.is_finite()
                    })
                && component_has_antenna_layout_pad(bound, component, &feed_path.antenna_net)
        })
        && bound
            .project
            .board
            .layout
            .placements
            .get(&feed_path.feed_component)
            .is_some_and(|placement| placement.x_mm.is_finite() && placement.y_mm.is_finite())
}

fn route_has_finite_segments(route: &NetRoute) -> bool {
    !route.segments.is_empty() && route.segments.iter().all(route_segment_is_finite)
}

fn route_segment_is_finite(segment: &RouteSegment) -> bool {
    !segment.layer.trim().is_empty()
        && segment.width_mm.is_finite()
        && segment.width_mm > 0.0
        && segment.start.x_mm.is_finite()
        && segment.start.y_mm.is_finite()
        && segment.end.x_mm.is_finite()
        && segment.end.y_mm.is_finite()
        && (segment.end.x_mm - segment.start.x_mm).hypot(segment.end.y_mm - segment.start.y_mm)
            > f64::EPSILON
}

fn component_has_antenna_pin(bound: &BoundBoard<'_>, component: &str, antenna_net: &str) -> bool {
    bound
        .project
        .board
        .components
        .get(component)
        .is_some_and(|spec| spec.pins.values().any(|net| net == antenna_net))
}

fn component_has_antenna_layout_pad(
    bound: &BoundBoard<'_>,
    component: &str,
    antenna_net: &str,
) -> bool {
    bound
        .project
        .board
        .layout
        .pads
        .get(component)
        .is_some_and(|pads| {
            pads.values().any(|pad| {
                pad.net == antenna_net && pad.at.x_mm.is_finite() && pad.at.y_mm.is_finite()
            })
        })
}

fn rf_antenna_feed_path_check_declared(bound: &BoundBoard<'_>, feed_path_name: &str) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario
                .checks
                .iter()
                .any(|declared| declared == RF_ANTENNA_FEED_PATH_VALID)
            && scenario
                .parameters
                .get("feed_paths")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .is_some_and(|feed_paths| {
                    feed_paths.iter().any(|item| {
                        item.as_mapping().and_then(|mapping| {
                            mapping
                                .get(serde_yaml_ng::Value::String("name".to_string()))
                                .and_then(serde_yaml_ng::Value::as_str)
                        }) == Some(feed_path_name)
                    })
                })
    })
}

fn rf_antenna_matching_topology_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for network in &bound
        .project
        .board
        .layout
        .constraints
        .rf_antenna
        .matching_networks
    {
        if !rf_antenna_matching_network_has_evidence(bound, network)
            || rf_antenna_matching_network_check_declared(bound, &network.name)
        {
            continue;
        }
        suggestions.push(manufacturing_suggestion(
            &format!(
                "rf_antenna_matching_topology_{}",
                sanitized_name(&network.name)
            ),
            true,
            &format!(
                "RF antenna matching network {} has reviewed topology metadata plus imported component pin and layout pad evidence.",
                network.name
            ),
            &format!(
                "{}_{}_rf_antenna_matching_topology",
                project_name,
                sanitized_name(&network.name)
            ),
            RF_ANTENNA_MATCHING_TOPOLOGY_VALID,
            Some(BTreeMap::from([(
                "matching_networks".to_string(),
                json!([{ "name": network.name }]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

fn rf_antenna_matching_network_has_evidence(
    bound: &BoundBoard<'_>,
    network: &RfAntennaMatchingNetworkRule,
) -> bool {
    !network.name.trim().is_empty()
        && !network.source.trim().is_empty()
        && !network.elements.is_empty()
        && matches!(
            normalize_rf_token(&network.topology).as_str(),
            "series" | "l" | "pi" | "t" | "custom"
        )
        && bound.project.board.nets.contains_key(&network.antenna_net)
        && network
            .reference_net
            .as_deref()
            .is_none_or(|net| bound.project.board.nets.contains_key(net))
        && network
            .elements
            .iter()
            .any(|element| matching_element_touches_net(element, &network.antenna_net))
        && network
            .elements
            .iter()
            .enumerate()
            .all(|(index, element)| matching_element_has_evidence(bound, network, element, index))
}

fn matching_element_has_evidence(
    bound: &BoundBoard<'_>,
    network: &RfAntennaMatchingNetworkRule,
    element: &RfAntennaMatchingElement,
    _index: usize,
) -> bool {
    if element.component.trim().is_empty()
        || !bound
            .project
            .board
            .components
            .contains_key(&element.component)
    {
        return false;
    }
    match normalize_rf_token(&element.role).as_str() {
        "series" => {
            let Some(input_net) = matching_element_net(element.input_net.as_deref()) else {
                return false;
            };
            let Some(output_net) = matching_element_net(element.output_net.as_deref()) else {
                return false;
            };
            input_net != output_net
                && bound.project.board.nets.contains_key(input_net)
                && bound.project.board.nets.contains_key(output_net)
                && component_has_pin_on_net(bound, &element.component, input_net)
                && component_has_pin_on_net(bound, &element.component, output_net)
                && component_has_layout_pad_on_net(bound, &element.component, input_net)
                && component_has_layout_pad_on_net(bound, &element.component, output_net)
        }
        "shunt" => {
            let Some(signal_net) = matching_element_net(element.signal_net.as_deref()) else {
                return false;
            };
            let Some(reference_net) = matching_element_net(
                element
                    .reference_net
                    .as_deref()
                    .or(network.reference_net.as_deref()),
            ) else {
                return false;
            };
            signal_net != reference_net
                && bound.project.board.nets.contains_key(signal_net)
                && bound.project.board.nets.contains_key(reference_net)
                && component_has_pin_on_net(bound, &element.component, signal_net)
                && component_has_pin_on_net(bound, &element.component, reference_net)
                && component_has_layout_pad_on_net(bound, &element.component, signal_net)
                && component_has_layout_pad_on_net(bound, &element.component, reference_net)
        }
        _ => false,
    }
}

fn matching_element_net(net: Option<&str>) -> Option<&str> {
    net.map(str::trim).filter(|value| !value.is_empty())
}

fn matching_element_touches_net(element: &RfAntennaMatchingElement, net: &str) -> bool {
    [
        element.input_net.as_deref(),
        element.output_net.as_deref(),
        element.signal_net.as_deref(),
        element.reference_net.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|candidate| candidate == net)
}

fn normalize_rf_token(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn component_has_pin_on_net(bound: &BoundBoard<'_>, component: &str, net: &str) -> bool {
    bound
        .project
        .board
        .components
        .get(component)
        .is_some_and(|spec| spec.pins.values().any(|candidate| candidate == net))
}

fn component_has_layout_pad_on_net(bound: &BoundBoard<'_>, component: &str, net: &str) -> bool {
    bound
        .project
        .board
        .layout
        .pads
        .get(component)
        .is_some_and(|pads| {
            pads.values()
                .any(|pad| pad.net == net && pad.at.x_mm.is_finite() && pad.at.y_mm.is_finite())
        })
}

fn rf_antenna_matching_network_check_declared(
    bound: &BoundBoard<'_>,
    matching_network_name: &str,
) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario
                .checks
                .iter()
                .any(|declared| declared == RF_ANTENNA_MATCHING_TOPOLOGY_VALID)
            && scenario
                .parameters
                .get("matching_networks")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .is_some_and(|matching_networks| {
                    matching_networks.iter().any(|item| {
                        item.as_mapping().and_then(|mapping| {
                            mapping
                                .get(serde_yaml_ng::Value::String("name".to_string()))
                                .and_then(serde_yaml_ng::Value::as_str)
                        }) == Some(matching_network_name)
                    })
                })
    })
}

fn rf_antenna_measured_performance_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    suggestions.extend(rf_antenna_measured_performance_sweep_suggestions(
        bound,
        project_name,
    ));
    for measurement in &bound
        .project
        .board
        .layout
        .constraints
        .rf_antenna
        .measurements
    {
        if !rf_antenna_measurement_has_evidence(bound, measurement)
            || rf_antenna_measurement_check_declared(bound, &measurement.name)
        {
            continue;
        }
        let matching_limits = bound
            .project
            .board
            .layout
            .constraints
            .rf_antenna
            .performance_limits
            .iter()
            .filter(|limit| {
                !rf_antenna_performance_limit_has_sweep_policy(limit)
                    && rf_antenna_performance_limit_matches(bound, measurement, limit)
            })
            .collect::<Vec<_>>();
        if !matching_limits.is_empty() {
            for limit in matching_limits {
                let mut parameters = BTreeMap::from([
                    (
                        "rf_measurements".to_string(),
                        json!([{ "name": measurement.name }]),
                    ),
                    (
                        "min_return_loss_db".to_string(),
                        json!(limit.min_return_loss_db),
                    ),
                ]);
                if let Some(min_mhz) = limit.frequency_min_mhz {
                    parameters.insert("frequency_min_mhz".to_string(), json!(min_mhz));
                }
                if let Some(max_mhz) = limit.frequency_max_mhz {
                    parameters.insert("frequency_max_mhz".to_string(), json!(max_mhz));
                }
                if let Some(condition) = limit.required_measurement_condition.as_deref() {
                    parameters.insert("measurement_condition".to_string(), json!(condition));
                }
                suggestions.push(manufacturing_suggestion(
                    &format!(
                        "rf_antenna_measured_performance_{}_{}",
                        sanitized_name(&measurement.name),
                        sanitized_name(&limit.name)
                    ),
                    true,
                    &format!(
                        "RF antenna measurement {} has reviewed return-loss evidence matched to reviewed RF performance limit {}.",
                        measurement.name, limit.name
                    ),
                    &format!(
                        "{}_{}_{}_rf_antenna_measured_performance",
                        project_name,
                        sanitized_name(&measurement.name),
                        sanitized_name(&limit.name)
                    ),
                    RF_ANTENNA_MEASURED_PERFORMANCE_VALID,
                    Some(parameters),
                    Vec::new(),
                ));
            }
            continue;
        }
        let matched_by_sweep_limit = bound
            .project
            .board
            .layout
            .constraints
            .rf_antenna
            .performance_limits
            .iter()
            .any(|limit| {
                rf_antenna_performance_limit_has_sweep_policy(limit)
                    && rf_antenna_performance_limit_matches(bound, measurement, limit)
            });
        if matched_by_sweep_limit {
            continue;
        }
        suggestions.push(manufacturing_suggestion(
            &format!(
                "rf_antenna_measured_performance_{}",
                sanitized_name(&measurement.name)
            ),
            false,
            &format!(
                "RF antenna measurement {} has reviewed source, antenna-net, frequency, and return-loss evidence.",
                measurement.name
            ),
            &format!(
                "{}_{}_rf_antenna_measured_performance",
                project_name,
                sanitized_name(&measurement.name)
            ),
            RF_ANTENNA_MEASURED_PERFORMANCE_VALID,
            Some(BTreeMap::from([(
                "rf_measurements".to_string(),
                json!([{ "name": measurement.name }]),
            )])),
            vec![
                "Review and set parameters.min_return_loss_db from the antenna module datasheet, RF design review, or product requirement.".to_string(),
                "Optionally set parameters.frequency_min_mhz and parameters.frequency_max_mhz for the reviewed operating band.".to_string(),
            ],
        ));
    }
    suggestions
}

fn rf_antenna_measured_performance_sweep_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let measurements = &bound
        .project
        .board
        .layout
        .constraints
        .rf_antenna
        .measurements;
    bound
        .project
        .board
        .layout
        .constraints
        .rf_antenna
        .performance_limits
        .iter()
        .filter(|limit| rf_antenna_performance_limit_has_sweep_policy(limit))
        .filter(|limit| rf_antenna_performance_limit_has_evidence(bound, limit))
        .filter_map(|limit| {
            let mut matching_measurements = measurements
                .iter()
                .filter(|measurement| {
                    rf_antenna_measurement_has_evidence(bound, measurement)
                        && rf_antenna_performance_limit_matches(bound, measurement, limit)
                })
                .collect::<Vec<_>>();
            matching_measurements
                .sort_by(|left, right| left.frequency_mhz.total_cmp(&right.frequency_mhz));
            if matching_measurements.is_empty()
                || matching_measurements
                    .iter()
                    .any(|measurement| rf_antenna_measurement_check_declared(bound, &measurement.name))
            {
                return None;
            }
            let mut parameters = BTreeMap::from([
                (
                    "rf_measurements".to_string(),
                    json!(
                        matching_measurements
                            .iter()
                            .map(|measurement| json!({ "name": measurement.name }))
                            .collect::<Vec<_>>()
                    ),
                ),
                (
                    "min_return_loss_db".to_string(),
                    json!(limit.min_return_loss_db),
                ),
            ]);
            if let Some(min_mhz) = limit.frequency_min_mhz {
                parameters.insert("frequency_min_mhz".to_string(), json!(min_mhz));
            }
            if let Some(max_mhz) = limit.frequency_max_mhz {
                parameters.insert("frequency_max_mhz".to_string(), json!(max_mhz));
            }
            if let Some(min_count) = limit.min_measurement_count {
                parameters.insert("min_measurement_count".to_string(), json!(min_count));
            }
            if let Some(max_step_mhz) = limit.max_frequency_step_mhz {
                parameters.insert("max_frequency_step_mhz".to_string(), json!(max_step_mhz));
            }
            if let Some(condition) = limit.required_measurement_condition.as_deref() {
                parameters.insert("measurement_condition".to_string(), json!(condition));
            }
            Some(manufacturing_suggestion(
                &format!(
                    "rf_antenna_measured_performance_sweep_{}",
                    sanitized_name(&limit.name)
                ),
                true,
                &format!(
                    "RF antenna measurements for {} have reviewed return-loss sweep evidence matched to reviewed RF performance limit {}.",
                    limit.antenna_net, limit.name
                ),
                &format!(
                    "{}_{}_rf_antenna_measured_performance_sweep",
                    project_name,
                    sanitized_name(&limit.name)
                ),
                RF_ANTENNA_MEASURED_PERFORMANCE_VALID,
                Some(parameters),
                Vec::new(),
            ))
        })
        .collect()
}

fn rf_antenna_measurement_has_evidence(
    bound: &BoundBoard<'_>,
    measurement: &RfAntennaMeasurement,
) -> bool {
    !measurement.name.trim().is_empty()
        && !measurement.source.trim().is_empty()
        && bound
            .project
            .board
            .nets
            .contains_key(&measurement.antenna_net)
        && measurement.frequency_mhz.is_finite()
        && measurement.frequency_mhz > 0.0
        && measurement.return_loss_db.is_finite()
        && measurement.return_loss_db > 0.0
        && rf_antenna_measurement_condition_has_evidence(bound, measurement)
}

fn rf_antenna_measurement_condition_has_evidence(
    bound: &BoundBoard<'_>,
    measurement: &RfAntennaMeasurement,
) -> bool {
    let Some(condition_name) = measurement.measurement_condition.as_deref() else {
        return true;
    };
    !condition_name.trim().is_empty()
        && bound
            .project
            .board
            .layout
            .constraints
            .rf_antenna
            .measurement_conditions
            .iter()
            .any(|condition| {
                condition.name == condition_name
                    && !condition.source.trim().is_empty()
                    && rf_antenna_measurement_condition_has_reviewed_detail(condition)
            })
}

fn rf_antenna_performance_limit_has_sweep_policy(limit: &RfAntennaPerformanceLimit) -> bool {
    limit.min_measurement_count.is_some() || limit.max_frequency_step_mhz.is_some()
}

fn rf_antenna_performance_limit_has_evidence(
    bound: &BoundBoard<'_>,
    limit: &RfAntennaPerformanceLimit,
) -> bool {
    !limit.name.trim().is_empty()
        && !limit.source.trim().is_empty()
        && bound.project.board.nets.contains_key(&limit.antenna_net)
        && limit.min_return_loss_db.is_finite()
        && limit.min_return_loss_db > 0.0
        && optional_positive_frequency(limit.frequency_min_mhz)
        && optional_positive_frequency(limit.frequency_max_mhz)
        && frequency_band_order_valid(limit.frequency_min_mhz, limit.frequency_max_mhz)
        && limit.min_measurement_count.is_none_or(|count| count > 0)
        && optional_positive_frequency(limit.max_frequency_step_mhz)
        && limit
            .required_measurement_condition
            .as_deref()
            .is_none_or(|condition_name| {
                !condition_name.trim().is_empty()
                    && bound
                        .project
                        .board
                        .layout
                        .constraints
                        .rf_antenna
                        .measurement_conditions
                        .iter()
                        .any(|condition| {
                            condition.name == condition_name
                                && !condition.source.trim().is_empty()
                                && rf_antenna_measurement_condition_has_reviewed_detail(condition)
                        })
            })
}

fn rf_antenna_measurement_condition_has_reviewed_detail(
    condition: &RfAntennaMeasurementCondition,
) -> bool {
    condition
        .fixture
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        || condition
            .cable_setup
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        || condition
            .enclosure_profile
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

fn rf_antenna_performance_limit_matches(
    bound: &BoundBoard<'_>,
    measurement: &RfAntennaMeasurement,
    limit: &RfAntennaPerformanceLimit,
) -> bool {
    rf_antenna_performance_limit_has_evidence(bound, limit)
        && limit.antenna_net == measurement.antenna_net
        && limit
            .required_measurement_condition
            .as_deref()
            .is_none_or(|condition| measurement.measurement_condition.as_deref() == Some(condition))
        && limit
            .frequency_min_mhz
            .is_none_or(|min_mhz| measurement.frequency_mhz >= min_mhz - f64::EPSILON)
        && limit
            .frequency_max_mhz
            .is_none_or(|max_mhz| measurement.frequency_mhz <= max_mhz + f64::EPSILON)
}

fn optional_positive_frequency(value: Option<f64>) -> bool {
    value.is_none_or(|frequency_mhz| frequency_mhz.is_finite() && frequency_mhz > 0.0)
}

fn frequency_band_order_valid(min_mhz: Option<f64>, max_mhz: Option<f64>) -> bool {
    match (min_mhz, max_mhz) {
        (Some(min_mhz), Some(max_mhz)) => max_mhz + f64::EPSILON >= min_mhz,
        _ => true,
    }
}

fn rf_antenna_measurement_check_declared(bound: &BoundBoard<'_>, measurement_name: &str) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario
                .checks
                .iter()
                .any(|declared| declared == RF_ANTENNA_MEASURED_PERFORMANCE_VALID)
            && scenario
                .parameters
                .get("rf_measurements")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .is_some_and(|measurements| {
                    measurements.iter().any(|item| {
                        item.as_mapping().and_then(|mapping| {
                            mapping
                                .get(serde_yaml_ng::Value::String("name".to_string()))
                                .and_then(serde_yaml_ng::Value::as_str)
                        }) == Some(measurement_name)
                    })
                })
    })
}
