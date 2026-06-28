use super::normalize_name;
use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

pub(crate) struct ParsedMetadata {
    pub(crate) headers: Vec<String>,
    pub(crate) data_rows: usize,
    pub(super) rows: Vec<MetadataCsvRow>,
}

#[derive(Debug, Clone)]
pub(super) struct MetadataCsvRow {
    pub(super) row_number: usize,
    pub(super) field: String,
    pub(super) value: String,
    pub(super) unit: Option<String>,
    pub(super) source: Option<String>,
    pub(super) notes: Option<String>,
    pub(super) raw_fields: BTreeMap<String, String>,
}

pub(crate) fn parse_metadata_csv(path: &Path) -> Result<ParsedMetadata> {
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

fn cell(row: &[String], index: usize) -> &str {
    row.get(index).map(String::as_str).unwrap_or("").trim()
}

fn optional_string(row: &[String], index: Option<usize>) -> Option<String> {
    index
        .map(|index| cell(row, index))
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}
