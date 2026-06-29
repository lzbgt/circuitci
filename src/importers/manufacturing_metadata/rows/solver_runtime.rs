use super::{
    AppliedControlledImpedanceSolverRuntimeAllowlist, MetadataCsvRow, optional_raw_column,
    parse_nonempty_list, required_raw_column_for,
};
use anyhow::{Context, Result, bail};
use std::path::Path;

pub(super) fn applied_controlled_impedance_solver_runtime_allowlist(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedControlledImpedanceSolverRuntimeAllowlist> {
    let allowlist_source = optional_raw_column(row, "allowlist_source");
    let source = row
        .source
        .as_deref()
        .or(allowlist_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} controlled_impedance_solver_runtime_allowlist requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedControlledImpedanceSolverRuntimeAllowlist {
        name: required_raw_column_for(
            row,
            path,
            "name",
            "controlled_impedance_solver_runtime_allowlist",
        )?,
        source,
        solver: required_raw_column_for(
            row,
            path,
            "solver",
            "controlled_impedance_solver_runtime_allowlist",
        )?,
        solver_config_lock_revision: required_raw_column_for(
            row,
            path,
            "solver_config_lock_revision",
            "controlled_impedance_solver_runtime_allowlist",
        )?,
        runtime_profile: required_raw_column_for(
            row,
            path,
            "runtime_profile",
            "controlled_impedance_solver_runtime_allowlist",
        )?,
        allowlist_revision: required_raw_column_for(
            row,
            path,
            "allowlist_revision",
            "controlled_impedance_solver_runtime_allowlist",
        )?,
        artifact_uri: required_raw_column_for(
            row,
            path,
            "artifact_uri",
            "controlled_impedance_solver_runtime_allowlist",
        )?,
        artifact_sha256: required_sha256_column(
            row,
            path,
            "artifact_sha256",
            "controlled_impedance_solver_runtime_allowlist",
        )?,
        allowed_options: required_list_column(
            row,
            path,
            "allowed_options",
            "controlled_impedance_solver_runtime_allowlist",
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

fn required_list_column(
    row: &MetadataCsvRow,
    path: &Path,
    column: &str,
    record: &str,
) -> Result<Vec<String>> {
    let raw = required_raw_column_for(row, path, column, record)?;
    parse_nonempty_list(&raw).with_context(|| {
        format!(
            "Manufacturing metadata CSV {} row {} {record} {column} must contain at least one value.",
            path.display(),
            row.row_number
        )
    })
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
