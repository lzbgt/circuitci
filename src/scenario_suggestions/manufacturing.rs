use super::{ScenarioSuggestion, SuggestedScenario, SuggestedTarget, sanitized_name};
use crate::board_ir::{
    LayoutCopper, LayoutCopperFeature, LayoutPoint, NetRoute, RfAntennaFeedPathRule,
    RfAntennaKeepoutRule, RfAntennaMatchingElement, RfAntennaMatchingNetworkRule,
    RfAntennaMeasurement, RfAntennaPerformanceLimit, RouteSegment,
};
use crate::library::BoundBoard;
use serde_json::{Value, json};
use std::collections::BTreeMap;

mod route_physics;
mod thermal;

const DRILL_DIAMETER_VALID: &str = "DRILL_DIAMETER_VALID";
const DRILL_TO_BOARD_EDGE_CLEARANCE_VALID: &str = "DRILL_TO_BOARD_EDGE_CLEARANCE_VALID";
const SLOT_TO_BOARD_EDGE_CLEARANCE_VALID: &str = "SLOT_TO_BOARD_EDGE_CLEARANCE_VALID";
const SLOT_WIDTH_VALID: &str = "SLOT_WIDTH_VALID";
const SLOT_ASPECT_RATIO_VALID: &str = "SLOT_ASPECT_RATIO_VALID";
const CASTELLATED_HOLE_VALID: &str = "CASTELLATED_HOLE_VALID";
const DRILL_ANNULAR_RING_VALID: &str = "DRILL_ANNULAR_RING_VALID";
const COPPER_TO_BOARD_EDGE_CLEARANCE_VALID: &str = "COPPER_TO_BOARD_EDGE_CLEARANCE_VALID";
const COPPER_SPACING_VALID: &str = "COPPER_SPACING_VALID";
const RF_ANTENNA_KEEPOUT_VALID: &str = "RF_ANTENNA_KEEPOUT_VALID";
const RF_ANTENNA_FEED_PATH_VALID: &str = "RF_ANTENNA_FEED_PATH_VALID";
const RF_ANTENNA_MATCHING_TOPOLOGY_VALID: &str = "RF_ANTENNA_MATCHING_TOPOLOGY_VALID";
const RF_ANTENNA_MEASURED_PERFORMANCE_VALID: &str = "RF_ANTENNA_MEASURED_PERFORMANCE_VALID";
const SOLDER_MASK_OPENING_VALID: &str = "SOLDER_MASK_OPENING_VALID";
const SOLDER_MASK_DAM_VALID: &str = "SOLDER_MASK_DAM_VALID";
const SOLDER_PASTE_OPENING_VALID: &str = "SOLDER_PASTE_OPENING_VALID";
const SOLDER_PASTE_APERTURE_SIZE_VALID: &str = "SOLDER_PASTE_APERTURE_SIZE_VALID";
const SOLDER_PASTE_APERTURE_AREA_RATIO_VALID: &str = "SOLDER_PASTE_APERTURE_AREA_RATIO_VALID";
const SOLDER_PASTE_IC_PIN_APERTURE_VALID: &str = "SOLDER_PASTE_IC_PIN_APERTURE_VALID";
const SOLDER_PASTE_BGA_APERTURE_VALID: &str = "SOLDER_PASTE_BGA_APERTURE_VALID";
const SOLDER_PASTE_SPACING_VALID: &str = "SOLDER_PASTE_SPACING_VALID";
const ASSEMBLY_FOOTPRINT_ALIGNMENT_VALID: &str = "ASSEMBLY_FOOTPRINT_ALIGNMENT_VALID";
const PIN_1_ORIENTATION_VALID: &str = "PIN_1_ORIENTATION_VALID";
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
    let has_castellated_drill = layout.drills.iter().any(|drill| drill.castellated);

    suggestions.extend(rf_antenna_keepout_suggestions(bound, &project_name));
    suggestions.extend(rf_antenna_feed_path_suggestions(bound, &project_name));
    suggestions.extend(rf_antenna_matching_topology_suggestions(
        bound,
        &project_name,
    ));
    suggestions.extend(rf_antenna_measured_performance_suggestions(
        bound,
        &project_name,
    ));
    suggestions.extend(thermal::thermal_suggestions(bound, &project_name));
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
            let drill_edge_clearance_mm = bound
                .project
                .board
                .manufacturing
                .min_drill_edge_clearance_mm
                .filter(|value| value.is_finite() && *value >= 0.0);
            let runnable = drill_edge_clearance_mm.is_some();
            push_if_not_declared(
                bound,
                &mut suggestions,
                DRILL_TO_BOARD_EDGE_CLEARANCE_VALID,
                manufacturing_suggestion(
                    "drill_to_board_edge_clearance",
                    runnable,
                    if runnable {
                        "Imported circular drill and board-outline evidence can be screened for drill-to-board-edge clearance using board-level manufacturing metadata."
                    } else {
                        "Imported circular drill and board-outline evidence can be screened for drill-to-board-edge clearance once the process limit is supplied."
                    },
                    &format!("{project_name}_drill_to_board_edge_clearance"),
                    DRILL_TO_BOARD_EDGE_CLEARANCE_VALID,
                    board_numeric_parameter("min_drill_edge_clearance_mm", drill_edge_clearance_mm),
                    if runnable {
                        Vec::new()
                    } else {
                        vec![
                            "Set manufacturing parameters.min_drill_edge_clearance_mm or board.manufacturing.min_drill_edge_clearance_mm from the selected fabrication process or board specification.".to_string(),
                        ]
                    },
                ),
            );
        }
    }

    if has_castellated_drill && has_outline {
        push_if_not_declared(
            bound,
            &mut suggestions,
            CASTELLATED_HOLE_VALID,
            manufacturing_suggestion(
                "castellated_hole_valid",
                true,
                "Explicit castellated drill evidence can be screened with the source-backed JLCPCB castellated-hole diameter, hole-to-board-edge, and hole-to-hole spacing limits.",
                &format!("{project_name}_castellated_hole"),
                CASTELLATED_HOLE_VALID,
                Some(fabrication_process("jlcpcb_castellated_hole_2026_06")),
                Vec::new(),
            ),
        );
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
        push_if_not_declared(
            bound,
            &mut suggestions,
            SLOT_ASPECT_RATIO_VALID,
            manufacturing_suggestion(
                "slot_aspect_ratio_valid",
                true,
                "Imported routed-slot evidence can be screened against the source-backed JLCPCB minimum slot length-to-width ratio.",
                &format!("{project_name}_slot_aspect_ratio"),
                SLOT_ASPECT_RATIO_VALID,
                Some(fabrication_process("jlcpcb_slot_min_2026_06")),
                Vec::new(),
            ),
        );
        if has_outline {
            let slot_edge_clearance_mm = bound
                .project
                .board
                .manufacturing
                .min_slot_edge_clearance_mm
                .filter(|value| value.is_finite() && *value >= 0.0);
            let runnable = slot_edge_clearance_mm.is_some();
            push_if_not_declared(
                bound,
                &mut suggestions,
                SLOT_TO_BOARD_EDGE_CLEARANCE_VALID,
                manufacturing_suggestion(
                    "slot_to_board_edge_clearance",
                    runnable,
                    if runnable {
                        "Imported routed-slot and board-outline evidence can be screened for slot-to-board-edge clearance using board-level manufacturing metadata."
                    } else {
                        "Imported routed-slot and board-outline evidence can be screened for slot-to-board-edge clearance once the process limit is supplied."
                    },
                    &format!("{project_name}_slot_to_board_edge_clearance"),
                    SLOT_TO_BOARD_EDGE_CLEARANCE_VALID,
                    board_numeric_parameter("min_slot_edge_clearance_mm", slot_edge_clearance_mm),
                    if runnable {
                        Vec::new()
                    } else {
                        vec![
                            "Set manufacturing parameters.min_slot_edge_clearance_mm or board.manufacturing.min_slot_edge_clearance_mm from the selected fabrication process or board specification.".to_string(),
                        ]
                    },
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
        let paste_area_parameters = paste_area_ratio_parameters(
            bound.project.board.manufacturing.min_paste_area_ratio,
            bound.project.board.manufacturing.max_paste_area_ratio,
        );
        let runnable = paste_area_parameters.is_some();
        push_if_not_declared(
            bound,
            &mut suggestions,
            SOLDER_PASTE_OPENING_VALID,
            manufacturing_suggestion(
                "solder_paste_opening_valid",
                runnable,
                if runnable {
                    "Imported Gerber copper flash and solder-paste evidence can be screened for stencil paste area ratio using board-level stencil metadata."
                } else {
                    "Imported Gerber copper flash and solder-paste evidence can be screened for stencil paste area ratio once package or process limits are supplied."
                },
                &format!("{project_name}_solder_paste_opening"),
                SOLDER_PASTE_OPENING_VALID,
                paste_area_parameters,
                if runnable {
                    Vec::new()
                } else {
                    vec![
                        "Set manufacturing parameters.min_paste_area_ratio and parameters.max_paste_area_ratio, or board.manufacturing.min_paste_area_ratio and board.manufacturing.max_paste_area_ratio, from the package stencil recommendation or fabrication process.".to_string(),
                    ]
                },
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

    if paste_objects > 0 {
        let stencil_thickness_mm = bound
            .project
            .board
            .manufacturing
            .stencil_thickness_mm
            .filter(|value| value.is_finite() && *value > 0.0);
        let runnable = stencil_thickness_mm.is_some();
        push_if_not_declared(
            bound,
            &mut suggestions,
            SOLDER_PASTE_APERTURE_AREA_RATIO_VALID,
            manufacturing_suggestion(
                "solder_paste_aperture_area_ratio_valid",
                runnable,
                if runnable {
                    "Imported Gerber solder-paste opening evidence can be screened against the source-backed JLCPCB/IPC stencil aperture area-ratio floor using board-level stencil thickness metadata."
                } else {
                    "Imported Gerber solder-paste opening evidence can be screened against the source-backed JLCPCB/IPC stencil aperture area-ratio floor once stencil thickness is supplied."
                },
                &format!("{project_name}_solder_paste_aperture_area_ratio"),
                SOLDER_PASTE_APERTURE_AREA_RATIO_VALID,
                Some(stencil_area_ratio_parameters(stencil_thickness_mm)),
                if runnable {
                    Vec::new()
                } else {
                    vec![
                        "Set manufacturing parameters.stencil_thickness_mm or board.manufacturing.stencil_thickness_mm for the stencil used to fabricate this paste layer.".to_string(),
                    ]
                },
            ),
        );
    }

    let bga_pitch_evidence = infer_bga_pitch_from_paste(&layout.solder_paste);
    if let Some(evidence) = &bga_pitch_evidence
        && !manufacturing_check_declared_for_target(
            bound,
            SOLDER_PASTE_BGA_APERTURE_VALID,
            &evidence.component,
        )
    {
        let mut suggestion = manufacturing_suggestion(
            "solder_paste_bga_aperture_valid",
            true,
            &format!(
                "Imported pad-owned solder-paste evidence for {} on {} has {} horizontal and {} vertical repeated {:.3} mm BGA pitch gaps matching the source-backed JLCPCB BGA stencil table.",
                evidence.component,
                evidence.layer,
                evidence.horizontal_gaps,
                evidence.vertical_gaps,
                evidence.pitch_mm
            ),
            &format!("{project_name}_solder_paste_bga_aperture"),
            SOLDER_PASTE_BGA_APERTURE_VALID,
            Some(pin_pitch_parameter(evidence.pitch_mm)),
            Vec::new(),
        );
        suggestion.scenario.target = Some(SuggestedTarget {
            component: evidence.component.clone(),
            power_pin: None,
            reset_pin: None,
        });
        suggestions.push(suggestion);
    }

    if let Some(evidence) = infer_ic_pin_pitch_from_paste(&layout.solder_paste)
        && bga_pitch_evidence
            .as_ref()
            .is_none_or(|bga| bga.component != evidence.component)
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
        let paste_spacing_mm = bound
            .project
            .board
            .manufacturing
            .min_solder_paste_spacing_mm
            .filter(|value| value.is_finite() && *value >= 0.0);
        let runnable = paste_spacing_mm.is_some();
        push_if_not_declared(
            bound,
            &mut suggestions,
            SOLDER_PASTE_SPACING_VALID,
            manufacturing_suggestion(
                "solder_paste_spacing_valid",
                runnable,
                if runnable {
                    "Imported Gerber solder-paste opening evidence can be screened for stencil aperture spacing using board-level stencil metadata."
                } else {
                    "Imported Gerber solder-paste opening evidence can be screened for stencil aperture spacing once the process limit is supplied."
                },
                &format!("{project_name}_solder_paste_spacing"),
                SOLDER_PASTE_SPACING_VALID,
                board_numeric_parameter("min_solder_paste_spacing_mm", paste_spacing_mm),
                if runnable {
                    Vec::new()
                } else {
                    vec![
                        "Set manufacturing parameters.min_solder_paste_spacing_mm or board.manufacturing.min_solder_paste_spacing_mm from the stencil fabrication process or package assembly rule.".to_string(),
                    ]
                },
            ),
        );
    }

    suggestions.extend(assembly_footprint_alignment_suggestions(
        bound,
        &project_name,
    ));
    suggestions.extend(route_physics::route_physics_suggestions(
        bound,
        &project_name,
    ));
    suggestions.extend(pin_1_orientation_suggestions(bound, &project_name));

    suggestions
}

fn assembly_footprint_alignment_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for (component_id, component) in &bound.project.board.components {
        if manufacturing_check_declared_for_target(
            bound,
            ASSEMBLY_FOOTPRINT_ALIGNMENT_VALID,
            component_id,
        ) || !has_comparable_assembly_alignment_evidence(bound, component_id)
        {
            continue;
        }
        let mut suggestion = manufacturing_suggestion(
            &format!(
                "assembly_footprint_alignment_{}",
                sanitized_name(component_id)
            ),
            true,
            &format!(
                "Component {component_id} has JLC/EasyEDA assembly source evidence and imported KiCad PCB footprint or placement evidence that can be screened for direct contradictions."
            ),
            &format!(
                "{}_{}_assembly_footprint_alignment",
                project_name,
                sanitized_name(component_id)
            ),
            ASSEMBLY_FOOTPRINT_ALIGNMENT_VALID,
            None,
            Vec::new(),
        );
        suggestion.kind = "manufacturing_assembly_footprint_alignment".to_string();
        suggestion.scenario.target = Some(SuggestedTarget {
            component: component_id.clone(),
            power_pin: None,
            reset_pin: None,
        });
        if component
            .source
            .as_ref()
            .and_then(|source| source.placement_orientation_confidence.as_deref())
            == Some("source_explicit")
        {
            suggestion.scenario.parameters = Some(BTreeMap::from([(
                "rotation_tolerance_deg".to_string(),
                json!(0.01),
            )]));
        }
        suggestions.push(suggestion);
    }
    suggestions
}

fn has_comparable_assembly_alignment_evidence(bound: &BoundBoard<'_>, component_id: &str) -> bool {
    let Some(component) = bound.project.board.components.get(component_id) else {
        return false;
    };
    let Some(source) = component.source.as_ref() else {
        return false;
    };
    if source.format.as_deref() != Some("jlc_assembly") {
        return false;
    }
    let Some(footprint) = bound.project.board.layout.footprints.get(component_id) else {
        return false;
    };
    let placement = bound.project.board.layout.placements.get(component_id);
    has_comparable_footprint_name_evidence(source, footprint)
        || has_comparable_part_property_evidence(source, footprint)
        || source.placement_side_confidence.as_deref() == Some("source_explicit")
            && source.placement_side.is_some()
            && placement
                .and_then(|placement| placement.side.as_ref())
                .is_some()
        || source.placement_orientation_confidence.as_deref() == Some("source_explicit")
            && source.placement_rotation_deg.is_some()
            && placement
                .and_then(|placement| placement.rotation_deg)
                .is_some()
}

fn has_comparable_footprint_name_evidence(
    source: &crate::board_ir::ComponentSourceSpec,
    footprint: &crate::board_ir::LayoutFootprint,
) -> bool {
    let has_assembly_footprint = [
        source.footprint.as_deref(),
        source.placement_footprint.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|value| !value.trim().is_empty());
    has_assembly_footprint
        && footprint.properties.iter().any(|property| {
            matches!(
                property.source.as_deref(),
                Some("kicad_footprint_identifier" | "kicad_footprint_property")
            )
        })
}

fn has_comparable_part_property_evidence(
    source: &crate::board_ir::ComponentSourceSpec,
    footprint: &crate::board_ir::LayoutFootprint,
) -> bool {
    has_comparable_part_property(
        source.supplier_part.as_deref(),
        footprint,
        &["jlcpcbpart", "lcscpart", "supplierpart", "supplierpn"],
    ) || has_comparable_part_property(
        source.manufacturer_part.as_deref(),
        footprint,
        &[
            "mpn",
            "manufacturerpart",
            "manufacturerpartnumber",
            "partnumber",
        ],
    )
}

fn has_comparable_part_property(
    assembly_part: Option<&str>,
    footprint: &crate::board_ir::LayoutFootprint,
    names: &[&str],
) -> bool {
    assembly_part.is_some_and(|value| !value.trim().is_empty())
        && footprint.properties.iter().any(|property| {
            property.source.as_deref() == Some("kicad_footprint_property")
                && names.contains(&normalize_property_name(&property.name).as_str())
        })
}

fn pin_1_orientation_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for component_id in bound.project.board.components.keys() {
        if manufacturing_check_declared_for_target(bound, PIN_1_ORIENTATION_VALID, component_id)
            || !has_pin_1_orientation_evidence(bound, component_id)
        {
            continue;
        }
        let mut suggestion = manufacturing_suggestion(
            &format!("pin_1_orientation_{}", sanitized_name(component_id)),
            false,
            &format!(
                "Component {component_id} has imported KiCad body-bounds and pad-1 marker evidence; add an explicit expected pin-1 direction from the package or assembly drawing before validating orientation."
            ),
            &format!("{}_{}_pin_1_orientation", project_name, sanitized_name(component_id)),
            PIN_1_ORIENTATION_VALID,
            Some(BTreeMap::from([
                ("expected_pin_1_direction_deg".to_string(), Value::Null),
                ("max_pin_1_direction_error_deg".to_string(), Value::Null),
            ])),
            vec![
                "Set manufacturing parameters.expected_pin_1_direction_deg from explicit package or assembly drawing evidence.".to_string(),
                "Set manufacturing parameters.max_pin_1_direction_error_deg from the allowed orientation tolerance.".to_string(),
            ],
        );
        suggestion.kind = "manufacturing_pin_1_orientation".to_string();
        suggestion.scenario.target = Some(SuggestedTarget {
            component: component_id.clone(),
            power_pin: None,
            reset_pin: None,
        });
        suggestions.push(suggestion);
    }
    suggestions
}

fn has_pin_1_orientation_evidence(bound: &BoundBoard<'_>, component_id: &str) -> bool {
    bound
        .project
        .board
        .layout
        .footprints
        .get(component_id)
        .and_then(|footprint| footprint.semantics.as_ref())
        .is_some_and(|semantics| semantics.body_bounds.is_some() && semantics.pin_1.is_some())
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
            control_effects: Vec::new(),
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

fn board_numeric_parameter(name: &str, value: Option<f64>) -> Option<BTreeMap<String, Value>> {
    value.map(|value| BTreeMap::from([(name.to_string(), json!(value))]))
}

fn normalize_property_name(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn paste_area_ratio_parameters(
    min_paste_area_ratio: Option<f64>,
    max_paste_area_ratio: Option<f64>,
) -> Option<BTreeMap<String, Value>> {
    let min_value = min_paste_area_ratio.filter(|value| value.is_finite() && *value >= 0.0)?;
    let max_value = max_paste_area_ratio.filter(|value| value.is_finite() && *value >= 0.0)?;
    if max_value < min_value {
        return None;
    }
    Some(BTreeMap::from([
        ("min_paste_area_ratio".to_string(), json!(min_value)),
        ("max_paste_area_ratio".to_string(), json!(max_value)),
    ]))
}

fn stencil_area_ratio_parameters(stencil_thickness_mm: Option<f64>) -> BTreeMap<String, Value> {
    let mut parameters = fabrication_process("jlcpcb_stencil_area_ratio_2026_06");
    if let Some(value) = stencil_thickness_mm {
        parameters.insert("stencil_thickness_mm".to_string(), json!(value));
    }
    parameters
}

fn copper_object_count(copper: &LayoutCopper) -> usize {
    copper.features.len() + copper.segments.len() + copper.regions.len()
}

fn rf_antenna_keepout_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for keepout in &bound.project.board.layout.constraints.rf_antenna.keepouts {
        if !rf_antenna_keepout_has_evidence(bound, keepout)
            || rf_antenna_keepout_check_declared(bound, &keepout.name)
        {
            continue;
        }
        suggestions.push(manufacturing_suggestion(
            &format!("rf_antenna_keepout_{}", sanitized_name(&keepout.name)),
            true,
            &format!(
                "RF antenna keepout {} has reviewed polygon/source metadata and imported same-layer copper evidence.",
                keepout.name
            ),
            &format!(
                "{}_{}_rf_antenna_keepout",
                project_name,
                sanitized_name(&keepout.name)
            ),
            RF_ANTENNA_KEEPOUT_VALID,
            Some(BTreeMap::from([(
                "keepouts".to_string(),
                json!([{ "name": keepout.name }]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

fn rf_antenna_keepout_has_evidence(bound: &BoundBoard<'_>, keepout: &RfAntennaKeepoutRule) -> bool {
    let metadata_valid = !keepout.name.trim().is_empty()
        && !keepout.layer.trim().is_empty()
        && !keepout.source.trim().is_empty()
        && keepout.min_copper_clearance_mm.is_finite()
        && keepout.min_copper_clearance_mm >= 0.0
        && keepout.polygon.len() >= 3
        && keepout
            .polygon
            .iter()
            .all(|point| point.x_mm.is_finite() && point.y_mm.is_finite())
        && keepout_polygon_area_mm2(&keepout.polygon) > f64::EPSILON
        && keepout
            .antenna_net
            .as_deref()
            .is_none_or(|net| bound.project.board.nets.contains_key(net));
    metadata_valid
        && (bound
            .project
            .board
            .layout
            .copper
            .features
            .iter()
            .any(|feature| {
                feature.layer == keepout.layer && !same_antenna_net(keepout, feature.net.as_deref())
            })
            || bound
                .project
                .board
                .layout
                .copper
                .segments
                .iter()
                .any(|segment| {
                    segment.layer == keepout.layer
                        && !same_antenna_net(keepout, segment.net.as_deref())
                })
            || bound
                .project
                .board
                .layout
                .copper
                .regions
                .iter()
                .any(|region| {
                    region.layer == keepout.layer
                        && !same_antenna_net(keepout, region.net.as_deref())
                }))
}

fn keepout_polygon_area_mm2(points: &[LayoutPoint]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(left, right)| left.x_mm * right.y_mm - right.x_mm * left.y_mm)
        .sum::<f64>()
        .abs()
        / 2.0
}

fn same_antenna_net(keepout: &RfAntennaKeepoutRule, net: Option<&str>) -> bool {
    matches!((keepout.antenna_net.as_deref(), net), (Some(antenna), Some(candidate)) if antenna == candidate)
}

fn rf_antenna_keepout_check_declared(bound: &BoundBoard<'_>, keepout_name: &str) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario
                .checks
                .iter()
                .any(|declared| declared == RF_ANTENNA_KEEPOUT_VALID)
            && scenario
                .parameters
                .get("keepouts")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .is_some_and(|keepouts| {
                    keepouts.iter().any(|item| {
                        item.as_mapping().and_then(|mapping| {
                            mapping
                                .get(serde_yaml_ng::Value::String("name".to_string()))
                                .and_then(serde_yaml_ng::Value::as_str)
                        }) == Some(keepout_name)
                    })
                })
    })
}

fn rf_antenna_feed_path_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for feed_path in &bound.project.board.layout.constraints.rf_antenna.feed_paths {
        if !rf_antenna_feed_path_has_evidence(bound, feed_path)
            || rf_antenna_feed_path_check_declared(bound, &feed_path.name)
        {
            continue;
        }
        suggestions.push(manufacturing_suggestion(
            &format!("rf_antenna_feed_path_{}", sanitized_name(&feed_path.name)),
            true,
            &format!(
                "RF antenna feed path {} has reviewed source metadata plus imported route, pad, placement, and matching-component evidence.",
                feed_path.name
            ),
            &format!(
                "{}_{}_rf_antenna_feed_path",
                project_name,
                sanitized_name(&feed_path.name)
            ),
            RF_ANTENNA_FEED_PATH_VALID,
            Some(BTreeMap::from([(
                "feed_paths".to_string(),
                json!([{ "name": feed_path.name }]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

fn rf_antenna_feed_path_has_evidence(
    bound: &BoundBoard<'_>,
    feed_path: &RfAntennaFeedPathRule,
) -> bool {
    !feed_path.name.trim().is_empty()
        && !feed_path.source.trim().is_empty()
        && bound
            .project
            .board
            .nets
            .contains_key(&feed_path.antenna_net)
        && feed_path.max_feed_route_length_mm.is_finite()
        && feed_path.max_feed_route_length_mm >= 0.0
        && feed_path.max_matching_component_distance_mm.is_finite()
        && feed_path.max_matching_component_distance_mm >= 0.0
        && !feed_path.matching_components.is_empty()
        && bound
            .project
            .board
            .components
            .get(&feed_path.feed_component)
            .is_some_and(|component| {
                component.pins.get(&feed_path.feed_pin) == Some(&feed_path.antenna_net)
            })
        && bound
            .project
            .board
            .layout
            .pads
            .get(&feed_path.feed_component)
            .and_then(|pads| pads.get(&feed_path.feed_pin))
            .is_some_and(|pad| {
                pad.net == feed_path.antenna_net
                    && pad.at.x_mm.is_finite()
                    && pad.at.y_mm.is_finite()
            })
        && bound
            .project
            .board
            .layout
            .routes
            .get(&feed_path.antenna_net)
            .is_some_and(route_has_finite_segments)
        && feed_path.matching_components.iter().all(|component| {
            bound.project.board.components.contains_key(component)
                && component_has_antenna_pin(bound, component, &feed_path.antenna_net)
                && bound
                    .project
                    .board
                    .layout
                    .placements
                    .get(component)
                    .is_some_and(|placement| {
                        placement.x_mm.is_finite() && placement.y_mm.is_finite()
                    })
                && component_has_antenna_layout_pad(bound, component, &feed_path.antenna_net)
        })
        && bound
            .project
            .board
            .layout
            .placements
            .get(&feed_path.feed_component)
            .is_some_and(|placement| placement.x_mm.is_finite() && placement.y_mm.is_finite())
}

fn route_has_finite_segments(route: &NetRoute) -> bool {
    !route.segments.is_empty() && route.segments.iter().all(route_segment_is_finite)
}

fn route_segment_is_finite(segment: &RouteSegment) -> bool {
    !segment.layer.trim().is_empty()
        && segment.width_mm.is_finite()
        && segment.width_mm > 0.0
        && segment.start.x_mm.is_finite()
        && segment.start.y_mm.is_finite()
        && segment.end.x_mm.is_finite()
        && segment.end.y_mm.is_finite()
        && (segment.end.x_mm - segment.start.x_mm).hypot(segment.end.y_mm - segment.start.y_mm)
            > f64::EPSILON
}

fn component_has_antenna_pin(bound: &BoundBoard<'_>, component: &str, antenna_net: &str) -> bool {
    bound
        .project
        .board
        .components
        .get(component)
        .is_some_and(|spec| spec.pins.values().any(|net| net == antenna_net))
}

fn component_has_antenna_layout_pad(
    bound: &BoundBoard<'_>,
    component: &str,
    antenna_net: &str,
) -> bool {
    bound
        .project
        .board
        .layout
        .pads
        .get(component)
        .is_some_and(|pads| {
            pads.values().any(|pad| {
                pad.net == antenna_net && pad.at.x_mm.is_finite() && pad.at.y_mm.is_finite()
            })
        })
}

fn rf_antenna_feed_path_check_declared(bound: &BoundBoard<'_>, feed_path_name: &str) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario
                .checks
                .iter()
                .any(|declared| declared == RF_ANTENNA_FEED_PATH_VALID)
            && scenario
                .parameters
                .get("feed_paths")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .is_some_and(|feed_paths| {
                    feed_paths.iter().any(|item| {
                        item.as_mapping().and_then(|mapping| {
                            mapping
                                .get(serde_yaml_ng::Value::String("name".to_string()))
                                .and_then(serde_yaml_ng::Value::as_str)
                        }) == Some(feed_path_name)
                    })
                })
    })
}

fn rf_antenna_matching_topology_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for network in &bound
        .project
        .board
        .layout
        .constraints
        .rf_antenna
        .matching_networks
    {
        if !rf_antenna_matching_network_has_evidence(bound, network)
            || rf_antenna_matching_network_check_declared(bound, &network.name)
        {
            continue;
        }
        suggestions.push(manufacturing_suggestion(
            &format!(
                "rf_antenna_matching_topology_{}",
                sanitized_name(&network.name)
            ),
            true,
            &format!(
                "RF antenna matching network {} has reviewed topology metadata plus imported component pin and layout pad evidence.",
                network.name
            ),
            &format!(
                "{}_{}_rf_antenna_matching_topology",
                project_name,
                sanitized_name(&network.name)
            ),
            RF_ANTENNA_MATCHING_TOPOLOGY_VALID,
            Some(BTreeMap::from([(
                "matching_networks".to_string(),
                json!([{ "name": network.name }]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

fn rf_antenna_matching_network_has_evidence(
    bound: &BoundBoard<'_>,
    network: &RfAntennaMatchingNetworkRule,
) -> bool {
    !network.name.trim().is_empty()
        && !network.source.trim().is_empty()
        && !network.elements.is_empty()
        && matches!(
            normalize_rf_token(&network.topology).as_str(),
            "series" | "l" | "pi" | "t" | "custom"
        )
        && bound.project.board.nets.contains_key(&network.antenna_net)
        && network
            .reference_net
            .as_deref()
            .is_none_or(|net| bound.project.board.nets.contains_key(net))
        && network
            .elements
            .iter()
            .any(|element| matching_element_touches_net(element, &network.antenna_net))
        && network
            .elements
            .iter()
            .enumerate()
            .all(|(index, element)| matching_element_has_evidence(bound, network, element, index))
}

fn matching_element_has_evidence(
    bound: &BoundBoard<'_>,
    network: &RfAntennaMatchingNetworkRule,
    element: &RfAntennaMatchingElement,
    _index: usize,
) -> bool {
    if element.component.trim().is_empty()
        || !bound
            .project
            .board
            .components
            .contains_key(&element.component)
    {
        return false;
    }
    match normalize_rf_token(&element.role).as_str() {
        "series" => {
            let Some(input_net) = matching_element_net(element.input_net.as_deref()) else {
                return false;
            };
            let Some(output_net) = matching_element_net(element.output_net.as_deref()) else {
                return false;
            };
            input_net != output_net
                && bound.project.board.nets.contains_key(input_net)
                && bound.project.board.nets.contains_key(output_net)
                && component_has_pin_on_net(bound, &element.component, input_net)
                && component_has_pin_on_net(bound, &element.component, output_net)
                && component_has_layout_pad_on_net(bound, &element.component, input_net)
                && component_has_layout_pad_on_net(bound, &element.component, output_net)
        }
        "shunt" => {
            let Some(signal_net) = matching_element_net(element.signal_net.as_deref()) else {
                return false;
            };
            let Some(reference_net) = matching_element_net(
                element
                    .reference_net
                    .as_deref()
                    .or(network.reference_net.as_deref()),
            ) else {
                return false;
            };
            signal_net != reference_net
                && bound.project.board.nets.contains_key(signal_net)
                && bound.project.board.nets.contains_key(reference_net)
                && component_has_pin_on_net(bound, &element.component, signal_net)
                && component_has_pin_on_net(bound, &element.component, reference_net)
                && component_has_layout_pad_on_net(bound, &element.component, signal_net)
                && component_has_layout_pad_on_net(bound, &element.component, reference_net)
        }
        _ => false,
    }
}

fn matching_element_net(net: Option<&str>) -> Option<&str> {
    net.map(str::trim).filter(|value| !value.is_empty())
}

fn matching_element_touches_net(element: &RfAntennaMatchingElement, net: &str) -> bool {
    [
        element.input_net.as_deref(),
        element.output_net.as_deref(),
        element.signal_net.as_deref(),
        element.reference_net.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|candidate| candidate == net)
}

fn normalize_rf_token(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn component_has_pin_on_net(bound: &BoundBoard<'_>, component: &str, net: &str) -> bool {
    bound
        .project
        .board
        .components
        .get(component)
        .is_some_and(|spec| spec.pins.values().any(|candidate| candidate == net))
}

fn component_has_layout_pad_on_net(bound: &BoundBoard<'_>, component: &str, net: &str) -> bool {
    bound
        .project
        .board
        .layout
        .pads
        .get(component)
        .is_some_and(|pads| {
            pads.values()
                .any(|pad| pad.net == net && pad.at.x_mm.is_finite() && pad.at.y_mm.is_finite())
        })
}

fn rf_antenna_matching_network_check_declared(
    bound: &BoundBoard<'_>,
    matching_network_name: &str,
) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario
                .checks
                .iter()
                .any(|declared| declared == RF_ANTENNA_MATCHING_TOPOLOGY_VALID)
            && scenario
                .parameters
                .get("matching_networks")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .is_some_and(|matching_networks| {
                    matching_networks.iter().any(|item| {
                        item.as_mapping().and_then(|mapping| {
                            mapping
                                .get(serde_yaml_ng::Value::String("name".to_string()))
                                .and_then(serde_yaml_ng::Value::as_str)
                        }) == Some(matching_network_name)
                    })
                })
    })
}

fn rf_antenna_measured_performance_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for measurement in &bound
        .project
        .board
        .layout
        .constraints
        .rf_antenna
        .measurements
    {
        if !rf_antenna_measurement_has_evidence(bound, measurement)
            || rf_antenna_measurement_check_declared(bound, &measurement.name)
        {
            continue;
        }
        let matching_limits = bound
            .project
            .board
            .layout
            .constraints
            .rf_antenna
            .performance_limits
            .iter()
            .filter(|limit| rf_antenna_performance_limit_matches(bound, measurement, limit))
            .collect::<Vec<_>>();
        if !matching_limits.is_empty() {
            for limit in matching_limits {
                let mut parameters = BTreeMap::from([
                    (
                        "rf_measurements".to_string(),
                        json!([{ "name": measurement.name }]),
                    ),
                    (
                        "min_return_loss_db".to_string(),
                        json!(limit.min_return_loss_db),
                    ),
                ]);
                if let Some(min_mhz) = limit.frequency_min_mhz {
                    parameters.insert("frequency_min_mhz".to_string(), json!(min_mhz));
                }
                if let Some(max_mhz) = limit.frequency_max_mhz {
                    parameters.insert("frequency_max_mhz".to_string(), json!(max_mhz));
                }
                suggestions.push(manufacturing_suggestion(
                    &format!(
                        "rf_antenna_measured_performance_{}_{}",
                        sanitized_name(&measurement.name),
                        sanitized_name(&limit.name)
                    ),
                    true,
                    &format!(
                        "RF antenna measurement {} has reviewed return-loss evidence matched to reviewed RF performance limit {}.",
                        measurement.name, limit.name
                    ),
                    &format!(
                        "{}_{}_{}_rf_antenna_measured_performance",
                        project_name,
                        sanitized_name(&measurement.name),
                        sanitized_name(&limit.name)
                    ),
                    RF_ANTENNA_MEASURED_PERFORMANCE_VALID,
                    Some(parameters),
                    Vec::new(),
                ));
            }
            continue;
        }
        suggestions.push(manufacturing_suggestion(
            &format!(
                "rf_antenna_measured_performance_{}",
                sanitized_name(&measurement.name)
            ),
            false,
            &format!(
                "RF antenna measurement {} has reviewed source, antenna-net, frequency, and return-loss evidence.",
                measurement.name
            ),
            &format!(
                "{}_{}_rf_antenna_measured_performance",
                project_name,
                sanitized_name(&measurement.name)
            ),
            RF_ANTENNA_MEASURED_PERFORMANCE_VALID,
            Some(BTreeMap::from([(
                "rf_measurements".to_string(),
                json!([{ "name": measurement.name }]),
            )])),
            vec![
                "Review and set parameters.min_return_loss_db from the antenna module datasheet, RF design review, or product requirement.".to_string(),
                "Optionally set parameters.frequency_min_mhz and parameters.frequency_max_mhz for the reviewed operating band.".to_string(),
            ],
        ));
    }
    suggestions
}

fn rf_antenna_measurement_has_evidence(
    bound: &BoundBoard<'_>,
    measurement: &RfAntennaMeasurement,
) -> bool {
    !measurement.name.trim().is_empty()
        && !measurement.source.trim().is_empty()
        && bound
            .project
            .board
            .nets
            .contains_key(&measurement.antenna_net)
        && measurement.frequency_mhz.is_finite()
        && measurement.frequency_mhz > 0.0
        && measurement.return_loss_db.is_finite()
        && measurement.return_loss_db > 0.0
}

fn rf_antenna_performance_limit_matches(
    bound: &BoundBoard<'_>,
    measurement: &RfAntennaMeasurement,
    limit: &RfAntennaPerformanceLimit,
) -> bool {
    !limit.name.trim().is_empty()
        && !limit.source.trim().is_empty()
        && limit.antenna_net == measurement.antenna_net
        && bound.project.board.nets.contains_key(&limit.antenna_net)
        && limit.min_return_loss_db.is_finite()
        && limit.min_return_loss_db > 0.0
        && optional_positive_frequency(limit.frequency_min_mhz)
        && optional_positive_frequency(limit.frequency_max_mhz)
        && frequency_band_order_valid(limit.frequency_min_mhz, limit.frequency_max_mhz)
        && limit
            .frequency_min_mhz
            .is_none_or(|min_mhz| measurement.frequency_mhz >= min_mhz - f64::EPSILON)
        && limit
            .frequency_max_mhz
            .is_none_or(|max_mhz| measurement.frequency_mhz <= max_mhz + f64::EPSILON)
}

fn optional_positive_frequency(value: Option<f64>) -> bool {
    value.is_none_or(|frequency_mhz| frequency_mhz.is_finite() && frequency_mhz > 0.0)
}

fn frequency_band_order_valid(min_mhz: Option<f64>, max_mhz: Option<f64>) -> bool {
    match (min_mhz, max_mhz) {
        (Some(min_mhz), Some(max_mhz)) => max_mhz + f64::EPSILON >= min_mhz,
        _ => true,
    }
}

fn rf_antenna_measurement_check_declared(bound: &BoundBoard<'_>, measurement_name: &str) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario
                .checks
                .iter()
                .any(|declared| declared == RF_ANTENNA_MEASURED_PERFORMANCE_VALID)
            && scenario
                .parameters
                .get("rf_measurements")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .is_some_and(|measurements| {
                    measurements.iter().any(|item| {
                        item.as_mapping().and_then(|mapping| {
                            mapping
                                .get(serde_yaml_ng::Value::String("name".to_string()))
                                .and_then(serde_yaml_ng::Value::as_str)
                        }) == Some(measurement_name)
                    })
                })
    })
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

#[derive(Debug, Clone, PartialEq)]
struct BgaPitchEvidence {
    component: String,
    layer: String,
    pitch_mm: f64,
    horizontal_gaps: usize,
    vertical_gaps: usize,
}

const JLC_BGA_PITCH_INFERENCE_CANDIDATES_MM: &[f64] = &[0.4, 0.45, 0.5, 0.65, 0.8, 1.0, 1.27];

fn infer_bga_pitch_from_paste(paste: &LayoutCopper) -> Option<BgaPitchEvidence> {
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

    let mut best: Option<BgaPitchEvidence> = None;
    for ((component, layer), features) in features_by_component_layer {
        if features.len() < 4 {
            continue;
        }
        for pitch_mm in JLC_BGA_PITCH_INFERENCE_CANDIDATES_MM {
            let (horizontal_gaps, vertical_gaps) =
                count_axis_aligned_pitch_gaps(&features, *pitch_mm);
            if horizontal_gaps < 2 || vertical_gaps < 2 {
                continue;
            }
            let candidate = BgaPitchEvidence {
                component: component.clone(),
                layer: layer.clone(),
                pitch_mm: *pitch_mm,
                horizontal_gaps,
                vertical_gaps,
            };
            if best
                .as_ref()
                .is_none_or(|current| is_better_bga_pitch_evidence(&candidate, current))
            {
                best = Some(candidate);
            }
        }
    }

    best
}

fn count_axis_aligned_pitch_gaps(
    features: &[&LayoutCopperFeature],
    pitch_mm: f64,
) -> (usize, usize) {
    let mut horizontal_gaps = 0usize;
    let mut vertical_gaps = 0usize;
    for (index, first) in features.iter().enumerate() {
        for second in features.iter().skip(index + 1) {
            let dx = first.at.x_mm - second.at.x_mm;
            let dy = first.at.y_mm - second.at.y_mm;
            if dy.abs() <= IC_PIN_PITCH_INFERENCE_TOLERANCE_MM
                && (dx.abs() - pitch_mm).abs() <= IC_PIN_PITCH_INFERENCE_TOLERANCE_MM
            {
                horizontal_gaps += 1;
            }
            if dx.abs() <= IC_PIN_PITCH_INFERENCE_TOLERANCE_MM
                && (dy.abs() - pitch_mm).abs() <= IC_PIN_PITCH_INFERENCE_TOLERANCE_MM
            {
                vertical_gaps += 1;
            }
        }
    }
    (horizontal_gaps, vertical_gaps)
}

fn is_better_bga_pitch_evidence(candidate: &BgaPitchEvidence, current: &BgaPitchEvidence) -> bool {
    candidate
        .horizontal_gaps
        .min(candidate.vertical_gaps)
        .cmp(&current.horizontal_gaps.min(current.vertical_gaps))
        .then_with(|| {
            (candidate.horizontal_gaps + candidate.vertical_gaps)
                .cmp(&(current.horizontal_gaps + current.vertical_gaps))
        })
        .then_with(|| current.pitch_mm.total_cmp(&candidate.pitch_mm))
        .then_with(|| current.component.cmp(&candidate.component))
        .then_with(|| current.layer.cmp(&candidate.layer))
        .is_gt()
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
