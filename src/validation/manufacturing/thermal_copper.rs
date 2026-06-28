use crate::board_ir::{
    LayoutCopperFeature, LayoutCopperRegion, LayoutCopperSegment, RouteVia, Scenario, StackupLayer,
    StackupLayerKind, ThermalCopperRule, ThermalPackageRule,
};
use crate::library::{BoundBoard, ComponentModel};
use crate::reports::Finding;
use serde_json::json;

use super::super::common::validation_input_missing;
use super::super::{
    THERMAL_COPPER_AREA_VALID, THERMAL_DERATING_ENVIRONMENT_VALID,
    THERMAL_PACKAGE_TEMPERATURE_VALID, THERMAL_VIA_STACKUP_VALID,
};
use super::geometry::{
    validate_copper_feature_geometry, validate_copper_region_geometry,
    validate_copper_segment_geometry,
};

pub(in crate::validation) fn validate_thermal_copper_area(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(names) = thermal_rule_names(scenario, findings) else {
        return;
    };
    for name in names {
        let Some(rule) = thermal_rule(bound, scenario, findings, THERMAL_COPPER_AREA_VALID, &name)
        else {
            return;
        };
        validate_thermal_rule(bound, scenario, findings, rule);
    }
}

pub(in crate::validation) fn validate_thermal_via_stackup(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(names) = named_thermal_rule_parameters(
        scenario,
        findings,
        THERMAL_VIA_STACKUP_VALID,
        "thermal_copper",
    ) else {
        return;
    };
    for name in names {
        let Some(rule) = thermal_rule(bound, scenario, findings, THERMAL_VIA_STACKUP_VALID, &name)
        else {
            return;
        };
        validate_via_stackup_rule(bound, scenario, findings, rule);
    }
}

pub(in crate::validation) fn validate_thermal_package_temperature(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(names) = named_thermal_rule_parameters(
        scenario,
        findings,
        THERMAL_PACKAGE_TEMPERATURE_VALID,
        "thermal_copper",
    ) else {
        return;
    };
    let Some(ambient_temperature_c) = required_finite_number(
        scenario,
        findings,
        THERMAL_PACKAGE_TEMPERATURE_VALID,
        "ambient_temperature_C",
    ) else {
        return;
    };
    let Some(max_temperature_rise_c) = required_positive_number(
        scenario,
        findings,
        THERMAL_PACKAGE_TEMPERATURE_VALID,
        "max_temperature_rise_C",
    ) else {
        return;
    };
    let Some(max_junction_temperature_margin_c) = optional_nonnegative_number(
        scenario,
        findings,
        THERMAL_PACKAGE_TEMPERATURE_VALID,
        "max_junction_temperature_margin_C",
    ) else {
        return;
    };
    for name in names {
        let Some(rule) = thermal_rule(
            bound,
            scenario,
            findings,
            THERMAL_PACKAGE_TEMPERATURE_VALID,
            &name,
        ) else {
            return;
        };
        validate_package_temperature_rule(
            bound,
            scenario,
            findings,
            rule,
            ambient_temperature_c,
            max_temperature_rise_c,
            max_junction_temperature_margin_c,
        );
    }
}

pub(in crate::validation) fn validate_thermal_derating_environment(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(names) = named_thermal_rule_parameters(
        scenario,
        findings,
        THERMAL_DERATING_ENVIRONMENT_VALID,
        "thermal_copper",
    ) else {
        return;
    };
    for name in names {
        let Some(rule) = thermal_rule(
            bound,
            scenario,
            findings,
            THERMAL_DERATING_ENVIRONMENT_VALID,
            &name,
        ) else {
            return;
        };
        validate_derating_environment_rule(bound, scenario, findings, rule);
    }
}

fn thermal_rule_names(scenario: &Scenario, findings: &mut Vec<Finding>) -> Option<Vec<String>> {
    named_thermal_rule_parameters(
        scenario,
        findings,
        THERMAL_COPPER_AREA_VALID,
        "thermal_copper",
    )
}

pub(super) fn named_thermal_rule_parameters(
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
        let Some(name) = mapping
            .get(serde_yaml_ng::Value::String("name".to_string()))
            .and_then(serde_yaml_ng::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
        else {
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

pub(super) fn thermal_rule<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    check_id: &str,
    name: &str,
) -> Option<&'a ThermalCopperRule> {
    let matches = bound
        .project
        .board
        .manufacturing
        .thermal_copper
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
                    "{check_id} thermal copper rule {name} is absent from board.manufacturing.thermal_copper."
                ),
            );
            None
        }
        _ => {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "{check_id} thermal copper rule {name} is ambiguous in board.manufacturing.thermal_copper."
                ),
            );
            None
        }
    }
}

fn validate_thermal_rule(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    rule: &ThermalCopperRule,
) {
    if let Err(message) = validate_rule_metadata(bound, rule) {
        validation_input_missing(findings, scenario, message);
        return;
    }

    let copper = &bound.project.board.layout.copper;
    let mut evidence = ThermalAreaEvidence::default();
    for (index, feature) in copper.features.iter().enumerate() {
        if !thermal_feature_matches(rule, feature) {
            continue;
        }
        if let Err(message) = validate_copper_feature_geometry(feature, index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        let Some(area_mm2) = feature_area_mm2(feature) else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "THERMAL_COPPER_AREA_VALID thermal copper rule {} cannot measure unsupported copper feature shape {} at board.layout.copper.features[{index}].",
                    rule.name, feature.shape
                ),
            );
            continue;
        };
        evidence.feature_area_mm2 += area_mm2;
        evidence.feature_count += 1;
    }
    for (index, segment) in copper.segments.iter().enumerate() {
        if !thermal_segment_matches(rule, segment) {
            continue;
        }
        if let Err(message) = validate_copper_segment_geometry(segment, index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        evidence.segment_area_mm2 += segment_area_mm2(segment);
        evidence.segment_count += 1;
    }
    for (index, region) in copper.regions.iter().enumerate() {
        if !thermal_region_matches(rule, region) {
            continue;
        }
        if let Err(message) = validate_copper_region_geometry(region, index) {
            validation_input_missing(findings, scenario, message);
            continue;
        }
        evidence.region_area_mm2 += region_area_mm2(region);
        evidence.region_count += 1;
    }

    if evidence.object_count() == 0 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "THERMAL_COPPER_AREA_VALID thermal copper rule {} has no comparable board.layout.copper evidence for component {}, nets {:?}, and layers {:?}.",
                rule.name, rule.component, rule.nets, rule.layers
            ),
        );
        return;
    }

    let total_area_mm2 = evidence.total_area_mm2();
    if total_area_mm2 + f64::EPSILON < rule.min_copper_area_mm2 {
        findings.push(thermal_area_finding(
            scenario,
            rule,
            &evidence,
            total_area_mm2,
        ));
    }
}

fn validate_derating_environment_rule(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    rule: &ThermalCopperRule,
) {
    if let Err(message) = validate_rule_metadata(bound, rule) {
        validation_input_missing(findings, scenario, message);
        return;
    }
    if !has_derating_metadata(rule) {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "THERMAL_DERATING_ENVIRONMENT_VALID thermal copper rule {} must declare rated_ambient_temperature_C, min_airflow_lfm, or enclosure_profile.",
                rule.name
            ),
        );
        return;
    }
    if let Some(rated_ambient_temperature_c) = rule.rated_ambient_temperature_c {
        let Some(ambient_temperature_c) = required_finite_number(
            scenario,
            findings,
            THERMAL_DERATING_ENVIRONMENT_VALID,
            "ambient_temperature_C",
        ) else {
            return;
        };
        if ambient_temperature_c > rated_ambient_temperature_c + f64::EPSILON {
            findings.push(thermal_derating_ambient_finding(
                scenario,
                rule,
                ambient_temperature_c,
                rated_ambient_temperature_c,
            ));
        }
    }
    if let Some(min_airflow_lfm) = rule.min_airflow_lfm {
        let Some(airflow_lfm) = required_nonnegative_number(
            scenario,
            findings,
            THERMAL_DERATING_ENVIRONMENT_VALID,
            "airflow_lfm",
        ) else {
            return;
        };
        if airflow_lfm + f64::EPSILON < min_airflow_lfm {
            findings.push(thermal_derating_airflow_finding(
                scenario,
                rule,
                airflow_lfm,
                min_airflow_lfm,
            ));
        }
    }
    if let Some(expected_enclosure_profile) = rule.enclosure_profile.as_deref() {
        let Some(enclosure_profile) = required_string_parameter(
            scenario,
            findings,
            THERMAL_DERATING_ENVIRONMENT_VALID,
            "enclosure_profile",
        ) else {
            return;
        };
        if enclosure_profile != expected_enclosure_profile {
            findings.push(thermal_derating_enclosure_finding(
                scenario,
                rule,
                &enclosure_profile,
                expected_enclosure_profile,
            ));
        }
    }
}

fn validate_via_stackup_rule(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    rule: &ThermalCopperRule,
) {
    if let Err(message) = validate_rule_metadata(bound, rule) {
        validation_input_missing(findings, scenario, message);
        return;
    }
    let Some(min_thermal_via_count) = rule.min_thermal_via_count else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "THERMAL_VIA_STACKUP_VALID thermal copper rule {} must declare min_thermal_via_count.",
                rule.name
            ),
        );
        return;
    };
    let Some(min_copper_thickness_um) = positive_number(rule.min_copper_thickness_um) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "THERMAL_VIA_STACKUP_VALID thermal copper rule {} must declare finite positive min_copper_thickness_um.",
                rule.name
            ),
        );
        return;
    };
    if min_thermal_via_count == 0 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "THERMAL_VIA_STACKUP_VALID thermal copper rule {} min_thermal_via_count must be positive.",
                rule.name
            ),
        );
        return;
    }
    if rule.nets.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "THERMAL_VIA_STACKUP_VALID thermal copper rule {} requires at least one reviewed net in board.manufacturing.thermal_copper[].nets.",
                rule.name
            ),
        );
        return;
    }
    if rule.layers.len() < 2 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "THERMAL_VIA_STACKUP_VALID thermal copper rule {} requires at least two reviewed copper layers.",
                rule.name
            ),
        );
        return;
    }
    let stackup_layers = &bound.project.board.layout.stackup.layers;
    if stackup_layers.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "THERMAL_VIA_STACKUP_VALID requires board.layout.stackup.layers evidence.",
        );
        return;
    }

    let mut observed_min_copper_thickness_um = f64::INFINITY;
    for layer_name in &rule.layers {
        let Some(layer) = stackup_layer(stackup_layers, layer_name) else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "THERMAL_VIA_STACKUP_VALID layer {layer_name} is absent from board.layout.stackup.layers."
                ),
            );
            return;
        };
        if !matches!(
            layer.kind,
            StackupLayerKind::Signal | StackupLayerKind::Plane
        ) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "THERMAL_VIA_STACKUP_VALID layer {} must be kind: signal or kind: plane.",
                    layer.name
                ),
            );
            return;
        }
        let Some(copper_thickness_um) = positive_number(layer.copper_thickness_um) else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "THERMAL_VIA_STACKUP_VALID stackup layer {} must declare finite positive copper_thickness_um.",
                    layer.name
                ),
            );
            return;
        };
        if layer
            .source
            .as_deref()
            .map(str::trim)
            .is_none_or(str::is_empty)
        {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "THERMAL_VIA_STACKUP_VALID stackup layer {} must declare non-empty source.",
                    layer.name
                ),
            );
            return;
        };
        observed_min_copper_thickness_um =
            observed_min_copper_thickness_um.min(copper_thickness_um);
        if copper_thickness_um + f64::EPSILON < min_copper_thickness_um {
            findings.push(thermal_copper_thickness_finding(
                scenario,
                rule,
                layer,
                copper_thickness_um,
                min_copper_thickness_um,
            ));
        }
    }

    let Some(via_evidence) = thermal_via_evidence(bound, scenario, findings, rule) else {
        return;
    };
    if via_evidence.thermal_via_count < min_thermal_via_count {
        findings.push(thermal_via_count_finding(
            scenario,
            rule,
            &via_evidence,
            min_thermal_via_count,
            min_copper_thickness_um,
            observed_min_copper_thickness_um,
        ));
    }
}

fn validate_package_temperature_rule(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    rule: &ThermalCopperRule,
    ambient_temperature_c: f64,
    max_temperature_rise_c: f64,
    max_junction_temperature_margin_c: f64,
) {
    if let Err(message) = validate_rule_metadata(bound, rule) {
        validation_input_missing(findings, scenario, message);
        return;
    }
    let Some(component) = bound.project.board.components.get(&rule.component) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "THERMAL_PACKAGE_TEMPERATURE_VALID thermal copper rule {} component {} is absent from board.components.",
                rule.name, rule.component
            ),
        );
        return;
    };
    let Some(model) = bound.library.get(&component.model) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "THERMAL_PACKAGE_TEMPERATURE_VALID component {} model {} is unresolved.",
                rule.component, component.model
            ),
        );
        return;
    };
    let package = match package_thermal_metadata(bound, &rule.component, &component.model, model) {
        Ok(package) => package,
        Err(message) => {
            validation_input_missing(findings, scenario, message);
            return;
        }
    };

    let estimated_temperature_rise_c = rule.power_loss_w * package.rja_c_per_w;
    let estimated_junction_temperature_c = ambient_temperature_c + estimated_temperature_rise_c;
    let allowed_junction_temperature_c =
        package.max_junction_temperature_c - max_junction_temperature_margin_c;

    if estimated_temperature_rise_c > max_temperature_rise_c + f64::EPSILON {
        findings.push(thermal_temperature_rise_finding(
            scenario,
            rule,
            &PackageThermalEvidence {
                model_id: &component.model,
                package_source: package.source.as_str(),
                rja_c_per_w: package.rja_c_per_w,
                max_junction_temperature_c: package.max_junction_temperature_c,
                ambient_temperature_c,
                max_temperature_rise_c,
                max_junction_temperature_margin_c,
                estimated_temperature_rise_c,
                estimated_junction_temperature_c,
                allowed_junction_temperature_c,
            },
        ));
    }
    if estimated_junction_temperature_c > allowed_junction_temperature_c + f64::EPSILON {
        findings.push(thermal_junction_temperature_finding(
            scenario,
            rule,
            &PackageThermalEvidence {
                model_id: &component.model,
                package_source: package.source.as_str(),
                rja_c_per_w: package.rja_c_per_w,
                max_junction_temperature_c: package.max_junction_temperature_c,
                ambient_temperature_c,
                max_temperature_rise_c,
                max_junction_temperature_margin_c,
                estimated_temperature_rise_c,
                estimated_junction_temperature_c,
                allowed_junction_temperature_c,
            },
        ));
    }
}

fn package_thermal_metadata(
    bound: &BoundBoard<'_>,
    component_id: &str,
    model_id: &str,
    model: &ComponentModel,
) -> Result<PackageThermalMetadata, String> {
    let mut package_matches = bound
        .project
        .board
        .manufacturing
        .thermal_packages
        .iter()
        .filter(|package| package.component == component_id);
    if let Some(package) = package_matches.next() {
        if package_matches.next().is_some() {
            return Err(format!(
                "THERMAL_PACKAGE_TEMPERATURE_VALID component {component_id} has duplicate board.manufacturing.thermal_packages entries."
            ));
        }
        return package_thermal_metadata_from_board(component_id, package);
    }
    package_thermal_metadata_from_model(component_id, model_id, model)
}

fn package_thermal_metadata_from_board(
    component_id: &str,
    package: &ThermalPackageRule,
) -> Result<PackageThermalMetadata, String> {
    let Some(rja_c_per_w) =
        positive_number(Some(package.thermal_resistance_junction_to_ambient_c_per_w))
    else {
        return Err(format!(
            "THERMAL_PACKAGE_TEMPERATURE_VALID component {component_id} board.manufacturing.thermal_packages thermal_resistance_junction_to_ambient_C_per_W must be finite and positive."
        ));
    };
    let Some(max_junction_temperature_c) =
        positive_number(Some(package.max_junction_temperature_c))
    else {
        return Err(format!(
            "THERMAL_PACKAGE_TEMPERATURE_VALID component {component_id} board.manufacturing.thermal_packages max_junction_temperature_C must be finite and positive."
        ));
    };
    if package.source.trim().is_empty() {
        return Err(format!(
            "THERMAL_PACKAGE_TEMPERATURE_VALID component {component_id} board.manufacturing.thermal_packages source must be non-empty."
        ));
    }
    Ok(PackageThermalMetadata {
        source: package.source.clone(),
        rja_c_per_w,
        max_junction_temperature_c,
    })
}

fn package_thermal_metadata_from_model(
    component_id: &str,
    model_id: &str,
    model: &ComponentModel,
) -> Result<PackageThermalMetadata, String> {
    let Some(package) = &model.thermal_package else {
        return Err(format!(
            "THERMAL_PACKAGE_TEMPERATURE_VALID component {component_id} model {model_id} must declare thermal_package metadata or board.manufacturing.thermal_packages evidence."
        ));
    };
    let Some(rja_c_per_w) =
        positive_number(Some(package.thermal_resistance_junction_to_ambient_c_per_w))
    else {
        return Err(format!(
            "THERMAL_PACKAGE_TEMPERATURE_VALID component {component_id} model {model_id} thermal_package.thermal_resistance_junction_to_ambient_C_per_W must be finite and positive."
        ));
    };
    let Some(max_junction_temperature_c) =
        positive_number(Some(package.max_junction_temperature_c))
    else {
        return Err(format!(
            "THERMAL_PACKAGE_TEMPERATURE_VALID component {component_id} model {model_id} thermal_package.max_junction_temperature_C must be finite and positive."
        ));
    };
    if package.source.trim().is_empty() {
        return Err(format!(
            "THERMAL_PACKAGE_TEMPERATURE_VALID component {component_id} model {model_id} thermal_package.source must be non-empty."
        ));
    }
    Ok(PackageThermalMetadata {
        source: package.source.clone(),
        rja_c_per_w,
        max_junction_temperature_c,
    })
}

pub(super) fn validate_rule_metadata(
    bound: &BoundBoard<'_>,
    rule: &ThermalCopperRule,
) -> Result<(), String> {
    if rule.name.trim().is_empty() {
        return Err(
            "THERMAL_COPPER_AREA_VALID thermal copper rule name must be non-empty.".to_string(),
        );
    }
    if !bound.project.board.components.contains_key(&rule.component) {
        return Err(format!(
            "THERMAL_COPPER_AREA_VALID thermal copper rule {} component {} is absent from board.components.",
            rule.name, rule.component
        ));
    }
    if rule.source.trim().is_empty() {
        return Err(format!(
            "THERMAL_COPPER_AREA_VALID thermal copper rule {} source must be non-empty.",
            rule.name
        ));
    }
    if !rule.power_loss_w.is_finite() || rule.power_loss_w <= 0.0 {
        return Err(format!(
            "THERMAL_COPPER_AREA_VALID thermal copper rule {} power_loss_w must be finite and positive.",
            rule.name
        ));
    }
    if !rule.min_copper_area_mm2.is_finite() || rule.min_copper_area_mm2 <= 0.0 {
        return Err(format!(
            "THERMAL_COPPER_AREA_VALID thermal copper rule {} min_copper_area_mm2 must be finite and positive.",
            rule.name
        ));
    }
    for net in &rule.nets {
        if !bound.project.board.nets.contains_key(net) {
            return Err(format!(
                "THERMAL_COPPER_AREA_VALID thermal copper rule {} net {net} is absent from board.nets.",
                rule.name
            ));
        }
    }
    if let Some(rated_ambient_temperature_c) = rule.rated_ambient_temperature_c
        && !rated_ambient_temperature_c.is_finite()
    {
        return Err(format!(
            "THERMAL_COPPER_AREA_VALID thermal copper rule {} rated_ambient_temperature_C must be finite when supplied.",
            rule.name
        ));
    }
    if let Some(min_airflow_lfm) = rule.min_airflow_lfm
        && (!min_airflow_lfm.is_finite() || min_airflow_lfm < 0.0)
    {
        return Err(format!(
            "THERMAL_COPPER_AREA_VALID thermal copper rule {} min_airflow_lfm must be finite and non-negative when supplied.",
            rule.name
        ));
    }
    if let Some(min_thermal_via_plating_thickness_um) = rule.min_thermal_via_plating_thickness_um
        && (!min_thermal_via_plating_thickness_um.is_finite()
            || min_thermal_via_plating_thickness_um <= 0.0)
    {
        return Err(format!(
            "THERMAL_COPPER_AREA_VALID thermal copper rule {} min_thermal_via_plating_thickness_um must be finite and positive when supplied.",
            rule.name
        ));
    }
    if rule
        .enclosure_profile
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(format!(
            "THERMAL_COPPER_AREA_VALID thermal copper rule {} enclosure_profile must be non-empty when supplied.",
            rule.name
        ));
    }
    Ok(())
}

fn has_derating_metadata(rule: &ThermalCopperRule) -> bool {
    rule.rated_ambient_temperature_c.is_some()
        || rule.min_airflow_lfm.is_some()
        || rule.enclosure_profile.is_some()
}

pub(super) fn positive_number(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value > 0.0)
}

fn required_finite_number(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    check_id: &str,
    key: &str,
) -> Option<f64> {
    let Some(value) = scenario.parameters.get(key) else {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} requires parameters.{key}."),
        );
        return None;
    };
    let Some(number) = value.as_f64().filter(|number| number.is_finite()) else {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} parameters.{key} must be a finite number."),
        );
        return None;
    };
    Some(number)
}

fn required_positive_number(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    check_id: &str,
    key: &str,
) -> Option<f64> {
    let number = required_finite_number(scenario, findings, check_id, key)?;
    if number <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} parameters.{key} must be positive."),
        );
        return None;
    }
    Some(number)
}

fn required_nonnegative_number(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    check_id: &str,
    key: &str,
) -> Option<f64> {
    let number = required_finite_number(scenario, findings, check_id, key)?;
    if number < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} parameters.{key} must be non-negative."),
        );
        return None;
    }
    Some(number)
}

fn required_string_parameter(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    check_id: &str,
    key: &str,
) -> Option<String> {
    let Some(value) = scenario.parameters.get(key) else {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} requires parameters.{key}."),
        );
        return None;
    };
    let Some(text) = value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} parameters.{key} must be a non-empty string."),
        );
        return None;
    };
    Some(text.to_string())
}

fn optional_nonnegative_number(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    check_id: &str,
    key: &str,
) -> Option<f64> {
    let Some(value) = scenario.parameters.get(key) else {
        return Some(0.0);
    };
    let Some(number) = value.as_f64().filter(|number| number.is_finite()) else {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} parameters.{key} must be a finite number when supplied."),
        );
        return None;
    };
    if number < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} parameters.{key} must be non-negative."),
        );
        return None;
    }
    Some(number)
}

fn stackup_layer<'a>(layers: &'a [StackupLayer], name: &str) -> Option<&'a StackupLayer> {
    layers.iter().find(|layer| layer.name == name)
}

fn thermal_via_evidence(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    rule: &ThermalCopperRule,
) -> Option<ThermalViaEvidence> {
    let mut evidence = ThermalViaEvidence::default();
    for net in &rule.nets {
        let Some(route) = bound.project.board.layout.routes.get(net) else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "THERMAL_VIA_STACKUP_VALID thermal copper rule {} net {net} has no board.layout.routes evidence.",
                    rule.name
                ),
            );
            return None;
        };
        evidence.route_via_count += route.vias.len();
        for (via_index, via) in route.vias.iter().enumerate() {
            if !valid_route_via(via) {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "THERMAL_VIA_STACKUP_VALID net {net} via {via_index} must have finite coordinates, positive size/drill, and at least two explicit layers."
                    ),
                );
                return None;
            }
            if thermal_via_spans_layers(via, &rule.layers) {
                evidence.thermal_via_count += 1;
            }
        }
    }
    Some(evidence)
}

pub(super) fn valid_route_via(via: &RouteVia) -> bool {
    via.at.x_mm.is_finite()
        && via.at.y_mm.is_finite()
        && via.size_mm.is_finite()
        && via.size_mm > 0.0
        && via.drill_mm.is_finite()
        && via.drill_mm > 0.0
        && via.layers.len() >= 2
        && via.layers.iter().all(|layer| !layer.trim().is_empty())
}

pub(super) fn thermal_via_spans_layers(via: &RouteVia, layers: &[String]) -> bool {
    layers
        .iter()
        .all(|layer| via.layers.iter().any(|candidate| candidate == layer))
}

fn thermal_feature_matches(rule: &ThermalCopperRule, feature: &LayoutCopperFeature) -> bool {
    copper_matches(
        rule,
        feature.component.as_deref(),
        feature.net.as_deref(),
        &feature.layer,
    )
}

fn thermal_segment_matches(rule: &ThermalCopperRule, segment: &LayoutCopperSegment) -> bool {
    copper_matches(
        rule,
        segment.component.as_deref(),
        segment.net.as_deref(),
        &segment.layer,
    )
}

fn thermal_region_matches(rule: &ThermalCopperRule, region: &LayoutCopperRegion) -> bool {
    copper_matches(
        rule,
        region.component.as_deref(),
        region.net.as_deref(),
        &region.layer,
    )
}

fn copper_matches(
    rule: &ThermalCopperRule,
    component: Option<&str>,
    net: Option<&str>,
    layer: &str,
) -> bool {
    if !rule.layers.is_empty() && !rule.layers.iter().any(|candidate| candidate == layer) {
        return false;
    }
    let component_match = component == Some(rule.component.as_str());
    let net_match = net.is_some_and(|candidate| rule.nets.iter().any(|net| net == candidate));
    component_match || (!rule.nets.is_empty() && net_match)
}

fn feature_area_mm2(feature: &LayoutCopperFeature) -> Option<f64> {
    let x = feature.size.x_mm;
    let y = feature.size.y_mm;
    if !x.is_finite() || !y.is_finite() || x <= 0.0 || y <= 0.0 {
        return None;
    }
    match feature.shape.as_str() {
        "rect" | "rectangle" => Some(x * y),
        "circle" => Some(std::f64::consts::PI * (x.min(y) / 2.0).powi(2)),
        "oval" | "roundrect" => Some(oval_area_mm2(x, y)),
        _ => None,
    }
}

fn oval_area_mm2(x_mm: f64, y_mm: f64) -> f64 {
    let major = x_mm.max(y_mm);
    let minor = x_mm.min(y_mm);
    (major - minor) * minor + std::f64::consts::PI * (minor / 2.0).powi(2)
}

fn segment_area_mm2(segment: &LayoutCopperSegment) -> f64 {
    let length_mm =
        (segment.end.x_mm - segment.start.x_mm).hypot(segment.end.y_mm - segment.start.y_mm);
    length_mm * segment.width_mm
}

fn region_area_mm2(region: &LayoutCopperRegion) -> f64 {
    region
        .points
        .iter()
        .zip(region.points.iter().cycle().skip(1))
        .take(region.points.len())
        .map(|(left, right)| left.x_mm * right.y_mm - right.x_mm * left.y_mm)
        .sum::<f64>()
        .abs()
        / 2.0
}

#[derive(Debug, Default)]
struct ThermalAreaEvidence {
    feature_area_mm2: f64,
    segment_area_mm2: f64,
    region_area_mm2: f64,
    feature_count: usize,
    segment_count: usize,
    region_count: usize,
}

#[derive(Debug, Default)]
struct ThermalViaEvidence {
    route_via_count: usize,
    thermal_via_count: usize,
}

#[derive(Debug)]
struct PackageThermalEvidence<'a> {
    model_id: &'a str,
    package_source: &'a str,
    rja_c_per_w: f64,
    max_junction_temperature_c: f64,
    ambient_temperature_c: f64,
    max_temperature_rise_c: f64,
    max_junction_temperature_margin_c: f64,
    estimated_temperature_rise_c: f64,
    estimated_junction_temperature_c: f64,
    allowed_junction_temperature_c: f64,
}

struct PackageThermalMetadata {
    source: String,
    rja_c_per_w: f64,
    max_junction_temperature_c: f64,
}

impl ThermalAreaEvidence {
    fn total_area_mm2(&self) -> f64 {
        self.feature_area_mm2 + self.segment_area_mm2 + self.region_area_mm2
    }

    fn object_count(&self) -> usize {
        self.feature_count + self.segment_count + self.region_count
    }
}

fn thermal_area_finding(
    scenario: &Scenario,
    rule: &ThermalCopperRule,
    evidence: &ThermalAreaEvidence,
    total_area_mm2: f64,
) -> Finding {
    let mut finding = Finding::critical(
        THERMAL_COPPER_AREA_VALID,
        &scenario.name,
        format!(
            "Thermal copper rule {} measured {:.3} mm^2 of explicit copper evidence, below the reviewed {:.3} mm^2 minimum.",
            rule.name, total_area_mm2, rule.min_copper_area_mm2
        ),
    );
    finding.component = Some(rule.component.clone());
    finding
        .measured
        .insert("thermal_copper_name".to_string(), json!(rule.name));
    finding
        .measured
        .insert("thermal_copper_source".to_string(), json!(rule.source));
    finding
        .measured
        .insert("component".to_string(), json!(rule.component));
    finding
        .measured
        .insert("power_loss_w".to_string(), json!(rule.power_loss_w));
    finding
        .measured
        .insert("nets".to_string(), json!(rule.nets));
    finding
        .measured
        .insert("layers".to_string(), json!(rule.layers));
    finding
        .measured
        .insert("copper_area_mm2".to_string(), json!(total_area_mm2));
    finding.measured.insert(
        "copper_feature_area_mm2".to_string(),
        json!(evidence.feature_area_mm2),
    );
    finding.measured.insert(
        "copper_segment_area_mm2".to_string(),
        json!(evidence.segment_area_mm2),
    );
    finding.measured.insert(
        "copper_region_area_mm2".to_string(),
        json!(evidence.region_area_mm2),
    );
    finding.measured.insert(
        "copper_object_count".to_string(),
        json!(evidence.object_count()),
    );
    finding.limit.insert(
        "min_copper_area_mm2".to_string(),
        json!(rule.min_copper_area_mm2),
    );
    finding.suggested_fixes = vec![
        "Increase explicit copper area tied to the component or reviewed thermal nets/layers, then re-import the layout evidence.".to_string(),
        "If the loss or copper-area requirement changed, update board.manufacturing.thermal_copper from the reviewed thermal note instead of relying on this screen as a thermal solver.".to_string(),
    ];
    finding
}

fn thermal_copper_thickness_finding(
    scenario: &Scenario,
    rule: &ThermalCopperRule,
    layer: &StackupLayer,
    copper_thickness_um: f64,
    min_copper_thickness_um: f64,
) -> Finding {
    let mut finding = Finding::critical(
        THERMAL_VIA_STACKUP_VALID,
        &scenario.name,
        format!(
            "Thermal copper rule {} stackup layer {} has {:.3} um copper thickness, below the reviewed {:.3} um minimum.",
            rule.name, layer.name, copper_thickness_um, min_copper_thickness_um
        ),
    );
    finding.component = Some(rule.component.clone());
    finding
        .measured
        .insert("thermal_copper_name".to_string(), json!(rule.name));
    finding
        .measured
        .insert("thermal_copper_source".to_string(), json!(rule.source));
    finding
        .measured
        .insert("component".to_string(), json!(rule.component));
    finding
        .measured
        .insert("power_loss_w".to_string(), json!(rule.power_loss_w));
    finding
        .measured
        .insert("stackup_layer".to_string(), json!(layer.name));
    finding.measured.insert(
        "stackup_layer_kind".to_string(),
        json!(format!("{:?}", layer.kind)),
    );
    finding
        .measured
        .insert("stackup_layer_source".to_string(), json!(layer.source));
    finding.measured.insert(
        "layer_copper_thickness_um".to_string(),
        json!(copper_thickness_um),
    );
    finding.limit.insert(
        "min_copper_thickness_um".to_string(),
        json!(min_copper_thickness_um),
    );
    finding.suggested_fixes = vec![
        "Use a stackup layer copper thickness that satisfies the reviewed thermal policy, or update the reviewed policy from the actual fabrication stackup.".to_string(),
        "If thermal relief depends on plating, vias, or external heatsinking, encode that evidence explicitly before using this screen for sign-off.".to_string(),
    ];
    finding
}

fn thermal_via_count_finding(
    scenario: &Scenario,
    rule: &ThermalCopperRule,
    evidence: &ThermalViaEvidence,
    min_thermal_via_count: usize,
    min_copper_thickness_um: f64,
    observed_min_copper_thickness_um: f64,
) -> Finding {
    let mut finding = Finding::critical(
        THERMAL_VIA_STACKUP_VALID,
        &scenario.name,
        format!(
            "Thermal copper rule {} found {} explicit thermal via(s), below the reviewed minimum of {}.",
            rule.name, evidence.thermal_via_count, min_thermal_via_count
        ),
    );
    finding.component = Some(rule.component.clone());
    finding
        .measured
        .insert("thermal_copper_name".to_string(), json!(rule.name));
    finding
        .measured
        .insert("thermal_copper_source".to_string(), json!(rule.source));
    finding
        .measured
        .insert("component".to_string(), json!(rule.component));
    finding
        .measured
        .insert("power_loss_w".to_string(), json!(rule.power_loss_w));
    finding
        .measured
        .insert("nets".to_string(), json!(rule.nets));
    finding
        .measured
        .insert("layers".to_string(), json!(rule.layers));
    finding.measured.insert(
        "route_via_count".to_string(),
        json!(evidence.route_via_count),
    );
    finding.measured.insert(
        "thermal_via_count".to_string(),
        json!(evidence.thermal_via_count),
    );
    finding.measured.insert(
        "observed_min_copper_thickness_um".to_string(),
        json!(observed_min_copper_thickness_um),
    );
    finding.limit.insert(
        "min_thermal_via_count".to_string(),
        json!(min_thermal_via_count),
    );
    finding.limit.insert(
        "min_copper_thickness_um".to_string(),
        json!(min_copper_thickness_um),
    );
    finding.suggested_fixes = vec![
        "Add explicit route via evidence on the reviewed thermal net spanning the reviewed thermal layers, then re-import the layout.".to_string(),
        "If the via count requirement changed, update board.manufacturing.thermal_copper from the reviewed thermal layout policy.".to_string(),
    ];
    finding
}

fn thermal_temperature_rise_finding(
    scenario: &Scenario,
    rule: &ThermalCopperRule,
    evidence: &PackageThermalEvidence<'_>,
) -> Finding {
    let mut finding = Finding::critical(
        THERMAL_PACKAGE_TEMPERATURE_VALID,
        &scenario.name,
        format!(
            "Thermal copper rule {} estimates {:.3} C package temperature rise, above the reviewed {:.3} C limit.",
            rule.name, evidence.estimated_temperature_rise_c, evidence.max_temperature_rise_c
        ),
    );
    populate_package_thermal_finding(&mut finding, rule, evidence);
    finding.limit.insert(
        "max_temperature_rise_C".to_string(),
        json!(evidence.max_temperature_rise_c),
    );
    finding.suggested_fixes = vec![
        "Reduce reviewed package power loss, improve the component thermal path, or select a package/model with lower junction-to-ambient thermal resistance.".to_string(),
        "If the board accepts a higher temperature rise, update parameters.max_temperature_rise_C from reviewed thermal requirements or measured evidence.".to_string(),
    ];
    finding
}

fn thermal_junction_temperature_finding(
    scenario: &Scenario,
    rule: &ThermalCopperRule,
    evidence: &PackageThermalEvidence<'_>,
) -> Finding {
    let mut finding = Finding::critical(
        THERMAL_PACKAGE_TEMPERATURE_VALID,
        &scenario.name,
        format!(
            "Thermal copper rule {} estimates {:.3} C junction temperature, above the allowed {:.3} C limit.",
            rule.name,
            evidence.estimated_junction_temperature_c,
            evidence.allowed_junction_temperature_c
        ),
    );
    populate_package_thermal_finding(&mut finding, rule, evidence);
    finding.limit.insert(
        "allowed_junction_temperature_C".to_string(),
        json!(evidence.allowed_junction_temperature_c),
    );
    finding.suggested_fixes = vec![
        "Reduce reviewed package power loss, lower ambient temperature, improve heat spreading, or select a package/model with a higher reviewed thermal margin.".to_string(),
        "If the junction limit or margin changed, update the source-backed model thermal_package metadata or parameters.max_junction_temperature_margin_C.".to_string(),
    ];
    finding
}

fn thermal_derating_ambient_finding(
    scenario: &Scenario,
    rule: &ThermalCopperRule,
    ambient_temperature_c: f64,
    rated_ambient_temperature_c: f64,
) -> Finding {
    let mut finding = Finding::critical(
        THERMAL_DERATING_ENVIRONMENT_VALID,
        &scenario.name,
        format!(
            "Thermal copper rule {} uses reviewed ambient rating {:.3} C, but scenario ambient is {:.3} C.",
            rule.name, rated_ambient_temperature_c, ambient_temperature_c
        ),
    );
    populate_thermal_derating_finding(&mut finding, rule);
    finding.measured.insert(
        "ambient_temperature_C".to_string(),
        json!(ambient_temperature_c),
    );
    finding.limit.insert(
        "rated_ambient_temperature_C".to_string(),
        json!(rated_ambient_temperature_c),
    );
    finding.suggested_fixes = vec![
        "Reduce the reviewed operating ambient, improve cooling, or update the thermal rule from a reviewed derating source that covers this ambient.".to_string(),
        "Do not treat this check as a thermal solver; encode airflow, enclosure, copper, and measured evidence explicitly before sign-off.".to_string(),
    ];
    finding
}

fn thermal_derating_airflow_finding(
    scenario: &Scenario,
    rule: &ThermalCopperRule,
    airflow_lfm: f64,
    min_airflow_lfm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        THERMAL_DERATING_ENVIRONMENT_VALID,
        &scenario.name,
        format!(
            "Thermal copper rule {} requires reviewed airflow {:.3} LFM, but scenario airflow is {:.3} LFM.",
            rule.name, min_airflow_lfm, airflow_lfm
        ),
    );
    populate_thermal_derating_finding(&mut finding, rule);
    finding
        .measured
        .insert("airflow_lfm".to_string(), json!(airflow_lfm));
    finding
        .limit
        .insert("min_airflow_lfm".to_string(), json!(min_airflow_lfm));
    finding.suggested_fixes = vec![
        "Increase reviewed airflow, reduce component loss, or update board thermal requirements from reviewed airflow/enclosure evidence.".to_string(),
        "Use measured temperature or a qualified thermal model for final thermal sign-off when airflow assumptions are not guaranteed.".to_string(),
    ];
    finding
}

fn thermal_derating_enclosure_finding(
    scenario: &Scenario,
    rule: &ThermalCopperRule,
    enclosure_profile: &str,
    expected_enclosure_profile: &str,
) -> Finding {
    let mut finding = Finding::critical(
        THERMAL_DERATING_ENVIRONMENT_VALID,
        &scenario.name,
        format!(
            "Thermal copper rule {} requires enclosure profile {}, but scenario profile is {}.",
            rule.name, expected_enclosure_profile, enclosure_profile
        ),
    );
    populate_thermal_derating_finding(&mut finding, rule);
    finding
        .measured
        .insert("enclosure_profile".to_string(), json!(enclosure_profile));
    finding.limit.insert(
        "required_enclosure_profile".to_string(),
        json!(expected_enclosure_profile),
    );
    finding.suggested_fixes = vec![
        "Use an enclosure profile covered by the reviewed thermal rule or update the thermal rule from reviewed enclosure evidence.".to_string(),
        "If enclosure effects dominate, add measured thermal evidence instead of relying on this metadata screen.".to_string(),
    ];
    finding
}

fn populate_thermal_derating_finding(finding: &mut Finding, rule: &ThermalCopperRule) {
    finding.component = Some(rule.component.clone());
    finding
        .measured
        .insert("thermal_copper_name".to_string(), json!(rule.name));
    finding
        .measured
        .insert("thermal_copper_source".to_string(), json!(rule.source));
    finding
        .measured
        .insert("component".to_string(), json!(rule.component));
    finding
        .measured
        .insert("power_loss_w".to_string(), json!(rule.power_loss_w));
}

fn populate_package_thermal_finding(
    finding: &mut Finding,
    rule: &ThermalCopperRule,
    evidence: &PackageThermalEvidence<'_>,
) {
    finding.component = Some(rule.component.clone());
    finding
        .measured
        .insert("thermal_copper_name".to_string(), json!(rule.name));
    finding
        .measured
        .insert("thermal_copper_source".to_string(), json!(rule.source));
    finding
        .measured
        .insert("component".to_string(), json!(rule.component));
    finding
        .measured
        .insert("model".to_string(), json!(evidence.model_id));
    finding.measured.insert(
        "thermal_package_source".to_string(),
        json!(evidence.package_source),
    );
    finding
        .measured
        .insert("power_loss_w".to_string(), json!(rule.power_loss_w));
    finding.measured.insert(
        "thermal_resistance_junction_to_ambient_C_per_W".to_string(),
        json!(evidence.rja_c_per_w),
    );
    finding.measured.insert(
        "ambient_temperature_C".to_string(),
        json!(evidence.ambient_temperature_c),
    );
    finding.measured.insert(
        "estimated_temperature_rise_C".to_string(),
        json!(evidence.estimated_temperature_rise_c),
    );
    finding.measured.insert(
        "estimated_junction_temperature_C".to_string(),
        json!(evidence.estimated_junction_temperature_c),
    );
    finding.limit.insert(
        "max_junction_temperature_C".to_string(),
        json!(evidence.max_junction_temperature_c),
    );
    finding.limit.insert(
        "max_junction_temperature_margin_C".to_string(),
        json!(evidence.max_junction_temperature_margin_c),
    );
}
