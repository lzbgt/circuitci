use super::{
    AppliedControlledImpedanceCoupon, AppliedControlledImpedanceCouponSample,
    AppliedControlledImpedanceNet, AppliedControlledImpedancePair,
    AppliedControlledImpedanceSolverConvergenceSample, AppliedControlledImpedanceSolverEntitlement,
    AppliedControlledImpedanceSolverExecutionEnvironment,
    AppliedControlledImpedanceSolverMaterialAcceptance,
    AppliedControlledImpedanceSolverMaterialCorner,
    AppliedControlledImpedanceSolverMaterialLibrary,
    AppliedControlledImpedanceSolverMaterialProcess, AppliedControlledImpedanceSolverQualification,
    AppliedControlledImpedanceSolverRerun, AppliedControlledImpedanceSolverResult,
    AppliedControlledImpedanceSolverRunLog, AppliedControlledImpedanceSolverRuntimeAllowlist,
    AppliedControlledImpedanceSolverSample, AppliedField, AppliedLayoutPoint,
    AppliedRfAntennaFeedPath, AppliedRfAntennaKeepout, AppliedRfAntennaMatchingElement,
    AppliedRfAntennaMatchingNetwork, AppliedRfAntennaMeasurement,
    AppliedRfAntennaMeasurementCondition, AppliedRfAntennaPerformanceLimit, AppliedStackupLayer,
    AppliedThermalCopper, AppliedThermalEnvironment, AppliedThermalLimit,
    AppliedThermalMeasurement, AppliedThermalPackage,
};
use anyhow::{Context, Result};
use serde::Serialize;
use serde_yaml_ng::Value;
use std::collections::BTreeMap;

pub(super) fn normalized_yaml_value(field: &AppliedField) -> Result<Value> {
    if let Some(target) = &field.controlled_impedance_net {
        return serde_yaml_ng::to_value(controlled_impedance_net_mapping(target)).with_context(
            || {
                format!(
                    "Failed to encode manufacturing metadata {}.",
                    field.field.board_key()
                )
            },
        );
    }
    if let Some(target) = &field.controlled_impedance_pair {
        return serde_yaml_ng::to_value(controlled_impedance_pair_mapping(target)).with_context(
            || {
                format!(
                    "Failed to encode manufacturing metadata {}.",
                    field.field.board_key()
                )
            },
        );
    }
    if let Some(coupon) = &field.controlled_impedance_coupon {
        return serde_yaml_ng::to_value(controlled_impedance_coupon_mapping(coupon)).with_context(
            || {
                format!(
                    "Failed to encode manufacturing metadata {}.",
                    field.field.board_key()
                )
            },
        );
    }
    if let Some(sample) = &field.controlled_impedance_coupon_sample {
        return serde_yaml_ng::to_value(controlled_impedance_coupon_sample_mapping(sample))
            .with_context(|| {
                format!(
                    "Failed to encode manufacturing metadata {}.",
                    field.field.board_key()
                )
            });
    }
    if let Some(result) = &field.controlled_impedance_solver_result {
        return serde_yaml_ng::to_value(controlled_impedance_solver_result_mapping(result))
            .with_context(|| {
                format!(
                    "Failed to encode manufacturing metadata {}.",
                    field.field.board_key()
                )
            });
    }
    if let Some(sample) = &field.controlled_impedance_solver_sample {
        return serde_yaml_ng::to_value(controlled_impedance_solver_sample_mapping(sample))
            .with_context(|| {
                format!(
                    "Failed to encode manufacturing metadata {}.",
                    field.field.board_key()
                )
            });
    }
    if let Some(corner) = &field.controlled_impedance_solver_material_corner {
        return serde_yaml_ng::to_value(controlled_impedance_solver_material_corner_mapping(
            corner,
        ))
        .with_context(|| {
            format!(
                "Failed to encode manufacturing metadata {}.",
                field.field.board_key()
            )
        });
    }
    if let Some(qualification) = &field.controlled_impedance_solver_qualification {
        return serde_yaml_ng::to_value(controlled_impedance_solver_qualification_mapping(
            qualification,
        ))
        .with_context(|| {
            format!(
                "Failed to encode manufacturing metadata {}.",
                field.field.board_key()
            )
        });
    }
    if let Some(library) = &field.controlled_impedance_solver_material_library {
        return serde_yaml_ng::to_value(controlled_impedance_solver_material_library_mapping(
            library,
        ))
        .with_context(|| {
            format!(
                "Failed to encode manufacturing metadata {}.",
                field.field.board_key()
            )
        });
    }
    if let Some(acceptance) = &field.controlled_impedance_solver_material_acceptance {
        return serde_yaml_ng::to_value(controlled_impedance_solver_material_acceptance_mapping(
            acceptance,
        ))
        .with_context(|| {
            format!(
                "Failed to encode manufacturing metadata {}.",
                field.field.board_key()
            )
        });
    }
    if let Some(process) = &field.controlled_impedance_solver_material_process {
        return serde_yaml_ng::to_value(controlled_impedance_solver_material_process_mapping(
            process,
        ))
        .with_context(|| {
            format!(
                "Failed to encode manufacturing metadata {}.",
                field.field.board_key()
            )
        });
    }
    if let Some(allowlist) = &field.controlled_impedance_solver_runtime_allowlist {
        return serde_yaml_ng::to_value(controlled_impedance_solver_runtime_allowlist_mapping(
            allowlist,
        ))
        .with_context(|| {
            format!(
                "Failed to encode manufacturing metadata {}.",
                field.field.board_key()
            )
        });
    }
    if let Some(entitlement) = &field.controlled_impedance_solver_entitlement {
        return serde_yaml_ng::to_value(controlled_impedance_solver_entitlement_mapping(
            entitlement,
        ))
        .with_context(|| {
            format!(
                "Failed to encode manufacturing metadata {}.",
                field.field.board_key()
            )
        });
    }
    if let Some(environment) = &field.controlled_impedance_solver_execution_environment {
        return serde_yaml_ng::to_value(controlled_impedance_solver_execution_environment_mapping(
            environment,
        ))
        .with_context(|| {
            format!(
                "Failed to encode manufacturing metadata {}.",
                field.field.board_key()
            )
        });
    }
    if let Some(run_log) = &field.controlled_impedance_solver_run_log {
        return serde_yaml_ng::to_value(controlled_impedance_solver_run_log_mapping(run_log))
            .with_context(|| {
                format!(
                    "Failed to encode manufacturing metadata {}.",
                    field.field.board_key()
                )
            });
    }
    if let Some(rerun) = &field.controlled_impedance_solver_rerun {
        return serde_yaml_ng::to_value(controlled_impedance_solver_rerun_mapping(rerun))
            .with_context(|| {
                format!(
                    "Failed to encode manufacturing metadata {}.",
                    field.field.board_key()
                )
            });
    }
    if let Some(sample) = &field.controlled_impedance_solver_convergence_sample {
        return serde_yaml_ng::to_value(controlled_impedance_solver_convergence_sample_mapping(
            sample,
        ))
        .with_context(|| {
            format!(
                "Failed to encode manufacturing metadata {}.",
                field.field.board_key()
            )
        });
    }
    if let Some(rule) = &field.thermal_copper {
        return serde_yaml_ng::to_value(thermal_copper_mapping(rule)).with_context(|| {
            format!(
                "Failed to encode manufacturing metadata {}.",
                field.field.board_key()
            )
        });
    }
    if let Some(measurement) = &field.thermal_measurement {
        return serde_yaml_ng::to_value(thermal_measurement_mapping(measurement)).with_context(
            || {
                format!(
                    "Failed to encode manufacturing metadata {}.",
                    field.field.board_key()
                )
            },
        );
    }
    if let Some(package) = &field.thermal_package {
        return serde_yaml_ng::to_value(thermal_package_mapping(package)).with_context(|| {
            format!(
                "Failed to encode manufacturing metadata {}.",
                field.field.board_key()
            )
        });
    }
    if let Some(environment) = &field.thermal_environment {
        return serde_yaml_ng::to_value(thermal_environment_mapping(environment)).with_context(
            || {
                format!(
                    "Failed to encode manufacturing metadata {}.",
                    field.field.board_key()
                )
            },
        );
    }
    if let Some(limit) = &field.thermal_limit {
        return serde_yaml_ng::to_value(thermal_limit_mapping(limit)).with_context(|| {
            format!(
                "Failed to encode manufacturing metadata {}.",
                field.field.board_key()
            )
        });
    }
    if let Some(layer) = &field.stackup_layer {
        return serde_yaml_ng::to_value(stackup_layer_mapping(layer)).with_context(|| {
            format!(
                "Failed to encode manufacturing metadata {}.",
                field.field.board_key()
            )
        });
    }
    if let Some(keepout) = &field.rf_antenna_keepout {
        return serde_yaml_ng::to_value(rf_antenna_keepout_mapping(keepout)).with_context(|| {
            format!(
                "Failed to encode manufacturing metadata {}.",
                field.field.board_key()
            )
        });
    }
    if let Some(feed_path) = &field.rf_antenna_feed_path {
        return serde_yaml_ng::to_value(rf_antenna_feed_path_mapping(feed_path)).with_context(
            || {
                format!(
                    "Failed to encode manufacturing metadata {}.",
                    field.field.board_key()
                )
            },
        );
    }
    if let Some(network) = &field.rf_antenna_matching_network {
        return serde_yaml_ng::to_value(rf_antenna_matching_network_mapping(network)).with_context(
            || {
                format!(
                    "Failed to encode manufacturing metadata {}.",
                    field.field.board_key()
                )
            },
        );
    }
    if let Some(measurement) = &field.rf_antenna_measurement {
        return serde_yaml_ng::to_value(rf_antenna_measurement_mapping(measurement)).with_context(
            || {
                format!(
                    "Failed to encode manufacturing metadata {}.",
                    field.field.board_key()
                )
            },
        );
    }
    if let Some(limit) = &field.rf_antenna_performance_limit {
        return serde_yaml_ng::to_value(rf_antenna_performance_limit_mapping(limit)).with_context(
            || {
                format!(
                    "Failed to encode manufacturing metadata {}.",
                    field.field.board_key()
                )
            },
        );
    }
    if let Some(condition) = &field.rf_antenna_measurement_condition {
        return serde_yaml_ng::to_value(rf_antenna_measurement_condition_mapping(condition))
            .with_context(|| {
                format!(
                    "Failed to encode manufacturing metadata {}.",
                    field.field.board_key()
                )
            });
    }
    if let Some(value) = field.numeric_value {
        return serde_yaml_ng::to_value(value).with_context(|| {
            format!(
                "Failed to encode manufacturing metadata {}.",
                field.field.board_key()
            )
        });
    }
    Ok(Value::String(
        field
            .string_value
            .as_ref()
            .context("source field must have a string value")?
            .clone(),
    ))
}

fn controlled_impedance_net_mapping(
    target: &AppliedControlledImpedanceNet,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("net".to_string(), Value::String(target.net.clone()));
    mapping.insert("source".to_string(), Value::String(target.source.clone()));
    mapping.insert(
        "target_impedance_ohm".to_string(),
        serde_yaml_ng::to_value(target.target_impedance_ohm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "expected_width_mm".to_string(),
        serde_yaml_ng::to_value(target.expected_width_mm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "max_width_error_mm".to_string(),
        serde_yaml_ng::to_value(target.max_width_error_mm).unwrap_or(Value::Null),
    );
    insert_optional_string(&mut mapping, "solder_mask_state", &target.solder_mask_state);
    insert_optional_string(&mut mapping, "solder_mask_layer", &target.solder_mask_layer);
    insert_optional_string(
        &mut mapping,
        "solder_mask_source",
        &target.solder_mask_source,
    );
    mapping
}

fn controlled_impedance_pair_mapping(
    target: &AppliedControlledImpedancePair,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert(
        "first_net".to_string(),
        Value::String(target.first_net.clone()),
    );
    mapping.insert(
        "second_net".to_string(),
        Value::String(target.second_net.clone()),
    );
    mapping.insert("source".to_string(), Value::String(target.source.clone()));
    mapping.insert(
        "target_differential_impedance_ohm".to_string(),
        serde_yaml_ng::to_value(target.target_differential_impedance_ohm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "expected_width_mm".to_string(),
        serde_yaml_ng::to_value(target.expected_width_mm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "expected_gap_mm".to_string(),
        serde_yaml_ng::to_value(target.expected_gap_mm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "max_width_error_mm".to_string(),
        serde_yaml_ng::to_value(target.max_width_error_mm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "max_gap_error_mm".to_string(),
        serde_yaml_ng::to_value(target.max_gap_error_mm).unwrap_or(Value::Null),
    );
    insert_optional_string(&mut mapping, "solder_mask_state", &target.solder_mask_state);
    insert_optional_string(&mut mapping, "solder_mask_layer", &target.solder_mask_layer);
    insert_optional_string(
        &mut mapping,
        "solder_mask_source",
        &target.solder_mask_source,
    );
    mapping
}

fn controlled_impedance_coupon_mapping(
    coupon: &AppliedControlledImpedanceCoupon,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(coupon.name.clone()));
    mapping.insert("source".to_string(), Value::String(coupon.source.clone()));
    mapping.insert(
        "coupon_type".to_string(),
        Value::String(coupon.coupon_type.clone()),
    );
    insert_optional_string(&mut mapping, "net", &coupon.net);
    insert_optional_string(&mut mapping, "first_net", &coupon.first_net);
    insert_optional_string(&mut mapping, "second_net", &coupon.second_net);
    mapping.insert(
        "target_impedance_ohm".to_string(),
        serde_yaml_ng::to_value(coupon.target_impedance_ohm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "measured_impedance_ohm".to_string(),
        serde_yaml_ng::to_value(coupon.measured_impedance_ohm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "max_impedance_error_ohm".to_string(),
        serde_yaml_ng::to_value(coupon.max_impedance_error_ohm).unwrap_or(Value::Null),
    );
    insert_optional_string(&mut mapping, "process_lot", &coupon.process_lot);
    insert_optional_string(&mut mapping, "panel_id", &coupon.panel_id);
    insert_optional_string(&mut mapping, "stackup_revision", &coupon.stackup_revision);
    insert_optional_string(
        &mut mapping,
        "coupon_trace_layer",
        &coupon.coupon_trace_layer,
    );
    insert_optional_number(
        &mut mapping,
        "coupon_trace_width_mm",
        coupon.coupon_trace_width_mm,
    );
    insert_optional_number(
        &mut mapping,
        "max_trace_width_delta_mm",
        coupon.max_trace_width_delta_mm,
    );
    insert_optional_number(
        &mut mapping,
        "coupon_trace_gap_mm",
        coupon.coupon_trace_gap_mm,
    );
    insert_optional_number(
        &mut mapping,
        "max_trace_gap_delta_mm",
        coupon.max_trace_gap_delta_mm,
    );
    insert_optional_number(
        &mut mapping,
        "min_batch_sample_count",
        coupon.min_batch_sample_count,
    );
    insert_optional_number(
        &mut mapping,
        "max_batch_mean_impedance_error_ohm",
        coupon.max_batch_mean_impedance_error_ohm,
    );
    insert_optional_number(
        &mut mapping,
        "max_batch_sample_impedance_error_ohm",
        coupon.max_batch_sample_impedance_error_ohm,
    );
    insert_optional_number(
        &mut mapping,
        "max_batch_stddev_ohm",
        coupon.max_batch_stddev_ohm,
    );
    mapping
}

fn controlled_impedance_coupon_sample_mapping(
    sample: &AppliedControlledImpedanceCouponSample,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(sample.name.clone()));
    mapping.insert("source".to_string(), Value::String(sample.source.clone()));
    mapping.insert(
        "measured_impedance_ohm".to_string(),
        serde_yaml_ng::to_value(sample.measured_impedance_ohm).unwrap_or(Value::Null),
    );
    mapping
}

fn controlled_impedance_solver_result_mapping(
    result: &AppliedControlledImpedanceSolverResult,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(result.name.clone()));
    mapping.insert("source".to_string(), Value::String(result.source.clone()));
    mapping.insert("solver".to_string(), Value::String(result.solver.clone()));
    insert_optional_string(&mut mapping, "solver_version", &result.solver_version);
    mapping.insert(
        "solver_artifact_uri".to_string(),
        Value::String(result.solver_artifact_uri.clone()),
    );
    mapping.insert(
        "solver_artifact_sha256".to_string(),
        Value::String(result.solver_artifact_sha256.clone()),
    );
    insert_optional_string(
        &mut mapping,
        "solver_artifact_signature_uri",
        &result.solver_artifact_signature_uri,
    );
    insert_optional_string(
        &mut mapping,
        "solver_artifact_signature_sha256",
        &result.solver_artifact_signature_sha256,
    );
    insert_optional_string(
        &mut mapping,
        "solver_artifact_signer",
        &result.solver_artifact_signer,
    );
    insert_optional_string(
        &mut mapping,
        "solver_output_schema",
        &result.solver_output_schema,
    );
    insert_optional_string(
        &mut mapping,
        "solver_output_schema_version",
        &result.solver_output_schema_version,
    );
    insert_optional_string(
        &mut mapping,
        "solver_output_schema_uri",
        &result.solver_output_schema_uri,
    );
    insert_optional_string(
        &mut mapping,
        "solver_output_schema_sha256",
        &result.solver_output_schema_sha256,
    );
    insert_optional_string(
        &mut mapping,
        "solver_config_lock_uri",
        &result.solver_config_lock_uri,
    );
    insert_optional_string(
        &mut mapping,
        "solver_config_lock_sha256",
        &result.solver_config_lock_sha256,
    );
    insert_optional_string(
        &mut mapping,
        "solver_config_lock_tool",
        &result.solver_config_lock_tool,
    );
    insert_optional_string(
        &mut mapping,
        "solver_config_lock_revision",
        &result.solver_config_lock_revision,
    );
    insert_optional_string(
        &mut mapping,
        "solver_runtime_allowlist",
        &result.solver_runtime_allowlist,
    );
    insert_optional_string(
        &mut mapping,
        "solver_runtime_profile",
        &result.solver_runtime_profile,
    );
    if !result.solver_runtime_options.is_empty() {
        mapping.insert(
            "solver_runtime_options".to_string(),
            serde_yaml_ng::to_value(&result.solver_runtime_options).unwrap_or(Value::Null),
        );
    }
    insert_optional_string(
        &mut mapping,
        "solver_entitlement",
        &result.solver_entitlement,
    );
    if !result.solver_entitlement_features.is_empty() {
        mapping.insert(
            "solver_entitlement_features".to_string(),
            serde_yaml_ng::to_value(&result.solver_entitlement_features).unwrap_or(Value::Null),
        );
    }
    insert_optional_string(
        &mut mapping,
        "solver_execution_environment",
        &result.solver_execution_environment,
    );
    insert_optional_string(
        &mut mapping,
        "solver_environment_fingerprint",
        &result.solver_environment_fingerprint,
    );
    if !result.solver_environment_components.is_empty() {
        mapping.insert(
            "solver_environment_components".to_string(),
            serde_yaml_ng::to_value(&result.solver_environment_components).unwrap_or(Value::Null),
        );
    }
    insert_optional_string(&mut mapping, "solver_run_log", &result.solver_run_log);
    insert_optional_string(&mut mapping, "solver_run_id", &result.solver_run_id);
    insert_optional_string(
        &mut mapping,
        "solver_random_seed",
        &result.solver_random_seed,
    );
    insert_optional_string(
        &mut mapping,
        "solver_numeric_tolerance_policy",
        &result.solver_numeric_tolerance_policy,
    );
    insert_optional_number(
        &mut mapping,
        "solver_residual_error",
        result.solver_residual_error,
    );
    if let Some(iterations) = result.solver_iterations {
        mapping.insert(
            "solver_iterations".to_string(),
            serde_yaml_ng::to_value(iterations).unwrap_or(Value::Null),
        );
    }
    insert_optional_string(
        &mut mapping,
        "solver_input_deck_uri",
        &result.solver_input_deck_uri,
    );
    insert_optional_string(
        &mut mapping,
        "solver_input_deck_sha256",
        &result.solver_input_deck_sha256,
    );
    mapping.insert(
        "result_type".to_string(),
        Value::String(result.result_type.clone()),
    );
    insert_optional_string(&mut mapping, "net", &result.net);
    insert_optional_string(&mut mapping, "first_net", &result.first_net);
    insert_optional_string(&mut mapping, "second_net", &result.second_net);
    mapping.insert(
        "target_impedance_ohm".to_string(),
        serde_yaml_ng::to_value(result.target_impedance_ohm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "solved_impedance_ohm".to_string(),
        serde_yaml_ng::to_value(result.solved_impedance_ohm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "max_impedance_error_ohm".to_string(),
        serde_yaml_ng::to_value(result.max_impedance_error_ohm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "stackup_revision".to_string(),
        Value::String(result.stackup_revision.clone()),
    );
    mapping.insert(
        "route_layer".to_string(),
        Value::String(result.route_layer.clone()),
    );
    mapping.insert(
        "reference_layer".to_string(),
        Value::String(result.reference_layer.clone()),
    );
    mapping.insert(
        "dielectric_layer".to_string(),
        Value::String(result.dielectric_layer.clone()),
    );
    mapping.insert(
        "solved_width_mm".to_string(),
        serde_yaml_ng::to_value(result.solved_width_mm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "max_route_width_delta_mm".to_string(),
        serde_yaml_ng::to_value(result.max_route_width_delta_mm).unwrap_or(Value::Null),
    );
    insert_optional_number(&mut mapping, "solved_gap_mm", result.solved_gap_mm);
    insert_optional_number(
        &mut mapping,
        "max_route_gap_delta_mm",
        result.max_route_gap_delta_mm,
    );
    insert_optional_string(
        &mut mapping,
        "input_stackup_revision",
        &result.input_stackup_revision,
    );
    insert_optional_string(&mut mapping, "input_route_layer", &result.input_route_layer);
    insert_optional_string(
        &mut mapping,
        "input_reference_layer",
        &result.input_reference_layer,
    );
    insert_optional_string(
        &mut mapping,
        "input_dielectric_layer",
        &result.input_dielectric_layer,
    );
    insert_optional_number(&mut mapping, "input_width_mm", result.input_width_mm);
    insert_optional_number(&mut mapping, "input_gap_mm", result.input_gap_mm);
    insert_optional_number(&mut mapping, "frequency_mhz", result.frequency_mhz);
    insert_optional_number(
        &mut mapping,
        "input_frequency_mhz",
        result.input_frequency_mhz,
    );
    insert_optional_string(
        &mut mapping,
        "copper_roughness_model",
        &result.copper_roughness_model,
    );
    insert_optional_number(
        &mut mapping,
        "copper_roughness_um",
        result.copper_roughness_um,
    );
    insert_optional_string(
        &mut mapping,
        "input_copper_roughness_model",
        &result.input_copper_roughness_model,
    );
    insert_optional_number(
        &mut mapping,
        "input_copper_roughness_um",
        result.input_copper_roughness_um,
    );
    insert_optional_string(
        &mut mapping,
        "etch_compensation_model",
        &result.etch_compensation_model,
    );
    insert_optional_number(
        &mut mapping,
        "etch_compensation_um",
        result.etch_compensation_um,
    );
    insert_optional_string(
        &mut mapping,
        "input_etch_compensation_model",
        &result.input_etch_compensation_model,
    );
    insert_optional_number(
        &mut mapping,
        "input_etch_compensation_um",
        result.input_etch_compensation_um,
    );
    insert_optional_string(
        &mut mapping,
        "solver_material_library",
        &result.solver_material_library,
    );
    insert_optional_string(
        &mut mapping,
        "solver_material_library_revision",
        &result.solver_material_library_revision,
    );
    insert_optional_string(
        &mut mapping,
        "solver_material_library_artifact_uri",
        &result.solver_material_library_artifact_uri,
    );
    insert_optional_string(
        &mut mapping,
        "solver_material_library_artifact_sha256",
        &result.solver_material_library_artifact_sha256,
    );
    insert_optional_string(
        &mut mapping,
        "input_material_library",
        &result.input_material_library,
    );
    insert_optional_string(
        &mut mapping,
        "input_material_library_revision",
        &result.input_material_library_revision,
    );
    insert_optional_string(
        &mut mapping,
        "stackup_signoff_source",
        &result.stackup_signoff_source,
    );
    insert_optional_string(
        &mut mapping,
        "fabricator_stackup_revision",
        &result.fabricator_stackup_revision,
    );
    insert_optional_string(
        &mut mapping,
        "stackup_signoff_artifact_uri",
        &result.stackup_signoff_artifact_uri,
    );
    insert_optional_string(
        &mut mapping,
        "stackup_signoff_artifact_sha256",
        &result.stackup_signoff_artifact_sha256,
    );
    insert_optional_number(
        &mut mapping,
        "min_solver_sample_count",
        result.min_solver_sample_count,
    );
    insert_optional_number(
        &mut mapping,
        "max_solver_frequency_step_mhz",
        result.max_solver_frequency_step_mhz,
    );
    if !result.required_solver_corners.is_empty() {
        mapping.insert(
            "required_solver_corners".to_string(),
            serde_yaml_ng::to_value(&result.required_solver_corners).unwrap_or(Value::Null),
        );
    }
    mapping
}

fn controlled_impedance_solver_sample_mapping(
    sample: &AppliedControlledImpedanceSolverSample,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(sample.name.clone()));
    mapping.insert("source".to_string(), Value::String(sample.source.clone()));
    mapping.insert("corner".to_string(), Value::String(sample.corner.clone()));
    mapping.insert(
        "frequency_mhz".to_string(),
        serde_yaml_ng::to_value(sample.frequency_mhz).unwrap_or(Value::Null),
    );
    mapping.insert(
        "solved_impedance_ohm".to_string(),
        serde_yaml_ng::to_value(sample.solved_impedance_ohm).unwrap_or(Value::Null),
    );
    mapping
}

fn controlled_impedance_solver_material_corner_mapping(
    corner: &AppliedControlledImpedanceSolverMaterialCorner,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(corner.name.clone()));
    mapping.insert("source".to_string(), Value::String(corner.source.clone()));
    mapping.insert("corner".to_string(), Value::String(corner.corner.clone()));
    mapping.insert(
        "dielectric_layer".to_string(),
        Value::String(corner.dielectric_layer.clone()),
    );
    mapping.insert(
        "material".to_string(),
        Value::String(corner.material.clone()),
    );
    mapping.insert(
        "dielectric_constant".to_string(),
        serde_yaml_ng::to_value(corner.dielectric_constant).unwrap_or(Value::Null),
    );
    mapping.insert(
        "nominal_dielectric_constant".to_string(),
        serde_yaml_ng::to_value(corner.nominal_dielectric_constant).unwrap_or(Value::Null),
    );
    mapping.insert(
        "material_library".to_string(),
        Value::String(corner.material_library.clone()),
    );
    mapping.insert(
        "material_library_revision".to_string(),
        Value::String(corner.material_library_revision.clone()),
    );
    mapping
}

fn controlled_impedance_solver_qualification_mapping(
    qualification: &AppliedControlledImpedanceSolverQualification,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert(
        "name".to_string(),
        Value::String(qualification.name.clone()),
    );
    mapping.insert(
        "source".to_string(),
        Value::String(qualification.source.clone()),
    );
    mapping.insert(
        "solver".to_string(),
        Value::String(qualification.solver.clone()),
    );
    mapping.insert(
        "solver_version".to_string(),
        Value::String(qualification.solver_version.clone()),
    );
    mapping.insert(
        "qualification_artifact_uri".to_string(),
        Value::String(qualification.qualification_artifact_uri.clone()),
    );
    mapping.insert(
        "qualification_artifact_sha256".to_string(),
        Value::String(qualification.qualification_artifact_sha256.clone()),
    );
    mapping
}

fn controlled_impedance_solver_material_library_mapping(
    library: &AppliedControlledImpedanceSolverMaterialLibrary,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(library.name.clone()));
    mapping.insert("source".to_string(), Value::String(library.source.clone()));
    mapping.insert(
        "material_library".to_string(),
        Value::String(library.material_library.clone()),
    );
    mapping.insert(
        "material_library_revision".to_string(),
        Value::String(library.material_library_revision.clone()),
    );
    mapping.insert(
        "artifact_uri".to_string(),
        Value::String(library.artifact_uri.clone()),
    );
    mapping.insert(
        "artifact_sha256".to_string(),
        Value::String(library.artifact_sha256.clone()),
    );
    mapping.insert(
        "corners".to_string(),
        serde_yaml_ng::to_value(&library.corners).unwrap_or(Value::Null),
    );
    mapping.insert(
        "dielectric_layers".to_string(),
        serde_yaml_ng::to_value(&library.dielectric_layers).unwrap_or(Value::Null),
    );
    mapping.insert(
        "materials".to_string(),
        serde_yaml_ng::to_value(&library.materials).unwrap_or(Value::Null),
    );
    mapping.insert(
        "content_fields".to_string(),
        serde_yaml_ng::to_value(&library.content_fields).unwrap_or(Value::Null),
    );
    mapping
}

fn controlled_impedance_solver_material_acceptance_mapping(
    acceptance: &AppliedControlledImpedanceSolverMaterialAcceptance,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(acceptance.name.clone()));
    mapping.insert(
        "source".to_string(),
        Value::String(acceptance.source.clone()),
    );
    mapping.insert(
        "material_library".to_string(),
        Value::String(acceptance.material_library.clone()),
    );
    mapping.insert(
        "material_library_revision".to_string(),
        Value::String(acceptance.material_library_revision.clone()),
    );
    mapping.insert(
        "fabricator_stackup_revision".to_string(),
        Value::String(acceptance.fabricator_stackup_revision.clone()),
    );
    mapping.insert(
        "acceptance_artifact_uri".to_string(),
        Value::String(acceptance.acceptance_artifact_uri.clone()),
    );
    mapping.insert(
        "acceptance_artifact_sha256".to_string(),
        Value::String(acceptance.acceptance_artifact_sha256.clone()),
    );
    insert_optional_string(&mut mapping, "accepted_by", &acceptance.accepted_by);
    mapping.insert(
        "accepted_corners".to_string(),
        serde_yaml_ng::to_value(&acceptance.accepted_corners).unwrap_or(Value::Null),
    );
    mapping.insert(
        "accepted_dielectric_layers".to_string(),
        serde_yaml_ng::to_value(&acceptance.accepted_dielectric_layers).unwrap_or(Value::Null),
    );
    mapping.insert(
        "accepted_materials".to_string(),
        serde_yaml_ng::to_value(&acceptance.accepted_materials).unwrap_or(Value::Null),
    );
    mapping
}

fn controlled_impedance_solver_runtime_allowlist_mapping(
    allowlist: &AppliedControlledImpedanceSolverRuntimeAllowlist,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(allowlist.name.clone()));
    mapping.insert(
        "source".to_string(),
        Value::String(allowlist.source.clone()),
    );
    mapping.insert(
        "solver".to_string(),
        Value::String(allowlist.solver.clone()),
    );
    mapping.insert(
        "solver_config_lock_revision".to_string(),
        Value::String(allowlist.solver_config_lock_revision.clone()),
    );
    mapping.insert(
        "runtime_profile".to_string(),
        Value::String(allowlist.runtime_profile.clone()),
    );
    mapping.insert(
        "allowlist_revision".to_string(),
        Value::String(allowlist.allowlist_revision.clone()),
    );
    mapping.insert(
        "artifact_uri".to_string(),
        Value::String(allowlist.artifact_uri.clone()),
    );
    mapping.insert(
        "artifact_sha256".to_string(),
        Value::String(allowlist.artifact_sha256.clone()),
    );
    mapping.insert(
        "allowed_options".to_string(),
        serde_yaml_ng::to_value(&allowlist.allowed_options).unwrap_or(Value::Null),
    );
    mapping
}

fn controlled_impedance_solver_entitlement_mapping(
    entitlement: &AppliedControlledImpedanceSolverEntitlement,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(entitlement.name.clone()));
    mapping.insert(
        "source".to_string(),
        Value::String(entitlement.source.clone()),
    );
    mapping.insert(
        "solver".to_string(),
        Value::String(entitlement.solver.clone()),
    );
    mapping.insert(
        "solver_version".to_string(),
        Value::String(entitlement.solver_version.clone()),
    );
    mapping.insert(
        "entitlement_id".to_string(),
        Value::String(entitlement.entitlement_id.clone()),
    );
    mapping.insert(
        "entitlement_revision".to_string(),
        Value::String(entitlement.entitlement_revision.clone()),
    );
    mapping.insert(
        "artifact_uri".to_string(),
        Value::String(entitlement.artifact_uri.clone()),
    );
    mapping.insert(
        "artifact_sha256".to_string(),
        Value::String(entitlement.artifact_sha256.clone()),
    );
    mapping.insert(
        "licensed_features".to_string(),
        serde_yaml_ng::to_value(&entitlement.licensed_features).unwrap_or(Value::Null),
    );
    mapping
}

fn controlled_impedance_solver_execution_environment_mapping(
    environment: &AppliedControlledImpedanceSolverExecutionEnvironment,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(environment.name.clone()));
    mapping.insert(
        "source".to_string(),
        Value::String(environment.source.clone()),
    );
    mapping.insert(
        "solver".to_string(),
        Value::String(environment.solver.clone()),
    );
    mapping.insert(
        "solver_version".to_string(),
        Value::String(environment.solver_version.clone()),
    );
    mapping.insert(
        "environment_id".to_string(),
        Value::String(environment.environment_id.clone()),
    );
    mapping.insert(
        "environment_revision".to_string(),
        Value::String(environment.environment_revision.clone()),
    );
    mapping.insert(
        "artifact_uri".to_string(),
        Value::String(environment.artifact_uri.clone()),
    );
    mapping.insert(
        "artifact_sha256".to_string(),
        Value::String(environment.artifact_sha256.clone()),
    );
    mapping.insert(
        "reproducibility_fingerprint".to_string(),
        Value::String(environment.reproducibility_fingerprint.clone()),
    );
    mapping.insert(
        "locked_components".to_string(),
        serde_yaml_ng::to_value(&environment.locked_components).unwrap_or(Value::Null),
    );
    mapping
}

fn controlled_impedance_solver_run_log_mapping(
    run_log: &AppliedControlledImpedanceSolverRunLog,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(run_log.name.clone()));
    mapping.insert("source".to_string(), Value::String(run_log.source.clone()));
    mapping.insert("solver".to_string(), Value::String(run_log.solver.clone()));
    mapping.insert(
        "solver_version".to_string(),
        Value::String(run_log.solver_version.clone()),
    );
    mapping.insert("run_id".to_string(), Value::String(run_log.run_id.clone()));
    mapping.insert(
        "artifact_uri".to_string(),
        Value::String(run_log.artifact_uri.clone()),
    );
    mapping.insert(
        "artifact_sha256".to_string(),
        Value::String(run_log.artifact_sha256.clone()),
    );
    mapping.insert(
        "random_seed".to_string(),
        Value::String(run_log.random_seed.clone()),
    );
    mapping.insert(
        "numeric_tolerance_policy".to_string(),
        Value::String(run_log.numeric_tolerance_policy.clone()),
    );
    mapping.insert(
        "max_residual_error".to_string(),
        serde_yaml_ng::to_value(run_log.max_residual_error).unwrap_or(Value::Null),
    );
    mapping.insert(
        "max_iterations".to_string(),
        serde_yaml_ng::to_value(run_log.max_iterations).unwrap_or(Value::Null),
    );
    insert_optional_number(&mut mapping, "min_rerun_count", run_log.min_rerun_count);
    insert_optional_number(
        &mut mapping,
        "max_rerun_impedance_delta_ohm",
        run_log.max_rerun_impedance_delta_ohm,
    );
    insert_optional_number(
        &mut mapping,
        "min_convergence_sample_count",
        run_log.min_convergence_sample_count,
    );
    insert_optional_number(
        &mut mapping,
        "max_convergence_impedance_delta_ohm",
        run_log.max_convergence_impedance_delta_ohm,
    );
    insert_optional_string(
        &mut mapping,
        "required_stopping_criteria",
        &run_log.required_stopping_criteria,
    );
    insert_optional_bool(
        &mut mapping,
        "require_monotonic_residual_decrease",
        run_log.require_monotonic_residual_decrease,
    );
    mapping
}

fn controlled_impedance_solver_rerun_mapping(
    rerun: &AppliedControlledImpedanceSolverRerun,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(rerun.name.clone()));
    mapping.insert("source".to_string(), Value::String(rerun.source.clone()));
    mapping.insert("run_id".to_string(), Value::String(rerun.run_id.clone()));
    mapping.insert(
        "artifact_uri".to_string(),
        Value::String(rerun.artifact_uri.clone()),
    );
    mapping.insert(
        "artifact_sha256".to_string(),
        Value::String(rerun.artifact_sha256.clone()),
    );
    mapping.insert(
        "random_seed".to_string(),
        Value::String(rerun.random_seed.clone()),
    );
    mapping.insert(
        "solved_impedance_ohm".to_string(),
        serde_yaml_ng::to_value(rerun.solved_impedance_ohm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "residual_error".to_string(),
        serde_yaml_ng::to_value(rerun.residual_error).unwrap_or(Value::Null),
    );
    mapping.insert(
        "iterations".to_string(),
        serde_yaml_ng::to_value(rerun.iterations).unwrap_or(Value::Null),
    );
    mapping
}

fn controlled_impedance_solver_convergence_sample_mapping(
    sample: &AppliedControlledImpedanceSolverConvergenceSample,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(sample.name.clone()));
    mapping.insert("source".to_string(), Value::String(sample.source.clone()));
    mapping.insert(
        "artifact_uri".to_string(),
        Value::String(sample.artifact_uri.clone()),
    );
    mapping.insert(
        "artifact_sha256".to_string(),
        Value::String(sample.artifact_sha256.clone()),
    );
    mapping.insert(
        "iteration".to_string(),
        serde_yaml_ng::to_value(sample.iteration).unwrap_or(Value::Null),
    );
    mapping.insert(
        "solved_impedance_ohm".to_string(),
        serde_yaml_ng::to_value(sample.solved_impedance_ohm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "residual_error".to_string(),
        serde_yaml_ng::to_value(sample.residual_error).unwrap_or(Value::Null),
    );
    mapping.insert(
        "stopping_criteria".to_string(),
        Value::String(sample.stopping_criteria.clone()),
    );
    mapping
}

fn controlled_impedance_solver_material_process_mapping(
    process: &AppliedControlledImpedanceSolverMaterialProcess,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(process.name.clone()));
    mapping.insert("source".to_string(), Value::String(process.source.clone()));
    mapping.insert(
        "material_library".to_string(),
        Value::String(process.material_library.clone()),
    );
    mapping.insert(
        "material_library_revision".to_string(),
        Value::String(process.material_library_revision.clone()),
    );
    mapping.insert(
        "fabricator_stackup_revision".to_string(),
        Value::String(process.fabricator_stackup_revision.clone()),
    );
    mapping.insert(
        "dielectric_layer".to_string(),
        Value::String(process.dielectric_layer.clone()),
    );
    mapping.insert(
        "material".to_string(),
        Value::String(process.material.clone()),
    );
    mapping.insert(
        "process_lot".to_string(),
        Value::String(process.process_lot.clone()),
    );
    mapping.insert(
        "material_lot".to_string(),
        Value::String(process.material_lot.clone()),
    );
    mapping.insert(
        "process_revision".to_string(),
        Value::String(process.process_revision.clone()),
    );
    mapping.insert(
        "drift_artifact_uri".to_string(),
        Value::String(process.drift_artifact_uri.clone()),
    );
    mapping.insert(
        "drift_artifact_sha256".to_string(),
        Value::String(process.drift_artifact_sha256.clone()),
    );
    mapping.insert(
        "accepted_dielectric_constant".to_string(),
        serde_yaml_ng::to_value(process.accepted_dielectric_constant).unwrap_or(Value::Null),
    );
    mapping.insert(
        "measured_dielectric_constant".to_string(),
        serde_yaml_ng::to_value(process.measured_dielectric_constant).unwrap_or(Value::Null),
    );
    mapping.insert(
        "max_dielectric_constant_delta".to_string(),
        serde_yaml_ng::to_value(process.max_dielectric_constant_delta).unwrap_or(Value::Null),
    );
    mapping.insert(
        "accepted_thickness_mm".to_string(),
        serde_yaml_ng::to_value(process.accepted_thickness_mm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "measured_thickness_mm".to_string(),
        serde_yaml_ng::to_value(process.measured_thickness_mm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "max_thickness_delta_mm".to_string(),
        serde_yaml_ng::to_value(process.max_thickness_delta_mm).unwrap_or(Value::Null),
    );
    mapping
}

fn thermal_copper_mapping(rule: &AppliedThermalCopper) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(rule.name.clone()));
    mapping.insert(
        "component".to_string(),
        Value::String(rule.component.clone()),
    );
    mapping.insert("source".to_string(), Value::String(rule.source.clone()));
    mapping.insert(
        "power_loss_w".to_string(),
        serde_yaml_ng::to_value(rule.power_loss_w).unwrap_or(Value::Null),
    );
    mapping.insert(
        "min_copper_area_mm2".to_string(),
        serde_yaml_ng::to_value(rule.min_copper_area_mm2).unwrap_or(Value::Null),
    );
    insert_optional_number(
        &mut mapping,
        "min_thermal_via_count",
        rule.min_thermal_via_count,
    );
    insert_optional_number(
        &mut mapping,
        "min_plated_thermal_via_count",
        rule.min_plated_thermal_via_count,
    );
    insert_optional_number(
        &mut mapping,
        "min_thermal_via_drill_mm",
        rule.min_thermal_via_drill_mm,
    );
    insert_optional_number(
        &mut mapping,
        "min_thermal_via_plating_thickness_um",
        rule.min_thermal_via_plating_thickness_um,
    );
    insert_optional_number(
        &mut mapping,
        "min_total_thermal_via_barrel_cross_section_mm2",
        rule.min_total_thermal_via_barrel_cross_section_mm2,
    );
    insert_optional_number(
        &mut mapping,
        "min_copper_thickness_um",
        rule.min_copper_thickness_um,
    );
    insert_optional_number(
        &mut mapping,
        "rated_ambient_temperature_C",
        rule.rated_ambient_temperature_c,
    );
    insert_optional_number(&mut mapping, "min_airflow_lfm", rule.min_airflow_lfm);
    if let Some(value) = &rule.enclosure_profile {
        mapping.insert(
            "enclosure_profile".to_string(),
            Value::String(value.clone()),
        );
    }
    insert_string_sequence(&mut mapping, "nets", &rule.nets);
    insert_string_sequence(&mut mapping, "layers", &rule.layers);
    mapping
}

fn insert_optional_number<T: Serialize>(
    mapping: &mut BTreeMap<String, Value>,
    key: &str,
    value: Option<T>,
) {
    if let Some(value) = value {
        mapping.insert(
            key.to_string(),
            serde_yaml_ng::to_value(value).unwrap_or(Value::Null),
        );
    }
}

fn insert_optional_bool(mapping: &mut BTreeMap<String, Value>, key: &str, value: Option<bool>) {
    if let Some(value) = value {
        mapping.insert(key.to_string(), Value::Bool(value));
    }
}

fn insert_string_sequence(mapping: &mut BTreeMap<String, Value>, key: &str, values: &[String]) {
    if !values.is_empty() {
        mapping.insert(
            key.to_string(),
            Value::Sequence(values.iter().cloned().map(Value::String).collect()),
        );
    }
}

fn thermal_measurement_mapping(measurement: &AppliedThermalMeasurement) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(measurement.name.clone()));
    mapping.insert(
        "component".to_string(),
        Value::String(measurement.component.clone()),
    );
    mapping.insert(
        "source".to_string(),
        Value::String(measurement.source.clone()),
    );
    mapping.insert(
        "measured_temperature_C".to_string(),
        serde_yaml_ng::to_value(measurement.measured_temperature_c).unwrap_or(Value::Null),
    );
    if let Some(value) = measurement.ambient_temperature_c {
        mapping.insert(
            "ambient_temperature_C".to_string(),
            serde_yaml_ng::to_value(value).unwrap_or(Value::Null),
        );
    }
    if let Some(value) = measurement.measurement_uncertainty_c {
        mapping.insert(
            "measurement_uncertainty_C".to_string(),
            serde_yaml_ng::to_value(value).unwrap_or(Value::Null),
        );
    }
    if let Some(value) = measurement.power_loss_w {
        mapping.insert(
            "power_loss_w".to_string(),
            serde_yaml_ng::to_value(value).unwrap_or(Value::Null),
        );
    }
    if let Some(value) = &measurement.measurement_point {
        mapping.insert(
            "measurement_point".to_string(),
            Value::String(value.clone()),
        );
    }
    if let Some(value) = &measurement.notes {
        mapping.insert("notes".to_string(), Value::String(value.clone()));
    }
    mapping
}

fn thermal_package_mapping(package: &AppliedThermalPackage) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert(
        "component".to_string(),
        Value::String(package.component.clone()),
    );
    mapping.insert("source".to_string(), Value::String(package.source.clone()));
    mapping.insert(
        "thermal_resistance_junction_to_ambient_C_per_W".to_string(),
        serde_yaml_ng::to_value(package.thermal_resistance_junction_to_ambient_c_per_w)
            .unwrap_or(Value::Null),
    );
    mapping.insert(
        "max_junction_temperature_C".to_string(),
        serde_yaml_ng::to_value(package.max_junction_temperature_c).unwrap_or(Value::Null),
    );
    mapping
}

fn thermal_environment_mapping(environment: &AppliedThermalEnvironment) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(environment.name.clone()));
    mapping.insert(
        "source".to_string(),
        Value::String(environment.source.clone()),
    );
    mapping.insert(
        "ambient_temperature_C".to_string(),
        serde_yaml_ng::to_value(environment.ambient_temperature_c).unwrap_or(Value::Null),
    );
    insert_optional_number(&mut mapping, "airflow_lfm", environment.airflow_lfm);
    if let Some(value) = &environment.enclosure_profile {
        mapping.insert(
            "enclosure_profile".to_string(),
            Value::String(value.clone()),
        );
    }
    mapping
}

fn thermal_limit_mapping(limit: &AppliedThermalLimit) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(limit.name.clone()));
    mapping.insert("source".to_string(), Value::String(limit.source.clone()));
    if let Some(component) = &limit.component {
        mapping.insert("component".to_string(), Value::String(component.clone()));
    }
    mapping.insert(
        "max_measured_temperature_C".to_string(),
        serde_yaml_ng::to_value(limit.max_measured_temperature_c).unwrap_or(Value::Null),
    );
    insert_optional_number(
        &mut mapping,
        "max_temperature_rise_C",
        limit.max_temperature_rise_c,
    );
    insert_optional_number(
        &mut mapping,
        "max_junction_temperature_margin_C",
        limit.max_junction_temperature_margin_c,
    );
    mapping
}

fn stackup_layer_mapping(layer: &AppliedStackupLayer) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(layer.name.clone()));
    mapping.insert("kind".to_string(), Value::String(layer.kind.clone()));
    if let Some(value) = &layer.reference_net {
        mapping.insert("reference_net".to_string(), Value::String(value.clone()));
    }
    insert_optional_number(&mut mapping, "thickness_mm", layer.thickness_mm);
    insert_optional_number(
        &mut mapping,
        "copper_thickness_um",
        layer.copper_thickness_um,
    );
    insert_optional_number(
        &mut mapping,
        "dielectric_constant",
        layer.dielectric_constant,
    );
    if let Some(value) = &layer.material {
        mapping.insert("material".to_string(), Value::String(value.clone()));
    }
    mapping.insert("source".to_string(), Value::String(layer.source.clone()));
    mapping
}

fn rf_antenna_keepout_mapping(keepout: &AppliedRfAntennaKeepout) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(keepout.name.clone()));
    if let Some(value) = &keepout.antenna_net {
        mapping.insert("antenna_net".to_string(), Value::String(value.clone()));
    }
    mapping.insert("layer".to_string(), Value::String(keepout.layer.clone()));
    mapping.insert(
        "polygon".to_string(),
        Value::Sequence(
            keepout
                .polygon
                .iter()
                .map(layout_point_value)
                .collect::<Vec<_>>(),
        ),
    );
    mapping.insert(
        "min_copper_clearance_mm".to_string(),
        serde_yaml_ng::to_value(keepout.min_copper_clearance_mm).unwrap_or(Value::Null),
    );
    mapping.insert("source".to_string(), Value::String(keepout.source.clone()));
    mapping
}

fn rf_antenna_feed_path_mapping(feed_path: &AppliedRfAntennaFeedPath) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(feed_path.name.clone()));
    mapping.insert(
        "antenna_net".to_string(),
        Value::String(feed_path.antenna_net.clone()),
    );
    mapping.insert(
        "feed_component".to_string(),
        Value::String(feed_path.feed_component.clone()),
    );
    mapping.insert(
        "feed_pin".to_string(),
        Value::String(feed_path.feed_pin.clone()),
    );
    insert_string_sequence(
        &mut mapping,
        "matching_components",
        &feed_path.matching_components,
    );
    mapping.insert(
        "max_feed_route_length_mm".to_string(),
        serde_yaml_ng::to_value(feed_path.max_feed_route_length_mm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "max_matching_component_distance_mm".to_string(),
        serde_yaml_ng::to_value(feed_path.max_matching_component_distance_mm)
            .unwrap_or(Value::Null),
    );
    mapping.insert(
        "source".to_string(),
        Value::String(feed_path.source.clone()),
    );
    mapping
}

fn rf_antenna_matching_network_mapping(
    network: &AppliedRfAntennaMatchingNetwork,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(network.name.clone()));
    mapping.insert(
        "antenna_net".to_string(),
        Value::String(network.antenna_net.clone()),
    );
    mapping.insert(
        "topology".to_string(),
        Value::String(network.topology.clone()),
    );
    if let Some(reference_net) = &network.reference_net {
        mapping.insert(
            "reference_net".to_string(),
            Value::String(reference_net.clone()),
        );
    }
    mapping.insert("source".to_string(), Value::String(network.source.clone()));
    mapping.insert(
        "elements".to_string(),
        Value::Sequence(
            network
                .elements
                .iter()
                .map(rf_antenna_matching_element_value)
                .collect::<Vec<_>>(),
        ),
    );
    mapping
}

fn rf_antenna_matching_element_value(element: &AppliedRfAntennaMatchingElement) -> Value {
    let mut mapping = BTreeMap::new();
    mapping.insert(
        "component".to_string(),
        Value::String(element.component.clone()),
    );
    mapping.insert("role".to_string(), Value::String(element.role.clone()));
    if let Some(value) = &element.input_net {
        mapping.insert("input_net".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &element.output_net {
        mapping.insert("output_net".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &element.signal_net {
        mapping.insert("signal_net".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &element.reference_net {
        mapping.insert("reference_net".to_string(), Value::String(value.clone()));
    }
    serde_yaml_ng::to_value(mapping).unwrap_or(Value::Null)
}

fn rf_antenna_measurement_mapping(
    measurement: &AppliedRfAntennaMeasurement,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(measurement.name.clone()));
    mapping.insert(
        "antenna_net".to_string(),
        Value::String(measurement.antenna_net.clone()),
    );
    mapping.insert(
        "frequency_mhz".to_string(),
        serde_yaml_ng::to_value(measurement.frequency_mhz).unwrap_or(Value::Null),
    );
    mapping.insert(
        "return_loss_db".to_string(),
        serde_yaml_ng::to_value(measurement.return_loss_db).unwrap_or(Value::Null),
    );
    mapping.insert(
        "source".to_string(),
        Value::String(measurement.source.clone()),
    );
    if let Some(value) = &measurement.measurement_method {
        mapping.insert(
            "measurement_method".to_string(),
            Value::String(value.clone()),
        );
    }
    if let Some(value) = &measurement.measurement_condition {
        mapping.insert(
            "measurement_condition".to_string(),
            Value::String(value.clone()),
        );
    }
    if let Some(value) = &measurement.notes {
        mapping.insert("notes".to_string(), Value::String(value.clone()));
    }
    mapping
}

fn rf_antenna_performance_limit_mapping(
    limit: &AppliedRfAntennaPerformanceLimit,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(limit.name.clone()));
    mapping.insert(
        "antenna_net".to_string(),
        Value::String(limit.antenna_net.clone()),
    );
    mapping.insert(
        "min_return_loss_db".to_string(),
        serde_yaml_ng::to_value(limit.min_return_loss_db).unwrap_or(Value::Null),
    );
    mapping.insert("source".to_string(), Value::String(limit.source.clone()));
    if let Some(value) = limit.frequency_min_mhz {
        mapping.insert(
            "frequency_min_mhz".to_string(),
            serde_yaml_ng::to_value(value).unwrap_or(Value::Null),
        );
    }
    if let Some(value) = limit.frequency_max_mhz {
        mapping.insert(
            "frequency_max_mhz".to_string(),
            serde_yaml_ng::to_value(value).unwrap_or(Value::Null),
        );
    }
    if let Some(value) = limit.min_measurement_count {
        mapping.insert(
            "min_measurement_count".to_string(),
            serde_yaml_ng::to_value(value).unwrap_or(Value::Null),
        );
    }
    if let Some(value) = limit.max_frequency_step_mhz {
        mapping.insert(
            "max_frequency_step_mhz".to_string(),
            serde_yaml_ng::to_value(value).unwrap_or(Value::Null),
        );
    }
    if let Some(value) = &limit.required_measurement_condition {
        mapping.insert(
            "required_measurement_condition".to_string(),
            Value::String(value.clone()),
        );
    }
    if let Some(value) = &limit.notes {
        mapping.insert("notes".to_string(), Value::String(value.clone()));
    }
    mapping
}

fn rf_antenna_measurement_condition_mapping(
    condition: &AppliedRfAntennaMeasurementCondition,
) -> BTreeMap<String, Value> {
    let mut mapping = BTreeMap::new();
    mapping.insert("name".to_string(), Value::String(condition.name.clone()));
    mapping.insert(
        "source".to_string(),
        Value::String(condition.source.clone()),
    );
    if let Some(value) = &condition.fixture {
        mapping.insert("fixture".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &condition.cable_setup {
        mapping.insert("cable_setup".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &condition.enclosure_profile {
        mapping.insert(
            "enclosure_profile".to_string(),
            Value::String(value.clone()),
        );
    }
    if let Some(value) = &condition.notes {
        mapping.insert("notes".to_string(), Value::String(value.clone()));
    }
    mapping
}

fn insert_optional_string(
    mapping: &mut BTreeMap<String, Value>,
    key: &str,
    value: &Option<String>,
) {
    if let Some(value) = value {
        mapping.insert(key.to_string(), Value::String(value.clone()));
    }
}

fn layout_point_value(point: &AppliedLayoutPoint) -> Value {
    let mut mapping = BTreeMap::new();
    mapping.insert(
        "x_mm".to_string(),
        serde_yaml_ng::to_value(point.x_mm).unwrap_or(Value::Null),
    );
    mapping.insert(
        "y_mm".to_string(),
        serde_yaml_ng::to_value(point.y_mm).unwrap_or(Value::Null),
    );
    serde_yaml_ng::to_value(mapping).unwrap_or(Value::Null)
}
