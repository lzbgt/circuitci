use super::WaveformProbe;
use std::collections::BTreeMap;

pub(super) fn append_derived_s_parameter_probes(probes: &mut Vec<WaveformProbe>) {
    let mut magnitude_db_by_parameter = BTreeMap::new();
    let mut magnitude_linear_by_parameter = BTreeMap::new();
    let mut phase_deg_by_parameter = BTreeMap::new();
    let reference_impedance_index = probes
        .iter()
        .position(|probe| probe.label == "reference_impedance_ohm");
    for (index, probe) in probes.iter().enumerate() {
        if let Some(parameter) = probe.label.strip_suffix(" magnitude dB")
            && parse_s_parameter_term(parameter).is_some()
        {
            magnitude_db_by_parameter.insert(parameter.to_ascii_lowercase(), index);
        }
        if let Some(parameter) = probe.label.strip_suffix(" linear magnitude")
            && parse_s_parameter_term(parameter).is_some()
        {
            magnitude_linear_by_parameter.insert(parameter.to_ascii_lowercase(), index);
        }
        if let Some(parameter) = probe.label.strip_suffix(" phase deg")
            && parse_s_parameter_term(parameter).is_some()
        {
            phase_deg_by_parameter.insert(parameter.to_ascii_lowercase(), index);
        }
    }
    for (parameter, db_index) in magnitude_db_by_parameter {
        let Some((output_port, input_port)) = parse_s_parameter_term(&parameter) else {
            continue;
        };
        let db_values = probes[db_index].values.clone();
        if output_port == input_port {
            append_reflection_term_probes(
                probes,
                &parameter,
                &db_values,
                &magnitude_linear_by_parameter,
                &phase_deg_by_parameter,
                reference_impedance_index,
            );
        } else {
            probes.push(WaveformProbe {
                label: format!("{parameter} insertion loss dB"),
                values: db_values.iter().map(|value| -value).collect(),
                derived: true,
                expression: Some(parameter),
                promoted_quantity: None,
            });
        }
    }
    append_derived_s_parameter_network_probes(
        probes,
        &magnitude_linear_by_parameter,
        &phase_deg_by_parameter,
    );
}

fn append_reflection_term_probes(
    probes: &mut Vec<WaveformProbe>,
    parameter: &str,
    db_values: &[f64],
    magnitude_linear_by_parameter: &BTreeMap<String, usize>,
    phase_deg_by_parameter: &BTreeMap<String, usize>,
    reference_impedance_index: Option<usize>,
) {
    probes.push(WaveformProbe {
        label: format!("{parameter} return loss dB"),
        values: db_values.iter().map(|value| -value).collect(),
        derived: true,
        expression: Some(parameter.to_string()),
        promoted_quantity: None,
    });
    let Some(linear_index) = magnitude_linear_by_parameter.get(parameter) else {
        return;
    };
    let linear_values = &probes[*linear_index].values;
    if linear_values
        .iter()
        .all(|value| value.is_finite() && *value >= 0.0 && *value < 1.0)
    {
        let mut vswr_values = Vec::with_capacity(linear_values.len());
        let mut mismatch_loss_values = Vec::with_capacity(linear_values.len());
        for value in linear_values {
            vswr_values.push((1.0 + value) / (1.0 - value));
            mismatch_loss_values.push(-10.0 * (1.0 - value * value).log10());
        }
        probes.push(WaveformProbe {
            label: format!("{parameter} VSWR"),
            values: vswr_values,
            derived: true,
            expression: Some(parameter.to_string()),
            promoted_quantity: None,
        });
        probes.push(WaveformProbe {
            label: format!("{parameter} mismatch loss dB"),
            values: mismatch_loss_values,
            derived: true,
            expression: Some(parameter.to_string()),
            promoted_quantity: None,
        });
    }
    if let Some(phase_index) = phase_deg_by_parameter.get(parameter)
        && let Some(impedance_values) = reflection_impedance_values_ohm(
            probes,
            *linear_index,
            *phase_index,
            reference_impedance_index,
        )
    {
        probes.push(WaveformProbe {
            label: format!("{parameter} impedance real ohm"),
            values: impedance_values.iter().map(|value| value.real).collect(),
            derived: true,
            expression: Some(parameter.to_string()),
            promoted_quantity: None,
        });
        probes.push(WaveformProbe {
            label: format!("{parameter} impedance imaginary ohm"),
            values: impedance_values
                .iter()
                .map(|value| value.imaginary)
                .collect(),
            derived: true,
            expression: Some(parameter.to_string()),
            promoted_quantity: None,
        });
        probes.push(WaveformProbe {
            label: format!("{parameter} impedance magnitude ohm"),
            values: impedance_values
                .iter()
                .map(|value| value.magnitude())
                .collect(),
            derived: true,
            expression: Some(parameter.to_string()),
            promoted_quantity: None,
        });
    }
}

fn reflection_impedance_values_ohm(
    probes: &[WaveformProbe],
    magnitude_linear_index: usize,
    phase_deg_index: usize,
    reference_impedance_index: Option<usize>,
) -> Option<Vec<SParameterComplexValue>> {
    let magnitude_values = &probes.get(magnitude_linear_index)?.values;
    let phase_values = &probes.get(phase_deg_index)?.values;
    if magnitude_values.len() != phase_values.len() {
        return None;
    }
    let reference_values = reference_impedance_index
        .and_then(|index| probes.get(index))
        .map(|probe| probe.values.as_slice());
    if let Some(values) = reference_values
        && values.len() != magnitude_values.len()
    {
        return None;
    }
    let mut values = Vec::with_capacity(magnitude_values.len());
    for (index, (magnitude, phase_deg)) in magnitude_values.iter().zip(phase_values).enumerate() {
        if !magnitude.is_finite() || *magnitude < 0.0 || !phase_deg.is_finite() {
            return None;
        }
        let reference_impedance_ohm = reference_values.map_or(50.0, |values| values[index]);
        if !reference_impedance_ohm.is_finite() || reference_impedance_ohm <= 0.0 {
            return None;
        }
        let gamma = SParameterComplexValue::from_polar_degrees(*magnitude, *phase_deg);
        let numerator = SParameterComplexValue {
            real: 1.0 + gamma.real,
            imaginary: gamma.imaginary,
        };
        let denominator = SParameterComplexValue {
            real: 1.0 - gamma.real,
            imaginary: -gamma.imaginary,
        };
        let impedance = numerator
            .divide(denominator)?
            .scale(reference_impedance_ohm);
        if !impedance.real.is_finite() || !impedance.imaginary.is_finite() {
            return None;
        }
        values.push(impedance);
    }
    Some(values)
}

fn parse_s_parameter_term(parameter: &str) -> Option<(usize, usize)> {
    let digits = parameter
        .trim()
        .to_ascii_lowercase()
        .strip_prefix('s')?
        .to_string();
    if digits.len() < 2 || !digits.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    let midpoint = digits.len() / 2;
    let output = digits[..midpoint].parse::<usize>().ok()?;
    let input = digits[midpoint..].parse::<usize>().ok()?;
    Some((output, input))
}

fn append_derived_s_parameter_network_probes(
    probes: &mut Vec<WaveformProbe>,
    magnitude_linear_by_parameter: &BTreeMap<String, usize>,
    phase_deg_by_parameter: &BTreeMap<String, usize>,
) {
    let Some(s11) = s_parameter_complex_values(
        probes,
        magnitude_linear_by_parameter,
        phase_deg_by_parameter,
        "s11",
    ) else {
        return;
    };
    let Some(s12) = s_parameter_complex_values(
        probes,
        magnitude_linear_by_parameter,
        phase_deg_by_parameter,
        "s12",
    ) else {
        return;
    };
    let Some(s21) = s_parameter_complex_values(
        probes,
        magnitude_linear_by_parameter,
        phase_deg_by_parameter,
        "s21",
    ) else {
        return;
    };
    let Some(s22) = s_parameter_complex_values(
        probes,
        magnitude_linear_by_parameter,
        phase_deg_by_parameter,
        "s22",
    ) else {
        return;
    };
    let sample_count = s11.len();
    if sample_count == 0
        || s12.len() != sample_count
        || s21.len() != sample_count
        || s22.len() != sample_count
    {
        return;
    }
    let source_reflection = metadata_complex_values(
        probes,
        "source_reflection_real",
        "source_reflection_imaginary",
        sample_count,
    );
    let load_reflection = metadata_complex_values(
        probes,
        "load_reflection_real",
        "load_reflection_imaginary",
        sample_count,
    );
    let mut reciprocity_values = Vec::with_capacity(sample_count);
    let mut passivity_values = Vec::with_capacity(sample_count);
    let mut stability_delta_values = Vec::with_capacity(sample_count);
    let mut rollet_k_values = Vec::with_capacity(sample_count);
    let mut rollet_k_available = true;
    let mut maximum_available_gain_values = Vec::with_capacity(sample_count);
    let mut maximum_available_gain_available = true;
    let mut maximum_stable_gain_values = Vec::with_capacity(sample_count);
    let mut maximum_stable_gain_available = true;
    let mut maximum_unilateral_gain_values = Vec::with_capacity(sample_count);
    let mut maximum_unilateral_gain_available = true;
    let mut transducer_gain_values = Vec::with_capacity(sample_count);
    let mut transducer_gain_available = source_reflection.is_some() && load_reflection.is_some();
    let mut available_gain_values = Vec::with_capacity(sample_count);
    let mut available_gain_available = source_reflection.is_some();
    let mut operating_gain_values = Vec::with_capacity(sample_count);
    let mut operating_gain_available = load_reflection.is_some();
    for index in 0..sample_count {
        let sample = SParameterNetworkSample {
            s11: s11[index],
            s12: s12[index],
            s21: s21[index],
            s22: s22[index],
        };
        reciprocity_values.push(sample.s21.subtract(sample.s12).magnitude());
        passivity_values.push(two_port_max_singular_value(sample));
        let delta = stability_delta(sample);
        let delta_magnitude = delta.magnitude();
        stability_delta_values.push(delta_magnitude);
        push_optional_value(
            &mut rollet_k_available,
            &mut rollet_k_values,
            rollet_stability_factor(sample, delta_magnitude),
        );
        push_optional_value(
            &mut maximum_available_gain_available,
            &mut maximum_available_gain_values,
            maximum_available_gain_db(sample, delta_magnitude),
        );
        push_optional_value(
            &mut maximum_stable_gain_available,
            &mut maximum_stable_gain_values,
            maximum_stable_gain_db(sample),
        );
        push_optional_value(
            &mut maximum_unilateral_gain_available,
            &mut maximum_unilateral_gain_values,
            maximum_unilateral_gain_db(sample),
        );
        if let (Some(source), Some(load)) = (&source_reflection, &load_reflection) {
            push_optional_value(
                &mut transducer_gain_available,
                &mut transducer_gain_values,
                transducer_gain_db(sample, source[index], load[index]),
            );
        }
        if let Some(source) = &source_reflection {
            push_optional_value(
                &mut available_gain_available,
                &mut available_gain_values,
                available_gain_db(sample, source[index]),
            );
        }
        if let Some(load) = &load_reflection {
            push_optional_value(
                &mut operating_gain_available,
                &mut operating_gain_values,
                operating_gain_db(sample, load[index]),
            );
        }
    }
    probes.push(WaveformProbe {
        label: "two-port reciprocity error".to_string(),
        values: reciprocity_values,
        derived: true,
        expression: Some("max |S21-S12|".to_string()),
        promoted_quantity: None,
    });
    probes.push(WaveformProbe {
        label: "two-port passivity singular value".to_string(),
        values: passivity_values,
        derived: true,
        expression: Some("max singular value(S)".to_string()),
        promoted_quantity: None,
    });
    probes.push(WaveformProbe {
        label: "two-port stability delta magnitude".to_string(),
        values: stability_delta_values,
        derived: true,
        expression: Some("|S11*S22 - S12*S21|".to_string()),
        promoted_quantity: None,
    });
    push_available_probe(
        probes,
        "two-port Rollet K",
        rollet_k_available,
        rollet_k_values,
        sample_count,
        "(1 - |S11|^2 - |S22|^2 + |Delta|^2) / (2*|S12*S21|)",
    );
    push_available_probe(
        probes,
        "two-port maximum available gain dB",
        maximum_available_gain_available,
        maximum_available_gain_values,
        sample_count,
        "10*log10((|S21|/|S12|)*(K-sqrt(K^2-1)))",
    );
    push_available_probe(
        probes,
        "two-port maximum stable gain dB",
        maximum_stable_gain_available,
        maximum_stable_gain_values,
        sample_count,
        "10*log10(|S21|/|S12|)",
    );
    push_available_probe(
        probes,
        "two-port maximum unilateral gain dB",
        maximum_unilateral_gain_available,
        maximum_unilateral_gain_values,
        sample_count,
        "10*log10(|S21|^2/((1-|S11|^2)*(1-|S22|^2)))",
    );
    push_available_probe(
        probes,
        "two-port transducer gain dB",
        transducer_gain_available,
        transducer_gain_values,
        sample_count,
        "10*log10(Gt(GammaS,GammaL))",
    );
    push_available_probe(
        probes,
        "two-port available gain dB",
        available_gain_available,
        available_gain_values,
        sample_count,
        "10*log10(Ga(GammaS))",
    );
    push_available_probe(
        probes,
        "two-port operating gain dB",
        operating_gain_available,
        operating_gain_values,
        sample_count,
        "10*log10(Gp(GammaL))",
    );
}

fn push_optional_value(available: &mut bool, values: &mut Vec<f64>, value: Option<f64>) {
    if !*available {
        return;
    }
    if let Some(value) = value {
        values.push(value);
    } else {
        *available = false;
        values.clear();
    }
}

fn push_available_probe(
    probes: &mut Vec<WaveformProbe>,
    label: &str,
    available: bool,
    values: Vec<f64>,
    sample_count: usize,
    expression: &str,
) {
    if available && values.len() == sample_count {
        probes.push(WaveformProbe {
            label: label.to_string(),
            values,
            derived: true,
            expression: Some(expression.to_string()),
            promoted_quantity: None,
        });
    }
}

fn s_parameter_complex_values(
    probes: &[WaveformProbe],
    magnitude_linear_by_parameter: &BTreeMap<String, usize>,
    phase_deg_by_parameter: &BTreeMap<String, usize>,
    parameter: &str,
) -> Option<Vec<SParameterComplexValue>> {
    let magnitude_index = *magnitude_linear_by_parameter.get(parameter)?;
    let phase_index = *phase_deg_by_parameter.get(parameter)?;
    let magnitude_values = &probes.get(magnitude_index)?.values;
    let phase_values = &probes.get(phase_index)?.values;
    if magnitude_values.len() != phase_values.len() {
        return None;
    }
    let mut values = Vec::with_capacity(magnitude_values.len());
    for (magnitude, phase_deg) in magnitude_values.iter().zip(phase_values) {
        if !magnitude.is_finite() || *magnitude < 0.0 || !phase_deg.is_finite() {
            return None;
        }
        values.push(SParameterComplexValue::from_polar_degrees(
            *magnitude, *phase_deg,
        ));
    }
    Some(values)
}

fn metadata_complex_values(
    probes: &[WaveformProbe],
    real_label: &str,
    imaginary_label: &str,
    sample_count: usize,
) -> Option<Vec<SParameterComplexValue>> {
    let real_values = &probes
        .iter()
        .find(|probe| probe.label == real_label)?
        .values;
    let imaginary_values = &probes
        .iter()
        .find(|probe| probe.label == imaginary_label)?
        .values;
    if real_values.len() != sample_count || imaginary_values.len() != sample_count {
        return None;
    }
    let mut values = Vec::with_capacity(sample_count);
    for (real, imaginary) in real_values.iter().zip(imaginary_values) {
        let value = SParameterComplexValue {
            real: *real,
            imaginary: *imaginary,
        };
        if !value.real.is_finite()
            || !value.imaginary.is_finite()
            || value.magnitude_squared() >= 1.0
        {
            return None;
        }
        values.push(value);
    }
    Some(values)
}

#[derive(Debug, Clone, Copy)]
struct SParameterNetworkSample {
    s11: SParameterComplexValue,
    s12: SParameterComplexValue,
    s21: SParameterComplexValue,
    s22: SParameterComplexValue,
}

#[derive(Debug, Clone, Copy)]
struct SParameterComplexValue {
    real: f64,
    imaginary: f64,
}

impl SParameterComplexValue {
    fn one() -> Self {
        Self {
            real: 1.0,
            imaginary: 0.0,
        }
    }

    fn from_polar_degrees(magnitude: f64, phase_deg: f64) -> Self {
        let phase_rad = phase_deg.to_radians();
        Self {
            real: magnitude * phase_rad.cos(),
            imaginary: magnitude * phase_rad.sin(),
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

fn two_port_max_singular_value(sample: SParameterNetworkSample) -> f64 {
    let a = sample.s11.magnitude_squared() + sample.s21.magnitude_squared();
    let d = sample.s12.magnitude_squared() + sample.s22.magnitude_squared();
    let b = sample
        .s11
        .conjugate()
        .multiply(sample.s12)
        .add(sample.s21.conjugate().multiply(sample.s22));
    let trace = a + d;
    let discriminant = (a - d).mul_add(a - d, 4.0 * b.magnitude_squared());
    let largest_eigenvalue = 0.5 * (trace + discriminant.max(0.0).sqrt());
    largest_eigenvalue.max(0.0).sqrt()
}

fn stability_delta(sample: SParameterNetworkSample) -> SParameterComplexValue {
    sample
        .s11
        .multiply(sample.s22)
        .subtract(sample.s12.multiply(sample.s21))
}

fn rollet_stability_factor(sample: SParameterNetworkSample, delta_magnitude: f64) -> Option<f64> {
    let denominator = 2.0 * sample.s12.multiply(sample.s21).magnitude();
    if !denominator.is_finite() || denominator <= f64::EPSILON {
        return None;
    }
    let numerator = 1.0 - sample.s11.magnitude_squared() - sample.s22.magnitude_squared()
        + delta_magnitude.powi(2);
    let rollet_k = numerator / denominator;
    rollet_k.is_finite().then_some(rollet_k)
}

fn maximum_available_gain_db(sample: SParameterNetworkSample, delta_magnitude: f64) -> Option<f64> {
    let rollet_k = rollet_stability_factor(sample, delta_magnitude)?;
    if rollet_k <= 1.0 || delta_magnitude >= 1.0 {
        return None;
    }
    let stable_gain = maximum_stable_gain_linear(sample)?;
    finite_positive_db(stable_gain * (rollet_k - (rollet_k * rollet_k - 1.0).max(0.0).sqrt()))
}

fn maximum_stable_gain_db(sample: SParameterNetworkSample) -> Option<f64> {
    finite_positive_db(maximum_stable_gain_linear(sample)?)
}

fn maximum_stable_gain_linear(sample: SParameterNetworkSample) -> Option<f64> {
    let reverse = sample.s12.magnitude();
    let forward = sample.s21.magnitude();
    if !reverse.is_finite() || !forward.is_finite() || reverse <= f64::EPSILON {
        return None;
    }
    let gain = forward / reverse;
    (gain.is_finite() && gain > 0.0).then_some(gain)
}

fn maximum_unilateral_gain_db(sample: SParameterNetworkSample) -> Option<f64> {
    let denominator =
        (1.0 - sample.s11.magnitude_squared()) * (1.0 - sample.s22.magnitude_squared());
    let forward_gain = sample.s21.magnitude_squared();
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
    sample: SParameterNetworkSample,
    source: SParameterComplexValue,
    load: SParameterComplexValue,
) -> Option<f64> {
    let source_term = passive_power_term(source)?;
    let load_term = passive_power_term(load)?;
    let one = SParameterComplexValue::one();
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

fn available_gain_db(
    sample: SParameterNetworkSample,
    source: SParameterComplexValue,
) -> Option<f64> {
    let source_term = passive_power_term(source)?;
    let one = SParameterComplexValue::one();
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

fn operating_gain_db(sample: SParameterNetworkSample, load: SParameterComplexValue) -> Option<f64> {
    let load_term = passive_power_term(load)?;
    let one = SParameterComplexValue::one();
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

fn passive_power_term(value: SParameterComplexValue) -> Option<f64> {
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
