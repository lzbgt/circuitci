use super::*;
use anyhow::{Result, bail};
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub(super) struct ImportedDirectiveMetadata {
    pub(super) temp_cards: Vec<String>,
    pub(super) option_cards: Vec<String>,
    pub(super) initial_condition_cards: Vec<String>,
    pub(super) nodeset_cards: Vec<String>,
    pub(super) model_cards: Vec<String>,
    pub(super) ambient_temperature_c: Option<f64>,
    pub(super) ambiguous_temperature: bool,
}

pub(super) fn record_imported_directive(
    metadata: &mut ImportedDirectiveMetadata,
    tokens: &[String],
    line: &str,
) -> Result<()> {
    let command = tokens[0].trim_start_matches('.').to_ascii_lowercase();
    if tokens.len() < 2 {
        bail!("SPICE .{command} directive requires at least one argument.");
    }
    match command.as_str() {
        "temp" => record_temp_directive(metadata, tokens, line),
        "option" | "options" => {
            metadata.option_cards.push(line.to_string());
            Ok(())
        }
        "ic" => {
            metadata.initial_condition_cards.push(line.to_string());
            Ok(())
        }
        "nodeset" => {
            metadata.nodeset_cards.push(line.to_string());
            Ok(())
        }
        "model" => {
            metadata.model_cards.push(line.to_string());
            Ok(())
        }
        _ => Ok(()),
    }
}

fn record_temp_directive(
    metadata: &mut ImportedDirectiveMetadata,
    tokens: &[String],
    line: &str,
) -> Result<()> {
    metadata.temp_cards.push(line.to_string());
    let parsed = tokens[1..]
        .iter()
        .map(|token| parse_spice_number(token))
        .collect::<Option<Vec<_>>>();
    let Some(values) = parsed else {
        metadata.ambiguous_temperature = true;
        metadata.ambient_temperature_c = None;
        return Ok(());
    };
    if values.len() != 1 || !values[0].is_finite() {
        metadata.ambiguous_temperature = true;
        metadata.ambient_temperature_c = None;
        return Ok(());
    }
    let value = values[0];
    if let Some(existing) = metadata.ambient_temperature_c {
        if (existing - value).abs() > f64::EPSILON {
            metadata.ambiguous_temperature = true;
            metadata.ambient_temperature_c = None;
        }
    } else if !metadata.ambiguous_temperature {
        metadata.ambient_temperature_c = Some(value);
    }
    Ok(())
}

pub(super) fn operating_conditions_for_directives(
    metadata: &ImportedDirectiveMetadata,
) -> OperatingConditionsYaml {
    OperatingConditionsYaml {
        ambient_temperature_c: metadata.ambient_temperature_c,
    }
}

pub(super) fn scenario_parameters_for_directives(
    metadata: &ImportedDirectiveMetadata,
) -> BTreeMap<String, serde_yaml_ng::Value> {
    let mut directive_map = BTreeMap::new();
    insert_string_array(
        &mut directive_map,
        "temp_cards",
        metadata.temp_cards.as_slice(),
    );
    insert_string_array(
        &mut directive_map,
        "option_cards",
        metadata.option_cards.as_slice(),
    );
    insert_string_array(
        &mut directive_map,
        "initial_condition_cards",
        metadata.initial_condition_cards.as_slice(),
    );
    insert_string_array(
        &mut directive_map,
        "nodeset_cards",
        metadata.nodeset_cards.as_slice(),
    );
    insert_string_array(
        &mut directive_map,
        "model_cards",
        metadata.model_cards.as_slice(),
    );
    if let Some(temperature_c) = metadata.ambient_temperature_c {
        directive_map.insert(
            "ambient_temperature_c".to_string(),
            serde_yaml_ng::to_value(temperature_c).unwrap_or(serde_yaml_ng::Value::Null),
        );
    }
    if metadata.ambiguous_temperature {
        directive_map.insert(
            "ambiguous_temperature".to_string(),
            serde_yaml_ng::Value::Bool(true),
        );
    }
    let mut parameters = BTreeMap::new();
    if !directive_map.is_empty() {
        parameters.insert(
            "imported_spice_directives".to_string(),
            serde_yaml_ng::to_value(directive_map).unwrap_or(serde_yaml_ng::Value::Null),
        );
    }
    parameters
}

fn insert_string_array(
    values: &mut BTreeMap<String, serde_yaml_ng::Value>,
    key: &str,
    cards: &[String],
) {
    if cards.is_empty() {
        return;
    }
    values.insert(
        key.to_string(),
        serde_yaml_ng::Value::Sequence(
            cards
                .iter()
                .map(|card| serde_yaml_ng::Value::String(card.clone()))
                .collect(),
        ),
    );
}
