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
        schema_version: "0.1.0".to_string(),
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
        if !seen_supported.insert(field) {
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
    Ok(())
}

fn normalized_yaml_value(field: &AppliedField) -> Result<Value> {
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
        manufacturing.insert(
            Value::String(field.field.board_key().to_string()),
            normalized_yaml_value(field)?,
        );
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
