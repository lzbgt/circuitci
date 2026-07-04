use super::Finding;
use serde_json::Value;

pub(super) fn push_findings(text: &mut String, findings: &[Finding]) {
    if findings.is_empty() {
        text.push_str("None.\n\n");
        return;
    }
    for finding in findings {
        text.push_str(&format!("- `{}`: {}\n", finding.id, finding.message));
        if let Some(blocker) = text_field(&finding.measured, "adapter_blocker") {
            text.push_str(&format!("  - Backend blocker: {blocker}\n"));
        }
        let evidence = string_array_field(&finding.measured, "evidence_sources");
        if !evidence.is_empty() {
            let evidence = evidence
                .iter()
                .map(|source| format!("`{source}`"))
                .collect::<Vec<_>>()
                .join(", ");
            text.push_str(&format!("  - Evidence: {evidence}\n"));
        }
        for fix in &finding.suggested_fixes {
            text.push_str(&format!("  - Fix: {fix}\n"));
        }
    }
    text.push('\n');
}

fn text_field(map: &std::collections::BTreeMap<String, Value>, name: &str) -> Option<String> {
    map.get(name)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn string_array_field(map: &std::collections::BTreeMap<String, Value>, name: &str) -> Vec<String> {
    map.get(name)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::push_findings;
    use crate::reports::Finding;
    use serde_json::json;

    #[test]
    fn push_findings_surfaces_adapter_blocker_evidence() {
        let mut finding = Finding::critical(
            "SPICE_PERIODIC_AC_ANALYSIS",
            "pac_case",
            "Periodic AC backend is planned.",
        );
        finding.measured.insert(
            "adapter_blocker".to_string(),
            json!("No trusted PAC/PXF backend path is available."),
        );
        finding.measured.insert(
            "evidence_sources".to_string(),
            json!([
                "docs/research/circuit_simulation_full_featured/periodic_ac_backend_evidence.md"
            ]),
        );
        let mut markdown = String::new();

        push_findings(&mut markdown, &[finding]);

        assert!(markdown.contains("Backend blocker: No trusted PAC/PXF backend path"));
        assert!(markdown.contains(
            "`docs/research/circuit_simulation_full_featured/periodic_ac_backend_evidence.md`"
        ));
    }
}
