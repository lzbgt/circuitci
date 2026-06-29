use super::{
    AppliedControlledImpedanceSolverConvergenceSample, AppliedControlledImpedanceSolverRerun,
    AppliedControlledImpedanceSolverRunLog, MetadataCsvRow, optional_raw_column,
    required_raw_column_for,
};
use anyhow::{Context, Result, bail};
use std::path::Path;

pub(super) fn applied_controlled_impedance_solver_run_log(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedControlledImpedanceSolverRunLog> {
    let run_log_source = optional_raw_column(row, "run_log_source");
    let source = row
        .source
        .as_deref()
        .or(run_log_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} controlled_impedance_solver_run_log requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedControlledImpedanceSolverRunLog {
        name: required_raw_column_for(row, path, "name", "controlled_impedance_solver_run_log")?,
        source,
        solver: required_raw_column_for(
            row,
            path,
            "solver",
            "controlled_impedance_solver_run_log",
        )?,
        solver_version: required_raw_column_for(
            row,
            path,
            "solver_version",
            "controlled_impedance_solver_run_log",
        )?,
        run_id: required_raw_column_for(
            row,
            path,
            "run_id",
            "controlled_impedance_solver_run_log",
        )?,
        artifact_uri: required_raw_column_for(
            row,
            path,
            "artifact_uri",
            "controlled_impedance_solver_run_log",
        )?,
        artifact_sha256: required_sha256_column(
            row,
            path,
            "artifact_sha256",
            "controlled_impedance_solver_run_log",
        )?,
        random_seed: required_raw_column_for(
            row,
            path,
            "random_seed",
            "controlled_impedance_solver_run_log",
        )?,
        numeric_tolerance_policy: required_raw_column_for(
            row,
            path,
            "numeric_tolerance_policy",
            "controlled_impedance_solver_run_log",
        )?,
        max_residual_error: required_nonnegative_number(
            row,
            path,
            "max_residual_error",
            "controlled_impedance_solver_run_log",
        )?,
        max_iterations: required_positive_usize(
            row,
            path,
            "max_iterations",
            "controlled_impedance_solver_run_log",
        )?,
        min_rerun_count: optional_raw_column(row, "min_rerun_count")
            .map(|value| {
                parse_positive_usize(
                    &value,
                    path,
                    row,
                    "min_rerun_count",
                    "controlled_impedance_solver_run_log",
                )
            })
            .transpose()?,
        max_rerun_impedance_delta_ohm: optional_raw_column(row, "max_rerun_impedance_delta_ohm")
            .map(|value| {
                parse_nonnegative_number(
                    &value,
                    path,
                    row,
                    "max_rerun_impedance_delta_ohm",
                    "controlled_impedance_solver_run_log",
                )
            })
            .transpose()?,
        min_convergence_sample_count: optional_raw_column(row, "min_convergence_sample_count")
            .map(|value| {
                parse_positive_usize(
                    &value,
                    path,
                    row,
                    "min_convergence_sample_count",
                    "controlled_impedance_solver_run_log",
                )
            })
            .transpose()?,
        max_convergence_impedance_delta_ohm: optional_raw_column(
            row,
            "max_convergence_impedance_delta_ohm",
        )
        .map(|value| {
            parse_nonnegative_number(
                &value,
                path,
                row,
                "max_convergence_impedance_delta_ohm",
                "controlled_impedance_solver_run_log",
            )
        })
        .transpose()?,
        required_stopping_criteria: optional_raw_column(row, "required_stopping_criteria"),
    })
}

pub(super) fn applied_controlled_impedance_solver_rerun(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedControlledImpedanceSolverRerun> {
    let rerun_source = optional_raw_column(row, "rerun_source");
    let source = row
        .source
        .as_deref()
        .or(rerun_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} controlled_impedance_solver_rerun requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedControlledImpedanceSolverRerun {
        solver_run_log_name: optional_raw_column(row, "solver_run_log")
            .or_else(|| optional_raw_column(row, "run_log"))
            .with_context(|| {
                format!(
                    "Manufacturing metadata CSV {} row {} controlled_impedance_solver_rerun requires solver_run_log.",
                    path.display(),
                    row.row_number
                )
            })?,
        name: required_raw_column_for(row, path, "name", "controlled_impedance_solver_rerun")?,
        source,
        run_id: required_raw_column_for(row, path, "run_id", "controlled_impedance_solver_rerun")?,
        artifact_uri: required_raw_column_for(
            row,
            path,
            "artifact_uri",
            "controlled_impedance_solver_rerun",
        )?,
        artifact_sha256: required_sha256_column(
            row,
            path,
            "artifact_sha256",
            "controlled_impedance_solver_rerun",
        )?,
        random_seed: required_raw_column_for(
            row,
            path,
            "random_seed",
            "controlled_impedance_solver_rerun",
        )?,
        solved_impedance_ohm: required_positive_number(
            row,
            path,
            "solved_impedance_ohm",
            "controlled_impedance_solver_rerun",
        )?,
        residual_error: required_nonnegative_number(
            row,
            path,
            "residual_error",
            "controlled_impedance_solver_rerun",
        )?,
        iterations: required_positive_usize(
            row,
            path,
            "iterations",
            "controlled_impedance_solver_rerun",
        )?,
    })
}

pub(super) fn applied_controlled_impedance_solver_convergence_sample(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedControlledImpedanceSolverConvergenceSample> {
    let sample_source = optional_raw_column(row, "sample_source");
    let source = row
        .source
        .as_deref()
        .or(sample_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} controlled_impedance_solver_convergence_sample requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedControlledImpedanceSolverConvergenceSample {
        solver_run_log_name: optional_raw_column(row, "solver_run_log")
            .or_else(|| optional_raw_column(row, "run_log"))
            .with_context(|| {
                format!(
                    "Manufacturing metadata CSV {} row {} controlled_impedance_solver_convergence_sample requires solver_run_log.",
                    path.display(),
                    row.row_number
                )
            })?,
        name: required_raw_column_for(
            row,
            path,
            "name",
            "controlled_impedance_solver_convergence_sample",
        )?,
        source,
        artifact_uri: required_raw_column_for(
            row,
            path,
            "artifact_uri",
            "controlled_impedance_solver_convergence_sample",
        )?,
        artifact_sha256: required_sha256_column(
            row,
            path,
            "artifact_sha256",
            "controlled_impedance_solver_convergence_sample",
        )?,
        iteration: required_positive_usize(
            row,
            path,
            "iteration",
            "controlled_impedance_solver_convergence_sample",
        )?,
        solved_impedance_ohm: required_positive_number(
            row,
            path,
            "solved_impedance_ohm",
            "controlled_impedance_solver_convergence_sample",
        )?,
        residual_error: required_nonnegative_number(
            row,
            path,
            "residual_error",
            "controlled_impedance_solver_convergence_sample",
        )?,
        stopping_criteria: required_raw_column_for(
            row,
            path,
            "stopping_criteria",
            "controlled_impedance_solver_convergence_sample",
        )?,
    })
}

fn required_sha256_column(
    row: &MetadataCsvRow,
    path: &Path,
    column: &str,
    record: &str,
) -> Result<String> {
    let digest = required_raw_column_for(row, path, column, record)?;
    if !is_sha256_hex(digest.trim()) {
        bail!(
            "Manufacturing metadata CSV {} row {} {record} {column} must be a 64-character SHA-256 hex digest.",
            path.display(),
            row.row_number
        );
    }
    Ok(digest)
}

fn required_nonnegative_number(
    row: &MetadataCsvRow,
    path: &Path,
    column: &str,
    record: &str,
) -> Result<f64> {
    let raw = required_raw_column_for(row, path, column, record)?;
    let value = raw.parse::<f64>().with_context(|| {
        format!(
            "Manufacturing metadata CSV {} row {} {record} {column} must be a finite non-negative number.",
            path.display(),
            row.row_number
        )
    })?;
    if !value.is_finite() || value < 0.0 {
        bail!(
            "Manufacturing metadata CSV {} row {} {record} {column} must be finite and non-negative.",
            path.display(),
            row.row_number
        );
    }
    Ok(value)
}

fn parse_nonnegative_number(
    raw: &str,
    path: &Path,
    row: &MetadataCsvRow,
    column: &str,
    record: &str,
) -> Result<f64> {
    let value = raw.parse::<f64>().with_context(|| {
        format!(
            "Manufacturing metadata CSV {} row {} {record} {column} must be a finite non-negative number.",
            path.display(),
            row.row_number
        )
    })?;
    if !value.is_finite() || value < 0.0 {
        bail!(
            "Manufacturing metadata CSV {} row {} {record} {column} must be finite and non-negative.",
            path.display(),
            row.row_number
        );
    }
    Ok(value)
}

fn required_positive_number(
    row: &MetadataCsvRow,
    path: &Path,
    column: &str,
    record: &str,
) -> Result<f64> {
    let raw = required_raw_column_for(row, path, column, record)?;
    let value = raw.parse::<f64>().with_context(|| {
        format!(
            "Manufacturing metadata CSV {} row {} {record} {column} must be a finite positive number.",
            path.display(),
            row.row_number
        )
    })?;
    if !value.is_finite() || value <= 0.0 {
        bail!(
            "Manufacturing metadata CSV {} row {} {record} {column} must be finite and positive.",
            path.display(),
            row.row_number
        );
    }
    Ok(value)
}

fn required_positive_usize(
    row: &MetadataCsvRow,
    path: &Path,
    column: &str,
    record: &str,
) -> Result<usize> {
    let raw = required_raw_column_for(row, path, column, record)?;
    let value = raw.parse::<usize>().with_context(|| {
        format!(
            "Manufacturing metadata CSV {} row {} {record} {column} must be a positive integer.",
            path.display(),
            row.row_number
        )
    })?;
    if value == 0 {
        bail!(
            "Manufacturing metadata CSV {} row {} {record} {column} must be positive.",
            path.display(),
            row.row_number
        );
    }
    Ok(value)
}

fn parse_positive_usize(
    raw: &str,
    path: &Path,
    row: &MetadataCsvRow,
    column: &str,
    record: &str,
) -> Result<usize> {
    let value = raw.parse::<usize>().with_context(|| {
        format!(
            "Manufacturing metadata CSV {} row {} {record} {column} must be a positive integer.",
            path.display(),
            row.row_number
        )
    })?;
    if value == 0 {
        bail!(
            "Manufacturing metadata CSV {} row {} {record} {column} must be positive.",
            path.display(),
            row.row_number
        );
    }
    Ok(value)
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
