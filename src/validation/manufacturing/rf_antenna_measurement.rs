use crate::board_ir::{RfAntennaMeasurement, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;

use super::super::RF_ANTENNA_MEASURED_PERFORMANCE_VALID;
use super::super::common::validation_input_missing;

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
