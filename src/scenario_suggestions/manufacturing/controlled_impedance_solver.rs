use super::controlled_impedance::{
    non_negative_finite, positive_finite, routes_have_parallel_gap_evidence,
    unordered_pair_matches, usable_route_segment,
};
use super::manufacturing_suggestion;
use crate::board_ir::{
    ControlledImpedanceSolverMaterialAcceptance, ControlledImpedanceSolverMaterialLibrary,
    ControlledImpedanceSolverMaterialProcess, ControlledImpedanceSolverResult,
    ControlledImpedanceSolverResultType, NetRoute, StackupLayer, StackupLayerKind,
};
use crate::library::BoundBoard;
use crate::scenario_suggestions::{ScenarioSuggestion, sanitized_name};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

const CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID: &str = "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID";

pub(super) fn controlled_impedance_solver_result_suggestions(
    bound: &BoundBoard<'_>,
    project_name: &str,
) -> Vec<ScenarioSuggestion> {
    let mut suggestions = Vec::new();
    for result in &bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .solver_results
    {
        if controlled_impedance_solver_result_check_declared(bound, &result.name)
            || !controlled_impedance_solver_result_has_evidence(bound, result)
        {
            continue;
        }
        suggestions.push(manufacturing_suggestion(
            &format!(
                "controlled_impedance_solver_result_{}",
                sanitized_name(&result.name)
            ),
            true,
            &format!(
                "Controlled-impedance solver result {} has reviewed source evidence from {}, a source-backed solver artifact digest, matching board target metadata, explicit stackup layers, and imported route geometry.",
                result.name, result.source
            ),
            &format!(
                "{}_{}_controlled_impedance_solver_result",
                project_name,
                sanitized_name(&result.name)
            ),
            CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID,
            Some(BTreeMap::from([(
                "solver_results".to_string(),
                json!([{ "name": result.name }]),
            )])),
            Vec::new(),
        ));
    }
    suggestions
}

fn controlled_impedance_solver_result_has_evidence(
    bound: &BoundBoard<'_>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    !result.name.trim().is_empty()
        && !result.source.trim().is_empty()
        && !result.solver.trim().is_empty()
        && result
            .solver_artifact_uri
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && result
            .solver_artifact_sha256
            .as_deref()
            .is_some_and(|value| is_sha256_hex(value.trim()))
        && controlled_impedance_solver_artifact_signature_has_evidence(result)
        && controlled_impedance_solver_output_schema_has_evidence(result)
        && controlled_impedance_solver_config_lock_has_evidence(result)
        && !result.stackup_revision.trim().is_empty()
        && !result.route_layer.trim().is_empty()
        && !result.reference_layer.trim().is_empty()
        && !result.dielectric_layer.trim().is_empty()
        && positive_finite(result.target_impedance_ohm)
        && positive_finite(result.solved_impedance_ohm)
        && non_negative_finite(result.max_impedance_error_ohm)
        && positive_finite(result.solved_width_mm)
        && non_negative_finite(result.max_route_width_delta_mm)
        && result.frequency_mhz.is_none_or(positive_finite)
        && controlled_impedance_solver_input_deck_has_evidence(result)
        && controlled_impedance_solver_material_library_artifact_has_evidence(bound, result)
        && controlled_impedance_solver_stackup_signoff_has_evidence(result)
        && controlled_impedance_solver_qualification_has_evidence(bound, result)
        && controlled_impedance_solver_sweep_has_evidence(result)
        && solver_stackup_has_evidence(bound, result)
        && match result.result_type {
            ControlledImpedanceSolverResultType::SingleEnded => result
                .net
                .as_deref()
                .map(str::trim)
                .filter(|net| !net.is_empty())
                .is_some_and(|net| {
                    result.first_net.is_none()
                        && result.second_net.is_none()
                        && result.solved_gap_mm.is_none()
                        && result.max_route_gap_delta_mm.is_none()
                        && result.input_gap_mm.is_none()
                        && bound.project.board.nets.contains_key(net)
                        && matching_single_ended_solver_target(bound, result, net)
                        && bound
                            .project
                            .board
                            .layout
                            .routes
                            .get(net)
                            .is_some_and(|route| {
                                route_has_layer_segments(route, &result.route_layer)
                            })
                }),
            ControlledImpedanceSolverResultType::Differential => {
                if result.net.is_some()
                    || !result.solved_gap_mm.is_some_and(positive_finite)
                    || !result
                        .max_route_gap_delta_mm
                        .is_some_and(non_negative_finite)
                    || (controlled_impedance_solver_input_deck_policy_requested(result)
                        && !result.input_gap_mm.is_some_and(positive_finite))
                {
                    return false;
                }
                let Some(first_net) = result
                    .first_net
                    .as_deref()
                    .map(str::trim)
                    .filter(|net| !net.is_empty())
                else {
                    return false;
                };
                let Some(second_net) = result
                    .second_net
                    .as_deref()
                    .map(str::trim)
                    .filter(|net| !net.is_empty())
                else {
                    return false;
                };
                first_net != second_net
                    && bound.project.board.nets.contains_key(first_net)
                    && bound.project.board.nets.contains_key(second_net)
                    && matching_differential_solver_target(bound, result, first_net, second_net)
                    && bound
                        .project
                        .board
                        .layout
                        .routes
                        .get(first_net)
                        .is_some_and(|first_route| {
                            bound
                                .project
                                .board
                                .layout
                                .routes
                                .get(second_net)
                                .is_some_and(|second_route| {
                                    route_has_layer_segments(first_route, &result.route_layer)
                                        && route_has_layer_segments(
                                            second_route,
                                            &result.route_layer,
                                        )
                                        && routes_have_parallel_gap_evidence(
                                            first_route,
                                            second_route,
                                        )
                                })
                        })
            }
        }
}

fn controlled_impedance_solver_qualification_has_evidence(
    bound: &BoundBoard<'_>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    let qualifications = &bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .solver_qualifications;
    if qualifications.is_empty() {
        return true;
    }
    let Some(version) = result
        .solver_version
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return false;
    };
    let matches = qualifications
        .iter()
        .filter(|qualification| {
            qualification.solver.trim() == result.solver.trim()
                && qualification.solver_version.trim() == version
        })
        .collect::<Vec<_>>();
    matches.len() == 1
        && !matches[0].name.trim().is_empty()
        && !matches[0].source.trim().is_empty()
        && !matches[0].qualification_artifact_uri.trim().is_empty()
        && is_sha256_hex(matches[0].qualification_artifact_sha256.trim())
}

fn controlled_impedance_solver_input_deck_policy_requested(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    result.solver_input_deck_uri.is_some()
        || result.solver_input_deck_sha256.is_some()
        || result.input_stackup_revision.is_some()
        || result.input_route_layer.is_some()
        || result.input_reference_layer.is_some()
        || result.input_dielectric_layer.is_some()
        || result.input_width_mm.is_some()
        || result.input_gap_mm.is_some()
        || result.input_frequency_mhz.is_some()
        || result.copper_roughness_model.is_some()
        || result.copper_roughness_um.is_some()
        || result.input_copper_roughness_model.is_some()
        || result.input_copper_roughness_um.is_some()
        || result.etch_compensation_model.is_some()
        || result.etch_compensation_um.is_some()
        || result.input_etch_compensation_model.is_some()
        || result.input_etch_compensation_um.is_some()
        || result.solver_material_library.is_some()
        || result.solver_material_library_revision.is_some()
        || result.solver_material_library_artifact_uri.is_some()
        || result.solver_material_library_artifact_sha256.is_some()
        || result.input_material_library.is_some()
        || result.input_material_library_revision.is_some()
}

fn controlled_impedance_solver_artifact_signature_policy_requested(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    result.solver_artifact_signature_uri.is_some()
        || result.solver_artifact_signature_sha256.is_some()
        || result.solver_artifact_signer.is_some()
}

fn controlled_impedance_solver_artifact_signature_has_evidence(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if !controlled_impedance_solver_artifact_signature_policy_requested(result) {
        return true;
    }
    result
        .solver_artifact_signature_uri
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        && result
            .solver_artifact_signature_sha256
            .as_deref()
            .is_some_and(|value| is_sha256_hex(value.trim()))
        && result
            .solver_artifact_signer
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

fn controlled_impedance_solver_output_schema_policy_requested(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    result.solver_output_schema.is_some()
        || result.solver_output_schema_version.is_some()
        || result.solver_output_schema_uri.is_some()
        || result.solver_output_schema_sha256.is_some()
}

fn controlled_impedance_solver_output_schema_has_evidence(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if !controlled_impedance_solver_output_schema_policy_requested(result) {
        return true;
    }
    result
        .solver_output_schema
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        && result
            .solver_output_schema_version
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && result
            .solver_output_schema_uri
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && result
            .solver_output_schema_sha256
            .as_deref()
            .is_some_and(|value| is_sha256_hex(value.trim()))
}

fn controlled_impedance_solver_config_lock_policy_requested(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    result.solver_config_lock_uri.is_some()
        || result.solver_config_lock_sha256.is_some()
        || result.solver_config_lock_tool.is_some()
        || result.solver_config_lock_revision.is_some()
}

fn controlled_impedance_solver_config_lock_has_evidence(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if !controlled_impedance_solver_config_lock_policy_requested(result) {
        return true;
    }
    result
        .solver_config_lock_uri
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        && result
            .solver_config_lock_sha256
            .as_deref()
            .is_some_and(|value| is_sha256_hex(value.trim()))
        && result
            .solver_config_lock_tool
            .as_deref()
            .is_some_and(|value| value.trim() == result.solver.trim())
        && result
            .solver_config_lock_revision
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
}

fn controlled_impedance_solver_input_deck_has_evidence(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if !controlled_impedance_solver_input_deck_policy_requested(result) {
        return true;
    }
    result
        .solver_input_deck_uri
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        && result
            .solver_input_deck_sha256
            .as_deref()
            .is_some_and(|value| is_sha256_hex(value.trim()))
        && result
            .input_stackup_revision
            .as_deref()
            .is_some_and(|value| value.trim() == result.stackup_revision)
        && result
            .input_route_layer
            .as_deref()
            .is_some_and(|value| value.trim() == result.route_layer)
        && result
            .input_reference_layer
            .as_deref()
            .is_some_and(|value| value.trim() == result.reference_layer)
        && result
            .input_dielectric_layer
            .as_deref()
            .is_some_and(|value| value.trim() == result.dielectric_layer)
        && result
            .input_width_mm
            .is_some_and(|value| (value - result.solved_width_mm).abs() <= f64::EPSILON)
        && controlled_impedance_solver_roughness_has_evidence(result)
        && controlled_impedance_solver_etch_compensation_has_evidence(result)
        && controlled_impedance_solver_material_library_has_evidence(result)
        && match result.result_type {
            ControlledImpedanceSolverResultType::SingleEnded => result.input_gap_mm.is_none(),
            ControlledImpedanceSolverResultType::Differential => {
                if let (Some(input_gap), Some(solved_gap)) =
                    (result.input_gap_mm, result.solved_gap_mm)
                {
                    (input_gap - solved_gap).abs() <= f64::EPSILON
                } else {
                    false
                }
            }
        }
        && match (result.frequency_mhz, result.input_frequency_mhz) {
            (Some(frequency), Some(input_frequency)) => {
                (input_frequency - frequency).abs() <= f64::EPSILON
            }
            (Some(_), None) => false,
            (None, Some(input_frequency)) => positive_finite(input_frequency),
            (None, None) => true,
        }
}

fn controlled_impedance_solver_etch_compensation_has_evidence(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if !controlled_impedance_solver_etch_compensation_policy_requested(result) {
        return true;
    }
    result
        .etch_compensation_model
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        && result.etch_compensation_um.is_some_and(positive_finite)
        && result
            .input_etch_compensation_model
            .as_deref()
            .is_some_and(|value| {
                result
                    .etch_compensation_model
                    .as_deref()
                    .is_some_and(|model| value.trim() == model.trim())
            })
        && result
            .input_etch_compensation_um
            .zip(result.etch_compensation_um)
            .is_some_and(|(input, solved)| (input - solved).abs() <= f64::EPSILON)
}

fn controlled_impedance_solver_etch_compensation_policy_requested(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    result.etch_compensation_model.is_some()
        || result.etch_compensation_um.is_some()
        || result.input_etch_compensation_model.is_some()
        || result.input_etch_compensation_um.is_some()
}

fn controlled_impedance_solver_roughness_has_evidence(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if !controlled_impedance_solver_roughness_policy_requested(result) {
        return true;
    }
    result
        .copper_roughness_model
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        && result.copper_roughness_um.is_some_and(positive_finite)
        && result
            .input_copper_roughness_model
            .as_deref()
            .is_some_and(|value| {
                result
                    .copper_roughness_model
                    .as_deref()
                    .is_some_and(|model| value.trim() == model.trim())
            })
        && result
            .input_copper_roughness_um
            .zip(result.copper_roughness_um)
            .is_some_and(|(input, solved)| (input - solved).abs() <= f64::EPSILON)
}

fn controlled_impedance_solver_roughness_policy_requested(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    result.copper_roughness_model.is_some()
        || result.copper_roughness_um.is_some()
        || result.input_copper_roughness_model.is_some()
        || result.input_copper_roughness_um.is_some()
}

fn controlled_impedance_solver_material_library_has_evidence(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if !controlled_impedance_solver_material_library_policy_requested(result) {
        return true;
    }
    result
        .solver_material_library
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        && result
            .solver_material_library_revision
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && result
            .solver_material_library_artifact_uri
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && result
            .solver_material_library_artifact_sha256
            .as_deref()
            .is_some_and(|value| is_sha256_hex(value.trim()))
        && result
            .input_material_library
            .as_deref()
            .is_some_and(|value| {
                result
                    .solver_material_library
                    .as_deref()
                    .is_some_and(|library| value.trim() == library.trim())
            })
        && result
            .input_material_library_revision
            .as_deref()
            .is_some_and(|value| {
                result
                    .solver_material_library_revision
                    .as_deref()
                    .is_some_and(|revision| value.trim() == revision.trim())
            })
}

fn controlled_impedance_solver_material_library_policy_requested(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    result.solver_material_library.is_some()
        || result.solver_material_library_revision.is_some()
        || result.solver_material_library_artifact_uri.is_some()
        || result.solver_material_library_artifact_sha256.is_some()
        || result.input_material_library.is_some()
        || result.input_material_library_revision.is_some()
}

fn controlled_impedance_solver_material_library_artifact_has_evidence(
    bound: &BoundBoard<'_>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if !controlled_impedance_solver_material_library_policy_requested(result) {
        return true;
    }
    if !controlled_impedance_solver_material_library_has_evidence(result) {
        return false;
    }
    let matches = bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .solver_material_libraries
        .iter()
        .filter(|library| {
            controlled_impedance_solver_material_library_matches_result(library, result)
        })
        .collect::<Vec<_>>();
    matches.len() == 1
        && controlled_impedance_solver_material_library_manifest_has_content(matches[0])
        && required_material_library_content_fields()
            .iter()
            .all(|field| {
                matches[0]
                    .content_fields
                    .iter()
                    .any(|value| value.trim() == *field)
            })
        && result.required_solver_corners.iter().all(|corner| {
            let corner = corner.trim();
            !corner.is_empty()
                && matches[0]
                    .corners
                    .iter()
                    .any(|library_corner| library_corner.trim() == corner)
        })
        && result.material_corners.iter().all(|corner| {
            matches[0]
                .corners
                .iter()
                .any(|value| value.trim() == corner.corner.trim())
                && matches[0]
                    .dielectric_layers
                    .iter()
                    .any(|value| value.trim() == corner.dielectric_layer.trim())
                && matches[0]
                    .materials
                    .iter()
                    .any(|value| value.trim() == corner.material.trim())
        })
        && controlled_impedance_solver_material_acceptance_has_evidence(bound, result)
        && controlled_impedance_solver_material_process_has_evidence(bound, result)
}

fn controlled_impedance_solver_material_library_matches_result(
    library: &ControlledImpedanceSolverMaterialLibrary,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    result
        .solver_material_library
        .as_deref()
        .is_some_and(|value| value.trim() == library.material_library.trim())
        && result
            .solver_material_library_revision
            .as_deref()
            .is_some_and(|value| value.trim() == library.material_library_revision.trim())
        && result
            .solver_material_library_artifact_uri
            .as_deref()
            .is_some_and(|value| value.trim() == library.artifact_uri.trim())
        && result
            .solver_material_library_artifact_sha256
            .as_deref()
            .is_some_and(|value| value.trim() == library.artifact_sha256.trim())
}

fn controlled_impedance_solver_material_library_manifest_has_content(
    library: &ControlledImpedanceSolverMaterialLibrary,
) -> bool {
    !library.name.trim().is_empty()
        && !library.source.trim().is_empty()
        && !library.artifact_uri.trim().is_empty()
        && is_sha256_hex(library.artifact_sha256.trim())
        && library.corners.iter().any(|value| !value.trim().is_empty())
        && library
            .dielectric_layers
            .iter()
            .any(|value| !value.trim().is_empty())
        && library
            .materials
            .iter()
            .any(|value| !value.trim().is_empty())
        && library
            .content_fields
            .iter()
            .any(|value| !value.trim().is_empty())
}

fn required_material_library_content_fields() -> [&'static str; 5] {
    [
        "corner",
        "dielectric_layer",
        "material",
        "dielectric_constant",
        "nominal_dielectric_constant",
    ]
}

fn controlled_impedance_solver_material_acceptance_has_evidence(
    bound: &BoundBoard<'_>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    let acceptances = &bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .solver_material_acceptances;
    if acceptances.is_empty() {
        return true;
    }
    let fabricator_revision = result
        .fabricator_stackup_revision
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(result.stackup_revision.trim());
    let matches = acceptances
        .iter()
        .filter(|acceptance| {
            controlled_impedance_solver_material_acceptance_matches_result(
                acceptance,
                result,
                fabricator_revision,
            )
        })
        .collect::<Vec<_>>();
    if matches.len() != 1
        || !controlled_impedance_solver_material_acceptance_manifest_has_content(matches[0])
    {
        return false;
    }
    let acceptance = matches[0];
    let accepted_corners = trimmed_set(&acceptance.accepted_corners);
    let accepted_layers = trimmed_set(&acceptance.accepted_dielectric_layers);
    let accepted_materials = trimmed_set(&acceptance.accepted_materials);
    if !accepted_layers.contains(result.dielectric_layer.trim())
        || result.required_solver_corners.iter().any(|corner| {
            let corner = corner.trim();
            corner.is_empty() || !accepted_corners.contains(corner)
        })
    {
        return false;
    }
    if result.material_corners.iter().any(|corner| {
        !accepted_corners.contains(corner.corner.trim())
            || !accepted_layers.contains(corner.dielectric_layer.trim())
            || !accepted_materials.contains(corner.material.trim())
    }) {
        return false;
    }
    if let Some(dielectric_layer) = bound
        .project
        .board
        .layout
        .stackup
        .layers
        .iter()
        .find(|layer| layer.name == result.dielectric_layer)
        && let Some(material) = dielectric_layer.material.as_deref()
        && !accepted_materials.contains(material.trim())
    {
        return false;
    }
    true
}

fn controlled_impedance_solver_material_acceptance_matches_result(
    acceptance: &ControlledImpedanceSolverMaterialAcceptance,
    result: &ControlledImpedanceSolverResult,
    fabricator_revision: &str,
) -> bool {
    result
        .solver_material_library
        .as_deref()
        .is_some_and(|value| value.trim() == acceptance.material_library.trim())
        && result
            .solver_material_library_revision
            .as_deref()
            .is_some_and(|value| value.trim() == acceptance.material_library_revision.trim())
        && acceptance.fabricator_stackup_revision.trim() == fabricator_revision
}

fn controlled_impedance_solver_material_acceptance_manifest_has_content(
    acceptance: &ControlledImpedanceSolverMaterialAcceptance,
) -> bool {
    !acceptance.name.trim().is_empty()
        && !acceptance.source.trim().is_empty()
        && !acceptance.material_library.trim().is_empty()
        && !acceptance.material_library_revision.trim().is_empty()
        && !acceptance.fabricator_stackup_revision.trim().is_empty()
        && !acceptance.acceptance_artifact_uri.trim().is_empty()
        && is_sha256_hex(acceptance.acceptance_artifact_sha256.trim())
        && acceptance
            .accepted_by
            .as_deref()
            .is_none_or(|value| !value.trim().is_empty())
        && !trimmed_set(&acceptance.accepted_corners).is_empty()
        && !trimmed_set(&acceptance.accepted_dielectric_layers).is_empty()
        && !trimmed_set(&acceptance.accepted_materials).is_empty()
}

fn controlled_impedance_solver_material_process_has_evidence(
    bound: &BoundBoard<'_>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    let processes = &bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .solver_material_processes;
    if processes.is_empty() {
        return true;
    }
    let Some(material) = controlled_impedance_solver_result_material(bound, result) else {
        return false;
    };
    let fabricator_revision = result
        .fabricator_stackup_revision
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(result.stackup_revision.trim());
    let matches = processes
        .iter()
        .filter(|process| {
            controlled_impedance_solver_material_process_matches_result(
                process,
                result,
                fabricator_revision,
                &material,
            )
        })
        .collect::<Vec<_>>();
    if matches.len() != 1
        || !controlled_impedance_solver_material_process_manifest_has_content(matches[0])
    {
        return false;
    }
    let process = matches[0];
    if (process.measured_dielectric_constant - process.accepted_dielectric_constant).abs()
        > process.max_dielectric_constant_delta + f64::EPSILON
        || (process.measured_thickness_mm - process.accepted_thickness_mm).abs()
            > process.max_thickness_delta_mm + f64::EPSILON
    {
        return false;
    }
    if let Some(dielectric_layer) = bound
        .project
        .board
        .layout
        .stackup
        .layers
        .iter()
        .find(|layer| layer.name == result.dielectric_layer)
    {
        if let Some(stackup_dk) = dielectric_layer.dielectric_constant
            && (stackup_dk - process.accepted_dielectric_constant).abs() > f64::EPSILON
        {
            return false;
        }
        if let Some(stackup_thickness) = dielectric_layer.thickness_mm
            && (stackup_thickness - process.accepted_thickness_mm).abs() > f64::EPSILON
        {
            return false;
        }
    }
    true
}

fn controlled_impedance_solver_result_material(
    bound: &BoundBoard<'_>,
    result: &ControlledImpedanceSolverResult,
) -> Option<String> {
    if let Some(layer) = bound
        .project
        .board
        .layout
        .stackup
        .layers
        .iter()
        .find(|layer| layer.name == result.dielectric_layer)
        && let Some(material) = layer.material.as_deref()
    {
        let material = material.trim();
        if !material.is_empty() {
            return Some(material.to_string());
        }
    }
    let materials = result
        .material_corners
        .iter()
        .filter(|corner| corner.dielectric_layer.trim() == result.dielectric_layer.trim())
        .map(|corner| corner.material.trim())
        .filter(|material| !material.is_empty())
        .collect::<BTreeSet<_>>();
    if materials.len() == 1 {
        materials
            .iter()
            .next()
            .map(|material| (*material).to_string())
    } else {
        None
    }
}

fn controlled_impedance_solver_material_process_matches_result(
    process: &ControlledImpedanceSolverMaterialProcess,
    result: &ControlledImpedanceSolverResult,
    fabricator_revision: &str,
    material: &str,
) -> bool {
    result
        .solver_material_library
        .as_deref()
        .is_some_and(|value| value.trim() == process.material_library.trim())
        && result
            .solver_material_library_revision
            .as_deref()
            .is_some_and(|value| value.trim() == process.material_library_revision.trim())
        && process.fabricator_stackup_revision.trim() == fabricator_revision
        && process.dielectric_layer.trim() == result.dielectric_layer.trim()
        && process.material.trim() == material
}

fn controlled_impedance_solver_material_process_manifest_has_content(
    process: &ControlledImpedanceSolverMaterialProcess,
) -> bool {
    !process.name.trim().is_empty()
        && !process.source.trim().is_empty()
        && !process.material_library.trim().is_empty()
        && !process.material_library_revision.trim().is_empty()
        && !process.fabricator_stackup_revision.trim().is_empty()
        && !process.dielectric_layer.trim().is_empty()
        && !process.material.trim().is_empty()
        && !process.process_lot.trim().is_empty()
        && !process.material_lot.trim().is_empty()
        && !process.process_revision.trim().is_empty()
        && !process.drift_artifact_uri.trim().is_empty()
        && is_sha256_hex(process.drift_artifact_sha256.trim())
        && positive_finite(process.accepted_dielectric_constant)
        && positive_finite(process.measured_dielectric_constant)
        && non_negative_finite(process.max_dielectric_constant_delta)
        && positive_finite(process.accepted_thickness_mm)
        && positive_finite(process.measured_thickness_mm)
        && non_negative_finite(process.max_thickness_delta_mm)
}

fn trimmed_set(values: &[String]) -> BTreeSet<&str> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .collect()
}

fn controlled_impedance_solver_sweep_has_evidence(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if result.min_solver_sample_count.is_none()
        && result.max_solver_frequency_step_mhz.is_none()
        && result.required_solver_corners.is_empty()
    {
        return true;
    }
    if result
        .min_solver_sample_count
        .is_some_and(|count| count == 0)
        || result
            .max_solver_frequency_step_mhz
            .is_some_and(|step| !positive_finite(step))
        || result.samples.is_empty()
    {
        return false;
    }
    let mut required_corners = BTreeSet::new();
    for corner in &result.required_solver_corners {
        let corner = corner.trim();
        if corner.is_empty() || !required_corners.insert(corner.to_string()) {
            return false;
        }
    }
    let mut sample_names = BTreeSet::new();
    let mut sample_corners = BTreeSet::new();
    let mut corner_frequencies: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for sample in &result.samples {
        if sample.name.trim().is_empty()
            || sample.source.trim().is_empty()
            || sample.corner.trim().is_empty()
            || !positive_finite(sample.frequency_mhz)
            || !positive_finite(sample.solved_impedance_ohm)
            || !sample_names.insert(sample.name.trim().to_string())
            || (sample.solved_impedance_ohm - result.target_impedance_ohm).abs()
                > result.max_impedance_error_ohm + f64::EPSILON
        {
            return false;
        }
        let corner = sample.corner.trim().to_string();
        sample_corners.insert(corner.clone());
        corner_frequencies
            .entry(corner)
            .or_default()
            .push(sample.frequency_mhz);
    }
    if let Some(min_count) = result.min_solver_sample_count
        && result.samples.len() < min_count
    {
        return false;
    }
    if required_corners
        .iter()
        .any(|corner| !sample_corners.contains(corner))
    {
        return false;
    }
    if let Some(max_step) = result.max_solver_frequency_step_mhz {
        for frequencies in corner_frequencies.values_mut() {
            if frequencies.len() < 2 {
                return false;
            }
            frequencies.sort_by(|a, b| a.total_cmp(b));
            if frequencies
                .windows(2)
                .any(|window| window[1] - window[0] > max_step + f64::EPSILON)
            {
                return false;
            }
        }
    }
    true
}

fn controlled_impedance_solver_stackup_signoff_has_evidence(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if !controlled_impedance_solver_stackup_signoff_policy_requested(result) {
        return true;
    }
    result
        .stackup_signoff_source
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        && result
            .fabricator_stackup_revision
            .as_deref()
            .is_some_and(|value| value.trim() == result.stackup_revision.trim())
        && result
            .stackup_signoff_artifact_uri
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && result
            .stackup_signoff_artifact_sha256
            .as_deref()
            .is_some_and(|value| is_sha256_hex(value.trim()))
}

fn controlled_impedance_solver_stackup_signoff_policy_requested(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    result.stackup_signoff_source.is_some()
        || result.fabricator_stackup_revision.is_some()
        || result.stackup_signoff_artifact_uri.is_some()
        || result.stackup_signoff_artifact_sha256.is_some()
}

fn solver_stackup_has_evidence(
    bound: &BoundBoard<'_>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    let layers = &bound.project.board.layout.stackup.layers;
    let Some(dielectric_layer) = layers
        .iter()
        .find(|layer| layer.name == result.dielectric_layer)
    else {
        return false;
    };
    layers
        .iter()
        .find(|layer| layer.name == result.route_layer)
        .is_some_and(|layer| layer.kind == StackupLayerKind::Signal)
        && layers
            .iter()
            .find(|layer| layer.name == result.reference_layer)
            .is_some_and(|layer| layer.kind == StackupLayerKind::Plane)
        && dielectric_layer.kind == StackupLayerKind::Dielectric
        && controlled_impedance_solver_material_corners_have_evidence(result, dielectric_layer)
}

fn controlled_impedance_solver_material_corners_have_evidence(
    result: &ControlledImpedanceSolverResult,
    dielectric_layer: &StackupLayer,
) -> bool {
    if result.material_corners.is_empty() {
        return true;
    }
    if !controlled_impedance_solver_material_library_policy_requested(result)
        || !controlled_impedance_solver_material_library_has_evidence(result)
    {
        return false;
    }
    if result.required_solver_corners.is_empty() {
        return false;
    }
    let mut names = BTreeSet::new();
    let mut corner_keys = BTreeSet::new();
    let required_corners: BTreeSet<&str> = result
        .required_solver_corners
        .iter()
        .map(|corner| corner.trim())
        .collect();
    for corner in &result.material_corners {
        if corner.name.trim().is_empty()
            || corner.source.trim().is_empty()
            || corner.corner.trim().is_empty()
            || corner.dielectric_layer.trim().is_empty()
            || corner.material.trim().is_empty()
            || !positive_finite(corner.dielectric_constant)
            || !positive_finite(corner.nominal_dielectric_constant)
            || corner.material_library.trim().is_empty()
            || corner.material_library_revision.trim().is_empty()
            || !names.insert(corner.name.trim().to_string())
            || !corner_keys.insert((
                corner.corner.trim().to_string(),
                corner.dielectric_layer.trim().to_string(),
            ))
            || !required_corners.contains(corner.corner.trim())
            || corner.dielectric_layer.trim() != result.dielectric_layer.trim()
        {
            return false;
        }
        if controlled_impedance_solver_material_library_policy_requested(result)
            && (result.solver_material_library.as_deref().map(str::trim)
                != Some(corner.material_library.trim())
                || result
                    .solver_material_library_revision
                    .as_deref()
                    .map(str::trim)
                    != Some(corner.material_library_revision.trim()))
        {
            return false;
        }
        if let Some(material) = dielectric_layer.material.as_deref()
            && material.trim() != corner.material.trim()
        {
            return false;
        }
        if let Some(stackup_dk) = dielectric_layer.dielectric_constant
            && (stackup_dk - corner.nominal_dielectric_constant).abs() > f64::EPSILON
        {
            return false;
        }
    }
    required_corners.iter().all(|required_corner| {
        result
            .material_corners
            .iter()
            .any(|corner| corner.corner.trim() == *required_corner)
    })
}

fn route_has_layer_segments(route: &NetRoute, layer: &str) -> bool {
    route
        .segments
        .iter()
        .any(|segment| segment.layer == layer && usable_route_segment(segment))
}

fn matching_single_ended_solver_target(
    bound: &BoundBoard<'_>,
    result: &ControlledImpedanceSolverResult,
    net: &str,
) -> bool {
    let targets = bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .nets
        .iter()
        .filter(|target| target.net == net)
        .collect::<Vec<_>>();
    targets.len() == 1
        && positive_finite(targets[0].target_impedance_ohm)
        && (targets[0].target_impedance_ohm - result.target_impedance_ohm).abs() <= 1.0e-9
}

fn matching_differential_solver_target(
    bound: &BoundBoard<'_>,
    result: &ControlledImpedanceSolverResult,
    first_net: &str,
    second_net: &str,
) -> bool {
    let targets = bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .differential_pairs
        .iter()
        .filter(|target| {
            unordered_pair_matches(&target.first_net, &target.second_net, first_net, second_net)
        })
        .collect::<Vec<_>>();
    targets.len() == 1
        && positive_finite(targets[0].target_differential_impedance_ohm)
        && (targets[0].target_differential_impedance_ohm - result.target_impedance_ohm).abs()
            <= 1.0e-9
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn controlled_impedance_solver_result_check_declared(
    bound: &BoundBoard<'_>,
    result_name: &str,
) -> bool {
    bound.project.scenarios.iter().any(|scenario| {
        scenario.scenario_type == "manufacturing"
            && scenario
                .checks
                .iter()
                .any(|declared| declared == CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID)
            && scenario
                .parameters
                .get("solver_results")
                .and_then(serde_yaml_ng::Value::as_sequence)
                .is_some_and(|results| {
                    results.iter().any(|item| {
                        item.as_mapping().and_then(|mapping| {
                            mapping
                                .get(serde_yaml_ng::Value::String("name".to_string()))
                                .and_then(serde_yaml_ng::Value::as_str)
                        }) == Some(result_name)
                    })
                })
    })
}
