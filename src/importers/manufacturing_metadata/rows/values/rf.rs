use super::super::{
    AppliedRfAntennaFeedPath, AppliedRfAntennaKeepout, AppliedRfAntennaMatchingElement,
    AppliedRfAntennaMatchingNetwork, AppliedRfAntennaMeasurement,
    AppliedRfAntennaMeasurementCondition, AppliedRfAntennaPerformanceLimit,
};
use super::{insert_string_sequence, layout_point_value};
use serde_yaml_ng::Value;
use std::collections::BTreeMap;

pub(super) fn rf_antenna_keepout_mapping(
    keepout: &AppliedRfAntennaKeepout,
) -> BTreeMap<String, Value> {
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

pub(super) fn rf_antenna_feed_path_mapping(
    feed_path: &AppliedRfAntennaFeedPath,
) -> BTreeMap<String, Value> {
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

pub(super) fn rf_antenna_matching_network_mapping(
    network: &AppliedRfAntennaMatchingNetwork,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(network.name.clone()));
    mapping.insert(
        "antenna_net".to_string(),
        Value::String(network.antenna_net.clone()),
    );
    mapping.insert(
        "topology".to_string(),
        Value::String(network.topology.clone()),
    );
    if let Some(reference_net) = &network.reference_net {
        mapping.insert(
            "reference_net".to_string(),
            Value::String(reference_net.clone()),
        );
    }
    mapping.insert("source".to_string(), Value::String(network.source.clone()));
    mapping.insert(
        "elements".to_string(),
        Value::Sequence(
            network
                .elements
                .iter()
                .map(rf_antenna_matching_element_value)
                .collect::<Vec<_>>(),
        ),
    );
    mapping
}

fn rf_antenna_matching_element_value(element: &AppliedRfAntennaMatchingElement) -> Value {
    let mut mapping = BTreeMap::new();
    mapping.insert(
        "component".to_string(),
        Value::String(element.component.clone()),
    );
    mapping.insert("role".to_string(), Value::String(element.role.clone()));
    if let Some(value) = &element.input_net {
        mapping.insert("input_net".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &element.output_net {
        mapping.insert("output_net".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &element.signal_net {
        mapping.insert("signal_net".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &element.reference_net {
        mapping.insert("reference_net".to_string(), Value::String(value.clone()));
    }
    serde_yaml_ng::to_value(mapping).unwrap_or(Value::Null)
}

pub(super) fn rf_antenna_measurement_mapping(
    measurement: &AppliedRfAntennaMeasurement,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(measurement.name.clone()));
    mapping.insert(
        "antenna_net".to_string(),
        Value::String(measurement.antenna_net.clone()),
    );
    mapping.insert(
        "frequency_mhz".to_string(),
        serde_yaml_ng::to_value(measurement.frequency_mhz).unwrap_or(Value::Null),
    );
    mapping.insert(
        "return_loss_db".to_string(),
        serde_yaml_ng::to_value(measurement.return_loss_db).unwrap_or(Value::Null),
    );
    mapping.insert(
        "source".to_string(),
        Value::String(measurement.source.clone()),
    );
    if let Some(value) = &measurement.measurement_method {
        mapping.insert(
            "measurement_method".to_string(),
            Value::String(value.clone()),
        );
    }
    if let Some(value) = &measurement.measurement_condition {
        mapping.insert(
            "measurement_condition".to_string(),
            Value::String(value.clone()),
        );
    }
    if let Some(value) = &measurement.notes {
        mapping.insert("notes".to_string(), Value::String(value.clone()));
    }
    mapping
}

pub(super) fn rf_antenna_performance_limit_mapping(
    limit: &AppliedRfAntennaPerformanceLimit,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(limit.name.clone()));
    mapping.insert(
        "antenna_net".to_string(),
        Value::String(limit.antenna_net.clone()),
    );
    mapping.insert(
        "min_return_loss_db".to_string(),
        serde_yaml_ng::to_value(limit.min_return_loss_db).unwrap_or(Value::Null),
    );
    mapping.insert("source".to_string(), Value::String(limit.source.clone()));
    if let Some(value) = limit.frequency_min_mhz {
        mapping.insert(
            "frequency_min_mhz".to_string(),
            serde_yaml_ng::to_value(value).unwrap_or(Value::Null),
        );
    }
    if let Some(value) = limit.frequency_max_mhz {
        mapping.insert(
            "frequency_max_mhz".to_string(),
            serde_yaml_ng::to_value(value).unwrap_or(Value::Null),
        );
    }
    if let Some(value) = limit.min_measurement_count {
        mapping.insert(
            "min_measurement_count".to_string(),
            serde_yaml_ng::to_value(value).unwrap_or(Value::Null),
        );
    }
    if let Some(value) = limit.max_frequency_step_mhz {
        mapping.insert(
            "max_frequency_step_mhz".to_string(),
            serde_yaml_ng::to_value(value).unwrap_or(Value::Null),
        );
    }
    if let Some(value) = &limit.required_measurement_condition {
        mapping.insert(
            "required_measurement_condition".to_string(),
            Value::String(value.clone()),
        );
    }
    if let Some(value) = &limit.notes {
        mapping.insert("notes".to_string(), Value::String(value.clone()));
    }
    mapping
}

pub(super) fn rf_antenna_measurement_condition_mapping(
    condition: &AppliedRfAntennaMeasurementCondition,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(condition.name.clone()));
    mapping.insert(
        "source".to_string(),
        Value::String(condition.source.clone()),
    );
    if let Some(value) = &condition.fixture {
        mapping.insert("fixture".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &condition.cable_setup {
        mapping.insert("cable_setup".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &condition.enclosure_profile {
        mapping.insert(
            "enclosure_profile".to_string(),
            Value::String(value.clone()),
        );
    }
    if let Some(value) = &condition.notes {
        mapping.insert("notes".to_string(), Value::String(value.clone()));
    }
    mapping
}
