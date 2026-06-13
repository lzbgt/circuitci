use super::{ScenarioSuggestion, SuggestedScenario, SuggestedTarget, sanitized_name};
use crate::board_ir::{LayoutCopper, LayoutCopperFeature};
use crate::library::BoundBoard;
use serde_json::{Value, json};
use std::collections::BTreeMap;

const DRILL_DIAMETER_VALID: &str = "DRILL_DIAMETER_VALID";
const DRILL_TO_BOARD_EDGE_CLEARANCE_VALID: &str = "DRILL_TO_BOARD_EDGE_CLEARANCE_VALID";
const SLOT_TO_BOARD_EDGE_CLEARANCE_VALID: &str = "SLOT_TO_BOARD_EDGE_CLEARANCE_VALID";
const SLOT_WIDTH_VALID: &str = "SLOT_WIDTH_VALID";
const DRILL_ANNULAR_RING_VALID: &str = "DRILL_ANNULAR_RING_VALID";
const COPPER_TO_BOARD_EDGE_CLEARANCE_VALID: &str = "COPPER_TO_BOARD_EDGE_CLEARANCE_VALID";
const COPPER_SPACING_VALID: &str = "COPPER_SPACING_VALID";
const SOLDER_MASK_OPENING_VALID: &str = "SOLDER_MASK_OPENING_VALID";
const SOLDER_MASK_DAM_VALID: &str = "SOLDER_MASK_DAM_VALID";
const SOLDER_PASTE_OPENING_VALID: &str = "SOLDER_PASTE_OPENING_VALID";
const SOLDER_PASTE_APERTURE_SIZE_VALID: &str = "SOLDER_PASTE_APERTURE_SIZE_VALID";
const SOLDER_PASTE_IC_PIN_APERTURE_VALID: &str = "SOLDER_PASTE_IC_PIN_APERTURE_VALID";
const SOLDER_PASTE_SPACING_VALID: &str = "SOLDER_PASTE_SPACING_VALID";
const IC_PIN_PITCH_INFERENCE_TOLERANCE_MM: f64 = 0.01;
const JLC_IC_PIN_PITCH_INFERENCE_CANDIDATES: &[IcPinPitchInferenceCandidate] = &[
    IcPinPitchInferenceCandidate {
        pitch_mm: 0.3,
        min_matched_gaps: 2,
    },
    IcPinPitchInferenceCandidate {
        pitch_mm: 0.35,
        min_matched_gaps: 2,
    },
    IcPinPitchInferenceCandidate {
        pitch_mm: 0.4,
        min_matched_gaps: 2,
    },
    IcPinPitchInferenceCandidate {
        pitch_mm: 0.5,
        min_matched_gaps: 2,
    },
    IcPinPitchInferenceCandidate {
        pitch_mm: 0.65,
        min_matched_gaps: 2,
    },
    IcPinPitchInferenceCandidate {
        pitch_mm: 0.8,
        min_matched_gaps: 3,
    },
    IcPinPitchInferenceCandidate {
        pitch_mm: 1.0,
        min_matched_gaps: 3,
    },
    IcPinPitchInferenceCandidate {
        pitch_mm: 1.27,
        min_matched_gaps: 3,
    },
];

pub(super) fn manufacturing_suggestions(bound: &BoundBoard<'_>) -> Vec<ScenarioSuggestion> {
    let layout = &bound.project.board.layout;
    let mut suggestions = Vec::new();
    let project_name = sanitized_name(&bound.project.project.name);
    let has_outline = !layout.outline.segments.is_empty();
    let copper_objects = copper_object_count(&layout.copper);
    let mask_objects = copper_object_count(&layout.solder_mask);
    let paste_objects = copper_object_count(&layout.solder_paste);

    if !layout.drills.is_empty() {
        push_if_not_declared(
            bound,
            &mut suggestions,
            DRILL_DIAMETER_VALID,
            manufacturing_suggestion(
                "drill_diameter_valid",
                true,
                "Imported circular drill evidence can be screened against the source-backed JLCPCB drill diameter range.",
                &format!("{project_name}_drill_diameter"),
                DRILL_DIAMETER_VALID,
                Some(fabrication_process("jlcpcb_drill_diameter_range_2026_06")),
                Vec::new(),
            ),
        );
        if has_outline {
            push_if_not_declared(
                bound,
                &mut suggestions,
                DRILL_TO_BOARD_EDGE_CLEARANCE_VALID,
                manufacturing_suggestion(
                    "drill_to_board_edge_clearance",
                    false,
                    "Imported circular drill and board-outline evidence can be screened for drill-to-board-edge clearance once the process limit is supplied.",
                    &format!("{project_name}_drill_to_board_edge_clearance"),
                    DRILL_TO_BOARD_EDGE_CLEARANCE_VALID,
                    None,
                    vec![
                        "Set manufacturing parameters.min_drill_edge_clearance_mm from the selected fabrication process or board specification.".to_string(),
                    ],
                ),
            );
        }
    }

    if !layout.slots.is_empty() {
        push_if_not_declared(
            bound,
            &mut suggestions,
            SLOT_WIDTH_VALID,
            manufacturing_suggestion(
                "slot_width_valid",
                true,
                "Imported routed-slot evidence can be screened against source-backed JLCPCB plated and non-plated slot width limits.",
                &format!("{project_name}_slot_width"),
                SLOT_WIDTH_VALID,
                Some(fabrication_process("jlcpcb_slot_min_2026_06")),
                Vec::new(),
            ),
        );
        if has_outline {
            push_if_not_declared(
                bound,
                &mut suggestions,
                SLOT_TO_BOARD_EDGE_CLEARANCE_VALID,
                manufacturing_suggestion(
                    "slot_to_board_edge_clearance",
                    false,
                    "Imported routed-slot and board-outline evidence can be screened for slot-to-board-edge clearance once the process limit is supplied.",
                    &format!("{project_name}_slot_to_board_edge_clearance"),
                    SLOT_TO_BOARD_EDGE_CLEARANCE_VALID,
                    None,
                    vec![
                        "Set manufacturing parameters.min_slot_edge_clearance_mm from the selected fabrication process or board specification.".to_string(),
                    ],
                ),
            );
        }
    }

    if !layout.drills.is_empty() && !layout.copper.features.is_empty() {
        push_if_not_declared(
            bound,
            &mut suggestions,
            DRILL_ANNULAR_RING_VALID,
            manufacturing_suggestion(
                "drill_annular_ring_valid",
                true,
                "Imported drill and Gerber copper flash evidence can be screened with the source-backed JLCPCB via annular-ring preset.",
                &format!("{project_name}_drill_annular_ring"),
                DRILL_ANNULAR_RING_VALID,
                Some(fabrication_process("jlcpcb_double_sided_via_min_2026_06")),
                Vec::new(),
            ),
        );
    }

    if copper_objects > 0 && has_outline {
        push_if_not_declared(
            bound,
            &mut suggestions,
            COPPER_TO_BOARD_EDGE_CLEARANCE_VALID,
            manufacturing_suggestion(
                "copper_to_board_edge_clearance",
                true,
                "Imported Gerber copper and routed board-outline evidence can be screened with the source-backed JLCPCB routed-edge copper clearance preset.",
                &format!("{project_name}_copper_to_board_edge_clearance"),
                COPPER_TO_BOARD_EDGE_CLEARANCE_VALID,
                Some(fabrication_process(
                    "jlcpcb_routed_edge_copper_clearance_2026_06",
                )),
                Vec::new(),
            ),
        );
    }

    if copper_objects >= 2 {
        push_if_not_declared(
            bound,
            &mut suggestions,
            COPPER_SPACING_VALID,
            manufacturing_suggestion(
                "copper_spacing_valid",
                true,
                "Imported same-layer Gerber copper evidence can be screened against the source-backed JLCPCB 1 oz copper spacing preset.",
                &format!("{project_name}_copper_spacing"),
                COPPER_SPACING_VALID,
                Some(fabrication_process("jlcpcb_1oz_copper_spacing_2026_06")),
                Vec::new(),
            ),
        );
    }

    if !layout.copper.features.is_empty() && mask_objects > 0 {
        push_if_not_declared(
            bound,
            &mut suggestions,
            SOLDER_MASK_OPENING_VALID,
            manufacturing_suggestion(
                "solder_mask_opening_valid",
                true,
                "Imported Gerber copper flash and solder-mask evidence can be screened with the source-backed JLCPCB mask expansion preset.",
                &format!("{project_name}_solder_mask_opening"),
                SOLDER_MASK_OPENING_VALID,
                Some(fabrication_process("jlcpcb_standard_2026_06")),
                Vec::new(),
            ),
        );
    }

    if mask_objects >= 2 {
        push_if_not_declared(
            bound,
            &mut suggestions,
            SOLDER_MASK_DAM_VALID,
            manufacturing_suggestion(
                "solder_mask_dam_valid",
                true,
                "Imported Gerber solder-mask opening evidence can be screened with the source-backed JLCPCB mask dam preset.",
                &format!("{project_name}_solder_mask_dam"),
                SOLDER_MASK_DAM_VALID,
                Some(fabrication_process("jlcpcb_standard_2026_06")),
                Vec::new(),
            ),
        );
    }

    if !layout.copper.features.is_empty() && paste_objects > 0 {
        push_if_not_declared(
            bound,
            &mut suggestions,
            SOLDER_PASTE_OPENING_VALID,
            manufacturing_suggestion(
                "solder_paste_opening_valid",
                false,
                "Imported Gerber copper flash and solder-paste evidence can be screened for stencil paste area ratio once package or process limits are supplied.",
                &format!("{project_name}_solder_paste_opening"),
                SOLDER_PASTE_OPENING_VALID,
                None,
                vec![
                    "Set manufacturing parameters.min_paste_area_ratio and parameters.max_paste_area_ratio from the package stencil recommendation or fabrication process.".to_string(),
                ],
            ),
        );
    }

    if !layout.solder_paste.features.is_empty() || !layout.solder_paste.segments.is_empty() {
        push_if_not_declared(
            bound,
            &mut suggestions,
            SOLDER_PASTE_APERTURE_SIZE_VALID,
            manufacturing_suggestion(
                "solder_paste_aperture_size_valid",
                true,
                "Imported Gerber solder-paste flash and draw evidence can be screened against the source-backed JLCPCB stencil minimum aperture size.",
                &format!("{project_name}_solder_paste_aperture_size"),
                SOLDER_PASTE_APERTURE_SIZE_VALID,
                Some(fabrication_process("jlcpcb_stencil_aperture_min_2026_06")),
                Vec::new(),
            ),
        );
    }

    if let Some(evidence) = infer_ic_pin_pitch_from_paste(&layout.solder_paste)
        && !manufacturing_check_declared_for_target(
            bound,
            SOLDER_PASTE_IC_PIN_APERTURE_VALID,
            &evidence.component,
        )
    {
        let mut suggestion = manufacturing_suggestion(
            "solder_paste_ic_pin_aperture_valid",
            true,
            &format!(
                "Imported pad-owned solder-paste evidence for {} on {} has {} repeated {:.3} mm pin-pitch gaps matching the source-backed JLCPCB IC stencil table.",
                evidence.component, evidence.layer, evidence.matched_gaps, evidence.pitch_mm
            ),
            &format!("{project_name}_solder_paste_ic_pin_aperture"),
            SOLDER_PASTE_IC_PIN_APERTURE_VALID,
            Some(pin_pitch_parameter(evidence.pitch_mm)),
            Vec::new(),
        );
        suggestion.scenario.target = Some(SuggestedTarget {
            component: evidence.component,
            power_pin: None,
            reset_pin: None,
        });
        suggestions.push(suggestion);
    }

    if paste_objects >= 2 {
        push_if_not_declared(
            bound,
            &mut suggestions,
            SOLDER_PASTE_SPACING_VALID,
            manufacturing_suggestion(
                "solder_paste_spacing_valid",
                false,
                "Imported Gerber solder-paste opening evidence can be screened for stencil aperture spacing once the process limit is supplied.",
                &format!("{project_name}_solder_paste_spacing"),
                SOLDER_PASTE_SPACING_VALID,
                None,
                vec![
                    "Set manufacturing parameters.min_solder_paste_spacing_mm from the stencil fabrication process or package assembly rule.".to_string(),
                ],
            ),
        );
    }

    suggestions
}

fn push_if_not_declared(
    bound: &BoundBoard<'_>,
    suggestions: &mut Vec<ScenarioSuggestion>,
    check: &str,
    suggestion: ScenarioSuggestion,
) {
    if !manufacturing_check_declared(bound, check) {
        suggestions.push(suggestion);
    }
}

fn manufacturing_check_declared(bound: &BoundBoard<'_>, check: &str) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario.checks.iter().any(|declared| declared == check)
    })
}

fn manufacturing_check_declared_for_target(
    bound: &BoundBoard<'_>,
    check: &str,
    target_component: &str,
) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario.checks.iter().any(|declared| declared == check)
            && scenario
                .target
                .as_ref()
                .is_none_or(|target| target.component == target_component)
    })
}

fn manufacturing_suggestion(
    id: &str,
    runnable: bool,
    reason: &str,
    scenario_name: &str,
    check: &str,
    parameters: Option<BTreeMap<String, Value>>,
    required_inputs: Vec<String>,
) -> ScenarioSuggestion {
    ScenarioSuggestion {
        id: id.to_string(),
        kind: format!("manufacturing_{}", id.trim_end_matches("_valid")),
        confidence: "high".to_string(),
        runnable,
        reason: reason.to_string(),
        scenario: SuggestedScenario {
            name: scenario_name.to_string(),
            scenario_type: "manufacturing".to_string(),
            checks: vec![check.to_string()],
            parameters,
            target: None,
            timing: None,
            required_boot_mode: None,
            straps: Vec::new(),
            bootloader: None,
            events: Vec::new(),
            conditioning: None,
            protection_clamps: Vec::new(),
            usb_connectors: Vec::new(),
            usb_routes: Vec::new(),
            usb_route_pairs: Vec::new(),
            clocks: Vec::new(),
            reset_supervisors: Vec::new(),
            regulators: Vec::new(),
            pin_states: Vec::new(),
            paths: Vec::new(),
        },
        required_inputs,
    }
}

fn fabrication_process(process: &str) -> BTreeMap<String, Value> {
    BTreeMap::from([("fabrication_process".to_string(), json!(process))])
}

fn pin_pitch_parameter(pin_pitch_mm: f64) -> BTreeMap<String, Value> {
    BTreeMap::from([("pin_pitch_mm".to_string(), json!(pin_pitch_mm))])
}

fn copper_object_count(copper: &LayoutCopper) -> usize {
    copper.features.len() + copper.segments.len() + copper.regions.len()
}

#[derive(Debug, Clone, PartialEq)]
struct IcPinPitchEvidence {
    component: String,
    layer: String,
    pitch_mm: f64,
    matched_gaps: usize,
}

struct IcPinPitchInferenceCandidate {
    pitch_mm: f64,
    min_matched_gaps: usize,
}

fn infer_ic_pin_pitch_from_paste(paste: &LayoutCopper) -> Option<IcPinPitchEvidence> {
    let mut features_by_component_layer: BTreeMap<(String, String), Vec<&LayoutCopperFeature>> =
        BTreeMap::new();
    for feature in &paste.features {
        if feature.owner_kind.as_deref() != Some("pad") || feature.polarity != "dark" {
            continue;
        }
        let Some(component) = &feature.component else {
            continue;
        };
        features_by_component_layer
            .entry((component.clone(), feature.layer.clone()))
            .or_default()
            .push(feature);
    }

    let mut best: Option<IcPinPitchEvidence> = None;
    for ((component, layer), features) in features_by_component_layer {
        if features.len() < 3 {
            continue;
        }
        for candidate_pitch in JLC_IC_PIN_PITCH_INFERENCE_CANDIDATES {
            let mut matched_gaps = 0;
            for (index, first) in features.iter().enumerate() {
                for second in features.iter().skip(index + 1) {
                    let dx = first.at.x_mm - second.at.x_mm;
                    let dy = first.at.y_mm - second.at.y_mm;
                    let distance_mm = (dx * dx + dy * dy).sqrt();
                    if (distance_mm - candidate_pitch.pitch_mm).abs()
                        <= IC_PIN_PITCH_INFERENCE_TOLERANCE_MM
                    {
                        matched_gaps += 1;
                    }
                }
            }
            if matched_gaps < candidate_pitch.min_matched_gaps {
                continue;
            }
            let candidate = IcPinPitchEvidence {
                component: component.clone(),
                layer: layer.clone(),
                pitch_mm: candidate_pitch.pitch_mm,
                matched_gaps,
            };
            if best
                .as_ref()
                .is_none_or(|current| is_better_pitch_evidence(&candidate, current))
            {
                best = Some(candidate);
            }
        }
    }

    best
}

fn is_better_pitch_evidence(candidate: &IcPinPitchEvidence, current: &IcPinPitchEvidence) -> bool {
    candidate
        .matched_gaps
        .cmp(&current.matched_gaps)
        .then_with(|| current.pitch_mm.total_cmp(&candidate.pitch_mm))
        .then_with(|| current.component.cmp(&candidate.component))
        .then_with(|| current.layer.cmp(&candidate.layer))
        .is_gt()
}
