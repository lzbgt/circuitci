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

const JLCPCB_DOUBLE_SIDED_VIA_MIN_2026_06: FabricationProcessPreset = FabricationProcessPreset {
    id: "jlcpcb_double_sided_via_min_2026_06",
    aliases: &[
        "jlcpcb_double_sided_via_min",
        "jlcpcb_multilayer_via_min_2026_06",
    ],
    numeric_defaults: &[("min_annular_ring_mm", 0.05)],
};

const JLCPCB_SLOT_MIN_2026_06: FabricationProcessPreset = FabricationProcessPreset {
    id: "jlcpcb_slot_min_2026_06",
    aliases: &["jlcpcb_slot_min"],
    numeric_defaults: &[
        ("min_plated_slot_width_mm", 0.65),
        ("min_non_plated_slot_width_mm", 1.0),
        ("min_slot_aspect_ratio", 2.5),
    ],
};

const JLCPCB_DRILL_DIAMETER_RANGE_2026_06: FabricationProcessPreset = FabricationProcessPreset {
    id: "jlcpcb_drill_diameter_range_2026_06",
    aliases: &["jlcpcb_drill_diameter_range"],
    numeric_defaults: &[
        ("min_drill_diameter_mm", 0.15),
        ("max_drill_diameter_mm", 6.30),
    ],
};

const JLCPCB_CASTELLATED_HOLE_2026_06: FabricationProcessPreset = FabricationProcessPreset {
    id: "jlcpcb_castellated_hole_2026_06",
    aliases: &["jlcpcb_castellated_hole"],
    numeric_defaults: &[
        ("min_castellated_hole_diameter_mm", 0.30),
        ("min_castellated_hole_edge_clearance_mm", 1.00),
        ("min_castellated_hole_to_hole_spacing_mm", 0.40),
    ],
};

const JLCPCB_1OZ_COPPER_SPACING_2026_06: FabricationProcessPreset = FabricationProcessPreset {
    id: "jlcpcb_1oz_copper_spacing_2026_06",
    aliases: &[
        "jlcpcb_1oz_copper_spacing",
        "jlcpcb_1oz_trace_spacing_2026_06",
    ],
    numeric_defaults: &[("min_copper_spacing_mm", 0.10)],
};

const JLCPCB_ROUTED_EDGE_COPPER_CLEARANCE_2026_06: FabricationProcessPreset =
    FabricationProcessPreset {
        id: "jlcpcb_routed_edge_copper_clearance_2026_06",
        aliases: &[
            "jlcpcb_routed_edge_copper_clearance",
            "jlcpcb_routed_outline_copper_clearance_2026_06",
        ],
        numeric_defaults: &[("min_copper_edge_clearance_mm", 0.20)],
    };

const JLCPCB_STENCIL_APERTURE_MIN_2026_06: FabricationProcessPreset = FabricationProcessPreset {
    id: "jlcpcb_stencil_aperture_min_2026_06",
    aliases: &["jlcpcb_stencil_aperture_min"],
    numeric_defaults: &[("min_solder_paste_aperture_size_mm", 0.08)],
};

const JLCPCB_STENCIL_AREA_RATIO_2026_06: FabricationProcessPreset = FabricationProcessPreset {
    id: "jlcpcb_stencil_area_ratio_2026_06",
    aliases: &[
        "jlcpcb_stencil_area_ratio",
        "jlcpcb_ipc_7525_area_ratio_2026_06",
    ],
    numeric_defaults: &[("min_solder_paste_aperture_area_ratio", 0.66)],
};

const FABRICATION_PROCESS_PRESETS: &[FabricationProcessPreset] = &[
    JLCPCB_STANDARD_2026_06,
    JLCPCB_DOUBLE_SIDED_VIA_MIN_2026_06,
    JLCPCB_SLOT_MIN_2026_06,
    JLCPCB_DRILL_DIAMETER_RANGE_2026_06,
    JLCPCB_CASTELLATED_HOLE_2026_06,
    JLCPCB_1OZ_COPPER_SPACING_2026_06,
    JLCPCB_ROUTED_EDGE_COPPER_CLEARANCE_2026_06,
    JLCPCB_STENCIL_APERTURE_MIN_2026_06,
    JLCPCB_STENCIL_AREA_RATIO_2026_06,
];

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
    let Some(presets) = fabrication_process_presets(scenario, process_value, findings) else {
        return ProcessDefaultLookup::InvalidProcess;
    };
    let mut matched_value = None;
    let mut matched_preset = None;
    for preset in presets {
        let Some(value) = preset
            .numeric_defaults
            .iter()
            .find_map(|(default_name, value)| (*default_name == name).then_some(*value))
        else {
            continue;
        };
        if let Some(existing) = matched_value {
            if existing != value {
                validation_input_missing(
                    findings,
                    scenario,
                    format!(
                        "manufacturing parameters.fabrication_process presets '{}' and '{}' provide conflicting defaults for parameters.{name}.",
                        matched_preset.unwrap_or("<unknown>"),
                        preset.id
                    ),
                );
                return ProcessDefaultLookup::InvalidProcess;
            }
        } else {
            matched_value = Some(value);
            matched_preset = Some(preset.id);
        }
    }
    matched_value
        .map(ProcessDefaultLookup::Default)
        .unwrap_or(ProcessDefaultLookup::NoDefault)
}

fn fabrication_process_presets(
    scenario: &Scenario,
    process_value: &serde_yaml_ng::Value,
    findings: &mut Vec<Finding>,
) -> Option<Vec<&'static FabricationProcessPreset>> {
    if let Some(process_id) = process_value.as_str() {
        return find_single_fabrication_process_preset(scenario, process_id, findings)
            .map(|preset| vec![preset]);
    }
    let Some(process_ids) = process_value.as_sequence() else {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.fabrication_process must be a string or list of strings when provided.",
        );
        return None;
    };
    if process_ids.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "manufacturing parameters.fabrication_process list must not be empty.",
        );
        return None;
    }
    let mut presets = Vec::with_capacity(process_ids.len());
    for process_id in process_ids {
        let Some(process_id) = process_id.as_str() else {
            validation_input_missing(
                findings,
                scenario,
                "manufacturing parameters.fabrication_process list entries must be strings.",
            );
            return None;
        };
        let preset = find_single_fabrication_process_preset(scenario, process_id, findings)?;
        presets.push(preset);
    }
    Some(presets)
}

fn find_single_fabrication_process_preset(
    scenario: &Scenario,
    process_id: &str,
    findings: &mut Vec<Finding>,
) -> Option<&'static FabricationProcessPreset> {
    let Some(preset) = find_fabrication_process_preset(process_id) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "manufacturing parameters.fabrication_process references unsupported process preset '{process_id}'."
            ),
        );
        return None;
    };
    Some(preset)
}

fn find_fabrication_process_preset(process_id: &str) -> Option<&'static FabricationProcessPreset> {
    FABRICATION_PROCESS_PRESETS
        .iter()
        .find(|preset| preset.id == process_id || preset.aliases.contains(&process_id))
}
