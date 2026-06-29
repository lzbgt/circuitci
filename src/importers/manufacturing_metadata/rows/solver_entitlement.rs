use super::{
    AppliedControlledImpedanceSolverEntitlement, MetadataCsvRow, optional_raw_column,
    parse_nonempty_list, required_raw_column_for,
};
use anyhow::{Context, Result, bail};
use std::path::Path;

pub(super) fn applied_controlled_impedance_solver_entitlement(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedControlledImpedanceSolverEntitlement> {
    let entitlement_source = optional_raw_column(row, "entitlement_source");
    let source = row
        .source
        .as_deref()
        .or(entitlement_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} controlled_impedance_solver_entitlement requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedControlledImpedanceSolverEntitlement {
        name: required_raw_column_for(
            row,
            path,
            "name",
            "controlled_impedance_solver_entitlement",
        )?,
        source,
        solver: required_raw_column_for(
            row,
            path,
            "solver",
            "controlled_impedance_solver_entitlement",
        )?,
        solver_version: required_raw_column_for(
            row,
            path,
            "solver_version",
            "controlled_impedance_solver_entitlement",
        )?,
        entitlement_id: required_raw_column_for(
            row,
            path,
            "entitlement_id",
            "controlled_impedance_solver_entitlement",
        )?,
        entitlement_revision: required_raw_column_for(
            row,
            path,
            "entitlement_revision",
            "controlled_impedance_solver_entitlement",
        )?,
        artifact_uri: required_raw_column_for(
            row,
            path,
            "artifact_uri",
            "controlled_impedance_solver_entitlement",
        )?,
        artifact_sha256: required_sha256_column(
            row,
            path,
            "artifact_sha256",
            "controlled_impedance_solver_entitlement",
        )?,
        licensed_features: required_list_column(
            row,
            path,
            "licensed_features",
            "controlled_impedance_solver_entitlement",
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
