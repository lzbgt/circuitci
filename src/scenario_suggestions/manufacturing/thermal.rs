use super::manufacturing_suggestion;
use crate::board_ir::{
    LayoutCopperFeature, LayoutCopperRegion, LayoutCopperSegment, LayoutDrill, RouteVia,
    StackupLayerKind, ThermalCopperRule, ThermalMeasurement,
};
use crate::library::BoundBoard;
use crate::scenario_suggestions::{ScenarioSuggestion, sanitized_name};
use serde_json::json;
use std::collections::BTreeMap;

const THERMAL_COPPER_AREA_VALID: &str = "THERMAL_COPPER_AREA_VALID";
const THERMAL_VIA_STACKUP_VALID: &str = "THERMAL_VIA_STACKUP_VALID";
const THERMAL_VIA_PLATING_VALID: &str = "THERMAL_VIA_PLATING_VALID";
const THERMAL_VIA_BARREL_CROSS_SECTION_VALID: &str = "THERMAL_VIA_BARREL_CROSS_SECTION_VALID";
const THERMAL_PACKAGE_TEMPERATURE_VALID: &str = "THERMAL_PACKAGE_TEMPERATURE_VALID";
const THERMAL_MEASURED_TEMPERATURE_VALID: &str = "THERMAL_MEASURED_TEMPERATURE_VALID";
const THERMAL_DERATING_ENVIRONMENT_VALID: &str = "THERMAL_DERATING_ENVIRONMENT_VALID";

pub(super) fn thermal_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    suggestions.extend(thermal_copper_area_suggestions(bound, project_name));
    suggestions.extend(thermal_via_stackup_suggestions(bound, project_name));
    suggestions.extend(thermal_via_plating_suggestions(bound, project_name));
    suggestions.extend(thermal_via_barrel_cross_section_suggestions(
        bound,
        project_name,
    ));
    suggestions.extend(thermal_package_temperature_suggestions(bound, project_name));
    suggestions.extend(thermal_measured_temperature_suggestions(
        bound,
        project_name,
    ));
    suggestions.extend(thermal_derating_environment_suggestions(
        bound,
        project_name,
    ));
    suggestions
}

fn thermal_copper_area_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for rule in &bound.project.board.manufacturing.thermal_copper {
        if !thermal_copper_rule_has_evidence(bound, rule)
            || thermal_copper_rule_check_declared(bound, &rule.name)
        {
            continue;
        }
        suggestions.push(manufacturing_suggestion(
            &format!("thermal_copper_area_{}", sanitized_name(&rule.name)),
            true,
            &format!(
                "Thermal copper rule {} has reviewed power-loss/source metadata and imported copper area evidence for component {}.",
                rule.name, rule.component
            ),
            &format!(
                "{}_{}_thermal_copper_area",
                project_name,
                sanitized_name(&rule.name)
            ),
            THERMAL_COPPER_AREA_VALID,
            Some(BTreeMap::from([(
                "thermal_copper".to_string(),
                json!([{ "name": rule.name }]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

fn thermal_copper_rule_has_evidence(bound: &BoundBoard<'_>, rule: &ThermalCopperRule) -> bool {
    !rule.name.trim().is_empty()
        && !rule.component.trim().is_empty()
        && !rule.source.trim().is_empty()
        && rule.power_loss_w.is_finite()
        && rule.power_loss_w > 0.0
        && rule.min_copper_area_mm2.is_finite()
        && rule.min_copper_area_mm2 > 0.0
        && bound.project.board.components.contains_key(&rule.component)
        && rule
            .nets
            .iter()
            .all(|net| bound.project.board.nets.contains_key(net))
        && thermal_copper_area_mm2(bound, rule) > f64::EPSILON
}

fn thermal_copper_area_mm2(bound: &BoundBoard<'_>, rule: &ThermalCopperRule) -> f64 {
    let copper = &bound.project.board.layout.copper;
    copper
        .features
        .iter()
        .filter(|feature| thermal_feature_matches(rule, feature))
        .filter_map(thermal_feature_area_mm2)
        .sum::<f64>()
        + copper
            .segments
            .iter()
            .filter(|segment| thermal_segment_matches(rule, segment))
            .map(thermal_segment_area_mm2)
            .sum::<f64>()
        + copper
            .regions
            .iter()
            .filter(|region| thermal_region_matches(rule, region))
            .map(thermal_region_area_mm2)
            .sum::<f64>()
}

fn thermal_feature_matches(rule: &ThermalCopperRule, feature: &LayoutCopperFeature) -> bool {
    thermal_copper_matches(
        rule,
        feature.component.as_deref(),
        feature.net.as_deref(),
        &feature.layer,
    )
}

fn thermal_segment_matches(rule: &ThermalCopperRule, segment: &LayoutCopperSegment) -> bool {
    thermal_copper_matches(
        rule,
        segment.component.as_deref(),
        segment.net.as_deref(),
        &segment.layer,
    )
}

fn thermal_region_matches(rule: &ThermalCopperRule, region: &LayoutCopperRegion) -> bool {
    thermal_copper_matches(
        rule,
        region.component.as_deref(),
        region.net.as_deref(),
        &region.layer,
    )
}

fn thermal_copper_matches(
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

fn thermal_feature_area_mm2(feature: &LayoutCopperFeature) -> Option<f64> {
    let x = feature.size.x_mm;
    let y = feature.size.y_mm;
    if !x.is_finite() || !y.is_finite() || x <= 0.0 || y <= 0.0 {
        return None;
    }
    match feature.shape.as_str() {
        "rect" | "rectangle" => Some(x * y),
        "circle" => Some(std::f64::consts::PI * (x.min(y) / 2.0).powi(2)),
        "oval" | "roundrect" => Some(thermal_oval_area_mm2(x, y)),
        _ => None,
    }
}

fn thermal_oval_area_mm2(x_mm: f64, y_mm: f64) -> f64 {
    let major = x_mm.max(y_mm);
    let minor = x_mm.min(y_mm);
    (major - minor) * minor + std::f64::consts::PI * (minor / 2.0).powi(2)
}

fn thermal_segment_area_mm2(segment: &LayoutCopperSegment) -> f64 {
    (segment.end.x_mm - segment.start.x_mm).hypot(segment.end.y_mm - segment.start.y_mm)
        * segment.width_mm
}

fn thermal_region_area_mm2(region: &LayoutCopperRegion) -> f64 {
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

fn thermal_copper_rule_check_declared(bound: &BoundBoard<'_>, rule_name: &str) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario
                .checks
                .iter()
                .any(|declared| declared == THERMAL_COPPER_AREA_VALID)
            && scenario
                .parameters
                .get("thermal_copper")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .is_some_and(|items| {
                    items.iter().any(|item| {
                        item.as_mapping().and_then(|mapping| {
                            mapping
                                .get(serde_yaml_ng::Value::String("name".to_string()))
                                .and_then(serde_yaml_ng::Value::as_str)
                        }) == Some(rule_name)
                    })
                })
    })
}

fn thermal_via_stackup_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for rule in &bound.project.board.manufacturing.thermal_copper {
        if !thermal_via_stackup_rule_has_evidence(bound, rule)
            || thermal_via_stackup_check_declared(bound, &rule.name)
        {
            continue;
        }
        suggestions.push(manufacturing_suggestion(
            &format!("thermal_via_stackup_{}", sanitized_name(&rule.name)),
            true,
            &format!(
                "Thermal copper rule {} has reviewed via-count/copper-thickness policy plus imported stackup and route-via evidence.",
                rule.name
            ),
            &format!(
                "{}_{}_thermal_via_stackup",
                project_name,
                sanitized_name(&rule.name)
            ),
            THERMAL_VIA_STACKUP_VALID,
            Some(BTreeMap::from([(
                "thermal_copper".to_string(),
                json!([{ "name": rule.name }]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

fn thermal_via_stackup_rule_has_evidence(bound: &BoundBoard<'_>, rule: &ThermalCopperRule) -> bool {
    thermal_copper_rule_has_evidence(bound, rule)
        && rule.min_thermal_via_count.is_some_and(|count| count > 0)
        && rule
            .min_copper_thickness_um
            .is_some_and(|value| value.is_finite() && value > 0.0)
        && !rule.nets.is_empty()
        && rule.layers.len() >= 2
        && rule.layers.iter().all(|layer_name| {
            bound
                .project
                .board
                .layout
                .stackup
                .layers
                .iter()
                .any(|layer| {
                    layer.name == *layer_name
                        && matches!(
                            layer.kind,
                            StackupLayerKind::Signal | StackupLayerKind::Plane
                        )
                        && layer
                            .copper_thickness_um
                            .is_some_and(|value| value.is_finite() && value > 0.0)
                        && layer
                            .source
                            .as_deref()
                            .map(str::trim)
                            .is_some_and(|value| !value.is_empty())
                })
        })
        && rule.nets.iter().all(|net| {
            bound
                .project
                .board
                .layout
                .routes
                .get(net)
                .is_some_and(|route| {
                    !route.vias.is_empty() && route.vias.iter().all(valid_thermal_route_via)
                })
        })
}

fn valid_thermal_route_via(via: &RouteVia) -> bool {
    via.at.x_mm.is_finite()
        && via.at.y_mm.is_finite()
        && via.size_mm.is_finite()
        && via.size_mm > 0.0
        && via.drill_mm.is_finite()
        && via.drill_mm > 0.0
        && via.layers.len() >= 2
        && via.layers.iter().all(|layer| !layer.trim().is_empty())
}

fn thermal_via_stackup_check_declared(bound: &BoundBoard<'_>, rule_name: &str) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario
                .checks
                .iter()
                .any(|declared| declared == THERMAL_VIA_STACKUP_VALID)
            && scenario
                .parameters
                .get("thermal_copper")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .is_some_and(|items| {
                    items.iter().any(|item| {
                        item.as_mapping().and_then(|mapping| {
                            mapping
                                .get(serde_yaml_ng::Value::String("name".to_string()))
                                .and_then(serde_yaml_ng::Value::as_str)
                        }) == Some(rule_name)
                    })
                })
    })
}

fn thermal_via_plating_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for rule in &bound.project.board.manufacturing.thermal_copper {
        if !thermal_via_plating_rule_has_evidence(bound, rule)
            || thermal_via_plating_check_declared(bound, &rule.name)
        {
            continue;
        }
        suggestions.push(manufacturing_suggestion(
            &format!("thermal_via_plating_{}", sanitized_name(&rule.name)),
            true,
            &format!(
                "Thermal copper rule {} has reviewed plated-via/drill policy plus imported route-via and drill plating evidence.",
                rule.name
            ),
            &format!(
                "{}_{}_thermal_via_plating",
                project_name,
                sanitized_name(&rule.name)
            ),
            THERMAL_VIA_PLATING_VALID,
            Some(BTreeMap::from([(
                "thermal_copper".to_string(),
                json!([{ "name": rule.name }]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

fn thermal_via_plating_rule_has_evidence(bound: &BoundBoard<'_>, rule: &ThermalCopperRule) -> bool {
    thermal_copper_rule_has_evidence(bound, rule)
        && rule
            .min_plated_thermal_via_count
            .is_some_and(|count| count > 0)
        && rule
            .min_thermal_via_drill_mm
            .is_some_and(|value| value.is_finite() && value > 0.0)
        && rule
            .min_thermal_via_plating_thickness_um
            .is_none_or(|value| value.is_finite() && value > 0.0)
        && !rule.nets.is_empty()
        && rule.layers.len() >= 2
        && rule.nets.iter().all(|net| {
            bound
                .project
                .board
                .layout
                .routes
                .get(net)
                .is_some_and(|route| {
                    route.vias.iter().enumerate().any(|(index, via)| {
                        valid_thermal_route_via(via)
                            && rule.layers.iter().all(|layer| via.layers.contains(layer))
                            && matching_thermal_via_drill(bound, net, index, via).is_some_and(
                                |drill| thermal_plating_drill_has_required_evidence(rule, drill),
                            )
                    })
                })
        })
}

fn thermal_plating_drill_has_required_evidence(
    rule: &ThermalCopperRule,
    drill: &LayoutDrill,
) -> bool {
    valid_thermal_drill(drill)
        && rule.min_thermal_via_plating_thickness_um.is_none_or(|_| {
            drill.plating == "plated"
                && drill
                    .plating_thickness_um
                    .is_some_and(|value| value.is_finite() && value > 0.0)
        })
}

fn matching_thermal_via_drill<'a>(
    bound: &'a BoundBoard<'_>,
    net: &str,
    via_index: usize,
    via: &RouteVia,
) -> Option<&'a LayoutDrill> {
    bound
        .project
        .board
        .layout
        .drills
        .iter()
        .find(|drill| {
            drill.via_index == Some(via_index)
                && drill.net.as_deref().is_none_or(|value| value == net)
        })
        .or_else(|| {
            bound.project.board.layout.drills.iter().find(|drill| {
                drill.net.as_deref().is_none_or(|value| value == net)
                    && (drill.at.x_mm - via.at.x_mm).abs() <= 1.0e-6
                    && (drill.at.y_mm - via.at.y_mm).abs() <= 1.0e-6
                    && (drill.drill_mm - via.drill_mm).abs() <= 1.0e-6
            })
        })
}

fn valid_thermal_drill(drill: &LayoutDrill) -> bool {
    drill.at.x_mm.is_finite()
        && drill.at.y_mm.is_finite()
        && drill.drill_mm.is_finite()
        && drill.drill_mm > 0.0
        && drill
            .plating_thickness_um
            .is_none_or(|value| value.is_finite() && value > 0.0)
        && matches!(drill.plating.as_str(), "plated" | "non_plated" | "unknown")
}

fn thermal_via_plating_check_declared(bound: &BoundBoard<'_>, rule_name: &str) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario
                .checks
                .iter()
                .any(|declared| declared == THERMAL_VIA_PLATING_VALID)
            && scenario
                .parameters
                .get("thermal_copper")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .is_some_and(|items| {
                    items.iter().any(|item| {
                        item.as_mapping().and_then(|mapping| {
                            mapping
                                .get(serde_yaml_ng::Value::String("name".to_string()))
                                .and_then(serde_yaml_ng::Value::as_str)
                        }) == Some(rule_name)
                    })
                })
    })
}

fn thermal_via_barrel_cross_section_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for rule in &bound.project.board.manufacturing.thermal_copper {
        if !thermal_via_barrel_cross_section_rule_has_evidence(bound, rule)
            || thermal_via_barrel_cross_section_check_declared(bound, &rule.name)
        {
            continue;
        }
        suggestions.push(manufacturing_suggestion(
            &format!(
                "thermal_via_barrel_cross_section_{}",
                sanitized_name(&rule.name)
            ),
            true,
            &format!(
                "Thermal copper rule {} has reviewed via-barrel cross-section policy plus imported plated drill diameter/thickness evidence.",
                rule.name
            ),
            &format!(
                "{}_{}_thermal_via_barrel_cross_section",
                project_name,
                sanitized_name(&rule.name)
            ),
            THERMAL_VIA_BARREL_CROSS_SECTION_VALID,
            Some(BTreeMap::from([(
                "thermal_copper".to_string(),
                json!([{ "name": rule.name }]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

fn thermal_via_barrel_cross_section_rule_has_evidence(
    bound: &BoundBoard<'_>,
    rule: &ThermalCopperRule,
) -> bool {
    thermal_copper_rule_has_evidence(bound, rule)
        && rule
            .min_total_thermal_via_barrel_cross_section_mm2
            .is_some_and(|value| value.is_finite() && value > 0.0)
        && !rule.nets.is_empty()
        && rule.layers.len() >= 2
        && rule.nets.iter().all(|net| {
            bound
                .project
                .board
                .layout
                .routes
                .get(net)
                .is_some_and(|route| {
                    route.vias.iter().enumerate().any(|(index, via)| {
                        valid_thermal_route_via(via)
                            && rule.layers.iter().all(|layer| via.layers.contains(layer))
                            && matching_thermal_via_drill(bound, net, index, via).is_some_and(
                                |drill| {
                                    valid_thermal_drill(drill)
                                        && drill.plating == "plated"
                                        && drill
                                            .plating_thickness_um
                                            .is_some_and(|value| value.is_finite() && value > 0.0)
                                },
                            )
                    })
                })
        })
}

fn thermal_via_barrel_cross_section_check_declared(
    bound: &BoundBoard<'_>,
    rule_name: &str,
) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario
                .checks
                .iter()
                .any(|declared| declared == THERMAL_VIA_BARREL_CROSS_SECTION_VALID)
            && scenario
                .parameters
                .get("thermal_copper")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .is_some_and(|items| {
                    items.iter().any(|item| {
                        item.as_mapping().and_then(|mapping| {
                            mapping
                                .get(serde_yaml_ng::Value::String("name".to_string()))
                                .and_then(serde_yaml_ng::Value::as_str)
                        }) == Some(rule_name)
                    })
                })
    })
}

fn thermal_package_temperature_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for rule in &bound.project.board.manufacturing.thermal_copper {
        if !thermal_package_rule_has_metadata(bound, rule)
            || thermal_package_temperature_check_declared(bound, &rule.name)
        {
            continue;
        }
        suggestions.push(manufacturing_suggestion(
            &format!("thermal_package_temperature_{}", sanitized_name(&rule.name)),
            false,
            &format!(
                "Thermal copper rule {} has reviewed package power-loss metadata and source-backed package thermal evidence; reviewed ambient and temperature-rise limits are still required.",
                rule.name
            ),
            &format!(
                "{}_{}_thermal_package_temperature",
                project_name,
                sanitized_name(&rule.name)
            ),
            THERMAL_PACKAGE_TEMPERATURE_VALID,
            Some(BTreeMap::from([(
                "thermal_copper".to_string(),
                json!([{ "name": rule.name }]),
            )])),
            vec![
                "Set parameters.ambient_temperature_C from the reviewed operating environment."
                    .to_string(),
                "Set parameters.max_temperature_rise_C from board or package thermal requirements."
                    .to_string(),
            ],
        ));
    }
    suggestions
}

fn thermal_package_rule_has_metadata(bound: &BoundBoard<'_>, rule: &ThermalCopperRule) -> bool {
    !rule.name.trim().is_empty()
        && !rule.component.trim().is_empty()
        && !rule.source.trim().is_empty()
        && rule.power_loss_w.is_finite()
        && rule.power_loss_w > 0.0
        && bound
            .project
            .board
            .components
            .get(&rule.component)
            .and_then(|component| bound.library.get(&component.model))
            .is_some_and(|model| {
                reviewed_package_metadata_valid(bound, &rule.component)
                    || model.thermal_package.as_ref().is_some_and(|package| {
                        package
                            .thermal_resistance_junction_to_ambient_c_per_w
                            .is_finite()
                            && package.thermal_resistance_junction_to_ambient_c_per_w > 0.0
                            && package.max_junction_temperature_c.is_finite()
                            && package.max_junction_temperature_c > 0.0
                            && !package.source.trim().is_empty()
                    })
            })
}

fn reviewed_package_metadata_valid(bound: &BoundBoard<'_>, component: &str) -> bool {
    let mut matches = bound
        .project
        .board
        .manufacturing
        .thermal_packages
        .iter()
        .filter(|package| package.component == component);
    let Some(package) = matches.next() else {
        return false;
    };
    if matches.next().is_some() {
        return false;
    }
    package
        .thermal_resistance_junction_to_ambient_c_per_w
        .is_finite()
        && package.thermal_resistance_junction_to_ambient_c_per_w > 0.0
        && package.max_junction_temperature_c.is_finite()
        && package.max_junction_temperature_c > 0.0
        && !package.source.trim().is_empty()
}

fn thermal_package_temperature_check_declared(bound: &BoundBoard<'_>, rule_name: &str) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario
                .checks
                .iter()
                .any(|declared| declared == THERMAL_PACKAGE_TEMPERATURE_VALID)
            && scenario
                .parameters
                .get("thermal_copper")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .is_some_and(|items| {
                    items.iter().any(|item| {
                        item.as_mapping().and_then(|mapping| {
                            mapping
                                .get(serde_yaml_ng::Value::String("name".to_string()))
                                .and_then(serde_yaml_ng::Value::as_str)
                        }) == Some(rule_name)
                    })
                })
    })
}

fn thermal_measured_temperature_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for measurement in &bound.project.board.manufacturing.thermal_measurements {
        if !thermal_measurement_has_evidence(bound, measurement)
            || thermal_measurement_check_declared(bound, &measurement.name)
        {
            continue;
        }
        let mut required_inputs = vec![
            "Set parameters.max_measured_temperature_C from the reviewed component or board thermal requirement.".to_string(),
        ];
        if measurement.ambient_temperature_c.is_some() {
            required_inputs.push(
                "Optionally set parameters.max_temperature_rise_C to screen measured rise over ambient."
                    .to_string(),
            );
        }
        if measurement.measurement_uncertainty_c.is_some() {
            required_inputs.push(
                "Optionally set parameters.include_measurement_uncertainty: true to screen worst-case measured temperature using reviewed uncertainty."
                    .to_string(),
            );
        }
        suggestions.push(manufacturing_suggestion(
            &format!(
                "thermal_measured_temperature_{}",
                sanitized_name(&measurement.name)
            ),
            false,
            &format!(
                "Thermal measurement {} has reviewed measured-temperature evidence for component {}; reviewed measured-temperature limits are still required.",
                measurement.name, measurement.component
            ),
            &format!(
                "{}_{}_thermal_measured_temperature",
                project_name,
                sanitized_name(&measurement.name)
            ),
            THERMAL_MEASURED_TEMPERATURE_VALID,
            Some(BTreeMap::from([(
                "thermal_measurements".to_string(),
                json!([{ "name": measurement.name }]),
            )])),
            required_inputs,
        ));
    }
    suggestions
}

fn thermal_measurement_has_evidence(
    bound: &BoundBoard<'_>,
    measurement: &ThermalMeasurement,
) -> bool {
    !measurement.name.trim().is_empty()
        && !measurement.component.trim().is_empty()
        && !measurement.source.trim().is_empty()
        && measurement.measured_temperature_c.is_finite()
        && bound
            .project
            .board
            .components
            .contains_key(&measurement.component)
        && measurement
            .ambient_temperature_c
            .is_none_or(|value| value.is_finite())
        && measurement
            .measurement_uncertainty_c
            .is_none_or(|value| value.is_finite() && value >= 0.0)
        && measurement
            .power_loss_w
            .is_none_or(|value| value.is_finite() && value > 0.0)
}

fn thermal_measurement_check_declared(bound: &BoundBoard<'_>, measurement_name: &str) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario
                .checks
                .iter()
                .any(|declared| declared == THERMAL_MEASURED_TEMPERATURE_VALID)
            && scenario
                .parameters
                .get("thermal_measurements")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .is_some_and(|items| {
                    items.iter().any(|item| {
                        item.as_mapping().and_then(|mapping| {
                            mapping
                                .get(serde_yaml_ng::Value::String("name".to_string()))
                                .and_then(serde_yaml_ng::Value::as_str)
                        }) == Some(measurement_name)
                    })
                })
    })
}

fn thermal_derating_environment_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for rule in &bound.project.board.manufacturing.thermal_copper {
        if !thermal_derating_rule_has_metadata(bound, rule)
            || thermal_derating_environment_check_declared(bound, &rule.name)
        {
            continue;
        }
        let mut required_inputs = Vec::new();
        if rule.rated_ambient_temperature_c.is_some() {
            required_inputs.push(
                "Set parameters.ambient_temperature_C from the reviewed operating environment."
                    .to_string(),
            );
        }
        if rule.min_airflow_lfm.is_some() {
            required_inputs.push(
                "Set parameters.airflow_lfm from reviewed airflow or fan characterization evidence."
                    .to_string(),
            );
        }
        if rule.enclosure_profile.is_some() {
            required_inputs.push(
                "Set parameters.enclosure_profile from reviewed enclosure or product configuration evidence."
                    .to_string(),
            );
        }
        suggestions.push(manufacturing_suggestion(
            &format!(
                "thermal_derating_environment_{}",
                sanitized_name(&rule.name)
            ),
            false,
            &format!(
                "Thermal copper rule {} has reviewed ambient/airflow/enclosure derating metadata; reviewed operating environment inputs are still required.",
                rule.name
            ),
            &format!(
                "{}_{}_thermal_derating_environment",
                project_name,
                sanitized_name(&rule.name)
            ),
            THERMAL_DERATING_ENVIRONMENT_VALID,
            Some(BTreeMap::from([(
                "thermal_copper".to_string(),
                json!([{ "name": rule.name }]),
            )])),
            required_inputs,
        ));
    }
    suggestions
}

fn thermal_derating_rule_has_metadata(bound: &BoundBoard<'_>, rule: &ThermalCopperRule) -> bool {
    !rule.name.trim().is_empty()
        && !rule.component.trim().is_empty()
        && !rule.source.trim().is_empty()
        && rule.power_loss_w.is_finite()
        && rule.power_loss_w > 0.0
        && rule.min_copper_area_mm2.is_finite()
        && rule.min_copper_area_mm2 > 0.0
        && bound.project.board.components.contains_key(&rule.component)
        && rule
            .rated_ambient_temperature_c
            .is_none_or(|value| value.is_finite())
        && rule
            .min_airflow_lfm
            .is_none_or(|value| value.is_finite() && value >= 0.0)
        && rule
            .enclosure_profile
            .as_deref()
            .is_none_or(|value| !value.trim().is_empty())
        && (rule.rated_ambient_temperature_c.is_some()
            || rule.min_airflow_lfm.is_some()
            || rule.enclosure_profile.is_some())
}

fn thermal_derating_environment_check_declared(bound: &BoundBoard<'_>, rule_name: &str) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario
                .checks
                .iter()
                .any(|declared| declared == THERMAL_DERATING_ENVIRONMENT_VALID)
            && scenario
                .parameters
                .get("thermal_copper")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .is_some_and(|items| {
                    items.iter().any(|item| {
                        item.as_mapping().and_then(|mapping| {
                            mapping
                                .get(serde_yaml_ng::Value::String("name".to_string()))
                                .and_then(serde_yaml_ng::Value::as_str)
                        }) == Some(rule_name)
                    })
                })
    })
}
