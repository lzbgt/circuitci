use super::super::{
    AppliedLayoutPoint, AppliedRfAntennaFeedPath, AppliedRfAntennaKeepout,
    AppliedRfAntennaMatchingElement, AppliedRfAntennaMatchingNetwork, AppliedRfAntennaMeasurement,
    AppliedRfAntennaMeasurementCondition, AppliedRfAntennaPerformanceLimit, MetadataCsvRow,
    normalize_name, optional_raw_column, parse_nonempty_list, parse_nonnegative_mm,
    parse_positive_number, parse_positive_usize, required_nonnegative_number,
    required_positive_number, required_raw_column_for,
};
use anyhow::{Context, Result, bail};
use std::path::Path;

pub(in crate::importers::manufacturing_metadata::rows) fn applied_rf_antenna_keepout(
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

pub(in crate::importers::manufacturing_metadata::rows) fn applied_rf_antenna_feed_path(
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

pub(in crate::importers::manufacturing_metadata::rows) fn applied_rf_antenna_matching_network(
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

pub(in crate::importers::manufacturing_metadata::rows) fn applied_rf_antenna_measurement(
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

pub(in crate::importers::manufacturing_metadata::rows) fn applied_rf_antenna_performance_limit(
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

pub(in crate::importers::manufacturing_metadata::rows) fn applied_rf_antenna_measurement_condition(
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
