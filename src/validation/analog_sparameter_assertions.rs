use crate::board_ir::{
    AnalogRelation, AnalogSParameterAggregation, AnalogSParameterAssertion, AnalogSParameterMetric,
    AnalogSParameterNetworkAssertion, AnalogSParameterNetworkMetric,
    AnalogSParameterReflectionCoefficient, AnalogScenario, Scenario,
};
use crate::reports::Finding;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::SPICE_S_PARAMETER_ANALYSIS;
use super::analog_assertions::AnalogAssertionMeasurement;
use super::analog_sparameter_summary::{
    SParameterNetworkSummaryRow, SParameterSummaryRow, optional_csv, parse_s_parameter_name,
    read_s_parameter_network_summary, read_s_parameter_summary, summarize_s_parameter_network,
    summarize_s_parameters,
};
use super::analog_util::normalize_artifact_path;

pub(super) fn validate_s_parameter_assertion_contract(
    analog: &AnalogScenario,
) -> Result<(), String> {
    let port_count = analog.analysis.s_parameter_ports.len();
    let mut names = BTreeSet::new();
    for assertion in &analog.analysis.s_parameter_assertions {
        let name = assertion.name.trim();
        if name.is_empty() {
            return Err(
                "analog_sparameter s_parameter_assertions entries require non-empty name."
                    .to_string(),
            );
        }
        if !names.insert(name.to_ascii_lowercase()) {
            return Err(format!(
                "analog_sparameter declares duplicate s_parameter_assertions name {name}."
            ));
        }
        let (output_port, input_port) = parse_s_parameter_name(&assertion.parameter)
            .ok_or_else(|| {
                format!(
                    "analog_sparameter s_parameter_assertion {name} parameter must be s<output><input>, for example s11 or s21."
                )
            })?;
        if output_port == 0
            || input_port == 0
            || output_port > port_count
            || input_port > port_count
        {
            return Err(format!(
                "analog_sparameter s_parameter_assertion {name} parameter {} is outside the declared {port_count}-port matrix.",
                assertion.parameter
            ));
        }
        if matches!(
            assertion.metric,
            AnalogSParameterMetric::ReturnLossDb
                | AnalogSParameterMetric::Vswr
                | AnalogSParameterMetric::MismatchLossDb
                | AnalogSParameterMetric::ImpedanceRealOhm
                | AnalogSParameterMetric::ImpedanceImagOhm
                | AnalogSParameterMetric::ImpedanceMagnitudeOhm
        ) && output_port != input_port
        {
            return Err(format!(
                "analog_sparameter s_parameter_assertion {name} metric {} requires a reflection parameter such as s11.",
                metric_name(assertion.metric)
            ));
        }
        if assertion.metric == AnalogSParameterMetric::InsertionLossDb && output_port == input_port
        {
            return Err(format!(
                "analog_sparameter s_parameter_assertion {name} metric insertion_loss_db requires a transmission parameter such as s21."
            ));
        }
        if !assertion.threshold.is_finite() {
            return Err(format!(
                "analog_sparameter s_parameter_assertion {name} requires finite threshold."
            ));
        }
        if assertion
            .unit
            .as_deref()
            .is_some_and(|unit| unit.trim().is_empty())
        {
            return Err(format!(
                "analog_sparameter s_parameter_assertion {name} unit must be non-empty when provided."
            ));
        }
    }
    let mut network_names = BTreeSet::new();
    for assertion in &analog.analysis.s_parameter_network_assertions {
        let name = assertion.name.trim();
        if name.is_empty() {
            return Err(
                "analog_sparameter s_parameter_network_assertions entries require non-empty name."
                    .to_string(),
            );
        }
        if !network_names.insert(name.to_ascii_lowercase()) {
            return Err(format!(
                "analog_sparameter declares duplicate s_parameter_network_assertions name {name}."
            ));
        }
        if port_count != 2 {
            return Err(format!(
                "analog_sparameter s_parameter_network_assertion {name} requires exactly two declared S-parameter ports."
            ));
        }
        if !assertion.threshold.is_finite() {
            return Err(format!(
                "analog_sparameter s_parameter_network_assertion {name} requires finite threshold."
            ));
        }
        if assertion
            .unit
            .as_deref()
            .is_some_and(|unit| unit.trim().is_empty())
        {
            return Err(format!(
                "analog_sparameter s_parameter_network_assertion {name} unit must be non-empty when provided."
            ));
        }
    }
    validate_optional_reflection(
        analog.analysis.s_parameter_source_reflection,
        "s_parameter_source_reflection",
    )?;
    validate_optional_reflection(
        analog.analysis.s_parameter_load_reflection,
        "s_parameter_load_reflection",
    )?;
    Ok(())
}

fn validate_optional_reflection(
    coefficient: Option<AnalogSParameterReflectionCoefficient>,
    name: &str,
) -> Result<(), String> {
    let Some(coefficient) = coefficient else {
        return Ok(());
    };
    if !coefficient.real.is_finite() || !coefficient.imaginary.is_finite() {
        return Err(format!(
            "analog_sparameter {name} requires finite real and imaginary values."
        ));
    }
    let magnitude_squared = coefficient.real.mul_add(
        coefficient.real,
        coefficient.imaginary * coefficient.imaginary,
    );
    if !magnitude_squared.is_finite() || magnitude_squared >= 1.0 {
        return Err(format!(
            "analog_sparameter {name} magnitude must be below 1.0 for finite passive gain calculations."
        ));
    }
    Ok(())
}

pub(super) fn write_s_parameter_summary(s_parameters: &Path) -> Result<PathBuf, String> {
    let summary = s_parameters
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("s_parameter_summary.csv");
    let rows = summarize_s_parameters(s_parameters)?;
    let mut text = String::from(
        "parameter,row_count,min_frequency_hz,max_frequency_hz,min_mag_db,max_mag_db,min_mag_linear,max_mag_linear,min_return_loss_db,max_return_loss_db,min_insertion_loss_db,max_insertion_loss_db,min_vswr,max_vswr,min_mismatch_loss_db,max_mismatch_loss_db,min_group_delay_s,max_group_delay_s,min_impedance_real_ohm,max_impedance_real_ohm,min_impedance_imag_ohm,max_impedance_imag_ohm,min_impedance_magnitude_ohm,max_impedance_magnitude_ohm\n",
    );
    for row in rows {
        text.push_str(&format!(
            "{},{},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            row.parameter,
            row.row_count,
            row.min_frequency_hz,
            row.max_frequency_hz,
            row.min_mag_db,
            row.max_mag_db,
            row.min_mag_linear,
            row.max_mag_linear,
            optional_csv(row.min_return_loss_db),
            optional_csv(row.max_return_loss_db),
            optional_csv(row.min_insertion_loss_db),
            optional_csv(row.max_insertion_loss_db),
            optional_csv(row.min_vswr),
            optional_csv(row.max_vswr),
            optional_csv(row.min_mismatch_loss_db),
            optional_csv(row.max_mismatch_loss_db),
            optional_csv(row.min_group_delay_s),
            optional_csv(row.max_group_delay_s),
            optional_csv(row.min_impedance_real_ohm),
            optional_csv(row.max_impedance_real_ohm),
            optional_csv(row.min_impedance_imag_ohm),
            optional_csv(row.max_impedance_imag_ohm),
            optional_csv(row.min_impedance_magnitude_ohm),
            optional_csv(row.max_impedance_magnitude_ohm),
        ));
    }
    fs::write(&summary, text).map_err(|error| {
        format!(
            "Failed to write S-parameter summary {}: {error}",
            summary.display()
        )
    })?;
    Ok(summary)
}

pub(super) fn evaluate_s_parameter_assertions(
    scenario: &Scenario,
    summary: &Path,
    findings: &mut Vec<Finding>,
) -> Vec<AnalogAssertionMeasurement> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before S-parameter assertions");
    if analog.analysis.s_parameter_assertions.is_empty() {
        return Vec::new();
    }
    let rows = match read_s_parameter_summary(summary) {
        Ok(rows) => rows,
        Err(message) => {
            findings.push(Finding::critical(
                SPICE_S_PARAMETER_ANALYSIS,
                &scenario.name,
                message,
            ));
            return Vec::new();
        }
    };
    let by_parameter: BTreeMap<String, SParameterSummaryRow> = rows
        .into_iter()
        .map(|row| (row.parameter.to_ascii_lowercase(), row))
        .collect();
    let mut measurements = Vec::new();
    for assertion in &analog.analysis.s_parameter_assertions {
        let key = assertion.parameter.to_ascii_lowercase();
        let Some(row) = by_parameter.get(&key) else {
            push_missing_parameter_finding(scenario, assertion, summary, findings);
            continue;
        };
        let Some(measured) = metric_value(assertion, row) else {
            push_metric_unavailable_finding(scenario, assertion, row, summary, findings);
            continue;
        };
        let evaluation = evaluate_assertion_value(assertion, measured);
        let unit = s_parameter_assertion_unit(assertion);
        measurements.push(AnalogAssertionMeasurement {
            assertion_name: assertion.name.clone(),
            probe_name: row.parameter.clone(),
            measured,
            limit: assertion.threshold,
            margin: evaluation.margin,
            relation: evaluation.relation,
            unit: unit.to_string(),
            quantity: format!(
                "s_parameter {} {}",
                aggregation_name(assertion.aggregation),
                metric_name(assertion.metric)
            ),
            passed: evaluation.passed,
        });
        push_s_parameter_assertion_finding(
            scenario, assertion, row, summary, measured, evaluation, findings,
        );
    }
    measurements
}

pub(super) fn write_s_parameter_network_summary(
    s_parameters: &Path,
    analog: &AnalogScenario,
) -> Result<PathBuf, String> {
    let summary = s_parameters
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("s_parameter_network_summary.csv");
    let row = summarize_s_parameter_network(s_parameters, analog)?;
    let text = format!(
        "port_count,row_count,min_frequency_hz,max_frequency_hz,max_reciprocity_error_linear,frequency_hz_at_max_reciprocity_error,max_passivity_singular_value,frequency_hz_at_max_passivity,min_rollet_k,frequency_hz_at_min_rollet_k,max_stability_delta_magnitude,frequency_hz_at_max_stability_delta_magnitude,min_maximum_available_gain_db,frequency_hz_at_min_maximum_available_gain,min_maximum_stable_gain_db,frequency_hz_at_min_maximum_stable_gain,min_maximum_unilateral_gain_db,frequency_hz_at_min_maximum_unilateral_gain,source_reflection_real,source_reflection_imaginary,load_reflection_real,load_reflection_imaginary,min_transducer_gain_db,frequency_hz_at_min_transducer_gain,min_available_gain_db,frequency_hz_at_min_available_gain,min_operating_gain_db,frequency_hz_at_min_operating_gain\n{},{},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
        row.port_count,
        row.row_count,
        row.min_frequency_hz,
        row.max_frequency_hz,
        row.max_reciprocity_error_linear,
        row.frequency_hz_at_max_reciprocity_error,
        row.max_passivity_singular_value,
        row.frequency_hz_at_max_passivity,
        optional_csv(row.min_rollet_k),
        optional_csv(row.frequency_hz_at_min_rollet_k),
        optional_csv(row.max_stability_delta_magnitude),
        optional_csv(row.frequency_hz_at_max_stability_delta_magnitude),
        optional_csv(row.min_maximum_available_gain_db),
        optional_csv(row.frequency_hz_at_min_maximum_available_gain),
        optional_csv(row.min_maximum_stable_gain_db),
        optional_csv(row.frequency_hz_at_min_maximum_stable_gain),
        optional_csv(row.min_maximum_unilateral_gain_db),
        optional_csv(row.frequency_hz_at_min_maximum_unilateral_gain),
        optional_csv(row.source_reflection_real),
        optional_csv(row.source_reflection_imaginary),
        optional_csv(row.load_reflection_real),
        optional_csv(row.load_reflection_imaginary),
        optional_csv(row.min_transducer_gain_db),
        optional_csv(row.frequency_hz_at_min_transducer_gain),
        optional_csv(row.min_available_gain_db),
        optional_csv(row.frequency_hz_at_min_available_gain),
        optional_csv(row.min_operating_gain_db),
        optional_csv(row.frequency_hz_at_min_operating_gain),
    );
    fs::write(&summary, text).map_err(|error| {
        format!(
            "Failed to write S-parameter network summary {}: {error}",
            summary.display()
        )
    })?;
    Ok(summary)
}

pub(super) fn evaluate_s_parameter_network_assertions(
    scenario: &Scenario,
    summary: &Path,
    findings: &mut Vec<Finding>,
) -> Vec<AnalogAssertionMeasurement> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before S-parameter network assertions");
    if analog.analysis.s_parameter_network_assertions.is_empty() {
        return Vec::new();
    }
    let row = match read_s_parameter_network_summary(summary) {
        Ok(row) => row,
        Err(message) => {
            findings.push(Finding::critical(
                SPICE_S_PARAMETER_ANALYSIS,
                &scenario.name,
                message,
            ));
            return Vec::new();
        }
    };
    let mut measurements = Vec::new();
    for assertion in &analog.analysis.s_parameter_network_assertions {
        let Some(measured) = network_metric_value(assertion.metric, &row) else {
            push_s_parameter_network_metric_unavailable_finding(
                scenario, assertion, &row, summary, findings,
            );
            continue;
        };
        let evaluation = evaluate_network_assertion_value(assertion, measured);
        let unit = s_parameter_network_assertion_unit(assertion);
        measurements.push(AnalogAssertionMeasurement {
            assertion_name: assertion.name.clone(),
            probe_name: "two_port_network".to_string(),
            measured,
            limit: assertion.threshold,
            margin: evaluation.margin,
            relation: evaluation.relation,
            unit: unit.to_string(),
            quantity: format!(
                "s_parameter_network {}",
                network_metric_name(assertion.metric)
            ),
            passed: evaluation.passed,
        });
        push_s_parameter_network_assertion_finding(
            scenario, assertion, &row, summary, measured, evaluation, findings,
        );
    }
    measurements
}

#[derive(Debug, Clone, Copy)]
struct SParameterAssertionEvaluation {
    relation: &'static str,
    margin: f64,
    passed: bool,
}

fn metric_value(assertion: &AnalogSParameterAssertion, row: &SParameterSummaryRow) -> Option<f64> {
    match (assertion.metric, assertion.aggregation) {
        (AnalogSParameterMetric::MagnitudeDb, AnalogSParameterAggregation::Min) => {
            Some(row.min_mag_db)
        }
        (AnalogSParameterMetric::MagnitudeDb, AnalogSParameterAggregation::Max) => {
            Some(row.max_mag_db)
        }
        (AnalogSParameterMetric::MagnitudeLinear, AnalogSParameterAggregation::Min) => {
            Some(row.min_mag_linear)
        }
        (AnalogSParameterMetric::MagnitudeLinear, AnalogSParameterAggregation::Max) => {
            Some(row.max_mag_linear)
        }
        (AnalogSParameterMetric::ReturnLossDb, AnalogSParameterAggregation::Min) => {
            row.min_return_loss_db
        }
        (AnalogSParameterMetric::ReturnLossDb, AnalogSParameterAggregation::Max) => {
            row.max_return_loss_db
        }
        (AnalogSParameterMetric::InsertionLossDb, AnalogSParameterAggregation::Min) => {
            row.min_insertion_loss_db
        }
        (AnalogSParameterMetric::InsertionLossDb, AnalogSParameterAggregation::Max) => {
            row.max_insertion_loss_db
        }
        (AnalogSParameterMetric::Vswr, AnalogSParameterAggregation::Min) => row.min_vswr,
        (AnalogSParameterMetric::Vswr, AnalogSParameterAggregation::Max) => row.max_vswr,
        (AnalogSParameterMetric::MismatchLossDb, AnalogSParameterAggregation::Min) => {
            row.min_mismatch_loss_db
        }
        (AnalogSParameterMetric::MismatchLossDb, AnalogSParameterAggregation::Max) => {
            row.max_mismatch_loss_db
        }
        (AnalogSParameterMetric::GroupDelayS, AnalogSParameterAggregation::Min) => {
            row.min_group_delay_s
        }
        (AnalogSParameterMetric::GroupDelayS, AnalogSParameterAggregation::Max) => {
            row.max_group_delay_s
        }
        (AnalogSParameterMetric::ImpedanceRealOhm, AnalogSParameterAggregation::Min) => {
            row.min_impedance_real_ohm
        }
        (AnalogSParameterMetric::ImpedanceRealOhm, AnalogSParameterAggregation::Max) => {
            row.max_impedance_real_ohm
        }
        (AnalogSParameterMetric::ImpedanceImagOhm, AnalogSParameterAggregation::Min) => {
            row.min_impedance_imag_ohm
        }
        (AnalogSParameterMetric::ImpedanceImagOhm, AnalogSParameterAggregation::Max) => {
            row.max_impedance_imag_ohm
        }
        (AnalogSParameterMetric::ImpedanceMagnitudeOhm, AnalogSParameterAggregation::Min) => {
            row.min_impedance_magnitude_ohm
        }
        (AnalogSParameterMetric::ImpedanceMagnitudeOhm, AnalogSParameterAggregation::Max) => {
            row.max_impedance_magnitude_ohm
        }
    }
}

fn network_metric_value(
    metric: AnalogSParameterNetworkMetric,
    row: &SParameterNetworkSummaryRow,
) -> Option<f64> {
    match metric {
        AnalogSParameterNetworkMetric::ReciprocityErrorLinear => {
            Some(row.max_reciprocity_error_linear)
        }
        AnalogSParameterNetworkMetric::PassivityMaxSingularValue => {
            Some(row.max_passivity_singular_value)
        }
        AnalogSParameterNetworkMetric::RolletKMin => row.min_rollet_k,
        AnalogSParameterNetworkMetric::StabilityDeltaMagnitudeMax => {
            row.max_stability_delta_magnitude
        }
        AnalogSParameterNetworkMetric::MaximumAvailableGainDbMin => {
            row.min_maximum_available_gain_db
        }
        AnalogSParameterNetworkMetric::MaximumStableGainDbMin => row.min_maximum_stable_gain_db,
        AnalogSParameterNetworkMetric::MaximumUnilateralGainDbMin => {
            row.min_maximum_unilateral_gain_db
        }
        AnalogSParameterNetworkMetric::TransducerGainDbMin => row.min_transducer_gain_db,
        AnalogSParameterNetworkMetric::AvailableGainDbMin => row.min_available_gain_db,
        AnalogSParameterNetworkMetric::OperatingGainDbMin => row.min_operating_gain_db,
    }
}

fn evaluate_assertion_value(
    assertion: &AnalogSParameterAssertion,
    measured: f64,
) -> SParameterAssertionEvaluation {
    match assertion.relation {
        AnalogRelation::Above => SParameterAssertionEvaluation {
            relation: "above",
            margin: measured - assertion.threshold,
            passed: measured > assertion.threshold,
        },
        AnalogRelation::Below => SParameterAssertionEvaluation {
            relation: "below",
            margin: assertion.threshold - measured,
            passed: measured < assertion.threshold,
        },
    }
}

fn evaluate_network_assertion_value(
    assertion: &AnalogSParameterNetworkAssertion,
    measured: f64,
) -> SParameterAssertionEvaluation {
    match assertion.relation {
        AnalogRelation::Above => SParameterAssertionEvaluation {
            relation: "above",
            margin: measured - assertion.threshold,
            passed: measured > assertion.threshold,
        },
        AnalogRelation::Below => SParameterAssertionEvaluation {
            relation: "below",
            margin: assertion.threshold - measured,
            passed: measured < assertion.threshold,
        },
    }
}

fn push_s_parameter_assertion_finding(
    scenario: &Scenario,
    assertion: &AnalogSParameterAssertion,
    row: &SParameterSummaryRow,
    summary: &Path,
    measured: f64,
    evaluation: SParameterAssertionEvaluation,
    findings: &mut Vec<Finding>,
) {
    if evaluation.passed {
        return;
    }
    let unit = s_parameter_assertion_unit(assertion);
    let mut finding = Finding::critical(
        SPICE_S_PARAMETER_ANALYSIS,
        &scenario.name,
        format!(
            "S-parameter assertion {} failed: {} {} {} measured {:.6e} {}, expected {} {:.6e} {}.",
            assertion.name,
            assertion.parameter,
            aggregation_name(assertion.aggregation),
            metric_name(assertion.metric),
            measured,
            unit,
            evaluation.relation,
            assertion.threshold,
            unit
        ),
    );
    insert_common_metadata(assertion, row, summary, &mut finding);
    finding
        .measured
        .insert("value".to_string(), json!(measured));
    finding.measured.insert("unit".to_string(), json!(unit));
    finding
        .measured
        .insert("margin".to_string(), json!(evaluation.margin));
    finding.limit.insert(
        format!("{}_threshold", evaluation.relation),
        json!(assertion.threshold),
    );
    if assertion.suggested_fixes.is_empty() {
        finding.suggested_fixes.push(
            "Adjust matching, termination, launch geometry, loss budget, or component parasitics so the normalized S-parameter summary meets the declared limit."
                .to_string(),
        );
    } else {
        finding
            .suggested_fixes
            .extend(assertion.suggested_fixes.clone());
    }
    findings.push(finding);
}

fn push_s_parameter_network_assertion_finding(
    scenario: &Scenario,
    assertion: &AnalogSParameterNetworkAssertion,
    row: &SParameterNetworkSummaryRow,
    summary: &Path,
    measured: f64,
    evaluation: SParameterAssertionEvaluation,
    findings: &mut Vec<Finding>,
) {
    if evaluation.passed {
        return;
    }
    let unit = s_parameter_network_assertion_unit(assertion);
    let mut finding = Finding::critical(
        SPICE_S_PARAMETER_ANALYSIS,
        &scenario.name,
        format!(
            "S-parameter network assertion {} failed: {} measured {:.6e} {}, expected {} {:.6e} {}.",
            assertion.name,
            network_metric_name(assertion.metric),
            measured,
            unit,
            evaluation.relation,
            assertion.threshold,
            unit
        ),
    );
    finding
        .measured
        .insert("assertion".to_string(), json!(&assertion.name));
    finding.measured.insert(
        "metric".to_string(),
        json!(network_metric_name(assertion.metric)),
    );
    finding
        .measured
        .insert("value".to_string(), json!(measured));
    finding.measured.insert("unit".to_string(), json!(unit));
    finding
        .measured
        .insert("margin".to_string(), json!(evaluation.margin));
    finding
        .measured
        .insert("port_count".to_string(), json!(row.port_count));
    finding
        .measured
        .insert("row_count".to_string(), json!(row.row_count));
    finding.measured.insert(
        "frequency_start_hz".to_string(),
        json!(row.min_frequency_hz),
    );
    finding
        .measured
        .insert("frequency_stop_hz".to_string(), json!(row.max_frequency_hz));
    finding.measured.insert(
        "frequency_hz_at_max_reciprocity_error".to_string(),
        json!(row.frequency_hz_at_max_reciprocity_error),
    );
    finding.measured.insert(
        "frequency_hz_at_max_passivity".to_string(),
        json!(row.frequency_hz_at_max_passivity),
    );
    finding
        .measured
        .insert("min_rollet_k".to_string(), json!(row.min_rollet_k));
    finding.measured.insert(
        "frequency_hz_at_min_rollet_k".to_string(),
        json!(row.frequency_hz_at_min_rollet_k),
    );
    finding.measured.insert(
        "max_stability_delta_magnitude".to_string(),
        json!(row.max_stability_delta_magnitude),
    );
    finding.measured.insert(
        "frequency_hz_at_max_stability_delta_magnitude".to_string(),
        json!(row.frequency_hz_at_max_stability_delta_magnitude),
    );
    finding.measured.insert(
        "min_maximum_available_gain_db".to_string(),
        json!(row.min_maximum_available_gain_db),
    );
    finding.measured.insert(
        "frequency_hz_at_min_maximum_available_gain".to_string(),
        json!(row.frequency_hz_at_min_maximum_available_gain),
    );
    finding.measured.insert(
        "min_maximum_stable_gain_db".to_string(),
        json!(row.min_maximum_stable_gain_db),
    );
    finding.measured.insert(
        "frequency_hz_at_min_maximum_stable_gain".to_string(),
        json!(row.frequency_hz_at_min_maximum_stable_gain),
    );
    finding.measured.insert(
        "min_maximum_unilateral_gain_db".to_string(),
        json!(row.min_maximum_unilateral_gain_db),
    );
    finding.measured.insert(
        "frequency_hz_at_min_maximum_unilateral_gain".to_string(),
        json!(row.frequency_hz_at_min_maximum_unilateral_gain),
    );
    finding.measured.insert(
        "source_reflection_real".to_string(),
        json!(row.source_reflection_real),
    );
    finding.measured.insert(
        "source_reflection_imaginary".to_string(),
        json!(row.source_reflection_imaginary),
    );
    finding.measured.insert(
        "load_reflection_real".to_string(),
        json!(row.load_reflection_real),
    );
    finding.measured.insert(
        "load_reflection_imaginary".to_string(),
        json!(row.load_reflection_imaginary),
    );
    finding.measured.insert(
        "min_transducer_gain_db".to_string(),
        json!(row.min_transducer_gain_db),
    );
    finding.measured.insert(
        "frequency_hz_at_min_transducer_gain".to_string(),
        json!(row.frequency_hz_at_min_transducer_gain),
    );
    finding.measured.insert(
        "min_available_gain_db".to_string(),
        json!(row.min_available_gain_db),
    );
    finding.measured.insert(
        "frequency_hz_at_min_available_gain".to_string(),
        json!(row.frequency_hz_at_min_available_gain),
    );
    finding.measured.insert(
        "min_operating_gain_db".to_string(),
        json!(row.min_operating_gain_db),
    );
    finding.measured.insert(
        "frequency_hz_at_min_operating_gain".to_string(),
        json!(row.frequency_hz_at_min_operating_gain),
    );
    finding.measured.insert(
        "s_parameter_network_summary".to_string(),
        json!(normalize_artifact_path(summary)),
    );
    finding.limit.insert(
        format!("{}_threshold", evaluation.relation),
        json!(assertion.threshold),
    );
    if assertion.suggested_fixes.is_empty() {
        finding.suggested_fixes.push(
            "Inspect the two-port fixture, port ordering, matching network, loss model, and active-device biasing so the S-parameter network quality limit is physically plausible."
                .to_string(),
        );
    } else {
        finding
            .suggested_fixes
            .extend(assertion.suggested_fixes.clone());
    }
    findings.push(finding);
}

fn push_s_parameter_network_metric_unavailable_finding(
    scenario: &Scenario,
    assertion: &AnalogSParameterNetworkAssertion,
    row: &SParameterNetworkSummaryRow,
    summary: &Path,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        SPICE_S_PARAMETER_ANALYSIS,
        &scenario.name,
        format!(
            "S-parameter network assertion {} metric {} is unavailable from the two-port summary.",
            assertion.name,
            network_metric_name(assertion.metric)
        ),
    );
    finding
        .measured
        .insert("assertion".to_string(), json!(&assertion.name));
    finding.measured.insert(
        "metric".to_string(),
        json!(network_metric_name(assertion.metric)),
    );
    finding
        .measured
        .insert("port_count".to_string(), json!(row.port_count));
    finding
        .measured
        .insert("row_count".to_string(), json!(row.row_count));
    finding.measured.insert(
        "frequency_start_hz".to_string(),
        json!(row.min_frequency_hz),
    );
    finding
        .measured
        .insert("frequency_stop_hz".to_string(), json!(row.max_frequency_hz));
    finding.measured.insert(
        "s_parameter_network_summary".to_string(),
        json!(normalize_artifact_path(summary)),
    );
    finding.limit.insert(
        "required_metric".to_string(),
        json!(network_metric_name(assertion.metric)),
    );
    finding.suggested_fixes.push(
        "Use a two-port sweep with nonzero forward and reverse transmission and a stable active-network region if gain or stability-factor sign-off is required."
            .to_string(),
    );
    findings.push(finding);
}

fn push_missing_parameter_finding(
    scenario: &Scenario,
    assertion: &AnalogSParameterAssertion,
    summary: &Path,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        SPICE_S_PARAMETER_ANALYSIS,
        &scenario.name,
        format!(
            "S-parameter assertion {} references missing parameter {}.",
            assertion.name, assertion.parameter
        ),
    );
    finding
        .measured
        .insert("assertion".to_string(), json!(&assertion.name));
    finding
        .measured
        .insert("parameter".to_string(), json!(&assertion.parameter));
    finding.measured.insert(
        "s_parameter_summary".to_string(),
        json!(normalize_artifact_path(summary)),
    );
    finding.limit.insert(
        "required_parameter".to_string(),
        json!(&assertion.parameter),
    );
    findings.push(finding);
}

fn push_metric_unavailable_finding(
    scenario: &Scenario,
    assertion: &AnalogSParameterAssertion,
    row: &SParameterSummaryRow,
    summary: &Path,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        SPICE_S_PARAMETER_ANALYSIS,
        &scenario.name,
        format!(
            "S-parameter assertion {} metric {} is unavailable for parameter {}.",
            assertion.name,
            metric_name(assertion.metric),
            row.parameter
        ),
    );
    insert_common_metadata(assertion, row, summary, &mut finding);
    finding.limit.insert(
        "required_metric".to_string(),
        json!(metric_name(assertion.metric)),
    );
    finding.suggested_fixes.push(
        "Use return_loss_db/vswr/mismatch_loss_db/impedance metrics on reflection terms such as s11 and insertion_loss_db on transmission terms such as s21."
            .to_string(),
    );
    findings.push(finding);
}

fn insert_common_metadata(
    assertion: &AnalogSParameterAssertion,
    row: &SParameterSummaryRow,
    summary: &Path,
    finding: &mut Finding,
) {
    finding
        .measured
        .insert("assertion".to_string(), json!(&assertion.name));
    finding
        .measured
        .insert("parameter".to_string(), json!(&row.parameter));
    finding
        .measured
        .insert("metric".to_string(), json!(metric_name(assertion.metric)));
    finding.measured.insert(
        "aggregation".to_string(),
        json!(aggregation_name(assertion.aggregation)),
    );
    finding
        .measured
        .insert("row_count".to_string(), json!(row.row_count));
    finding.measured.insert(
        "frequency_start_hz".to_string(),
        json!(row.min_frequency_hz),
    );
    finding
        .measured
        .insert("frequency_stop_hz".to_string(), json!(row.max_frequency_hz));
    finding.measured.insert(
        "s_parameter_summary".to_string(),
        json!(normalize_artifact_path(summary)),
    );
}

fn s_parameter_assertion_unit(assertion: &AnalogSParameterAssertion) -> &str {
    assertion.unit.as_deref().unwrap_or(match assertion.metric {
        AnalogSParameterMetric::MagnitudeDb
        | AnalogSParameterMetric::ReturnLossDb
        | AnalogSParameterMetric::InsertionLossDb
        | AnalogSParameterMetric::MismatchLossDb => "dB",
        AnalogSParameterMetric::MagnitudeLinear => "ratio",
        AnalogSParameterMetric::Vswr => "ratio",
        AnalogSParameterMetric::GroupDelayS => "s",
        AnalogSParameterMetric::ImpedanceRealOhm
        | AnalogSParameterMetric::ImpedanceImagOhm
        | AnalogSParameterMetric::ImpedanceMagnitudeOhm => "ohm",
    })
}

fn s_parameter_network_assertion_unit(assertion: &AnalogSParameterNetworkAssertion) -> &str {
    assertion.unit.as_deref().unwrap_or(match assertion.metric {
        AnalogSParameterNetworkMetric::MaximumAvailableGainDbMin
        | AnalogSParameterNetworkMetric::MaximumStableGainDbMin
        | AnalogSParameterNetworkMetric::MaximumUnilateralGainDbMin
        | AnalogSParameterNetworkMetric::TransducerGainDbMin
        | AnalogSParameterNetworkMetric::AvailableGainDbMin
        | AnalogSParameterNetworkMetric::OperatingGainDbMin => "dB",
        _ => "ratio",
    })
}

fn metric_name(metric: AnalogSParameterMetric) -> &'static str {
    match metric {
        AnalogSParameterMetric::MagnitudeDb => "magnitude_db",
        AnalogSParameterMetric::MagnitudeLinear => "magnitude_linear",
        AnalogSParameterMetric::ReturnLossDb => "return_loss_db",
        AnalogSParameterMetric::InsertionLossDb => "insertion_loss_db",
        AnalogSParameterMetric::Vswr => "vswr",
        AnalogSParameterMetric::MismatchLossDb => "mismatch_loss_db",
        AnalogSParameterMetric::GroupDelayS => "group_delay_s",
        AnalogSParameterMetric::ImpedanceRealOhm => "impedance_real_ohm",
        AnalogSParameterMetric::ImpedanceImagOhm => "impedance_imag_ohm",
        AnalogSParameterMetric::ImpedanceMagnitudeOhm => "impedance_magnitude_ohm",
    }
}

fn network_metric_name(metric: AnalogSParameterNetworkMetric) -> &'static str {
    match metric {
        AnalogSParameterNetworkMetric::ReciprocityErrorLinear => "reciprocity_error_linear",
        AnalogSParameterNetworkMetric::PassivityMaxSingularValue => "passivity_max_singular_value",
        AnalogSParameterNetworkMetric::RolletKMin => "rollet_k_min",
        AnalogSParameterNetworkMetric::StabilityDeltaMagnitudeMax => {
            "stability_delta_magnitude_max"
        }
        AnalogSParameterNetworkMetric::MaximumAvailableGainDbMin => "maximum_available_gain_db_min",
        AnalogSParameterNetworkMetric::MaximumStableGainDbMin => "maximum_stable_gain_db_min",
        AnalogSParameterNetworkMetric::MaximumUnilateralGainDbMin => {
            "maximum_unilateral_gain_db_min"
        }
        AnalogSParameterNetworkMetric::TransducerGainDbMin => "transducer_gain_db_min",
        AnalogSParameterNetworkMetric::AvailableGainDbMin => "available_gain_db_min",
        AnalogSParameterNetworkMetric::OperatingGainDbMin => "operating_gain_db_min",
    }
}

fn aggregation_name(aggregation: AnalogSParameterAggregation) -> &'static str {
    match aggregation {
        AnalogSParameterAggregation::Min => "min",
        AnalogSParameterAggregation::Max => "max",
    }
}
