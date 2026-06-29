use crate::board_ir::{
    ControlledImpedanceSolverConvergenceSample, ControlledImpedanceSolverRerun,
    ControlledImpedanceSolverResult, ControlledImpedanceSolverRunLog, Scenario,
};
use crate::library::BoundBoard;
use crate::reports::Finding;
use std::collections::BTreeSet;

use super::super::common::validation_input_missing;

pub(super) fn solver_run_log_metadata_is_valid(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    result: &ControlledImpedanceSolverResult,
) -> bool {
    if !solver_run_log_policy_requested(result) {
        return true;
    }
    if !solver_run_log_metadata_is_complete(result) {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} run-log evidence must declare non-empty solver_run_log, solver_version, solver_run_id, solver_random_seed, solver_numeric_tolerance_policy, non-negative solver_residual_error, and positive solver_iterations.",
                result.name
            ),
        );
        return false;
    }
    let matches = bound
        .project
        .board
        .manufacturing
        .controlled_impedance
        .solver_run_logs
        .iter()
        .filter(|run_log| solver_run_log_matches_result(run_log, result))
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} requires exactly one reviewed solver run log for solver {} version {} run {}; found {}.",
                result.name,
                result.solver,
                result.solver_version.as_deref().unwrap_or_default(),
                result.solver_run_id.as_deref().unwrap_or_default(),
                matches.len()
            ),
        );
        return false;
    }
    let run_log = matches[0];
    if !solver_run_log_has_valid_metadata(run_log) {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID solver run log {} for result {} must declare non-empty source/solver/version/run/artifact/seed/tolerance metadata, a 64-character SHA-256 digest, non-negative max_residual_error, and positive max_iterations.",
                run_log.name, result.name
            ),
        );
        return false;
    }
    let residual = result.solver_residual_error.unwrap_or(f64::INFINITY);
    if !residual.is_finite() || residual < 0.0 || residual > run_log.max_residual_error {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} solver residual error {} exceeds reviewed run-log limit {}.",
                result.name, residual, run_log.max_residual_error
            ),
        );
        return false;
    }
    let iterations = result.solver_iterations.unwrap_or(usize::MAX);
    if iterations == 0 || iterations > run_log.max_iterations {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID result {} solver iterations {} exceed reviewed run-log limit {}.",
                result.name, iterations, run_log.max_iterations
            ),
        );
        return false;
    }
    if !solver_run_log_reruns_are_valid(scenario, findings, result, run_log) {
        return false;
    }
    if !solver_run_log_convergence_samples_are_valid(scenario, findings, result, run_log) {
        return false;
    }
    true
}

fn solver_run_log_policy_requested(result: &ControlledImpedanceSolverResult) -> bool {
    result.solver_run_log.is_some()
        || result.solver_run_id.is_some()
        || result.solver_random_seed.is_some()
        || result.solver_numeric_tolerance_policy.is_some()
        || result.solver_residual_error.is_some()
        || result.solver_iterations.is_some()
}

fn solver_run_log_metadata_is_complete(result: &ControlledImpedanceSolverResult) -> bool {
    if !solver_run_log_policy_requested(result) {
        return true;
    }
    non_empty_option(result.solver_run_log.as_deref()).is_some()
        && non_empty_option(result.solver_version.as_deref()).is_some()
        && non_empty_option(result.solver_run_id.as_deref()).is_some()
        && non_empty_option(result.solver_random_seed.as_deref()).is_some()
        && non_empty_option(result.solver_numeric_tolerance_policy.as_deref()).is_some()
        && result
            .solver_residual_error
            .is_some_and(|value| value.is_finite() && value >= 0.0)
        && result.solver_iterations.is_some_and(|value| value > 0)
}

fn solver_run_log_matches_result(
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

fn solver_run_log_has_valid_metadata(run_log: &ControlledImpedanceSolverRunLog) -> bool {
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
        && solver_convergence_policy_has_valid_shape(run_log)
}

fn solver_run_log_reruns_are_valid(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    result: &ControlledImpedanceSolverResult,
    run_log: &ControlledImpedanceSolverRunLog,
) -> bool {
    let rerun_policy_requested =
        run_log.min_rerun_count.is_some() || run_log.max_rerun_impedance_delta_ohm.is_some();
    if !rerun_policy_requested {
        return true;
    }
    let Some(min_rerun_count) = run_log.min_rerun_count else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID run log {} deterministic-rerun evidence requires min_rerun_count.",
                run_log.name
            ),
        );
        return false;
    };
    let Some(max_impedance_delta) = run_log.max_rerun_impedance_delta_ohm else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID run log {} deterministic-rerun evidence requires max_rerun_impedance_delta_ohm.",
                run_log.name
            ),
        );
        return false;
    };
    if run_log.reruns.len() < min_rerun_count {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID run log {} requires at least {} deterministic rerun samples; found {}.",
                run_log.name,
                min_rerun_count,
                run_log.reruns.len()
            ),
        );
        return false;
    }
    let mut rerun_names = BTreeSet::new();
    let mut rerun_ids = BTreeSet::new();
    for rerun in &run_log.reruns {
        if !solver_rerun_has_valid_metadata(rerun)
            || !rerun_names.insert(rerun.name.trim())
            || !rerun_ids.insert(rerun.run_id.trim())
        {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID run log {} deterministic reruns must declare unique non-empty names/run IDs, artifact URI and SHA-256, positive impedance, finite non-negative residual error, and positive iterations.",
                    run_log.name
                ),
            );
            return false;
        }
        if rerun.random_seed.trim() != run_log.random_seed.trim() {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID rerun {} for run log {} must use reviewed random_seed {}.",
                    rerun.name, run_log.name, run_log.random_seed
                ),
            );
            return false;
        }
        if (rerun.solved_impedance_ohm - result.solved_impedance_ohm).abs() > max_impedance_delta {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID rerun {} for result {} exceeds deterministic rerun impedance delta limit.",
                    rerun.name, result.name
                ),
            );
            return false;
        }
        if rerun.residual_error > run_log.max_residual_error {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID rerun {} for run log {} exceeds reviewed residual error limit.",
                    rerun.name, run_log.name
                ),
            );
            return false;
        }
        if rerun.iterations > run_log.max_iterations {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID rerun {} for run log {} exceeds reviewed iteration limit.",
                    rerun.name, run_log.name
                ),
            );
            return false;
        }
    }
    true
}

fn solver_rerun_has_valid_metadata(rerun: &ControlledImpedanceSolverRerun) -> bool {
    !rerun.name.trim().is_empty()
        && !rerun.source.trim().is_empty()
        && !rerun.run_id.trim().is_empty()
        && !rerun.artifact_uri.trim().is_empty()
        && is_sha256_hex(rerun.artifact_sha256.trim())
        && !rerun.random_seed.trim().is_empty()
        && positive(rerun.solved_impedance_ohm)
        && rerun.residual_error.is_finite()
        && rerun.residual_error >= 0.0
        && rerun.iterations > 0
}

fn solver_convergence_policy_has_valid_shape(run_log: &ControlledImpedanceSolverRunLog) -> bool {
    let declared = [
        run_log.min_convergence_sample_count.is_some(),
        run_log.max_convergence_impedance_delta_ohm.is_some(),
        non_empty_option(run_log.required_stopping_criteria.as_deref()).is_some(),
    ];
    declared.iter().all(|declared| !declared)
        || (declared.iter().all(|declared| *declared)
            && run_log.min_convergence_sample_count.unwrap_or(1) > 0
            && run_log
                .max_convergence_impedance_delta_ohm
                .is_none_or(|value| value.is_finite() && value >= 0.0))
}

fn solver_run_log_convergence_samples_are_valid(
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
    result: &ControlledImpedanceSolverResult,
    run_log: &ControlledImpedanceSolverRunLog,
) -> bool {
    let policy_requested = run_log.min_convergence_sample_count.is_some()
        || run_log.max_convergence_impedance_delta_ohm.is_some()
        || non_empty_option(run_log.required_stopping_criteria.as_deref()).is_some();
    if !policy_requested {
        return true;
    }
    let (Some(min_count), Some(max_impedance_delta), Some(stopping_criteria)) = (
        run_log.min_convergence_sample_count,
        run_log.max_convergence_impedance_delta_ohm,
        non_empty_option(run_log.required_stopping_criteria.as_deref()),
    ) else {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID run log {} convergence-window evidence requires min_convergence_sample_count, max_convergence_impedance_delta_ohm, and required_stopping_criteria.",
                run_log.name
            ),
        );
        return false;
    };
    if run_log.convergence_samples.len() < min_count {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID run log {} requires at least {} convergence-window samples; found {}.",
                run_log.name,
                min_count,
                run_log.convergence_samples.len()
            ),
        );
        return false;
    }
    let mut names = BTreeSet::new();
    let mut iterations = BTreeSet::new();
    let mut min_impedance = f64::INFINITY;
    let mut max_impedance = f64::NEG_INFINITY;
    for sample in &run_log.convergence_samples {
        if !solver_convergence_sample_has_valid_metadata(sample)
            || !names.insert(sample.name.trim())
            || !iterations.insert(sample.iteration)
        {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID run log {} convergence samples must declare unique non-empty names/iterations, artifact URI and SHA-256, positive impedance, finite non-negative residual error, positive iteration, and stopping criteria.",
                    run_log.name
                ),
            );
            return false;
        }
        if sample.stopping_criteria.trim() != stopping_criteria {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID convergence sample {} for run log {} must use reviewed stopping criteria {}.",
                    sample.name, run_log.name, stopping_criteria
                ),
            );
            return false;
        }
        if sample.iteration > run_log.max_iterations {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID convergence sample {} for run log {} exceeds reviewed iteration limit.",
                    sample.name, run_log.name
                ),
            );
            return false;
        }
        if sample.residual_error > run_log.max_residual_error {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID convergence sample {} for run log {} exceeds reviewed residual error limit.",
                    sample.name, run_log.name
                ),
            );
            return false;
        }
        if (sample.solved_impedance_ohm - result.solved_impedance_ohm).abs() > max_impedance_delta {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID convergence sample {} for result {} exceeds convergence impedance delta limit.",
                    sample.name, result.name
                ),
            );
            return false;
        }
        min_impedance = min_impedance.min(sample.solved_impedance_ohm);
        max_impedance = max_impedance.max(sample.solved_impedance_ohm);
    }
    if max_impedance - min_impedance > max_impedance_delta {
        validation_input_missing(
            findings,
            scenario,
            format!(
                "CONTROLLED_IMPEDANCE_SOLVER_RESULT_VALID run log {} convergence-window impedance span exceeds reviewed limit.",
                run_log.name
            ),
        );
        return false;
    }
    true
}

fn solver_convergence_sample_has_valid_metadata(
    sample: &ControlledImpedanceSolverConvergenceSample,
) -> bool {
    !sample.name.trim().is_empty()
        && !sample.source.trim().is_empty()
        && !sample.artifact_uri.trim().is_empty()
        && is_sha256_hex(sample.artifact_sha256.trim())
        && sample.iteration > 0
        && positive(sample.solved_impedance_ohm)
        && sample.residual_error.is_finite()
        && sample.residual_error >= 0.0
        && !sample.stopping_criteria.trim().is_empty()
}

fn non_empty_option(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}
