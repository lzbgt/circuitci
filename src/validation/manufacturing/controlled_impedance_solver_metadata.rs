use crate::board_ir::{
    ControlledImpedanceSolverMaterialAcceptance, ControlledImpedanceSolverMaterialCorner,
    ControlledImpedanceSolverMaterialLibrary, ControlledImpedanceSolverMaterialProcess,
    ControlledImpedanceSolverQualification, ControlledImpedanceSolverResult,
    ControlledImpedanceSolverResultType, ControlledImpedanceSolverRuntimeAllowlist, Scenario,
    StackupLayer, StackupLayerKind,
};
use crate::library::BoundBoard;
use crate::reports::Finding;
use std::collections::BTreeSet;

use super::super::common::validation_input_missing;

pub(super) fn stackup_layers_match(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    let layers = &bound.project.board.layout.stackup.layers;
    if layers.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID requires board.layout.stackup.layers evidence.",
        );
        return false;
    }
    let Some(route_layer) = named_stackup_layer(layers, &result.route_layer) else {
        missing_layer(findings, scenario, result, &result.route_layer);
        return false;
    };
    let Some(reference_layer) = named_stackup_layer(layers, &result.reference_layer) else {
        missing_layer(findings, scenario, result, &result.reference_layer);
        return false;
    };
    let Some(dielectric_layer) = named_stackup_layer(layers, &result.dielectric_layer) else {
        missing_layer(findings, scenario, result, &result.dielectric_layer);
        return false;
    };
    if route_layer.kind != StackupLayerKind::Signal
        || reference_layer.kind != StackupLayerKind::Plane
        || dielectric_layer.kind != StackupLayerKind::Dielectric
    {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} requires route_layer kind signal, reference_layer kind plane, and dielectric_layer kind dielectric.",
                result.name
            ),
        );
        return false;
    }
    if !solver_material_corner_metadata_is_valid(scenario, findings, result, dielectric_layer) {
        return false;
    }
    true
}

pub(super) fn solver_qualification_metadata_is_valid(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
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
    let Some(version) = non_empty_option(result.solver_version.as_deref()) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} requires solver_version when reviewed solver qualification metadata exists.",
                result.name
            ),
        );
        return false;
    };
    let matches = qualifications
        .iter()
        .filter(|qualification| {
            qualification.solver.trim() == result.solver.trim()
                && qualification.solver_version.trim() == version
        })
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} requires exactly one reviewed solver qualification for solver {} version {version}; found {}.",
                result.name,
                result.solver,
                matches.len()
            ),
        );
        return false;
    }
    if !solver_qualification_has_valid_metadata(matches[0]) {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID solver qualification {} for result {} must declare non-empty name/source/solver/version/artifact URI and a 64-character SHA-256 digest.",
                matches[0].name, result.name
            ),
        );
        return false;
    }
    true
}

pub(super) fn solver_artifact_signature_metadata_is_valid(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if !solver_artifact_signature_policy_requested(result) {
        return true;
    }
    let Some(_) = non_empty_option(result.solver_artifact_signature_uri.as_deref()) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} signed-artifact evidence requires non-empty solver_artifact_signature_uri.",
                result.name
            ),
        );
        return false;
    };
    let Some(signature_sha256) =
        non_empty_option(result.solver_artifact_signature_sha256.as_deref())
    else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} signed-artifact evidence requires solver_artifact_signature_sha256 as a 64-character SHA-256 hex digest.",
                result.name
            ),
        );
        return false;
    };
    if !is_sha256_hex(signature_sha256)
        || non_empty_option(result.solver_artifact_signer.as_deref()).is_none()
    {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} signed-artifact evidence must declare non-empty solver_artifact_signer and a 64-character solver_artifact_signature_sha256 digest.",
                result.name
            ),
        );
        return false;
    }
    true
}

pub(super) fn solver_output_schema_metadata_is_valid(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if !solver_output_schema_policy_requested(result) {
        return true;
    }
    let schema = non_empty_option(result.solver_output_schema.as_deref());
    let version = non_empty_option(result.solver_output_schema_version.as_deref());
    let uri = non_empty_option(result.solver_output_schema_uri.as_deref());
    let sha256 = non_empty_option(result.solver_output_schema_sha256.as_deref());
    if schema.is_none() || version.is_none() || uri.is_none() || !sha256.is_some_and(is_sha256_hex)
    {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} output-schema evidence must declare non-empty solver_output_schema, solver_output_schema_version, solver_output_schema_uri, and a 64-character solver_output_schema_sha256 digest.",
                result.name
            ),
        );
        return false;
    }
    true
}

pub(super) fn solver_config_lock_metadata_is_valid(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if !solver_config_lock_policy_requested(result) {
        return true;
    }
    let uri = non_empty_option(result.solver_config_lock_uri.as_deref());
    let sha256 = non_empty_option(result.solver_config_lock_sha256.as_deref());
    let tool = non_empty_option(result.solver_config_lock_tool.as_deref());
    let revision = non_empty_option(result.solver_config_lock_revision.as_deref());
    if uri.is_none() || !sha256.is_some_and(is_sha256_hex) || tool.is_none() || revision.is_none() {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} solver config-lock evidence must declare non-empty solver_config_lock_uri, solver_config_lock_tool, solver_config_lock_revision, and a 64-character solver_config_lock_sha256 digest.",
                result.name
            ),
        );
        return false;
    }
    if tool != Some(result.solver.trim()) {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} solver_config_lock_tool must match reviewed solver {}.",
                result.name, result.solver
            ),
        );
        return false;
    }
    true
}

pub(super) fn solver_runtime_allowlist_metadata_is_valid(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    let allowlists = &bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .solver_runtime_allowlists;
    if !solver_runtime_allowlist_policy_requested(result) {
        return true;
    }
    if !solver_runtime_allowlist_metadata_is_complete(result) {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} runtime allowlist evidence must declare non-empty solver_runtime_allowlist, solver_runtime_profile, solver_runtime_options, and solver_config_lock_revision.",
                result.name
            ),
        );
        return false;
    }
    let matches = allowlists
        .iter()
        .filter(|allowlist| solver_runtime_allowlist_matches_result(allowlist, result))
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} requires exactly one reviewed solver runtime allowlist for solver {} config lock {}; found {}.",
                result.name,
                result.solver,
                result
                    .solver_config_lock_revision
                    .as_deref()
                    .unwrap_or_default(),
                matches.len()
            ),
        );
        return false;
    }
    let allowlist = matches[0];
    if !solver_runtime_allowlist_has_valid_metadata(allowlist) {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID runtime allowlist {} for result {} must declare non-empty source/solver/config-lock/profile/revision/artifact metadata, a 64-character SHA-256 digest, and unique allowed_options.",
                allowlist.name, result.name
            ),
        );
        return false;
    }
    let allowed_options = trimmed_set(&allowlist.allowed_options);
    for option in &result.solver_runtime_options {
        if !allowed_options.contains(option.trim()) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} solver runtime option {} is not in reviewed allowlist {}.",
                    result.name, option, allowlist.name
                ),
            );
            return false;
        }
    }
    true
}

pub(super) fn solver_material_library_artifact_metadata_is_valid(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if !solver_material_library_policy_requested(result) {
        return true;
    }
    if !solver_material_library_metadata_is_complete(result) {
        return true;
    }
    let libraries = &bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .solver_material_libraries;
    let matches = libraries
        .iter()
        .filter(|library| solver_material_library_matches_result(library, result))
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} requires exactly one reviewed solver material-library artifact content row for library {} revision {} artifact {}; found {}.",
                result.name,
                result
                    .solver_material_library
                    .as_deref()
                    .unwrap_or_default(),
                result
                    .solver_material_library_revision
                    .as_deref()
                    .unwrap_or_default(),
                result
                    .solver_material_library_artifact_sha256
                    .as_deref()
                    .unwrap_or_default(),
                matches.len()
            ),
        );
        return false;
    }
    let library = matches[0];
    if !solver_material_library_has_valid_metadata(library) {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID material library {} for result {} must declare non-empty source/artifact metadata, a 64-character SHA-256 digest, and non-empty corners/layers/materials/content_fields.",
                library.name, result.name
            ),
        );
        return false;
    }
    let library_fields = trimmed_set(&library.content_fields);
    for required_field in required_material_library_content_fields() {
        if !library_fields.contains(required_field) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID material library {} for result {} does not declare required artifact content field {}.",
                    library.name, result.name, required_field
                ),
            );
            return false;
        }
    }
    let library_corners = trimmed_set(&library.corners);
    for required_corner in &result.required_solver_corners {
        if !library_corners.contains(required_corner.trim()) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID material library {} for result {} does not declare required corner {}.",
                    library.name, result.name, required_corner
                ),
            );
            return false;
        }
    }
    let library_layers = trimmed_set(&library.dielectric_layers);
    let library_materials = trimmed_set(&library.materials);
    for corner in &result.material_corners {
        if !library_corners.contains(corner.corner.trim())
            || !library_layers.contains(corner.dielectric_layer.trim())
            || !library_materials.contains(corner.material.trim())
        {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID material corner {} for result {} is not backed by reviewed material-library artifact content {}.",
                    corner.name, result.name, library.name
                ),
            );
            return false;
        }
    }
    if !solver_material_acceptance_metadata_is_valid(bound, scenario, findings, result) {
        return false;
    }
    if !solver_material_process_metadata_is_valid(bound, scenario, findings, result) {
        return false;
    }
    true
}

pub(super) fn solver_stackup_signoff_metadata_is_valid(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if !solver_stackup_signoff_policy_requested(result) {
        return true;
    }
    if !solver_stackup_signoff_metadata_is_complete(result) {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} stackup signoff evidence must declare non-empty stackup_signoff_source, fabricator_stackup_revision, stackup_signoff_artifact_uri, and a 64-character stackup_signoff_artifact_sha256 digest.",
                result.name
            ),
        );
        return false;
    }
    let fabricator_revision = result
        .fabricator_stackup_revision
        .as_deref()
        .map(str::trim)
        .unwrap_or_default();
    if fabricator_revision != result.stackup_revision.trim() {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} fabricator_stackup_revision {fabricator_revision} must match reviewed solver stackup_revision {}.",
                result.name, result.stackup_revision
            ),
        );
        return false;
    }
    true
}

pub(super) fn solver_input_deck_policy_requested(result: &ControlledImpedanceSolverResult) -> bool {
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

pub(super) fn solver_input_deck_metadata_is_valid(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if !solver_input_deck_policy_requested(result) {
        return true;
    }
    let Some(_) = non_empty_option(result.solver_input_deck_uri.as_deref()) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} input-deck evidence requires non-empty solver_input_deck_uri.",
                result.name
            ),
        );
        return false;
    };
    let Some(input_sha256) = non_empty_option(result.solver_input_deck_sha256.as_deref()) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} input-deck evidence requires solver_input_deck_sha256 as a 64-character SHA-256 hex digest.",
                result.name
            ),
        );
        return false;
    };
    if !is_sha256_hex(input_sha256)
        || non_empty_option(result.input_stackup_revision.as_deref()).is_none()
        || non_empty_option(result.input_route_layer.as_deref()).is_none()
        || non_empty_option(result.input_reference_layer.as_deref()).is_none()
        || non_empty_option(result.input_dielectric_layer.as_deref()).is_none()
        || !positive_option(result.input_width_mm)
        || result
            .input_frequency_mhz
            .is_some_and(|value| !positive(value))
        || !solver_roughness_metadata_is_complete(result)
        || !solver_etch_compensation_metadata_is_complete(result)
        || !solver_material_library_metadata_is_complete(result)
    {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} input-deck evidence must declare digest, stackup/layer setup, positive input_width_mm, optional positive input_frequency_mhz, complete positive copper roughness metadata when roughness evidence is declared, complete positive etch compensation metadata when etch evidence is declared, and complete solver material-library metadata when material-library evidence is declared.",
                result.name
            ),
        );
        return false;
    }
    true
}

pub(super) fn solver_input_deck_matches_result(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    let mismatches = solver_input_deck_mismatches(result);
    if mismatches.is_empty() {
        return true;
    }
    if mismatches.contains(&"input_frequency_mhz_missing") {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} declares solver frequency_mhz but input-deck evidence omits input_frequency_mhz.",
                result.name
            ),
        );
        return true;
    }
    false
}

pub(super) fn solver_input_deck_mismatches(
    result: &ControlledImpedanceSolverResult,
) -> Vec<&'static str> {
    let mut mismatches = Vec::new();
    if result.input_stackup_revision.as_deref().map(str::trim)
        != Some(result.stackup_revision.as_str())
    {
        mismatches.push("stackup_revision");
    }
    if result.input_route_layer.as_deref().map(str::trim) != Some(result.route_layer.as_str()) {
        mismatches.push("route_layer");
    }
    if result.input_reference_layer.as_deref().map(str::trim)
        != Some(result.reference_layer.as_str())
    {
        mismatches.push("reference_layer");
    }
    if result.input_dielectric_layer.as_deref().map(str::trim)
        != Some(result.dielectric_layer.as_str())
    {
        mismatches.push("dielectric_layer");
    }
    if result
        .input_width_mm
        .is_some_and(|value| (value - result.solved_width_mm).abs() > f64::EPSILON)
    {
        mismatches.push("solved_width_mm");
    }
    if solver_material_library_policy_requested(result) {
        if result.input_material_library.as_deref().map(str::trim)
            != result.solver_material_library.as_deref().map(str::trim)
        {
            mismatches.push("solver_material_library");
        }
        if result
            .input_material_library_revision
            .as_deref()
            .map(str::trim)
            != result
                .solver_material_library_revision
                .as_deref()
                .map(str::trim)
        {
            mismatches.push("solver_material_library_revision");
        }
    }
    match result.result_type {
        ControlledImpedanceSolverResultType::SingleEnded => {}
        ControlledImpedanceSolverResultType::Differential => {
            if let (Some(input_gap), Some(solved_gap)) = (result.input_gap_mm, result.solved_gap_mm)
                && (input_gap - solved_gap).abs() > f64::EPSILON
            {
                mismatches.push("solved_gap_mm");
            }
        }
    }
    if let Some(frequency_mhz) = result.frequency_mhz {
        match result.input_frequency_mhz {
            Some(input_frequency) => {
                if (input_frequency - frequency_mhz).abs() > f64::EPSILON {
                    mismatches.push("frequency_mhz");
                }
            }
            None => mismatches.push("input_frequency_mhz_missing"),
        }
    }
    if result.copper_roughness_model.as_deref().map(str::trim)
        != result
            .input_copper_roughness_model
            .as_deref()
            .map(str::trim)
    {
        mismatches.push("copper_roughness_model");
    }
    if let (Some(roughness), Some(input_roughness)) =
        (result.copper_roughness_um, result.input_copper_roughness_um)
        && (input_roughness - roughness).abs() > f64::EPSILON
    {
        mismatches.push("copper_roughness_um");
    }
    if result.etch_compensation_model.as_deref().map(str::trim)
        != result
            .input_etch_compensation_model
            .as_deref()
            .map(str::trim)
    {
        mismatches.push("etch_compensation_model");
    }
    if let (Some(compensation), Some(input_compensation)) = (
        result.etch_compensation_um,
        result.input_etch_compensation_um,
    ) && (input_compensation - compensation).abs() > f64::EPSILON
    {
        mismatches.push("etch_compensation_um");
    }
    mismatches
}

fn solver_material_corner_metadata_is_valid(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    result: &ControlledImpedanceSolverResult,
    stackup_dielectric_layer: &StackupLayer,
) -> bool {
    if result.material_corners.is_empty() {
        return true;
    }
    if !solver_material_library_policy_requested(result)
        || !solver_material_library_metadata_is_complete(result)
    {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} material_corners require complete solver material-library evidence.",
                result.name
            ),
        );
        return false;
    }
    if result.required_solver_corners.is_empty() {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} material_corners require non-empty required_solver_corners.",
                result.name
            ),
        );
        return false;
    }
    let mut names = BTreeSet::new();
    let mut corner_keys = BTreeSet::new();
    for corner in &result.material_corners {
        if !solver_material_corner_has_valid_metadata(corner) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} material corner {} must declare non-empty name/source/corner/layer/material/library metadata and positive dielectric constants.",
                    result.name, corner.name
                ),
            );
            return false;
        }
        if !names.insert(corner.name.trim().to_string()) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} repeats material corner name {}.",
                    result.name, corner.name
                ),
            );
            return false;
        }
        let key = (
            corner.corner.trim().to_string(),
            corner.dielectric_layer.trim().to_string(),
        );
        if !corner_keys.insert(key) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} repeats material-corner evidence for corner {} layer {}.",
                    result.name, corner.corner, corner.dielectric_layer
                ),
            );
            return false;
        }
    }
    let required_corners: BTreeSet<&str> = result
        .required_solver_corners
        .iter()
        .map(|corner| corner.trim())
        .collect();
    for corner in &result.material_corners {
        if !required_corners.contains(corner.corner.trim()) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} material corner {} is not listed in required_solver_corners.",
                    result.name, corner.corner
                ),
            );
            return false;
        }
        if corner.dielectric_layer.trim() != result.dielectric_layer.trim() {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} material corner {} references dielectric_layer {} but solver result uses {}.",
                    result.name, corner.name, corner.dielectric_layer, result.dielectric_layer
                ),
            );
            return false;
        }
        if solver_material_library_policy_requested(result)
            && (result.solver_material_library.as_deref().map(str::trim)
                != Some(corner.material_library.trim())
                || result
                    .solver_material_library_revision
                    .as_deref()
                    .map(str::trim)
                    != Some(corner.material_library_revision.trim()))
        {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} material corner {} must match the solver material library and revision.",
                    result.name, corner.name
                ),
            );
            return false;
        }
        if let Some(material) = stackup_dielectric_layer.material.as_deref()
            && material.trim() != corner.material.trim()
        {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} material corner {} material {} does not match stackup layer {} material {}.",
                    result.name,
                    corner.name,
                    corner.material,
                    stackup_dielectric_layer.name,
                    material
                ),
            );
            return false;
        }
        if let Some(stackup_dk) = stackup_dielectric_layer.dielectric_constant
            && (stackup_dk - corner.nominal_dielectric_constant).abs() > f64::EPSILON
        {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} material corner {} nominal_dielectric_constant must match reviewed stackup layer {} dielectric_constant.",
                    result.name, corner.name, stackup_dielectric_layer.name
                ),
            );
            return false;
        }
    }
    for required_corner in required_corners {
        if !result
            .material_corners
            .iter()
            .any(|corner| corner.corner.trim() == required_corner)
        {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} requires material-corner evidence for solver corner {}.",
                    result.name, required_corner
                ),
            );
            return false;
        }
    }
    true
}

fn solver_material_corner_has_valid_metadata(
    corner: &ControlledImpedanceSolverMaterialCorner,
) -> bool {
    !corner.name.trim().is_empty()
        && !corner.source.trim().is_empty()
        && !corner.corner.trim().is_empty()
        && !corner.dielectric_layer.trim().is_empty()
        && !corner.material.trim().is_empty()
        && positive(corner.dielectric_constant)
        && positive(corner.nominal_dielectric_constant)
        && !corner.material_library.trim().is_empty()
        && !corner.material_library_revision.trim().is_empty()
}

fn missing_layer(
    findings: &mut Vec<Finding>,
    scenario: &Scenario,
    result: &ControlledImpedanceSolverResult,
    layer: &str,
) {
    validation_input_missing(
        findings,
        scenario,
        format!(
            "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} references stackup layer {layer} absent from board.layout.stackup.layers.",
            result.name
        ),
    );
}

fn solver_qualification_has_valid_metadata(
    qualification: &ControlledImpedanceSolverQualification,
) -> bool {
    !qualification.name.trim().is_empty()
        && !qualification.source.trim().is_empty()
        && !qualification.solver.trim().is_empty()
        && !qualification.solver_version.trim().is_empty()
        && !qualification.qualification_artifact_uri.trim().is_empty()
        && is_sha256_hex(qualification.qualification_artifact_sha256.trim())
}

fn solver_material_library_matches_result(
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

fn solver_material_library_has_valid_metadata(
    library: &ControlledImpedanceSolverMaterialLibrary,
) -> bool {
    !library.name.trim().is_empty()
        && !library.source.trim().is_empty()
        && !library.material_library.trim().is_empty()
        && !library.material_library_revision.trim().is_empty()
        && !library.artifact_uri.trim().is_empty()
        && is_sha256_hex(library.artifact_sha256.trim())
        && !trimmed_set(&library.corners).is_empty()
        && !trimmed_set(&library.dielectric_layers).is_empty()
        && !trimmed_set(&library.materials).is_empty()
        && !trimmed_set(&library.content_fields).is_empty()
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

fn solver_material_acceptance_metadata_is_valid(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
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
            solver_material_acceptance_matches_result(acceptance, result, fabricator_revision)
        })
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} requires exactly one reviewed solver material acceptance row for library {} revision {} fabricator stackup revision {fabricator_revision}; found {}.",
                result.name,
                result
                    .solver_material_library
                    .as_deref()
                    .unwrap_or_default(),
                result
                    .solver_material_library_revision
                    .as_deref()
                    .unwrap_or_default(),
                matches.len()
            ),
        );
        return false;
    }
    let acceptance = matches[0];
    if !solver_material_acceptance_has_valid_metadata(acceptance) {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID material acceptance {} for result {} must declare non-empty source/library/revision/fabricator/artifact metadata, a 64-character SHA-256 digest, and non-empty accepted corners/layers/materials.",
                acceptance.name, result.name
            ),
        );
        return false;
    }
    let accepted_corners = trimmed_set(&acceptance.accepted_corners);
    for required_corner in &result.required_solver_corners {
        if !accepted_corners.contains(required_corner.trim()) {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID material acceptance {} for result {} does not accept required corner {}.",
                    acceptance.name, result.name, required_corner
                ),
            );
            return false;
        }
    }
    let accepted_layers = trimmed_set(&acceptance.accepted_dielectric_layers);
    let accepted_materials = trimmed_set(&acceptance.accepted_materials);
    if !accepted_layers.contains(result.dielectric_layer.trim()) {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID material acceptance {} for result {} does not accept dielectric layer {}.",
                acceptance.name, result.name, result.dielectric_layer
            ),
        );
        return false;
    }
    for corner in &result.material_corners {
        if !accepted_corners.contains(corner.corner.trim())
            || !accepted_layers.contains(corner.dielectric_layer.trim())
            || !accepted_materials.contains(corner.material.trim())
        {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID material corner {} for result {} is not covered by reviewed material acceptance {}.",
                    corner.name, result.name, acceptance.name
                ),
            );
            return false;
        }
    }
    if let Some(dielectric_layer) = named_stackup_layer(
        &bound.project.board.layout.stackup.layers,
        &result.dielectric_layer,
    ) && let Some(material) = dielectric_layer.material.as_deref()
        && !accepted_materials.contains(material.trim())
    {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID material acceptance {} for result {} does not accept stackup material {}.",
                acceptance.name, result.name, material
            ),
        );
        return false;
    }
    true
}

fn solver_material_acceptance_matches_result(
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

fn solver_material_acceptance_has_valid_metadata(
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

fn solver_material_process_metadata_is_valid(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
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
    let Some(material) = solver_result_material(bound, result) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} requires reviewed stackup or material-corner material evidence when solver material process rows exist.",
                result.name
            ),
        );
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
            solver_material_process_matches_result(process, result, fabricator_revision, &material)
        })
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} requires exactly one reviewed solver material process row for library {} revision {} fabricator stackup revision {fabricator_revision} dielectric layer {} material {material}; found {}.",
                result.name,
                result
                    .solver_material_library
                    .as_deref()
                    .unwrap_or_default(),
                result
                    .solver_material_library_revision
                    .as_deref()
                    .unwrap_or_default(),
                result.dielectric_layer,
                matches.len()
            ),
        );
        return false;
    }
    let process = matches[0];
    if !solver_material_process_has_valid_metadata(process) {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID material process {} for result {} must declare non-empty source/library/revision/fabricator/layer/material/lot/artifact metadata, a 64-character SHA-256 digest, positive Dk/thickness values, and non-negative drift limits.",
                process.name, result.name
            ),
        );
        return false;
    }
    if (process.measured_dielectric_constant - process.accepted_dielectric_constant).abs()
        > process.max_dielectric_constant_delta + f64::EPSILON
    {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID material process {} for result {} exceeds reviewed dielectric-constant drift limit.",
                process.name, result.name
            ),
        );
        return false;
    }
    if (process.measured_thickness_mm - process.accepted_thickness_mm).abs()
        > process.max_thickness_delta_mm + f64::EPSILON
    {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID material process {} for result {} exceeds reviewed dielectric-thickness drift limit.",
                process.name, result.name
            ),
        );
        return false;
    }
    if let Some(dielectric_layer) = named_stackup_layer(
        &bound.project.board.layout.stackup.layers,
        &result.dielectric_layer,
    ) {
        if let Some(stackup_dk) = dielectric_layer.dielectric_constant
            && (stackup_dk - process.accepted_dielectric_constant).abs() > f64::EPSILON
        {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID material process {} for result {} accepted_dielectric_constant must match reviewed stackup layer {} dielectric_constant.",
                    process.name, result.name, dielectric_layer.name
                ),
            );
            return false;
        }
        if let Some(stackup_thickness) = dielectric_layer.thickness_mm
            && (stackup_thickness - process.accepted_thickness_mm).abs() > f64::EPSILON
        {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID material process {} for result {} accepted_thickness_mm must match reviewed stackup layer {} thickness_mm.",
                    process.name, result.name, dielectric_layer.name
                ),
            );
            return false;
        }
    }
    true
}

fn solver_result_material(
    bound: &BoundBoard<'_>,
    result: &ControlledImpedanceSolverResult,
) -> Option<String> {
    if let Some(layer) = named_stackup_layer(
        &bound.project.board.layout.stackup.layers,
        &result.dielectric_layer,
    ) && let Some(material) = layer.material.as_deref()
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

fn solver_material_process_matches_result(
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

fn solver_material_process_has_valid_metadata(
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
        && positive(process.accepted_dielectric_constant)
        && positive(process.measured_dielectric_constant)
        && process.max_dielectric_constant_delta.is_finite()
        && process.max_dielectric_constant_delta >= 0.0
        && positive(process.accepted_thickness_mm)
        && positive(process.measured_thickness_mm)
        && process.max_thickness_delta_mm.is_finite()
        && process.max_thickness_delta_mm >= 0.0
}

fn trimmed_set(values: &[String]) -> BTreeSet<&str> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .collect()
}

fn solver_artifact_signature_policy_requested(result: &ControlledImpedanceSolverResult) -> bool {
    result.solver_artifact_signature_uri.is_some()
        || result.solver_artifact_signature_sha256.is_some()
        || result.solver_artifact_signer.is_some()
}

fn solver_output_schema_policy_requested(result: &ControlledImpedanceSolverResult) -> bool {
    result.solver_output_schema.is_some()
        || result.solver_output_schema_version.is_some()
        || result.solver_output_schema_uri.is_some()
        || result.solver_output_schema_sha256.is_some()
}

fn solver_config_lock_policy_requested(result: &ControlledImpedanceSolverResult) -> bool {
    result.solver_config_lock_uri.is_some()
        || result.solver_config_lock_sha256.is_some()
        || result.solver_config_lock_tool.is_some()
        || result.solver_config_lock_revision.is_some()
}

fn solver_runtime_allowlist_policy_requested(result: &ControlledImpedanceSolverResult) -> bool {
    result.solver_runtime_allowlist.is_some()
        || result.solver_runtime_profile.is_some()
        || !result.solver_runtime_options.is_empty()
}

fn solver_runtime_allowlist_metadata_is_complete(result: &ControlledImpedanceSolverResult) -> bool {
    if !solver_runtime_allowlist_policy_requested(result) {
        return true;
    }
    non_empty_option(result.solver_runtime_allowlist.as_deref()).is_some()
        && non_empty_option(result.solver_runtime_profile.as_deref()).is_some()
        && non_empty_option(result.solver_config_lock_revision.as_deref()).is_some()
        && has_unique_non_empty_values(&result.solver_runtime_options)
}

fn solver_runtime_allowlist_matches_result(
    allowlist: &ControlledImpedanceSolverRuntimeAllowlist,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    result
        .solver_runtime_allowlist
        .as_deref()
        .is_some_and(|name| name.trim() == allowlist.name.trim())
        && allowlist.solver.trim() == result.solver.trim()
        && result
            .solver_config_lock_revision
            .as_deref()
            .is_some_and(|revision| revision.trim() == allowlist.solver_config_lock_revision.trim())
        && result
            .solver_runtime_profile
            .as_deref()
            .is_some_and(|profile| profile.trim() == allowlist.runtime_profile.trim())
}

fn solver_runtime_allowlist_has_valid_metadata(
    allowlist: &ControlledImpedanceSolverRuntimeAllowlist,
) -> bool {
    !allowlist.name.trim().is_empty()
        && !allowlist.source.trim().is_empty()
        && !allowlist.solver.trim().is_empty()
        && !allowlist.solver_config_lock_revision.trim().is_empty()
        && !allowlist.runtime_profile.trim().is_empty()
        && !allowlist.allowlist_revision.trim().is_empty()
        && !allowlist.artifact_uri.trim().is_empty()
        && is_sha256_hex(allowlist.artifact_sha256.trim())
        && has_unique_non_empty_values(&allowlist.allowed_options)
}

fn has_unique_non_empty_values(values: &[String]) -> bool {
    let mut seen = BTreeSet::new();
    !values.is_empty()
        && values
            .iter()
            .map(|value| value.trim())
            .all(|value| !value.is_empty() && seen.insert(value))
}

fn solver_stackup_signoff_metadata_is_complete(result: &ControlledImpedanceSolverResult) -> bool {
    non_empty_option(result.stackup_signoff_source.as_deref()).is_some()
        && non_empty_option(result.fabricator_stackup_revision.as_deref()).is_some()
        && non_empty_option(result.stackup_signoff_artifact_uri.as_deref()).is_some()
        && result
            .stackup_signoff_artifact_sha256
            .as_deref()
            .is_some_and(is_sha256_hex)
}

fn solver_stackup_signoff_policy_requested(result: &ControlledImpedanceSolverResult) -> bool {
    result.stackup_signoff_source.is_some()
        || result.fabricator_stackup_revision.is_some()
        || result.stackup_signoff_artifact_uri.is_some()
        || result.stackup_signoff_artifact_sha256.is_some()
}

fn solver_etch_compensation_metadata_is_complete(result: &ControlledImpedanceSolverResult) -> bool {
    if !solver_etch_compensation_policy_requested(result) {
        return true;
    }
    non_empty_option(result.etch_compensation_model.as_deref()).is_some()
        && positive_option(result.etch_compensation_um)
        && non_empty_option(result.input_etch_compensation_model.as_deref()).is_some()
        && positive_option(result.input_etch_compensation_um)
}

fn solver_etch_compensation_policy_requested(result: &ControlledImpedanceSolverResult) -> bool {
    result.etch_compensation_model.is_some()
        || result.etch_compensation_um.is_some()
        || result.input_etch_compensation_model.is_some()
        || result.input_etch_compensation_um.is_some()
}

fn solver_roughness_metadata_is_complete(result: &ControlledImpedanceSolverResult) -> bool {
    if !solver_roughness_policy_requested(result) {
        return true;
    }
    non_empty_option(result.copper_roughness_model.as_deref()).is_some()
        && positive_option(result.copper_roughness_um)
        && non_empty_option(result.input_copper_roughness_model.as_deref()).is_some()
        && positive_option(result.input_copper_roughness_um)
}

fn solver_roughness_policy_requested(result: &ControlledImpedanceSolverResult) -> bool {
    result.copper_roughness_model.is_some()
        || result.copper_roughness_um.is_some()
        || result.input_copper_roughness_model.is_some()
        || result.input_copper_roughness_um.is_some()
}

fn solver_material_library_metadata_is_complete(result: &ControlledImpedanceSolverResult) -> bool {
    if !solver_material_library_policy_requested(result) {
        return true;
    }
    non_empty_option(result.solver_material_library.as_deref()).is_some()
        && non_empty_option(result.solver_material_library_revision.as_deref()).is_some()
        && non_empty_option(result.solver_material_library_artifact_uri.as_deref()).is_some()
        && result
            .solver_material_library_artifact_sha256
            .as_deref()
            .is_some_and(is_sha256_hex)
        && non_empty_option(result.input_material_library.as_deref()).is_some()
        && non_empty_option(result.input_material_library_revision.as_deref()).is_some()
}

fn solver_material_library_policy_requested(result: &ControlledImpedanceSolverResult) -> bool {
    result.solver_material_library.is_some()
        || result.solver_material_library_revision.is_some()
        || result.solver_material_library_artifact_uri.is_some()
        || result.solver_material_library_artifact_sha256.is_some()
        || result.input_material_library.is_some()
        || result.input_material_library_revision.is_some()
}

fn named_stackup_layer<'a>(layers: &'a [StackupLayer], name: &str) -> Option<&'a StackupLayer> {
    layers.iter().find(|layer| layer.name == name)
}

fn non_empty_option(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn positive_option(value: Option<f64>) -> bool {
    value.is_some_and(positive)
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
