use super::{
    AppliedControlledImpedanceCoupon, AppliedControlledImpedanceCouponSample,
    AppliedControlledImpedanceNet, AppliedControlledImpedancePair,
    AppliedControlledImpedanceSolverMaterialAcceptance,
    AppliedControlledImpedanceSolverMaterialCorner,
    AppliedControlledImpedanceSolverMaterialLibrary,
    AppliedControlledImpedanceSolverMaterialProcess, AppliedControlledImpedanceSolverQualification,
    AppliedControlledImpedanceSolverResult, AppliedControlledImpedanceSolverSample,
    AppliedStackupLayer, AppliedThermalCopper, AppliedThermalEnvironment, AppliedThermalLimit,
    AppliedThermalMeasurement, AppliedThermalPackage, MetadataCsvRow, normalize_name,
    optional_raw_column, parse_nonempty_list, parse_nonnegative_number,
    parse_nonnegative_temperature_delta_c, parse_positive_area_mm2, parse_positive_c_per_w,
    parse_positive_number, parse_positive_ohms, parse_positive_usize, parse_positive_watts,
    parse_temperature_c, required_nonnegative_number, required_positive_number,
    required_positive_watts, required_raw_column_for,
};
use anyhow::{Context, Result, bail};
use std::path::Path;

#[path = "rf.rs"]
mod rf;

pub(super) use rf::{
    applied_rf_antenna_feed_path, applied_rf_antenna_keepout, applied_rf_antenna_matching_network,
    applied_rf_antenna_measurement, applied_rf_antenna_measurement_condition,
    applied_rf_antenna_performance_limit,
};

pub(super) fn applied_controlled_impedance_net(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedControlledImpedanceNet> {
    let target_source = optional_raw_column(row, "target_source");
    let source = row
        .source
        .as_deref()
        .or(target_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} controlled_impedance_net requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedControlledImpedanceNet {
        net: required_raw_column_for(row, path, "net", "controlled_impedance_net")?,
        source,
        target_impedance_ohm: parse_positive_ohms(
            row.value.trim(),
            row.unit.as_deref(),
            path,
            row,
            "value",
        )?,
        expected_width_mm: required_positive_number(
            row,
            path,
            "expected_width_mm",
            "controlled_impedance_net",
        )?,
        max_width_error_mm: required_nonnegative_number(
            row,
            path,
            "max_width_error_mm",
            "controlled_impedance_net",
        )?,
        solder_mask_state: optional_solder_mask_state(row, path, "controlled_impedance_net")?,
        solder_mask_layer: optional_raw_column(row, "solder_mask_layer"),
        solder_mask_source: optional_raw_column(row, "solder_mask_source"),
    })
}

pub(super) fn applied_controlled_impedance_pair(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedControlledImpedancePair> {
    let first_net = required_raw_column_for(row, path, "first_net", "controlled_impedance_pair")?;
    let second_net = required_raw_column_for(row, path, "second_net", "controlled_impedance_pair")?;
    if first_net == second_net {
        bail!(
            "Manufacturing metadata CSV {} row {} controlled_impedance_pair requires two distinct nets.",
            path.display(),
            row.row_number
        );
    }
    let target_source = optional_raw_column(row, "target_source");
    let source = row
        .source
        .as_deref()
        .or(target_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} controlled_impedance_pair requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedControlledImpedancePair {
        first_net,
        second_net,
        source,
        target_differential_impedance_ohm: parse_positive_ohms(
            row.value.trim(),
            row.unit.as_deref(),
            path,
            row,
            "value",
        )?,
        expected_width_mm: required_positive_number(
            row,
            path,
            "expected_width_mm",
            "controlled_impedance_pair",
        )?,
        expected_gap_mm: required_positive_number(
            row,
            path,
            "expected_gap_mm",
            "controlled_impedance_pair",
        )?,
        max_width_error_mm: required_nonnegative_number(
            row,
            path,
            "max_width_error_mm",
            "controlled_impedance_pair",
        )?,
        max_gap_error_mm: required_nonnegative_number(
            row,
            path,
            "max_gap_error_mm",
            "controlled_impedance_pair",
        )?,
        solder_mask_state: optional_solder_mask_state(row, path, "controlled_impedance_pair")?,
        solder_mask_layer: optional_raw_column(row, "solder_mask_layer"),
        solder_mask_source: optional_raw_column(row, "solder_mask_source"),
    })
}

fn optional_solder_mask_state(
    row: &MetadataCsvRow,
    path: &Path,
    field_name: &str,
) -> Result<Option<String>> {
    let Some(state) = optional_raw_column(row, "solder_mask_state") else {
        return Ok(None);
    };
    match normalize_name(&state).as_str() {
        "covered" | "masked" | "soldermaskcovered" | "maskcovered" => {
            Ok(Some("covered".to_string()))
        }
        "opened" | "open" | "exposed" | "soldermaskopened" | "maskopened" => {
            Ok(Some("opened".to_string()))
        }
        _ => bail!(
            "Manufacturing metadata CSV {} row {} {} solder_mask_state must be covered or opened.",
            path.display(),
            row.row_number,
            field_name
        ),
    }
}

pub(super) fn applied_controlled_impedance_coupon(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedControlledImpedanceCoupon> {
    let name = required_raw_column_for(row, path, "name", "controlled_impedance_coupon")?;
    let coupon_type = required_coupon_type(row, path)?;
    let net = optional_raw_column(row, "net");
    let first_net = optional_raw_column(row, "first_net");
    let second_net = optional_raw_column(row, "second_net");
    match coupon_type.as_str() {
        "single_ended" => {
            if net.is_none() || first_net.is_some() || second_net.is_some() {
                bail!(
                    "Manufacturing metadata CSV {} row {} controlled_impedance_coupon single_ended requires net and no first_net/second_net.",
                    path.display(),
                    row.row_number
                );
            }
        }
        "differential" => {
            if net.is_some() || first_net.is_none() || second_net.is_none() {
                bail!(
                    "Manufacturing metadata CSV {} row {} controlled_impedance_coupon differential requires first_net and second_net and no net.",
                    path.display(),
                    row.row_number
                );
            }
            if first_net == second_net {
                bail!(
                    "Manufacturing metadata CSV {} row {} controlled_impedance_coupon requires two distinct differential nets.",
                    path.display(),
                    row.row_number
                );
            }
        }
        _ => unreachable!("coupon type is normalized"),
    }
    let coupon_source = optional_raw_column(row, "coupon_source");
    let source = row
        .source
        .as_deref()
        .or(coupon_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} controlled_impedance_coupon requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedControlledImpedanceCoupon {
        name,
        source,
        coupon_type,
        net,
        first_net,
        second_net,
        target_impedance_ohm: required_positive_number(
            row,
            path,
            "target_impedance_ohm",
            "controlled_impedance_coupon",
        )?,
        measured_impedance_ohm: parse_positive_ohms(
            row.value.trim(),
            row.unit.as_deref(),
            path,
            row,
            "value",
        )?,
        max_impedance_error_ohm: required_nonnegative_number(
            row,
            path,
            "max_impedance_error_ohm",
            "controlled_impedance_coupon",
        )?,
        process_lot: optional_raw_column(row, "process_lot"),
        panel_id: optional_raw_column(row, "panel_id"),
        stackup_revision: optional_raw_column(row, "stackup_revision"),
        coupon_trace_layer: optional_raw_column(row, "coupon_trace_layer"),
        coupon_trace_width_mm: optional_raw_column(row, "coupon_trace_width_mm")
            .map(|value| parse_positive_number(&value, path, row, "coupon_trace_width_mm"))
            .transpose()?,
        max_trace_width_delta_mm: optional_raw_column(row, "max_trace_width_delta_mm")
            .map(|value| parse_nonnegative_number(&value, path, row, "max_trace_width_delta_mm"))
            .transpose()?,
        coupon_trace_gap_mm: optional_raw_column(row, "coupon_trace_gap_mm")
            .map(|value| parse_positive_number(&value, path, row, "coupon_trace_gap_mm"))
            .transpose()?,
        max_trace_gap_delta_mm: optional_raw_column(row, "max_trace_gap_delta_mm")
            .map(|value| parse_nonnegative_number(&value, path, row, "max_trace_gap_delta_mm"))
            .transpose()?,
        min_batch_sample_count: optional_raw_column(row, "min_batch_sample_count")
            .map(|value| parse_positive_usize(&value, path, row, "min_batch_sample_count"))
            .transpose()?,
        max_batch_mean_impedance_error_ohm: optional_raw_column(
            row,
            "max_batch_mean_impedance_error_ohm",
        )
        .map(|value| {
            parse_nonnegative_number(&value, path, row, "max_batch_mean_impedance_error_ohm")
        })
        .transpose()?,
        max_batch_sample_impedance_error_ohm: optional_raw_column(
            row,
            "max_batch_sample_impedance_error_ohm",
        )
        .map(|value| {
            parse_nonnegative_number(&value, path, row, "max_batch_sample_impedance_error_ohm")
        })
        .transpose()?,
        max_batch_stddev_ohm: optional_raw_column(row, "max_batch_stddev_ohm")
            .map(|value| parse_nonnegative_number(&value, path, row, "max_batch_stddev_ohm"))
            .transpose()?,
    })
}

pub(super) fn applied_controlled_impedance_coupon_sample(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedControlledImpedanceCouponSample> {
    let sample_source = optional_raw_column(row, "sample_source");
    let source = row
        .source
        .as_deref()
        .or(sample_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} controlled_impedance_coupon_sample requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedControlledImpedanceCouponSample {
        coupon_name: required_raw_column_for(
            row,
            path,
            "coupon_name",
            "controlled_impedance_coupon_sample",
        )?,
        name: required_raw_column_for(row, path, "name", "controlled_impedance_coupon_sample")?,
        source,
        measured_impedance_ohm: parse_positive_ohms(
            row.value.trim(),
            row.unit.as_deref(),
            path,
            row,
            "value",
        )?,
    })
}

pub(super) fn applied_controlled_impedance_solver_result(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedControlledImpedanceSolverResult> {
    let name = required_raw_column_for(row, path, "name", "controlled_impedance_solver_result")?;
    let result_type = required_solver_result_type(row, path)?;
    let net = optional_raw_column(row, "net");
    let first_net = optional_raw_column(row, "first_net");
    let second_net = optional_raw_column(row, "second_net");
    match result_type.as_str() {
        "single_ended" => {
            if net.is_none() || first_net.is_some() || second_net.is_some() {
                bail!(
                    "Manufacturing metadata CSV {} row {} controlled_impedance_solver_result single_ended requires net and no first_net/second_net.",
                    path.display(),
                    row.row_number
                );
            }
        }
        "differential" => {
            if net.is_some() || first_net.is_none() || second_net.is_none() {
                bail!(
                    "Manufacturing metadata CSV {} row {} controlled_impedance_solver_result differential requires first_net and second_net and no net.",
                    path.display(),
                    row.row_number
                );
            }
            if first_net == second_net {
                bail!(
                    "Manufacturing metadata CSV {} row {} controlled_impedance_solver_result requires two distinct differential nets.",
                    path.display(),
                    row.row_number
                );
            }
        }
        _ => unreachable!("solver result type is normalized"),
    }
    let result_source = optional_raw_column(row, "solver_source");
    let source = row
        .source
        .as_deref()
        .or(result_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} controlled_impedance_solver_result requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedControlledImpedanceSolverResult {
        name,
        source,
        solver: required_raw_column_for(row, path, "solver", "controlled_impedance_solver_result")?,
        solver_version: optional_raw_column(row, "solver_version"),
        solver_artifact_uri: required_raw_column_for(
            row,
            path,
            "solver_artifact_uri",
            "controlled_impedance_solver_result",
        )?,
        solver_artifact_sha256: required_solver_artifact_sha256(row, path)?,
        solver_artifact_signature_uri: optional_raw_column(row, "solver_artifact_signature_uri"),
        solver_artifact_signature_sha256: optional_solver_artifact_signature_sha256(row, path)?,
        solver_artifact_signer: optional_raw_column(row, "solver_artifact_signer"),
        solver_output_schema: optional_raw_column(row, "solver_output_schema"),
        solver_output_schema_version: optional_raw_column(row, "solver_output_schema_version"),
        solver_output_schema_uri: optional_raw_column(row, "solver_output_schema_uri"),
        solver_output_schema_sha256: optional_solver_output_schema_sha256(row, path)?,
        solver_config_lock_uri: optional_raw_column(row, "solver_config_lock_uri"),
        solver_config_lock_sha256: optional_solver_config_lock_sha256(row, path)?,
        solver_config_lock_tool: optional_raw_column(row, "solver_config_lock_tool"),
        solver_config_lock_revision: optional_raw_column(row, "solver_config_lock_revision"),
        solver_runtime_allowlist: optional_raw_column(row, "solver_runtime_allowlist"),
        solver_runtime_profile: optional_raw_column(row, "solver_runtime_profile"),
        solver_runtime_options: optional_raw_column(row, "solver_runtime_options")
            .map(|value| parse_nonempty_list(&value))
            .transpose()?
            .unwrap_or_default(),
        solver_entitlement: optional_raw_column(row, "solver_entitlement"),
        solver_entitlement_features: optional_raw_column(row, "solver_entitlement_features")
            .map(|value| parse_nonempty_list(&value))
            .transpose()?
            .unwrap_or_default(),
        solver_execution_environment: optional_raw_column(row, "solver_execution_environment"),
        solver_environment_fingerprint: optional_raw_column(row, "solver_environment_fingerprint"),
        solver_environment_components: optional_raw_column(row, "solver_environment_components")
            .map(|value| parse_nonempty_list(&value))
            .transpose()?
            .unwrap_or_default(),
        solver_run_log: optional_raw_column(row, "solver_run_log"),
        solver_run_id: optional_raw_column(row, "solver_run_id"),
        solver_random_seed: optional_raw_column(row, "solver_random_seed"),
        solver_numeric_tolerance_policy: optional_raw_column(
            row,
            "solver_numeric_tolerance_policy",
        ),
        solver_residual_error: optional_raw_column(row, "solver_residual_error")
            .map(|value| parse_nonnegative_number(&value, path, row, "solver_residual_error"))
            .transpose()?,
        solver_iterations: optional_raw_column(row, "solver_iterations")
            .map(|value| parse_positive_usize(&value, path, row, "solver_iterations"))
            .transpose()?,
        solver_input_deck_uri: optional_raw_column(row, "solver_input_deck_uri"),
        solver_input_deck_sha256: optional_solver_input_deck_sha256(row, path)?,
        result_type,
        net,
        first_net,
        second_net,
        target_impedance_ohm: required_positive_number(
            row,
            path,
            "target_impedance_ohm",
            "controlled_impedance_solver_result",
        )?,
        solved_impedance_ohm: parse_positive_ohms(
            row.value.trim(),
            row.unit.as_deref(),
            path,
            row,
            "value",
        )?,
        max_impedance_error_ohm: required_nonnegative_number(
            row,
            path,
            "max_impedance_error_ohm",
            "controlled_impedance_solver_result",
        )?,
        stackup_revision: required_raw_column_for(
            row,
            path,
            "stackup_revision",
            "controlled_impedance_solver_result",
        )?,
        route_layer: required_raw_column_for(
            row,
            path,
            "route_layer",
            "controlled_impedance_solver_result",
        )?,
        reference_layer: required_raw_column_for(
            row,
            path,
            "reference_layer",
            "controlled_impedance_solver_result",
        )?,
        dielectric_layer: required_raw_column_for(
            row,
            path,
            "dielectric_layer",
            "controlled_impedance_solver_result",
        )?,
        solved_width_mm: required_positive_number(
            row,
            path,
            "solved_width_mm",
            "controlled_impedance_solver_result",
        )?,
        max_route_width_delta_mm: required_nonnegative_number(
            row,
            path,
            "max_route_width_delta_mm",
            "controlled_impedance_solver_result",
        )?,
        input_stackup_revision: optional_raw_column(row, "input_stackup_revision"),
        input_route_layer: optional_raw_column(row, "input_route_layer"),
        input_reference_layer: optional_raw_column(row, "input_reference_layer"),
        input_dielectric_layer: optional_raw_column(row, "input_dielectric_layer"),
        input_width_mm: optional_raw_column(row, "input_width_mm")
            .map(|value| parse_positive_number(&value, path, row, "input_width_mm"))
            .transpose()?,
        solved_gap_mm: optional_raw_column(row, "solved_gap_mm")
            .map(|value| parse_positive_number(&value, path, row, "solved_gap_mm"))
            .transpose()?,
        max_route_gap_delta_mm: optional_raw_column(row, "max_route_gap_delta_mm")
            .map(|value| parse_nonnegative_number(&value, path, row, "max_route_gap_delta_mm"))
            .transpose()?,
        input_gap_mm: optional_raw_column(row, "input_gap_mm")
            .map(|value| parse_positive_number(&value, path, row, "input_gap_mm"))
            .transpose()?,
        frequency_mhz: optional_raw_column(row, "frequency_mhz")
            .map(|value| parse_positive_number(&value, path, row, "frequency_mhz"))
            .transpose()?,
        input_frequency_mhz: optional_raw_column(row, "input_frequency_mhz")
            .map(|value| parse_positive_number(&value, path, row, "input_frequency_mhz"))
            .transpose()?,
        copper_roughness_model: optional_raw_column(row, "copper_roughness_model"),
        copper_roughness_um: optional_raw_column(row, "copper_roughness_um")
            .map(|value| parse_positive_number(&value, path, row, "copper_roughness_um"))
            .transpose()?,
        input_copper_roughness_model: optional_raw_column(row, "input_copper_roughness_model"),
        input_copper_roughness_um: optional_raw_column(row, "input_copper_roughness_um")
            .map(|value| parse_positive_number(&value, path, row, "input_copper_roughness_um"))
            .transpose()?,
        etch_compensation_model: optional_raw_column(row, "etch_compensation_model"),
        etch_compensation_um: optional_raw_column(row, "etch_compensation_um")
            .map(|value| parse_positive_number(&value, path, row, "etch_compensation_um"))
            .transpose()?,
        input_etch_compensation_model: optional_raw_column(row, "input_etch_compensation_model"),
        input_etch_compensation_um: optional_raw_column(row, "input_etch_compensation_um")
            .map(|value| parse_positive_number(&value, path, row, "input_etch_compensation_um"))
            .transpose()?,
        solver_material_library: optional_raw_column(row, "solver_material_library"),
        solver_material_library_revision: optional_raw_column(
            row,
            "solver_material_library_revision",
        ),
        solver_material_library_artifact_uri: optional_raw_column(
            row,
            "solver_material_library_artifact_uri",
        ),
        solver_material_library_artifact_sha256: optional_solver_material_library_artifact_sha256(
            row, path,
        )?,
        input_material_library: optional_raw_column(row, "input_material_library"),
        input_material_library_revision: optional_raw_column(
            row,
            "input_material_library_revision",
        ),
        stackup_signoff_source: optional_raw_column(row, "stackup_signoff_source"),
        fabricator_stackup_revision: optional_raw_column(row, "fabricator_stackup_revision"),
        stackup_signoff_artifact_uri: optional_raw_column(row, "stackup_signoff_artifact_uri"),
        stackup_signoff_artifact_sha256: optional_stackup_signoff_artifact_sha256(row, path)?,
        min_solver_sample_count: optional_raw_column(row, "min_solver_sample_count")
            .as_deref()
            .map(|value| parse_positive_usize(value, path, row, "min_solver_sample_count"))
            .transpose()?,
        max_solver_frequency_step_mhz: optional_raw_column(row, "max_solver_frequency_step_mhz")
            .as_deref()
            .map(|value| parse_positive_number(value, path, row, "max_solver_frequency_step_mhz"))
            .transpose()?,
        required_solver_corners: optional_raw_column(row, "required_solver_corners")
            .map(|value| parse_nonempty_list(&value))
            .transpose()?
            .unwrap_or_default(),
    })
}

pub(super) fn applied_controlled_impedance_solver_sample(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedControlledImpedanceSolverSample> {
    let sample_source = optional_raw_column(row, "sample_source");
    let source = row
        .source
        .as_deref()
        .or(sample_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} controlled_impedance_solver_sample requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedControlledImpedanceSolverSample {
        solver_result_name: required_raw_column_for(
            row,
            path,
            "solver_result_name",
            "controlled_impedance_solver_sample",
        )?,
        name: required_raw_column_for(row, path, "name", "controlled_impedance_solver_sample")?,
        source,
        corner: required_raw_column_for(row, path, "corner", "controlled_impedance_solver_sample")?,
        frequency_mhz: required_positive_number(
            row,
            path,
            "frequency_mhz",
            "controlled_impedance_solver_sample",
        )?,
        solved_impedance_ohm: parse_positive_ohms(
            row.value.trim(),
            row.unit.as_deref(),
            path,
            row,
            "value",
        )?,
    })
}

pub(super) fn applied_controlled_impedance_solver_material_corner(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedControlledImpedanceSolverMaterialCorner> {
    let corner_source = optional_raw_column(row, "corner_source");
    let source = row
        .source
        .as_deref()
        .or(corner_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} controlled_impedance_solver_material_corner requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedControlledImpedanceSolverMaterialCorner {
        solver_result_name: required_raw_column_for(
            row,
            path,
            "solver_result_name",
            "controlled_impedance_solver_material_corner",
        )?,
        name: required_raw_column_for(
            row,
            path,
            "name",
            "controlled_impedance_solver_material_corner",
        )?,
        source,
        corner: required_raw_column_for(
            row,
            path,
            "corner",
            "controlled_impedance_solver_material_corner",
        )?,
        dielectric_layer: required_raw_column_for(
            row,
            path,
            "dielectric_layer",
            "controlled_impedance_solver_material_corner",
        )?,
        material: required_raw_column_for(
            row,
            path,
            "material",
            "controlled_impedance_solver_material_corner",
        )?,
        dielectric_constant: parse_positive_number(row.value.trim(), path, row, "value")?,
        nominal_dielectric_constant: required_positive_number(
            row,
            path,
            "nominal_dielectric_constant",
            "controlled_impedance_solver_material_corner",
        )?,
        material_library: required_raw_column_for(
            row,
            path,
            "material_library",
            "controlled_impedance_solver_material_corner",
        )?,
        material_library_revision: required_raw_column_for(
            row,
            path,
            "material_library_revision",
            "controlled_impedance_solver_material_corner",
        )?,
    })
}

pub(super) fn applied_controlled_impedance_solver_qualification(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedControlledImpedanceSolverQualification> {
    let qualification_source = optional_raw_column(row, "qualification_source");
    let source = row
        .source
        .as_deref()
        .or(qualification_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} controlled_impedance_solver_qualification requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedControlledImpedanceSolverQualification {
        name: required_raw_column_for(
            row,
            path,
            "name",
            "controlled_impedance_solver_qualification",
        )?,
        source,
        solver: required_raw_column_for(
            row,
            path,
            "solver",
            "controlled_impedance_solver_qualification",
        )?,
        solver_version: required_raw_column_for(
            row,
            path,
            "solver_version",
            "controlled_impedance_solver_qualification",
        )?,
        qualification_artifact_uri: required_raw_column_for(
            row,
            path,
            "qualification_artifact_uri",
            "controlled_impedance_solver_qualification",
        )?,
        qualification_artifact_sha256: required_solver_qualification_sha256(row, path)?,
    })
}

pub(super) fn applied_controlled_impedance_solver_material_library(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedControlledImpedanceSolverMaterialLibrary> {
    let library_source = optional_raw_column(row, "library_source");
    let source = row
        .source
        .as_deref()
        .or(library_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} controlled_impedance_solver_material_library requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedControlledImpedanceSolverMaterialLibrary {
        name: required_raw_column_for(
            row,
            path,
            "name",
            "controlled_impedance_solver_material_library",
        )?,
        source,
        material_library: required_raw_column_for(
            row,
            path,
            "material_library",
            "controlled_impedance_solver_material_library",
        )?,
        material_library_revision: required_raw_column_for(
            row,
            path,
            "material_library_revision",
            "controlled_impedance_solver_material_library",
        )?,
        artifact_uri: required_raw_column_for(
            row,
            path,
            "artifact_uri",
            "controlled_impedance_solver_material_library",
        )?,
        artifact_sha256: required_sha256_column(
            row,
            path,
            "artifact_sha256",
            "controlled_impedance_solver_material_library",
        )?,
        corners: required_list_column(
            row,
            path,
            "corners",
            "controlled_impedance_solver_material_library",
        )?,
        dielectric_layers: required_list_column(
            row,
            path,
            "dielectric_layers",
            "controlled_impedance_solver_material_library",
        )?,
        materials: required_list_column(
            row,
            path,
            "materials",
            "controlled_impedance_solver_material_library",
        )?,
        content_fields: required_list_column(
            row,
            path,
            "content_fields",
            "controlled_impedance_solver_material_library",
        )?,
    })
}

pub(super) fn applied_controlled_impedance_solver_material_acceptance(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedControlledImpedanceSolverMaterialAcceptance> {
    let acceptance_source = optional_raw_column(row, "acceptance_source");
    let source = row
        .source
        .as_deref()
        .or(acceptance_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} controlled_impedance_solver_material_acceptance requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedControlledImpedanceSolverMaterialAcceptance {
        name: required_raw_column_for(
            row,
            path,
            "name",
            "controlled_impedance_solver_material_acceptance",
        )?,
        source,
        material_library: required_raw_column_for(
            row,
            path,
            "material_library",
            "controlled_impedance_solver_material_acceptance",
        )?,
        material_library_revision: required_raw_column_for(
            row,
            path,
            "material_library_revision",
            "controlled_impedance_solver_material_acceptance",
        )?,
        fabricator_stackup_revision: required_raw_column_for(
            row,
            path,
            "fabricator_stackup_revision",
            "controlled_impedance_solver_material_acceptance",
        )?,
        acceptance_artifact_uri: required_raw_column_for(
            row,
            path,
            "acceptance_artifact_uri",
            "controlled_impedance_solver_material_acceptance",
        )?,
        acceptance_artifact_sha256: required_sha256_column(
            row,
            path,
            "acceptance_artifact_sha256",
            "controlled_impedance_solver_material_acceptance",
        )?,
        accepted_by: optional_raw_column(row, "accepted_by"),
        accepted_corners: required_list_column(
            row,
            path,
            "accepted_corners",
            "controlled_impedance_solver_material_acceptance",
        )?,
        accepted_dielectric_layers: required_list_column(
            row,
            path,
            "accepted_dielectric_layers",
            "controlled_impedance_solver_material_acceptance",
        )?,
        accepted_materials: required_list_column(
            row,
            path,
            "accepted_materials",
            "controlled_impedance_solver_material_acceptance",
        )?,
    })
}

pub(super) fn applied_controlled_impedance_solver_material_process(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedControlledImpedanceSolverMaterialProcess> {
    let process_source = optional_raw_column(row, "process_source");
    let source = row
        .source
        .as_deref()
        .into_iter()
        .chain(process_source.as_deref())
        .map(str::trim)
        .find(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} controlled_impedance_solver_material_process requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedControlledImpedanceSolverMaterialProcess {
        name: required_raw_column_for(
            row,
            path,
            "name",
            "controlled_impedance_solver_material_process",
        )?,
        source,
        material_library: required_raw_column_for(
            row,
            path,
            "material_library",
            "controlled_impedance_solver_material_process",
        )?,
        material_library_revision: required_raw_column_for(
            row,
            path,
            "material_library_revision",
            "controlled_impedance_solver_material_process",
        )?,
        fabricator_stackup_revision: required_raw_column_for(
            row,
            path,
            "fabricator_stackup_revision",
            "controlled_impedance_solver_material_process",
        )?,
        dielectric_layer: required_raw_column_for(
            row,
            path,
            "dielectric_layer",
            "controlled_impedance_solver_material_process",
        )?,
        material: required_raw_column_for(
            row,
            path,
            "material",
            "controlled_impedance_solver_material_process",
        )?,
        process_lot: required_raw_column_for(
            row,
            path,
            "process_lot",
            "controlled_impedance_solver_material_process",
        )?,
        material_lot: required_raw_column_for(
            row,
            path,
            "material_lot",
            "controlled_impedance_solver_material_process",
        )?,
        process_revision: required_raw_column_for(
            row,
            path,
            "process_revision",
            "controlled_impedance_solver_material_process",
        )?,
        drift_artifact_uri: required_raw_column_for(
            row,
            path,
            "drift_artifact_uri",
            "controlled_impedance_solver_material_process",
        )?,
        drift_artifact_sha256: required_sha256_column(
            row,
            path,
            "drift_artifact_sha256",
            "controlled_impedance_solver_material_process",
        )?,
        accepted_dielectric_constant: required_positive_number(
            row,
            path,
            "accepted_dielectric_constant",
            "controlled_impedance_solver_material_process",
        )?,
        measured_dielectric_constant: required_positive_number(
            row,
            path,
            "measured_dielectric_constant",
            "controlled_impedance_solver_material_process",
        )?,
        max_dielectric_constant_delta: required_nonnegative_number(
            row,
            path,
            "max_dielectric_constant_delta",
            "controlled_impedance_solver_material_process",
        )?,
        accepted_thickness_mm: required_positive_number(
            row,
            path,
            "accepted_thickness_mm",
            "controlled_impedance_solver_material_process",
        )?,
        measured_thickness_mm: required_positive_number(
            row,
            path,
            "measured_thickness_mm",
            "controlled_impedance_solver_material_process",
        )?,
        max_thickness_delta_mm: required_nonnegative_number(
            row,
            path,
            "max_thickness_delta_mm",
            "controlled_impedance_solver_material_process",
        )?,
    })
}

fn required_coupon_type(row: &MetadataCsvRow, path: &Path) -> Result<String> {
    let raw = required_raw_column_for(row, path, "coupon_type", "controlled_impedance_coupon")?;
    match normalize_name(&raw).as_str() {
        "singleended" | "single" | "se" => Ok("single_ended".to_string()),
        "differential" | "diff" | "differentialpair" | "pair" => Ok("differential".to_string()),
        _ => bail!(
            "Manufacturing metadata CSV {} row {} controlled_impedance_coupon coupon_type must be single_ended or differential.",
            path.display(),
            row.row_number
        ),
    }
}

fn required_solver_result_type(row: &MetadataCsvRow, path: &Path) -> Result<String> {
    let raw = required_raw_column_for(
        row,
        path,
        "result_type",
        "controlled_impedance_solver_result",
    )?;
    match normalize_name(&raw).as_str() {
        "singleended" | "single" | "se" => Ok("single_ended".to_string()),
        "differential" | "diff" | "differentialpair" | "pair" => Ok("differential".to_string()),
        _ => bail!(
            "Manufacturing metadata CSV {} row {} controlled_impedance_solver_result result_type must be single_ended or differential.",
            path.display(),
            row.row_number
        ),
    }
}

fn required_solver_artifact_sha256(row: &MetadataCsvRow, path: &Path) -> Result<String> {
    let digest = required_raw_column_for(
        row,
        path,
        "solver_artifact_sha256",
        "controlled_impedance_solver_result",
    )?;
    if is_sha256_hex(&digest) {
        Ok(digest.to_ascii_lowercase())
    } else {
        bail!(
            "Manufacturing metadata CSV {} row {} controlled_impedance_solver_result solver_artifact_sha256 must be a 64-character SHA-256 hex digest.",
            path.display(),
            row.row_number
        )
    }
}

fn optional_solver_input_deck_sha256(row: &MetadataCsvRow, path: &Path) -> Result<Option<String>> {
    let Some(digest) = optional_raw_column(row, "solver_input_deck_sha256") else {
        return Ok(None);
    };
    if is_sha256_hex(&digest) {
        Ok(Some(digest.to_ascii_lowercase()))
    } else {
        bail!(
            "Manufacturing metadata CSV {} row {} controlled_impedance_solver_result solver_input_deck_sha256 must be a 64-character SHA-256 hex digest.",
            path.display(),
            row.row_number
        )
    }
}

fn optional_solver_artifact_signature_sha256(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<Option<String>> {
    let Some(digest) = optional_raw_column(row, "solver_artifact_signature_sha256") else {
        return Ok(None);
    };
    if is_sha256_hex(&digest) {
        Ok(Some(digest.to_ascii_lowercase()))
    } else {
        bail!(
            "Manufacturing metadata CSV {} row {} controlled_impedance_solver_result solver_artifact_signature_sha256 must be a 64-character SHA-256 hex digest.",
            path.display(),
            row.row_number
        )
    }
}

fn optional_solver_output_schema_sha256(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<Option<String>> {
    let Some(digest) = optional_raw_column(row, "solver_output_schema_sha256") else {
        return Ok(None);
    };
    if is_sha256_hex(&digest) {
        Ok(Some(digest.to_ascii_lowercase()))
    } else {
        bail!(
            "Manufacturing metadata CSV {} row {} controlled_impedance_solver_result solver_output_schema_sha256 must be a 64-character SHA-256 hex digest.",
            path.display(),
            row.row_number
        )
    }
}

fn optional_solver_config_lock_sha256(row: &MetadataCsvRow, path: &Path) -> Result<Option<String>> {
    let Some(digest) = optional_raw_column(row, "solver_config_lock_sha256") else {
        return Ok(None);
    };
    if is_sha256_hex(&digest) {
        Ok(Some(digest.to_ascii_lowercase()))
    } else {
        bail!(
            "Manufacturing metadata CSV {} row {} controlled_impedance_solver_result solver_config_lock_sha256 must be a 64-character SHA-256 hex digest.",
            path.display(),
            row.row_number
        )
    }
}

fn optional_solver_material_library_artifact_sha256(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<Option<String>> {
    let Some(digest) = optional_raw_column(row, "solver_material_library_artifact_sha256") else {
        return Ok(None);
    };
    if is_sha256_hex(&digest) {
        Ok(Some(digest.to_ascii_lowercase()))
    } else {
        bail!(
            "Manufacturing metadata CSV {} row {} controlled_impedance_solver_result solver_material_library_artifact_sha256 must be a 64-character SHA-256 hex digest.",
            path.display(),
            row.row_number
        )
    }
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

fn optional_stackup_signoff_artifact_sha256(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<Option<String>> {
    let Some(digest) = optional_raw_column(row, "stackup_signoff_artifact_sha256") else {
        return Ok(None);
    };
    if is_sha256_hex(&digest) {
        Ok(Some(digest.to_ascii_lowercase()))
    } else {
        bail!(
            "Manufacturing metadata CSV {} row {} controlled_impedance_solver_result stackup_signoff_artifact_sha256 must be a 64-character SHA-256 hex digest.",
            path.display(),
            row.row_number
        )
    }
}

fn required_solver_qualification_sha256(row: &MetadataCsvRow, path: &Path) -> Result<String> {
    let digest = required_raw_column_for(
        row,
        path,
        "qualification_artifact_sha256",
        "controlled_impedance_solver_qualification",
    )?;
    if is_sha256_hex(&digest) {
        Ok(digest.to_ascii_lowercase())
    } else {
        bail!(
            "Manufacturing metadata CSV {} row {} controlled_impedance_solver_qualification qualification_artifact_sha256 must be a 64-character SHA-256 hex digest.",
            path.display(),
            row.row_number
        )
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn applied_thermal_copper(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedThermalCopper> {
    let name = required_raw_column_for(row, path, "name", "thermal_copper")?;
    let component = required_raw_column_for(row, path, "component", "thermal_copper")?;
    let thermal_source = optional_raw_column(row, "thermal_source");
    let source = row
        .source
        .as_deref()
        .or(thermal_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} thermal_copper requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    let min_copper_area_mm2 =
        parse_positive_area_mm2(row.value.trim(), row.unit.as_deref(), path, row, "value")?;
    let power_loss_w = required_positive_watts(row, path, "power_loss_w", "thermal_copper")?;
    Ok(AppliedThermalCopper {
        name,
        component,
        source,
        power_loss_w,
        min_copper_area_mm2,
        min_thermal_via_count: optional_raw_column(row, "min_thermal_via_count")
            .as_deref()
            .map(|value| parse_positive_usize(value, path, row, "min_thermal_via_count"))
            .transpose()?,
        min_plated_thermal_via_count: optional_raw_column(row, "min_plated_thermal_via_count")
            .as_deref()
            .map(|value| parse_positive_usize(value, path, row, "min_plated_thermal_via_count"))
            .transpose()?,
        min_thermal_via_drill_mm: optional_raw_column(row, "min_thermal_via_drill_mm")
            .as_deref()
            .map(|value| parse_positive_number(value, path, row, "min_thermal_via_drill_mm"))
            .transpose()?,
        min_thermal_via_plating_thickness_um: optional_raw_column(
            row,
            "min_thermal_via_plating_thickness_um",
        )
        .as_deref()
        .map(|value| {
            parse_positive_number(value, path, row, "min_thermal_via_plating_thickness_um")
        })
        .transpose()?,
        min_total_thermal_via_barrel_cross_section_mm2: optional_raw_column(
            row,
            "min_total_thermal_via_barrel_cross_section_mm2",
        )
        .as_deref()
        .map(|value| {
            parse_positive_number(
                value,
                path,
                row,
                "min_total_thermal_via_barrel_cross_section_mm2",
            )
        })
        .transpose()?,
        min_copper_thickness_um: optional_raw_column(row, "min_copper_thickness_um")
            .as_deref()
            .map(|value| parse_positive_number(value, path, row, "min_copper_thickness_um"))
            .transpose()?,
        rated_ambient_temperature_c: optional_raw_column(row, "rated_ambient_temperature_C")
            .as_deref()
            .map(|value| parse_temperature_c(value, None, path, row, "rated_ambient_temperature_C"))
            .transpose()?,
        min_airflow_lfm: optional_raw_column(row, "min_airflow_lfm")
            .as_deref()
            .map(|value| parse_nonnegative_number(value, path, row, "min_airflow_lfm"))
            .transpose()?,
        enclosure_profile: optional_raw_column(row, "enclosure_profile"),
        nets: optional_raw_column(row, "nets")
            .map(|value| parse_nonempty_list(&value))
            .transpose()?
            .unwrap_or_default(),
        layers: optional_raw_column(row, "layers")
            .map(|value| parse_nonempty_list(&value))
            .transpose()?
            .unwrap_or_default(),
    })
}

pub(super) fn applied_thermal_measurement(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedThermalMeasurement> {
    let name = required_raw_column_for(row, path, "name", "thermal_measurement")?;
    let component = required_raw_column_for(row, path, "component", "thermal_measurement")?;
    let measurement_source = optional_raw_column(row, "measurement_source");
    let source = row
        .source
        .as_deref()
        .or(measurement_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} thermal_measurement requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    let measured_temperature_c =
        parse_temperature_c(row.value.trim(), row.unit.as_deref(), path, row, "value")?;
    let ambient_temperature_c = optional_raw_column(row, "ambient_temperature_C")
        .as_deref()
        .map(|value| parse_temperature_c(value, None, path, row, "ambient_temperature_C"))
        .transpose()?;
    let measurement_uncertainty_c = optional_raw_column(row, "measurement_uncertainty_C")
        .as_deref()
        .map(|value| {
            parse_nonnegative_temperature_delta_c(value, path, row, "measurement_uncertainty_C")
        })
        .transpose()?;
    let power_loss_w = optional_raw_column(row, "power_loss_w")
        .as_deref()
        .map(|value| parse_positive_watts(value, path, row, "power_loss_w"))
        .transpose()?;
    Ok(AppliedThermalMeasurement {
        name,
        component,
        source,
        measured_temperature_c,
        ambient_temperature_c,
        measurement_uncertainty_c,
        power_loss_w,
        measurement_point: optional_raw_column(row, "measurement_point"),
        notes: row.notes.clone(),
    })
}

pub(super) fn applied_thermal_package(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedThermalPackage> {
    let package_source = optional_raw_column(row, "package_source");
    let source = row
        .source
        .as_deref()
        .or(package_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} thermal_package requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedThermalPackage {
        component: required_raw_column_for(row, path, "component", "thermal_package")?,
        source,
        thermal_resistance_junction_to_ambient_c_per_w: parse_positive_c_per_w(
            row.value.trim(),
            row.unit.as_deref(),
            path,
            row,
            "value",
        )?,
        max_junction_temperature_c: parse_temperature_c(
            &required_raw_column_for(row, path, "max_junction_temperature_C", "thermal_package")?,
            None,
            path,
            row,
            "max_junction_temperature_C",
        )?,
    })
}

pub(super) fn applied_thermal_environment(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedThermalEnvironment> {
    let name = required_raw_column_for(row, path, "name", "thermal_environment")?;
    let environment_source = optional_raw_column(row, "environment_source");
    let source = row
        .source
        .as_deref()
        .or(environment_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} thermal_environment requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedThermalEnvironment {
        name,
        source,
        ambient_temperature_c: parse_temperature_c(
            row.value.trim(),
            row.unit.as_deref(),
            path,
            row,
            "value",
        )?,
        airflow_lfm: optional_raw_column(row, "airflow_lfm")
            .as_deref()
            .map(|value| parse_nonnegative_number(value, path, row, "airflow_lfm"))
            .transpose()?,
        enclosure_profile: optional_raw_column(row, "enclosure_profile"),
    })
}

pub(super) fn applied_thermal_limit(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedThermalLimit> {
    let name = required_raw_column_for(row, path, "name", "thermal_limit")?;
    let limit_source = optional_raw_column(row, "limit_source");
    let source = row
        .source
        .as_deref()
        .or(limit_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} thermal_limit requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedThermalLimit {
        name,
        source,
        component: optional_raw_column(row, "component"),
        max_measured_temperature_c: parse_temperature_c(
            row.value.trim(),
            row.unit.as_deref(),
            path,
            row,
            "value",
        )?,
        max_temperature_rise_c: optional_raw_column(row, "max_temperature_rise_C")
            .as_deref()
            .map(|value| parse_positive_number(value, path, row, "max_temperature_rise_C"))
            .transpose()?,
        max_junction_temperature_margin_c: optional_raw_column(
            row,
            "max_junction_temperature_margin_C",
        )
        .as_deref()
        .map(|value| {
            parse_nonnegative_temperature_delta_c(
                value,
                path,
                row,
                "max_junction_temperature_margin_C",
            )
        })
        .transpose()?,
    })
}

pub(super) fn applied_stackup_layer(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedStackupLayer> {
    let name = required_raw_column_for(row, path, "name", "stackup_layer")?;
    let stackup_source = optional_raw_column(row, "stackup_source");
    let source = row
        .source
        .as_deref()
        .or(stackup_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} stackup_layer requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedStackupLayer {
        name,
        kind: parse_stackup_layer_kind(row.value.trim(), path, row)?,
        source,
        reference_net: optional_raw_column(row, "reference_net"),
        thickness_mm: optional_raw_column(row, "thickness_mm")
            .as_deref()
            .map(|value| parse_positive_number(value, path, row, "thickness_mm"))
            .transpose()?,
        copper_thickness_um: optional_raw_column(row, "copper_thickness_um")
            .as_deref()
            .map(|value| parse_positive_number(value, path, row, "copper_thickness_um"))
            .transpose()?,
        dielectric_constant: optional_raw_column(row, "dielectric_constant")
            .as_deref()
            .map(|value| parse_positive_number(value, path, row, "dielectric_constant"))
            .transpose()?,
        material: optional_raw_column(row, "material"),
    })
}

fn parse_stackup_layer_kind(raw: &str, path: &Path, row: &MetadataCsvRow) -> Result<String> {
    match normalize_name(raw).as_str() {
        "signal" | "copper" | "routing" => Ok("signal".to_string()),
        "plane" | "referenceplane" | "powerplane" | "groundplane" => Ok("plane".to_string()),
        "dielectric" | "core" | "prepreg" | "insulator" => Ok("dielectric".to_string()),
        "other" | "mask" | "soldermask" => Ok("other".to_string()),
        _ => bail!(
            "Manufacturing metadata CSV {} row {} stackup_layer kind must be signal, plane, dielectric, or other.",
            path.display(),
            row.row_number
        ),
    }
}
