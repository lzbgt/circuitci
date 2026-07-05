use super::controlled_impedance_solver::is_sha256_hex;
use crate::board_ir::{
    ControlledImpedanceSolverConvergenceSample, ControlledImpedanceSolverEntitlement,
    ControlledImpedanceSolverExecutionEnvironment, ControlledImpedanceSolverRerun,
    ControlledImpedanceSolverResult, ControlledImpedanceSolverRunLog,
    ControlledImpedanceSolverRuntimeAllowlist,
};
use crate::library::BoundBoard;
use std::collections::BTreeSet;

fn controlled_impedance_solver_runtime_allowlist_policy_requested(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    result.solver_runtime_allowlist.is_some()
        || result.solver_runtime_profile.is_some()
        || !result.solver_runtime_options.is_empty()
}

pub(super) fn controlled_impedance_solver_runtime_allowlist_has_evidence(
    bound: &BoundBoard<'_>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    let allowlists = &bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .solver_runtime_allowlists;
    if !controlled_impedance_solver_runtime_allowlist_policy_requested(result) {
        return true;
    }
    if !controlled_impedance_solver_runtime_metadata_is_complete(result) {
        return false;
    }
    let matches = allowlists
        .iter()
        .filter(|allowlist| {
            controlled_impedance_solver_runtime_allowlist_matches_result(allowlist, result)
        })
        .collect::<Vec<_>>();
    matches.len() == 1
        && controlled_impedance_solver_runtime_allowlist_has_valid_metadata(matches[0])
        && result.solver_runtime_options.iter().all(|option| {
            matches[0]
                .allowed_options
                .iter()
                .any(|allowed| allowed.trim() == option.trim())
        })
}

fn controlled_impedance_solver_runtime_metadata_is_complete(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    result
        .solver_runtime_allowlist
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        && result
            .solver_runtime_profile
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && result
            .solver_config_lock_revision
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && has_unique_non_empty_values(&result.solver_runtime_options)
}

fn controlled_impedance_solver_runtime_allowlist_matches_result(
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

fn controlled_impedance_solver_runtime_allowlist_has_valid_metadata(
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

fn controlled_impedance_solver_entitlement_policy_requested(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    result.solver_entitlement.is_some() || !result.solver_entitlement_features.is_empty()
}

pub(super) fn controlled_impedance_solver_entitlement_has_evidence(
    bound: &BoundBoard<'_>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if !controlled_impedance_solver_entitlement_policy_requested(result) {
        return true;
    }
    if !controlled_impedance_solver_entitlement_metadata_is_complete(result) {
        return false;
    }
    let matches = bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .solver_entitlements
        .iter()
        .filter(|entitlement| {
            controlled_impedance_solver_entitlement_matches_result(entitlement, result)
        })
        .collect::<Vec<_>>();
    matches.len() == 1
        && controlled_impedance_solver_entitlement_has_valid_metadata(matches[0])
        && result.solver_entitlement_features.iter().all(|feature| {
            matches[0]
                .licensed_features
                .iter()
                .any(|licensed| licensed.trim() == feature.trim())
        })
}

fn controlled_impedance_solver_entitlement_metadata_is_complete(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    result
        .solver_entitlement
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        && result
            .solver_version
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && has_unique_non_empty_values(&result.solver_entitlement_features)
}

fn controlled_impedance_solver_entitlement_matches_result(
    entitlement: &ControlledImpedanceSolverEntitlement,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    result
        .solver_entitlement
        .as_deref()
        .is_some_and(|name| name.trim() == entitlement.name.trim())
        && entitlement.solver.trim() == result.solver.trim()
        && result
            .solver_version
            .as_deref()
            .is_some_and(|version| version.trim() == entitlement.solver_version.trim())
}

fn controlled_impedance_solver_entitlement_has_valid_metadata(
    entitlement: &ControlledImpedanceSolverEntitlement,
) -> bool {
    !entitlement.name.trim().is_empty()
        && !entitlement.source.trim().is_empty()
        && !entitlement.solver.trim().is_empty()
        && !entitlement.solver_version.trim().is_empty()
        && !entitlement.entitlement_id.trim().is_empty()
        && !entitlement.entitlement_revision.trim().is_empty()
        && !entitlement.artifact_uri.trim().is_empty()
        && is_sha256_hex(entitlement.artifact_sha256.trim())
        && has_unique_non_empty_values(&entitlement.licensed_features)
}

fn controlled_impedance_solver_execution_environment_policy_requested(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    result.solver_execution_environment.is_some()
        || result.solver_environment_fingerprint.is_some()
        || !result.solver_environment_components.is_empty()
}

pub(super) fn controlled_impedance_solver_execution_environment_has_evidence(
    bound: &BoundBoard<'_>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if !controlled_impedance_solver_execution_environment_policy_requested(result) {
        return true;
    }
    if !controlled_impedance_solver_execution_environment_metadata_is_complete(result) {
        return false;
    }
    let matches = bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .solver_execution_environments
        .iter()
        .filter(|environment| {
            controlled_impedance_solver_execution_environment_matches_result(environment, result)
        })
        .collect::<Vec<_>>();
    matches.len() == 1
        && controlled_impedance_solver_execution_environment_has_valid_metadata(matches[0])
        && result
            .solver_environment_components
            .iter()
            .all(|component| {
                matches[0]
                    .locked_components
                    .iter()
                    .any(|locked| locked.trim() == component.trim())
            })
}

fn controlled_impedance_solver_execution_environment_metadata_is_complete(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    result
        .solver_execution_environment
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        && result
            .solver_version
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && result
            .solver_environment_fingerprint
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && has_unique_non_empty_values(&result.solver_environment_components)
}

fn controlled_impedance_solver_execution_environment_matches_result(
    environment: &ControlledImpedanceSolverExecutionEnvironment,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    result
        .solver_execution_environment
        .as_deref()
        .is_some_and(|name| name.trim() == environment.name.trim())
        && environment.solver.trim() == result.solver.trim()
        && result
            .solver_version
            .as_deref()
            .is_some_and(|version| version.trim() == environment.solver_version.trim())
        && result
            .solver_environment_fingerprint
            .as_deref()
            .is_some_and(|fingerprint| {
                fingerprint.trim() == environment.reproducibility_fingerprint.trim()
            })
}

fn controlled_impedance_solver_execution_environment_has_valid_metadata(
    environment: &ControlledImpedanceSolverExecutionEnvironment,
) -> bool {
    !environment.name.trim().is_empty()
        && !environment.source.trim().is_empty()
        && !environment.solver.trim().is_empty()
        && !environment.solver_version.trim().is_empty()
        && !environment.environment_id.trim().is_empty()
        && !environment.environment_revision.trim().is_empty()
        && !environment.artifact_uri.trim().is_empty()
        && is_sha256_hex(environment.artifact_sha256.trim())
        && !environment.reproducibility_fingerprint.trim().is_empty()
        && has_unique_non_empty_values(&environment.locked_components)
}

fn controlled_impedance_solver_run_log_policy_requested(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    result.solver_run_log.is_some()
        || result.solver_run_id.is_some()
        || result.solver_random_seed.is_some()
        || result.solver_numeric_tolerance_policy.is_some()
        || result.solver_residual_error.is_some()
        || result.solver_iterations.is_some()
}

pub(super) fn controlled_impedance_solver_run_log_has_evidence(
    bound: &BoundBoard<'_>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if !controlled_impedance_solver_run_log_policy_requested(result) {
        return true;
    }
    if !controlled_impedance_solver_run_log_metadata_is_complete(result) {
        return false;
    }
    let matches = bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .solver_run_logs
        .iter()
        .filter(|run_log| controlled_impedance_solver_run_log_matches_result(run_log, result))
        .collect::<Vec<_>>();
    matches.len() == 1
        && controlled_impedance_solver_run_log_has_valid_metadata(matches[0])
        && result
            .solver_residual_error
            .is_some_and(|value| value <= matches[0].max_residual_error)
        && result
            .solver_iterations
            .is_some_and(|value| value <= matches[0].max_iterations)
        && controlled_impedance_solver_precision_policy_has_evidence(matches[0], result)
        && controlled_impedance_solver_run_log_reruns_have_evidence(matches[0], result)
        && controlled_impedance_solver_convergence_samples_have_evidence(matches[0], result)
}

fn controlled_impedance_solver_run_log_metadata_is_complete(
    result: &ControlledImpedanceSolverResult,
) -> bool {
    result
        .solver_run_log
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        && result
            .solver_version
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && result
            .solver_run_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && result
            .solver_random_seed
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && result
            .solver_numeric_tolerance_policy
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && result
            .solver_residual_error
            .is_some_and(|value| value.is_finite() && value >= 0.0)
        && result.solver_iterations.is_some_and(|value| value > 0)
}

fn controlled_impedance_solver_run_log_matches_result(
    run_log: &ControlledImpedanceSolverRunLog,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    result
        .solver_run_log
        .as_deref()
        .is_some_and(|name| name.trim() == run_log.name.trim())
        && run_log.solver.trim() == result.solver.trim()
        && result
            .solver_version
            .as_deref()
            .is_some_and(|version| version.trim() == run_log.solver_version.trim())
        && result
            .solver_run_id
            .as_deref()
            .is_some_and(|run_id| run_id.trim() == run_log.run_id.trim())
        && result
            .solver_random_seed
            .as_deref()
            .is_some_and(|seed| seed.trim() == run_log.random_seed.trim())
        && result
            .solver_numeric_tolerance_policy
            .as_deref()
            .is_some_and(|policy| policy.trim() == run_log.numeric_tolerance_policy.trim())
}

fn controlled_impedance_solver_run_log_has_valid_metadata(
    run_log: &ControlledImpedanceSolverRunLog,
) -> bool {
    !run_log.name.trim().is_empty()
        && !run_log.source.trim().is_empty()
        && !run_log.solver.trim().is_empty()
        && !run_log.solver_version.trim().is_empty()
        && !run_log.run_id.trim().is_empty()
        && !run_log.artifact_uri.trim().is_empty()
        && is_sha256_hex(run_log.artifact_sha256.trim())
        && !run_log.random_seed.trim().is_empty()
        && !run_log.numeric_tolerance_policy.trim().is_empty()
        && run_log.max_residual_error.is_finite()
        && run_log.max_residual_error >= 0.0
        && run_log.max_iterations > 0
        && run_log.min_rerun_count.unwrap_or(1) > 0
        && run_log
            .max_rerun_impedance_delta_ohm
            .is_none_or(|value| value.is_finite() && value >= 0.0)
        && (run_log.min_rerun_count.is_some() == run_log.max_rerun_impedance_delta_ohm.is_some())
        && controlled_impedance_solver_convergence_policy_has_valid_shape(run_log)
}

fn controlled_impedance_solver_run_log_reruns_have_evidence(
    run_log: &ControlledImpedanceSolverRunLog,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    let rerun_policy_requested =
        run_log.min_rerun_count.is_some() || run_log.max_rerun_impedance_delta_ohm.is_some();
    if !rerun_policy_requested {
        return true;
    }
    let (Some(min_rerun_count), Some(max_impedance_delta)) = (
        run_log.min_rerun_count,
        run_log.max_rerun_impedance_delta_ohm,
    ) else {
        return false;
    };
    min_rerun_count > 0
        && max_impedance_delta.is_finite()
        && max_impedance_delta >= 0.0
        && run_log.reruns.len() >= min_rerun_count
        && has_unique_named_reruns(&run_log.reruns)
        && run_log.reruns.iter().all(|rerun| {
            controlled_impedance_solver_rerun_has_valid_metadata(rerun)
                && rerun.random_seed.trim() == run_log.random_seed.trim()
                && (rerun.solved_impedance_ohm - result.solved_impedance_ohm).abs()
                    <= max_impedance_delta
                && rerun.residual_error <= run_log.max_residual_error
                && rerun.iterations <= run_log.max_iterations
        })
}

fn controlled_impedance_solver_precision_policy_has_evidence(
    run_log: &ControlledImpedanceSolverRunLog,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    let policy_requested = run_log.precision_policy_source.is_some()
        || run_log.precision_policy_artifact_uri.is_some()
        || run_log.precision_policy_artifact_sha256.is_some()
        || run_log.floating_point_precision.is_some()
        || run_log.min_significant_digits.is_some()
        || run_log.max_roundoff_error_ohm.is_some();
    if !policy_requested {
        return true;
    }
    let Some(max_roundoff) = run_log.max_roundoff_error_ohm else {
        return false;
    };
    run_log
        .precision_policy_source
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
        && run_log
            .precision_policy_artifact_uri
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && run_log
            .precision_policy_artifact_sha256
            .as_deref()
            .is_some_and(|digest| is_sha256_hex(digest.trim()))
        && run_log
            .floating_point_precision
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        && run_log
            .min_significant_digits
            .is_some_and(|digits| digits > 0)
        && max_roundoff.is_finite()
        && max_roundoff >= 0.0
        && max_roundoff <= result.max_impedance_error_ohm
        && run_log
            .max_convergence_impedance_delta_ohm
            .is_none_or(|max_delta| max_roundoff <= max_delta)
}

fn controlled_impedance_solver_rerun_has_valid_metadata(
    rerun: &ControlledImpedanceSolverRerun,
) -> bool {
    !rerun.name.trim().is_empty()
        && !rerun.source.trim().is_empty()
        && !rerun.run_id.trim().is_empty()
        && !rerun.artifact_uri.trim().is_empty()
        && is_sha256_hex(rerun.artifact_sha256.trim())
        && !rerun.random_seed.trim().is_empty()
        && rerun.solved_impedance_ohm.is_finite()
        && rerun.solved_impedance_ohm > 0.0
        && rerun.residual_error.is_finite()
        && rerun.residual_error >= 0.0
        && rerun.iterations > 0
}

fn has_unique_named_reruns(reruns: &[ControlledImpedanceSolverRerun]) -> bool {
    let mut names = BTreeSet::new();
    let mut run_ids = BTreeSet::new();
    reruns.iter().all(|rerun| {
        let name = rerun.name.trim();
        let run_id = rerun.run_id.trim();
        !name.is_empty() && !run_id.is_empty() && names.insert(name) && run_ids.insert(run_id)
    })
}

fn controlled_impedance_solver_convergence_policy_has_valid_shape(
    run_log: &ControlledImpedanceSolverRunLog,
) -> bool {
    let declared = [
        run_log.min_convergence_sample_count.is_some(),
        run_log.max_convergence_impedance_delta_ohm.is_some(),
        run_log
            .required_stopping_criteria
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()),
    ];
    let convergence_policy_valid = declared.iter().all(|declared| !declared)
        || (declared.iter().all(|declared| *declared)
            && run_log.min_convergence_sample_count.unwrap_or(1) > 0
            && run_log
                .max_convergence_impedance_delta_ohm
                .is_none_or(|value| value.is_finite() && value >= 0.0));
    convergence_policy_valid
        && (!run_log
            .require_monotonic_residual_decrease
            .is_some_and(|value| value)
            || declared.iter().all(|declared| *declared))
}

fn controlled_impedance_solver_convergence_samples_have_evidence(
    run_log: &ControlledImpedanceSolverRunLog,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    let policy_requested = run_log.min_convergence_sample_count.is_some()
        || run_log.max_convergence_impedance_delta_ohm.is_some()
        || run_log
            .required_stopping_criteria
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        || run_log
            .require_monotonic_residual_decrease
            .is_some_and(|value| value);
    if !policy_requested {
        return true;
    }
    let (Some(min_count), Some(max_impedance_delta), Some(stopping_criteria)) = (
        run_log.min_convergence_sample_count,
        run_log.max_convergence_impedance_delta_ohm,
        run_log
            .required_stopping_criteria
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty()),
    ) else {
        return false;
    };
    if run_log.convergence_samples.len() < min_count
        || !has_unique_named_convergence_samples(&run_log.convergence_samples)
    {
        return false;
    }
    let mut min_impedance = f64::INFINITY;
    let mut max_impedance = f64::NEG_INFINITY;
    for sample in &run_log.convergence_samples {
        if !controlled_impedance_solver_convergence_sample_has_valid_metadata(sample)
            || sample.stopping_criteria.trim() != stopping_criteria
            || sample.iteration > run_log.max_iterations
            || sample.residual_error > run_log.max_residual_error
            || (sample.solved_impedance_ohm - result.solved_impedance_ohm).abs()
                > max_impedance_delta
        {
            return false;
        }
        min_impedance = min_impedance.min(sample.solved_impedance_ohm);
        max_impedance = max_impedance.max(sample.solved_impedance_ohm);
    }
    max_impedance - min_impedance <= max_impedance_delta
        && (!run_log
            .require_monotonic_residual_decrease
            .is_some_and(|value| value)
            || convergence_residuals_are_monotonic(&run_log.convergence_samples))
}

fn controlled_impedance_solver_convergence_sample_has_valid_metadata(
    sample: &ControlledImpedanceSolverConvergenceSample,
) -> bool {
    !sample.name.trim().is_empty()
        && !sample.source.trim().is_empty()
        && !sample.artifact_uri.trim().is_empty()
        && is_sha256_hex(sample.artifact_sha256.trim())
        && sample.iteration > 0
        && sample.solved_impedance_ohm.is_finite()
        && sample.solved_impedance_ohm > 0.0
        && sample.residual_error.is_finite()
        && sample.residual_error >= 0.0
        && !sample.stopping_criteria.trim().is_empty()
}

fn has_unique_named_convergence_samples(
    samples: &[ControlledImpedanceSolverConvergenceSample],
) -> bool {
    let mut names = BTreeSet::new();
    let mut iterations = BTreeSet::new();
    samples.iter().all(|sample| {
        let name = sample.name.trim();
        !name.is_empty() && names.insert(name) && iterations.insert(sample.iteration)
    })
}

fn convergence_residuals_are_monotonic(
    samples: &[ControlledImpedanceSolverConvergenceSample],
) -> bool {
    let mut ordered = samples.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|sample| sample.iteration);
    ordered
        .windows(2)
        .all(|window| window[1].residual_error <= window[0].residual_error)
}

fn has_unique_non_empty_values(values: &[String]) -> bool {
    let mut seen = BTreeSet::new();
    !values.is_empty()
        && values
            .iter()
            .map(|value| value.trim())
            .all(|value| !value.is_empty() && seen.insert(value))
}
