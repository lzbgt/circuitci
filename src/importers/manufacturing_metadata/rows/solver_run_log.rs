use super::{
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

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
