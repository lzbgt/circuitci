use super::ManufacturingMetadataImportOptions;
use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_yaml_ng::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

mod csv;
mod families;
mod values;

use csv::MetadataCsvRow;
pub(super) use csv::{ParsedMetadata, parse_metadata_csv};

#[derive(Debug, Clone)]
pub(super) struct AppliedField {
    pub(super) field: ManufacturingField,
    pub(super) numeric_value: Option<f64>,
    pub(super) string_value: Option<String>,
    pub(super) controlled_impedance_net: Option<AppliedControlledImpedanceNet>,
    pub(super) controlled_impedance_pair: Option<AppliedControlledImpedancePair>,
    pub(super) thermal_copper: Option<AppliedThermalCopper>,
    pub(super) thermal_measurement: Option<AppliedThermalMeasurement>,
    pub(super) thermal_package: Option<AppliedThermalPackage>,
    pub(super) stackup_layer: Option<AppliedStackupLayer>,
    pub(super) rf_antenna_keepout: Option<AppliedRfAntennaKeepout>,
    pub(super) rf_antenna_feed_path: Option<AppliedRfAntennaFeedPath>,
}

#[derive(Debug, Clone)]
pub(super) struct AppliedControlledImpedanceNet {
    pub(super) net: String,
    source: String,
    target_impedance_ohm: f64,
    expected_width_mm: f64,
    max_width_error_mm: f64,
}

#[derive(Debug, Clone)]
pub(super) struct AppliedControlledImpedancePair {
    pub(super) first_net: String,
    pub(super) second_net: String,
    source: String,
    target_differential_impedance_ohm: f64,
    expected_width_mm: f64,
    expected_gap_mm: f64,
    max_width_error_mm: f64,
    max_gap_error_mm: f64,
}

#[derive(Debug, Clone)]
pub(super) struct AppliedThermalCopper {
    pub(super) name: String,
    component: String,
    source: String,
    power_loss_w: f64,
    min_copper_area_mm2: f64,
    min_thermal_via_count: Option<usize>,
    min_plated_thermal_via_count: Option<usize>,
    min_thermal_via_drill_mm: Option<f64>,
    min_thermal_via_plating_thickness_um: Option<f64>,
    min_total_thermal_via_barrel_cross_section_mm2: Option<f64>,
    min_copper_thickness_um: Option<f64>,
    rated_ambient_temperature_c: Option<f64>,
    min_airflow_lfm: Option<f64>,
    enclosure_profile: Option<String>,
    nets: Vec<String>,
    layers: Vec<String>,
}

#[derive(Debug, Clone)]
pub(super) struct AppliedThermalMeasurement {
    pub(super) name: String,
    component: String,
    source: String,
    measured_temperature_c: f64,
    ambient_temperature_c: Option<f64>,
    measurement_uncertainty_c: Option<f64>,
    power_loss_w: Option<f64>,
    measurement_point: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct AppliedThermalPackage {
    pub(super) component: String,
    source: String,
    thermal_resistance_junction_to_ambient_c_per_w: f64,
    max_junction_temperature_c: f64,
}

#[derive(Debug, Clone)]
pub(super) struct AppliedStackupLayer {
    pub(super) name: String,
    kind: String,
    source: String,
    reference_net: Option<String>,
    thickness_mm: Option<f64>,
    copper_thickness_um: Option<f64>,
    dielectric_constant: Option<f64>,
    material: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct AppliedLayoutPoint {
    x_mm: f64,
    y_mm: f64,
}

#[derive(Debug, Clone)]
pub(super) struct AppliedRfAntennaKeepout {
    pub(super) name: String,
    antenna_net: Option<String>,
    layer: String,
    polygon: Vec<AppliedLayoutPoint>,
    min_copper_clearance_mm: f64,
    source: String,
}

#[derive(Debug, Clone)]
pub(super) struct AppliedRfAntennaFeedPath {
    pub(super) name: String,
    antenna_net: String,
    feed_component: String,
    feed_pin: String,
    matching_components: Vec<String>,
    max_feed_route_length_mm: f64,
    max_matching_component_distance_mm: f64,
    source: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ManufacturingField {
    StencilThicknessMm,
    MinDrillEdgeClearanceMm,
    MinSlotEdgeClearanceMm,
    MinPasteAreaRatio,
    MaxPasteAreaRatio,
    MinSolderPasteSpacingMm,
    MaxStitchViaDistanceMm,
    ControlledImpedanceNet,
    ControlledImpedancePair,
    ThermalCopper,
    ThermalMeasurement,
    ThermalPackage,
    StackupLayer,
    RfAntennaKeepout,
    RfAntennaFeedPath,
    Source,
}

impl ManufacturingField {
    pub(super) fn board_key(self) -> &'static str {
        match self {
            Self::StencilThicknessMm => "stencil_thickness_mm",
            Self::MinDrillEdgeClearanceMm => "min_drill_edge_clearance_mm",
            Self::MinSlotEdgeClearanceMm => "min_slot_edge_clearance_mm",
            Self::MinPasteAreaRatio => "min_paste_area_ratio",
            Self::MaxPasteAreaRatio => "max_paste_area_ratio",
            Self::MinSolderPasteSpacingMm => "min_solder_paste_spacing_mm",
            Self::MaxStitchViaDistanceMm => "max_stitch_via_distance_mm",
            Self::ControlledImpedanceNet => "controlled_impedance.nets[]",
            Self::ControlledImpedancePair => "controlled_impedance.differential_pairs[]",
            Self::ThermalCopper => "thermal_copper[]",
            Self::ThermalMeasurement => "thermal_measurements[]",
            Self::ThermalPackage => "thermal_packages[]",
            Self::StackupLayer => "layout.stackup.layers[]",
            Self::RfAntennaKeepout => "layout.constraints.rf_antenna.keepouts[]",
            Self::RfAntennaFeedPath => "layout.constraints.rf_antenna.feed_paths[]",
            Self::Source => "source",
        }
    }

    fn expects_mm(self) -> bool {
        matches!(
            self,
            Self::StencilThicknessMm
                | Self::MinDrillEdgeClearanceMm
                | Self::MinSlotEdgeClearanceMm
                | Self::MinSolderPasteSpacingMm
                | Self::MaxStitchViaDistanceMm
        )
    }

    fn expects_ratio(self) -> bool {
        matches!(self, Self::MinPasteAreaRatio | Self::MaxPasteAreaRatio)
    }

    fn expects_positive(self) -> bool {
        matches!(self, Self::StencilThicknessMm)
    }

    fn is_repeatable(self) -> bool {
        matches!(
            self,
            Self::ControlledImpedanceNet
                | Self::ControlledImpedancePair
                | Self::ThermalCopper
                | Self::ThermalMeasurement
                | Self::ThermalPackage
                | Self::StackupLayer
                | Self::RfAntennaKeepout
                | Self::RfAntennaFeedPath
        )
    }
}

#[derive(Debug, Serialize)]
pub(super) struct RowManifest {
    row_number: usize,
    raw_field: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    board_field: Option<String>,
    raw_value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    normalized_value: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    notes: Option<String>,
    raw_columns: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
}

pub(super) fn normalize_rows(
    parsed: &ParsedMetadata,
    options: &ManufacturingMetadataImportOptions,
) -> Result<(Vec<AppliedField>, Vec<RowManifest>, usize)> {
    let mut applied = Vec::new();
    let mut manifests = Vec::new();
    let mut seen_supported = BTreeSet::new();
    let mut skipped = 0;
    for row in &parsed.rows {
        let Some(field) = normalize_field(&row.field) else {
            if options.allow_unknown_fields {
                skipped += 1;
                manifests.push(row_manifest(
                    row,
                    "skipped_unknown_field",
                    None,
                    None,
                    Some(format!(
                        "Unsupported manufacturing metadata field {}",
                        row.field
                    )),
                ));
                continue;
            }
            bail!(
                "Manufacturing metadata CSV {} row {} has unsupported field {}.",
                options.metadata.display(),
                row.row_number,
                row.field
            );
        };
        if !field.is_repeatable() && !seen_supported.insert(field) {
            bail!(
                "Manufacturing metadata CSV {} repeats supported field {}.",
                options.metadata.display(),
                field.board_key()
            );
        }
        let applied_field = applied_field(row, field, &options.metadata)?;
        let normalized_value = normalized_yaml_value(&applied_field)?;
        manifests.push(row_manifest(
            row,
            "applied",
            Some(field.board_key().to_string()),
            Some(normalized_value),
            None,
        ));
        applied.push(applied_field);
    }
    validate_applied_fields(&applied)?;
    Ok((applied, manifests, skipped))
}

fn applied_field(
    row: &MetadataCsvRow,
    field: ManufacturingField,
    path: &Path,
) -> Result<AppliedField> {
    if field == ManufacturingField::Source {
        let value = row.value.trim();
        if value.is_empty() {
            bail!(
                "Manufacturing metadata CSV {} row {} has empty source value.",
                path.display(),
                row.row_number
            );
        }
        return Ok(AppliedField {
            field,
            numeric_value: None,
            string_value: Some(value.to_string()),
            controlled_impedance_net: None,
            controlled_impedance_pair: None,
            thermal_copper: None,
            thermal_measurement: None,
            thermal_package: None,
            stackup_layer: None,
            rf_antenna_keepout: None,
            rf_antenna_feed_path: None,
        });
    }
    if field == ManufacturingField::ControlledImpedanceNet {
        return Ok(AppliedField {
            field,
            numeric_value: None,
            string_value: None,
            controlled_impedance_net: Some(applied_controlled_impedance_net(row, path)?),
            controlled_impedance_pair: None,
            thermal_copper: None,
            thermal_measurement: None,
            thermal_package: None,
            stackup_layer: None,
            rf_antenna_keepout: None,
            rf_antenna_feed_path: None,
        });
    }
    if field == ManufacturingField::ControlledImpedancePair {
        return Ok(AppliedField {
            field,
            numeric_value: None,
            string_value: None,
            controlled_impedance_net: None,
            controlled_impedance_pair: Some(applied_controlled_impedance_pair(row, path)?),
            thermal_copper: None,
            thermal_measurement: None,
            thermal_package: None,
            stackup_layer: None,
            rf_antenna_keepout: None,
            rf_antenna_feed_path: None,
        });
    }
    if field == ManufacturingField::ThermalCopper {
        return Ok(AppliedField {
            field,
            numeric_value: None,
            string_value: None,
            controlled_impedance_net: None,
            controlled_impedance_pair: None,
            thermal_copper: Some(applied_thermal_copper(row, path)?),
            thermal_measurement: None,
            thermal_package: None,
            stackup_layer: None,
            rf_antenna_keepout: None,
            rf_antenna_feed_path: None,
        });
    }
    if field == ManufacturingField::ThermalMeasurement {
        return Ok(AppliedField {
            field,
            numeric_value: None,
            string_value: None,
            controlled_impedance_net: None,
            controlled_impedance_pair: None,
            thermal_copper: None,
            thermal_measurement: Some(applied_thermal_measurement(row, path)?),
            thermal_package: None,
            stackup_layer: None,
            rf_antenna_keepout: None,
            rf_antenna_feed_path: None,
        });
    }
    if field == ManufacturingField::ThermalPackage {
        return Ok(AppliedField {
            field,
            numeric_value: None,
            string_value: None,
            controlled_impedance_net: None,
            controlled_impedance_pair: None,
            thermal_copper: None,
            thermal_measurement: None,
            thermal_package: Some(applied_thermal_package(row, path)?),
            stackup_layer: None,
            rf_antenna_keepout: None,
            rf_antenna_feed_path: None,
        });
    }
    if field == ManufacturingField::StackupLayer {
        return Ok(AppliedField {
            field,
            numeric_value: None,
            string_value: None,
            controlled_impedance_net: None,
            controlled_impedance_pair: None,
            thermal_copper: None,
            thermal_measurement: None,
            thermal_package: None,
            stackup_layer: Some(applied_stackup_layer(row, path)?),
            rf_antenna_keepout: None,
            rf_antenna_feed_path: None,
        });
    }
    if field == ManufacturingField::RfAntennaKeepout {
        return Ok(AppliedField {
            field,
            numeric_value: None,
            string_value: None,
            controlled_impedance_net: None,
            controlled_impedance_pair: None,
            thermal_copper: None,
            thermal_measurement: None,
            thermal_package: None,
            stackup_layer: None,
            rf_antenna_keepout: Some(applied_rf_antenna_keepout(row, path)?),
            rf_antenna_feed_path: None,
        });
    }
    if field == ManufacturingField::RfAntennaFeedPath {
        return Ok(AppliedField {
            field,
            numeric_value: None,
            string_value: None,
            controlled_impedance_net: None,
            controlled_impedance_pair: None,
            thermal_copper: None,
            thermal_measurement: None,
            thermal_package: None,
            stackup_layer: None,
            rf_antenna_keepout: None,
            rf_antenna_feed_path: Some(applied_rf_antenna_feed_path(row, path)?),
        });
    }
    let value = row.value.trim();
    if value.is_empty() {
        bail!(
            "Manufacturing metadata CSV {} row {} has empty value for {}.",
            path.display(),
            row.row_number,
            field.board_key()
        );
    }
    let numeric_value = value.parse::<f64>().with_context(|| {
        format!(
            "Manufacturing metadata CSV {} row {} has invalid number {}.",
            path.display(),
            row.row_number,
            value
        )
    })?;
    let normalized = normalize_numeric_value(field, numeric_value, row.unit.as_deref(), path, row)?;
    Ok(AppliedField {
        field,
        numeric_value: Some(normalized),
        string_value: None,
        controlled_impedance_net: None,
        controlled_impedance_pair: None,
        thermal_copper: None,
        thermal_measurement: None,
        thermal_package: None,
        stackup_layer: None,
        rf_antenna_keepout: None,
        rf_antenna_feed_path: None,
    })
}

fn applied_controlled_impedance_net(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedControlledImpedanceNet> {
    families::applied_controlled_impedance_net(row, path)
}

fn applied_controlled_impedance_pair(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedControlledImpedancePair> {
    families::applied_controlled_impedance_pair(row, path)
}

fn applied_thermal_copper(row: &MetadataCsvRow, path: &Path) -> Result<AppliedThermalCopper> {
    families::applied_thermal_copper(row, path)
}

fn applied_thermal_measurement(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedThermalMeasurement> {
    families::applied_thermal_measurement(row, path)
}

fn applied_thermal_package(row: &MetadataCsvRow, path: &Path) -> Result<AppliedThermalPackage> {
    families::applied_thermal_package(row, path)
}

fn applied_stackup_layer(row: &MetadataCsvRow, path: &Path) -> Result<AppliedStackupLayer> {
    families::applied_stackup_layer(row, path)
}

fn applied_rf_antenna_keepout(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedRfAntennaKeepout> {
    families::applied_rf_antenna_keepout(row, path)
}

fn applied_rf_antenna_feed_path(
    row: &MetadataCsvRow,
    path: &Path,
) -> Result<AppliedRfAntennaFeedPath> {
    families::applied_rf_antenna_feed_path(row, path)
}

fn normalize_numeric_value(
    field: ManufacturingField,
    value: f64,
    unit: Option<&str>,
    path: &Path,
    row: &MetadataCsvRow,
) -> Result<f64> {
    if !value.is_finite() {
        bail!(
            "Manufacturing metadata CSV {} row {} has non-finite value for {}.",
            path.display(),
            row.row_number,
            field.board_key()
        );
    }
    let unit = unit.map(normalize_unit);
    if field.expects_mm()
        && !matches!(
            unit.as_deref(),
            None | Some("") | Some("mm") | Some("millimeter") | Some("millimeters")
        )
    {
        bail!(
            "Manufacturing metadata CSV {} row {} must use mm for {}.",
            path.display(),
            row.row_number,
            field.board_key()
        );
    }
    if field.expects_ratio() {
        let normalized = if matches!(unit.as_deref(), Some("%") | Some("percent")) {
            value / 100.0
        } else if matches!(
            unit.as_deref(),
            None | Some("") | Some("ratio") | Some("fraction")
        ) {
            value
        } else {
            bail!(
                "Manufacturing metadata CSV {} row {} must use a ratio, fraction, or percent unit for {}.",
                path.display(),
                row.row_number,
                field.board_key()
            );
        };
        if normalized < 0.0 {
            bail!(
                "Manufacturing metadata CSV {} row {} ratio {} must be non-negative.",
                path.display(),
                row.row_number,
                field.board_key()
            );
        }
        return Ok(normalized);
    }
    if field.expects_positive() && value <= 0.0 {
        bail!(
            "Manufacturing metadata CSV {} row {} value {} must be greater than zero.",
            path.display(),
            row.row_number,
            field.board_key()
        );
    }
    if !field.expects_positive() && value < 0.0 {
        bail!(
            "Manufacturing metadata CSV {} row {} value {} must be non-negative.",
            path.display(),
            row.row_number,
            field.board_key()
        );
    }
    Ok(value)
}

fn parse_temperature_c(
    raw: &str,
    unit: Option<&str>,
    path: &Path,
    row: &MetadataCsvRow,
    column: &str,
) -> Result<f64> {
    if raw.trim().is_empty() {
        bail!(
            "Manufacturing metadata CSV {} row {} has empty {column}.",
            path.display(),
            row.row_number
        );
    }
    let value = raw.trim().parse::<f64>().with_context(|| {
        format!(
            "Manufacturing metadata CSV {} row {} has invalid temperature {}.",
            path.display(),
            row.row_number,
            raw
        )
    })?;
    if !value.is_finite() {
        bail!(
            "Manufacturing metadata CSV {} row {} temperature {column} must be finite.",
            path.display(),
            row.row_number
        );
    }
    let unit = unit.map(normalize_unit);
    if !matches!(
        unit.as_deref(),
        None | Some("") | Some("c") | Some("degc") | Some("celsius") | Some("°c")
    ) {
        bail!(
            "Manufacturing metadata CSV {} row {} must use C/celsius for {column}.",
            path.display(),
            row.row_number
        );
    }
    Ok(value)
}

fn parse_positive_watts(raw: &str, path: &Path, row: &MetadataCsvRow, column: &str) -> Result<f64> {
    parse_positive_number(raw, path, row, column).with_context(|| {
        format!(
            "Manufacturing metadata CSV {} row {} has invalid power {}.",
            path.display(),
            row.row_number,
            raw
        )
    })
}

fn required_positive_watts(
    row: &MetadataCsvRow,
    path: &Path,
    column: &str,
    record: &str,
) -> Result<f64> {
    let value = required_raw_column_for(row, path, column, record)?;
    parse_positive_watts(&value, path, row, column)
}

fn required_positive_number(
    row: &MetadataCsvRow,
    path: &Path,
    column: &str,
    record: &str,
) -> Result<f64> {
    let value = required_raw_column_for(row, path, column, record)?;
    parse_positive_number(&value, path, row, column)
}

fn required_nonnegative_number(
    row: &MetadataCsvRow,
    path: &Path,
    column: &str,
    record: &str,
) -> Result<f64> {
    let value = required_raw_column_for(row, path, column, record)?;
    parse_nonnegative_number(&value, path, row, column)
}

fn parse_positive_ohms(
    raw: &str,
    unit: Option<&str>,
    path: &Path,
    row: &MetadataCsvRow,
    column: &str,
) -> Result<f64> {
    let value = parse_positive_number(raw, path, row, column)?;
    let unit = unit.map(normalize_unit);
    if !matches!(
        unit.as_deref(),
        None | Some("") | Some("ohm") | Some("ohms") | Some("ω")
    ) {
        bail!(
            "Manufacturing metadata CSV {} row {} must use ohms for {column}.",
            path.display(),
            row.row_number
        );
    }
    Ok(value)
}

fn parse_positive_area_mm2(
    raw: &str,
    unit: Option<&str>,
    path: &Path,
    row: &MetadataCsvRow,
    column: &str,
) -> Result<f64> {
    let value = parse_positive_number(raw, path, row, column)?;
    let unit = unit.map(normalize_unit);
    if !matches!(
        unit.as_deref(),
        None | Some("")
            | Some("mm2")
            | Some("mm^2")
            | Some("squaremm")
            | Some("squaremillimeter")
            | Some("squaremillimeters")
    ) {
        bail!(
            "Manufacturing metadata CSV {} row {} must use mm2/square millimeters for {column}.",
            path.display(),
            row.row_number
        );
    }
    Ok(value)
}

fn parse_positive_c_per_w(
    raw: &str,
    unit: Option<&str>,
    path: &Path,
    row: &MetadataCsvRow,
    column: &str,
) -> Result<f64> {
    let value = parse_positive_number(raw, path, row, column)?;
    let unit = unit.map(normalize_unit);
    let normalized_unit = unit.as_deref().map(normalize_name);
    if !matches!(
        normalized_unit.as_deref(),
        None | Some("") | Some("cw") | Some("cperw") | Some("celsiusperwatt") | Some("celsiusperw")
    ) {
        bail!(
            "Manufacturing metadata CSV {} row {} must use C/W for {column}.",
            path.display(),
            row.row_number
        );
    }
    Ok(value)
}

fn parse_nonnegative_mm(
    raw: &str,
    unit: Option<&str>,
    path: &Path,
    row: &MetadataCsvRow,
    column: &str,
) -> Result<f64> {
    let value = parse_nonnegative_number(raw, path, row, column)?;
    let unit = unit.map(normalize_unit);
    if !matches!(
        unit.as_deref(),
        None | Some("") | Some("mm") | Some("millimeter") | Some("millimeters")
    ) {
        bail!(
            "Manufacturing metadata CSV {} row {} must use mm for {column}.",
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
) -> Result<usize> {
    let value = raw.trim().parse::<usize>().with_context(|| {
        format!(
            "Manufacturing metadata CSV {} row {} has invalid integer {}.",
            path.display(),
            row.row_number,
            raw
        )
    })?;
    if value == 0 {
        bail!(
            "Manufacturing metadata CSV {} row {} {column} must be positive.",
            path.display(),
            row.row_number
        );
    }
    Ok(value)
}

fn parse_positive_number(
    raw: &str,
    path: &Path,
    row: &MetadataCsvRow,
    column: &str,
) -> Result<f64> {
    let value = raw.trim().parse::<f64>().with_context(|| {
        format!(
            "Manufacturing metadata CSV {} row {} has invalid number {}.",
            path.display(),
            row.row_number,
            raw
        )
    })?;
    if !value.is_finite() || value <= 0.0 {
        bail!(
            "Manufacturing metadata CSV {} row {} {column} must be finite and positive.",
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
) -> Result<f64> {
    let value = raw.trim().parse::<f64>().with_context(|| {
        format!(
            "Manufacturing metadata CSV {} row {} has invalid number {}.",
            path.display(),
            row.row_number,
            raw
        )
    })?;
    if !value.is_finite() || value < 0.0 {
        bail!(
            "Manufacturing metadata CSV {} row {} {column} must be finite and non-negative.",
            path.display(),
            row.row_number
        );
    }
    Ok(value)
}

fn parse_nonempty_list(raw: &str) -> Result<Vec<String>> {
    let values: Vec<String> = raw
        .split([',', ';', '|'])
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect();
    if values.is_empty() {
        bail!("list column must contain at least one value.");
    }
    Ok(values)
}

fn parse_nonnegative_temperature_delta_c(
    raw: &str,
    path: &Path,
    row: &MetadataCsvRow,
    column: &str,
) -> Result<f64> {
    let value = parse_temperature_c(raw, None, path, row, column)?;
    if value < 0.0 {
        bail!(
            "Manufacturing metadata CSV {} row {} {column} must be non-negative.",
            path.display(),
            row.row_number
        );
    }
    Ok(value)
}

fn validate_applied_fields(fields: &[AppliedField]) -> Result<()> {
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
    Ok(())
}

fn ordered_pair_key(first: &str, second: &str) -> String {
    if first <= second {
        format!("{first}/{second}")
    } else {
        format!("{second}/{first}")
    }
}

pub(super) fn normalized_yaml_value(field: &AppliedField) -> Result<Value> {
    values::normalized_yaml_value(field)
}

fn row_manifest(
    row: &MetadataCsvRow,
    status: &str,
    board_field: Option<String>,
    normalized_value: Option<Value>,
    message: Option<String>,
) -> RowManifest {
    RowManifest {
        row_number: row.row_number,
        raw_field: row.field.clone(),
        status: status.to_string(),
        board_field,
        raw_value: row.value.clone(),
        normalized_value,
        unit: row.unit.clone(),
        source: row.source.clone(),
        notes: row.notes.clone(),
        raw_columns: row.raw_fields.clone(),
        message,
    }
}

fn normalize_field(value: &str) -> Option<ManufacturingField> {
    match normalize_name(value).as_str() {
        "stencilthicknessmm" | "stencilthickness" | "stencilfoilthickness" => {
            Some(ManufacturingField::StencilThicknessMm)
        }
        "mindrilledgeclearancemm"
        | "mindrilledgeclearance"
        | "holetoboardedgeclearance"
        | "minimumholetoboardedgeclearance" => Some(ManufacturingField::MinDrillEdgeClearanceMm),
        "minslotedgeclearancemm" | "minslotedgeclearance" | "slottoboardedgeclearance" => {
            Some(ManufacturingField::MinSlotEdgeClearanceMm)
        }
        "minpastearearatio" | "minimumsolderpastearearatio" => {
            Some(ManufacturingField::MinPasteAreaRatio)
        }
        "maxpastearearatio" | "maximumsolderpastearearatio" => {
            Some(ManufacturingField::MaxPasteAreaRatio)
        }
        "minsolderpastespacingmm" | "minsolderpastespacing" | "minpastespace" => {
            Some(ManufacturingField::MinSolderPasteSpacingMm)
        }
        "maxstitchviadistancemm"
        | "maxstitchviadistance"
        | "maximumstitchviadistance"
        | "stitchviadistance" => Some(ManufacturingField::MaxStitchViaDistanceMm),
        "controlledimpedancenet" | "controlledimpedance" | "impedancenet" => {
            Some(ManufacturingField::ControlledImpedanceNet)
        }
        "controlledimpedancepair"
        | "controlledimpedancedifferentialpair"
        | "differentialimpedancepair"
        | "differentialpairimpedance" => Some(ManufacturingField::ControlledImpedancePair),
        "thermalcopper" | "thermalcopperpolicy" | "thermalpolicy" | "thermalcopperarea" => {
            Some(ManufacturingField::ThermalCopper)
        }
        "thermalmeasurement" | "thermalmeasuredtemperature" | "measuredtemperature" => {
            Some(ManufacturingField::ThermalMeasurement)
        }
        "thermalpackage" | "packagethermal" | "componentthermalpackage" => {
            Some(ManufacturingField::ThermalPackage)
        }
        "stackuplayer" | "stackuplayermetadata" | "boardstackuplayer" => {
            Some(ManufacturingField::StackupLayer)
        }
        "rfantennakeepout" | "antennakeepout" | "rfkeepout" => {
            Some(ManufacturingField::RfAntennaKeepout)
        }
        "rfantennafeedpath" | "antennafeedpath" | "rffeedpath" => {
            Some(ManufacturingField::RfAntennaFeedPath)
        }
        "source" | "evidencesource" => Some(ManufacturingField::Source),
        _ => None,
    }
}

fn normalize_name(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn normalize_unit(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn optional_raw_column(row: &MetadataCsvRow, name: &str) -> Option<String> {
    let normalized_name = normalize_name(name);
    row.raw_fields
        .iter()
        .find(|(key, _)| normalize_name(key) == normalized_name)
        .map(|(_, value)| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn required_raw_column_for(
    row: &MetadataCsvRow,
    path: &Path,
    name: &str,
    record: &str,
) -> Result<String> {
    optional_raw_column(row, name).with_context(|| {
        format!(
            "Manufacturing metadata CSV {} row {} {record} requires column {name}.",
            path.display(),
            row.row_number
        )
    })
}
