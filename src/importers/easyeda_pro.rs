use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const SQLITE_HEADER: &[u8] = b"SQLite format 3\0";

#[derive(Debug, Clone)]
pub struct EasyedaProInspectOptions {
    pub eprj2: PathBuf,
    pub output: PathBuf,
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
}

#[derive(Debug, Clone, Default)]
struct ProjectRow {
    uuid: String,
    name: String,
    branch_uuid: Option<String>,
    ticket: Option<usize>,
}

#[derive(Debug, Clone, Default)]
struct BranchRow {
    uuid: String,
    name: String,
    history_uuid: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct NamedObject {
    uuid: String,
    title: String,
}

#[derive(Debug, Clone, Default)]
struct StructureSummary {
    ticket: usize,
    boards: Vec<NamedObject>,
    schematics: Vec<NamedObject>,
    sheets: Vec<NamedObject>,
    pcbs: Vec<NamedObject>,
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
    let latest_structure = latest_structure(&options.eprj2)?;
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

fn latest_structure(path: &Path) -> Result<Option<StructureSummary>> {
    let rows = sqlite_rows(
        path,
        "SELECT ticket, structure FROM project_structures ORDER BY ticket DESC, id DESC LIMIT 1;",
    )?;
    let Some(columns) = rows.into_iter().next() else {
        return Ok(None);
    };
    if columns.len() != 2 {
        bail!(
            "EasyEDA Pro project_structures query returned {} columns.",
            columns.len()
        );
    }
    let ticket = columns[0]
        .parse::<usize>()
        .context("EasyEDA Pro latest project structure ticket is not an integer.")?;
    let value: Value = serde_json::from_str(&columns[1])
        .context("EasyEDA Pro latest project structure is not valid JSON.")?;
    Ok(Some(StructureSummary {
        ticket,
        boards: named_objects(&value, "boards", "title"),
        schematics: named_objects(&value, "schematics", "name"),
        sheets: named_objects(&value, "sheets", "title"),
        pcbs: named_objects(&value, "pcbs", "title"),
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

fn non_empty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

fn inspection_markdown(
    path: &Path,
    projects: &[ProjectRow],
    branches: &[BranchRow],
    latest_structure: Option<&StructureSummary>,
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
    } else {
        markdown.push_str("- No rows in `project_structures`.\n");
    }

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
