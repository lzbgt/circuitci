use crate::board_ir::Scenario;
use crate::reports::Finding;

use super::super::common::validation_input_missing;

const FABRICATION_PROCESS_PARAMETER: &str = "fabrication_process";

struct FabricationProcessPreset {
    id: &'static str,
    aliases: &'static [&'static str],
    numeric_defaults: &'static [(&'static str, f64)],
}

const JLCPCB_STANDARD_2026_06: FabricationProcessPreset = FabricationProcessPreset {
    id: "jlcpcb_standard_2026_06",
    aliases: &["jlcpcb_standard", "jlcpcb_2layer_standard_2026_06"],
    numeric_defaults: &[
        ("min_mask_expansion_mm", 0.05),
        ("min_solder_mask_dam_mm", 0.1),
    ],
};

const FABRICATION_PROCESS_PRESETS: &[FabricationProcessPreset] = &[JLCPCB_STANDARD_2026_06];

pub(super) fn required_numeric_parameter(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    if let Some(value) = explicit_numeric_parameter(scenario, name, findings) {
        return Some(value);
    }
    if scenario.parameters.contains_key(name) {
        return None;
    }
    match fabrication_process_numeric_default(scenario, name, findings) {
        ProcessDefaultLookup::Default(value) => return Some(value),
        ProcessDefaultLookup::InvalidProcess => return None,
        ProcessDefaultLookup::NoProcess | ProcessDefaultLookup::NoDefault => {}
    }
    validation_input_missing(
        findings,
        scenario,
        format!("manufacturing parameters.{name} is required."),
    );
    None
}

pub(super) fn optional_numeric_parameter(
    scenario: &Scenario,
    name: &str,
    default: f64,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    if let Some(value) = explicit_numeric_parameter(scenario, name, findings) {
        return Some(value);
    }
    if scenario.parameters.contains_key(name) {
        return None;
    }
    match fabrication_process_numeric_default(scenario, name, findings) {
        ProcessDefaultLookup::Default(value) => return Some(value),
        ProcessDefaultLookup::InvalidProcess => return None,
        ProcessDefaultLookup::NoProcess | ProcessDefaultLookup::NoDefault => {}
    }
    Some(default)
}

fn explicit_numeric_parameter(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    if !scenario.parameters.contains_key(name) {
        return None;
    }
    let Some(value) = scenario
        .parameters
        .get(name)
        .and_then(serde_yaml_ng::Value::as_f64)
    else {
        validation_input_missing(
            findings,
            scenario,
            format!("manufacturing parameters.{name} must be numeric."),
        );
        return None;
    };
    if !value.is_finite() {
        validation_input_missing(
            findings,
            scenario,
            format!("manufacturing parameters.{name} must be finite."),
        );
        return None;
    }
    Some(value)
}

enum ProcessDefaultLookup {
    Default(f64),
    InvalidProcess,
    NoDefault,
    NoProcess,
}

fn fabrication_process_numeric_default(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> ProcessDefaultLookup {
    let Some(process_value) = scenario.parameters.get(FABRICATION_PROCESS_PARAMETER) else {
        return ProcessDefaultLookup::NoProcess;
    };
    let process_id = process_value.as_str();
    let Some(process_id) = process_id else {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.fabrication_process must be a string when provided.",
        );
        return ProcessDefaultLookup::InvalidProcess;
    };
    let Some(preset) = find_fabrication_process_preset(process_id) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "manufacturing parameters.fabrication_process references unsupported process preset '{process_id}'."
            ),
        );
        return ProcessDefaultLookup::InvalidProcess;
    };
    preset
        .numeric_defaults
        .iter()
        .find_map(|(default_name, value)| (*default_name == name).then_some(*value))
        .map(ProcessDefaultLookup::Default)
        .unwrap_or(ProcessDefaultLookup::NoDefault)
}

fn find_fabrication_process_preset(process_id: &str) -> Option<&'static FabricationProcessPreset> {
    FABRICATION_PROCESS_PRESETS
        .iter()
        .find(|preset| preset.id == process_id || preset.aliases.contains(&process_id))
}
