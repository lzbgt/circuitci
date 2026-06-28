use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const SQLITE_HEADER: &[u8] = b"SQLite format 3\0";

#[derive(Debug, Clone)]
pub struct EasyedaProInspectOptions {
    pub eprj2: PathBuf,
    pub output: PathBuf,
    pub manifest: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct EasyedaProInspectSummary {
    pub projects: usize,
    pub branches: usize,
    pub project_structures: usize,
    pub history_payloads: usize,
    pub encoded_history_payloads: usize,
    pub latest_ticket: Option<usize>,
    pub boards: usize,
    pub schematics: usize,
    pub sheets: usize,
    pub pcbs: usize,
    pub structure_objects: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
struct ProjectRow {
    uuid: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch_uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ticket: Option<usize>,
}

#[derive(Debug, Clone, Default, Serialize)]
struct BranchRow {
    uuid: String,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    history_uuid: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize)]
struct NamedObject {
    uuid: String,
    title: String,
}

#[derive(Debug, Clone, Default, Serialize)]
struct StructureSummary {
    ticket: usize,
    boards: Vec<NamedObject>,
    schematics: Vec<NamedObject>,
    sheets: Vec<NamedObject>,
    pcbs: Vec<NamedObject>,
    objects: Vec<StructureObjectManifest>,
}

#[derive(Debug, Serialize)]
struct InspectionManifest {
    schema_version: String,
    source: SourceManifest,
    sqlite: SqliteManifest,
    easyeda_pro: EasyedaProManifest,
    importability: ImportabilityManifest,
}

#[derive(Debug, Serialize)]
struct SourceManifest {
    path: String,
    size_bytes: u64,
    sha256: String,
}

#[derive(Debug, Serialize)]
struct SqliteManifest {
    tables: Vec<TableManifest>,
}

#[derive(Debug, Serialize)]
struct TableManifest {
    name: String,
    row_count: usize,
    columns: Vec<ColumnManifest>,
}

#[derive(Debug, Serialize)]
struct ColumnManifest {
    cid: usize,
    name: String,
    #[serde(rename = "type")]
    column_type: String,
    not_null: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_value: Option<String>,
    primary_key: bool,
}

#[derive(Debug, Serialize)]
struct EasyedaProManifest {
    projects: Vec<ProjectRow>,
    branches: Vec<BranchRow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latest_structure: Option<StructureManifest>,
    history_payloads: PayloadManifest,
}

#[derive(Debug, Serialize)]
struct StructureManifest {
    ticket: usize,
    sha256: String,
    length_bytes: usize,
    boards: Vec<NamedObject>,
    schematics: Vec<NamedObject>,
    sheets: Vec<NamedObject>,
    pcbs: Vec<NamedObject>,
    objects: Vec<StructureObjectManifest>,
}

#[derive(Debug, Clone, Serialize)]
struct StructureObjectManifest {
    kind: String,
    map_key: String,
    uuid: String,
    title: String,
    length_bytes: usize,
    sha256: String,
    field_names: Vec<String>,
    references: Vec<StructureReferenceManifest>,
}

#[derive(Debug, Clone, Serialize)]
struct StructureReferenceManifest {
    field: String,
    value: String,
}

#[derive(Debug, Serialize)]
struct PayloadManifest {
    total: usize,
    encoded_or_non_json: usize,
    max_length_bytes: usize,
    rows: Vec<PayloadRowManifest>,
}

#[derive(Debug, Serialize)]
struct PayloadRowManifest {
    id: usize,
    length_bytes: usize,
    sha256: String,
    looks_like_json: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    json_shape: Option<PayloadJsonShapeManifest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    json_parse_error: Option<String>,
}

#[derive(Debug, Serialize)]
struct PayloadJsonShapeManifest {
    kind: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    top_level_keys: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_level_item_count: Option<usize>,
}

#[derive(Debug, Serialize)]
struct ImportabilityManifest {
    status: String,
    notes: Vec<String>,
}

impl From<&StructureManifest> for StructureSummary {
    fn from(structure: &StructureManifest) -> Self {
        Self {
            ticket: structure.ticket,
            boards: structure.boards.clone(),
            schematics: structure.schematics.clone(),
            sheets: structure.sheets.clone(),
            pcbs: structure.pcbs.clone(),
            objects: structure.objects.clone(),
        }
    }
}

pub fn inspect_easyeda_pro_project(
    options: &EasyedaProInspectOptions,
) -> Result<EasyedaProInspectSummary> {
    ensure_sqlite_file(&options.eprj2)?;
    ensure_easyeda_tables(&options.eprj2)?;

    let projects = project_rows(&options.eprj2)?;
    let branches = branch_rows(&options.eprj2)?;
    let structure_count = scalar_usize(
        &options.eprj2,
        "SELECT count(*) FROM project_structures;",
        "project_structures count",
    )?;
    let latest_structure_manifest = latest_structure_manifest(&options.eprj2)?;
    let latest_structure = latest_structure_manifest
        .as_ref()
        .map(StructureSummary::from);
    let history_payloads = scalar_usize(
        &options.eprj2,
        "SELECT count(*) FROM history_data;",
        "history_data count",
    )?;
    let encoded_history_payloads = scalar_usize(
        &options.eprj2,
        "SELECT count(*) FROM history_data WHERE trim(dataStr) NOT LIKE '{%' AND trim(dataStr) NOT LIKE '[%';",
        "encoded history_data count",
    )?;
    let max_history_payload_len = scalar_usize(
        &options.eprj2,
        "SELECT coalesce(max(length(dataStr)), 0) FROM history_data;",
        "max history_data length",
    )?;
    let payload_rows = history_payload_rows(&options.eprj2)?;

    let summary = EasyedaProInspectSummary {
        projects: projects.len(),
        branches: branches.len(),
        project_structures: structure_count,
        history_payloads,
        encoded_history_payloads,
        latest_ticket: latest_structure.as_ref().map(|structure| structure.ticket),
        boards: latest_structure
            .as_ref()
            .map(|structure| structure.boards.len())
            .unwrap_or(0),
        schematics: latest_structure
            .as_ref()
            .map(|structure| structure.schematics.len())
            .unwrap_or(0),
        sheets: latest_structure
            .as_ref()
            .map(|structure| structure.sheets.len())
            .unwrap_or(0),
        pcbs: latest_structure
            .as_ref()
            .map(|structure| structure.pcbs.len())
            .unwrap_or(0),
        structure_objects: latest_structure
            .as_ref()
            .map(|structure| structure.objects.len())
            .unwrap_or(0),
    };

    if let Some(parent) = options.output.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create EasyEDA Pro inspection output directory {}",
                parent.display()
            )
        })?;
    }
    fs::write(
        &options.output,
        inspection_markdown(
            &options.eprj2,
            &projects,
            &branches,
            latest_structure.as_ref(),
            &payload_rows,
            &summary,
            max_history_payload_len,
        ),
    )
    .with_context(|| {
        format!(
            "Failed to write EasyEDA Pro inspection report {}",
            options.output.display()
        )
    })?;
    let manifest = InspectionManifest {
        schema_version: "0.3.0".to_string(),
        source: SourceManifest {
            path: options.eprj2.display().to_string(),
            size_bytes: fs::metadata(&options.eprj2)
                .with_context(|| format!("Failed to stat {}", options.eprj2.display()))?
                .len(),
            sha256: file_sha256_hex(&options.eprj2)?,
        },
        sqlite: SqliteManifest {
            tables: table_manifests(&options.eprj2)?,
        },
        easyeda_pro: EasyedaProManifest {
            projects,
            branches,
            latest_structure: latest_structure_manifest,
            history_payloads: PayloadManifest {
                total: history_payloads,
                encoded_or_non_json: encoded_history_payloads,
                max_length_bytes: max_history_payload_len,
                rows: payload_rows,
            },
        },
        importability: importability_manifest(encoded_history_payloads),
    };
    if let Some(parent) = options.manifest.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create EasyEDA Pro manifest output directory {}",
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
            "Failed to write EasyEDA Pro inspection manifest {}",
            options.manifest.display()
        )
    })?;

    Ok(summary)
}

fn ensure_sqlite_file(path: &Path) -> Result<()> {
    let bytes = fs::read(path)
        .with_context(|| format!("Failed to read EasyEDA Pro project {}", path.display()))?;
    if bytes.len() < SQLITE_HEADER.len() || &bytes[..SQLITE_HEADER.len()] != SQLITE_HEADER {
        bail!(
            "EasyEDA Pro project {} is not a SQLite 3 database.",
            path.display()
        );
    }
    Ok(())
}

fn ensure_easyeda_tables(path: &Path) -> Result<()> {
    for table in ["projects", "branches", "project_structures", "history_data"] {
        let exists = scalar_usize(
            path,
            &format!(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='{}';",
                table
            ),
            table,
        )?;
        if exists != 1 {
            bail!(
                "EasyEDA Pro project {} is missing required SQLite table {}.",
                path.display(),
                table
            );
        }
    }
    Ok(())
}

fn project_rows(path: &Path) -> Result<Vec<ProjectRow>> {
    sqlite_rows(
        path,
        "SELECT uuid, name, coalesce(branch_uuid, ''), ticket FROM projects ORDER BY name;",
    )?
    .into_iter()
    .map(|columns| {
        if columns.len() != 4 {
            bail!(
                "EasyEDA Pro projects query returned {} columns.",
                columns.len()
            );
        }
        Ok(ProjectRow {
            uuid: columns[0].clone(),
            name: columns[1].clone(),
            branch_uuid: non_empty(columns[2].clone()),
            ticket: columns[3].parse::<usize>().ok(),
        })
    })
    .collect()
}

fn branch_rows(path: &Path) -> Result<Vec<BranchRow>> {
    sqlite_rows(
        path,
        "SELECT uuid, name, coalesce(history_uuid, '') FROM branches ORDER BY id;",
    )?
    .into_iter()
    .map(|columns| {
        if columns.len() != 3 {
            bail!(
                "EasyEDA Pro branches query returned {} columns.",
                columns.len()
            );
        }
        Ok(BranchRow {
            uuid: columns[0].clone(),
            name: columns[1].clone(),
            history_uuid: non_empty(columns[2].clone()),
        })
    })
    .collect()
}

fn latest_structure_manifest(path: &Path) -> Result<Option<StructureManifest>> {
    let rows = sqlite_rows(
        path,
        "SELECT ticket, length(structure), hex(structure), structure FROM project_structures ORDER BY ticket DESC, id DESC LIMIT 1;",
    )?;
    let Some(columns) = rows.into_iter().next() else {
        return Ok(None);
    };
    if columns.len() != 4 {
        bail!(
            "EasyEDA Pro project_structures query returned {} columns.",
            columns.len()
        );
    }
    let ticket = columns[0]
        .parse::<usize>()
        .context("EasyEDA Pro latest project structure ticket is not an integer.")?;
    let length_bytes = columns[1]
        .parse::<usize>()
        .context("EasyEDA Pro latest project structure length is not an integer.")?;
    let structure_bytes = bytes_from_sqlite_hex(&columns[2], "latest project structure")?;
    let value: Value = serde_json::from_str(&columns[3])
        .context("EasyEDA Pro latest project structure is not valid JSON.")?;
    Ok(Some(StructureManifest {
        ticket,
        sha256: sha256_hex(&structure_bytes),
        length_bytes,
        boards: named_objects(&value, "boards", "title"),
        schematics: named_objects(&value, "schematics", "name"),
        sheets: named_objects(&value, "sheets", "title"),
        pcbs: named_objects(&value, "pcbs", "title"),
        objects: structure_object_manifests(&value)?,
    }))
}

fn named_objects(value: &Value, key: &str, name_key: &str) -> Vec<NamedObject> {
    let mut objects = value
        .get(key)
        .and_then(Value::as_object)
        .map(|entries| {
            entries
                .iter()
                .map(|(uuid, object)| NamedObject {
                    uuid: object
                        .get("uuid")
                        .and_then(Value::as_str)
                        .unwrap_or(uuid)
                        .to_string(),
                    title: object
                        .get(name_key)
                        .or_else(|| object.get("title"))
                        .or_else(|| object.get("name"))
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    objects.sort_by(|left, right| {
        left.title
            .cmp(&right.title)
            .then_with(|| left.uuid.cmp(&right.uuid))
    });
    objects
}

fn structure_object_manifests(value: &Value) -> Result<Vec<StructureObjectManifest>> {
    let mut objects = Vec::new();
    for (container_key, kind, name_key) in [
        ("boards", "board", "title"),
        ("schematics", "schematic", "name"),
        ("sheets", "sheet", "title"),
        ("pcbs", "pcb", "title"),
    ] {
        let Some(entries) = value.get(container_key).and_then(Value::as_object) else {
            continue;
        };
        for (map_key, object) in entries {
            let bytes = serde_json::to_vec(object)
                .with_context(|| format!("Failed to canonicalize EasyEDA Pro {kind} object."))?;
            let object_map = object.as_object();
            let uuid = object_map
                .and_then(|entries| entries.get("uuid"))
                .and_then(Value::as_str)
                .unwrap_or(map_key)
                .to_string();
            let title = object_map
                .and_then(|entries| entries.get(name_key))
                .or_else(|| object_map.and_then(|entries| entries.get("title")))
                .or_else(|| object_map.and_then(|entries| entries.get("name")))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let mut field_names = object_map
                .map(|entries| entries.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default();
            field_names.sort();
            let mut references = object_map
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(|(field, value)| {
                            let lower = field.to_ascii_lowercase();
                            if field == "uuid" || !lower.contains("uuid") {
                                return None;
                            }
                            value.as_str().and_then(|reference| {
                                (!reference.trim().is_empty()).then(|| StructureReferenceManifest {
                                    field: field.clone(),
                                    value: reference.trim().to_string(),
                                })
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            references.sort_by(|left, right| {
                left.field
                    .cmp(&right.field)
                    .then_with(|| left.value.cmp(&right.value))
            });
            objects.push(StructureObjectManifest {
                kind: kind.to_string(),
                map_key: map_key.clone(),
                uuid,
                title,
                length_bytes: bytes.len(),
                sha256: sha256_hex(&bytes),
                field_names,
                references,
            });
        }
    }
    Ok(objects)
}

fn sqlite_rows(path: &Path, sql: &str) -> Result<Vec<Vec<String>>> {
    let output = Command::new("sqlite3")
        .arg("-batch")
        .arg("-noheader")
        .arg("-separator")
        .arg("\x1f")
        .arg(path)
        .arg(sql)
        .output()
        .with_context(
            || "Failed to run sqlite3; install sqlite3 to inspect EasyEDA Pro projects.",
        )?;
    if !output.status.success() {
        bail!(
            "sqlite3 failed while inspecting {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let stdout = String::from_utf8(output.stdout).context("sqlite3 emitted non-UTF-8 output.")?;
    Ok(stdout
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.split('\x1f').map(str::to_string).collect())
        .collect())
}

fn scalar_usize(path: &Path, sql: &str, label: &str) -> Result<usize> {
    let rows = sqlite_rows(path, sql)?;
    let value = rows
        .first()
        .and_then(|row| row.first())
        .with_context(|| format!("EasyEDA Pro SQLite query returned no value for {label}."))?;
    value
        .parse::<usize>()
        .with_context(|| format!("EasyEDA Pro SQLite query returned non-integer {label}: {value}."))
}

fn table_manifests(path: &Path) -> Result<Vec<TableManifest>> {
    let mut tables = Vec::new();
    for row in sqlite_rows(
        path,
        "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name;",
    )? {
        let Some(name) = row.first() else {
            continue;
        };
        let row_count = scalar_usize(
            path,
            &format!("SELECT count(*) FROM \"{}\";", sqlite_identifier(name)),
            &format!("{name} row count"),
        )?;
        tables.push(TableManifest {
            name: name.clone(),
            row_count,
            columns: column_manifests(path, name)?,
        });
    }
    Ok(tables)
}

fn column_manifests(path: &Path, table: &str) -> Result<Vec<ColumnManifest>> {
    sqlite_rows(
        path,
        &format!("PRAGMA table_info(\"{}\");", sqlite_identifier(table)),
    )?
    .into_iter()
    .map(|columns| {
        if columns.len() != 6 {
            bail!(
                "EasyEDA Pro PRAGMA table_info({table}) returned {} columns.",
                columns.len()
            );
        }
        Ok(ColumnManifest {
            cid: columns[0].parse::<usize>().with_context(|| {
                format!("EasyEDA Pro table {table} column cid is not an integer.")
            })?,
            name: columns[1].clone(),
            column_type: columns[2].clone(),
            not_null: columns[3] == "1",
            default_value: non_empty(columns[4].clone()),
            primary_key: columns[5] == "1",
        })
    })
    .collect()
}

fn history_payload_rows(path: &Path) -> Result<Vec<PayloadRowManifest>> {
    sqlite_rows(
        path,
        "SELECT id, length(dataStr), hex(dataStr), CASE WHEN trim(dataStr) LIKE '{%' OR trim(dataStr) LIKE '[%' THEN 1 ELSE 0 END, dataStr FROM history_data ORDER BY id;",
    )?
    .into_iter()
    .map(|columns| {
        if columns.len() != 5 {
            bail!(
                "EasyEDA Pro history_data payload query returned {} columns.",
                columns.len()
            );
        }
        let id = columns[0]
            .parse::<usize>()
            .context("EasyEDA Pro history_data id is not an integer.")?;
        let length_bytes = columns[1]
            .parse::<usize>()
            .context("EasyEDA Pro history_data payload length is not an integer.")?;
        let bytes = bytes_from_sqlite_hex(&columns[2], "history_data payload")?;
        let looks_like_json = columns[3] == "1";
        let (json_shape, json_parse_error) = if looks_like_json {
            match payload_json_shape(&columns[4]) {
                Ok(shape) => (Some(shape), None),
                Err(error) => (None, Some(error)),
            }
        } else {
            (None, None)
        };
        Ok(PayloadRowManifest {
            id,
            length_bytes,
            sha256: sha256_hex(&bytes),
            looks_like_json,
            json_shape,
            json_parse_error,
        })
    })
    .collect()
}

fn payload_json_shape(payload: &str) -> std::result::Result<PayloadJsonShapeManifest, String> {
    let value: Value = serde_json::from_str(payload.trim()).map_err(|error| error.to_string())?;
    let (kind, mut top_level_keys, top_level_item_count) = match &value {
        Value::Object(map) => (
            "object",
            map.keys().cloned().collect::<Vec<_>>(),
            Some(map.len()),
        ),
        Value::Array(values) => ("array", Vec::new(), Some(values.len())),
        Value::String(_) => ("string", Vec::new(), None),
        Value::Number(_) => ("number", Vec::new(), None),
        Value::Bool(_) => ("boolean", Vec::new(), None),
        Value::Null => ("null", Vec::new(), None),
    };
    top_level_keys.sort();
    Ok(PayloadJsonShapeManifest {
        kind: kind.to_string(),
        top_level_keys,
        top_level_item_count,
    })
}

fn importability_manifest(encoded_history_payloads: usize) -> ImportabilityManifest {
    if encoded_history_payloads > 0 {
        ImportabilityManifest {
            status: "blocked_encoded_history_payloads".to_string(),
            notes: vec![
                "Project structure metadata is plaintext JSON.".to_string(),
                "At least one design-object history payload is encoded or non-JSON.".to_string(),
                "CircuitCI will not infer pad, route, zone, via, or net geometry from opaque payloads.".to_string(),
            ],
        }
    } else {
        ImportabilityManifest {
            status: "plaintext_history_payloads_possible".to_string(),
            notes: vec![
                "History payload prefixes look like plaintext JSON.".to_string(),
                "A future adapter can inspect payload object shapes before converting geometry."
                    .to_string(),
            ],
        }
    }
}

fn sqlite_identifier(identifier: &str) -> String {
    identifier.replace('"', "\"\"")
}

fn file_sha256_hex(path: &Path) -> Result<String> {
    let bytes = fs::read(path)
        .with_context(|| format!("Failed to hash EasyEDA Pro project {}", path.display()))?;
    Ok(sha256_hex(&bytes))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn bytes_from_sqlite_hex(hex: &str, label: &str) -> Result<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        bail!("EasyEDA Pro SQLite hex payload for {label} has odd length.");
    }
    (0..hex.len())
        .step_by(2)
        .map(|index| {
            u8::from_str_radix(&hex[index..index + 2], 16)
                .with_context(|| format!("EasyEDA Pro SQLite hex payload for {label} is invalid."))
        })
        .collect()
}

fn non_empty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

fn inspection_markdown(
    path: &Path,
    projects: &[ProjectRow],
    branches: &[BranchRow],
    latest_structure: Option<&StructureSummary>,
    payload_rows: &[PayloadRowManifest],
    summary: &EasyedaProInspectSummary,
    max_history_payload_len: usize,
) -> String {
    let mut markdown = String::new();
    markdown.push_str("# EasyEDA Pro Project Inspection\n\n");
    markdown.push_str(&format!("- Source: `{}`\n", path.display()));
    markdown.push_str(&format!("- Projects: `{}`\n", summary.projects));
    markdown.push_str(&format!("- Branches: `{}`\n", summary.branches));
    markdown.push_str(&format!(
        "- Project structure snapshots: `{}`\n",
        summary.project_structures
    ));
    markdown.push_str(&format!(
        "- History payloads: `{}` total, `{}` encoded/non-JSON, max payload length `{}` bytes\n\n",
        summary.history_payloads, summary.encoded_history_payloads, max_history_payload_len
    ));

    markdown.push_str("## Projects\n\n");
    for project in projects {
        markdown.push_str(&format!(
            "- `{}`: `{}`",
            project.uuid,
            project.name.replace('`', "'")
        ));
        if let Some(branch_uuid) = &project.branch_uuid {
            markdown.push_str(&format!(", branch `{branch_uuid}`"));
        }
        if let Some(ticket) = project.ticket {
            markdown.push_str(&format!(", ticket `{ticket}`"));
        }
        markdown.push('\n');
    }
    if projects.is_empty() {
        markdown.push_str("- No rows in `projects`.\n");
    }

    markdown.push_str("\n## Branches\n\n");
    for branch in branches {
        markdown.push_str(&format!(
            "- `{}`: `{}`",
            branch.uuid,
            branch.name.replace('`', "'")
        ));
        if let Some(history_uuid) = &branch.history_uuid {
            markdown.push_str(&format!(", history `{history_uuid}`"));
        }
        markdown.push('\n');
    }
    if branches.is_empty() {
        markdown.push_str("- No rows in `branches`.\n");
    }

    markdown.push_str("\n## Latest Structure\n\n");
    if let Some(structure) = latest_structure {
        markdown.push_str(&format!("- Ticket: `{}`\n", structure.ticket));
        append_named_objects(&mut markdown, "Boards", &structure.boards);
        append_named_objects(&mut markdown, "Schematics", &structure.schematics);
        append_named_objects(&mut markdown, "Sheets", &structure.sheets);
        append_named_objects(&mut markdown, "PCBs", &structure.pcbs);
        append_structure_object_evidence(&mut markdown, &structure.objects);
    } else {
        markdown.push_str("- No rows in `project_structures`.\n");
    }

    append_payload_shape_evidence(&mut markdown, payload_rows);

    markdown.push_str("\n## Importability\n\n");
    if summary.encoded_history_payloads > 0 {
        markdown.push_str(
            "The project structure metadata is plaintext JSON, but design-object history payloads are encoded/non-JSON in this `.eprj2` file. CircuitCI therefore treats pad, via, route, zone, and net geometry as unavailable from this source until an exported unencoded EasyEDA layout artifact or a documented decoder is provided.\n",
        );
    } else {
        markdown.push_str(
            "History payloads look like plaintext JSON by prefix. A future importer can inspect them for pad, via, route, zone, and net geometry.\n",
        );
    }
    markdown
}

fn append_payload_shape_evidence(markdown: &mut String, rows: &[PayloadRowManifest]) {
    markdown.push_str("\n## History Payload Shapes\n\n");
    if rows.is_empty() {
        markdown.push_str("- No rows in `history_data`.\n");
        return;
    }
    for row in rows {
        markdown.push_str(&format!(
            "- `{}`: `{}` bytes; sha256 `{}`",
            row.id, row.length_bytes, row.sha256
        ));
        if let Some(shape) = &row.json_shape {
            markdown.push_str(&format!("; JSON `{}`", shape.kind));
            if let Some(count) = shape.top_level_item_count {
                markdown.push_str(&format!("; top-level items `{count}`"));
            }
            if !shape.top_level_keys.is_empty() {
                markdown.push_str(&format!("; keys `{}`", shape.top_level_keys.join("`, `")));
            }
        } else if let Some(error) = &row.json_parse_error {
            markdown.push_str(&format!(
                "; JSON-prefix parse error `{}`",
                error.replace('`', "'")
            ));
        } else {
            markdown.push_str("; encoded/non-JSON prefix");
        }
        markdown.push('\n');
    }
}

fn append_structure_object_evidence(markdown: &mut String, objects: &[StructureObjectManifest]) {
    markdown.push_str("\n### Object Evidence\n\n");
    if objects.is_empty() {
        markdown.push_str("- None.\n");
        return;
    }
    for object in objects {
        markdown.push_str(&format!(
            "- `{}` `{}`: `{}`; `{}` bytes; sha256 `{}`; fields `{}`",
            object.kind,
            object.uuid,
            object.title.replace('`', "'"),
            object.length_bytes,
            object.sha256,
            object.field_names.len()
        ));
        if !object.references.is_empty() {
            let references = object
                .references
                .iter()
                .map(|reference| format!("{}={}", reference.field, reference.value))
                .collect::<Vec<_>>()
                .join(", ");
            markdown.push_str(&format!("; references `{}`", references.replace('`', "'")));
        }
        markdown.push('\n');
    }
}

fn append_named_objects(markdown: &mut String, title: &str, objects: &[NamedObject]) {
    markdown.push_str(&format!("\n### {title}\n\n"));
    if objects.is_empty() {
        markdown.push_str("- None.\n");
        return;
    }
    for object in objects {
        markdown.push_str(&format!(
            "- `{}`: `{}`\n",
            object.uuid,
            object.title.replace('`', "'")
        ));
    }
}
