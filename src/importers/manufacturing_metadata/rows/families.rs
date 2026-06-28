use super::{
    AppliedControlledImpedanceNet, AppliedControlledImpedancePair, AppliedLayoutPoint,
    AppliedRfAntennaFeedPath, AppliedRfAntennaKeepout, AppliedStackupLayer, AppliedThermalCopper,
    AppliedThermalEnvironment, AppliedThermalLimit, AppliedThermalMeasurement,
    AppliedThermalPackage, MetadataCsvRow, normalize_name, optional_raw_column,
    parse_nonempty_list, parse_nonnegative_mm, parse_nonnegative_number,
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
    })
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
