use crate::board_ir::{
    ComponentPlacement, LayoutCopperFeature, LayoutCopperRegion, LayoutCopperSegment, LayoutPad,
    LayoutPoint, NetRoute, RfAntennaFeedPathRule, RfAntennaKeepoutRule, RfAntennaMatchingElement,
    RfAntennaMatchingNetworkRule, RfAntennaMeasurement, RouteSegment, Scenario,
};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use super::super::common::validation_input_missing;
use super::super::{
    RF_ANTENNA_FEED_PATH_VALID, RF_ANTENNA_KEEPOUT_VALID, RF_ANTENNA_MATCHING_TOPOLOGY_VALID,
    RF_ANTENNA_MEASURED_PERFORMANCE_VALID,
};
use super::geometry::{
    copper_feature_to_polygon_clearance_mm, copper_segment_to_polygon_clearance_mm,
    point_inside_polygon, polygon_to_polygon_clearance_mm, validate_copper_feature_geometry,
    validate_copper_region_geometry, validate_copper_segment_geometry,
};
use std::collections::BTreeSet;

pub(in crate::validation) fn validate_rf_antenna_keepout(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(names) = keepout_names(scenario, findings) else {
        return;
    };
    for name in names {
        let Some(rule) = keepout_rule(bound, scenario, findings, &name) else {
            return;
        };
        validate_keepout_rule(bound, scenario, findings, rule);
    }
}

pub(in crate::validation) fn validate_rf_antenna_feed_path(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(names) =
        named_rule_parameters(scenario, findings, RF_ANTENNA_FEED_PATH_VALID, "feed_paths")
    else {
        return;
    };
    for name in names {
        let Some(rule) = feed_path_rule(bound, scenario, findings, &name) else {
            return;
        };
        validate_feed_path_rule(bound, scenario, findings, rule);
    }
}

pub(in crate::validation) fn validate_rf_antenna_matching_topology(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(names) = named_rule_parameters(
        scenario,
        findings,
        RF_ANTENNA_MATCHING_TOPOLOGY_VALID,
        "matching_networks",
    ) else {
        return;
    };
    for name in names {
        let Some(rule) = matching_network_rule(bound, scenario, findings, &name) else {
            return;
        };
        validate_matching_network_rule(bound, scenario, findings, rule);
    }
}

pub(in crate::validation) fn validate_rf_antenna_measured_performance(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(names) = named_rule_parameters(
        scenario,
        findings,
        RF_ANTENNA_MEASURED_PERFORMANCE_VALID,
        "rf_measurements",
    ) else {
        return;
    };
    let Some(min_return_loss_db) = required_positive_numeric_parameter(
        scenario,
        findings,
        RF_ANTENNA_MEASURED_PERFORMANCE_VALID,
        "min_return_loss_db",
    ) else {
        return;
    };
    let Some(frequency_band) = optional_frequency_band(scenario, findings) else {
        return;
    };
    let Some(min_measurement_count) = optional_positive_usize_parameter(
        scenario,
        findings,
        RF_ANTENNA_MEASURED_PERFORMANCE_VALID,
        "min_measurement_count",
    ) else {
        return;
    };
    let Some(max_frequency_step_mhz) = optional_positive_numeric_parameter(
        scenario,
        findings,
        RF_ANTENNA_MEASURED_PERFORMANCE_VALID,
        "max_frequency_step_mhz",
    ) else {
        return;
    };
    let mut measurements = Vec::new();
    for name in names {
        let Some(measurement) = rf_measurement(bound, scenario, findings, &name) else {
            return;
        };
        measurements.push(measurement);
    }
    if (min_measurement_count.is_some() || max_frequency_step_mhz.is_some())
        && let Err(message) = validate_rf_measurement_sweep_metadata(&measurements)
    {
        validation_input_missing(findings, scenario, message);
        return;
    }
    for measurement in &measurements {
        validate_rf_measurement(
            bound,
            scenario,
            findings,
            measurement,
            min_return_loss_db,
            frequency_band,
        );
    }
    validate_rf_measurement_sweep(
        scenario,
        findings,
        &measurements,
        frequency_band,
        min_measurement_count,
        max_frequency_step_mhz,
    );
}

fn keepout_names(scenario: &Scenario, findings: &mut Vec<Finding>) -> Option<Vec<String>> {
    named_rule_parameters(scenario, findings, RF_ANTENNA_KEEPOUT_VALID, "keepouts")
}

#[derive(Debug, Clone, Copy)]
struct FrequencyBand {
    min_mhz: Option<f64>,
    max_mhz: Option<f64>,
}

fn named_rule_parameters(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    check_id: &str,
    parameter_name: &str,
) -> Option<Vec<String>> {
    let Some(value) = scenario.parameters.get(parameter_name) else {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} requires parameters.{parameter_name}."),
        );
        return None;
    };
    let Some(items) = value.as_sequence() else {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} parameters.{parameter_name} must be a list."),
        );
        return None;
    };
    if items.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} parameters.{parameter_name} must not be empty."),
        );
        return None;
    }
    let mut names = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let Some(mapping) = item.as_mapping() else {
            validation_input_missing(
                findings,
                scenario,
                format!("{check_id} parameters.{parameter_name}[{index}] must be an object."),
            );
            return None;
        };
        let name = mapping
            .get(serde_yaml_ng::Value::String("name".to_string()))
            .and_then(serde_yaml_ng::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let Some(name) = name else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "{check_id} parameters.{parameter_name}[{index}].name must be a non-empty string."
                ),
            );
            return None;
        };
        names.push(name);
    }
    Some(names)
}

fn keepout_rule<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    name: &str,
) -> Option<&'a RfAntennaKeepoutRule> {
    let matches = bound
        .project
        .board
        .layout
        .constraints
        .rf_antenna
        .keepouts
        .iter()
        .filter(|rule| rule.name == name)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [rule] => Some(*rule),
        [] => {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "RF_ANTENNA_KEEPOUT_VALID keepout {name} is absent from board.layout.constraints.rf_antenna.keepouts."
                ),
            );
            None
        }
        _ => {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "RF_ANTENNA_KEEPOUT_VALID keepout {name} is ambiguous in board.layout.constraints.rf_antenna.keepouts."
                ),
            );
            None
        }
    }
}

fn feed_path_rule<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    name: &str,
) -> Option<&'a RfAntennaFeedPathRule> {
    let matches = bound
        .project
        .board
        .layout
        .constraints
        .rf_antenna
        .feed_paths
        .iter()
        .filter(|rule| rule.name == name)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [rule] => Some(*rule),
        [] => {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "RF_ANTENNA_FEED_PATH_VALID feed path {name} is absent from board.layout.constraints.rf_antenna.feed_paths."
                ),
            );
            None
        }
        _ => {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "RF_ANTENNA_FEED_PATH_VALID feed path {name} is ambiguous in board.layout.constraints.rf_antenna.feed_paths."
                ),
            );
            None
        }
    }
}

fn matching_network_rule<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    name: &str,
) -> Option<&'a RfAntennaMatchingNetworkRule> {
    let matches = bound
        .project
        .board
        .layout
        .constraints
        .rf_antenna
        .matching_networks
        .iter()
        .filter(|rule| rule.name == name)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [rule] => Some(*rule),
        [] => {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "RF_ANTENNA_MATCHING_TOPOLOGY_VALID matching network {name} is absent from board.layout.constraints.rf_antenna.matching_networks."
                ),
            );
            None
        }
        _ => {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "RF_ANTENNA_MATCHING_TOPOLOGY_VALID matching network {name} is ambiguous in board.layout.constraints.rf_antenna.matching_networks."
                ),
            );
            None
        }
    }
}

fn rf_measurement<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    name: &str,
) -> Option<&'a RfAntennaMeasurement> {
    let matches = bound
        .project
        .board
        .layout
        .constraints
        .rf_antenna
        .measurements
        .iter()
        .filter(|measurement| measurement.name == name)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [measurement] => Some(*measurement),
        [] => {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "RF_ANTENNA_MEASURED_PERFORMANCE_VALID measurement {name} is absent from board.layout.constraints.rf_antenna.measurements."
                ),
            );
            None
        }
        _ => {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "RF_ANTENNA_MEASURED_PERFORMANCE_VALID measurement {name} is ambiguous in board.layout.constraints.rf_antenna.measurements."
                ),
            );
            None
        }
    }
}

fn validate_keepout_rule(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    rule: &RfAntennaKeepoutRule,
) {
    if let Err(message) = validate_keepout_metadata(bound, rule) {
        validation_input_missing(findings, scenario, message);
        return;
    }

    let mut comparable_count = 0usize;
    for (index, feature) in bound
        .project
        .board
        .layout
        .copper
        .features
        .iter()
        .enumerate()
    {
        if !copper_feature_is_comparable(rule, feature) {
            continue;
        }
        comparable_count += 1;
        if let Err(message) = validate_copper_feature_geometry(feature, index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        let Some(clearance_mm) = copper_feature_to_polygon_clearance_mm(feature, &rule.polygon)
        else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "RF_ANTENNA_KEEPOUT_VALID keepout {} cannot compare copper feature {index}.",
                    rule.name
                ),
            );
            continue;
        };
        if clearance_mm + f64::EPSILON < rule.min_copper_clearance_mm {
            findings.push(feature_keepout_finding(
                scenario,
                rule,
                feature,
                index,
                clearance_mm,
            ));
        }
    }
    for (index, segment) in bound
        .project
        .board
        .layout
        .copper
        .segments
        .iter()
        .enumerate()
    {
        if !copper_segment_is_comparable(rule, segment) {
            continue;
        }
        comparable_count += 1;
        if let Err(message) = validate_copper_segment_geometry(segment, index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        let clearance_mm = copper_segment_to_polygon_clearance_mm(segment, &rule.polygon);
        if clearance_mm + f64::EPSILON < rule.min_copper_clearance_mm {
            findings.push(segment_keepout_finding(
                scenario,
                rule,
                segment,
                index,
                clearance_mm,
            ));
        }
    }
    for (index, region) in bound.project.board.layout.copper.regions.iter().enumerate() {
        if !copper_region_is_comparable(rule, region) {
            continue;
        }
        comparable_count += 1;
        if let Err(message) = validate_copper_region_geometry(region, index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        let clearance_mm = polygon_to_polygon_clearance_mm(&region.points, &rule.polygon);
        if clearance_mm + f64::EPSILON < rule.min_copper_clearance_mm {
            findings.push(region_keepout_finding(
                scenario,
                rule,
                region,
                index,
                clearance_mm,
            ));
        }
    }

    if comparable_count == 0 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "RF_ANTENNA_KEEPOUT_VALID keepout {} has no comparable board.layout.copper evidence on layer {}.",
                rule.name, rule.layer
            ),
        );
    }
}

fn validate_feed_path_rule(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    rule: &RfAntennaFeedPathRule,
) {
    if let Err(message) = validate_feed_path_metadata(bound, rule) {
        validation_input_missing(findings, scenario, message);
        return;
    }
    let Some(route) = bound.project.board.layout.routes.get(&rule.antenna_net) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "RF_ANTENNA_FEED_PATH_VALID feed path {} antenna_net {} has no board.layout.routes evidence.",
                rule.name, rule.antenna_net
            ),
        );
        return;
    };
    if let Err(message) = validate_route_geometry(&rule.antenna_net, route) {
        validation_input_missing(findings, scenario, message);
        return;
    }
    let Some(feed_pad) = feed_pad(bound, rule) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "RF_ANTENNA_FEED_PATH_VALID feed path {} requires board.layout.pads.{}.{} on antenna net {}.",
                rule.name, rule.feed_component, rule.feed_pin, rule.antenna_net
            ),
        );
        return;
    };
    if let Err(message) =
        validate_pad_geometry(&rule.name, &rule.feed_component, &rule.feed_pin, feed_pad)
    {
        validation_input_missing(findings, scenario, message);
        return;
    }

    let route_length_mm = route_length_mm(route);
    if route_length_mm > rule.max_feed_route_length_mm + f64::EPSILON {
        findings.push(feed_route_length_finding(
            scenario,
            rule,
            route_length_mm,
            route.segments.len(),
        ));
    }

    for component in &rule.matching_components {
        let Some(distance_mm) = matching_component_distance_mm(bound, rule, component) else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "RF_ANTENNA_FEED_PATH_VALID feed path {} matching component {component} lacks placement and antenna-net pad evidence.",
                    rule.name
                ),
            );
            continue;
        };
        if distance_mm > rule.max_matching_component_distance_mm + f64::EPSILON {
            findings.push(matching_component_distance_finding(
                scenario,
                rule,
                component,
                distance_mm,
            ));
        }
    }
}

fn validate_matching_network_rule(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    rule: &RfAntennaMatchingNetworkRule,
) {
    if let Err(message) = validate_matching_network_metadata(bound, rule) {
        validation_input_missing(findings, scenario, message);
        return;
    }
    for (index, element) in rule.elements.iter().enumerate() {
        if let Err(message) = validate_matching_element(bound, rule, element, index) {
            validation_input_missing(findings, scenario, message);
            return;
        }
    }
    let role_counts = matching_role_counts(&rule.elements);
    if !topology_counts_match(rule.topology.as_str(), role_counts) {
        findings.push(matching_topology_finding(scenario, rule, role_counts));
    }
}

fn validate_rf_measurement(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    measurement: &RfAntennaMeasurement,
    min_return_loss_db: f64,
    frequency_band: FrequencyBand,
) {
    if let Err(message) = validate_rf_measurement_metadata(bound, measurement) {
        validation_input_missing(findings, scenario, message);
        return;
    }
    if let Some(min_mhz) = frequency_band.min_mhz
        && measurement.frequency_mhz < min_mhz - f64::EPSILON
    {
        findings.push(rf_measurement_frequency_finding(
            scenario,
            measurement,
            frequency_band,
        ));
        return;
    }
    if let Some(max_mhz) = frequency_band.max_mhz
        && measurement.frequency_mhz > max_mhz + f64::EPSILON
    {
        findings.push(rf_measurement_frequency_finding(
            scenario,
            measurement,
            frequency_band,
        ));
        return;
    }
    if measurement.return_loss_db + f64::EPSILON < min_return_loss_db {
        findings.push(rf_measurement_return_loss_finding(
            scenario,
            measurement,
            min_return_loss_db,
            frequency_band,
        ));
    }
}

fn validate_rf_measurement_sweep_metadata(
    measurements: &[&RfAntennaMeasurement],
) -> Result<(), String> {
    let antenna_nets = measurements
        .iter()
        .map(|measurement| measurement.antenna_net.as_str())
        .collect::<BTreeSet<_>>();
    if antenna_nets.len() > 1 {
        return Err(
            "RF_ANTENNA_MEASURED_PERFORMANCE_VALID sweep coverage parameters require all selected rf_measurements to use the same antenna_net."
                .to_string(),
        );
    }
    Ok(())
}

fn validate_rf_measurement_sweep(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    measurements: &[&RfAntennaMeasurement],
    frequency_band: FrequencyBand,
    min_measurement_count: Option<usize>,
    max_frequency_step_mhz: Option<f64>,
) {
    if min_measurement_count.is_none() && max_frequency_step_mhz.is_none() {
        return;
    }
    let in_band = rf_measurement_frequencies_in_band(measurements, frequency_band);
    if let Some(min_count) = min_measurement_count
        && in_band.len() < min_count
    {
        findings.push(rf_measurement_sweep_count_finding(
            scenario,
            measurements,
            frequency_band,
            in_band.len(),
            min_count,
        ));
    }
    if let Some(max_step_mhz) = max_frequency_step_mhz
        && let Some((measured_gap_mhz, gap_start_mhz, gap_end_mhz)) =
            max_rf_measurement_frequency_gap(&in_band, frequency_band)
        && measured_gap_mhz > max_step_mhz + f64::EPSILON
    {
        findings.push(rf_measurement_sweep_gap_finding(
            scenario,
            measurements,
            frequency_band,
            measured_gap_mhz,
            gap_start_mhz,
            gap_end_mhz,
            max_step_mhz,
        ));
    }
}

fn rf_measurement_frequencies_in_band(
    measurements: &[&RfAntennaMeasurement],
    frequency_band: FrequencyBand,
) -> Vec<f64> {
    let mut frequencies = measurements
        .iter()
        .map(|measurement| measurement.frequency_mhz)
        .filter(|frequency_mhz| {
            frequency_band
                .min_mhz
                .is_none_or(|min_mhz| *frequency_mhz >= min_mhz - f64::EPSILON)
                && frequency_band
                    .max_mhz
                    .is_none_or(|max_mhz| *frequency_mhz <= max_mhz + f64::EPSILON)
        })
        .collect::<Vec<_>>();
    frequencies.sort_by(f64::total_cmp);
    frequencies.dedup_by(|left, right| (*left - *right).abs() <= f64::EPSILON);
    frequencies
}

fn max_rf_measurement_frequency_gap(
    in_band_frequencies: &[f64],
    frequency_band: FrequencyBand,
) -> Option<(f64, f64, f64)> {
    if in_band_frequencies.is_empty() {
        return match (frequency_band.min_mhz, frequency_band.max_mhz) {
            (Some(min_mhz), Some(max_mhz)) => Some((max_mhz - min_mhz, min_mhz, max_mhz)),
            _ => None,
        };
    }
    let mut largest_gap = None::<(f64, f64, f64)>;
    let mut update_gap = |start_mhz: f64, end_mhz: f64| {
        let gap_mhz = end_mhz - start_mhz;
        if gap_mhz.is_finite()
            && gap_mhz >= 0.0
            && largest_gap.is_none_or(|(largest_mhz, _, _)| gap_mhz > largest_mhz)
        {
            largest_gap = Some((gap_mhz, start_mhz, end_mhz));
        }
    };
    if let Some(min_mhz) = frequency_band.min_mhz {
        update_gap(min_mhz, in_band_frequencies[0]);
    }
    for window in in_band_frequencies.windows(2) {
        update_gap(window[0], window[1]);
    }
    if let Some(max_mhz) = frequency_band.max_mhz {
        update_gap(
            *in_band_frequencies.last().expect("checked non-empty"),
            max_mhz,
        );
    }
    largest_gap
}

fn validate_keepout_metadata(
    bound: &BoundBoard<'_>,
    rule: &RfAntennaKeepoutRule,
) -> Result<(), String> {
    if rule.name.trim().is_empty() {
        return Err("RF_ANTENNA_KEEPOUT_VALID keepout name must be non-empty.".to_string());
    }
    if rule.layer.trim().is_empty() {
        return Err(format!(
            "RF_ANTENNA_KEEPOUT_VALID keepout {} layer must be non-empty.",
            rule.name
        ));
    }
    if !rule.min_copper_clearance_mm.is_finite() || rule.min_copper_clearance_mm < 0.0 {
        return Err(format!(
            "RF_ANTENNA_KEEPOUT_VALID keepout {} min_copper_clearance_mm must be finite and non-negative.",
            rule.name
        ));
    }
    if rule.source.trim().is_empty() {
        return Err(format!(
            "RF_ANTENNA_KEEPOUT_VALID keepout {} source must be non-empty.",
            rule.name
        ));
    }
    if let Some(net) = rule.antenna_net.as_deref()
        && !bound.project.board.nets.contains_key(net)
    {
        return Err(format!(
            "RF_ANTENNA_KEEPOUT_VALID keepout {} antenna_net {net} is absent from board.nets.",
            rule.name
        ));
    }
    validate_polygon(&rule.name, &rule.polygon)
}

fn validate_feed_path_metadata(
    bound: &BoundBoard<'_>,
    rule: &RfAntennaFeedPathRule,
) -> Result<(), String> {
    if rule.name.trim().is_empty() {
        return Err("RF_ANTENNA_FEED_PATH_VALID feed path name must be non-empty.".to_string());
    }
    if rule.antenna_net.trim().is_empty()
        || !bound.project.board.nets.contains_key(&rule.antenna_net)
    {
        return Err(format!(
            "RF_ANTENNA_FEED_PATH_VALID feed path {} antenna_net {} is absent from board.nets.",
            rule.name, rule.antenna_net
        ));
    }
    if rule.feed_component.trim().is_empty()
        || !bound
            .project
            .board
            .components
            .contains_key(&rule.feed_component)
    {
        return Err(format!(
            "RF_ANTENNA_FEED_PATH_VALID feed path {} feed_component {} is absent from board.components.",
            rule.name, rule.feed_component
        ));
    }
    let feed_component = &bound.project.board.components[&rule.feed_component];
    if feed_component.pins.get(&rule.feed_pin) != Some(&rule.antenna_net) {
        return Err(format!(
            "RF_ANTENNA_FEED_PATH_VALID feed path {} feed pin {}.{} must be explicitly connected to antenna_net {}.",
            rule.name, rule.feed_component, rule.feed_pin, rule.antenna_net
        ));
    }
    if rule.matching_components.is_empty() {
        return Err(format!(
            "RF_ANTENNA_FEED_PATH_VALID feed path {} requires at least one reviewed matching component.",
            rule.name
        ));
    }
    for component in &rule.matching_components {
        if component.trim().is_empty() || !bound.project.board.components.contains_key(component) {
            return Err(format!(
                "RF_ANTENNA_FEED_PATH_VALID feed path {} matching component {component} is absent from board.components.",
                rule.name
            ));
        }
        if !component_has_antenna_net_pin(bound, component, &rule.antenna_net) {
            return Err(format!(
                "RF_ANTENNA_FEED_PATH_VALID feed path {} matching component {component} has no explicit pin on antenna_net {}.",
                rule.name, rule.antenna_net
            ));
        }
    }
    if !rule.max_feed_route_length_mm.is_finite() || rule.max_feed_route_length_mm < 0.0 {
        return Err(format!(
            "RF_ANTENNA_FEED_PATH_VALID feed path {} max_feed_route_length_mm must be finite and non-negative.",
            rule.name
        ));
    }
    if !rule.max_matching_component_distance_mm.is_finite()
        || rule.max_matching_component_distance_mm < 0.0
    {
        return Err(format!(
            "RF_ANTENNA_FEED_PATH_VALID feed path {} max_matching_component_distance_mm must be finite and non-negative.",
            rule.name
        ));
    }
    if rule.source.trim().is_empty() {
        return Err(format!(
            "RF_ANTENNA_FEED_PATH_VALID feed path {} source must be non-empty.",
            rule.name
        ));
    }
    Ok(())
}

fn validate_rf_measurement_metadata(
    bound: &BoundBoard<'_>,
    measurement: &RfAntennaMeasurement,
) -> Result<(), String> {
    if measurement.name.trim().is_empty() {
        return Err(
            "RF_ANTENNA_MEASURED_PERFORMANCE_VALID measurement name must be non-empty.".to_string(),
        );
    }
    if measurement.antenna_net.trim().is_empty()
        || !bound
            .project
            .board
            .nets
            .contains_key(&measurement.antenna_net)
    {
        return Err(format!(
            "RF_ANTENNA_MEASURED_PERFORMANCE_VALID measurement {} antenna_net {} is absent from board.nets.",
            measurement.name, measurement.antenna_net
        ));
    }
    if !measurement.frequency_mhz.is_finite() || measurement.frequency_mhz <= 0.0 {
        return Err(format!(
            "RF_ANTENNA_MEASURED_PERFORMANCE_VALID measurement {} frequency_mhz must be finite and positive.",
            measurement.name
        ));
    }
    if !measurement.return_loss_db.is_finite() || measurement.return_loss_db <= 0.0 {
        return Err(format!(
            "RF_ANTENNA_MEASURED_PERFORMANCE_VALID measurement {} return_loss_db must be finite and positive.",
            measurement.name
        ));
    }
    if measurement.source.trim().is_empty() {
        return Err(format!(
            "RF_ANTENNA_MEASURED_PERFORMANCE_VALID measurement {} source must be non-empty.",
            measurement.name
        ));
    }
    Ok(())
}

fn validate_matching_network_metadata(
    bound: &BoundBoard<'_>,
    rule: &RfAntennaMatchingNetworkRule,
) -> Result<(), String> {
    if rule.name.trim().is_empty() {
        return Err(
            "RF_ANTENNA_MATCHING_TOPOLOGY_VALID matching network name must be non-empty."
                .to_string(),
        );
    }
    if rule.antenna_net.trim().is_empty()
        || !bound.project.board.nets.contains_key(&rule.antenna_net)
    {
        return Err(format!(
            "RF_ANTENNA_MATCHING_TOPOLOGY_VALID matching network {} antenna_net {} is absent from board.nets.",
            rule.name, rule.antenna_net
        ));
    }
    if rule.source.trim().is_empty() {
        return Err(format!(
            "RF_ANTENNA_MATCHING_TOPOLOGY_VALID matching network {} source must be non-empty.",
            rule.name
        ));
    }
    if !matches!(
        normalize_rf_token(&rule.topology).as_str(),
        "series" | "l" | "pi" | "t" | "custom"
    ) {
        return Err(format!(
            "RF_ANTENNA_MATCHING_TOPOLOGY_VALID matching network {} topology must be series, l, pi, t, or custom.",
            rule.name
        ));
    }
    if let Some(reference_net) = rule.reference_net.as_deref()
        && !bound.project.board.nets.contains_key(reference_net)
    {
        return Err(format!(
            "RF_ANTENNA_MATCHING_TOPOLOGY_VALID matching network {} reference_net {reference_net} is absent from board.nets.",
            rule.name
        ));
    }
    if rule.elements.is_empty() {
        return Err(format!(
            "RF_ANTENNA_MATCHING_TOPOLOGY_VALID matching network {} requires at least one reviewed element.",
            rule.name
        ));
    }
    if !rule
        .elements
        .iter()
        .any(|element| element_touches_net(element, &rule.antenna_net))
    {
        return Err(format!(
            "RF_ANTENNA_MATCHING_TOPOLOGY_VALID matching network {} requires at least one element tied to antenna_net {}.",
            rule.name, rule.antenna_net
        ));
    }
    Ok(())
}

fn validate_matching_element(
    bound: &BoundBoard<'_>,
    rule: &RfAntennaMatchingNetworkRule,
    element: &RfAntennaMatchingElement,
    index: usize,
) -> Result<(), String> {
    if element.component.trim().is_empty()
        || !bound
            .project
            .board
            .components
            .contains_key(&element.component)
    {
        return Err(format!(
            "RF_ANTENNA_MATCHING_TOPOLOGY_VALID matching network {} element {index} component {} is absent from board.components.",
            rule.name, element.component
        ));
    }
    match normalize_rf_token(&element.role).as_str() {
        "series" => {
            let input_net = required_element_net(rule, element, index, "input_net")?;
            let output_net = required_element_net(rule, element, index, "output_net")?;
            if input_net == output_net {
                return Err(format!(
                    "RF_ANTENNA_MATCHING_TOPOLOGY_VALID matching network {} element {index} series input_net and output_net must differ.",
                    rule.name
                ));
            }
            validate_net_exists(bound, rule, index, input_net)?;
            validate_net_exists(bound, rule, index, output_net)?;
            validate_component_pin_and_pad(bound, rule, index, element, input_net)?;
            validate_component_pin_and_pad(bound, rule, index, element, output_net)?;
        }
        "shunt" => {
            let signal_net = required_element_net(rule, element, index, "signal_net")?;
            let reference_net = element
                .reference_net
                .as_deref()
                .or(rule.reference_net.as_deref())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| {
                    format!(
                        "RF_ANTENNA_MATCHING_TOPOLOGY_VALID matching network {} element {index} shunt requires reference_net or rule reference_net.",
                        rule.name
                    )
                })?;
            if signal_net == reference_net {
                return Err(format!(
                    "RF_ANTENNA_MATCHING_TOPOLOGY_VALID matching network {} element {index} shunt signal_net and reference_net must differ.",
                    rule.name
                ));
            }
            validate_net_exists(bound, rule, index, signal_net)?;
            validate_net_exists(bound, rule, index, reference_net)?;
            validate_component_pin_and_pad(bound, rule, index, element, signal_net)?;
            validate_component_pin_and_pad(bound, rule, index, element, reference_net)?;
        }
        _ => {
            return Err(format!(
                "RF_ANTENNA_MATCHING_TOPOLOGY_VALID matching network {} element {index} role must be series or shunt.",
                rule.name
            ));
        }
    }
    Ok(())
}

fn required_positive_numeric_parameter(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    check_id: &str,
    parameter_name: &str,
) -> Option<f64> {
    let Some(value) = scenario.parameters.get(parameter_name) else {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} requires parameters.{parameter_name}."),
        );
        return None;
    };
    let Some(value) = value.as_f64() else {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} parameters.{parameter_name} must be numeric."),
        );
        return None;
    };
    if !value.is_finite() || value <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} parameters.{parameter_name} must be finite and positive."),
        );
        return None;
    }
    Some(value)
}

fn optional_frequency_band(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> Option<FrequencyBand> {
    let min_mhz = optional_positive_numeric_parameter(
        scenario,
        findings,
        RF_ANTENNA_MEASURED_PERFORMANCE_VALID,
        "frequency_min_mhz",
    )?;
    let max_mhz = optional_positive_numeric_parameter(
        scenario,
        findings,
        RF_ANTENNA_MEASURED_PERFORMANCE_VALID,
        "frequency_max_mhz",
    )?;
    if let (Some(min_mhz), Some(max_mhz)) = (min_mhz, max_mhz)
        && max_mhz < min_mhz
    {
        validation_input_missing(
            findings,
            scenario,
            "RF_ANTENNA_MEASURED_PERFORMANCE_VALID parameters.frequency_max_mhz must be greater than or equal to parameters.frequency_min_mhz.",
        );
        return None;
    }
    Some(FrequencyBand { min_mhz, max_mhz })
}

fn optional_positive_numeric_parameter(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    check_id: &str,
    parameter_name: &str,
) -> Option<Option<f64>> {
    let Some(value) = scenario.parameters.get(parameter_name) else {
        return Some(None);
    };
    let Some(value) = value.as_f64() else {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} parameters.{parameter_name} must be numeric when provided."),
        );
        return None;
    };
    if !value.is_finite() || value <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "{check_id} parameters.{parameter_name} must be finite and positive when provided."
            ),
        );
        return None;
    }
    Some(Some(value))
}

fn optional_positive_usize_parameter(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    check_id: &str,
    parameter_name: &str,
) -> Option<Option<usize>> {
    let Some(value) = scenario.parameters.get(parameter_name) else {
        return Some(None);
    };
    let Some(value) = value.as_u64() else {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} parameters.{parameter_name} must be an integer when provided."),
        );
        return None;
    };
    let Ok(value) = usize::try_from(value) else {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} parameters.{parameter_name} is too large for this platform."),
        );
        return None;
    };
    if value == 0 {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} parameters.{parameter_name} must be positive when provided."),
        );
        return None;
    }
    Some(Some(value))
}

#[derive(Debug, Clone, Copy)]
struct MatchingRoleCounts {
    series: usize,
    shunt: usize,
}

fn matching_role_counts(elements: &[RfAntennaMatchingElement]) -> MatchingRoleCounts {
    let mut counts = MatchingRoleCounts {
        series: 0,
        shunt: 0,
    };
    for element in elements {
        match normalize_rf_token(&element.role).as_str() {
            "series" => counts.series += 1,
            "shunt" => counts.shunt += 1,
            _ => {}
        }
    }
    counts
}

fn topology_counts_match(topology: &str, counts: MatchingRoleCounts) -> bool {
    match normalize_rf_token(topology).as_str() {
        "series" => counts.series >= 1 && counts.shunt == 0,
        "l" => counts.series >= 1 && counts.shunt >= 1,
        "pi" => counts.series >= 1 && counts.shunt >= 2,
        "t" => counts.series >= 2 && counts.shunt >= 1,
        "custom" => counts.series + counts.shunt > 0,
        _ => false,
    }
}

fn normalize_rf_token(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn element_touches_net(element: &RfAntennaMatchingElement, net: &str) -> bool {
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

fn required_element_net<'a>(
    rule: &RfAntennaMatchingNetworkRule,
    element: &'a RfAntennaMatchingElement,
    index: usize,
    field: &str,
) -> Result<&'a str, String> {
    let value = match field {
        "input_net" => element.input_net.as_deref(),
        "output_net" => element.output_net.as_deref(),
        "signal_net" => element.signal_net.as_deref(),
        _ => None,
    };
    value.map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            format!(
                "RF_ANTENNA_MATCHING_TOPOLOGY_VALID matching network {} element {index} requires {field}.",
                rule.name
            )
        })
}

fn validate_net_exists(
    bound: &BoundBoard<'_>,
    rule: &RfAntennaMatchingNetworkRule,
    index: usize,
    net: &str,
) -> Result<(), String> {
    if bound.project.board.nets.contains_key(net) {
        Ok(())
    } else {
        Err(format!(
            "RF_ANTENNA_MATCHING_TOPOLOGY_VALID matching network {} element {index} net {net} is absent from board.nets.",
            rule.name
        ))
    }
}

fn validate_component_pin_and_pad(
    bound: &BoundBoard<'_>,
    rule: &RfAntennaMatchingNetworkRule,
    index: usize,
    element: &RfAntennaMatchingElement,
    net: &str,
) -> Result<(), String> {
    if !component_has_pin_on_net(bound, &element.component, net) {
        return Err(format!(
            "RF_ANTENNA_MATCHING_TOPOLOGY_VALID matching network {} element {index} component {} has no explicit pin on net {net}.",
            rule.name, element.component
        ));
    }
    if !component_has_layout_pad_on_net(bound, &element.component, net) {
        return Err(format!(
            "RF_ANTENNA_MATCHING_TOPOLOGY_VALID matching network {} element {index} component {} has no finite layout pad on net {net}.",
            rule.name, element.component
        ));
    }
    Ok(())
}

fn validate_polygon(name: &str, polygon: &[LayoutPoint]) -> Result<(), String> {
    if polygon.len() < 3 {
        return Err(format!(
            "RF_ANTENNA_KEEPOUT_VALID keepout {name} polygon must contain at least three points."
        ));
    }
    if polygon
        .iter()
        .any(|point| !point.x_mm.is_finite() || !point.y_mm.is_finite())
    {
        return Err(format!(
            "RF_ANTENNA_KEEPOUT_VALID keepout {name} polygon points must be finite."
        ));
    }
    let area = polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(polygon.len())
        .map(|(left, right)| left.x_mm * right.y_mm - right.x_mm * left.y_mm)
        .sum::<f64>()
        .abs()
        / 2.0;
    if area <= f64::EPSILON {
        return Err(format!(
            "RF_ANTENNA_KEEPOUT_VALID keepout {name} polygon must be non-degenerate."
        ));
    }
    Ok(())
}

fn copper_feature_is_comparable(
    rule: &RfAntennaKeepoutRule,
    feature: &LayoutCopperFeature,
) -> bool {
    feature.layer == rule.layer && !same_antenna_net(rule, feature.net.as_deref())
}

fn copper_segment_is_comparable(
    rule: &RfAntennaKeepoutRule,
    segment: &LayoutCopperSegment,
) -> bool {
    segment.layer == rule.layer && !same_antenna_net(rule, segment.net.as_deref())
}

fn copper_region_is_comparable(rule: &RfAntennaKeepoutRule, region: &LayoutCopperRegion) -> bool {
    region.layer == rule.layer && !same_antenna_net(rule, region.net.as_deref())
}

fn same_antenna_net(rule: &RfAntennaKeepoutRule, net: Option<&str>) -> bool {
    matches!((rule.antenna_net.as_deref(), net), (Some(antenna), Some(candidate)) if antenna == candidate)
}

fn feature_keepout_finding(
    scenario: &Scenario,
    rule: &RfAntennaKeepoutRule,
    feature: &LayoutCopperFeature,
    index: usize,
    clearance_mm: f64,
) -> Finding {
    let mut finding = base_keepout_finding(scenario, rule, "feature", index, clearance_mm);
    finding
        .measured
        .insert("copper_feature_shape".to_string(), json!(feature.shape));
    finding
        .measured
        .insert("copper_feature_at".to_string(), point_json(&feature.at));
    finding.measured.insert(
        "copper_feature_size".to_string(),
        json!({
            "x_mm": feature.size.x_mm,
            "y_mm": feature.size.y_mm
        }),
    );
    insert_optional_copper_owner(
        &mut finding,
        feature.net.as_deref(),
        feature.component.as_deref(),
    );
    finding
}

fn segment_keepout_finding(
    scenario: &Scenario,
    rule: &RfAntennaKeepoutRule,
    segment: &LayoutCopperSegment,
    index: usize,
    clearance_mm: f64,
) -> Finding {
    let mut finding = base_keepout_finding(scenario, rule, "segment", index, clearance_mm);
    finding.measured.insert(
        "copper_segment_start".to_string(),
        point_json(&segment.start),
    );
    finding
        .measured
        .insert("copper_segment_end".to_string(), point_json(&segment.end));
    finding.measured.insert(
        "copper_segment_width_mm".to_string(),
        json!(segment.width_mm),
    );
    insert_optional_copper_owner(
        &mut finding,
        segment.net.as_deref(),
        segment.component.as_deref(),
    );
    finding
}

fn region_keepout_finding(
    scenario: &Scenario,
    rule: &RfAntennaKeepoutRule,
    region: &LayoutCopperRegion,
    index: usize,
    clearance_mm: f64,
) -> Finding {
    let mut finding = base_keepout_finding(scenario, rule, "region", index, clearance_mm);
    finding.measured.insert(
        "copper_region_point_count".to_string(),
        json!(region.points.len()),
    );
    let intrudes = region
        .points
        .iter()
        .any(|point| point_inside_polygon(point, &rule.polygon))
        || rule
            .polygon
            .iter()
            .any(|point| point_inside_polygon(point, &region.points));
    finding.measured.insert(
        "copper_region_intrudes_keepout".to_string(),
        json!(intrudes),
    );
    insert_optional_copper_owner(
        &mut finding,
        region.net.as_deref(),
        region.component.as_deref(),
    );
    finding
}

fn base_keepout_finding(
    scenario: &Scenario,
    rule: &RfAntennaKeepoutRule,
    copper_kind: &str,
    copper_index: usize,
    clearance_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        RF_ANTENNA_KEEPOUT_VALID,
        &scenario.name,
        format!(
            "RF antenna keepout {} has {copper_kind} copper clearance {:.3} mm below the reviewed {:.3} mm limit.",
            rule.name, clearance_mm, rule.min_copper_clearance_mm
        ),
    );
    finding
        .measured
        .insert("keepout_name".to_string(), json!(rule.name));
    finding
        .measured
        .insert("keepout_source".to_string(), json!(rule.source));
    finding
        .measured
        .insert("keepout_layer".to_string(), json!(rule.layer));
    finding.measured.insert(
        "keepout_polygon_point_count".to_string(),
        json!(rule.polygon.len()),
    );
    finding
        .measured
        .insert("copper_kind".to_string(), json!(copper_kind));
    finding
        .measured
        .insert("copper_index".to_string(), json!(copper_index));
    finding
        .measured
        .insert("clearance_mm".to_string(), json!(clearance_mm));
    if let Some(net) = &rule.antenna_net {
        finding
            .measured
            .insert("antenna_net".to_string(), json!(net));
    }
    finding.limit.insert(
        "min_copper_clearance_mm".to_string(),
        json!(rule.min_copper_clearance_mm),
    );
    finding.suggested_fixes = vec![
        "Move non-antenna copper outside the reviewed antenna keepout polygon.".to_string(),
        "Update the keepout only from antenna datasheet, module layout-guide, or RF review evidence.".to_string(),
        "Use RF simulation or measurement for final antenna performance; this check only screens imported copper geometry against explicit keepout metadata.".to_string(),
    ];
    finding
}

fn feed_pad<'a>(bound: &'a BoundBoard<'_>, rule: &RfAntennaFeedPathRule) -> Option<&'a LayoutPad> {
    let pad = bound
        .project
        .board
        .layout
        .pads
        .get(&rule.feed_component)?
        .get(&rule.feed_pin)?;
    (pad.net == rule.antenna_net).then_some(pad)
}

fn validate_pad_geometry(
    rule_name: &str,
    component: &str,
    pin: &str,
    pad: &LayoutPad,
) -> Result<(), String> {
    if !pad.at.x_mm.is_finite() || !pad.at.y_mm.is_finite() {
        return Err(format!(
            "RF_ANTENNA_FEED_PATH_VALID feed path {rule_name} pad {component}.{pin} coordinates must be finite."
        ));
    }
    Ok(())
}

fn validate_route_geometry(net: &str, route: &NetRoute) -> Result<(), String> {
    if route.segments.is_empty() {
        return Err(format!(
            "RF_ANTENNA_FEED_PATH_VALID antenna_net {net} route must contain at least one segment."
        ));
    }
    for (index, segment) in route.segments.iter().enumerate() {
        validate_route_segment(net, index, segment)?;
    }
    Ok(())
}

fn validate_route_segment(net: &str, index: usize, segment: &RouteSegment) -> Result<(), String> {
    if segment.layer.trim().is_empty()
        || !segment.width_mm.is_finite()
        || segment.width_mm <= 0.0
        || !segment.start.x_mm.is_finite()
        || !segment.start.y_mm.is_finite()
        || !segment.end.x_mm.is_finite()
        || !segment.end.y_mm.is_finite()
        || route_segment_length_mm(segment) <= f64::EPSILON
    {
        return Err(format!(
            "RF_ANTENNA_FEED_PATH_VALID antenna_net {net} route segment {index} must have finite non-zero geometry, positive width, and a non-empty layer."
        ));
    }
    Ok(())
}

fn route_length_mm(route: &NetRoute) -> f64 {
    route.segments.iter().map(route_segment_length_mm).sum()
}

fn route_segment_length_mm(segment: &RouteSegment) -> f64 {
    (segment.end.x_mm - segment.start.x_mm).hypot(segment.end.y_mm - segment.start.y_mm)
}

fn component_has_antenna_net_pin(
    bound: &BoundBoard<'_>,
    component: &str,
    antenna_net: &str,
) -> bool {
    bound
        .project
        .board
        .components
        .get(component)
        .is_some_and(|spec| spec.pins.values().any(|net| net == antenna_net))
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

fn matching_component_distance_mm(
    bound: &BoundBoard<'_>,
    rule: &RfAntennaFeedPathRule,
    component: &str,
) -> Option<f64> {
    let feed = bound
        .project
        .board
        .layout
        .placements
        .get(&rule.feed_component)?;
    let matching = bound.project.board.layout.placements.get(component)?;
    if !placement_is_finite(feed) || !placement_is_finite(matching) {
        return None;
    }
    has_antenna_net_layout_pad(bound, component, &rule.antenna_net)
        .then_some(placement_distance_mm(feed, matching))
}

fn has_antenna_net_layout_pad(bound: &BoundBoard<'_>, component: &str, antenna_net: &str) -> bool {
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

fn placement_is_finite(placement: &ComponentPlacement) -> bool {
    placement.x_mm.is_finite() && placement.y_mm.is_finite()
}

fn placement_distance_mm(first: &ComponentPlacement, second: &ComponentPlacement) -> f64 {
    (first.x_mm - second.x_mm).hypot(first.y_mm - second.y_mm)
}

fn feed_route_length_finding(
    scenario: &Scenario,
    rule: &RfAntennaFeedPathRule,
    route_length_mm: f64,
    route_segment_count: usize,
) -> Finding {
    let mut finding = base_feed_path_finding(
        scenario,
        rule,
        format!(
            "RF antenna feed path {} route length {:.3} mm exceeds the reviewed {:.3} mm limit.",
            rule.name, route_length_mm, rule.max_feed_route_length_mm
        ),
    );
    finding
        .measured
        .insert("feed_route_length_mm".to_string(), json!(route_length_mm));
    finding.measured.insert(
        "feed_route_segment_count".to_string(),
        json!(route_segment_count),
    );
    finding.limit.insert(
        "max_feed_route_length_mm".to_string(),
        json!(rule.max_feed_route_length_mm),
    );
    finding
}

fn matching_component_distance_finding(
    scenario: &Scenario,
    rule: &RfAntennaFeedPathRule,
    component: &str,
    distance_mm: f64,
) -> Finding {
    let mut finding = base_feed_path_finding(
        scenario,
        rule,
        format!(
            "RF antenna feed path {} matching component {component} is {:.3} mm from the feed component, above the reviewed {:.3} mm limit.",
            rule.name, distance_mm, rule.max_matching_component_distance_mm
        ),
    );
    finding
        .measured
        .insert("matching_component".to_string(), json!(component));
    finding.measured.insert(
        "matching_component_distance_mm".to_string(),
        json!(distance_mm),
    );
    finding.limit.insert(
        "max_matching_component_distance_mm".to_string(),
        json!(rule.max_matching_component_distance_mm),
    );
    finding
}

fn base_feed_path_finding(
    scenario: &Scenario,
    rule: &RfAntennaFeedPathRule,
    message: String,
) -> Finding {
    let mut finding = Finding::critical(RF_ANTENNA_FEED_PATH_VALID, &scenario.name, message);
    finding
        .measured
        .insert("feed_path_name".to_string(), json!(rule.name));
    finding
        .measured
        .insert("feed_path_source".to_string(), json!(rule.source));
    finding
        .measured
        .insert("antenna_net".to_string(), json!(rule.antenna_net));
    finding
        .measured
        .insert("feed_component".to_string(), json!(rule.feed_component));
    finding
        .measured
        .insert("feed_pin".to_string(), json!(rule.feed_pin));
    finding.measured.insert(
        "matching_component_count".to_string(),
        json!(rule.matching_components.len()),
    );
    finding.suggested_fixes = vec![
        "Move the antenna matching components closer to the reviewed feed component/pin.".to_string(),
        "Shorten the imported antenna feed route or update the limit only from antenna module, RF layout-guide, or RF review evidence.".to_string(),
        "Use RF simulation or measured S-parameters for final matching quality; this check only screens explicit feed-path layout evidence.".to_string(),
    ];
    finding
}

fn matching_topology_finding(
    scenario: &Scenario,
    rule: &RfAntennaMatchingNetworkRule,
    role_counts: MatchingRoleCounts,
) -> Finding {
    let mut finding = Finding::critical(
        RF_ANTENNA_MATCHING_TOPOLOGY_VALID,
        &scenario.name,
        format!(
            "RF antenna matching network {} declares {} topology but has {} series and {} shunt reviewed elements.",
            rule.name, rule.topology, role_counts.series, role_counts.shunt
        ),
    );
    finding
        .measured
        .insert("matching_network_name".to_string(), json!(rule.name));
    finding
        .measured
        .insert("matching_network_source".to_string(), json!(rule.source));
    finding
        .measured
        .insert("antenna_net".to_string(), json!(rule.antenna_net));
    finding
        .measured
        .insert("topology".to_string(), json!(rule.topology));
    finding.measured.insert(
        "series_element_count".to_string(),
        json!(role_counts.series),
    );
    finding
        .measured
        .insert("shunt_element_count".to_string(), json!(role_counts.shunt));
    finding
        .measured
        .insert("element_count".to_string(), json!(rule.elements.len()));
    finding
        .limit
        .insert("required_topology".to_string(), json!(rule.topology));
    finding.suggested_fixes = vec![
        "Update the reviewed matching-network metadata if the intended RF topology is different."
            .to_string(),
        "Add missing explicit component pin and layout pad evidence for every reviewed matching element."
            .to_string(),
        "Use RF simulation or measured S-parameters for matching quality; this check only screens explicit matching topology evidence.".to_string(),
    ];
    finding
}

fn rf_measurement_frequency_finding(
    scenario: &Scenario,
    measurement: &RfAntennaMeasurement,
    frequency_band: FrequencyBand,
) -> Finding {
    let mut finding = base_rf_measurement_finding(
        scenario,
        measurement,
        format!(
            "RF antenna measurement {} frequency {:.3} MHz is outside the reviewed frequency band.",
            measurement.name, measurement.frequency_mhz
        ),
    );
    finding
        .measured
        .insert("frequency_in_band".to_string(), json!(false));
    insert_frequency_band_limits(&mut finding, frequency_band);
    finding
}

fn rf_measurement_return_loss_finding(
    scenario: &Scenario,
    measurement: &RfAntennaMeasurement,
    min_return_loss_db: f64,
    frequency_band: FrequencyBand,
) -> Finding {
    let mut finding = base_rf_measurement_finding(
        scenario,
        measurement,
        format!(
            "RF antenna measurement {} return loss {:.3} dB is below the reviewed minimum {:.3} dB.",
            measurement.name, measurement.return_loss_db, min_return_loss_db
        ),
    );
    finding
        .measured
        .insert("frequency_in_band".to_string(), json!(true));
    finding
        .limit
        .insert("min_return_loss_db".to_string(), json!(min_return_loss_db));
    insert_frequency_band_limits(&mut finding, frequency_band);
    finding
}

fn rf_measurement_sweep_count_finding(
    scenario: &Scenario,
    measurements: &[&RfAntennaMeasurement],
    frequency_band: FrequencyBand,
    measured_count: usize,
    min_count: usize,
) -> Finding {
    let mut finding = base_rf_measurement_sweep_finding(
        scenario,
        measurements,
        format!(
            "RF antenna measured-performance sweep has {measured_count} unique in-band point(s), below the reviewed minimum {min_count}."
        ),
    );
    finding.measured.insert(
        "unique_in_band_measurement_count".to_string(),
        json!(measured_count),
    );
    finding
        .limit
        .insert("min_measurement_count".to_string(), json!(min_count));
    insert_frequency_band_limits(&mut finding, frequency_band);
    finding
}

fn rf_measurement_sweep_gap_finding(
    scenario: &Scenario,
    measurements: &[&RfAntennaMeasurement],
    frequency_band: FrequencyBand,
    measured_gap_mhz: f64,
    gap_start_mhz: f64,
    gap_end_mhz: f64,
    max_step_mhz: f64,
) -> Finding {
    let mut finding = base_rf_measurement_sweep_finding(
        scenario,
        measurements,
        format!(
            "RF antenna measured-performance sweep has a {:.3} MHz frequency gap, above the reviewed {:.3} MHz maximum step.",
            measured_gap_mhz, max_step_mhz
        ),
    );
    finding
        .measured
        .insert("max_frequency_gap_mhz".to_string(), json!(measured_gap_mhz));
    finding
        .measured
        .insert("frequency_gap_start_mhz".to_string(), json!(gap_start_mhz));
    finding
        .measured
        .insert("frequency_gap_end_mhz".to_string(), json!(gap_end_mhz));
    finding
        .limit
        .insert("max_frequency_step_mhz".to_string(), json!(max_step_mhz));
    insert_frequency_band_limits(&mut finding, frequency_band);
    finding
}

fn base_rf_measurement_sweep_finding(
    scenario: &Scenario,
    measurements: &[&RfAntennaMeasurement],
    message: String,
) -> Finding {
    let mut finding = Finding::critical(
        RF_ANTENNA_MEASURED_PERFORMANCE_VALID,
        &scenario.name,
        message,
    );
    finding.measured.insert(
        "measurement_names".to_string(),
        json!(
            measurements
                .iter()
                .map(|measurement| measurement.name.as_str())
                .collect::<Vec<_>>()
        ),
    );
    finding.measured.insert(
        "measurement_frequencies_mhz".to_string(),
        json!(
            measurements
                .iter()
                .map(|measurement| measurement.frequency_mhz)
                .collect::<Vec<_>>()
        ),
    );
    if let Some(first) = measurements.first() {
        finding
            .measured
            .insert("antenna_net".to_string(), json!(first.antenna_net));
    }
    finding.suggested_fixes = vec![
        "Import additional reviewed VNA sweep points inside the operating band or relax the reviewed sweep coverage policy.".to_string(),
        "Re-run the RF measurement with the reviewed antenna fixture, calibration, and enclosure state.".to_string(),
        "Use RF simulation or chamber/VNA measurements for final antenna qualification; this check only screens explicit measured S-parameter evidence.".to_string(),
    ];
    finding
}

fn base_rf_measurement_finding(
    scenario: &Scenario,
    measurement: &RfAntennaMeasurement,
    message: String,
) -> Finding {
    let mut finding = Finding::critical(
        RF_ANTENNA_MEASURED_PERFORMANCE_VALID,
        &scenario.name,
        message,
    );
    finding
        .measured
        .insert("measurement_name".to_string(), json!(measurement.name));
    finding
        .measured
        .insert("measurement_source".to_string(), json!(measurement.source));
    finding
        .measured
        .insert("antenna_net".to_string(), json!(measurement.antenna_net));
    finding.measured.insert(
        "frequency_mhz".to_string(),
        json!(measurement.frequency_mhz),
    );
    finding.measured.insert(
        "return_loss_db".to_string(),
        json!(measurement.return_loss_db),
    );
    if let Some(method) = measurement.measurement_method.as_deref() {
        finding
            .measured
            .insert("measurement_method".to_string(), json!(method));
    }
    finding.suggested_fixes = vec![
        "Re-run the RF measurement with the reviewed antenna fixture, calibration, and enclosure state.".to_string(),
        "Tune the antenna matching network or feed layout, then import updated source-backed S11 evidence.".to_string(),
        "Use RF simulation or chamber/VNA measurements for final antenna qualification; this check only screens explicit measured return-loss evidence.".to_string(),
    ];
    finding
}

fn insert_frequency_band_limits(finding: &mut Finding, frequency_band: FrequencyBand) {
    if let Some(min_mhz) = frequency_band.min_mhz {
        finding
            .limit
            .insert("frequency_min_mhz".to_string(), json!(min_mhz));
    }
    if let Some(max_mhz) = frequency_band.max_mhz {
        finding
            .limit
            .insert("frequency_max_mhz".to_string(), json!(max_mhz));
    }
}

fn point_json(point: &LayoutPoint) -> serde_json::Value {
    json!({
        "x_mm": point.x_mm,
        "y_mm": point.y_mm
    })
}

fn insert_optional_copper_owner(finding: &mut Finding, net: Option<&str>, component: Option<&str>) {
    if let Some(net) = net {
        finding
            .measured
            .insert("copper_net".to_string(), json!(net));
    }
    if let Some(component) = component {
        finding
            .measured
            .insert("copper_component".to_string(), json!(component));
    }
}
