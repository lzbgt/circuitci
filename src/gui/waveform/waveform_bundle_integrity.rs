use super::waveform_snapshots::markdown_escape;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub(super) const SCOPE_REPORT_BUNDLE_ARTIFACTS: [(&str, &str); 10] = [
    ("index.html", "index.html"),
    ("scope_plot.svg", "scope_plot.svg"),
    ("measurement_snapshots.csv", "measurement_snapshots.csv"),
    ("measurement_snapshots.md", "measurement_snapshots.md"),
    ("operating_points.csv", "operating_points.csv"),
    ("operating_points.md", "operating_points.md"),
    ("sweep_margin_summaries.csv", "sweep_margin_summaries.csv"),
    ("sweep_margin_summaries.md", "sweep_margin_summaries.md"),
    ("README.md", "README.md"),
    ("artifact_manifest.csv", "artifact_manifest.csv"),
];
pub(super) const SCOPE_REPORT_BUNDLE_INTEGRITY_DETAILS_CSV: &str = "artifact_integrity_details.csv";
pub(super) const SCOPE_REPORT_BUNDLE_INTEGRITY_DETAILS_MARKDOWN: &str =
    "artifact_integrity_details.md";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ScopeReportBundleArtifactMetadata {
    label: &'static str,
    size_bytes: usize,
    sha256: String,
}

pub(super) struct ScopeReportBundleArtifactStatus {
    missing: Vec<&'static str>,
    changed: Vec<String>,
    integrity_error: Option<String>,
}

impl ScopeReportBundleArtifactStatus {
    pub(super) fn needs_refresh(&self) -> bool {
        !self.missing.is_empty() || !self.changed.is_empty() || self.integrity_error.is_some()
    }

    pub(super) fn label(&self) -> String {
        if !self.missing.is_empty() {
            format!("Missing: {}", self.missing.join(", "))
        } else if !self.changed.is_empty() {
            format!("Changed: {}", self.changed.join(", "))
        } else if let Some(error) = &self.integrity_error {
            format!("Integrity unavailable: {error}")
        } else {
            "Artifacts OK".to_string()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ScopeReportBundleArtifactDetail {
    pub(super) path: String,
    pub(super) label: String,
    pub(super) state: ScopeReportBundleArtifactState,
    pub(super) expected_size: Option<usize>,
    pub(super) current_size: Option<usize>,
    pub(super) expected_sha256: Option<String>,
    pub(super) current_sha256: Option<String>,
}

#[derive(Clone, Debug)]
struct ScopeReportBundleExpectedArtifact {
    label: String,
    size_bytes: usize,
    sha256: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ScopeReportBundleArtifactState {
    Ok,
    Missing,
    Changed,
    Untracked,
}

impl ScopeReportBundleArtifactState {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Missing => "Missing",
            Self::Changed => "Changed",
            Self::Untracked => "Untracked",
        }
    }

    pub(super) fn is_problem(self) -> bool {
        !matches!(self, Self::Ok)
    }
}

pub(super) struct ScopeReportBundleIntegrityDetails {
    pub(super) rows: Vec<ScopeReportBundleArtifactDetail>,
    pub(super) manifest_error: Option<String>,
}

pub(super) fn html_escape(value: &str) -> String {
    let value = value.trim();
    if value.is_empty() {
        return "-".to_string();
    }
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub(super) fn scope_report_bundle_content_metadata(
    scope_plot_svg: &str,
    measurement_snapshots_csv: &str,
    measurement_snapshots_markdown: &str,
    operating_points_csv: &str,
    operating_points_markdown: &str,
    sweep_margin_summaries_csv: &str,
    sweep_margin_summaries_markdown: &str,
) -> Vec<ScopeReportBundleArtifactMetadata> {
    vec![
        artifact_metadata_for_bytes("scope_plot.svg", scope_plot_svg.as_bytes()),
        artifact_metadata_for_bytes(
            "measurement_snapshots.csv",
            measurement_snapshots_csv.as_bytes(),
        ),
        artifact_metadata_for_bytes(
            "measurement_snapshots.md",
            measurement_snapshots_markdown.as_bytes(),
        ),
        artifact_metadata_for_bytes("operating_points.csv", operating_points_csv.as_bytes()),
        artifact_metadata_for_bytes("operating_points.md", operating_points_markdown.as_bytes()),
        artifact_metadata_for_bytes(
            "sweep_margin_summaries.csv",
            sweep_margin_summaries_csv.as_bytes(),
        ),
        artifact_metadata_for_bytes(
            "sweep_margin_summaries.md",
            sweep_margin_summaries_markdown.as_bytes(),
        ),
    ]
}

fn artifact_metadata_for_bytes(
    label: &'static str,
    bytes: &[u8],
) -> ScopeReportBundleArtifactMetadata {
    ScopeReportBundleArtifactMetadata {
        label,
        size_bytes: bytes.len(),
        sha256: sha256_hex(bytes),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

pub(super) fn scope_report_bundle_artifact_metadata_markdown(
    metadata: &[ScopeReportBundleArtifactMetadata],
) -> String {
    let mut markdown = String::from("| Artifact | Size bytes | SHA-256 |\n| --- | ---: | --- |\n");
    for artifact in metadata {
        markdown.push_str(&format!(
            "| {} | {} | `{}` |\n",
            markdown_escape(artifact.label),
            artifact.size_bytes,
            artifact.sha256
        ));
    }
    markdown
}

pub(super) fn scope_report_bundle_artifact_metadata_html(
    metadata: &[ScopeReportBundleArtifactMetadata],
) -> String {
    let mut html = String::from(
        "\
<h2>Artifact Metadata</h2>
<p><code>artifact_manifest.csv</code> records expected size and SHA-256 metadata for required bundle artifacts after export.</p>
<table>
  <thead>
    <tr><th>Artifact</th><th>Size bytes</th><th>SHA-256</th></tr>
  </thead>
  <tbody>
",
    );
    for artifact in metadata {
        html.push_str(&format!(
            "    <tr><td>{}</td><td class=\"number\">{}</td><td><code>{}</code></td></tr>\n",
            html_escape(artifact.label),
            artifact.size_bytes,
            html_escape(&artifact.sha256)
        ));
    }
    html.push_str("  </tbody>\n</table>");
    html
}

pub(super) fn scope_report_bundle_integrity_details_csv(
    details: &ScopeReportBundleIntegrityDetails,
) -> String {
    let mut csv = String::from(
        "artifact,state,expected_size_bytes,current_size_bytes,expected_sha256,current_sha256,path\n",
    );
    for row in &details.rows {
        let fields = [
            row.label.clone(),
            row.state.label().to_string(),
            optional_size_label(row.expected_size),
            optional_size_label(row.current_size),
            row.expected_sha256
                .clone()
                .unwrap_or_else(|| "-".to_string()),
            row.current_sha256
                .clone()
                .unwrap_or_else(|| "-".to_string()),
            row.path.clone(),
        ];
        csv.push_str(
            &fields
                .into_iter()
                .map(integrity_detail_csv_escape)
                .collect::<Vec<_>>()
                .join(","),
        );
        csv.push('\n');
    }
    if let Some(error) = &details.manifest_error {
        csv.push_str(&format!(
            "{},{},,,,,{}\n",
            integrity_detail_csv_escape("Manifest".to_string()),
            integrity_detail_csv_escape("Error".to_string()),
            integrity_detail_csv_escape(error.clone())
        ));
    }
    csv
}

pub(super) fn scope_report_bundle_integrity_details_markdown(
    details: &ScopeReportBundleIntegrityDetails,
) -> String {
    let mut markdown = String::from(
        "| Artifact | State | Expected size bytes | Current size bytes | Expected SHA-256 | Current SHA-256 | Path |\n| --- | --- | ---: | ---: | --- | --- | --- |\n",
    );
    for row in &details.rows {
        markdown.push_str(&format!(
            "| {} | {} | {} | {} | `{}` | `{}` | {} |\n",
            markdown_escape(&row.label),
            markdown_escape(row.state.label()),
            markdown_escape(&optional_size_label(row.expected_size)),
            markdown_escape(&optional_size_label(row.current_size)),
            markdown_escape(row.expected_sha256.as_deref().unwrap_or("-")),
            markdown_escape(row.current_sha256.as_deref().unwrap_or("-")),
            markdown_escape(&row.path)
        ));
    }
    if let Some(error) = &details.manifest_error {
        markdown.push_str(&format!(
            "\nManifest integrity metadata was unavailable: {}\n",
            markdown_escape(error)
        ));
    }
    markdown
}

pub(super) fn scope_report_bundle_artifact_manifest_csv(
    bundle_dir: &Path,
) -> std::io::Result<String> {
    let mut csv = String::from("path,label,size_bytes,sha256\n");
    for (path, label) in SCOPE_REPORT_BUNDLE_ARTIFACTS
        .iter()
        .filter(|(path, _)| *path != "artifact_manifest.csv")
    {
        let artifact_path = bundle_dir.join(path);
        let bytes = fs::read(&artifact_path)?;
        csv.push_str(&format!(
            "{},{},{},{}\n",
            manifest_csv_escape(path),
            manifest_csv_escape(label),
            bytes.len(),
            sha256_hex(&bytes)
        ));
    }
    Ok(csv)
}

fn manifest_csv_escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

fn integrity_detail_csv_escape(value: String) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value
    }
}

fn read_scope_report_bundle_manifest(
    bundle_dir: &Path,
) -> std::io::Result<BTreeMap<String, ScopeReportBundleExpectedArtifact>> {
    let manifest_path = bundle_dir.join("artifact_manifest.csv");
    let manifest = fs::read_to_string(manifest_path)?;
    let mut expected = BTreeMap::new();
    for line in manifest.lines().skip(1) {
        let columns = parse_manifest_csv_line(line);
        if columns.len() != 4 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "artifact manifest row must have four columns",
            ));
        }
        let size_bytes = columns[2].parse::<usize>().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid artifact size for {}", columns[0]),
            )
        })?;
        expected.insert(
            columns[0].clone(),
            ScopeReportBundleExpectedArtifact {
                label: if columns[1].is_empty() {
                    columns[0].clone()
                } else {
                    columns[1].clone()
                },
                size_bytes,
                sha256: columns[3].trim().to_string(),
            },
        );
    }
    Ok(expected)
}

pub(super) fn scope_report_bundle_integrity_details(
    bundle_dir: &Path,
) -> ScopeReportBundleIntegrityDetails {
    let expected = read_scope_report_bundle_manifest(bundle_dir);
    let manifest_error = expected.as_ref().err().map(ToString::to_string);
    let expected = expected.unwrap_or_default();
    let rows = SCOPE_REPORT_BUNDLE_ARTIFACTS
        .iter()
        .map(|(path, fallback_label)| {
            let expected_artifact = expected.get(*path);
            let current = fs::read(bundle_dir.join(path)).ok();
            let current_size = current.as_ref().map(Vec::len);
            let current_sha256 = current.as_deref().map(sha256_hex);
            let state = match (&current, expected_artifact) {
                (None, _) => ScopeReportBundleArtifactState::Missing,
                (Some(_), None) => ScopeReportBundleArtifactState::Untracked,
                (Some(_), Some(expected)) => {
                    if current_size == Some(expected.size_bytes)
                        && current_sha256
                            .as_deref()
                            .is_some_and(|sha| sha.eq_ignore_ascii_case(&expected.sha256))
                    {
                        ScopeReportBundleArtifactState::Ok
                    } else {
                        ScopeReportBundleArtifactState::Changed
                    }
                }
            };
            ScopeReportBundleArtifactDetail {
                path: (*path).to_string(),
                label: expected_artifact
                    .map(|artifact| artifact.label.clone())
                    .unwrap_or_else(|| (*fallback_label).to_string()),
                state,
                expected_size: expected_artifact.map(|artifact| artifact.size_bytes),
                current_size,
                expected_sha256: expected_artifact.map(|artifact| artifact.sha256.clone()),
                current_sha256,
            }
        })
        .collect();
    ScopeReportBundleIntegrityDetails {
        rows,
        manifest_error,
    }
}

pub(super) fn scope_report_bundle_integrity_projected_details(
    details: &ScopeReportBundleIntegrityDetails,
    problems_only: bool,
) -> ScopeReportBundleIntegrityDetails {
    let rows = details
        .rows
        .iter()
        .filter(|row| !problems_only || row.state.is_problem())
        .cloned()
        .collect();
    ScopeReportBundleIntegrityDetails {
        rows,
        manifest_error: details.manifest_error.clone(),
    }
}

#[cfg(test)]
type ArtifactDetailRowTuple = (
    String,
    String,
    Option<usize>,
    Option<usize>,
    Option<String>,
    Option<String>,
);

#[cfg(test)]
pub(super) fn scope_report_bundle_artifact_detail_rows(
    bundle_dir: &Path,
) -> Vec<ArtifactDetailRowTuple> {
    scope_report_bundle_integrity_details(bundle_dir)
        .rows
        .into_iter()
        .map(|row| {
            (
                row.label,
                row.state.label().to_string(),
                row.expected_size,
                row.current_size,
                row.expected_sha256,
                row.current_sha256,
            )
        })
        .collect()
}

pub(super) fn scope_report_bundle_missing_artifacts(bundle_dir: &Path) -> Vec<&'static str> {
    if !bundle_dir.is_dir() {
        return vec!["bundle folder"];
    }
    SCOPE_REPORT_BUNDLE_ARTIFACTS
        .iter()
        .filter_map(|(path, label)| {
            let artifact = bundle_dir.join(path);
            (!artifact.is_file()).then_some(*label)
        })
        .collect()
}

pub(super) fn scope_report_bundle_changed_artifacts(
    bundle_dir: &Path,
) -> std::io::Result<Vec<String>> {
    let details = scope_report_bundle_integrity_details(bundle_dir);
    if let Some(error) = details.manifest_error {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, error));
    }
    let mut changed = details
        .rows
        .iter()
        .filter(|row| row.state == ScopeReportBundleArtifactState::Changed)
        .map(|row| row.label.clone())
        .collect::<Vec<_>>();
    changed.sort();
    changed.dedup();
    Ok(changed)
}

fn parse_manifest_csv_line(line: &str) -> Vec<String> {
    let mut columns = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut quoted = false;
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                current.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => {
                columns.push(std::mem::take(&mut current));
            }
            _ => current.push(ch),
        }
    }
    columns.push(current);
    columns
}

pub(super) fn optional_size_label(size: Option<usize>) -> String {
    size.map(|bytes| bytes.to_string())
        .unwrap_or_else(|| "-".to_string())
}

pub(super) fn short_optional_sha(sha: Option<&str>) -> String {
    sha.map(|value| value.chars().take(16).collect())
        .unwrap_or_else(|| "-".to_string())
}

pub(super) fn scope_report_bundle_artifact_status(
    bundle_dir: &Path,
) -> ScopeReportBundleArtifactStatus {
    let missing = scope_report_bundle_missing_artifacts(bundle_dir);
    if !missing.is_empty() {
        return ScopeReportBundleArtifactStatus {
            missing,
            changed: Vec::new(),
            integrity_error: None,
        };
    }
    match scope_report_bundle_changed_artifacts(bundle_dir) {
        Ok(changed) => ScopeReportBundleArtifactStatus {
            missing,
            changed,
            integrity_error: None,
        },
        Err(error) => ScopeReportBundleArtifactStatus {
            missing,
            changed: Vec::new(),
            integrity_error: Some(error.to_string()),
        },
    }
}
