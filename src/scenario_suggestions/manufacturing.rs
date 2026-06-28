use super::{ScenarioSuggestion, SuggestedScenario, SuggestedTarget, sanitized_name};
use crate::board_ir::{
    CopperZone, LayoutCopper, LayoutCopperFeature, LayoutPoint, NetKind, RouteSegment, RouteVia,
    StackupLayer, StackupLayerKind,
};
use crate::library::BoundBoard;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const DRILL_DIAMETER_VALID: &str = "DRILL_DIAMETER_VALID";
const DRILL_TO_BOARD_EDGE_CLEARANCE_VALID: &str = "DRILL_TO_BOARD_EDGE_CLEARANCE_VALID";
const SLOT_TO_BOARD_EDGE_CLEARANCE_VALID: &str = "SLOT_TO_BOARD_EDGE_CLEARANCE_VALID";
const SLOT_WIDTH_VALID: &str = "SLOT_WIDTH_VALID";
const SLOT_ASPECT_RATIO_VALID: &str = "SLOT_ASPECT_RATIO_VALID";
const CASTELLATED_HOLE_VALID: &str = "CASTELLATED_HOLE_VALID";
const DRILL_ANNULAR_RING_VALID: &str = "DRILL_ANNULAR_RING_VALID";
const COPPER_TO_BOARD_EDGE_CLEARANCE_VALID: &str = "COPPER_TO_BOARD_EDGE_CLEARANCE_VALID";
const COPPER_SPACING_VALID: &str = "COPPER_SPACING_VALID";
const SOLDER_MASK_OPENING_VALID: &str = "SOLDER_MASK_OPENING_VALID";
const SOLDER_MASK_DAM_VALID: &str = "SOLDER_MASK_DAM_VALID";
const SOLDER_PASTE_OPENING_VALID: &str = "SOLDER_PASTE_OPENING_VALID";
const SOLDER_PASTE_APERTURE_SIZE_VALID: &str = "SOLDER_PASTE_APERTURE_SIZE_VALID";
const SOLDER_PASTE_APERTURE_AREA_RATIO_VALID: &str = "SOLDER_PASTE_APERTURE_AREA_RATIO_VALID";
const SOLDER_PASTE_IC_PIN_APERTURE_VALID: &str = "SOLDER_PASTE_IC_PIN_APERTURE_VALID";
const SOLDER_PASTE_BGA_APERTURE_VALID: &str = "SOLDER_PASTE_BGA_APERTURE_VALID";
const SOLDER_PASTE_SPACING_VALID: &str = "SOLDER_PASTE_SPACING_VALID";
const ADJACENT_PLANE_RETURN_PATH_VALID: &str = "ADJACENT_PLANE_RETURN_PATH_VALID";
const REFERENCE_PLANE_SLOT_CROSSING_VALID: &str = "REFERENCE_PLANE_SLOT_CROSSING_VALID";
const RETURN_PATH_STITCHING_VIA_VALID: &str = "RETURN_PATH_STITCHING_VIA_VALID";
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
    suggestions.extend(adjacent_plane_return_path_suggestions(bound, &project_name));
    suggestions.extend(reference_plane_slot_crossing_suggestions(
        bound,
        &project_name,
    ));
    suggestions.extend(return_path_stitching_via_suggestions(bound, &project_name));
    suggestions.extend(pin_1_orientation_suggestions(bound, &project_name));

    suggestions
}

fn adjacent_plane_return_path_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for (net_name, net) in &bound.project.board.nets {
        if net.kind != NetKind::DigitalOrAnalog
            || adjacent_plane_check_declared_for_net(bound, net_name)
        {
            continue;
        }
        let Some(evidence) = adjacent_plane_return_path_evidence(bound, net_name) else {
            continue;
        };
        suggestions.push(manufacturing_suggestion(
            &format!("adjacent_plane_return_path_{}", sanitized_name(net_name)),
            true,
            &format!(
                "Route {net_name} has imported route segments, explicit stackup evidence, and sampled adjacent {} plane-zone coverage on {}.",
                evidence.reference_net, evidence.reference_layer
            ),
            &format!(
                "{}_{}_adjacent_plane_return_path",
                project_name,
                sanitized_name(net_name)
            ),
            ADJACENT_PLANE_RETURN_PATH_VALID,
            Some(BTreeMap::from([(
                "routes".to_string(),
                json!([{
                    "net": net_name,
                    "reference_net": evidence.reference_net,
                    "reference_layer": evidence.reference_layer,
                    "max_unreferenced_length_mm": 0.0
                }]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

fn reference_plane_slot_crossing_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for (net_name, net) in &bound.project.board.nets {
        if net.kind != NetKind::DigitalOrAnalog
            || manufacturing_route_check_declared_for_net(
                bound,
                REFERENCE_PLANE_SLOT_CROSSING_VALID,
                net_name,
            )
        {
            continue;
        }
        let Some(evidence) = reference_plane_slot_crossing_evidence(bound, net_name) else {
            continue;
        };
        suggestions.push(manufacturing_suggestion(
            &format!("reference_plane_slot_crossing_{}", sanitized_name(net_name)),
            true,
            &format!(
                "Route {net_name} has imported route segments, explicit stackup evidence, adjacent {} plane-zone evidence on {}, and {} internal reference-plane gap(s) along the route centerline.",
                evidence.reference_net, evidence.reference_layer, evidence.slot_crossing_count
            ),
            &format!(
                "{}_{}_reference_plane_slot_crossing",
                project_name,
                sanitized_name(net_name)
            ),
            REFERENCE_PLANE_SLOT_CROSSING_VALID,
            Some(BTreeMap::from([(
                "routes".to_string(),
                json!([{
                    "net": net_name,
                    "reference_net": evidence.reference_net,
                    "reference_layer": evidence.reference_layer,
                    "max_slot_crossings": 0
                }]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

fn return_path_stitching_via_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let Some(max_stitch_via_distance_mm) = bound
        .project
        .board
        .manufacturing
        .max_stitch_via_distance_mm
        .filter(|value| value.is_finite() && *value >= 0.0)
    else {
        return Vec::new();
    };
    let mut suggestions = Vec::new();
    for (net_name, net) in &bound.project.board.nets {
        if net.kind != NetKind::DigitalOrAnalog
            || manufacturing_route_check_declared_for_net(
                bound,
                RETURN_PATH_STITCHING_VIA_VALID,
                net_name,
            )
        {
            continue;
        }
        let Some(evidence) = return_path_stitching_via_evidence(bound, net_name) else {
            continue;
        };
        suggestions.push(manufacturing_suggestion(
            &format!("return_path_stitching_via_{}", sanitized_name(net_name)),
            true,
            &format!(
                "Route {net_name} has {} imported layer-transition via(s), explicit stackup evidence, {} declared {} stitching via(s), and reviewed board.manufacturing.max_stitch_via_distance_mm policy.",
                evidence.signal_via_count, evidence.reference_via_count, evidence.reference_net
            ),
            &format!(
                "{}_{}_return_path_stitching_via",
                project_name,
                sanitized_name(net_name)
            ),
            RETURN_PATH_STITCHING_VIA_VALID,
            Some(BTreeMap::from([(
                "routes".to_string(),
                json!([{
                    "net": net_name,
                    "reference_net": evidence.reference_net,
                    "max_stitch_via_distance_mm": max_stitch_via_distance_mm
                }]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

#[derive(Debug)]
struct AdjacentPlaneEvidence {
    reference_net: String,
    reference_layer: String,
}

#[derive(Debug)]
struct SlotCrossingEvidence {
    reference_net: String,
    reference_layer: String,
    slot_crossing_count: usize,
}

#[derive(Debug)]
struct StitchingViaEvidence {
    reference_net: String,
    signal_via_count: usize,
    reference_via_count: usize,
}

fn adjacent_plane_return_path_evidence(
    bound: &BoundBoard<'_>,
    net_name: &str,
) -> Option<AdjacentPlaneEvidence> {
    let route = bound.project.board.layout.routes.get(net_name)?;
    if route.segments.is_empty() || bound.project.board.layout.stackup.layers.is_empty() {
        return None;
    }
    let mut reference_net = None::<String>;
    let mut reference_layer = None::<String>;
    for segment in &route.segments {
        if !usable_route_segment(segment) {
            return None;
        }
        let layer = adjacent_reference_plane(bound, &segment.layer)?;
        let net = layer.reference_net.as_ref()?;
        let zones = bound.project.board.layout.zones.get(net)?;
        if !segment_has_plane_coverage(segment, &layer.name, zones) {
            return None;
        }
        if reference_net
            .as_deref()
            .is_some_and(|current| current != net)
        {
            return None;
        }
        if reference_layer
            .as_deref()
            .is_some_and(|current| current != layer.name)
        {
            return None;
        }
        reference_net = Some(net.clone());
        reference_layer = Some(layer.name.clone());
    }
    Some(AdjacentPlaneEvidence {
        reference_net: reference_net?,
        reference_layer: reference_layer?,
    })
}

fn reference_plane_slot_crossing_evidence(
    bound: &BoundBoard<'_>,
    net_name: &str,
) -> Option<SlotCrossingEvidence> {
    let route = bound.project.board.layout.routes.get(net_name)?;
    if route.segments.is_empty() || bound.project.board.layout.stackup.layers.is_empty() {
        return None;
    }
    let mut reference_net = None::<String>;
    let mut reference_layer = None::<String>;
    let mut slot_crossing_count = 0usize;
    for segment in &route.segments {
        if !usable_route_segment(segment) {
            return None;
        }
        let layer = adjacent_reference_plane(bound, &segment.layer)?;
        let net = layer.reference_net.as_ref()?;
        let zones = bound.project.board.layout.zones.get(net)?;
        let segment_crossings = segment_slot_crossing_count(segment, &layer.name, zones)?;
        slot_crossing_count += segment_crossings;
        if reference_net
            .as_deref()
            .is_some_and(|current| current != net)
        {
            return None;
        }
        if reference_layer
            .as_deref()
            .is_some_and(|current| current != layer.name)
        {
            return None;
        }
        reference_net = Some(net.clone());
        reference_layer = Some(layer.name.clone());
    }
    if slot_crossing_count == 0 {
        return None;
    }
    Some(SlotCrossingEvidence {
        reference_net: reference_net?,
        reference_layer: reference_layer?,
        slot_crossing_count,
    })
}

fn return_path_stitching_via_evidence(
    bound: &BoundBoard<'_>,
    net_name: &str,
) -> Option<StitchingViaEvidence> {
    let route = bound.project.board.layout.routes.get(net_name)?;
    if route.vias.is_empty() || bound.project.board.layout.stackup.layers.is_empty() {
        return None;
    }
    let stackup_layers = stackup_layer_names(bound);
    let reference_net = route_reference_net(bound, route)?;
    if !route
        .vias
        .iter()
        .all(|via| usable_route_via(via, &stackup_layers))
    {
        return None;
    }
    let reference_route = bound.project.board.layout.routes.get(&reference_net)?;
    if reference_route.vias.is_empty()
        || !reference_route
            .vias
            .iter()
            .all(|via| usable_route_via(via, &stackup_layers))
    {
        return None;
    }
    let has_matching_reference_via = route.vias.iter().any(|signal_via| {
        reference_route
            .vias
            .iter()
            .any(|reference_via| via_layers_match(signal_via, reference_via))
    });
    has_matching_reference_via.then_some(StitchingViaEvidence {
        reference_net,
        signal_via_count: route.vias.len(),
        reference_via_count: reference_route.vias.len(),
    })
}

fn route_reference_net(
    bound: &BoundBoard<'_>,
    route: &crate::board_ir::NetRoute,
) -> Option<String> {
    let mut reference_net = None::<String>;
    for segment in &route.segments {
        if !usable_route_segment(segment) {
            return None;
        }
        let layer = adjacent_reference_plane(bound, &segment.layer)?;
        let net = layer.reference_net.as_ref()?;
        if reference_net
            .as_deref()
            .is_some_and(|current| current != net)
        {
            return None;
        }
        reference_net = Some(net.clone());
    }
    reference_net
}

fn stackup_layer_names(bound: &BoundBoard<'_>) -> BTreeSet<String> {
    bound
        .project
        .board
        .layout
        .stackup
        .layers
        .iter()
        .map(|layer| layer.name.clone())
        .collect()
}

fn adjacent_reference_plane<'a>(
    bound: &'a BoundBoard<'_>,
    route_layer: &str,
) -> Option<&'a StackupLayer> {
    let layers = &bound.project.board.layout.stackup.layers;
    let route_index = layers.iter().position(|layer| layer.name == route_layer)?;
    let mut candidates = Vec::new();
    for direction in [-1, 1] {
        if let Some(layer) = nearest_conductive_layer(layers, route_index, direction)
            && layer.kind == StackupLayerKind::Plane
            && layer.reference_net.as_ref().is_some_and(|net| {
                bound
                    .project
                    .board
                    .nets
                    .get(net)
                    .is_some_and(|spec| spec.kind == NetKind::Ground)
            })
        {
            candidates.push(layer);
        }
    }
    (candidates.len() == 1).then(|| candidates[0])
}

fn nearest_conductive_layer(
    layers: &[StackupLayer],
    route_index: usize,
    direction: isize,
) -> Option<&StackupLayer> {
    let mut index = route_index as isize + direction;
    while index >= 0 && (index as usize) < layers.len() {
        let layer = &layers[index as usize];
        if layer.kind != StackupLayerKind::Dielectric {
            return Some(layer);
        }
        index += direction;
    }
    None
}

fn usable_route_segment(segment: &RouteSegment) -> bool {
    segment.start.x_mm.is_finite()
        && segment.start.y_mm.is_finite()
        && segment.end.x_mm.is_finite()
        && segment.end.y_mm.is_finite()
        && segment.width_mm.is_finite()
        && segment.width_mm > 0.0
        && !segment.layer.trim().is_empty()
        && segment_length_mm(segment) > f64::EPSILON
}

fn usable_route_via(via: &RouteVia, stackup_layers: &BTreeSet<String>) -> bool {
    via.at.x_mm.is_finite()
        && via.at.y_mm.is_finite()
        && via.size_mm.is_finite()
        && via.size_mm > 0.0
        && via.drill_mm.is_finite()
        && via.drill_mm > 0.0
        && via.layers.len() >= 2
        && via
            .layers
            .iter()
            .all(|layer| !layer.trim().is_empty() && stackup_layers.contains(layer))
}

fn via_layers_match(signal_via: &RouteVia, reference_via: &RouteVia) -> bool {
    signal_via.layers.iter().all(|layer| {
        reference_via
            .layers
            .iter()
            .any(|candidate| candidate == layer)
    })
}

fn segment_has_plane_coverage(
    segment: &RouteSegment,
    reference_layer: &str,
    zones: &[CopperZone],
) -> bool {
    let polygons = zones
        .iter()
        .filter(|zone| zone.layer == reference_layer)
        .flat_map(zone_polygons)
        .filter(|polygon| usable_polygon(polygon))
        .collect::<Vec<_>>();
    if polygons.is_empty() {
        return false;
    }
    let samples = [
        (segment.start.x_mm, segment.start.y_mm),
        (
            (segment.start.x_mm + segment.end.x_mm) / 2.0,
            (segment.start.y_mm + segment.end.y_mm) / 2.0,
        ),
        (segment.end.x_mm, segment.end.y_mm),
    ];
    samples.iter().all(|sample| {
        polygons
            .iter()
            .any(|polygon| point_in_polygon(sample.0, sample.1, polygon))
    })
}

fn zone_polygons(zone: &CopperZone) -> Box<dyn Iterator<Item = &Vec<LayoutPoint>> + '_> {
    if zone.filled_polygons.is_empty() {
        Box::new(std::iter::once(&zone.polygon))
    } else {
        Box::new(zone.filled_polygons.iter())
    }
}

fn usable_polygon(polygon: &[LayoutPoint]) -> bool {
    polygon.len() >= 3
        && polygon
            .iter()
            .all(|point| point.x_mm.is_finite() && point.y_mm.is_finite())
}

fn segment_slot_crossing_count(
    segment: &RouteSegment,
    reference_layer: &str,
    zones: &[CopperZone],
) -> Option<usize> {
    let mut intervals = Vec::new();
    for polygon in zones
        .iter()
        .filter(|zone| zone.layer == reference_layer)
        .flat_map(zone_polygons)
        .filter(|polygon| usable_polygon(polygon))
    {
        intervals.extend(segment_polygon_coverage_intervals(segment, polygon));
    }
    let merged = merge_intervals(intervals);
    (!merged.is_empty()).then(|| {
        merged
            .windows(2)
            .filter(|pair| pair[1].0 > pair[0].1 + 1.0e-9)
            .count()
    })
}

fn segment_polygon_coverage_intervals(
    segment: &RouteSegment,
    polygon: &[LayoutPoint],
) -> Vec<(f64, f64)> {
    let mut samples = vec![0.0, 1.0];
    for current in 0..polygon.len() {
        let next = (current + 1) % polygon.len();
        if let Some(t) = segment_edge_intersection_t(segment, &polygon[current], &polygon[next])
            && t.is_finite()
            && (-1.0e-9..=1.0 + 1.0e-9).contains(&t)
        {
            samples.push(t.clamp(0.0, 1.0));
        }
    }
    samples.sort_by(f64::total_cmp);
    samples.dedup_by(|a, b| (*a - *b).abs() <= 1.0e-9);

    let mut intervals = Vec::new();
    for pair in samples.windows(2) {
        if pair[1] <= pair[0] + 1.0e-9 {
            continue;
        }
        let midpoint = (pair[0] + pair[1]) / 2.0;
        let (x, y) = point_at_t(segment, midpoint);
        if point_in_polygon(x, y, polygon) {
            intervals.push((pair[0], pair[1]));
        }
    }
    for sample in samples {
        let (x, y) = point_at_t(segment, sample);
        if point_in_polygon(x, y, polygon) {
            let start = (sample - 1.0e-9).clamp(0.0, 1.0);
            let end = (sample + 1.0e-9).clamp(0.0, 1.0);
            if end > start {
                intervals.push((start, end));
            }
        }
    }
    intervals
}

fn segment_edge_intersection_t(
    segment: &RouteSegment,
    edge_start: &LayoutPoint,
    edge_end: &LayoutPoint,
) -> Option<f64> {
    let px = segment.start.x_mm;
    let py = segment.start.y_mm;
    let rx = segment.end.x_mm - segment.start.x_mm;
    let ry = segment.end.y_mm - segment.start.y_mm;
    let qx = edge_start.x_mm;
    let qy = edge_start.y_mm;
    let sx = edge_end.x_mm - edge_start.x_mm;
    let sy = edge_end.y_mm - edge_start.y_mm;
    let denominator = cross(rx, ry, sx, sy);
    if denominator.abs() <= 1.0e-12 {
        return None;
    }
    let qpx = qx - px;
    let qpy = qy - py;
    let t = cross(qpx, qpy, sx, sy) / denominator;
    let u = cross(qpx, qpy, rx, ry) / denominator;
    ((-1.0e-9..=1.0 + 1.0e-9).contains(&t) && (-1.0e-9..=1.0 + 1.0e-9).contains(&u))
        .then_some(t.clamp(0.0, 1.0))
}

fn merge_intervals(mut intervals: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    intervals.retain(|(start, end)| start.is_finite() && end.is_finite() && *end > *start + 1.0e-9);
    intervals.sort_by(|left, right| {
        left.0
            .total_cmp(&right.0)
            .then_with(|| left.1.total_cmp(&right.1))
    });
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for (start, end) in intervals {
        if let Some(last) = merged.last_mut()
            && start <= last.1 + 1.0e-9
        {
            last.1 = last.1.max(end);
            continue;
        }
        merged.push((start, end));
    }
    merged
}

fn point_in_polygon(x: f64, y: f64, polygon: &[LayoutPoint]) -> bool {
    let mut inside = false;
    let mut previous = polygon.len() - 1;
    for current in 0..polygon.len() {
        let current_point = &polygon[current];
        let previous_point = &polygon[previous];
        if point_on_segment(x, y, current_point, previous_point) {
            return true;
        }
        let intersects = ((current_point.y_mm > y) != (previous_point.y_mm > y))
            && (x
                < (previous_point.x_mm - current_point.x_mm) * (y - current_point.y_mm)
                    / (previous_point.y_mm - current_point.y_mm)
                    + current_point.x_mm);
        if intersects {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn point_on_segment(x: f64, y: f64, start: &LayoutPoint, end: &LayoutPoint) -> bool {
    let cross_product = cross(
        x - start.x_mm,
        y - start.y_mm,
        end.x_mm - start.x_mm,
        end.y_mm - start.y_mm,
    );
    if cross_product.abs() > 1.0e-9 {
        return false;
    }
    x >= start.x_mm.min(end.x_mm) - 1.0e-9
        && x <= start.x_mm.max(end.x_mm) + 1.0e-9
        && y >= start.y_mm.min(end.y_mm) - 1.0e-9
        && y <= start.y_mm.max(end.y_mm) + 1.0e-9
}

fn point_at_t(segment: &RouteSegment, t: f64) -> (f64, f64) {
    (
        segment.start.x_mm + (segment.end.x_mm - segment.start.x_mm) * t,
        segment.start.y_mm + (segment.end.y_mm - segment.start.y_mm) * t,
    )
}

fn cross(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    ax * by - ay * bx
}

fn segment_length_mm(segment: &RouteSegment) -> f64 {
    (segment.end.x_mm - segment.start.x_mm).hypot(segment.end.y_mm - segment.start.y_mm)
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

fn adjacent_plane_check_declared_for_net(bound: &BoundBoard<'_>, net_name: &str) -> bool {
    manufacturing_route_check_declared_for_net(bound, ADJACENT_PLANE_RETURN_PATH_VALID, net_name)
}

fn manufacturing_route_check_declared_for_net(
    bound: &BoundBoard<'_>,
    check: &str,
    net_name: &str,
) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario.checks.iter().any(|declared| declared == check)
            && scenario
                .parameters
                .get("routes")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .is_some_and(|routes| {
                    routes.iter().any(|route| {
                        route.as_mapping().and_then(|mapping| {
                            mapping
                                .get(serde_yaml_ng::Value::String("net".to_string()))
                                .and_then(serde_yaml_ng::Value::as_str)
                        }) == Some(net_name)
                    })
                })
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
