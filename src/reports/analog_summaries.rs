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
pub struct HarmonicBalanceSummary {
    pub artifact: String,
    pub output_expression: String,
    pub fundamental_frequency_hz: f64,
    pub harmonic: i64,
    pub frequency_hz: f64,
    pub real: f64,
    pub imaginary: f64,
    pub magnitude: f64,
    pub phase_deg: f64,
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
    pub min_mismatch_loss_db: Option<f64>,
    pub max_mismatch_loss_db: Option<f64>,
    pub min_group_delay_s: Option<f64>,
    pub max_group_delay_s: Option<f64>,
    pub min_impedance_real_ohm: Option<f64>,
    pub max_impedance_real_ohm: Option<f64>,
    pub min_impedance_imag_ohm: Option<f64>,
    pub max_impedance_imag_ohm: Option<f64>,
    pub min_impedance_magnitude_ohm: Option<f64>,
    pub max_impedance_magnitude_ohm: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SParameterNetworkSummary {
    pub artifact: String,
    pub port_count: usize,
    pub row_count: usize,
    pub min_frequency_hz: f64,
    pub max_frequency_hz: f64,
    pub max_reciprocity_error_linear: f64,
    pub frequency_hz_at_max_reciprocity_error: f64,
    pub max_passivity_singular_value: f64,
    pub frequency_hz_at_max_passivity: f64,
    pub min_rollet_k: Option<f64>,
    pub frequency_hz_at_min_rollet_k: Option<f64>,
    pub max_stability_delta_magnitude: Option<f64>,
    pub frequency_hz_at_max_stability_delta_magnitude: Option<f64>,
    pub min_maximum_available_gain_db: Option<f64>,
    pub frequency_hz_at_min_maximum_available_gain: Option<f64>,
    pub min_maximum_stable_gain_db: Option<f64>,
    pub frequency_hz_at_min_maximum_stable_gain: Option<f64>,
    pub min_maximum_unilateral_gain_db: Option<f64>,
    pub frequency_hz_at_min_maximum_unilateral_gain: Option<f64>,
    pub source_reflection_real: Option<f64>,
    pub source_reflection_imaginary: Option<f64>,
    pub load_reflection_real: Option<f64>,
    pub load_reflection_imaginary: Option<f64>,
    pub min_transducer_gain_db: Option<f64>,
    pub frequency_hz_at_min_transducer_gain: Option<f64>,
    pub min_available_gain_db: Option<f64>,
    pub frequency_hz_at_min_available_gain: Option<f64>,
    pub min_operating_gain_db: Option<f64>,
    pub frequency_hz_at_min_operating_gain: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SParameterNoiseSummary {
    pub artifact: String,
    pub row_count: usize,
    pub min_frequency_hz: f64,
    pub max_frequency_hz: f64,
    pub max_noise_figure_db: f64,
    pub frequency_hz_at_max_noise_figure: f64,
    pub max_minimum_noise_figure_db: f64,
    pub frequency_hz_at_max_minimum_noise_figure: f64,
    pub max_equivalent_noise_resistance_ohm: f64,
    pub frequency_hz_at_max_equivalent_noise_resistance: f64,
    pub max_optimum_source_reflection_magnitude: f64,
    pub frequency_hz_at_max_optimum_source_reflection_magnitude: f64,
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

pub(super) fn collect_harmonic_balance_summaries(
    artifacts: &[String],
) -> Vec<HarmonicBalanceSummary> {
    let mut records = Vec::new();
    for artifact in artifacts {
        if !artifact.ends_with("hb_spectrum.csv") {
            continue;
        }
        let Ok(text) = fs::read_to_string(artifact) else {
            continue;
        };
        records.extend(parse_hb_spectrum_csv(artifact, &text));
    }
    records.sort_by(|left, right| {
        left.artifact
            .cmp(&right.artifact)
            .then_with(|| left.output_expression.cmp(&right.output_expression))
            .then_with(|| left.harmonic.cmp(&right.harmonic))
    });
    records
}

pub(super) fn render_harmonic_balance_summary_markdown(rows: &[HarmonicBalanceSummary]) -> String {
    let mut text = String::from("## Harmonic Balance Summary\n\n");
    if rows.is_empty() {
        text.push_str("None.\n\n");
        return text;
    }
    for row in rows {
        text.push_str(&format!(
            "- `{}` h{}: fundamental={:.6e} Hz frequency={:.6e} Hz magnitude={:.6e} phase={:.6e} deg real={:.6e} imaginary={:.6e}\n",
            row.output_expression,
            row.harmonic,
            row.fundamental_frequency_hz,
            row.frequency_hz,
            row.magnitude,
            row.phase_deg,
            row.real,
            row.imaginary
        ));
        text.push_str(&format!("  - Artifact: `{}`\n", row.artifact));
    }
    text.push('\n');
    text
}

fn parse_hb_spectrum_csv(artifact: &str, text: &str) -> Vec<HarmonicBalanceSummary> {
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
            "harmonic",
            "frequency_hz",
            "real",
            "imaginary",
            "magnitude",
            "phase_deg",
        ]
    {
        return Vec::new();
    }
    let mut rows = Vec::new();
    for line in lines {
        let Some(fields) = split_csv_fields(line) else {
            continue;
        };
        if fields.len() != 8 {
            continue;
        }
        let Some(fundamental_frequency_hz) = parse_finite_f64(&fields[1]) else {
            continue;
        };
        let Some(harmonic) = fields[2].parse::<i64>().ok() else {
            continue;
        };
        let Some(frequency_hz) = parse_finite_f64(&fields[3]) else {
            continue;
        };
        let Some(real) = parse_finite_f64(&fields[4]) else {
            continue;
        };
        let Some(imaginary) = parse_finite_f64(&fields[5]) else {
            continue;
        };
        let Some(magnitude) = parse_finite_f64(&fields[6]) else {
            continue;
        };
        let Some(phase_deg) = parse_finite_f64(&fields[7]) else {
            continue;
        };
        rows.push(HarmonicBalanceSummary {
            artifact: artifact.to_string(),
            output_expression: fields[0].clone(),
            fundamental_frequency_hz,
            harmonic,
            frequency_hz,
            real,
            imaginary,
            magnitude,
            phase_deg,
        });
    }
    rows
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

pub(super) fn collect_s_parameter_network_summaries(
    artifacts: &[String],
) -> Vec<SParameterNetworkSummary> {
    let mut records = Vec::new();
    for artifact in artifacts {
        if !artifact.ends_with("s_parameter_network_summary.csv") {
            continue;
        }
        let Ok(text) = fs::read_to_string(artifact) else {
            continue;
        };
        records.extend(parse_s_parameter_network_summary_csv(artifact, &text));
    }
    records.sort_by(|left, right| left.artifact.cmp(&right.artifact));
    records
}

pub(super) fn collect_s_parameter_noise_summaries(
    artifacts: &[String],
) -> Vec<SParameterNoiseSummary> {
    let mut records = Vec::new();
    for artifact in artifacts {
        if !artifact.ends_with("s_parameter_noise_summary.csv") {
            continue;
        }
        let Ok(text) = fs::read_to_string(artifact) else {
            continue;
        };
        records.extend(parse_s_parameter_noise_summary_csv(artifact, &text));
    }
    records.sort_by(|left, right| left.artifact.cmp(&right.artifact));
    records
}

fn parse_s_parameter_network_summary_csv(
    artifact: &str,
    text: &str,
) -> Vec<SParameterNetworkSummary> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let Some(header) = lines.next() else {
        return Vec::new();
    };
    let Some(header) = split_csv_fields(header) else {
        return Vec::new();
    };
    if header
        != [
            "port_count",
            "row_count",
            "min_frequency_hz",
            "max_frequency_hz",
            "max_reciprocity_error_linear",
            "frequency_hz_at_max_reciprocity_error",
            "max_passivity_singular_value",
            "frequency_hz_at_max_passivity",
            "min_rollet_k",
            "frequency_hz_at_min_rollet_k",
            "max_stability_delta_magnitude",
            "frequency_hz_at_max_stability_delta_magnitude",
            "min_maximum_available_gain_db",
            "frequency_hz_at_min_maximum_available_gain",
            "min_maximum_stable_gain_db",
            "frequency_hz_at_min_maximum_stable_gain",
            "min_maximum_unilateral_gain_db",
            "frequency_hz_at_min_maximum_unilateral_gain",
            "source_reflection_real",
            "source_reflection_imaginary",
            "load_reflection_real",
            "load_reflection_imaginary",
            "min_transducer_gain_db",
            "frequency_hz_at_min_transducer_gain",
            "min_available_gain_db",
            "frequency_hz_at_min_available_gain",
            "min_operating_gain_db",
            "frequency_hz_at_min_operating_gain",
        ]
    {
        return Vec::new();
    }
    let mut rows = Vec::new();
    for line in lines {
        let Some(fields) = split_csv_fields(line) else {
            continue;
        };
        if fields.len() != 28 {
            continue;
        }
        let Some(port_count) = fields[0].parse::<usize>().ok() else {
            continue;
        };
        let Some(row_count) = fields[1].parse::<usize>().ok() else {
            continue;
        };
        let Some(min_frequency_hz) = parse_finite_f64(&fields[2]) else {
            continue;
        };
        let Some(max_frequency_hz) = parse_finite_f64(&fields[3]) else {
            continue;
        };
        let Some(max_reciprocity_error_linear) = parse_finite_f64(&fields[4]) else {
            continue;
        };
        let Some(frequency_hz_at_max_reciprocity_error) = parse_finite_f64(&fields[5]) else {
            continue;
        };
        let Some(max_passivity_singular_value) = parse_finite_f64(&fields[6]) else {
            continue;
        };
        let Some(frequency_hz_at_max_passivity) = parse_finite_f64(&fields[7]) else {
            continue;
        };
        let Some(min_rollet_k) = parse_optional_finite_f64(&fields[8]) else {
            continue;
        };
        let Some(frequency_hz_at_min_rollet_k) = parse_optional_finite_f64(&fields[9]) else {
            continue;
        };
        let Some(max_stability_delta_magnitude) = parse_optional_finite_f64(&fields[10]) else {
            continue;
        };
        let Some(frequency_hz_at_max_stability_delta_magnitude) =
            parse_optional_finite_f64(&fields[11])
        else {
            continue;
        };
        let Some(min_maximum_available_gain_db) = parse_optional_finite_f64(&fields[12]) else {
            continue;
        };
        let Some(frequency_hz_at_min_maximum_available_gain) =
            parse_optional_finite_f64(&fields[13])
        else {
            continue;
        };
        let Some(min_maximum_stable_gain_db) = parse_optional_finite_f64(&fields[14]) else {
            continue;
        };
        let Some(frequency_hz_at_min_maximum_stable_gain) = parse_optional_finite_f64(&fields[15])
        else {
            continue;
        };
        let Some(min_maximum_unilateral_gain_db) = parse_optional_finite_f64(&fields[16]) else {
            continue;
        };
        let Some(frequency_hz_at_min_maximum_unilateral_gain) =
            parse_optional_finite_f64(&fields[17])
        else {
            continue;
        };
        let Some(source_reflection_real) = parse_optional_finite_f64(&fields[18]) else {
            continue;
        };
        let Some(source_reflection_imaginary) = parse_optional_finite_f64(&fields[19]) else {
            continue;
        };
        let Some(load_reflection_real) = parse_optional_finite_f64(&fields[20]) else {
            continue;
        };
        let Some(load_reflection_imaginary) = parse_optional_finite_f64(&fields[21]) else {
            continue;
        };
        let Some(min_transducer_gain_db) = parse_optional_finite_f64(&fields[22]) else {
            continue;
        };
        let Some(frequency_hz_at_min_transducer_gain) = parse_optional_finite_f64(&fields[23])
        else {
            continue;
        };
        let Some(min_available_gain_db) = parse_optional_finite_f64(&fields[24]) else {
            continue;
        };
        let Some(frequency_hz_at_min_available_gain) = parse_optional_finite_f64(&fields[25])
        else {
            continue;
        };
        let Some(min_operating_gain_db) = parse_optional_finite_f64(&fields[26]) else {
            continue;
        };
        let Some(frequency_hz_at_min_operating_gain) = parse_optional_finite_f64(&fields[27])
        else {
            continue;
        };
        rows.push(SParameterNetworkSummary {
            artifact: artifact.to_string(),
            port_count,
            row_count,
            min_frequency_hz,
            max_frequency_hz,
            max_reciprocity_error_linear,
            frequency_hz_at_max_reciprocity_error,
            max_passivity_singular_value,
            frequency_hz_at_max_passivity,
            min_rollet_k,
            frequency_hz_at_min_rollet_k,
            max_stability_delta_magnitude,
            frequency_hz_at_max_stability_delta_magnitude,
            min_maximum_available_gain_db,
            frequency_hz_at_min_maximum_available_gain,
            min_maximum_stable_gain_db,
            frequency_hz_at_min_maximum_stable_gain,
            min_maximum_unilateral_gain_db,
            frequency_hz_at_min_maximum_unilateral_gain,
            source_reflection_real,
            source_reflection_imaginary,
            load_reflection_real,
            load_reflection_imaginary,
            min_transducer_gain_db,
            frequency_hz_at_min_transducer_gain,
            min_available_gain_db,
            frequency_hz_at_min_available_gain,
            min_operating_gain_db,
            frequency_hz_at_min_operating_gain,
        });
    }
    rows
}

fn parse_s_parameter_noise_summary_csv(artifact: &str, text: &str) -> Vec<SParameterNoiseSummary> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let Some(header) = lines.next() else {
        return Vec::new();
    };
    let Some(header) = split_csv_fields(header) else {
        return Vec::new();
    };
    if header
        != [
            "row_count",
            "min_frequency_hz",
            "max_frequency_hz",
            "max_noise_figure_db",
            "frequency_hz_at_max_noise_figure",
            "max_minimum_noise_figure_db",
            "frequency_hz_at_max_minimum_noise_figure",
            "max_equivalent_noise_resistance_ohm",
            "frequency_hz_at_max_equivalent_noise_resistance",
            "max_optimum_source_reflection_magnitude",
            "frequency_hz_at_max_optimum_source_reflection_magnitude",
        ]
    {
        return Vec::new();
    }
    let mut rows = Vec::new();
    for line in lines {
        let Some(fields) = split_csv_fields(line) else {
            continue;
        };
        if fields.len() != 11 {
            continue;
        }
        let Some(row_count) = fields[0].parse::<usize>().ok() else {
            continue;
        };
        let Some(min_frequency_hz) = parse_finite_f64(&fields[1]) else {
            continue;
        };
        let Some(max_frequency_hz) = parse_finite_f64(&fields[2]) else {
            continue;
        };
        let Some(max_noise_figure_db) = parse_finite_f64(&fields[3]) else {
            continue;
        };
        let Some(frequency_hz_at_max_noise_figure) = parse_finite_f64(&fields[4]) else {
            continue;
        };
        let Some(max_minimum_noise_figure_db) = parse_finite_f64(&fields[5]) else {
            continue;
        };
        let Some(frequency_hz_at_max_minimum_noise_figure) = parse_finite_f64(&fields[6]) else {
            continue;
        };
        let Some(max_equivalent_noise_resistance_ohm) = parse_finite_f64(&fields[7]) else {
            continue;
        };
        let Some(frequency_hz_at_max_equivalent_noise_resistance) = parse_finite_f64(&fields[8])
        else {
            continue;
        };
        let Some(max_optimum_source_reflection_magnitude) = parse_finite_f64(&fields[9]) else {
            continue;
        };
        let Some(frequency_hz_at_max_optimum_source_reflection_magnitude) =
            parse_finite_f64(&fields[10])
        else {
            continue;
        };
        rows.push(SParameterNoiseSummary {
            artifact: artifact.to_string(),
            row_count,
            min_frequency_hz,
            max_frequency_hz,
            max_noise_figure_db,
            frequency_hz_at_max_noise_figure,
            max_minimum_noise_figure_db,
            frequency_hz_at_max_minimum_noise_figure,
            max_equivalent_noise_resistance_ohm,
            frequency_hz_at_max_equivalent_noise_resistance,
            max_optimum_source_reflection_magnitude,
            frequency_hz_at_max_optimum_source_reflection_magnitude,
        });
    }
    rows
}

pub(super) fn render_s_parameter_network_summary_markdown(
    row: &SParameterNetworkSummary,
) -> String {
    format!(
        "- ports={} rows={} frequency={:.6e}..{:.6e} Hz max_reciprocity_error={:.6e} at {:.6e} Hz max_passivity_singular_value={:.6e} at {:.6e} Hz min_rollet_k={} at {} Hz max_stability_delta_magnitude={} at {} Hz min_maximum_available_gain_db={} at {} Hz min_maximum_stable_gain_db={} at {} Hz min_maximum_unilateral_gain_db={} at {} Hz min_transducer_gain_db={} at {} Hz min_available_gain_db={} at {} Hz min_operating_gain_db={} at {} Hz source_reflection=({}, {}) load_reflection=({}, {})\n  - Artifact: `{}`\n",
        row.port_count,
        row.row_count,
        row.min_frequency_hz,
        row.max_frequency_hz,
        row.max_reciprocity_error_linear,
        row.frequency_hz_at_max_reciprocity_error,
        row.max_passivity_singular_value,
        row.frequency_hz_at_max_passivity,
        format_optional_value(row.min_rollet_k),
        format_optional_value(row.frequency_hz_at_min_rollet_k),
        format_optional_value(row.max_stability_delta_magnitude),
        format_optional_value(row.frequency_hz_at_max_stability_delta_magnitude),
        format_optional_value(row.min_maximum_available_gain_db),
        format_optional_value(row.frequency_hz_at_min_maximum_available_gain),
        format_optional_value(row.min_maximum_stable_gain_db),
        format_optional_value(row.frequency_hz_at_min_maximum_stable_gain),
        format_optional_value(row.min_maximum_unilateral_gain_db),
        format_optional_value(row.frequency_hz_at_min_maximum_unilateral_gain),
        format_optional_value(row.min_transducer_gain_db),
        format_optional_value(row.frequency_hz_at_min_transducer_gain),
        format_optional_value(row.min_available_gain_db),
        format_optional_value(row.frequency_hz_at_min_available_gain),
        format_optional_value(row.min_operating_gain_db),
        format_optional_value(row.frequency_hz_at_min_operating_gain),
        format_optional_value(row.source_reflection_real),
        format_optional_value(row.source_reflection_imaginary),
        format_optional_value(row.load_reflection_real),
        format_optional_value(row.load_reflection_imaginary),
        row.artifact
    )
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
            "min_mismatch_loss_db",
            "max_mismatch_loss_db",
            "min_group_delay_s",
            "max_group_delay_s",
            "min_impedance_real_ohm",
            "max_impedance_real_ohm",
            "min_impedance_imag_ohm",
            "max_impedance_imag_ohm",
            "min_impedance_magnitude_ohm",
            "max_impedance_magnitude_ohm",
        ]
    {
        return Vec::new();
    }
    let mut rows = Vec::new();
    for line in lines {
        let Some(fields) = split_csv_fields(line) else {
            continue;
        };
        if fields.len() != 24 {
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
        let Some(min_mismatch_loss_db) = parse_optional_finite_f64(&fields[14]) else {
            continue;
        };
        let Some(max_mismatch_loss_db) = parse_optional_finite_f64(&fields[15]) else {
            continue;
        };
        let Some(min_group_delay_s) = parse_optional_finite_f64(&fields[16]) else {
            continue;
        };
        let Some(max_group_delay_s) = parse_optional_finite_f64(&fields[17]) else {
            continue;
        };
        let Some(min_impedance_real_ohm) = parse_optional_finite_f64(&fields[18]) else {
            continue;
        };
        let Some(max_impedance_real_ohm) = parse_optional_finite_f64(&fields[19]) else {
            continue;
        };
        let Some(min_impedance_imag_ohm) = parse_optional_finite_f64(&fields[20]) else {
            continue;
        };
        let Some(max_impedance_imag_ohm) = parse_optional_finite_f64(&fields[21]) else {
            continue;
        };
        let Some(min_impedance_magnitude_ohm) = parse_optional_finite_f64(&fields[22]) else {
            continue;
        };
        let Some(max_impedance_magnitude_ohm) = parse_optional_finite_f64(&fields[23]) else {
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
            min_mismatch_loss_db,
            max_mismatch_loss_db,
            min_group_delay_s,
            max_group_delay_s,
            min_impedance_real_ohm,
            max_impedance_real_ohm,
            min_impedance_imag_ohm,
            max_impedance_imag_ohm,
            min_impedance_magnitude_ohm,
            max_impedance_magnitude_ohm,
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

fn format_optional_value(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.6e}"))
        .unwrap_or_else(|| "unavailable".to_string())
}
