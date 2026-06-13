use super::{
    ComponentMapping, ComponentSpiceYaml, ParsedComponent, SpicePrimitiveYaml, SpiceValueSourceYaml,
};
use anyhow::{Context, Result, bail};

pub(super) fn resolve_component_spice(
    component: &ParsedComponent,
    mapping: Option<&ComponentMapping>,
) -> Result<Option<ComponentSpiceYaml>> {
    let Some(mut spice) = mapping.and_then(|item| item.spice.clone()) else {
        return Ok(None);
    };
    if spice.value_ohm.is_some() && spice.value_ohm_from.is_some() {
        bail!(
            "KiCad mapping for component {} cannot declare both spice.value_ohm and spice.value_ohm_from.",
            component.refdes
        );
    }
    if spice.value_f.is_some() && spice.value_f_from.is_some() {
        bail!(
            "KiCad mapping for component {} cannot declare both spice.value_f and spice.value_f_from.",
            component.refdes
        );
    }
    if spice.value_h.is_some() && spice.value_h_from.is_some() {
        bail!(
            "KiCad mapping for component {} cannot declare both spice.value_h and spice.value_h_from.",
            component.refdes
        );
    }
    if let Some(SpiceValueSourceYaml::SchematicValue) = spice.value_ohm_from {
        if spice.primitive != SpicePrimitiveYaml::Resistor {
            bail!(
                "KiCad mapping for component {} can use spice.value_ohm_from only with primitive resistor.",
                component.refdes
            );
        }
        spice.value_ohm = Some(parse_schematic_passive_value(
            component,
            PassiveValueKind::Resistance,
        )?);
    }
    if let Some(SpiceValueSourceYaml::SchematicValue) = spice.value_f_from {
        if spice.primitive != SpicePrimitiveYaml::Capacitor {
            bail!(
                "KiCad mapping for component {} can use spice.value_f_from only with primitive capacitor.",
                component.refdes
            );
        }
        spice.value_f = Some(parse_schematic_passive_value(
            component,
            PassiveValueKind::Capacitance,
        )?);
    }
    if let Some(SpiceValueSourceYaml::SchematicValue) = spice.value_h_from {
        if spice.primitive != SpicePrimitiveYaml::Inductor {
            bail!(
                "KiCad mapping for component {} can use spice.value_h_from only with primitive inductor.",
                component.refdes
            );
        }
        spice.value_h = Some(parse_schematic_passive_value(
            component,
            PassiveValueKind::Inductance,
        )?);
    }
    spice.value_ohm_from = None;
    spice.value_f_from = None;
    spice.value_h_from = None;
    validate_component_spice_shape(component, &spice)?;
    Ok(Some(spice))
}

fn validate_component_spice_shape(
    component: &ParsedComponent,
    spice: &ComponentSpiceYaml,
) -> Result<()> {
    match spice.primitive {
        SpicePrimitiveYaml::Resistor => {
            if spice.value_f.is_some()
                || spice.value_h.is_some()
                || spice.dc_v.is_some()
                || spice.pulse.is_some()
            {
                bail!(
                    "KiCad mapping for component {} primitive resistor may declare only spice.value_ohm.",
                    component.refdes
                );
            }
            positive_spice_field(component, spice.value_ohm, "spice.value_ohm")?;
        }
        SpicePrimitiveYaml::Capacitor => {
            if spice.value_ohm.is_some()
                || spice.value_h.is_some()
                || spice.dc_v.is_some()
                || spice.pulse.is_some()
            {
                bail!(
                    "KiCad mapping for component {} primitive capacitor may declare only spice.value_f.",
                    component.refdes
                );
            }
            positive_spice_field(component, spice.value_f, "spice.value_f")?;
        }
        SpicePrimitiveYaml::Inductor => {
            if spice.value_ohm.is_some()
                || spice.value_f.is_some()
                || spice.dc_v.is_some()
                || spice.pulse.is_some()
            {
                bail!(
                    "KiCad mapping for component {} primitive inductor may declare only spice.value_h.",
                    component.refdes
                );
            }
            positive_spice_field(component, spice.value_h, "spice.value_h")?;
        }
        SpicePrimitiveYaml::DcVoltageSource => {
            if spice.value_ohm.is_some()
                || spice.value_f.is_some()
                || spice.value_h.is_some()
                || spice.pulse.is_some()
            {
                bail!(
                    "KiCad mapping for component {} primitive dc_voltage_source may declare only spice.dc_v.",
                    component.refdes
                );
            }
            finite_spice_field(component, spice.dc_v, "spice.dc_v")?;
        }
        SpicePrimitiveYaml::PulseVoltageSource => {
            if spice.value_ohm.is_some()
                || spice.value_f.is_some()
                || spice.value_h.is_some()
                || spice.dc_v.is_some()
            {
                bail!(
                    "KiCad mapping for component {} primitive pulse_voltage_source may declare only spice.pulse.",
                    component.refdes
                );
            }
            if spice.pulse.is_none() {
                bail!(
                    "KiCad mapping for component {} primitive pulse_voltage_source requires spice.pulse.",
                    component.refdes
                );
            }
        }
    }
    Ok(())
}

fn positive_spice_field(
    component: &ParsedComponent,
    value: Option<f64>,
    field: &str,
) -> Result<()> {
    let value = finite_spice_field(component, value, field)?;
    if value <= 0.0 {
        bail!(
            "KiCad mapping for component {} {} must be greater than zero.",
            component.refdes,
            field
        );
    }
    Ok(())
}

fn finite_spice_field(component: &ParsedComponent, value: Option<f64>, field: &str) -> Result<f64> {
    let Some(value) = value else {
        bail!(
            "KiCad mapping for component {} requires finite {}.",
            component.refdes,
            field
        );
    };
    if !value.is_finite() {
        bail!(
            "KiCad mapping for component {} {} must be finite.",
            component.refdes,
            field
        );
    }
    Ok(value)
}

#[derive(Debug, Clone, Copy)]
enum PassiveValueKind {
    Resistance,
    Capacitance,
    Inductance,
}

impl PassiveValueKind {
    fn label(self) -> &'static str {
        match self {
            Self::Resistance => "resistance",
            Self::Capacitance => "capacitance",
            Self::Inductance => "inductance",
        }
    }

    fn unit(self) -> &'static str {
        match self {
            Self::Resistance => "ohm",
            Self::Capacitance => "farad",
            Self::Inductance => "henry",
        }
    }
}

fn parse_schematic_passive_value(
    component: &ParsedComponent,
    kind: PassiveValueKind,
) -> Result<f64> {
    let Some(raw_value) = component.value.as_deref() else {
        bail!(
            "KiCad mapping for component {} requests {} from schematic Value, but the component has no Value field.",
            component.refdes,
            kind.label()
        );
    };
    parse_passive_value(raw_value, kind).with_context(|| {
        format!(
            "KiCad component {} schematic Value {:?} is not a strict positive {} value.",
            component.refdes,
            raw_value,
            kind.unit()
        )
    })
}

fn parse_passive_value(raw_value: &str, kind: PassiveValueKind) -> Result<f64> {
    let trimmed = raw_value.trim();
    if trimmed.is_empty()
        || trimmed.chars().any(char::is_whitespace)
        || trimmed.contains(',')
        || trimmed.starts_with('+')
        || trimmed.starts_with('-')
    {
        bail!("value contains unsupported annotation or sign");
    }
    if let Ok(value) = trimmed.parse::<f64>() {
        if matches!(
            kind,
            PassiveValueKind::Capacitance | PassiveValueKind::Inductance
        ) {
            bail!("plain numeric passive value requires an explicit unit suffix");
        }
        return positive_passive_value(value);
    }
    let (normalized, explicit_farads) = match kind {
        PassiveValueKind::Resistance => (trimmed, false),
        PassiveValueKind::Capacitance | PassiveValueKind::Inductance => {
            let unit = match kind {
                PassiveValueKind::Capacitance => ['F', 'f'],
                PassiveValueKind::Inductance => ['H', 'h'],
                PassiveValueKind::Resistance => unreachable!(),
            };
            let stripped = trimmed
                .strip_suffix(unit[0])
                .or_else(|| trimmed.strip_suffix(unit[1]))
                .filter(|value| !value.is_empty());
            (stripped.unwrap_or(trimmed), stripped.is_some())
        }
    };
    if explicit_farads && let Ok(value) = normalized.parse::<f64>() {
        return positive_passive_value(value);
    }
    if let Some(value) = parse_embedded_designator_value(normalized, kind)? {
        return positive_passive_value(value);
    }
    let (number, suffix) = split_number_suffix(normalized)?;
    let value = number
        .parse::<f64>()
        .with_context(|| format!("invalid numeric prefix {number:?}"))?;
    let scale = suffix_scale(suffix, kind)?;
    positive_passive_value(value * scale)
}

fn split_number_suffix(value: &str) -> Result<(&str, &str)> {
    let suffix_start = value
        .find(|character: char| character.is_ascii_alphabetic())
        .with_context(|| format!("missing supported unit suffix in {value:?}"))?;
    let (number, suffix) = value.split_at(suffix_start);
    if number.is_empty() || suffix.is_empty() {
        bail!("missing numeric prefix or unit suffix");
    }
    if suffix
        .chars()
        .any(|character| !character.is_ascii_alphabetic())
    {
        bail!("unit suffix contains non-unit characters");
    }
    Ok((number, suffix))
}

fn parse_embedded_designator_value(value: &str, kind: PassiveValueKind) -> Result<Option<f64>> {
    let designators = value
        .char_indices()
        .filter(|(_, character)| character.is_ascii_alphabetic())
        .collect::<Vec<_>>();
    if designators.len() != 1 {
        return Ok(None);
    }
    let (index, designator) = designators[0];
    let after_index = index + designator.len_utf8();
    if after_index >= value.len() {
        return Ok(None);
    }
    let before = &value[..index];
    let after = &value[after_index..];
    if before.is_empty()
        || after.is_empty()
        || !after.chars().all(|character| character.is_ascii_digit())
    {
        return Ok(None);
    }
    let integer = before
        .parse::<f64>()
        .with_context(|| format!("invalid numeric prefix {before:?}"))?;
    let fraction = after
        .parse::<f64>()
        .with_context(|| format!("invalid embedded suffix digits {after:?}"))?
        / 10_f64.powi(after.len() as i32);
    let scale = suffix_scale(&designator.to_string(), kind)?;
    Ok(Some((integer + fraction) * scale))
}

fn suffix_scale(suffix: &str, kind: PassiveValueKind) -> Result<f64> {
    match kind {
        PassiveValueKind::Resistance => match suffix {
            "R" | "r" => Ok(1.0),
            "m" => Ok(1e-3),
            "k" | "K" => Ok(1e3),
            "M" | "meg" | "Meg" | "MEG" => Ok(1e6),
            "G" => Ok(1e9),
            _ => bail!("unsupported resistance suffix {suffix:?}"),
        },
        PassiveValueKind::Capacitance => match suffix {
            "p" | "P" => Ok(1e-12),
            "n" | "N" => Ok(1e-9),
            "u" | "U" => Ok(1e-6),
            "m" => Ok(1e-3),
            _ => bail!("unsupported capacitance suffix {suffix:?}"),
        },
        PassiveValueKind::Inductance => match suffix {
            "n" | "N" => Ok(1e-9),
            "u" | "U" => Ok(1e-6),
            "m" => Ok(1e-3),
            _ => bail!("unsupported inductance suffix {suffix:?}"),
        },
    }
}

fn positive_passive_value(value: f64) -> Result<f64> {
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        bail!("value must be finite and greater than zero")
    }
}

#[cfg(test)]
mod tests {
    use super::{PassiveValueKind, parse_passive_value};

    #[test]
    fn parses_strict_resistance_values() {
        assert_eq!(
            parse_passive_value("10k", PassiveValueKind::Resistance).unwrap(),
            10_000.0
        );
        assert_eq!(
            parse_passive_value("4k7", PassiveValueKind::Resistance).unwrap(),
            4_700.0
        );
        assert_eq!(
            parse_passive_value("1M", PassiveValueKind::Resistance).unwrap(),
            1_000_000.0
        );
        assert_eq!(
            parse_passive_value("1m", PassiveValueKind::Resistance).unwrap(),
            0.001
        );
        assert_eq!(
            parse_passive_value("0R05", PassiveValueKind::Resistance).unwrap(),
            0.05
        );
        assert!(parse_passive_value("10k 1%", PassiveValueKind::Resistance).is_err());
    }

    #[test]
    fn parses_strict_capacitance_values() {
        assert!(
            (parse_passive_value("100n", PassiveValueKind::Capacitance).unwrap() - 100e-9).abs()
                < 1e-18
        );
        assert!(
            (parse_passive_value("100nF", PassiveValueKind::Capacitance).unwrap() - 100e-9).abs()
                < 1e-18
        );
        assert!(
            (parse_passive_value("4u7", PassiveValueKind::Capacitance).unwrap() - 4.7e-6).abs()
                < 1e-18
        );
        assert!(
            (parse_passive_value("1mF", PassiveValueKind::Capacitance).unwrap() - 1e-3).abs()
                < 1e-18
        );
        assert!(parse_passive_value("100", PassiveValueKind::Capacitance).is_err());
        assert!(parse_passive_value("100n/50V", PassiveValueKind::Capacitance).is_err());
    }

    #[test]
    fn parses_strict_inductance_values() {
        assert!(
            (parse_passive_value("2u2", PassiveValueKind::Inductance).unwrap() - 2.2e-6).abs()
                < 1e-18
        );
        assert!(
            (parse_passive_value("2.2uH", PassiveValueKind::Inductance).unwrap() - 2.2e-6).abs()
                < 1e-18
        );
        assert!(
            (parse_passive_value("330nH", PassiveValueKind::Inductance).unwrap() - 330e-9).abs()
                < 1e-18
        );
        assert!(parse_passive_value("2.2", PassiveValueKind::Inductance).is_err());
        assert!(parse_passive_value("2.2uH shielded", PassiveValueKind::Inductance).is_err());
    }
}
