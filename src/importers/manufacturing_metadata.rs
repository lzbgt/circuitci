use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_yaml_ng::{Mapping, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ManufacturingMetadataImportOptions {
    pub project: PathBuf,
    pub metadata: PathBuf,
    pub output: PathBuf,
    pub manifest: PathBuf,
    pub source: Option<String>,
    pub allow_unknown_fields: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ManufacturingMetadataImportSummary {
    pub rows: usize,
    pub applied_fields: usize,
    pub skipped_rows: usize,
}

#[derive(Debug, Clone)]
struct ParsedMetadata {
    headers: Vec<String>,
    data_rows: usize,
    rows: Vec<MetadataCsvRow>,
}

#[derive(Debug, Clone)]
struct MetadataCsvRow {
    row_number: usize,
    field: String,
    value: String,
    unit: Option<String>,
    source: Option<String>,
    notes: Option<String>,
    raw_fields: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
struct AppliedField {
    field: ManufacturingField,
    numeric_value: Option<f64>,
    string_value: Option<String>,
    controlled_impedance_net: Option<AppliedControlledImpedanceNet>,
    controlled_impedance_pair: Option<AppliedControlledImpedancePair>,
    thermal_copper: Option<AppliedThermalCopper>,
    thermal_measurement: Option<AppliedThermalMeasurement>,
}

#[derive(Debug, Clone)]
struct AppliedControlledImpedanceNet {
    net: String,
    source: String,
    target_impedance_ohm: f64,
    expected_width_mm: f64,
    max_width_error_mm: f64,
}

#[derive(Debug, Clone)]
struct AppliedControlledImpedancePair {
    first_net: String,
    second_net: String,
    source: String,
    target_differential_impedance_ohm: f64,
    expected_width_mm: f64,
    expected_gap_mm: f64,
    max_width_error_mm: f64,
    max_gap_error_mm: f64,
}

#[derive(Debug, Clone)]
struct AppliedThermalCopper {
    name: String,
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
struct AppliedThermalMeasurement {
    name: String,
    component: String,
    source: String,
    measured_temperature_c: f64,
    ambient_temperature_c: Option<f64>,
    measurement_uncertainty_c: Option<f64>,
    power_loss_w: Option<f64>,
    measurement_point: Option<String>,
    notes: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ManufacturingField {
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
    Source,
}

impl ManufacturingField {
    fn board_key(self) -> &'static str {
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
        )
    }
}

#[derive(Debug, Serialize)]
struct ImportManifest {
    schema_version: String,
    sources: SourceManifest,
    import: ImportSummaryManifest,
    rows: Vec<RowManifest>,
}

#[derive(Debug, Serialize)]
struct SourceManifest {
    project: SourceFileManifest,
    metadata: SourceCsvManifest,
}

#[derive(Debug, Serialize)]
struct SourceFileManifest {
    path: String,
    size_bytes: u64,
    sha256: String,
}

#[derive(Debug, Serialize)]
struct SourceCsvManifest {
    path: String,
    size_bytes: u64,
    sha256: String,
    columns: Vec<String>,
    data_rows: usize,
}

#[derive(Debug, Serialize)]
struct ImportSummaryManifest {
    applied_fields: usize,
    skipped_rows: usize,
    allow_unknown_fields: bool,
    source_label: String,
}

#[derive(Debug, Serialize)]
struct RowManifest {
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

pub fn import_manufacturing_metadata(
    options: &ManufacturingMetadataImportOptions,
) -> Result<ManufacturingMetadataImportSummary> {
    let parsed = parse_metadata_csv(&options.metadata)?;
    let text = fs::read_to_string(&options.project).with_context(|| {
        format!(
            "Failed to read Board IR project {}",
            options.project.display()
        )
    })?;
    let mut project_yaml: Value = serde_yaml_ng::from_str(&text).with_context(|| {
        format!(
            "Failed to parse Board IR project YAML {}",
            options.project.display()
        )
    })?;

    let (applied, row_manifests, skipped_rows) = normalize_rows(&parsed, options)?;
    if applied.is_empty() {
        bail!(
            "Manufacturing metadata import found no supported fields in {}.",
            options.metadata.display()
        );
    }
    apply_metadata(&mut project_yaml, &applied, source_label(options, &applied))?;
    absolutize_relative_libraries(
        &mut project_yaml,
        options
            .project
            .parent()
            .unwrap_or_else(|| std::path::Path::new(".")),
    )?;

    if let Some(parent) = options.output.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create manufacturing metadata import output directory {}",
                parent.display()
            )
        })?;
    }
    let mut yaml = serde_yaml_ng::to_string(&project_yaml)?;
    yaml.insert_str(
        0,
        "# Generated by CircuitCI from reviewed board/order manufacturing metadata CSV evidence.\n",
    );
    fs::write(&options.output, yaml).with_context(|| {
        format!(
            "Failed to write manufacturing metadata import project {}",
            options.output.display()
        )
    })?;

    let manifest = ImportManifest {
        schema_version: "0.3.0".to_string(),
        sources: SourceManifest {
            project: source_file_manifest(&options.project)?,
            metadata: source_csv_manifest(&options.metadata, &parsed)?,
        },
        import: ImportSummaryManifest {
            applied_fields: applied.len(),
            skipped_rows,
            allow_unknown_fields: options.allow_unknown_fields,
            source_label: source_label(options, &applied).to_string(),
        },
        rows: row_manifests,
    };
    if let Some(parent) = options.manifest.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create manufacturing metadata manifest output directory {}",
                parent.display()
            )
        })?;
    }
    fs::write(
        &options.manifest,
        serde_json::to_string_pretty(&manifest)?.as_bytes(),
    )
    .with_context(|| {
        format!(
            "Failed to write manufacturing metadata manifest {}",
            options.manifest.display()
        )
    })?;

    Ok(ManufacturingMetadataImportSummary {
        rows: parsed.data_rows,
        applied_fields: applied.len(),
        skipped_rows,
    })
}

fn normalize_rows(
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
    })
}

fn applied_controlled_impedance_net(
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

fn applied_controlled_impedance_pair(
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

fn applied_thermal_copper(row: &MetadataCsvRow, path: &Path) -> Result<AppliedThermalCopper> {
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

fn applied_thermal_measurement(
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
    Ok(())
}

fn normalized_yaml_value(field: &AppliedField) -> Result<Value> {
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

fn apply_metadata(
    project_yaml: &mut Value,
    fields: &[AppliedField],
    source_label: &str,
) -> Result<()> {
    let root = project_yaml
        .as_mapping_mut()
        .context("Board IR project must be a YAML object.")?;
    let board = ensure_mapping_field_mut(root, "board")?;
    let manufacturing = ensure_mapping_field_mut(board, "manufacturing")?;
    for field in fields {
        if field.field == ManufacturingField::ControlledImpedanceNet {
            let target = field
                .controlled_impedance_net
                .as_ref()
                .context("controlled_impedance_net field must have target value")?;
            let value = normalized_yaml_value(field)?;
            let controlled_impedance =
                ensure_mapping_field_mut(manufacturing, "controlled_impedance")?;
            let targets = ensure_sequence_field_mut(controlled_impedance, "nets")?;
            upsert_controlled_impedance_net_value(targets, &target.net, value)?;
        } else if field.field == ManufacturingField::ControlledImpedancePair {
            let target = field
                .controlled_impedance_pair
                .as_ref()
                .context("controlled_impedance_pair field must have target value")?;
            let value = normalized_yaml_value(field)?;
            let controlled_impedance =
                ensure_mapping_field_mut(manufacturing, "controlled_impedance")?;
            let targets = ensure_sequence_field_mut(controlled_impedance, "differential_pairs")?;
            upsert_controlled_impedance_pair_value(
                targets,
                &target.first_net,
                &target.second_net,
                value,
            )?;
        } else if field.field == ManufacturingField::ThermalCopper {
            let rule = field
                .thermal_copper
                .as_ref()
                .context("thermal_copper field must have thermal copper value")?;
            let value = normalized_yaml_value(field)?;
            let rules = ensure_sequence_field_mut(manufacturing, "thermal_copper")?;
            upsert_named_sequence_value(rules, &rule.name, value)?;
        } else if field.field == ManufacturingField::ThermalMeasurement {
            let measurements = ensure_sequence_field_mut(manufacturing, "thermal_measurements")?;
            measurements.push(normalized_yaml_value(field)?);
        } else {
            manufacturing.insert(
                Value::String(field.field.board_key().to_string()),
                normalized_yaml_value(field)?,
            );
        }
    }
    manufacturing.insert(
        Value::String("source".to_string()),
        Value::String(source_label.to_string()),
    );
    Ok(())
}

fn source_label<'a>(
    options: &'a ManufacturingMetadataImportOptions,
    fields: &'a [AppliedField],
) -> &'a str {
    options
        .source
        .as_deref()
        .or_else(|| {
            fields
                .iter()
                .find(|field| field.field == ManufacturingField::Source)
                .and_then(|field| field.string_value.as_deref())
        })
        .unwrap_or("manufacturing_metadata_csv")
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
        "source" | "evidencesource" => Some(ManufacturingField::Source),
        _ => None,
    }
}

fn parse_metadata_csv(path: &Path) -> Result<ParsedMetadata> {
    let table = read_csv_table(path)?;
    let (headers, rows) = table
        .split_first()
        .with_context(|| format!("Manufacturing metadata CSV {} is empty.", path.display()))?;
    let columns = HeaderMap::new(headers);
    let field_column = columns.required("field", path)?;
    let value_column = columns.required("value", path)?;
    let mut parsed_rows = Vec::new();
    for (row_index, row) in rows.iter().enumerate() {
        if row.iter().all(|cell| cell.trim().is_empty()) {
            continue;
        }
        let row_number = row_index + 2;
        let field = cell(row, field_column);
        if field.is_empty() {
            bail!(
                "Manufacturing metadata CSV {} row {} has empty field.",
                path.display(),
                row_number
            );
        }
        let mut raw_fields = BTreeMap::new();
        for (index, header) in headers.iter().enumerate() {
            raw_fields.insert(header.clone(), cell(row, index).to_string());
        }
        parsed_rows.push(MetadataCsvRow {
            row_number,
            field: field.to_string(),
            value: cell(row, value_column).to_string(),
            unit: optional_string(row, columns.optional("unit")),
            source: optional_string(row, columns.optional("source")),
            notes: optional_string(row, columns.optional("notes")),
            raw_fields,
        });
    }
    Ok(ParsedMetadata {
        headers: headers.clone(),
        data_rows: rows.len(),
        rows: parsed_rows,
    })
}

fn read_csv_table(path: &Path) -> Result<Vec<Vec<String>>> {
    let text = fs::read_to_string(path).with_context(|| {
        format!(
            "Failed to read manufacturing metadata CSV {}",
            path.display()
        )
    })?;
    parse_csv(&text).with_context(|| format!("Failed to parse CSV {}", path.display()))
}

fn parse_csv(input: &str) -> Result<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut chars = input.chars().peekable();
    let mut in_quotes = false;
    while let Some(character) = chars.next() {
        match character {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                cell.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                row.push(std::mem::take(&mut cell));
            }
            '\n' if !in_quotes => {
                row.push(trim_cr(std::mem::take(&mut cell)));
                rows.push(std::mem::take(&mut row));
            }
            character => cell.push(character),
        }
    }
    if in_quotes {
        bail!("CSV has an unterminated quoted field.");
    }
    if !cell.is_empty() || !row.is_empty() {
        row.push(trim_cr(cell));
        rows.push(row);
    }
    Ok(rows)
}

fn trim_cr(mut value: String) -> String {
    if value.ends_with('\r') {
        value.pop();
    }
    value
}

struct HeaderMap {
    columns: BTreeMap<String, usize>,
}

impl HeaderMap {
    fn new(headers: &[String]) -> Self {
        let columns = headers
            .iter()
            .enumerate()
            .map(|(index, header)| (normalize_name(header), index))
            .collect();
        Self { columns }
    }

    fn required(&self, name: &str, path: &Path) -> Result<usize> {
        self.optional(name).with_context(|| {
            format!(
                "Manufacturing metadata CSV {} is missing required column {name}.",
                path.display()
            )
        })
    }

    fn optional(&self, name: &str) -> Option<usize> {
        self.columns.get(&normalize_name(name)).copied()
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

fn cell(row: &[String], index: usize) -> &str {
    row.get(index).map(String::as_str).unwrap_or("").trim()
}

fn optional_string(row: &[String], index: Option<usize>) -> Option<String> {
    index
        .map(|index| cell(row, index))
        .filter(|value| !value.is_empty())
        .map(str::to_string)
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

fn upsert_controlled_impedance_net_value(
    sequence: &mut Vec<Value>,
    net: &str,
    value: Value,
) -> Result<()> {
    let mut matched_index = None;
    for (index, item) in sequence.iter().enumerate() {
        if yaml_mapping_string(item, "net") == Some(net) {
            if matched_index.is_some() {
                bail!(
                    "Board IR field controlled_impedance.nets has duplicate existing net {net}; refusing to update ambiguous target."
                );
            }
            matched_index = Some(index);
        }
    }
    if let Some(index) = matched_index {
        sequence[index] = value;
    } else {
        sequence.push(value);
    }
    Ok(())
}

fn upsert_controlled_impedance_pair_value(
    sequence: &mut Vec<Value>,
    first_net: &str,
    second_net: &str,
    value: Value,
) -> Result<()> {
    let target_key = ordered_pair_key(first_net, second_net);
    let mut matched_index = None;
    for (index, item) in sequence.iter().enumerate() {
        let Some(existing_first) = yaml_mapping_string(item, "first_net") else {
            continue;
        };
        let Some(existing_second) = yaml_mapping_string(item, "second_net") else {
            continue;
        };
        if ordered_pair_key(existing_first, existing_second) == target_key {
            if matched_index.is_some() {
                bail!(
                    "Board IR field controlled_impedance.differential_pairs has duplicate existing pair {target_key}; refusing to update ambiguous target."
                );
            }
            matched_index = Some(index);
        }
    }
    if let Some(index) = matched_index {
        sequence[index] = value;
    } else {
        sequence.push(value);
    }
    Ok(())
}

fn upsert_named_sequence_value(sequence: &mut Vec<Value>, name: &str, value: Value) -> Result<()> {
    let mut matched_index = None;
    for (index, item) in sequence.iter().enumerate() {
        if yaml_mapping_name(item) == Some(name) {
            if matched_index.is_some() {
                bail!(
                    "Board IR field thermal_copper has duplicate existing name {name}; refusing to update ambiguous policy."
                );
            }
            matched_index = Some(index);
        }
    }
    if let Some(index) = matched_index {
        sequence[index] = value;
    } else {
        sequence.push(value);
    }
    Ok(())
}

fn yaml_mapping_name(value: &Value) -> Option<&str> {
    yaml_mapping_string(value, "name")
}

fn yaml_mapping_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .as_mapping()?
        .get(Value::String(key.to_string()))?
        .as_str()
}

fn ordered_pair_key(first: &str, second: &str) -> String {
    if first <= second {
        format!("{first}/{second}")
    } else {
        format!("{second}/{first}")
    }
}

fn source_file_manifest(path: &Path) -> Result<SourceFileManifest> {
    Ok(SourceFileManifest {
        path: path.display().to_string(),
        size_bytes: fs::metadata(path)
            .with_context(|| format!("Failed to stat {}", path.display()))?
            .len(),
        sha256: file_sha256_hex(path)?,
    })
}

fn source_csv_manifest(path: &Path, parsed: &ParsedMetadata) -> Result<SourceCsvManifest> {
    Ok(SourceCsvManifest {
        path: path.display().to_string(),
        size_bytes: fs::metadata(path)
            .with_context(|| format!("Failed to stat {}", path.display()))?
            .len(),
        sha256: file_sha256_hex(path)?,
        columns: parsed.headers.clone(),
        data_rows: parsed.data_rows,
    })
}

fn file_sha256_hex(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("Failed to hash {}", path.display()))?;
    Ok(sha256_hex(&bytes))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn ensure_mapping_field_mut<'a>(mapping: &'a mut Mapping, key: &str) -> Result<&'a mut Mapping> {
    let key_value = Value::String(key.to_string());
    if !mapping.contains_key(&key_value) {
        mapping.insert(key_value.clone(), Value::Mapping(Mapping::new()));
    }
    mapping
        .get_mut(&key_value)
        .expect("field was inserted when absent")
        .as_mapping_mut()
        .with_context(|| format!("Board IR field {key} must be an object."))
}

fn ensure_sequence_field_mut<'a>(
    mapping: &'a mut Mapping,
    key: &str,
) -> Result<&'a mut Vec<Value>> {
    let key_value = Value::String(key.to_string());
    if !mapping.contains_key(&key_value) {
        mapping.insert(key_value.clone(), Value::Sequence(Vec::new()));
    }
    mapping
        .get_mut(&key_value)
        .expect("field was inserted when absent")
        .as_sequence_mut()
        .with_context(|| format!("Board IR field {key} must be a list."))
}

fn absolutize_relative_libraries(project_yaml: &mut Value, project_dir: &Path) -> Result<()> {
    let mapping = project_yaml
        .as_mapping_mut()
        .context("Board IR project must be a YAML object.")?;
    let Some(libraries) = mapping.get_mut(Value::String("libraries".to_string())) else {
        return Ok(());
    };
    let libraries = libraries
        .as_sequence_mut()
        .context("Board IR field libraries must be a list.")?;
    for library in libraries {
        let Some(path_text) = library.as_str() else {
            bail!("Board IR libraries entries must be strings.");
        };
        let path = Path::new(path_text);
        if path.is_absolute() {
            continue;
        }
        let resolved = normalize_path(&project_dir.join(path));
        let absolute = fs::canonicalize(&resolved).unwrap_or(resolved);
        *library = Value::String(absolute.to_string_lossy().to_string());
    }
    Ok(())
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}
