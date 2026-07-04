use serde::Serialize;
use std::fs;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct DistortionSummary {
    pub artifact: String,
    pub component: String,
    pub output_expression: String,
    pub row_count: usize,
    pub max_magnitude: f64,
    pub frequency_hz_at_max: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct FourierSummary {
    pub artifact: String,
    pub output_expression: String,
    pub harmonic: u32,
    pub frequency_hz: f64,
    pub magnitude: f64,
    pub phase_deg: f64,
    pub normalized_magnitude: f64,
    pub normalized_phase_deg: f64,
    pub thd_percent: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PoleZeroSummary {
    pub artifact: String,
    pub output_node: String,
    pub reference_node: String,
    pub input_source: String,
    pub mode: String,
    pub root_kind: String,
    pub root_index: u32,
    pub real_rad_per_s: f64,
    pub imaginary_rad_per_s: f64,
    pub frequency_hz: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SensitivitySummary {
    pub artifact: String,
    pub output_expression: String,
    pub mode: String,
    pub parameter: String,
    pub frequency_hz: Option<f64>,
    pub sensitivity_real: f64,
    pub sensitivity_imaginary: f64,
    pub sensitivity_magnitude: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct TransferFunctionSummary {
    pub artifact: String,
    pub output_expression: String,
    pub input_source: String,
    pub transfer_function_gain: f64,
    pub input_resistance_ohm: f64,
    pub output_resistance_ohm: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SParameterSummary {
    pub artifact: String,
    pub parameter: String,
    pub row_count: usize,
    pub min_frequency_hz: f64,
    pub max_frequency_hz: f64,
    pub min_mag_db: f64,
    pub max_mag_db: f64,
    pub min_mag_linear: f64,
    pub max_mag_linear: f64,
    pub min_return_loss_db: Option<f64>,
    pub max_return_loss_db: Option<f64>,
    pub min_insertion_loss_db: Option<f64>,
    pub max_insertion_loss_db: Option<f64>,
    pub min_vswr: Option<f64>,
    pub max_vswr: Option<f64>,
}

pub(super) fn collect_distortion_summaries(artifacts: &[String]) -> Vec<DistortionSummary> {
    let mut records = Vec::new();
    for artifact in artifacts {
        if !artifact.ends_with("distortion_summary.csv") {
            continue;
        }
        let Ok(text) = fs::read_to_string(artifact) else {
            continue;
        };
        records.extend(parse_distortion_summary_csv(artifact, &text));
    }
    records.sort_by(|left, right| {
        left.artifact
            .cmp(&right.artifact)
            .then_with(|| left.component.cmp(&right.component))
            .then_with(|| left.output_expression.cmp(&right.output_expression))
    });
    records
}

fn parse_distortion_summary_csv(artifact: &str, text: &str) -> Vec<DistortionSummary> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let Some(header) = lines.next() else {
        return Vec::new();
    };
    let Some(header) = split_csv_fields(header) else {
        return Vec::new();
    };
    if header
        != [
            "component",
            "output_expression",
            "row_count",
            "max_magnitude",
            "frequency_hz_at_max",
        ]
    {
        return Vec::new();
    }
    let mut rows = Vec::new();
    for line in lines {
        let Some(fields) = split_csv_fields(line) else {
            continue;
        };
        if fields.len() != 5 {
            continue;
        }
        let Some(row_count) = fields[2].parse::<usize>().ok() else {
            continue;
        };
        let Some(max_magnitude) = parse_finite_f64(&fields[3]) else {
            continue;
        };
        let Some(frequency_hz_at_max) = parse_finite_f64(&fields[4]) else {
            continue;
        };
        rows.push(DistortionSummary {
            artifact: artifact.to_string(),
            component: fields[0].clone(),
            output_expression: fields[1].clone(),
            row_count,
            max_magnitude,
            frequency_hz_at_max,
        });
    }
    rows
}

pub(super) fn collect_fourier_summaries(artifacts: &[String]) -> Vec<FourierSummary> {
    let mut records = Vec::new();
    for artifact in artifacts {
        if !artifact.ends_with("fourier_summary.csv") {
            continue;
        }
        let Ok(text) = fs::read_to_string(artifact) else {
            continue;
        };
        records.extend(parse_fourier_summary_csv(artifact, &text));
    }
    records.sort_by(|left, right| {
        left.artifact
            .cmp(&right.artifact)
            .then_with(|| left.output_expression.cmp(&right.output_expression))
            .then_with(|| left.harmonic.cmp(&right.harmonic))
    });
    records
}

fn parse_fourier_summary_csv(artifact: &str, text: &str) -> Vec<FourierSummary> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let Some(header) = lines.next() else {
        return Vec::new();
    };
    let Some(header) = split_csv_fields(header) else {
        return Vec::new();
    };
    if header
        != [
            "output_expression",
            "fundamental_frequency_hz",
            "reported_harmonics",
            "harmonic",
            "frequency_hz",
            "magnitude",
            "phase_deg",
            "normalized_magnitude",
            "normalized_phase_deg",
            "thd_percent",
            "grid_size",
            "interpolation_degree",
            "periods",
        ]
    {
        return Vec::new();
    }
    let mut rows = Vec::new();
    for line in lines {
        let Some(fields) = split_csv_fields(line) else {
            continue;
        };
        if fields.len() != 13 {
            continue;
        }
        let Some(harmonic) = fields[3].parse::<u32>().ok() else {
            continue;
        };
        let Some(frequency_hz) = parse_finite_f64(&fields[4]) else {
            continue;
        };
        let Some(magnitude) = parse_finite_f64(&fields[5]) else {
            continue;
        };
        let Some(phase_deg) = parse_finite_f64(&fields[6]) else {
            continue;
        };
        let Some(normalized_magnitude) = parse_finite_f64(&fields[7]) else {
            continue;
        };
        let Some(normalized_phase_deg) = parse_finite_f64(&fields[8]) else {
            continue;
        };
        let thd_percent = match fields[9].is_empty() {
            true => None,
            false => {
                let Some(value) = parse_finite_f64(&fields[9]) else {
                    continue;
                };
                Some(value)
            }
        };
        rows.push(FourierSummary {
            artifact: artifact.to_string(),
            output_expression: fields[0].clone(),
            harmonic,
            frequency_hz,
            magnitude,
            phase_deg,
            normalized_magnitude,
            normalized_phase_deg,
            thd_percent,
        });
    }
    rows
}

pub(super) fn collect_pole_zero_summaries(artifacts: &[String]) -> Vec<PoleZeroSummary> {
    let mut records = Vec::new();
    for artifact in artifacts {
        if !artifact.ends_with("pole_zero_summary.csv") {
            continue;
        }
        let Ok(text) = fs::read_to_string(artifact) else {
            continue;
        };
        records.extend(parse_pole_zero_summary_csv(artifact, &text));
    }
    records.sort_by(|left, right| {
        left.artifact
            .cmp(&right.artifact)
            .then_with(|| left.root_kind.cmp(&right.root_kind))
            .then_with(|| left.root_index.cmp(&right.root_index))
    });
    records
}

pub(super) fn collect_sensitivity_summaries(artifacts: &[String]) -> Vec<SensitivitySummary> {
    let mut records = Vec::new();
    for artifact in artifacts {
        if !artifact.ends_with("sensitivity_summary.csv") {
            continue;
        }
        let Ok(text) = fs::read_to_string(artifact) else {
            continue;
        };
        records.extend(parse_sensitivity_summary_csv(artifact, &text));
    }
    records.sort_by(|left, right| {
        left.artifact
            .cmp(&right.artifact)
            .then_with(|| left.mode.cmp(&right.mode))
            .then_with(|| left.parameter.cmp(&right.parameter))
            .then_with(|| {
                left.frequency_hz
                    .unwrap_or(f64::NEG_INFINITY)
                    .total_cmp(&right.frequency_hz.unwrap_or(f64::NEG_INFINITY))
            })
    });
    records
}

fn parse_sensitivity_summary_csv(artifact: &str, text: &str) -> Vec<SensitivitySummary> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let Some(header) = lines.next() else {
        return Vec::new();
    };
    let Some(header) = split_csv_fields(header) else {
        return Vec::new();
    };
    if header
        != [
            "output_expression",
            "mode",
            "parameter",
            "frequency_hz",
            "sensitivity_real",
            "sensitivity_imaginary",
            "sensitivity_magnitude",
        ]
    {
        return Vec::new();
    }
    let mut rows = Vec::new();
    for line in lines {
        let Some(fields) = split_csv_fields(line) else {
            continue;
        };
        if fields.len() != 7 || !matches!(fields[1].as_str(), "dc" | "ac") {
            continue;
        }
        let frequency_hz = if fields[3].is_empty() {
            None
        } else {
            let Some(frequency_hz) = parse_finite_f64(&fields[3]) else {
                continue;
            };
            Some(frequency_hz)
        };
        let Some(sensitivity_real) = parse_finite_f64(&fields[4]) else {
            continue;
        };
        let Some(sensitivity_imaginary) = parse_finite_f64(&fields[5]) else {
            continue;
        };
        let Some(sensitivity_magnitude) = parse_finite_f64(&fields[6]) else {
            continue;
        };
        rows.push(SensitivitySummary {
            artifact: artifact.to_string(),
            output_expression: fields[0].clone(),
            mode: fields[1].clone(),
            parameter: fields[2].clone(),
            frequency_hz,
            sensitivity_real,
            sensitivity_imaginary,
            sensitivity_magnitude,
        });
    }
    rows
}

pub(super) fn collect_transfer_function_summaries(
    artifacts: &[String],
) -> Vec<TransferFunctionSummary> {
    let mut records = Vec::new();
    for artifact in artifacts {
        if !artifact.ends_with("transfer_function_summary.csv") {
            continue;
        }
        let Ok(text) = fs::read_to_string(artifact) else {
            continue;
        };
        records.extend(parse_transfer_function_summary_csv(artifact, &text));
    }
    records.sort_by(|left, right| {
        left.artifact
            .cmp(&right.artifact)
            .then_with(|| left.output_expression.cmp(&right.output_expression))
            .then_with(|| left.input_source.cmp(&right.input_source))
    });
    records
}

fn parse_transfer_function_summary_csv(artifact: &str, text: &str) -> Vec<TransferFunctionSummary> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let Some(header) = lines.next() else {
        return Vec::new();
    };
    let Some(header) = split_csv_fields(header) else {
        return Vec::new();
    };
    if header
        != [
            "output_expression",
            "input_source",
            "transfer_function_gain",
            "input_resistance_ohm",
            "output_resistance_ohm",
        ]
    {
        return Vec::new();
    }
    let mut rows = Vec::new();
    for line in lines {
        let Some(fields) = split_csv_fields(line) else {
            continue;
        };
        if fields.len() != 5 {
            continue;
        }
        let Some(transfer_function_gain) = parse_finite_f64(&fields[2]) else {
            continue;
        };
        let Some(input_resistance_ohm) = parse_finite_f64(&fields[3]) else {
            continue;
        };
        let Some(output_resistance_ohm) = parse_finite_f64(&fields[4]) else {
            continue;
        };
        rows.push(TransferFunctionSummary {
            artifact: artifact.to_string(),
            output_expression: fields[0].clone(),
            input_source: fields[1].clone(),
            transfer_function_gain,
            input_resistance_ohm,
            output_resistance_ohm,
        });
    }
    rows
}

pub(super) fn collect_s_parameter_summaries(artifacts: &[String]) -> Vec<SParameterSummary> {
    let mut records = Vec::new();
    for artifact in artifacts {
        if !artifact.ends_with("s_parameter_summary.csv") {
            continue;
        }
        let Ok(text) = fs::read_to_string(artifact) else {
            continue;
        };
        records.extend(parse_s_parameter_summary_csv(artifact, &text));
    }
    records.sort_by(|left, right| {
        left.artifact
            .cmp(&right.artifact)
            .then_with(|| left.parameter.cmp(&right.parameter))
    });
    records
}

fn parse_s_parameter_summary_csv(artifact: &str, text: &str) -> Vec<SParameterSummary> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let Some(header) = lines.next() else {
        return Vec::new();
    };
    let Some(header) = split_csv_fields(header) else {
        return Vec::new();
    };
    if header
        != [
            "parameter",
            "row_count",
            "min_frequency_hz",
            "max_frequency_hz",
            "min_mag_db",
            "max_mag_db",
            "min_mag_linear",
            "max_mag_linear",
            "min_return_loss_db",
            "max_return_loss_db",
            "min_insertion_loss_db",
            "max_insertion_loss_db",
            "min_vswr",
            "max_vswr",
        ]
    {
        return Vec::new();
    }
    let mut rows = Vec::new();
    for line in lines {
        let Some(fields) = split_csv_fields(line) else {
            continue;
        };
        if fields.len() != 14 {
            continue;
        }
        let Some(row_count) = fields[1].parse::<usize>().ok() else {
            continue;
        };
        let Some(min_frequency_hz) = parse_finite_f64(&fields[2]) else {
            continue;
        };
        let Some(max_frequency_hz) = parse_finite_f64(&fields[3]) else {
            continue;
        };
        let Some(min_mag_db) = parse_finite_f64(&fields[4]) else {
            continue;
        };
        let Some(max_mag_db) = parse_finite_f64(&fields[5]) else {
            continue;
        };
        let Some(min_mag_linear) = parse_finite_f64(&fields[6]) else {
            continue;
        };
        let Some(max_mag_linear) = parse_finite_f64(&fields[7]) else {
            continue;
        };
        let Some(min_return_loss_db) = parse_optional_finite_f64(&fields[8]) else {
            continue;
        };
        let Some(max_return_loss_db) = parse_optional_finite_f64(&fields[9]) else {
            continue;
        };
        let Some(min_insertion_loss_db) = parse_optional_finite_f64(&fields[10]) else {
            continue;
        };
        let Some(max_insertion_loss_db) = parse_optional_finite_f64(&fields[11]) else {
            continue;
        };
        let Some(min_vswr) = parse_optional_finite_f64(&fields[12]) else {
            continue;
        };
        let Some(max_vswr) = parse_optional_finite_f64(&fields[13]) else {
            continue;
        };
        rows.push(SParameterSummary {
            artifact: artifact.to_string(),
            parameter: fields[0].clone(),
            row_count,
            min_frequency_hz,
            max_frequency_hz,
            min_mag_db,
            max_mag_db,
            min_mag_linear,
            max_mag_linear,
            min_return_loss_db,
            max_return_loss_db,
            min_insertion_loss_db,
            max_insertion_loss_db,
            min_vswr,
            max_vswr,
        });
    }
    rows
}

fn parse_pole_zero_summary_csv(artifact: &str, text: &str) -> Vec<PoleZeroSummary> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let Some(header) = lines.next() else {
        return Vec::new();
    };
    let Some(header) = split_csv_fields(header) else {
        return Vec::new();
    };
    if header
        != [
            "output_node",
            "reference_node",
            "input_source",
            "mode",
            "root_kind",
            "root_index",
            "real_rad_per_s",
            "imaginary_rad_per_s",
            "frequency_hz",
        ]
    {
        return Vec::new();
    }
    let mut rows = Vec::new();
    for line in lines {
        let Some(fields) = split_csv_fields(line) else {
            continue;
        };
        if fields.len() != 9 {
            continue;
        }
        if !matches!(fields[4].as_str(), "pole" | "zero") {
            continue;
        }
        let Some(root_index) = fields[5].parse::<u32>().ok() else {
            continue;
        };
        let Some(real_rad_per_s) = parse_finite_f64(&fields[6]) else {
            continue;
        };
        let Some(imaginary_rad_per_s) = parse_finite_f64(&fields[7]) else {
            continue;
        };
        let Some(frequency_hz) = parse_finite_f64(&fields[8]) else {
            continue;
        };
        rows.push(PoleZeroSummary {
            artifact: artifact.to_string(),
            output_node: fields[0].clone(),
            reference_node: fields[1].clone(),
            input_source: fields[2].clone(),
            mode: fields[3].clone(),
            root_kind: fields[4].clone(),
            root_index,
            real_rad_per_s,
            imaginary_rad_per_s,
            frequency_hz,
        });
    }
    rows
}

fn split_csv_fields(line: &str) -> Option<Vec<String>> {
    let mut fields = Vec::new();
    let mut field = String::new();
    let mut chars = line.chars().peekable();
    let mut quoted = false;
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                field.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => {
                fields.push(field.trim().to_string());
                field = String::new();
            }
            _ => field.push(ch),
        }
    }
    if quoted {
        return None;
    }
    fields.push(field.trim().to_string());
    Some(fields)
}

fn parse_finite_f64(value: &str) -> Option<f64> {
    let value = value.parse::<f64>().ok()?;
    value.is_finite().then_some(value)
}

fn parse_optional_finite_f64(value: &str) -> Option<Option<f64>> {
    if value.is_empty() {
        return Some(None);
    }
    parse_finite_f64(value).map(Some)
}
