use super::{
    AppliedControlledImpedanceNet, AppliedControlledImpedancePair, AppliedField,
    AppliedLayoutPoint, AppliedRfAntennaFeedPath, AppliedRfAntennaKeepout, AppliedStackupLayer,
    AppliedThermalCopper, AppliedThermalMeasurement,
};
use anyhow::{Context, Result};
use serde::Serialize;
use serde_yaml_ng::Value;
use std::collections::BTreeMap;

pub(super) fn normalized_yaml_value(field: &AppliedField) -> Result<Value> {
    if let Some(target) = &field.controlled_impedance_net {
        return serde_yaml_ng::to_value(controlled_impedance_net_mapping(target)).with_context(
            || {
                format!(
                    "Failed to encode manufacturing metadata {}.",
                    field.field.board_key()
                )
            },
        );
    }
    if let Some(target) = &field.controlled_impedance_pair {
        return serde_yaml_ng::to_value(controlled_impedance_pair_mapping(target)).with_context(
            || {
                format!(
                    "Failed to encode manufacturing metadata {}.",
                    field.field.board_key()
                )
            },
        );
    }
    if let Some(rule) = &field.thermal_copper {
        return serde_yaml_ng::to_value(thermal_copper_mapping(rule)).with_context(|| {
            format!(
                "Failed to encode manufacturing metadata {}.",
                field.field.board_key()
            )
        });
    }
    if let Some(measurement) = &field.thermal_measurement {
        return serde_yaml_ng::to_value(thermal_measurement_mapping(measurement)).with_context(
            || {
                format!(
                    "Failed to encode manufacturing metadata {}.",
                    field.field.board_key()
                )
            },
        );
    }
    if let Some(layer) = &field.stackup_layer {
        return serde_yaml_ng::to_value(stackup_layer_mapping(layer)).with_context(|| {
            format!(
                "Failed to encode manufacturing metadata {}.",
                field.field.board_key()
            )
        });
    }
    if let Some(keepout) = &field.rf_antenna_keepout {
        return serde_yaml_ng::to_value(rf_antenna_keepout_mapping(keepout)).with_context(|| {
            format!(
                "Failed to encode manufacturing metadata {}.",
                field.field.board_key()
            )
        });
    }
    if let Some(feed_path) = &field.rf_antenna_feed_path {
        return serde_yaml_ng::to_value(rf_antenna_feed_path_mapping(feed_path)).with_context(
            || {
                format!(
                    "Failed to encode manufacturing metadata {}.",
                    field.field.board_key()
                )
            },
        );
    }
    if let Some(value) = field.numeric_value {
        return serde_yaml_ng::to_value(value).with_context(|| {
            format!(
                "Failed to encode manufacturing metadata {}.",
                field.field.board_key()
            )
        });
    }
    Ok(Value::String(
        field
            .string_value
            .as_ref()
            .context("source field must have a string value")?
            .clone(),
    ))
}

fn controlled_impedance_net_mapping(
    target: &AppliedControlledImpedanceNet,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("net".to_string(), Value::String(target.net.clone()));
    mapping.insert("source".to_string(), Value::String(target.source.clone()));
    mapping.insert(
        "target_impedance_ohm".to_string(),
        serde_yaml_ng::to_value(target.target_impedance_ohm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "expected_width_mm".to_string(),
        serde_yaml_ng::to_value(target.expected_width_mm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "max_width_error_mm".to_string(),
        serde_yaml_ng::to_value(target.max_width_error_mm).unwrap_or(Value::Null),
    );
    mapping
}

fn controlled_impedance_pair_mapping(
    target: &AppliedControlledImpedancePair,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert(
        "first_net".to_string(),
        Value::String(target.first_net.clone()),
    );
    mapping.insert(
        "second_net".to_string(),
        Value::String(target.second_net.clone()),
    );
    mapping.insert("source".to_string(), Value::String(target.source.clone()));
    mapping.insert(
        "target_differential_impedance_ohm".to_string(),
        serde_yaml_ng::to_value(target.target_differential_impedance_ohm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "expected_width_mm".to_string(),
        serde_yaml_ng::to_value(target.expected_width_mm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "expected_gap_mm".to_string(),
        serde_yaml_ng::to_value(target.expected_gap_mm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "max_width_error_mm".to_string(),
        serde_yaml_ng::to_value(target.max_width_error_mm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "max_gap_error_mm".to_string(),
        serde_yaml_ng::to_value(target.max_gap_error_mm).unwrap_or(Value::Null),
    );
    mapping
}

fn thermal_copper_mapping(rule: &AppliedThermalCopper) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(rule.name.clone()));
    mapping.insert(
        "component".to_string(),
        Value::String(rule.component.clone()),
    );
    mapping.insert("source".to_string(), Value::String(rule.source.clone()));
    mapping.insert(
        "power_loss_w".to_string(),
        serde_yaml_ng::to_value(rule.power_loss_w).unwrap_or(Value::Null),
    );
    mapping.insert(
        "min_copper_area_mm2".to_string(),
        serde_yaml_ng::to_value(rule.min_copper_area_mm2).unwrap_or(Value::Null),
    );
    insert_optional_number(
        &mut mapping,
        "min_thermal_via_count",
        rule.min_thermal_via_count,
    );
    insert_optional_number(
        &mut mapping,
        "min_plated_thermal_via_count",
        rule.min_plated_thermal_via_count,
    );
    insert_optional_number(
        &mut mapping,
        "min_thermal_via_drill_mm",
        rule.min_thermal_via_drill_mm,
    );
    insert_optional_number(
        &mut mapping,
        "min_thermal_via_plating_thickness_um",
        rule.min_thermal_via_plating_thickness_um,
    );
    insert_optional_number(
        &mut mapping,
        "min_total_thermal_via_barrel_cross_section_mm2",
        rule.min_total_thermal_via_barrel_cross_section_mm2,
    );
    insert_optional_number(
        &mut mapping,
        "min_copper_thickness_um",
        rule.min_copper_thickness_um,
    );
    insert_optional_number(
        &mut mapping,
        "rated_ambient_temperature_C",
        rule.rated_ambient_temperature_c,
    );
    insert_optional_number(&mut mapping, "min_airflow_lfm", rule.min_airflow_lfm);
    if let Some(value) = &rule.enclosure_profile {
        mapping.insert(
            "enclosure_profile".to_string(),
            Value::String(value.clone()),
        );
    }
    insert_string_sequence(&mut mapping, "nets", &rule.nets);
    insert_string_sequence(&mut mapping, "layers", &rule.layers);
    mapping
}

fn insert_optional_number<T: Serialize>(
    mapping: &mut BTreeMap<String, Value>,
    key: &str,
    value: Option<T>,
) {
    if let Some(value) = value {
        mapping.insert(
            key.to_string(),
            serde_yaml_ng::to_value(value).unwrap_or(Value::Null),
        );
    }
}

fn insert_string_sequence(mapping: &mut BTreeMap<String, Value>, key: &str, values: &[String]) {
    if !values.is_empty() {
        mapping.insert(
            key.to_string(),
            Value::Sequence(values.iter().cloned().map(Value::String).collect()),
        );
    }
}

fn thermal_measurement_mapping(measurement: &AppliedThermalMeasurement) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(measurement.name.clone()));
    mapping.insert(
        "component".to_string(),
        Value::String(measurement.component.clone()),
    );
    mapping.insert(
        "source".to_string(),
        Value::String(measurement.source.clone()),
    );
    mapping.insert(
        "measured_temperature_C".to_string(),
        serde_yaml_ng::to_value(measurement.measured_temperature_c).unwrap_or(Value::Null),
    );
    if let Some(value) = measurement.ambient_temperature_c {
        mapping.insert(
            "ambient_temperature_C".to_string(),
            serde_yaml_ng::to_value(value).unwrap_or(Value::Null),
        );
    }
    if let Some(value) = measurement.measurement_uncertainty_c {
        mapping.insert(
            "measurement_uncertainty_C".to_string(),
            serde_yaml_ng::to_value(value).unwrap_or(Value::Null),
        );
    }
    if let Some(value) = measurement.power_loss_w {
        mapping.insert(
            "power_loss_w".to_string(),
            serde_yaml_ng::to_value(value).unwrap_or(Value::Null),
        );
    }
    if let Some(value) = &measurement.measurement_point {
        mapping.insert(
            "measurement_point".to_string(),
            Value::String(value.clone()),
        );
    }
    if let Some(value) = &measurement.notes {
        mapping.insert("notes".to_string(), Value::String(value.clone()));
    }
    mapping
}

fn stackup_layer_mapping(layer: &AppliedStackupLayer) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(layer.name.clone()));
    mapping.insert("kind".to_string(), Value::String(layer.kind.clone()));
    if let Some(value) = &layer.reference_net {
        mapping.insert("reference_net".to_string(), Value::String(value.clone()));
    }
    insert_optional_number(&mut mapping, "thickness_mm", layer.thickness_mm);
    insert_optional_number(
        &mut mapping,
        "copper_thickness_um",
        layer.copper_thickness_um,
    );
    insert_optional_number(
        &mut mapping,
        "dielectric_constant",
        layer.dielectric_constant,
    );
    if let Some(value) = &layer.material {
        mapping.insert("material".to_string(), Value::String(value.clone()));
    }
    mapping.insert("source".to_string(), Value::String(layer.source.clone()));
    mapping
}

fn rf_antenna_keepout_mapping(keepout: &AppliedRfAntennaKeepout) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(keepout.name.clone()));
    if let Some(value) = &keepout.antenna_net {
        mapping.insert("antenna_net".to_string(), Value::String(value.clone()));
    }
    mapping.insert("layer".to_string(), Value::String(keepout.layer.clone()));
    mapping.insert(
        "polygon".to_string(),
        Value::Sequence(
            keepout
                .polygon
                .iter()
                .map(layout_point_value)
                .collect::<Vec<_>>(),
        ),
    );
    mapping.insert(
        "min_copper_clearance_mm".to_string(),
        serde_yaml_ng::to_value(keepout.min_copper_clearance_mm).unwrap_or(Value::Null),
    );
    mapping.insert("source".to_string(), Value::String(keepout.source.clone()));
    mapping
}

fn rf_antenna_feed_path_mapping(feed_path: &AppliedRfAntennaFeedPath) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(feed_path.name.clone()));
    mapping.insert(
        "antenna_net".to_string(),
        Value::String(feed_path.antenna_net.clone()),
    );
    mapping.insert(
        "feed_component".to_string(),
        Value::String(feed_path.feed_component.clone()),
    );
    mapping.insert(
        "feed_pin".to_string(),
        Value::String(feed_path.feed_pin.clone()),
    );
    insert_string_sequence(
        &mut mapping,
        "matching_components",
        &feed_path.matching_components,
    );
    mapping.insert(
        "max_feed_route_length_mm".to_string(),
        serde_yaml_ng::to_value(feed_path.max_feed_route_length_mm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "max_matching_component_distance_mm".to_string(),
        serde_yaml_ng::to_value(feed_path.max_matching_component_distance_mm)
            .unwrap_or(Value::Null),
    );
    mapping.insert(
        "source".to_string(),
        Value::String(feed_path.source.clone()),
    );
    mapping
}

fn layout_point_value(point: &AppliedLayoutPoint) -> Value {
    let mut mapping = BTreeMap::new();
    mapping.insert(
        "x_mm".to_string(),
        serde_yaml_ng::to_value(point.x_mm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "y_mm".to_string(),
        serde_yaml_ng::to_value(point.y_mm).unwrap_or(Value::Null),
    );
    serde_yaml_ng::to_value(mapping).unwrap_or(Value::Null)
}
