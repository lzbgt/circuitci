use crate::board_ir::{
    AnalogRelation, AnalogSParameterAggregation, AnalogSParameterAssertion, AnalogSParameterMetric,
    AnalogSParameterNetworkAssertion, AnalogSParameterNetworkMetric, AnalogScenario, Scenario,
};
use crate::reports::Finding;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use super::SPICE_S_PARAMETER_ANALYSIS;
use super::analog_assertions::AnalogAssertionMeasurement;
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
            AnalogSParameterMetric::ReturnLossDb | AnalogSParameterMetric::Vswr
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
    Ok(())
}

pub(super) fn write_s_parameter_summary(s_parameters: &Path) -> Result<PathBuf, String> {
    let summary = s_parameters
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("s_parameter_summary.csv");
    let rows = summarize_s_parameters(s_parameters)?;
    let mut text = String::from(
        "parameter,row_count,min_frequency_hz,max_frequency_hz,min_mag_db,max_mag_db,min_mag_linear,max_mag_linear,min_return_loss_db,max_return_loss_db,min_insertion_loss_db,max_insertion_loss_db,min_vswr,max_vswr\n",
    );
    for row in rows {
        text.push_str(&format!(
            "{},{},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{},{},{},{},{},{}\n",
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

pub(super) fn write_s_parameter_network_summary(s_parameters: &Path) -> Result<PathBuf, String> {
    let summary = s_parameters
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("s_parameter_network_summary.csv");
    let row = summarize_s_parameter_network(s_parameters)?;
    let text = format!(
        "port_count,row_count,min_frequency_hz,max_frequency_hz,max_reciprocity_error_linear,frequency_hz_at_max_reciprocity_error,max_passivity_singular_value,frequency_hz_at_max_passivity\n{},{},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e},{:.12e}\n",
        row.port_count,
        row.row_count,
        row.min_frequency_hz,
        row.max_frequency_hz,
        row.max_reciprocity_error_linear,
        row.frequency_hz_at_max_reciprocity_error,
        row.max_passivity_singular_value,
        row.frequency_hz_at_max_passivity,
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
        let measured = network_metric_value(assertion.metric, &row);
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

#[derive(Debug, Clone)]
struct SParameterSample {
    frequency_hz: f64,
    mag_db: f64,
    mag_linear: f64,
}

#[derive(Debug, Clone)]
struct SParameterSummaryRow {
    parameter: String,
    row_count: usize,
    min_frequency_hz: f64,
    max_frequency_hz: f64,
    min_mag_db: f64,
    max_mag_db: f64,
    min_mag_linear: f64,
    max_mag_linear: f64,
    min_return_loss_db: Option<f64>,
    max_return_loss_db: Option<f64>,
    min_insertion_loss_db: Option<f64>,
    max_insertion_loss_db: Option<f64>,
    min_vswr: Option<f64>,
    max_vswr: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
struct ComplexValue {
    real: f64,
    imaginary: f64,
}

#[derive(Debug, Clone)]
struct SParameterNetworkSample {
    frequency_hz: f64,
    s11: ComplexValue,
    s12: ComplexValue,
    s21: ComplexValue,
    s22: ComplexValue,
}

#[derive(Debug, Clone)]
struct SParameterNetworkSummaryRow {
    port_count: usize,
    row_count: usize,
    min_frequency_hz: f64,
    max_frequency_hz: f64,
    max_reciprocity_error_linear: f64,
    frequency_hz_at_max_reciprocity_error: f64,
    max_passivity_singular_value: f64,
    frequency_hz_at_max_passivity: f64,
}

#[derive(Debug, Clone, Copy)]
struct SParameterAssertionEvaluation {
    relation: &'static str,
    margin: f64,
    passed: bool,
}

fn summarize_s_parameters(path: &Path) -> Result<Vec<SParameterSummaryRow>, String> {
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "Failed to read normalized S-parameter CSV {}: {error}",
            path.display()
        )
    })?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| "S-parameter CSV has no header row.".to_string())?;
    let header: Vec<_> = header.split(',').map(str::trim).collect();
    if header.first() != Some(&"frequency_hz") {
        return Err("S-parameter CSV header must start with frequency_hz.".to_string());
    }
    let mut parameter_columns = Vec::new();
    for (index, column) in header.iter().enumerate() {
        let Some(parameter) = column.strip_suffix("_mag_db") else {
            continue;
        };
        let Some(mag_linear_index) = header
            .iter()
            .position(|candidate| *candidate == format!("{parameter}_mag_linear"))
        else {
            return Err(format!(
                "S-parameter CSV is missing {parameter}_mag_linear for {parameter}_mag_db."
            ));
        };
        parameter_columns.push((parameter.to_string(), index, mag_linear_index));
    }
    if parameter_columns.is_empty() {
        return Err("S-parameter CSV has no *_mag_db columns.".to_string());
    }

    let mut samples: BTreeMap<String, Vec<SParameterSample>> = BTreeMap::new();
    for (line_index, line) in lines.enumerate() {
        let fields: Vec<_> = line.split(',').map(str::trim).collect();
        if fields.len() != header.len() {
            return Err(format!(
                "S-parameter CSV row {} has {} fields, expected {}.",
                line_index + 2,
                fields.len(),
                header.len()
            ));
        }
        let frequency_hz = parse_finite_f64(fields[0], "frequency_hz")?;
        if frequency_hz <= 0.0 {
            return Err(format!(
                "S-parameter CSV row {} has non-positive frequency {}.",
                line_index + 2,
                frequency_hz
            ));
        }
        for (parameter, mag_db_index, mag_linear_index) in &parameter_columns {
            let mag_db = parse_finite_f64(fields[*mag_db_index], parameter)?;
            let mag_linear = parse_finite_f64(fields[*mag_linear_index], parameter)?;
            if mag_linear < 0.0 {
                return Err(format!(
                    "S-parameter CSV row {} has negative linear magnitude for {parameter}.",
                    line_index + 2
                ));
            }
            samples
                .entry(parameter.clone())
                .or_default()
                .push(SParameterSample {
                    frequency_hz,
                    mag_db,
                    mag_linear,
                });
        }
    }
    if samples.values().any(Vec::is_empty) {
        return Err("S-parameter CSV has no numeric data rows.".to_string());
    }
    Ok(samples
        .into_iter()
        .map(|(parameter, values)| summarize_parameter(parameter, &values))
        .collect())
}

fn summarize_parameter(parameter: String, values: &[SParameterSample]) -> SParameterSummaryRow {
    let mut min_frequency_hz = f64::INFINITY;
    let mut max_frequency_hz = f64::NEG_INFINITY;
    let mut min_mag_db = f64::INFINITY;
    let mut max_mag_db = f64::NEG_INFINITY;
    let mut min_mag_linear = f64::INFINITY;
    let mut max_mag_linear = f64::NEG_INFINITY;
    let mut return_losses = Vec::new();
    let mut insertion_losses = Vec::new();
    let mut vswrs = Vec::new();
    let reflection = is_reflection_parameter(&parameter);
    for sample in values {
        min_frequency_hz = min_frequency_hz.min(sample.frequency_hz);
        max_frequency_hz = max_frequency_hz.max(sample.frequency_hz);
        min_mag_db = min_mag_db.min(sample.mag_db);
        max_mag_db = max_mag_db.max(sample.mag_db);
        min_mag_linear = min_mag_linear.min(sample.mag_linear);
        max_mag_linear = max_mag_linear.max(sample.mag_linear);
        if reflection {
            return_losses.push(-sample.mag_db);
            if sample.mag_linear < 1.0 {
                vswrs.push((1.0 + sample.mag_linear) / (1.0 - sample.mag_linear));
            }
        } else {
            insertion_losses.push(-sample.mag_db);
        }
    }
    SParameterSummaryRow {
        parameter,
        row_count: values.len(),
        min_frequency_hz,
        max_frequency_hz,
        min_mag_db,
        max_mag_db,
        min_mag_linear,
        max_mag_linear,
        min_return_loss_db: finite_min(&return_losses),
        max_return_loss_db: finite_max(&return_losses),
        min_insertion_loss_db: finite_min(&insertion_losses),
        max_insertion_loss_db: finite_max(&insertion_losses),
        min_vswr: finite_min(&vswrs),
        max_vswr: finite_max(&vswrs),
    }
}

fn summarize_s_parameter_network(path: &Path) -> Result<SParameterNetworkSummaryRow, String> {
    let samples = read_s_parameter_network_samples(path)?;
    let mut min_frequency_hz = f64::INFINITY;
    let mut max_frequency_hz = f64::NEG_INFINITY;
    let mut max_reciprocity_error_linear = f64::NEG_INFINITY;
    let mut frequency_hz_at_max_reciprocity_error = f64::NAN;
    let mut max_passivity_singular_value = f64::NEG_INFINITY;
    let mut frequency_hz_at_max_passivity = f64::NAN;
    for sample in &samples {
        min_frequency_hz = min_frequency_hz.min(sample.frequency_hz);
        max_frequency_hz = max_frequency_hz.max(sample.frequency_hz);
        let reciprocity_error = sample.s21.subtract(sample.s12).magnitude();
        if reciprocity_error > max_reciprocity_error_linear {
            max_reciprocity_error_linear = reciprocity_error;
            frequency_hz_at_max_reciprocity_error = sample.frequency_hz;
        }
        let passivity = two_port_max_singular_value(sample);
        if passivity > max_passivity_singular_value {
            max_passivity_singular_value = passivity;
            frequency_hz_at_max_passivity = sample.frequency_hz;
        }
    }
    Ok(SParameterNetworkSummaryRow {
        port_count: 2,
        row_count: samples.len(),
        min_frequency_hz,
        max_frequency_hz,
        max_reciprocity_error_linear,
        frequency_hz_at_max_reciprocity_error,
        max_passivity_singular_value,
        frequency_hz_at_max_passivity,
    })
}

fn read_s_parameter_network_samples(path: &Path) -> Result<Vec<SParameterNetworkSample>, String> {
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "Failed to read normalized S-parameter CSV {}: {error}",
            path.display()
        )
    })?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| "S-parameter CSV has no header row.".to_string())?;
    let header: Vec<_> = header.split(',').map(str::trim).collect();
    if header.first() != Some(&"frequency_hz") {
        return Err("S-parameter CSV header must start with frequency_hz.".to_string());
    }
    let required = ["s11", "s12", "s21", "s22"];
    let mut columns = BTreeMap::new();
    for parameter in required {
        let mag_index = header
            .iter()
            .position(|candidate| *candidate == format!("{parameter}_mag_linear"))
            .ok_or_else(|| {
                format!(
                    "S-parameter network summary requires {parameter}_mag_linear in normalized CSV."
                )
            })?;
        let phase_index = header
            .iter()
            .position(|candidate| *candidate == format!("{parameter}_phase_deg"))
            .ok_or_else(|| {
                format!(
                    "S-parameter network summary requires {parameter}_phase_deg in normalized CSV."
                )
            })?;
        columns.insert(parameter, (mag_index, phase_index));
    }

    let mut samples = Vec::new();
    let mut previous_frequency_hz: Option<f64> = None;
    for (line_index, line) in lines.enumerate() {
        let fields: Vec<_> = line.split(',').map(str::trim).collect();
        if fields.len() != header.len() {
            return Err(format!(
                "S-parameter CSV row {} has {} fields, expected {}.",
                line_index + 2,
                fields.len(),
                header.len()
            ));
        }
        let frequency_hz = parse_finite_f64(fields[0], "frequency_hz")?;
        if frequency_hz <= 0.0 {
            return Err(format!(
                "S-parameter CSV row {} has non-positive frequency {}.",
                line_index + 2,
                frequency_hz
            ));
        }
        if previous_frequency_hz.is_some_and(|previous| frequency_hz <= previous) {
            return Err(format!(
                "S-parameter CSV row {} has duplicate or non-increasing frequency {}.",
                line_index + 2,
                frequency_hz
            ));
        }
        previous_frequency_hz = Some(frequency_hz);
        samples.push(SParameterNetworkSample {
            frequency_hz,
            s11: parse_complex_parameter("s11", &columns, &fields)?,
            s12: parse_complex_parameter("s12", &columns, &fields)?,
            s21: parse_complex_parameter("s21", &columns, &fields)?,
            s22: parse_complex_parameter("s22", &columns, &fields)?,
        });
    }
    if samples.is_empty() {
        return Err("S-parameter CSV has no numeric network rows.".to_string());
    }
    Ok(samples)
}

fn parse_complex_parameter(
    parameter: &'static str,
    columns: &BTreeMap<&'static str, (usize, usize)>,
    fields: &[&str],
) -> Result<ComplexValue, String> {
    let (mag_index, phase_index) = columns
        .get(parameter)
        .copied()
        .expect("required columns were validated");
    let magnitude = parse_finite_f64(fields[mag_index], parameter)?;
    if magnitude < 0.0 {
        return Err(format!(
            "S-parameter CSV has negative linear magnitude for {parameter}."
        ));
    }
    let phase_deg = parse_finite_f64(fields[phase_index], parameter)?;
    Ok(ComplexValue::from_polar_degrees(magnitude, phase_deg))
}

impl ComplexValue {
    fn from_polar_degrees(magnitude: f64, phase_deg: f64) -> Self {
        let radians = phase_deg.to_radians();
        Self {
            real: magnitude * radians.cos(),
            imaginary: magnitude * radians.sin(),
        }
    }

    fn subtract(self, other: Self) -> Self {
        Self {
            real: self.real - other.real,
            imaginary: self.imaginary - other.imaginary,
        }
    }

    fn add(self, other: Self) -> Self {
        Self {
            real: self.real + other.real,
            imaginary: self.imaginary + other.imaginary,
        }
    }

    fn conjugate(self) -> Self {
        Self {
            real: self.real,
            imaginary: -self.imaginary,
        }
    }

    fn multiply(self, other: Self) -> Self {
        Self {
            real: self.real * other.real - self.imaginary * other.imaginary,
            imaginary: self.real * other.imaginary + self.imaginary * other.real,
        }
    }

    fn magnitude_squared(self) -> f64 {
        self.real
            .mul_add(self.real, self.imaginary * self.imaginary)
    }

    fn magnitude(self) -> f64 {
        self.magnitude_squared().sqrt()
    }
}

fn two_port_max_singular_value(sample: &SParameterNetworkSample) -> f64 {
    let a = sample.s11.magnitude_squared() + sample.s21.magnitude_squared();
    let d = sample.s12.magnitude_squared() + sample.s22.magnitude_squared();
    let b = sample
        .s11
        .conjugate()
        .multiply(sample.s12)
        .add(sample.s21.conjugate().multiply(sample.s22));
    let discriminant = (a - d).mul_add(a - d, 4.0 * b.magnitude_squared());
    let lambda_max = 0.5 * (a + d + discriminant.max(0.0).sqrt());
    lambda_max.max(0.0).sqrt()
}

fn read_s_parameter_summary(path: &Path) -> Result<Vec<SParameterSummaryRow>, String> {
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "Failed to read S-parameter summary {}: {error}",
            path.display()
        )
    })?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| "S-parameter summary CSV has no header row.".to_string())?;
    if header
        != "parameter,row_count,min_frequency_hz,max_frequency_hz,min_mag_db,max_mag_db,min_mag_linear,max_mag_linear,min_return_loss_db,max_return_loss_db,min_insertion_loss_db,max_insertion_loss_db,min_vswr,max_vswr"
    {
        return Err("S-parameter summary CSV has unexpected header.".to_string());
    }
    let mut rows = Vec::new();
    for (line_index, line) in lines.enumerate() {
        let fields: Vec<_> = line.split(',').map(str::trim).collect();
        if fields.len() != 14 {
            return Err(format!(
                "S-parameter summary row {} has {} fields, expected 14.",
                line_index + 2,
                fields.len()
            ));
        }
        rows.push(SParameterSummaryRow {
            parameter: fields[0].to_string(),
            row_count: fields[1].parse::<usize>().map_err(|_| {
                format!(
                    "S-parameter summary row {} has invalid row_count.",
                    line_index + 2
                )
            })?,
            min_frequency_hz: parse_finite_f64(fields[2], "min_frequency_hz")?,
            max_frequency_hz: parse_finite_f64(fields[3], "max_frequency_hz")?,
            min_mag_db: parse_finite_f64(fields[4], "min_mag_db")?,
            max_mag_db: parse_finite_f64(fields[5], "max_mag_db")?,
            min_mag_linear: parse_finite_f64(fields[6], "min_mag_linear")?,
            max_mag_linear: parse_finite_f64(fields[7], "max_mag_linear")?,
            min_return_loss_db: parse_optional_finite_f64(fields[8], "min_return_loss_db")?,
            max_return_loss_db: parse_optional_finite_f64(fields[9], "max_return_loss_db")?,
            min_insertion_loss_db: parse_optional_finite_f64(fields[10], "min_insertion_loss_db")?,
            max_insertion_loss_db: parse_optional_finite_f64(fields[11], "max_insertion_loss_db")?,
            min_vswr: parse_optional_finite_f64(fields[12], "min_vswr")?,
            max_vswr: parse_optional_finite_f64(fields[13], "max_vswr")?,
        });
    }
    if rows.is_empty() {
        return Err("S-parameter summary CSV has no parameter rows.".to_string());
    }
    Ok(rows)
}

fn read_s_parameter_network_summary(path: &Path) -> Result<SParameterNetworkSummaryRow, String> {
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "Failed to read S-parameter network summary {}: {error}",
            path.display()
        )
    })?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| "S-parameter network summary CSV has no header row.".to_string())?;
    if header
        != "port_count,row_count,min_frequency_hz,max_frequency_hz,max_reciprocity_error_linear,frequency_hz_at_max_reciprocity_error,max_passivity_singular_value,frequency_hz_at_max_passivity"
    {
        return Err("S-parameter network summary CSV has unexpected header.".to_string());
    }
    let row = lines
        .next()
        .ok_or_else(|| "S-parameter network summary CSV has no data row.".to_string())?;
    if lines.next().is_some() {
        return Err(
            "S-parameter network summary CSV must contain exactly one data row.".to_string(),
        );
    }
    let fields: Vec<_> = row.split(',').map(str::trim).collect();
    if fields.len() != 8 {
        return Err(format!(
            "S-parameter network summary row has {} fields, expected 8.",
            fields.len()
        ));
    }
    Ok(SParameterNetworkSummaryRow {
        port_count: fields[0]
            .parse::<usize>()
            .map_err(|_| "S-parameter network summary has invalid port_count.".to_string())?,
        row_count: fields[1]
            .parse::<usize>()
            .map_err(|_| "S-parameter network summary has invalid row_count.".to_string())?,
        min_frequency_hz: parse_finite_f64(fields[2], "min_frequency_hz")?,
        max_frequency_hz: parse_finite_f64(fields[3], "max_frequency_hz")?,
        max_reciprocity_error_linear: parse_finite_f64(fields[4], "max_reciprocity_error_linear")?,
        frequency_hz_at_max_reciprocity_error: parse_finite_f64(
            fields[5],
            "frequency_hz_at_max_reciprocity_error",
        )?,
        max_passivity_singular_value: parse_finite_f64(fields[6], "max_passivity_singular_value")?,
        frequency_hz_at_max_passivity: parse_finite_f64(
            fields[7],
            "frequency_hz_at_max_passivity",
        )?,
    })
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
    }
}

fn network_metric_value(
    metric: AnalogSParameterNetworkMetric,
    row: &SParameterNetworkSummaryRow,
) -> f64 {
    match metric {
        AnalogSParameterNetworkMetric::ReciprocityErrorLinear => row.max_reciprocity_error_linear,
        AnalogSParameterNetworkMetric::PassivityMaxSingularValue => {
            row.max_passivity_singular_value
        }
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
        "Use return_loss_db/vswr on reflection terms such as s11 and insertion_loss_db on transmission terms such as s21."
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

fn parse_s_parameter_name(parameter: &str) -> Option<(usize, usize)> {
    let lower = parameter.trim().to_ascii_lowercase();
    let digits = lower.strip_prefix('s')?;
    if digits.len() < 2 || !digits.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let midpoint = digits.len() / 2;
    let output = digits[..midpoint].parse::<usize>().ok()?;
    let input = digits[midpoint..].parse::<usize>().ok()?;
    Some((output, input))
}

fn is_reflection_parameter(parameter: &str) -> bool {
    parse_s_parameter_name(parameter).is_some_and(|(output, input)| output == input)
}

fn optional_csv(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.12e}"))
        .unwrap_or_default()
}

fn finite_min(values: &[f64]) -> Option<f64> {
    values.iter().copied().reduce(f64::min)
}

fn finite_max(values: &[f64]) -> Option<f64> {
    values.iter().copied().reduce(f64::max)
}

fn parse_optional_finite_f64(field: &str, name: &str) -> Result<Option<f64>, String> {
    if field.is_empty() {
        return Ok(None);
    }
    parse_finite_f64(field, name).map(Some)
}

fn parse_finite_f64(field: &str, name: &str) -> Result<f64, String> {
    let value = field
        .parse::<f64>()
        .map_err(|_| format!("S-parameter row has invalid {name}."))?;
    if !value.is_finite() {
        return Err(format!("S-parameter row has non-finite {name}."));
    }
    Ok(value)
}

fn s_parameter_assertion_unit(assertion: &AnalogSParameterAssertion) -> &str {
    assertion.unit.as_deref().unwrap_or(match assertion.metric {
        AnalogSParameterMetric::MagnitudeDb
        | AnalogSParameterMetric::ReturnLossDb
        | AnalogSParameterMetric::InsertionLossDb => "dB",
        AnalogSParameterMetric::MagnitudeLinear => "ratio",
        AnalogSParameterMetric::Vswr => "ratio",
    })
}

fn s_parameter_network_assertion_unit(assertion: &AnalogSParameterNetworkAssertion) -> &str {
    assertion.unit.as_deref().unwrap_or("ratio")
}

fn metric_name(metric: AnalogSParameterMetric) -> &'static str {
    match metric {
        AnalogSParameterMetric::MagnitudeDb => "magnitude_db",
        AnalogSParameterMetric::MagnitudeLinear => "magnitude_linear",
        AnalogSParameterMetric::ReturnLossDb => "return_loss_db",
        AnalogSParameterMetric::InsertionLossDb => "insertion_loss_db",
        AnalogSParameterMetric::Vswr => "vswr",
    }
}

fn network_metric_name(metric: AnalogSParameterNetworkMetric) -> &'static str {
    match metric {
        AnalogSParameterNetworkMetric::ReciprocityErrorLinear => "reciprocity_error_linear",
        AnalogSParameterNetworkMetric::PassivityMaxSingularValue => "passivity_max_singular_value",
    }
}

fn aggregation_name(aggregation: AnalogSParameterAggregation) -> &'static str {
    match aggregation {
        AnalogSParameterAggregation::Min => "min",
        AnalogSParameterAggregation::Max => "max",
    }
}
