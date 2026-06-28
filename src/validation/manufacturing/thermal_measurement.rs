use crate::board_ir::{Scenario, ThermalMeasurement};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;

use super::super::THERMAL_MEASURED_TEMPERATURE_VALID;
use super::super::common::validation_input_missing;

pub(in crate::validation) fn validate_thermal_measured_temperature(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(names) = named_thermal_measurement_parameters(
        scenario,
        findings,
        THERMAL_MEASURED_TEMPERATURE_VALID,
        "thermal_measurements",
    ) else {
        return;
    };
    let Some(max_measured_temperature_c) = required_finite_number(
        scenario,
        findings,
        THERMAL_MEASURED_TEMPERATURE_VALID,
        "max_measured_temperature_C",
    ) else {
        return;
    };
    let Some(max_temperature_rise_c) = optional_positive_number(
        scenario,
        findings,
        THERMAL_MEASURED_TEMPERATURE_VALID,
        "max_temperature_rise_C",
    ) else {
        return;
    };
    for name in names {
        let Some(measurement) = thermal_measurement(
            bound,
            scenario,
            findings,
            THERMAL_MEASURED_TEMPERATURE_VALID,
            &name,
        ) else {
            return;
        };
        validate_measured_temperature_rule(
            bound,
            scenario,
            findings,
            measurement,
            max_measured_temperature_c,
            max_temperature_rise_c,
        );
    }
}

fn named_thermal_measurement_parameters(
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

fn thermal_measurement<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    check_id: &str,
    name: &str,
) -> Option<&'a ThermalMeasurement> {
    let matches = bound
        .project
        .board
        .manufacturing
        .thermal_measurements
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
                    "{check_id} thermal measurement {name} is absent from board.manufacturing.thermal_measurements."
                ),
            );
            None
        }
        _ => {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "{check_id} thermal measurement {name} is ambiguous in board.manufacturing.thermal_measurements."
                ),
            );
            None
        }
    }
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

fn optional_positive_number(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    check_id: &str,
    key: &str,
) -> Option<Option<f64>> {
    let Some(value) = scenario.parameters.get(key) else {
        return Some(None);
    };
    let Some(number) = value.as_f64().filter(|number| number.is_finite()) else {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} parameters.{key} must be a finite number when supplied."),
        );
        return None;
    };
    if number <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            format!("{check_id} parameters.{key} must be positive when supplied."),
        );
        return None;
    }
    Some(Some(number))
}

fn validate_measured_temperature_rule(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    measurement: &ThermalMeasurement,
    max_measured_temperature_c: f64,
    max_temperature_rise_c: Option<f64>,
) {
    if let Err(message) = validate_measurement_metadata(bound, measurement) {
        validation_input_missing(findings, scenario, message);
        return;
    }
    if measurement.measured_temperature_c > max_measured_temperature_c + f64::EPSILON {
        findings.push(thermal_measured_temperature_finding(
            scenario,
            measurement,
            max_measured_temperature_c,
        ));
    }
    if let Some(max_temperature_rise_c) = max_temperature_rise_c {
        let Some(ambient_temperature_c) = measurement.ambient_temperature_c else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "THERMAL_MEASURED_TEMPERATURE_VALID thermal measurement {} must declare ambient_temperature_C when parameters.max_temperature_rise_C is supplied.",
                    measurement.name
                ),
            );
            return;
        };
        if !ambient_temperature_c.is_finite() {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "THERMAL_MEASURED_TEMPERATURE_VALID thermal measurement {} ambient_temperature_C must be finite.",
                    measurement.name
                ),
            );
            return;
        }
        let measured_temperature_rise_c =
            measurement.measured_temperature_c - ambient_temperature_c;
        if measured_temperature_rise_c > max_temperature_rise_c + f64::EPSILON {
            findings.push(thermal_measured_rise_finding(
                scenario,
                measurement,
                ambient_temperature_c,
                measured_temperature_rise_c,
                max_temperature_rise_c,
            ));
        }
    }
}

fn validate_measurement_metadata(
    bound: &BoundBoard<'_>,
    measurement: &ThermalMeasurement,
) -> Result<(), String> {
    if measurement.name.trim().is_empty() {
        return Err(
            "THERMAL_MEASURED_TEMPERATURE_VALID thermal measurement name must be non-empty."
                .to_string(),
        );
    }
    if !bound
        .project
        .board
        .components
        .contains_key(&measurement.component)
    {
        return Err(format!(
            "THERMAL_MEASURED_TEMPERATURE_VALID thermal measurement {} component {} is absent from board.components.",
            measurement.name, measurement.component
        ));
    }
    if measurement.source.trim().is_empty() {
        return Err(format!(
            "THERMAL_MEASURED_TEMPERATURE_VALID thermal measurement {} source must be non-empty.",
            measurement.name
        ));
    }
    if !measurement.measured_temperature_c.is_finite() {
        return Err(format!(
            "THERMAL_MEASURED_TEMPERATURE_VALID thermal measurement {} measured_temperature_C must be finite.",
            measurement.name
        ));
    }
    if let Some(power_loss_w) = measurement.power_loss_w
        && (!power_loss_w.is_finite() || power_loss_w <= 0.0)
    {
        return Err(format!(
            "THERMAL_MEASURED_TEMPERATURE_VALID thermal measurement {} power_loss_w must be finite and positive when supplied.",
            measurement.name
        ));
    }
    Ok(())
}

fn thermal_measured_temperature_finding(
    scenario: &Scenario,
    measurement: &ThermalMeasurement,
    max_measured_temperature_c: f64,
) -> Finding {
    let mut finding = Finding::critical(
        THERMAL_MEASURED_TEMPERATURE_VALID,
        &scenario.name,
        format!(
            "Thermal measurement {} observed {:.3} C, above the reviewed {:.3} C limit.",
            measurement.name, measurement.measured_temperature_c, max_measured_temperature_c
        ),
    );
    populate_measured_thermal_finding(&mut finding, measurement);
    finding.limit.insert(
        "max_measured_temperature_C".to_string(),
        json!(max_measured_temperature_c),
    );
    finding.suggested_fixes = vec![
        "Reduce dissipated power, improve heat spreading or airflow, or repeat the measurement after the reviewed thermal change.".to_string(),
        "If the limit changed, update parameters.max_measured_temperature_C from reviewed thermal requirements.".to_string(),
    ];
    finding
}

fn thermal_measured_rise_finding(
    scenario: &Scenario,
    measurement: &ThermalMeasurement,
    ambient_temperature_c: f64,
    measured_temperature_rise_c: f64,
    max_temperature_rise_c: f64,
) -> Finding {
    let mut finding = Finding::critical(
        THERMAL_MEASURED_TEMPERATURE_VALID,
        &scenario.name,
        format!(
            "Thermal measurement {} observed {:.3} C rise over ambient, above the reviewed {:.3} C limit.",
            measurement.name, measured_temperature_rise_c, max_temperature_rise_c
        ),
    );
    populate_measured_thermal_finding(&mut finding, measurement);
    finding.measured.insert(
        "ambient_temperature_C".to_string(),
        json!(ambient_temperature_c),
    );
    finding.measured.insert(
        "measured_temperature_rise_C".to_string(),
        json!(measured_temperature_rise_c),
    );
    finding.limit.insert(
        "max_temperature_rise_C".to_string(),
        json!(max_temperature_rise_c),
    );
    finding.suggested_fixes = vec![
        "Reduce dissipated power, improve board/package heat transfer, or repeat the measurement under the reviewed ambient condition.".to_string(),
        "If the allowed temperature rise changed, update parameters.max_temperature_rise_C from reviewed thermal requirements.".to_string(),
    ];
    finding
}

fn populate_measured_thermal_finding(finding: &mut Finding, measurement: &ThermalMeasurement) {
    finding.component = Some(measurement.component.clone());
    finding.measured.insert(
        "thermal_measurement_name".to_string(),
        json!(measurement.name),
    );
    finding.measured.insert(
        "thermal_measurement_source".to_string(),
        json!(measurement.source),
    );
    finding
        .measured
        .insert("component".to_string(), json!(measurement.component));
    finding.measured.insert(
        "measured_temperature_C".to_string(),
        json!(measurement.measured_temperature_c),
    );
    if let Some(point) = measurement.measurement_point.as_deref() {
        finding
            .measured
            .insert("measurement_point".to_string(), json!(point));
    }
    if let Some(power_loss_w) = measurement.power_loss_w {
        finding
            .measured
            .insert("power_loss_w".to_string(), json!(power_loss_w));
    }
    if let Some(notes) = measurement.notes.as_deref() {
        finding
            .measured
            .insert("measurement_notes".to_string(), json!(notes));
    }
}
