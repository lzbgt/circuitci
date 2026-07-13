use super::*;
use anyhow::{Context, Result, bail};

pub(super) fn parse_element(tokens: &[String], line: &str) -> Result<ParsedElement> {
    let name = tokens[0].clone();
    validate_refdes(&name)?;
    let prefix = name
        .chars()
        .next()
        .expect("tokenized element name is not empty")
        .to_ascii_uppercase();
    match prefix {
        'R' => parse_two_terminal(tokens, "generic.analog.resistor", |value| {
            SpicePrimitiveSpec::Resistor { value_ohm: value }
        }),
        'C' => parse_two_terminal(tokens, "generic.analog.capacitor", |value| {
            SpicePrimitiveSpec::Capacitor { value_f: value }
        }),
        'L' => parse_two_terminal(tokens, "generic.analog.inductor", |value| {
            SpicePrimitiveSpec::Inductor { value_h: value }
        }),
        'V' => parse_voltage_source(tokens),
        'I' => parse_current_source(tokens),
        'E' => parse_voltage_controlled_source(tokens, ImportedSourceKind::Voltage),
        'G' => parse_voltage_controlled_source(tokens, ImportedSourceKind::Current),
        'F' => parse_current_controlled_source(tokens, ImportedSourceKind::Current),
        'H' => parse_current_controlled_source(tokens, ImportedSourceKind::Voltage),
        'B' => parse_behavioral_source(tokens),
        'D' => parse_fixed_pins(
            tokens,
            3,
            &["A", "K"],
            "generic.analog.imported_spice_device",
        ),
        'Q' => {
            if tokens.len() >= 5 {
                let pin_names = if tokens.len() >= 6 {
                    vec!["C", "B", "E", "S"]
                } else {
                    vec!["C", "B", "E"]
                };
                parse_fixed_pins(
                    tokens,
                    pin_names.len() + 2,
                    &pin_names,
                    "generic.analog.imported_spice_device",
                )
            } else {
                bail!("Malformed BJT element line: {line}")
            }
        }
        'M' => parse_fixed_pins(
            tokens,
            6,
            &["D", "G", "S", "B"],
            "generic.analog.imported_spice_device",
        ),
        'X' => parse_subckt(tokens, line),
        _ if tokens.len() >= 4 => parse_fixed_pins(
            tokens,
            4,
            &["A", "B"],
            "generic.analog.imported_spice_device",
        ),
        _ => bail!("Unsupported or malformed SPICE element line: {line}"),
    }
}

fn parse_two_terminal<F>(tokens: &[String], model: &str, primitive: F) -> Result<ParsedElement>
where
    F: FnOnce(f64) -> SpicePrimitiveSpec,
{
    if tokens.len() < 4 {
        bail!(
            "Malformed two-terminal SPICE element line: {}",
            tokens.join(" ")
        );
    }
    let spice = parse_spice_number(&tokens[3]).map(primitive);
    Ok(ParsedElement {
        name: tokens[0].clone(),
        model: model.to_string(),
        pins: vec![
            ("A".to_string(), tokens[1].clone()),
            ("B".to_string(), tokens[2].clone()),
        ],
        spice,
        source_kind: None,
        distortion_role: distortion_role_from_tokens(tokens),
    })
}

fn parse_voltage_source(tokens: &[String]) -> Result<ParsedElement> {
    if tokens.len() < 4 {
        bail!(
            "Malformed voltage-source SPICE element line: {}",
            tokens.join(" ")
        );
    }
    let spice = parse_voltage_source_primitive(tokens)?;
    Ok(ParsedElement {
        name: tokens[0].clone(),
        model: "generic.analog.imported_spice_device".to_string(),
        pins: vec![
            ("P".to_string(), tokens[1].clone()),
            ("N".to_string(), tokens[2].clone()),
        ],
        spice,
        source_kind: Some(ImportedSourceKind::Voltage),
        distortion_role: distortion_role_from_tokens(tokens),
    })
}

fn parse_current_source(tokens: &[String]) -> Result<ParsedElement> {
    if tokens.len() < 4 {
        bail!(
            "Malformed current-source SPICE element line: {}",
            tokens.join(" ")
        );
    }
    let spice = parse_current_source_primitive(tokens)?;
    Ok(ParsedElement {
        name: tokens[0].clone(),
        model: "generic.analog.imported_spice_device".to_string(),
        pins: vec![
            ("P".to_string(), tokens[1].clone()),
            ("N".to_string(), tokens[2].clone()),
        ],
        spice,
        source_kind: Some(ImportedSourceKind::Current),
        distortion_role: distortion_role_from_tokens(tokens),
    })
}

fn parse_voltage_controlled_source(
    tokens: &[String],
    source_kind: ImportedSourceKind,
) -> Result<ParsedElement> {
    if tokens.len() < 6 {
        bail!(
            "Malformed voltage-controlled SPICE source line: {}",
            tokens.join(" ")
        );
    }
    Ok(ParsedElement {
        name: tokens[0].clone(),
        model: "generic.analog.imported_spice_device".to_string(),
        pins: vec![
            ("P".to_string(), tokens[1].clone()),
            ("N".to_string(), tokens[2].clone()),
            ("CP".to_string(), tokens[3].clone()),
            ("CN".to_string(), tokens[4].clone()),
        ],
        spice: None,
        source_kind: Some(source_kind),
        distortion_role: distortion_role_from_tokens(tokens),
    })
}

fn parse_current_controlled_source(
    tokens: &[String],
    source_kind: ImportedSourceKind,
) -> Result<ParsedElement> {
    if tokens.len() < 5 {
        bail!(
            "Malformed current-controlled SPICE source line: {}",
            tokens.join(" ")
        );
    }
    Ok(ParsedElement {
        name: tokens[0].clone(),
        model: "generic.analog.imported_spice_device".to_string(),
        pins: vec![
            ("P".to_string(), tokens[1].clone()),
            ("N".to_string(), tokens[2].clone()),
        ],
        spice: None,
        source_kind: Some(source_kind),
        distortion_role: distortion_role_from_tokens(tokens),
    })
}

fn parse_behavioral_source(tokens: &[String]) -> Result<ParsedElement> {
    if tokens.len() < 4 {
        bail!(
            "Malformed behavioral SPICE source line: {}",
            tokens.join(" ")
        );
    }
    let source_expression = tokens[3..].join("");
    let source_expression = source_expression.to_ascii_uppercase();
    let source_kind = if source_expression.starts_with("V=") {
        Some(ImportedSourceKind::Voltage)
    } else if source_expression.starts_with("I=") {
        Some(ImportedSourceKind::Current)
    } else {
        None
    };
    Ok(ParsedElement {
        name: tokens[0].clone(),
        model: "generic.analog.imported_spice_device".to_string(),
        pins: vec![
            ("P".to_string(), tokens[1].clone()),
            ("N".to_string(), tokens[2].clone()),
        ],
        spice: None,
        source_kind,
        distortion_role: distortion_role_from_tokens(tokens),
    })
}

fn parse_fixed_pins(
    tokens: &[String],
    min_tokens: usize,
    pin_names: &[&str],
    model: &str,
) -> Result<ParsedElement> {
    if tokens.len() < min_tokens {
        bail!("Malformed SPICE element line: {}", tokens.join(" "));
    }
    Ok(ParsedElement {
        name: tokens[0].clone(),
        model: model.to_string(),
        pins: pin_names
            .iter()
            .enumerate()
            .map(|(index, pin)| ((*pin).to_string(), tokens[index + 1].clone()))
            .collect(),
        spice: None,
        source_kind: None,
        distortion_role: distortion_role_from_tokens(tokens),
    })
}

fn parse_subckt(tokens: &[String], line: &str) -> Result<ParsedElement> {
    if tokens.len() < 4 {
        bail!("Malformed subcircuit instance line: {line}");
    }
    let node_count = tokens.len() - 2;
    Ok(ParsedElement {
        name: tokens[0].clone(),
        model: "generic.analog.imported_spice_device".to_string(),
        pins: (0..node_count)
            .map(|index| (format!("P{}", index + 1), tokens[index + 1].clone()))
            .collect(),
        spice: None,
        source_kind: None,
        distortion_role: distortion_role_from_tokens(tokens),
    })
}

fn parse_voltage_source_primitive(tokens: &[String]) -> Result<Option<SpicePrimitiveSpec>> {
    let spec = tokens[3..].join(" ");
    if spec.trim_start().to_ascii_uppercase().starts_with("PULSE") {
        return Ok(Some(SpicePrimitiveSpec::PulseVoltageSource {
            pulse: parse_pulse(&spec)?,
        }));
    }
    if source_uses_file_backed_waveform(tokens) {
        return Ok(None);
    }
    Ok(Some(SpicePrimitiveSpec::DcVoltageSource {
        dc_v: parse_independent_source_dc_value(tokens, "voltage")?,
    }))
}

fn parse_current_source_primitive(tokens: &[String]) -> Result<Option<SpicePrimitiveSpec>> {
    let spec = tokens[3..].join(" ");
    if spec.trim_start().to_ascii_uppercase().starts_with("PULSE") {
        return Ok(Some(SpicePrimitiveSpec::PulseCurrentSource {
            pulse: parse_current_pulse(&spec)?,
        }));
    }
    if source_uses_file_backed_waveform(tokens) {
        return Ok(None);
    }
    Ok(Some(SpicePrimitiveSpec::DcCurrentSource {
        dc_a: parse_independent_source_dc_value(tokens, "current")?,
    }))
}

fn source_uses_file_backed_waveform(tokens: &[String]) -> bool {
    tokens[3..].iter().any(|token| {
        let upper = token.trim_start().to_ascii_uppercase();
        let function = upper
            .split_once('(')
            .map_or(upper.as_str(), |(name, _)| name);
        matches!(function, "SIN" | "SINE" | "PWL" | "EXP" | "SFFM" | "AM")
    })
}

fn parse_pulse(spec: &str) -> Result<PulseSpec> {
    let start = spec.find('(').context("PULSE source is missing '('")?;
    let end = spec.rfind(')').context("PULSE source is missing ')'")?;
    let values: Vec<_> = spec[start + 1..end]
        .split(|character: char| character == ',' || character.is_whitespace())
        .filter(|field| !field.is_empty())
        .map(parse_spice_number)
        .collect::<Option<Vec<_>>>()
        .context("PULSE source contains an unparseable numeric value")?;
    if values.len() < 7 {
        bail!("PULSE source requires at least seven values.");
    }
    Ok(PulseSpec {
        initial_v: values[0],
        pulsed_v: values[1],
        delay_us: values[2] * 1_000_000.0,
        rise_us: values[3] * 1_000_000.0,
        fall_us: values[4] * 1_000_000.0,
        width_us: values[5] * 1_000_000.0,
        period_us: values[6] * 1_000_000.0,
    })
}

fn parse_independent_source_dc_value(tokens: &[String], kind: &str) -> Result<f64> {
    if tokens[3].eq_ignore_ascii_case("AC") {
        return Ok(0.0);
    }
    let value_token = if tokens[3].eq_ignore_ascii_case("DC") {
        if tokens.len() < 5 {
            bail!("{} source DC keyword requires a value.", kind);
        }
        &tokens[4]
    } else {
        &tokens[3]
    };
    parse_spice_number(value_token)
        .with_context(|| format!("Could not parse {kind} source value {value_token}"))
}

fn parse_current_pulse(spec: &str) -> Result<CurrentPulseSpec> {
    let pulse = parse_pulse(spec)?;
    Ok(CurrentPulseSpec {
        initial_a: pulse.initial_v,
        pulsed_a: pulse.pulsed_v,
        delay_us: pulse.delay_us,
        rise_us: pulse.rise_us,
        fall_us: pulse.fall_us,
        width_us: pulse.width_us,
        period_us: pulse.period_us,
    })
}

fn validate_refdes(name: &str) -> Result<()> {
    if name.is_empty()
        || !name.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.')
        })
    {
        bail!("SPICE element name {name:?} cannot be represented as a Board IR component id.");
    }
    Ok(())
}

pub(super) fn parse_spice_number(token: &str) -> Option<f64> {
    let upper = token.trim().to_ascii_uppercase();
    for (suffix, scale) in [
        ("MEG", 1e6),
        ("T", 1e12),
        ("G", 1e9),
        ("K", 1e3),
        ("M", 1e-3),
        ("U", 1e-6),
        ("N", 1e-9),
        ("P", 1e-12),
        ("F", 1e-15),
    ] {
        if let Some(number) = upper.strip_suffix(suffix) {
            return number.parse::<f64>().ok().map(|value| value * scale);
        }
    }
    upper.parse::<f64>().ok()
}
