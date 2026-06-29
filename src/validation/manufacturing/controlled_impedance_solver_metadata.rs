use crate::board_ir::{
    ControlledImpedanceSolverMaterialCorner, ControlledImpedanceSolverQualification,
    ControlledImpedanceSolverResult, ControlledImpedanceSolverResultType, Scenario, StackupLayer,
    StackupLayerKind,
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

fn solver_artifact_signature_policy_requested(result: &ControlledImpedanceSolverResult) -> bool {
    result.solver_artifact_signature_uri.is_some()
        || result.solver_artifact_signature_sha256.is_some()
        || result.solver_artifact_signer.is_some()
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
