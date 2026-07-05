use crate::board_ir::{AnalogSParameterReflectionCoefficient, AnalogScenario};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
struct SParameterSample {
    frequency_hz: f64,
    mag_db: f64,
    mag_linear: f64,
    phase_deg: f64,
    reference_impedance_ohm: f64,
}

#[derive(Debug, Clone)]
pub(super) struct SParameterSummaryRow {
    pub(super) parameter: String,
    pub(super) row_count: usize,
    pub(super) min_frequency_hz: f64,
    pub(super) max_frequency_hz: f64,
    pub(super) min_mag_db: f64,
    pub(super) max_mag_db: f64,
    pub(super) min_mag_linear: f64,
    pub(super) max_mag_linear: f64,
    pub(super) min_return_loss_db: Option<f64>,
    pub(super) max_return_loss_db: Option<f64>,
    pub(super) min_insertion_loss_db: Option<f64>,
    pub(super) max_insertion_loss_db: Option<f64>,
    pub(super) min_vswr: Option<f64>,
    pub(super) max_vswr: Option<f64>,
    pub(super) min_mismatch_loss_db: Option<f64>,
    pub(super) max_mismatch_loss_db: Option<f64>,
    pub(super) min_group_delay_s: Option<f64>,
    pub(super) max_group_delay_s: Option<f64>,
    pub(super) min_impedance_real_ohm: Option<f64>,
    pub(super) max_impedance_real_ohm: Option<f64>,
    pub(super) min_impedance_imag_ohm: Option<f64>,
    pub(super) max_impedance_imag_ohm: Option<f64>,
    pub(super) min_impedance_magnitude_ohm: Option<f64>,
    pub(super) max_impedance_magnitude_ohm: Option<f64>,
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
pub(super) struct SParameterNetworkSummaryRow {
    pub(super) port_count: usize,
    pub(super) row_count: usize,
    pub(super) min_frequency_hz: f64,
    pub(super) max_frequency_hz: f64,
    pub(super) max_reciprocity_error_linear: f64,
    pub(super) frequency_hz_at_max_reciprocity_error: f64,
    pub(super) max_passivity_singular_value: f64,
    pub(super) frequency_hz_at_max_passivity: f64,
    pub(super) min_rollet_k: Option<f64>,
    pub(super) frequency_hz_at_min_rollet_k: Option<f64>,
    pub(super) max_stability_delta_magnitude: Option<f64>,
    pub(super) frequency_hz_at_max_stability_delta_magnitude: Option<f64>,
    pub(super) min_maximum_available_gain_db: Option<f64>,
    pub(super) frequency_hz_at_min_maximum_available_gain: Option<f64>,
    pub(super) min_maximum_stable_gain_db: Option<f64>,
    pub(super) frequency_hz_at_min_maximum_stable_gain: Option<f64>,
    pub(super) min_maximum_unilateral_gain_db: Option<f64>,
    pub(super) frequency_hz_at_min_maximum_unilateral_gain: Option<f64>,
    pub(super) source_reflection_real: Option<f64>,
    pub(super) source_reflection_imaginary: Option<f64>,
    pub(super) load_reflection_real: Option<f64>,
    pub(super) load_reflection_imaginary: Option<f64>,
    pub(super) min_transducer_gain_db: Option<f64>,
    pub(super) frequency_hz_at_min_transducer_gain: Option<f64>,
    pub(super) min_available_gain_db: Option<f64>,
    pub(super) frequency_hz_at_min_available_gain: Option<f64>,
    pub(super) min_operating_gain_db: Option<f64>,
    pub(super) frequency_hz_at_min_operating_gain: Option<f64>,
}

pub(super) fn summarize_s_parameters(path: &Path) -> Result<Vec<SParameterSummaryRow>, String> {
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
    let reference_impedance_index = header
        .iter()
        .position(|candidate| *candidate == "reference_impedance_ohm");
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
        let Some(phase_index) = header
            .iter()
            .position(|candidate| *candidate == format!("{parameter}_phase_deg"))
        else {
            return Err(format!(
                "S-parameter CSV is missing {parameter}_phase_deg for {parameter}_mag_db."
            ));
        };
        parameter_columns.push((parameter.to_string(), index, mag_linear_index, phase_index));
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
        let reference_impedance_ohm = if let Some(index) = reference_impedance_index {
            let value = parse_finite_f64(fields[index], "reference_impedance_ohm")?;
            if value <= 0.0 {
                return Err(format!(
                    "S-parameter CSV row {} has non-positive reference impedance {}.",
                    line_index + 2,
                    value
                ));
            }
            value
        } else {
            50.0
        };
        for (parameter, mag_db_index, mag_linear_index, phase_index) in &parameter_columns {
            let mag_db = parse_finite_f64(fields[*mag_db_index], parameter)?;
            let mag_linear = parse_finite_f64(fields[*mag_linear_index], parameter)?;
            if mag_linear < 0.0 {
                return Err(format!(
                    "S-parameter CSV row {} has negative linear magnitude for {parameter}.",
                    line_index + 2
                ));
            }
            let phase_deg = parse_finite_f64(fields[*phase_index], parameter)?;
            samples
                .entry(parameter.clone())
                .or_default()
                .push(SParameterSample {
                    frequency_hz,
                    mag_db,
                    mag_linear,
                    phase_deg,
                    reference_impedance_ohm,
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
    let mut mismatch_losses = Vec::new();
    let mut impedance_real = Vec::new();
    let mut impedance_imag = Vec::new();
    let mut impedance_magnitude = Vec::new();
    let group_delays = group_delay_values_s(values);
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
                let delivered_power_ratio = 1.0 - sample.mag_linear * sample.mag_linear;
                if delivered_power_ratio > 0.0 {
                    mismatch_losses.push(-10.0 * delivered_power_ratio.log10());
                }
            }
            if let Some(impedance) = reflection_impedance_ohm(sample) {
                impedance_real.push(impedance.real);
                impedance_imag.push(impedance.imaginary);
                impedance_magnitude.push(impedance.magnitude());
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
        min_mismatch_loss_db: finite_min(&mismatch_losses),
        max_mismatch_loss_db: finite_max(&mismatch_losses),
        min_group_delay_s: group_delays.as_deref().and_then(finite_min),
        max_group_delay_s: group_delays.as_deref().and_then(finite_max),
        min_impedance_real_ohm: finite_min(&impedance_real),
        max_impedance_real_ohm: finite_max(&impedance_real),
        min_impedance_imag_ohm: finite_min(&impedance_imag),
        max_impedance_imag_ohm: finite_max(&impedance_imag),
        min_impedance_magnitude_ohm: finite_min(&impedance_magnitude),
        max_impedance_magnitude_ohm: finite_max(&impedance_magnitude),
    }
}

fn reflection_impedance_ohm(sample: &SParameterSample) -> Option<ComplexValue> {
    let gamma = ComplexValue::from_polar_degrees(sample.mag_linear, sample.phase_deg);
    let numerator = ComplexValue {
        real: 1.0 + gamma.real,
        imaginary: gamma.imaginary,
    };
    let denominator = ComplexValue {
        real: 1.0 - gamma.real,
        imaginary: -gamma.imaginary,
    };
    let impedance = numerator
        .divide(denominator)?
        .scale(sample.reference_impedance_ohm);
    impedance
        .real
        .is_finite()
        .then_some(impedance)
        .filter(|value| value.imaginary.is_finite())
}

fn group_delay_values_s(values: &[SParameterSample]) -> Option<Vec<f64>> {
    if values.len() < 2 {
        return None;
    }
    let mut phase_rad = Vec::with_capacity(values.len());
    let mut previous = None;
    let mut offset = 0.0;
    for (index, sample) in values.iter().enumerate() {
        if index > 0 && sample.frequency_hz <= values[index - 1].frequency_hz {
            return None;
        }
        let raw = sample.phase_deg.to_radians();
        if let Some(previous_unwrapped) = previous {
            let mut delta = raw + offset - previous_unwrapped;
            while delta > std::f64::consts::PI {
                offset -= std::f64::consts::TAU;
                delta -= std::f64::consts::TAU;
            }
            while delta < -std::f64::consts::PI {
                offset += std::f64::consts::TAU;
                delta += std::f64::consts::TAU;
            }
        }
        let unwrapped = raw + offset;
        previous = Some(unwrapped);
        phase_rad.push(unwrapped);
    }
    let mut delays = Vec::with_capacity(values.len());
    for index in 0..values.len() {
        let (left, right) = if index == 0 {
            (0, 1)
        } else if index + 1 == values.len() {
            (index - 1, index)
        } else {
            (index - 1, index + 1)
        };
        let omega_span =
            std::f64::consts::TAU * (values[right].frequency_hz - values[left].frequency_hz);
        if !omega_span.is_finite() || omega_span <= 0.0 {
            return None;
        }
        delays.push(-(phase_rad[right] - phase_rad[left]) / omega_span);
    }
    Some(delays)
}

pub(super) fn summarize_s_parameter_network(
    path: &Path,
    analog: &AnalogScenario,
) -> Result<SParameterNetworkSummaryRow, String> {
    let samples = read_s_parameter_network_samples(path)?;
    let source_reflection = analog
        .analysis
        .s_parameter_source_reflection
        .map(Into::into);
    let load_reflection = analog.analysis.s_parameter_load_reflection.map(Into::into);
    let mut min_frequency_hz = f64::INFINITY;
    let mut max_frequency_hz = f64::NEG_INFINITY;
    let mut max_reciprocity_error_linear = f64::NEG_INFINITY;
    let mut frequency_hz_at_max_reciprocity_error = f64::NAN;
    let mut max_passivity_singular_value = f64::NEG_INFINITY;
    let mut frequency_hz_at_max_passivity = f64::NAN;
    let mut min_rollet_k: Option<f64> = None;
    let mut frequency_hz_at_min_rollet_k: Option<f64> = None;
    let mut max_stability_delta_magnitude: Option<f64> = None;
    let mut frequency_hz_at_max_stability_delta_magnitude: Option<f64> = None;
    let mut min_maximum_available_gain_db: Option<f64> = None;
    let mut frequency_hz_at_min_maximum_available_gain: Option<f64> = None;
    let mut min_maximum_stable_gain_db: Option<f64> = None;
    let mut frequency_hz_at_min_maximum_stable_gain: Option<f64> = None;
    let mut min_maximum_unilateral_gain_db: Option<f64> = None;
    let mut frequency_hz_at_min_maximum_unilateral_gain: Option<f64> = None;
    let mut min_transducer_gain_db: Option<f64> = None;
    let mut frequency_hz_at_min_transducer_gain: Option<f64> = None;
    let mut min_available_gain_db: Option<f64> = None;
    let mut frequency_hz_at_min_available_gain: Option<f64> = None;
    let mut min_operating_gain_db: Option<f64> = None;
    let mut frequency_hz_at_min_operating_gain: Option<f64> = None;
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
        let delta = stability_delta(sample);
        let delta_magnitude = delta.magnitude();
        if max_stability_delta_magnitude.is_none_or(|current| delta_magnitude > current) {
            max_stability_delta_magnitude = Some(delta_magnitude);
            frequency_hz_at_max_stability_delta_magnitude = Some(sample.frequency_hz);
        }
        if let Some(rollet_k) = rollet_stability_factor(sample, delta_magnitude)
            && min_rollet_k.is_none_or(|current| rollet_k < current)
        {
            min_rollet_k = Some(rollet_k);
            frequency_hz_at_min_rollet_k = Some(sample.frequency_hz);
        }
        if let Some(gain_db) = maximum_available_gain_db(sample, delta_magnitude)
            && min_maximum_available_gain_db.is_none_or(|current| gain_db < current)
        {
            min_maximum_available_gain_db = Some(gain_db);
            frequency_hz_at_min_maximum_available_gain = Some(sample.frequency_hz);
        }
        if let Some(gain_db) = maximum_stable_gain_db(sample)
            && min_maximum_stable_gain_db.is_none_or(|current| gain_db < current)
        {
            min_maximum_stable_gain_db = Some(gain_db);
            frequency_hz_at_min_maximum_stable_gain = Some(sample.frequency_hz);
        }
        if let Some(gain_db) = maximum_unilateral_gain_db(sample)
            && min_maximum_unilateral_gain_db.is_none_or(|current| gain_db < current)
        {
            min_maximum_unilateral_gain_db = Some(gain_db);
            frequency_hz_at_min_maximum_unilateral_gain = Some(sample.frequency_hz);
        }
        if let (Some(source), Some(load)) = (source_reflection, load_reflection)
            && let Some(gain_db) = transducer_gain_db(sample, source, load)
            && min_transducer_gain_db.is_none_or(|current| gain_db < current)
        {
            min_transducer_gain_db = Some(gain_db);
            frequency_hz_at_min_transducer_gain = Some(sample.frequency_hz);
        }
        if let Some(source) = source_reflection
            && let Some(gain_db) = available_gain_db(sample, source)
            && min_available_gain_db.is_none_or(|current| gain_db < current)
        {
            min_available_gain_db = Some(gain_db);
            frequency_hz_at_min_available_gain = Some(sample.frequency_hz);
        }
        if let Some(load) = load_reflection
            && let Some(gain_db) = operating_gain_db(sample, load)
            && min_operating_gain_db.is_none_or(|current| gain_db < current)
        {
            min_operating_gain_db = Some(gain_db);
            frequency_hz_at_min_operating_gain = Some(sample.frequency_hz);
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
        source_reflection_real: source_reflection.map(|value: ComplexValue| value.real),
        source_reflection_imaginary: source_reflection.map(|value: ComplexValue| value.imaginary),
        load_reflection_real: load_reflection.map(|value: ComplexValue| value.real),
        load_reflection_imaginary: load_reflection.map(|value: ComplexValue| value.imaginary),
        min_transducer_gain_db,
        frequency_hz_at_min_transducer_gain,
        min_available_gain_db,
        frequency_hz_at_min_available_gain,
        min_operating_gain_db,
        frequency_hz_at_min_operating_gain,
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
    fn one() -> Self {
        Self {
            real: 1.0,
            imaginary: 0.0,
        }
    }

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

    fn divide(self, other: Self) -> Option<Self> {
        let denominator = other.magnitude_squared();
        if !denominator.is_finite() || denominator <= f64::EPSILON {
            return None;
        }
        Some(Self {
            real: (self.real * other.real + self.imaginary * other.imaginary) / denominator,
            imaginary: (self.imaginary * other.real - self.real * other.imaginary) / denominator,
        })
    }

    fn scale(self, factor: f64) -> Self {
        Self {
            real: self.real * factor,
            imaginary: self.imaginary * factor,
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

impl From<AnalogSParameterReflectionCoefficient> for ComplexValue {
    fn from(value: AnalogSParameterReflectionCoefficient) -> Self {
        Self {
            real: value.real,
            imaginary: value.imaginary,
        }
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

fn stability_delta(sample: &SParameterNetworkSample) -> ComplexValue {
    sample
        .s11
        .multiply(sample.s22)
        .subtract(sample.s12.multiply(sample.s21))
}

fn rollet_stability_factor(sample: &SParameterNetworkSample, delta_magnitude: f64) -> Option<f64> {
    let denominator = 2.0 * sample.s12.multiply(sample.s21).magnitude();
    if !denominator.is_finite() || denominator <= f64::EPSILON {
        return None;
    }
    let numerator = 1.0 - sample.s11.magnitude_squared() - sample.s22.magnitude_squared()
        + delta_magnitude * delta_magnitude;
    let rollet_k = numerator / denominator;
    rollet_k.is_finite().then_some(rollet_k)
}

fn maximum_available_gain_db(
    sample: &SParameterNetworkSample,
    delta_magnitude: f64,
) -> Option<f64> {
    let rollet_k = rollet_stability_factor(sample, delta_magnitude)?;
    if rollet_k <= 1.0 || delta_magnitude >= 1.0 {
        return None;
    }
    let stable_gain = maximum_stable_gain_linear(sample)?;
    let radical = (rollet_k * rollet_k - 1.0).max(0.0).sqrt();
    let gain = stable_gain * (rollet_k - radical);
    finite_positive_db(gain)
}

fn maximum_stable_gain_db(sample: &SParameterNetworkSample) -> Option<f64> {
    finite_positive_db(maximum_stable_gain_linear(sample)?)
}

fn maximum_stable_gain_linear(sample: &SParameterNetworkSample) -> Option<f64> {
    let reverse = sample.s12.magnitude();
    let forward = sample.s21.magnitude();
    if !reverse.is_finite() || !forward.is_finite() || reverse <= f64::EPSILON {
        return None;
    }
    let gain = forward / reverse;
    (gain.is_finite() && gain > 0.0).then_some(gain)
}

fn maximum_unilateral_gain_db(sample: &SParameterNetworkSample) -> Option<f64> {
    let input_match = 1.0 - sample.s11.magnitude_squared();
    let output_match = 1.0 - sample.s22.magnitude_squared();
    let forward_gain = sample.s21.magnitude_squared();
    let denominator = input_match * output_match;
    if !denominator.is_finite()
        || denominator <= f64::EPSILON
        || !forward_gain.is_finite()
        || forward_gain <= f64::EPSILON
    {
        return None;
    }
    finite_positive_db(forward_gain / denominator)
}

fn transducer_gain_db(
    sample: &SParameterNetworkSample,
    source: ComplexValue,
    load: ComplexValue,
) -> Option<f64> {
    let source_term = passive_power_term(source)?;
    let load_term = passive_power_term(load)?;
    let one = ComplexValue::one();
    let denominator = one
        .subtract(sample.s11.multiply(source))
        .multiply(one.subtract(sample.s22.multiply(load)))
        .subtract(
            sample
                .s12
                .multiply(sample.s21)
                .multiply(source)
                .multiply(load),
        )
        .magnitude_squared();
    if !denominator.is_finite() || denominator <= f64::EPSILON {
        return None;
    }
    finite_positive_db(source_term * sample.s21.magnitude_squared() * load_term / denominator)
}

fn available_gain_db(sample: &SParameterNetworkSample, source: ComplexValue) -> Option<f64> {
    let source_term = passive_power_term(source)?;
    let one = ComplexValue::one();
    let input_denominator = one
        .subtract(sample.s11.multiply(source))
        .magnitude_squared();
    if !input_denominator.is_finite() || input_denominator <= f64::EPSILON {
        return None;
    }
    let gamma_out = sample.s22.add(
        sample
            .s12
            .multiply(sample.s21)
            .multiply(source)
            .divide(one.subtract(sample.s11.multiply(source)))?,
    );
    let output_term = passive_power_term(gamma_out)?;
    finite_positive_db(
        source_term * sample.s21.magnitude_squared() / (input_denominator * output_term),
    )
}

fn operating_gain_db(sample: &SParameterNetworkSample, load: ComplexValue) -> Option<f64> {
    let load_term = passive_power_term(load)?;
    let one = ComplexValue::one();
    let output_denominator = one.subtract(sample.s22.multiply(load)).magnitude_squared();
    if !output_denominator.is_finite() || output_denominator <= f64::EPSILON {
        return None;
    }
    let gamma_in = sample.s11.add(
        sample
            .s12
            .multiply(sample.s21)
            .multiply(load)
            .divide(one.subtract(sample.s22.multiply(load)))?,
    );
    let input_term = passive_power_term(gamma_in)?;
    finite_positive_db(
        sample.s21.magnitude_squared() * load_term / (input_term * output_denominator),
    )
}

fn passive_power_term(value: ComplexValue) -> Option<f64> {
    let term = 1.0 - value.magnitude_squared();
    (term.is_finite() && term > f64::EPSILON).then_some(term)
}

fn finite_positive_db(value: f64) -> Option<f64> {
    if !value.is_finite() || value <= 0.0 {
        return None;
    }
    let db = 10.0 * value.log10();
    db.is_finite().then_some(db)
}

pub(super) fn read_s_parameter_summary(path: &Path) -> Result<Vec<SParameterSummaryRow>, String> {
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
        != "parameter,row_count,min_frequency_hz,max_frequency_hz,min_mag_db,max_mag_db,min_mag_linear,max_mag_linear,min_return_loss_db,max_return_loss_db,min_insertion_loss_db,max_insertion_loss_db,min_vswr,max_vswr,min_mismatch_loss_db,max_mismatch_loss_db,min_group_delay_s,max_group_delay_s,min_impedance_real_ohm,max_impedance_real_ohm,min_impedance_imag_ohm,max_impedance_imag_ohm,min_impedance_magnitude_ohm,max_impedance_magnitude_ohm"
    {
        return Err("S-parameter summary CSV has unexpected header.".to_string());
    }
    let mut rows = Vec::new();
    for (line_index, line) in lines.enumerate() {
        let fields: Vec<_> = line.split(',').map(str::trim).collect();
        if fields.len() != 24 {
            return Err(format!(
                "S-parameter summary row {} has {} fields, expected 24.",
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
            min_mismatch_loss_db: parse_optional_finite_f64(fields[14], "min_mismatch_loss_db")?,
            max_mismatch_loss_db: parse_optional_finite_f64(fields[15], "max_mismatch_loss_db")?,
            min_group_delay_s: parse_optional_finite_f64(fields[16], "min_group_delay_s")?,
            max_group_delay_s: parse_optional_finite_f64(fields[17], "max_group_delay_s")?,
            min_impedance_real_ohm: parse_optional_finite_f64(
                fields[18],
                "min_impedance_real_ohm",
            )?,
            max_impedance_real_ohm: parse_optional_finite_f64(
                fields[19],
                "max_impedance_real_ohm",
            )?,
            min_impedance_imag_ohm: parse_optional_finite_f64(
                fields[20],
                "min_impedance_imag_ohm",
            )?,
            max_impedance_imag_ohm: parse_optional_finite_f64(
                fields[21],
                "max_impedance_imag_ohm",
            )?,
            min_impedance_magnitude_ohm: parse_optional_finite_f64(
                fields[22],
                "min_impedance_magnitude_ohm",
            )?,
            max_impedance_magnitude_ohm: parse_optional_finite_f64(
                fields[23],
                "max_impedance_magnitude_ohm",
            )?,
        });
    }
    if rows.is_empty() {
        return Err("S-parameter summary CSV has no parameter rows.".to_string());
    }
    Ok(rows)
}

pub(super) fn read_s_parameter_network_summary(
    path: &Path,
) -> Result<SParameterNetworkSummaryRow, String> {
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
        != "port_count,row_count,min_frequency_hz,max_frequency_hz,max_reciprocity_error_linear,frequency_hz_at_max_reciprocity_error,max_passivity_singular_value,frequency_hz_at_max_passivity,min_rollet_k,frequency_hz_at_min_rollet_k,max_stability_delta_magnitude,frequency_hz_at_max_stability_delta_magnitude,min_maximum_available_gain_db,frequency_hz_at_min_maximum_available_gain,min_maximum_stable_gain_db,frequency_hz_at_min_maximum_stable_gain,min_maximum_unilateral_gain_db,frequency_hz_at_min_maximum_unilateral_gain,source_reflection_real,source_reflection_imaginary,load_reflection_real,load_reflection_imaginary,min_transducer_gain_db,frequency_hz_at_min_transducer_gain,min_available_gain_db,frequency_hz_at_min_available_gain,min_operating_gain_db,frequency_hz_at_min_operating_gain"
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
    if fields.len() != 28 {
        return Err(format!(
            "S-parameter network summary row has {} fields, expected 28.",
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
        min_rollet_k: parse_optional_finite_f64(fields[8], "min_rollet_k")?,
        frequency_hz_at_min_rollet_k: parse_optional_finite_f64(
            fields[9],
            "frequency_hz_at_min_rollet_k",
        )?,
        max_stability_delta_magnitude: parse_optional_finite_f64(
            fields[10],
            "max_stability_delta_magnitude",
        )?,
        frequency_hz_at_max_stability_delta_magnitude: parse_optional_finite_f64(
            fields[11],
            "frequency_hz_at_max_stability_delta_magnitude",
        )?,
        min_maximum_available_gain_db: parse_optional_finite_f64(
            fields[12],
            "min_maximum_available_gain_db",
        )?,
        frequency_hz_at_min_maximum_available_gain: parse_optional_finite_f64(
            fields[13],
            "frequency_hz_at_min_maximum_available_gain",
        )?,
        min_maximum_stable_gain_db: parse_optional_finite_f64(
            fields[14],
            "min_maximum_stable_gain_db",
        )?,
        frequency_hz_at_min_maximum_stable_gain: parse_optional_finite_f64(
            fields[15],
            "frequency_hz_at_min_maximum_stable_gain",
        )?,
        min_maximum_unilateral_gain_db: parse_optional_finite_f64(
            fields[16],
            "min_maximum_unilateral_gain_db",
        )?,
        frequency_hz_at_min_maximum_unilateral_gain: parse_optional_finite_f64(
            fields[17],
            "frequency_hz_at_min_maximum_unilateral_gain",
        )?,
        source_reflection_real: parse_optional_finite_f64(fields[18], "source_reflection_real")?,
        source_reflection_imaginary: parse_optional_finite_f64(
            fields[19],
            "source_reflection_imaginary",
        )?,
        load_reflection_real: parse_optional_finite_f64(fields[20], "load_reflection_real")?,
        load_reflection_imaginary: parse_optional_finite_f64(
            fields[21],
            "load_reflection_imaginary",
        )?,
        min_transducer_gain_db: parse_optional_finite_f64(fields[22], "min_transducer_gain_db")?,
        frequency_hz_at_min_transducer_gain: parse_optional_finite_f64(
            fields[23],
            "frequency_hz_at_min_transducer_gain",
        )?,
        min_available_gain_db: parse_optional_finite_f64(fields[24], "min_available_gain_db")?,
        frequency_hz_at_min_available_gain: parse_optional_finite_f64(
            fields[25],
            "frequency_hz_at_min_available_gain",
        )?,
        min_operating_gain_db: parse_optional_finite_f64(fields[26], "min_operating_gain_db")?,
        frequency_hz_at_min_operating_gain: parse_optional_finite_f64(
            fields[27],
            "frequency_hz_at_min_operating_gain",
        )?,
    })
}

pub(super) fn parse_s_parameter_name(parameter: &str) -> Option<(usize, usize)> {
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

pub(super) fn optional_csv(value: Option<f64>) -> String {
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
