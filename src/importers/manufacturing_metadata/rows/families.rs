use super::{
    AppliedControlledImpedanceCoupon, AppliedControlledImpedanceCouponSample,
    AppliedControlledImpedanceNet, AppliedControlledImpedancePair,
    AppliedControlledImpedanceSolverResult, AppliedControlledImpedanceSolverSample,
    AppliedLayoutPoint, AppliedRfAntennaFeedPath, AppliedRfAntennaKeepout,
    AppliedRfAntennaMatchingElement, AppliedRfAntennaMatchingNetwork, AppliedRfAntennaMeasurement,
    AppliedRfAntennaMeasurementCondition, AppliedRfAntennaPerformanceLimit, AppliedStackupLayer,
    AppliedThermalCopper, AppliedThermalEnvironment, AppliedThermalLimit,
    AppliedThermalMeasurement, AppliedThermalPackage, MetadataCsvRow, normalize_name,
    optional_raw_column, parse_nonempty_list, parse_nonnegative_mm, parse_nonnegative_number,
    parse_nonnegative_temperature_delta_c, parse_positive_area_mm2, parse_positive_c_per_w,
    parse_positive_number, parse_positive_ohms, parse_positive_usize, parse_positive_watts,
    parse_temperature_c, required_nonnegative_number, required_positive_number,
    required_positive_watts, required_raw_column_for,
};
use anyhow::{Context, Result, bail};
use std::path::Path;

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
        solved_gap_mm: optional_raw_column(row, "solved_gap_mm")
            .map(|value| parse_positive_number(&value, path, row, "solved_gap_mm"))
            .transpose()?,
        max_route_gap_delta_mm: optional_raw_column(row, "max_route_gap_delta_mm")
            .map(|value| parse_nonnegative_number(&value, path, row, "max_route_gap_delta_mm"))
            .transpose()?,
        frequency_mhz: optional_raw_column(row, "frequency_mhz")
            .map(|value| parse_positive_number(&value, path, row, "frequency_mhz"))
            .transpose()?,
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

pub(super) fn applied_rf_antenna_keepout(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedRfAntennaKeepout> {
    let keepout_source = optional_raw_column(row, "keepout_source");
    let source = row
        .source
        .as_deref()
        .or(keepout_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} rf_antenna_keepout requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedRfAntennaKeepout {
        name: required_raw_column_for(row, path, "name", "rf_antenna_keepout")?,
        antenna_net: optional_raw_column(row, "antenna_net"),
        layer: required_raw_column_for(row, path, "layer", "rf_antenna_keepout")?,
        polygon: parse_polygon_points(
            &required_raw_column_for(row, path, "polygon", "rf_antenna_keepout")?,
            path,
            row,
            "polygon",
        )?,
        min_copper_clearance_mm: parse_nonnegative_mm(
            row.value.trim(),
            row.unit.as_deref(),
            path,
            row,
            "value",
        )?,
        source,
    })
}

pub(super) fn applied_rf_antenna_feed_path(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedRfAntennaFeedPath> {
    let feed_path_source = optional_raw_column(row, "feed_path_source");
    let source = row
        .source
        .as_deref()
        .or(feed_path_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} rf_antenna_feed_path requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedRfAntennaFeedPath {
        name: required_raw_column_for(row, path, "name", "rf_antenna_feed_path")?,
        antenna_net: required_raw_column_for(row, path, "antenna_net", "rf_antenna_feed_path")?,
        feed_component: required_raw_column_for(
            row,
            path,
            "feed_component",
            "rf_antenna_feed_path",
        )?,
        feed_pin: required_raw_column_for(row, path, "feed_pin", "rf_antenna_feed_path")?,
        matching_components: parse_nonempty_list(&required_raw_column_for(
            row,
            path,
            "matching_components",
            "rf_antenna_feed_path",
        )?)?,
        max_feed_route_length_mm: parse_nonnegative_mm(
            row.value.trim(),
            row.unit.as_deref(),
            path,
            row,
            "value",
        )?,
        max_matching_component_distance_mm: required_nonnegative_number(
            row,
            path,
            "max_matching_component_distance_mm",
            "rf_antenna_feed_path",
        )?,
        source,
    })
}

pub(super) fn applied_rf_antenna_matching_network(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedRfAntennaMatchingNetwork> {
    let matching_source = optional_raw_column(row, "matching_source");
    let source = row
        .source
        .as_deref()
        .or(matching_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} rf_antenna_matching_network requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedRfAntennaMatchingNetwork {
        name: required_raw_column_for(row, path, "name", "rf_antenna_matching_network")?,
        antenna_net: required_raw_column_for(
            row,
            path,
            "antenna_net",
            "rf_antenna_matching_network",
        )?,
        topology: parse_rf_matching_topology(row.value.trim(), path, row)?,
        source,
        reference_net: optional_raw_column(row, "reference_net"),
        elements: parse_rf_matching_elements(
            &required_raw_column_for(row, path, "elements", "rf_antenna_matching_network")?,
            path,
            row,
        )?,
    })
}

pub(super) fn applied_rf_antenna_measurement(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedRfAntennaMeasurement> {
    let measurement_source = optional_raw_column(row, "measurement_source");
    let source = row
        .source
        .as_deref()
        .or(measurement_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} rf_antenna_measurement requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedRfAntennaMeasurement {
        name: required_raw_column_for(row, path, "name", "rf_antenna_measurement")?,
        antenna_net: required_raw_column_for(row, path, "antenna_net", "rf_antenna_measurement")?,
        frequency_mhz: required_positive_number(
            row,
            path,
            "frequency_mhz",
            "rf_antenna_measurement",
        )?,
        return_loss_db: parse_positive_db(row.value.trim(), row.unit.as_deref(), path, row)?,
        source,
        measurement_method: optional_raw_column(row, "measurement_method"),
        measurement_condition: optional_raw_column(row, "measurement_condition"),
        notes: row.notes.clone(),
    })
}

pub(super) fn applied_rf_antenna_performance_limit(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedRfAntennaPerformanceLimit> {
    let limit_source = optional_raw_column(row, "limit_source");
    let source = row
        .source
        .as_deref()
        .or(limit_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} rf_antenna_performance_limit requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    let frequency_min_mhz = optional_raw_column(row, "frequency_min_mhz")
        .as_deref()
        .map(|value| parse_positive_number(value, path, row, "frequency_min_mhz"))
        .transpose()?;
    let frequency_max_mhz = optional_raw_column(row, "frequency_max_mhz")
        .as_deref()
        .map(|value| parse_positive_number(value, path, row, "frequency_max_mhz"))
        .transpose()?;
    if let (Some(min_mhz), Some(max_mhz)) = (frequency_min_mhz, frequency_max_mhz)
        && max_mhz + f64::EPSILON < min_mhz
    {
        bail!(
            "Manufacturing metadata CSV {} row {} rf_antenna_performance_limit frequency_max_mhz must be greater than or equal to frequency_min_mhz.",
            path.display(),
            row.row_number
        );
    }
    let min_measurement_count = optional_raw_column(row, "min_measurement_count")
        .as_deref()
        .map(|value| parse_positive_usize(value, path, row, "min_measurement_count"))
        .transpose()?;
    Ok(AppliedRfAntennaPerformanceLimit {
        name: required_raw_column_for(row, path, "name", "rf_antenna_performance_limit")?,
        antenna_net: required_raw_column_for(
            row,
            path,
            "antenna_net",
            "rf_antenna_performance_limit",
        )?,
        min_return_loss_db: parse_positive_db(row.value.trim(), row.unit.as_deref(), path, row)?,
        source,
        frequency_min_mhz,
        frequency_max_mhz,
        min_measurement_count,
        max_frequency_step_mhz: optional_raw_column(row, "max_frequency_step_mhz")
            .as_deref()
            .map(|value| parse_positive_number(value, path, row, "max_frequency_step_mhz"))
            .transpose()?,
        required_measurement_condition: optional_raw_column(row, "required_measurement_condition"),
        notes: row.notes.clone(),
    })
}

pub(super) fn applied_rf_antenna_measurement_condition(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedRfAntennaMeasurementCondition> {
    let condition_source = optional_raw_column(row, "condition_source");
    let source = row
        .source
        .as_deref()
        .or(condition_source.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .with_context(|| {
            format!(
                "Manufacturing metadata CSV {} row {} rf_antenna_measurement_condition requires source.",
                path.display(),
                row.row_number
            )
        })?
        .to_string();
    Ok(AppliedRfAntennaMeasurementCondition {
        name: required_raw_column_for(row, path, "name", "rf_antenna_measurement_condition")?,
        source,
        fixture: optional_raw_column(row, "fixture"),
        cable_setup: optional_raw_column(row, "cable_setup"),
        enclosure_profile: optional_raw_column(row, "enclosure_profile"),
        notes: row.notes.clone(),
    })
}

fn parse_rf_matching_topology(raw: &str, path: &Path, row: &MetadataCsvRow) -> Result<String> {
    match normalize_name(raw).as_str() {
        "series" => Ok("series".to_string()),
        "l" | "lnetwork" | "lmatch" => Ok("l".to_string()),
        "pi" | "pinetwork" | "pimatch" => Ok("pi".to_string()),
        "t" | "tnetwork" | "tmatch" => Ok("t".to_string()),
        "custom" => Ok("custom".to_string()),
        _ => bail!(
            "Manufacturing metadata CSV {} row {} rf_antenna_matching_network value must be series, l, pi, t, or custom topology.",
            path.display(),
            row.row_number
        ),
    }
}

fn parse_rf_matching_elements(
    raw: &str,
    path: &Path,
    row: &MetadataCsvRow,
) -> Result<Vec<AppliedRfAntennaMatchingElement>> {
    let elements = raw
        .split([';', '|'])
        .map(str::trim)
        .filter(|element| !element.is_empty())
        .map(|element| parse_rf_matching_element(element, path, row))
        .collect::<Result<Vec<_>>>()?;
    if elements.is_empty() {
        bail!(
            "Manufacturing metadata CSV {} row {} rf_antenna_matching_network elements must contain at least one element.",
            path.display(),
            row.row_number
        );
    }
    Ok(elements)
}

fn parse_rf_matching_element(
    raw: &str,
    path: &Path,
    row: &MetadataCsvRow,
) -> Result<AppliedRfAntennaMatchingElement> {
    let parts = raw
        .split([':', '>'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    match parts.as_slice() {
        ["series", component, input_net, output_net] => Ok(AppliedRfAntennaMatchingElement {
            component: (*component).to_string(),
            role: "series".to_string(),
            input_net: Some((*input_net).to_string()),
            output_net: Some((*output_net).to_string()),
            signal_net: None,
            reference_net: None,
        }),
        ["shunt", component, signal_net] => Ok(AppliedRfAntennaMatchingElement {
            component: (*component).to_string(),
            role: "shunt".to_string(),
            input_net: None,
            output_net: None,
            signal_net: Some((*signal_net).to_string()),
            reference_net: None,
        }),
        ["shunt", component, signal_net, reference_net] => Ok(AppliedRfAntennaMatchingElement {
            component: (*component).to_string(),
            role: "shunt".to_string(),
            input_net: None,
            output_net: None,
            signal_net: Some((*signal_net).to_string()),
            reference_net: Some((*reference_net).to_string()),
        }),
        _ => bail!(
            "Manufacturing metadata CSV {} row {} rf_antenna_matching_network element {raw} must be series:COMPONENT:INPUT_NET:OUTPUT_NET or shunt:COMPONENT:SIGNAL_NET[:REFERENCE_NET].",
            path.display(),
            row.row_number
        ),
    }
}

fn parse_positive_db(
    raw: &str,
    unit: Option<&str>,
    path: &Path,
    row: &MetadataCsvRow,
) -> Result<f64> {
    let unit = unit.map(normalize_name);
    if !matches!(
        unit.as_deref(),
        None | Some("") | Some("db") | Some("decibel") | Some("decibels")
    ) {
        bail!(
            "Manufacturing metadata CSV {} row {} must use dB for rf_antenna_measurement value.",
            path.display(),
            row.row_number
        );
    }
    parse_positive_number(raw, path, row, "value")
}

fn parse_polygon_points(
    raw: &str,
    path: &Path,
    row: &MetadataCsvRow,
    column: &str,
) -> Result<Vec<AppliedLayoutPoint>> {
    let points: Vec<AppliedLayoutPoint> = raw
        .split([';', '|'])
        .map(str::trim)
        .filter(|point| !point.is_empty())
        .map(|point| parse_polygon_point(point, path, row, column))
        .collect::<Result<Vec<_>>>()?;
    if points.len() < 3 {
        bail!(
            "Manufacturing metadata CSV {} row {} {column} must contain at least three x:y points.",
            path.display(),
            row.row_number
        );
    }
    if polygon_area_mm2(&points) <= f64::EPSILON {
        bail!(
            "Manufacturing metadata CSV {} row {} {column} must have non-zero area.",
            path.display(),
            row.row_number
        );
    }
    Ok(points)
}

fn parse_polygon_point(
    raw: &str,
    path: &Path,
    row: &MetadataCsvRow,
    column: &str,
) -> Result<AppliedLayoutPoint> {
    let mut parts = raw
        .split([':', '/', ' '])
        .map(str::trim)
        .filter(|part| !part.is_empty());
    let Some(x_raw) = parts.next() else {
        bail!(
            "Manufacturing metadata CSV {} row {} {column} point {raw} is missing x.",
            path.display(),
            row.row_number
        );
    };
    let Some(y_raw) = parts.next() else {
        bail!(
            "Manufacturing metadata CSV {} row {} {column} point {raw} is missing y.",
            path.display(),
            row.row_number
        );
    };
    if parts.next().is_some() {
        bail!(
            "Manufacturing metadata CSV {} row {} {column} point {raw} must be x:y.",
            path.display(),
            row.row_number
        );
    }
    let x_mm = x_raw.parse::<f64>().with_context(|| {
        format!(
            "Manufacturing metadata CSV {} row {} has invalid {column} x coordinate {x_raw}.",
            path.display(),
            row.row_number
        )
    })?;
    let y_mm = y_raw.parse::<f64>().with_context(|| {
        format!(
            "Manufacturing metadata CSV {} row {} has invalid {column} y coordinate {y_raw}.",
            path.display(),
            row.row_number
        )
    })?;
    if !x_mm.is_finite() || !y_mm.is_finite() {
        bail!(
            "Manufacturing metadata CSV {} row {} {column} point {raw} must be finite.",
            path.display(),
            row.row_number
        );
    }
    Ok(AppliedLayoutPoint { x_mm, y_mm })
}

fn polygon_area_mm2(points: &[AppliedLayoutPoint]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(left, right)| left.x_mm * right.y_mm - right.x_mm * left.y_mm)
        .sum::<f64>()
        .abs()
        / 2.0
}
