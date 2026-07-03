use crate::board_ir::Scenario;
use crate::reports::Finding;
use serde_json::json;

use super::analog_runner::ANALOG_SOLVER_MANIFEST_SCHEMA;

pub(super) struct UnsupportedBackendPlan<'a> {
    pub(super) check_id: &'a str,
    pub(super) selected_backend: &'a str,
    pub(super) implemented_backend: &'a str,
    pub(super) analysis_kind: &'a str,
    pub(super) required_normalized_outputs: &'a [&'a str],
}

pub(super) fn unsupported_backend_plan_finding(
    scenario: &Scenario,
    plan: UnsupportedBackendPlan<'_>,
) -> Finding {
    let mut finding = Finding::critical(
        plan.check_id,
        &scenario.name,
        format!(
            "Backend {} was detected, but {} output normalization is not implemented in this runtime slice.",
            plan.selected_backend, plan.analysis_kind
        ),
    );
    finding
        .measured
        .insert("selected_backend".to_string(), json!(plan.selected_backend));
    finding
        .measured
        .insert("analysis_kind".to_string(), json!(plan.analysis_kind));
    finding.measured.insert(
        "adapter_status".to_string(),
        json!("planned_not_implemented"),
    );
    finding.measured.insert(
        "planned_manifest_schema".to_string(),
        json!(ANALOG_SOLVER_MANIFEST_SCHEMA),
    );
    finding.measured.insert(
        "required_normalized_outputs".to_string(),
        json!(plan.required_normalized_outputs),
    );
    finding.limit.insert(
        "implemented_backend".to_string(),
        json!(plan.implemented_backend),
    );
    finding.limit.insert(
        "required_adapter".to_string(),
        json!(required_adapter(plan.selected_backend)),
    );
    if plan.implemented_backend == "none_yet" {
        finding.suggested_fixes.push(format!(
            "Keep this scenario as planned evidence until the {} adapter emits {} artifacts and a {} manifest.",
            plan.selected_backend,
            plan.required_normalized_outputs.join(", "),
            ANALOG_SOLVER_MANIFEST_SCHEMA
        ));
    } else {
        finding.suggested_fixes.push(format!(
            "Use {} for this analysis until the {} adapter emits {} artifacts and a {} manifest.",
            plan.implemented_backend,
            plan.selected_backend,
            plan.required_normalized_outputs.join(", "),
            ANALOG_SOLVER_MANIFEST_SCHEMA
        ));
    }
    finding
}

fn required_adapter(selected_backend: &str) -> &'static str {
    if selected_backend.eq_ignore_ascii_case("xyce") {
        "xyce_result_normalizer"
    } else {
        "analysis_result_normalizer"
    }
}
