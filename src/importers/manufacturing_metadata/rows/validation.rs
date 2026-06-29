use super::{AppliedField, ManufacturingField};
use anyhow::{Result, bail};
use std::collections::BTreeSet;

pub(super) fn validate_applied_fields(fields: &[AppliedField]) -> Result<()> {
    let min = fields
        .iter()
        .find(|field| field.field == ManufacturingField::MinPasteAreaRatio)
        .and_then(|field| field.numeric_value);
    let max = fields
        .iter()
        .find(|field| field.field == ManufacturingField::MaxPasteAreaRatio)
        .and_then(|field| field.numeric_value);
    if let (Some(min), Some(max)) = (min, max)
        && max < min
    {
        bail!("max_paste_area_ratio must be greater than or equal to min_paste_area_ratio.");
    }
    let mut controlled_impedance_nets = BTreeSet::new();
    for target in fields
        .iter()
        .filter_map(|field| field.controlled_impedance_net.as_ref())
    {
        if !controlled_impedance_nets.insert(target.net.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats controlled_impedance_net target for net {}.",
                target.net
            );
        }
    }
    let mut controlled_impedance_pairs = BTreeSet::new();
    for target in fields
        .iter()
        .filter_map(|field| field.controlled_impedance_pair.as_ref())
    {
        let key = ordered_pair_key(&target.first_net, &target.second_net);
        if !controlled_impedance_pairs.insert(key.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats controlled_impedance_pair target for pair {}.",
                key
            );
        }
    }
    let mut controlled_impedance_coupons = BTreeSet::new();
    for coupon in fields
        .iter()
        .filter_map(|field| field.controlled_impedance_coupon.as_ref())
    {
        if !controlled_impedance_coupons.insert(coupon.name.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats controlled_impedance_coupon row name {}.",
                coupon.name
            );
        }
    }
    let mut controlled_impedance_coupon_samples = BTreeSet::new();
    for sample in fields
        .iter()
        .filter_map(|field| field.controlled_impedance_coupon_sample.as_ref())
    {
        let key = format!("{}/{}", sample.coupon_name, sample.name);
        if !controlled_impedance_coupon_samples.insert(key.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats controlled_impedance_coupon_sample row {}.",
                key
            );
        }
    }
    let mut controlled_impedance_solver_results = BTreeSet::new();
    for result in fields
        .iter()
        .filter_map(|field| field.controlled_impedance_solver_result.as_ref())
    {
        if !controlled_impedance_solver_results.insert(result.name.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats controlled_impedance_solver_result row name {}.",
                result.name
            );
        }
    }
    let mut controlled_impedance_solver_samples = BTreeSet::new();
    for sample in fields
        .iter()
        .filter_map(|field| field.controlled_impedance_solver_sample.as_ref())
    {
        let key = format!("{}/{}", sample.solver_result_name, sample.name);
        if !controlled_impedance_solver_samples.insert(key.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats controlled_impedance_solver_sample row {}.",
                key
            );
        }
    }
    let mut controlled_impedance_solver_material_corners = BTreeSet::new();
    for corner in fields
        .iter()
        .filter_map(|field| field.controlled_impedance_solver_material_corner.as_ref())
    {
        let key = format!("{}/{}", corner.solver_result_name, corner.name);
        if !controlled_impedance_solver_material_corners.insert(key.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats controlled_impedance_solver_material_corner row {}.",
                key
            );
        }
    }
    let mut controlled_impedance_solver_qualifications = BTreeSet::new();
    for qualification in fields
        .iter()
        .filter_map(|field| field.controlled_impedance_solver_qualification.as_ref())
    {
        if !controlled_impedance_solver_qualifications.insert(qualification.name.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats controlled_impedance_solver_qualification row name {}.",
                qualification.name
            );
        }
    }
    let mut controlled_impedance_solver_material_libraries = BTreeSet::new();
    for library in fields
        .iter()
        .filter_map(|field| field.controlled_impedance_solver_material_library.as_ref())
    {
        if !controlled_impedance_solver_material_libraries.insert(library.name.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats controlled_impedance_solver_material_library row name {}.",
                library.name
            );
        }
    }
    let mut controlled_impedance_solver_material_acceptances = BTreeSet::new();
    for acceptance in fields.iter().filter_map(|field| {
        field
            .controlled_impedance_solver_material_acceptance
            .as_ref()
    }) {
        if !controlled_impedance_solver_material_acceptances.insert(acceptance.name.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats controlled_impedance_solver_material_acceptance row name {}.",
                acceptance.name
            );
        }
    }
    let mut controlled_impedance_solver_material_processes = BTreeSet::new();
    for process in fields
        .iter()
        .filter_map(|field| field.controlled_impedance_solver_material_process.as_ref())
    {
        if !controlled_impedance_solver_material_processes.insert(process.name.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats controlled_impedance_solver_material_process row name {}.",
                process.name
            );
        }
    }
    let mut controlled_impedance_solver_runtime_allowlists = BTreeSet::new();
    for allowlist in fields
        .iter()
        .filter_map(|field| field.controlled_impedance_solver_runtime_allowlist.as_ref())
    {
        if !controlled_impedance_solver_runtime_allowlists.insert(allowlist.name.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats controlled_impedance_solver_runtime_allowlist row name {}.",
                allowlist.name
            );
        }
    }
    let mut controlled_impedance_solver_entitlements = BTreeSet::new();
    for entitlement in fields
        .iter()
        .filter_map(|field| field.controlled_impedance_solver_entitlement.as_ref())
    {
        if !controlled_impedance_solver_entitlements.insert(entitlement.name.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats controlled_impedance_solver_entitlement row name {}.",
                entitlement.name
            );
        }
    }
    let mut controlled_impedance_solver_execution_environments = BTreeSet::new();
    for environment in fields.iter().filter_map(|field| {
        field
            .controlled_impedance_solver_execution_environment
            .as_ref()
    }) {
        if !controlled_impedance_solver_execution_environments.insert(environment.name.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats controlled_impedance_solver_execution_environment row name {}.",
                environment.name
            );
        }
    }
    let mut controlled_impedance_solver_run_logs = BTreeSet::new();
    for run_log in fields
        .iter()
        .filter_map(|field| field.controlled_impedance_solver_run_log.as_ref())
    {
        if !controlled_impedance_solver_run_logs.insert(run_log.name.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats controlled_impedance_solver_run_log row name {}.",
                run_log.name
            );
        }
    }
    let mut controlled_impedance_solver_reruns = BTreeSet::new();
    for rerun in fields
        .iter()
        .filter_map(|field| field.controlled_impedance_solver_rerun.as_ref())
    {
        let key = (rerun.solver_run_log_name.clone(), rerun.name.clone());
        if !controlled_impedance_solver_reruns.insert(key.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats controlled_impedance_solver_rerun row {} for run log {}.",
                rerun.name,
                rerun.solver_run_log_name
            );
        }
    }
    let mut controlled_impedance_solver_convergence_samples = BTreeSet::new();
    for sample in fields.iter().filter_map(|field| {
        field
            .controlled_impedance_solver_convergence_sample
            .as_ref()
    }) {
        let key = (sample.solver_run_log_name.clone(), sample.name.clone());
        if !controlled_impedance_solver_convergence_samples.insert(key.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats controlled_impedance_solver_convergence_sample row {} for run log {}.",
                sample.name,
                sample.solver_run_log_name
            );
        }
    }
    let mut thermal_copper_names = BTreeSet::new();
    for rule in fields
        .iter()
        .filter_map(|field| field.thermal_copper.as_ref())
    {
        if !thermal_copper_names.insert(rule.name.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats thermal_copper row name {}.",
                rule.name
            );
        }
    }
    let mut stackup_layer_names = BTreeSet::new();
    for layer in fields
        .iter()
        .filter_map(|field| field.stackup_layer.as_ref())
    {
        if !stackup_layer_names.insert(layer.name.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats stackup_layer row name {}.",
                layer.name
            );
        }
    }
    let mut thermal_package_components = BTreeSet::new();
    for package in fields
        .iter()
        .filter_map(|field| field.thermal_package.as_ref())
    {
        if !thermal_package_components.insert(package.component.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats thermal_package row component {}.",
                package.component
            );
        }
    }
    let mut thermal_environment_names = BTreeSet::new();
    for environment in fields
        .iter()
        .filter_map(|field| field.thermal_environment.as_ref())
    {
        if !thermal_environment_names.insert(environment.name.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats thermal_environment row name {}.",
                environment.name
            );
        }
    }
    let mut thermal_limit_names = BTreeSet::new();
    for limit in fields
        .iter()
        .filter_map(|field| field.thermal_limit.as_ref())
    {
        if !thermal_limit_names.insert(limit.name.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats thermal_limit row name {}.",
                limit.name
            );
        }
    }
    let mut rf_antenna_keepout_names = BTreeSet::new();
    for keepout in fields
        .iter()
        .filter_map(|field| field.rf_antenna_keepout.as_ref())
    {
        if !rf_antenna_keepout_names.insert(keepout.name.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats rf_antenna_keepout row name {}.",
                keepout.name
            );
        }
    }
    let mut rf_antenna_feed_path_names = BTreeSet::new();
    for feed_path in fields
        .iter()
        .filter_map(|field| field.rf_antenna_feed_path.as_ref())
    {
        if !rf_antenna_feed_path_names.insert(feed_path.name.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats rf_antenna_feed_path row name {}.",
                feed_path.name
            );
        }
    }
    let mut rf_antenna_matching_network_names = BTreeSet::new();
    for network in fields
        .iter()
        .filter_map(|field| field.rf_antenna_matching_network.as_ref())
    {
        if !rf_antenna_matching_network_names.insert(network.name.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats rf_antenna_matching_network row name {}.",
                network.name
            );
        }
    }
    let mut rf_antenna_measurement_names = BTreeSet::new();
    for measurement in fields
        .iter()
        .filter_map(|field| field.rf_antenna_measurement.as_ref())
    {
        if !rf_antenna_measurement_names.insert(measurement.name.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats rf_antenna_measurement row name {}.",
                measurement.name
            );
        }
    }
    let mut rf_antenna_performance_limit_names = BTreeSet::new();
    for limit in fields
        .iter()
        .filter_map(|field| field.rf_antenna_performance_limit.as_ref())
    {
        if !rf_antenna_performance_limit_names.insert(limit.name.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats rf_antenna_performance_limit row name {}.",
                limit.name
            );
        }
    }
    let mut rf_antenna_measurement_condition_names = BTreeSet::new();
    for condition in fields
        .iter()
        .filter_map(|field| field.rf_antenna_measurement_condition.as_ref())
    {
        if !rf_antenna_measurement_condition_names.insert(condition.name.clone()) {
            bail!(
                "Manufacturing metadata CSV repeats rf_antenna_measurement_condition row name {}.",
                condition.name
            );
        }
    }
    Ok(())
}

fn ordered_pair_key(first: &str, second: &str) -> String {
    if first <= second {
        format!("{first}/{second}")
    } else {
        format!("{second}/{first}")
    }
}
