use crate::board_ir::{
    AnalogRelation, AnalogSParameterNoiseAssertion, AnalogSParameterNoiseMetric, AnalogScenario,
    Scenario,
};
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;
use std::path::Path;

use super::SPICE_S_PARAMETER_ANALYSIS;
use super::analog_util::normalize_artifact_path;

pub(super) fn validate_s_parameter_noise_assertion_contract(
    analog: &AnalogScenario,
) -> Result<(), String> {
    if analog.analysis.s_parameter_noise_assertions.is_empty() {
        return Ok(());
    }
    if analog.analysis.s_parameter_ports.len() != 2 {
        return Err(
            "analog_sparameter s_parameter_noise_assertions require exactly two declared S-parameter ports."
                .to_string(),
        );
    }
    let mut names = BTreeSet::new();
    for assertion in &analog.analysis.s_parameter_noise_assertions {
        let name = assertion.name.trim();
        if name.is_empty() {
            return Err(
                "analog_sparameter s_parameter_noise_assertions entries require non-empty name."
                    .to_string(),
            );
        }
        if !names.insert(name.to_ascii_lowercase()) {
            return Err(format!(
                "analog_sparameter declares duplicate s_parameter_noise_assertions name {name}."
            ));
        }
        if !assertion.threshold.is_finite() {
            return Err(format!(
                "analog_sparameter s_parameter_noise_assertion {name} requires finite threshold."
            ));
        }
        if assertion
            .unit
            .as_deref()
            .is_some_and(|unit| unit.trim().is_empty())
        {
            return Err(format!(
                "analog_sparameter s_parameter_noise_assertion {name} unit must be non-empty when provided."
            ));
        }
    }
    Ok(())
}

pub(super) fn evaluate_s_parameter_noise_assertion_boundary(
    scenario: &Scenario,
    s_parameters: &Path,
    findings: &mut Vec<Finding>,
) {
    let Some(analog) = scenario.analog.as_ref() else {
        return;
    };
    if analog.analysis.s_parameter_noise_assertions.is_empty() {
        return;
    }
    for assertion in &analog.analysis.s_parameter_noise_assertions {
        push_s_parameter_noise_unavailable_finding(scenario, assertion, s_parameters, findings);
    }
}

fn push_s_parameter_noise_unavailable_finding(
    scenario: &Scenario,
    assertion: &AnalogSParameterNoiseAssertion,
    s_parameters: &Path,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        SPICE_S_PARAMETER_ANALYSIS,
        &scenario.name,
        format!(
            "S-parameter noise assertion {} metric {} requires normalized two-port SP-noise evidence that this backend does not yet emit.",
            assertion.name,
            noise_metric_name(assertion.metric)
        ),
    );
    finding
        .measured
        .insert("assertion".to_string(), json!(&assertion.name));
    finding.measured.insert(
        "metric".to_string(),
        json!(noise_metric_name(assertion.metric)),
    );
    finding.measured.insert(
        "relation".to_string(),
        json!(relation_name(&assertion.relation)),
    );
    finding.measured.insert(
        "s_parameters".to_string(),
        json!(normalize_artifact_path(s_parameters)),
    );
    finding.measured.insert(
        "backend_status".to_string(),
        json!("planned_not_implemented"),
    );
    finding.measured.insert(
        "source_evidence".to_string(),
        json!("ngspice .SP donoise=1 provides NF, NFmin, Rn, and SOpt for two-port SP noise; CircuitCI has not normalized those outputs yet."),
    );
    finding.limit.insert(
        format!(
            "{}_{}",
            relation_name(&assertion.relation),
            noise_metric_unit(assertion)
        ),
        json!(assertion.threshold),
    );
    finding.limit.insert(
        "required_normalized_output".to_string(),
        json!("s_parameter_noise_summary"),
    );
    finding.limit.insert(
        "required_backend_feature".to_string(),
        json!("ngspice_sp_donoise_two_port_noise_outputs"),
    );
    if assertion.suggested_fixes.is_empty() {
        finding.suggested_fixes.push(
            "Run an RF SP-noise backend that emits NF/NFmin/Rn/SOpt and retain it as s_parameter_noise_summary.csv before enabling this sign-off gate."
                .to_string(),
        );
    } else {
        finding
            .suggested_fixes
            .extend(assertion.suggested_fixes.iter().cloned());
    }
    findings.push(finding);
}

fn noise_metric_name(metric: AnalogSParameterNoiseMetric) -> &'static str {
    match metric {
        AnalogSParameterNoiseMetric::NoiseFigureDbMax => "noise_figure_db_max",
        AnalogSParameterNoiseMetric::MinimumNoiseFigureDbMax => "minimum_noise_figure_db_max",
        AnalogSParameterNoiseMetric::EquivalentNoiseResistanceOhmMax => {
            "equivalent_noise_resistance_ohm_max"
        }
        AnalogSParameterNoiseMetric::OptimumSourceReflectionMagnitudeMax => {
            "optimum_source_reflection_magnitude_max"
        }
    }
}

fn noise_metric_unit(assertion: &AnalogSParameterNoiseAssertion) -> &str {
    assertion.unit.as_deref().unwrap_or(match assertion.metric {
        AnalogSParameterNoiseMetric::NoiseFigureDbMax
        | AnalogSParameterNoiseMetric::MinimumNoiseFigureDbMax => "dB",
        AnalogSParameterNoiseMetric::EquivalentNoiseResistanceOhmMax => "ohm",
        AnalogSParameterNoiseMetric::OptimumSourceReflectionMagnitudeMax => "ratio",
    })
}

fn relation_name(relation: &AnalogRelation) -> &'static str {
    match relation {
        AnalogRelation::Above => "above",
        AnalogRelation::Below => "below",
    }
}
