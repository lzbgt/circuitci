use crate::board_ir::{AnalogMonteCarloCriteria, Scenario};
use crate::reports::{Finding, Severity};
use serde_json::json;
use std::collections::BTreeMap;

use super::analog_assertions::AnalogAssertionMeasurement;
use super::analog_runner::{ModelSectionOverride, ParameterOverride};
use super::analog_spice::{AnalogRunPlan, ComponentValueOverride};

pub(super) const ANALOG_SWEEP_MARGIN_SUMMARY: &str = "ANALOG_SWEEP_MARGIN_SUMMARY";
pub(super) const ANALOG_MONTE_CARLO_YIELD_SUMMARY: &str = "ANALOG_MONTE_CARLO_YIELD_SUMMARY";

#[derive(Debug, Clone)]
pub(super) struct SweepAssertionMeasurement {
    sweep_name: String,
    corner_name: String,
    parameters: BTreeMap<String, f64>,
    component_values: BTreeMap<String, f64>,
    model_sections: BTreeMap<String, String>,
    assertion: AnalogAssertionMeasurement,
}

pub(super) fn tag_corner_findings(
    findings: &mut [Finding],
    start: usize,
    run_plan: &AnalogRunPlan,
    demote_assertion_failures: bool,
) {
    for finding in findings.iter_mut().skip(start) {
        tag_corner_finding(finding, run_plan);
        if demote_assertion_failures && finding.measured.contains_key("assertion") {
            finding.severity = Severity::Info;
            finding.measured.insert(
                "monte_carlo_sample_assertion_evidence".to_string(),
                json!(true),
            );
        }
    }
}

pub(super) fn monte_carlo_criteria_enabled(scenario: &Scenario, run_plan: &AnalogRunPlan) -> bool {
    let Some(sweep_name) = run_plan.sweep_name.as_deref() else {
        return false;
    };
    scenario
        .analog
        .as_ref()
        .and_then(|analog| analog.sweeps.iter().find(|sweep| sweep.name == sweep_name))
        .and_then(|sweep| sweep.monte_carlo.as_ref())
        .is_some_and(|monte_carlo| monte_carlo.criteria.is_some())
}

pub(super) fn tag_corner_finding(finding: &mut Finding, run_plan: &AnalogRunPlan) {
    if let Some(sweep_name) = &run_plan.sweep_name {
        finding
            .measured
            .insert("analog_sweep".to_string(), json!(sweep_name));
        finding
            .measured
            .insert("analog_corner".to_string(), json!(run_plan.corner_name));
        finding.measured.insert(
            "analog_parameters".to_string(),
            json!(parameter_override_map(&run_plan.parameter_overrides)),
        );
        finding.measured.insert(
            "analog_model_sections".to_string(),
            json!(model_section_override_map(
                &run_plan.model_section_overrides
            )),
        );
        finding.measured.insert(
            "analog_component_values".to_string(),
            json!(component_value_override_map(
                &run_plan.component_value_overrides
            )),
        );
    }
}

pub(super) fn record_sweep_measurements(
    output: &mut Vec<SweepAssertionMeasurement>,
    run_plan: &AnalogRunPlan,
    measurements: Vec<AnalogAssertionMeasurement>,
) {
    let Some(sweep_name) = &run_plan.sweep_name else {
        return;
    };
    let parameters = parameter_override_map(&run_plan.parameter_overrides);
    let component_values = component_value_override_map(&run_plan.component_value_overrides);
    let model_sections = model_section_override_map(&run_plan.model_section_overrides);
    output.extend(
        measurements
            .into_iter()
            .map(|assertion| SweepAssertionMeasurement {
                sweep_name: sweep_name.clone(),
                corner_name: run_plan.corner_name.clone(),
                parameters: parameters.clone(),
                component_values: component_values.clone(),
                model_sections: model_sections.clone(),
                assertion,
            }),
    );
}

pub(super) fn push_sweep_margin_summaries(
    findings: &mut Vec<Finding>,
    scenario: &Scenario,
    measurements: &[SweepAssertionMeasurement],
) {
    push_worst_corner_summaries(findings, scenario, measurements);
    push_monte_carlo_yield_summaries(findings, scenario, measurements);
}

fn push_worst_corner_summaries(
    findings: &mut Vec<Finding>,
    scenario: &Scenario,
    measurements: &[SweepAssertionMeasurement],
) {
    let mut worst_by_assertion: BTreeMap<(String, String), &SweepAssertionMeasurement> =
        BTreeMap::new();
    let mut counts_by_assertion: BTreeMap<(String, String), usize> = BTreeMap::new();
    for measurement in measurements {
        let key = (
            measurement.sweep_name.clone(),
            measurement.assertion.assertion_name.clone(),
        );
        *counts_by_assertion.entry(key.clone()).or_default() += 1;
        let replace = worst_by_assertion
            .get(&key)
            .is_none_or(|current| measurement.assertion.margin < current.assertion.margin);
        if replace {
            worst_by_assertion.insert(key, measurement);
        }
    }
    for ((sweep_name, assertion_name), worst) in worst_by_assertion {
        let evaluated_corners = counts_by_assertion
            .get(&(sweep_name.clone(), assertion_name.clone()))
            .copied()
            .unwrap_or(0);
        let mut finding = Finding::info(
            ANALOG_SWEEP_MARGIN_SUMMARY,
            &scenario.name,
            format!(
                "Analog sweep {sweep_name} worst margin for assertion {assertion_name} is {:.6} {} at {}.",
                worst.assertion.margin, worst.assertion.unit, worst.corner_name
            ),
        );
        insert_common_summary_fields(&mut finding, &sweep_name, &assertion_name, worst);
        finding
            .measured
            .insert("evaluated_corners".to_string(), json!(evaluated_corners));
        finding
            .limit
            .insert("relation".to_string(), json!(worst.assertion.relation));
        finding
            .limit
            .insert("limit_value".to_string(), json!(worst.assertion.limit));
        finding
            .limit
            .insert("limit_unit".to_string(), json!(worst.assertion.unit));
        finding
            .limit
            .insert("minimum_margin".to_string(), json!(0.0));
        findings.push(finding);
    }
}

fn push_monte_carlo_yield_summaries(
    findings: &mut Vec<Finding>,
    scenario: &Scenario,
    measurements: &[SweepAssertionMeasurement],
) {
    let Some(analog) = scenario.analog.as_ref() else {
        return;
    };
    let monte_carlo_sweeps: BTreeMap<&str, Option<&AnalogMonteCarloCriteria>> = analog
        .sweeps
        .iter()
        .filter(|sweep| sweep.monte_carlo.is_some())
        .map(|sweep| {
            (
                sweep.name.as_str(),
                sweep
                    .monte_carlo
                    .as_ref()
                    .and_then(|monte_carlo| monte_carlo.criteria.as_ref()),
            )
        })
        .collect();
    if monte_carlo_sweeps.is_empty() {
        return;
    }
    let mut grouped: BTreeMap<(String, String), Vec<&SweepAssertionMeasurement>> = BTreeMap::new();
    for measurement in measurements {
        if monte_carlo_sweeps.contains_key(measurement.sweep_name.as_str()) {
            grouped
                .entry((
                    measurement.sweep_name.clone(),
                    measurement.assertion.assertion_name.clone(),
                ))
                .or_default()
                .push(measurement);
        }
    }
    for ((sweep_name, assertion_name), group) in grouped {
        let Some(stats) = MonteCarloStats::from_measurements(&group) else {
            continue;
        };
        let worst = stats.worst;
        let criteria = monte_carlo_sweeps
            .get(sweep_name.as_str())
            .copied()
            .flatten();
        let criteria_result =
            criteria.map(|criteria| MonteCarloCriteriaResult::evaluate(&stats, criteria));
        let message = monte_carlo_summary_message(
            &sweep_name,
            &assertion_name,
            &stats,
            criteria_result.as_ref(),
        );
        let mut finding = if criteria_result
            .as_ref()
            .is_some_and(|result| !result.passed)
        {
            Finding::critical(ANALOG_MONTE_CARLO_YIELD_SUMMARY, &scenario.name, message)
        } else {
            Finding::info(ANALOG_MONTE_CARLO_YIELD_SUMMARY, &scenario.name, message)
        };
        insert_common_summary_fields(&mut finding, &sweep_name, &assertion_name, worst);
        if let Some(result) = criteria_result {
            result.insert_fields(&mut finding);
        }
        finding.measured.insert(
            "evaluated_samples".to_string(),
            json!(stats.evaluated_samples),
        );
        finding
            .measured
            .insert("passed_samples".to_string(), json!(stats.passed_samples));
        finding
            .measured
            .insert("failed_samples".to_string(), json!(stats.failed_samples));
        finding
            .measured
            .insert("yield_percent".to_string(), json!(stats.yield_percent));
        finding
            .measured
            .insert("mean_margin".to_string(), json!(stats.mean_margin));
        finding
            .measured
            .insert("stddev_margin".to_string(), json!(stats.stddev_margin));
        finding
            .measured
            .insert("min_margin".to_string(), json!(stats.min_margin));
        finding
            .measured
            .insert("max_margin".to_string(), json!(stats.max_margin));
        finding
            .measured
            .insert("p1_margin".to_string(), json!(stats.p1_margin));
        finding
            .measured
            .insert("p5_margin".to_string(), json!(stats.p5_margin));
        finding
            .measured
            .insert("p50_margin".to_string(), json!(stats.p50_margin));
        finding
            .measured
            .insert("p95_margin".to_string(), json!(stats.p95_margin));
        finding
            .limit
            .insert("minimum_margin".to_string(), json!(0.0));
        findings.push(finding);
    }
}

fn monte_carlo_summary_message(
    sweep_name: &str,
    assertion_name: &str,
    stats: &MonteCarloStats<'_>,
    criteria_result: Option<&MonteCarloCriteriaResult>,
) -> String {
    let worst = stats.worst;
    let mut message = format!(
        "Monte Carlo sweep {sweep_name} yield for assertion {assertion_name} is {:.3}% ({}/{} pass); mean margin {:.6} {} with stddev {:.6} {}, p5/p50/p95 margins {:.6}/{:.6}/{:.6} {}, worst {:.6} {} at {}.",
        stats.yield_percent,
        stats.passed_samples,
        stats.evaluated_samples,
        stats.mean_margin,
        worst.assertion.unit,
        stats.stddev_margin,
        worst.assertion.unit,
        stats.p5_margin,
        stats.p50_margin,
        stats.p95_margin,
        worst.assertion.unit,
        stats.min_margin,
        worst.assertion.unit,
        worst.corner_name
    );
    if let Some(result) = criteria_result {
        if result.passed {
            message.push_str(" Monte Carlo criteria passed.");
        } else {
            message.push_str(" Monte Carlo criteria failed: ");
            message.push_str(&result.failures.join("; "));
            message.push('.');
        }
    }
    message
}

struct MonteCarloCriteriaResult {
    passed: bool,
    failures: Vec<String>,
    min_yield_percent: Option<f64>,
    min_p1_margin: Option<f64>,
    min_p5_margin: Option<f64>,
    min_p50_margin: Option<f64>,
    min_p95_margin: Option<f64>,
}

impl MonteCarloCriteriaResult {
    fn evaluate(stats: &MonteCarloStats<'_>, criteria: &AnalogMonteCarloCriteria) -> Self {
        let mut failures = Vec::new();
        if let Some(limit) = criteria.min_yield_percent
            && stats.yield_percent < limit
        {
            failures.push(format!(
                "yield {:.3}% is below required {:.3}%",
                stats.yield_percent, limit
            ));
        }
        for (label, measured, limit) in [
            ("p1 margin", stats.p1_margin, criteria.min_p1_margin),
            ("p5 margin", stats.p5_margin, criteria.min_p5_margin),
            ("p50 margin", stats.p50_margin, criteria.min_p50_margin),
            ("p95 margin", stats.p95_margin, criteria.min_p95_margin),
        ] {
            if let Some(limit) = limit
                && measured < limit
            {
                failures.push(format!(
                    "{label} {:.6} {} is below required {:.6} {}",
                    measured, stats.worst.assertion.unit, limit, stats.worst.assertion.unit
                ));
            }
        }
        Self {
            passed: failures.is_empty(),
            failures,
            min_yield_percent: criteria.min_yield_percent,
            min_p1_margin: criteria.min_p1_margin,
            min_p5_margin: criteria.min_p5_margin,
            min_p50_margin: criteria.min_p50_margin,
            min_p95_margin: criteria.min_p95_margin,
        }
    }

    fn insert_fields(self, finding: &mut Finding) {
        finding
            .measured
            .insert("criteria_passed".to_string(), json!(self.passed));
        finding
            .measured
            .insert("passed".to_string(), json!(self.passed));
        for (field, value) in [
            ("minimum_yield_percent", self.min_yield_percent),
            ("minimum_p1_margin", self.min_p1_margin),
            ("minimum_p5_margin", self.min_p5_margin),
            ("minimum_p50_margin", self.min_p50_margin),
            ("minimum_p95_margin", self.min_p95_margin),
        ] {
            if let Some(value) = value {
                finding.limit.insert(field.to_string(), json!(value));
            }
        }
    }
}

struct MonteCarloStats<'a> {
    evaluated_samples: usize,
    passed_samples: usize,
    failed_samples: usize,
    yield_percent: f64,
    mean_margin: f64,
    stddev_margin: f64,
    min_margin: f64,
    max_margin: f64,
    p1_margin: f64,
    p5_margin: f64,
    p50_margin: f64,
    p95_margin: f64,
    worst: &'a SweepAssertionMeasurement,
}

impl<'a> MonteCarloStats<'a> {
    fn from_measurements(measurements: &[&'a SweepAssertionMeasurement]) -> Option<Self> {
        let evaluated_samples = measurements.len();
        if evaluated_samples == 0 {
            return None;
        }
        let passed_samples = measurements
            .iter()
            .filter(|measurement| measurement.assertion.passed)
            .count();
        let failed_samples = evaluated_samples - passed_samples;
        let yield_percent = passed_samples as f64 * 100.0 / evaluated_samples as f64;
        let mean_margin = measurements
            .iter()
            .map(|measurement| measurement.assertion.margin)
            .sum::<f64>()
            / evaluated_samples as f64;
        let variance = measurements
            .iter()
            .map(|measurement| {
                let delta = measurement.assertion.margin - mean_margin;
                delta * delta
            })
            .sum::<f64>()
            / evaluated_samples as f64;
        let worst = measurements
            .iter()
            .min_by(|left, right| left.assertion.margin.total_cmp(&right.assertion.margin))?
            .to_owned();
        let max_margin = measurements
            .iter()
            .map(|measurement| measurement.assertion.margin)
            .fold(f64::NEG_INFINITY, f64::max);
        let mut margins = measurements
            .iter()
            .map(|measurement| measurement.assertion.margin)
            .collect::<Vec<_>>();
        margins.sort_by(f64::total_cmp);
        Some(Self {
            evaluated_samples,
            passed_samples,
            failed_samples,
            yield_percent,
            mean_margin,
            stddev_margin: variance.sqrt(),
            min_margin: worst.assertion.margin,
            max_margin,
            p1_margin: percentile_margin(&margins, 1.0),
            p5_margin: percentile_margin(&margins, 5.0),
            p50_margin: percentile_margin(&margins, 50.0),
            p95_margin: percentile_margin(&margins, 95.0),
            worst,
        })
    }
}

fn percentile_margin(sorted_margins: &[f64], percentile: f64) -> f64 {
    if sorted_margins.is_empty() {
        return f64::NAN;
    }
    if sorted_margins.len() == 1 {
        return sorted_margins[0];
    }
    let clamped = percentile.clamp(0.0, 100.0);
    let rank = clamped / 100.0 * (sorted_margins.len() - 1) as f64;
    let lower_index = rank.floor() as usize;
    let upper_index = rank.ceil() as usize;
    if lower_index == upper_index {
        sorted_margins[lower_index]
    } else {
        let fraction = rank - lower_index as f64;
        sorted_margins[lower_index] * (1.0 - fraction) + sorted_margins[upper_index] * fraction
    }
}

fn insert_common_summary_fields(
    finding: &mut Finding,
    sweep_name: &str,
    assertion_name: &str,
    measurement: &SweepAssertionMeasurement,
) {
    finding
        .measured
        .insert("analog_sweep".to_string(), json!(sweep_name));
    finding
        .measured
        .insert("analog_corner".to_string(), json!(measurement.corner_name));
    finding.measured.insert(
        "analog_parameters".to_string(),
        json!(measurement.parameters.clone()),
    );
    finding.measured.insert(
        "analog_model_sections".to_string(),
        json!(measurement.model_sections.clone()),
    );
    finding.measured.insert(
        "analog_component_values".to_string(),
        json!(measurement.component_values.clone()),
    );
    finding
        .measured
        .insert("assertion".to_string(), json!(assertion_name));
    finding.measured.insert(
        "probe".to_string(),
        json!(&measurement.assertion.probe_name),
    );
    finding.measured.insert(
        "quantity".to_string(),
        json!(&measurement.assertion.quantity),
    );
    finding.measured.insert(
        "measured_value".to_string(),
        json!(measurement.assertion.measured),
    );
    finding.measured.insert(
        "measured_unit".to_string(),
        json!(measurement.assertion.unit),
    );
    finding
        .measured
        .insert("margin".to_string(), json!(measurement.assertion.margin));
    finding
        .measured
        .insert("passed".to_string(), json!(measurement.assertion.passed));
    finding.limit.insert(
        "relation".to_string(),
        json!(measurement.assertion.relation),
    );
    finding.limit.insert(
        "limit_value".to_string(),
        json!(measurement.assertion.limit),
    );
    finding
        .limit
        .insert("limit_unit".to_string(), json!(measurement.assertion.unit));
}

fn parameter_override_map(overrides: &[ParameterOverride]) -> BTreeMap<String, f64> {
    overrides
        .iter()
        .map(|override_| (override_.name.clone(), override_.value))
        .collect()
}

fn model_section_override_map(overrides: &[ModelSectionOverride]) -> BTreeMap<String, String> {
    overrides
        .iter()
        .map(|override_| (override_.path.clone(), override_.section.clone()))
        .collect()
}

fn component_value_override_map(overrides: &[ComponentValueOverride]) -> BTreeMap<String, f64> {
    overrides
        .iter()
        .map(|override_| {
            (
                format!("{}.{}", override_.component, override_.field.as_str()),
                override_.value,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        ANALOG_MONTE_CARLO_YIELD_SUMMARY, ANALOG_SWEEP_MARGIN_SUMMARY, SweepAssertionMeasurement,
        push_sweep_margin_summaries, tag_corner_finding, tag_corner_findings,
    };
    use crate::board_ir::{AnalogSweepComponentField, BoardProject};
    use crate::reports::{Finding, Severity};
    use crate::validation::analog_assertions::AnalogAssertionMeasurement;
    use crate::validation::analog_runner::{ModelSectionOverride, ParameterOverride};
    use crate::validation::analog_spice::{AnalogRunPlan, ComponentValueOverride};
    use serde_json::json;
    use std::collections::BTreeMap;

    fn scenario_yaml(sweep_yaml: &str) -> String {
        format!(
            r#"
project:
  name: sweep_report_test
  version: 0.1.0
board:
  name: sweep_report_test
  components: {{}}
  nets: {{}}
scenarios:
  - name: analog_sweep
    type: analog_transient
    checks: [SPICE_TRANSIENT_ANALYSIS]
    analog:
      backend: auto
      netlist_source: file
      netlist: deck.cir
      model_files: []
      node_bindings:
        - {{ node: "0", net: gnd }}
      pin_bindings:
        - {{ node: "0", endpoint: {{ component: U1, pin: GND }} }}
      analysis:
        type: tran
        stop_time_us: 1000.0
        max_step_us: 1.0
      stimuli: []
      sweeps:
        - name: rc_tolerance
{sweep_yaml}
      probes:
        - name: v_out
          expression: V(out)
      assertions: []
"#
        )
    }

    #[test]
    fn sweep_corner_tags_findings_with_parameter_values() {
        let run_plan = AnalogRunPlan {
            sweep_name: Some("rc_tolerance".to_string()),
            corner_name: "corner_007".to_string(),
            run_subdir: Some("rc_tolerance_corner_007".to_string()),
            parameter_overrides: vec![ParameterOverride {
                name: "RIN_VALUE".to_string(),
                value: 1050.0,
            }],
            component_value_overrides: vec![ComponentValueOverride {
                component: "RLOAD".to_string(),
                field: AnalogSweepComponentField::ValueOhm,
                parameter_name: "CCI_RLOAD_VALUE_OHM".to_string(),
                value: 1100.0,
            }],
            model_section_overrides: vec![ModelSectionOverride {
                path: "models/vendor.lib".to_string(),
                section: "slow".to_string(),
            }],
        };
        let mut finding = Finding::critical(
            "SPICE_TRANSIENT_ANALYSIS",
            "rc_lowpass",
            "filtered output exceeded the attenuation limit",
        );

        tag_corner_finding(&mut finding, &run_plan);

        assert_eq!(finding.measured["analog_sweep"], "rc_tolerance");
        assert_eq!(finding.measured["analog_corner"], "corner_007");
        assert_eq!(finding.measured["analog_parameters"]["RIN_VALUE"], 1050.0);
        assert_eq!(
            finding.measured["analog_model_sections"]["models/vendor.lib"],
            "slow"
        );
        assert_eq!(
            finding.measured["analog_component_values"]["RLOAD.value_ohm"],
            1100.0
        );
    }

    #[test]
    fn monte_carlo_criteria_mode_demotes_sample_assertion_findings() {
        let run_plan = AnalogRunPlan {
            sweep_name: Some("rc_monte_carlo".to_string()),
            corner_name: "corner_002".to_string(),
            run_subdir: Some("rc_monte_carlo_corner_002".to_string()),
            parameter_overrides: Vec::new(),
            component_value_overrides: Vec::new(),
            model_section_overrides: Vec::new(),
        };
        let mut findings = vec![Finding::critical(
            "SPICE_AC_ANALYSIS",
            "rc_lowpass",
            "AC assertion failed",
        )];
        findings[0]
            .measured
            .insert("assertion".to_string(), json!("cutoff_above"));

        tag_corner_findings(&mut findings, 0, &run_plan, true);

        assert_eq!(findings[0].severity, Severity::Info);
        assert_eq!(findings[0].measured["analog_sweep"], "rc_monte_carlo");
        assert_eq!(
            findings[0].measured["monte_carlo_sample_assertion_evidence"],
            true
        );
    }

    #[test]
    fn sweep_margin_summary_reports_worst_assertion_corner() {
        let project: BoardProject = serde_yaml_ng::from_str(&scenario_yaml(
            r#"          parameters:
            - name: RIN_VALUE
              values: [950.0, 1000.0, 1050.0]
"#,
        ))
        .unwrap();
        let scenario = &project.scenarios[0];
        let measurements = vec![
            sweep_measurement("corner_001", 950.0, 0.12, 0.52, true),
            sweep_measurement("corner_002", 1000.0, 0.04, 0.60, true),
            sweep_measurement("corner_003", 1050.0, -0.02, 0.66, false),
        ];
        let mut findings = Vec::new();

        push_sweep_margin_summaries(&mut findings, scenario, &measurements);

        assert_eq!(findings.len(), 1);
        let summary = &findings[0];
        assert_eq!(summary.id, ANALOG_SWEEP_MARGIN_SUMMARY);
        assert_eq!(summary.measured["analog_sweep"], "rc_tolerance");
        assert_eq!(summary.measured["analog_corner"], "corner_003");
        assert_eq!(summary.measured["analog_parameters"]["RIN_VALUE"], 1050.0);
        assert_eq!(
            summary.measured["analog_model_sections"]["models/vendor.lib"],
            "typ"
        );
        assert_eq!(
            summary.measured["analog_component_values"]["RLOAD.value_ohm"],
            1100.0
        );
        assert_eq!(summary.measured["assertion"], "filtered_rms_below");
        assert_eq!(summary.measured["evaluated_corners"], 3);
        assert_eq!(summary.measured["passed"], false);
        assert_eq!(summary.limit["relation"], "below");
        assert_eq!(summary.limit["minimum_margin"], 0.0);
    }

    #[test]
    fn monte_carlo_yield_summary_reports_margin_distribution() {
        let project: BoardProject = serde_yaml_ng::from_str(&scenario_yaml(
            r#"          monte_carlo:
            samples: 3
            seed: 1
            component_values:
              - component: RLOAD
                field: value_ohm
                nominal: 1000.0
                tolerance_percent: 5.0
"#,
        ))
        .unwrap();
        let scenario = &project.scenarios[0];
        let measurements = vec![
            sweep_measurement("corner_001", 950.0, 0.12, 0.52, true),
            sweep_measurement("corner_002", 1000.0, -0.03, 0.67, false),
            sweep_measurement("corner_003", 1050.0, 0.06, 0.58, true),
        ];
        let mut findings = Vec::new();

        push_sweep_margin_summaries(&mut findings, scenario, &measurements);

        let summary = findings
            .iter()
            .find(|finding| finding.id == ANALOG_MONTE_CARLO_YIELD_SUMMARY)
            .unwrap();
        assert_eq!(summary.measured["analog_sweep"], "rc_tolerance");
        assert_eq!(summary.measured["assertion"], "filtered_rms_below");
        assert_eq!(summary.measured["analog_corner"], "corner_002");
        assert_eq!(summary.measured["evaluated_samples"], 3);
        assert_eq!(summary.measured["passed_samples"], 2);
        assert_eq!(summary.measured["failed_samples"], 1);
        assert_eq!(summary.measured["yield_percent"], 200.0 / 3.0);
        assert_eq!(summary.measured["min_margin"], -0.03);
        assert_eq!(summary.measured["max_margin"], 0.12);
        assert_close(summary.measured["p1_margin"].as_f64().unwrap(), -0.0282);
        assert_close(summary.measured["p5_margin"].as_f64().unwrap(), -0.021);
        assert_close(summary.measured["p50_margin"].as_f64().unwrap(), 0.06);
        assert_close(summary.measured["p95_margin"].as_f64().unwrap(), 0.114);
        assert_eq!(summary.limit["minimum_margin"], 0.0);
    }

    #[test]
    fn monte_carlo_yield_summary_passes_declared_criteria() {
        let project: BoardProject = serde_yaml_ng::from_str(&scenario_yaml(
            r#"          monte_carlo:
            samples: 3
            seed: 1
            criteria:
              min_yield_percent: 60.0
              min_p5_margin: -0.03
              min_p50_margin: 0.05
            component_values:
              - component: RLOAD
                field: value_ohm
                nominal: 1000.0
                tolerance_percent: 5.0
"#,
        ))
        .unwrap();
        let scenario = &project.scenarios[0];
        let measurements = vec![
            sweep_measurement("corner_001", 950.0, 0.12, 0.52, true),
            sweep_measurement("corner_002", 1000.0, -0.03, 0.67, false),
            sweep_measurement("corner_003", 1050.0, 0.06, 0.58, true),
        ];
        let mut findings = Vec::new();

        push_sweep_margin_summaries(&mut findings, scenario, &measurements);

        let summary = findings
            .iter()
            .find(|finding| finding.id == ANALOG_MONTE_CARLO_YIELD_SUMMARY)
            .unwrap();
        assert_eq!(summary.severity, Severity::Info);
        assert_eq!(summary.measured["criteria_passed"], true);
        assert_eq!(summary.measured["passed"], true);
        assert_eq!(summary.limit["minimum_yield_percent"], 60.0);
        assert_eq!(summary.limit["minimum_p5_margin"], -0.03);
        assert_eq!(summary.limit["minimum_p50_margin"], 0.05);
        assert!(summary.message.contains("criteria passed"));
    }

    #[test]
    fn monte_carlo_yield_summary_fails_declared_criteria() {
        let project: BoardProject = serde_yaml_ng::from_str(&scenario_yaml(
            r#"          monte_carlo:
            samples: 3
            seed: 1
            criteria:
              min_yield_percent: 95.0
              min_p5_margin: 0.0
            component_values:
              - component: RLOAD
                field: value_ohm
                nominal: 1000.0
                tolerance_percent: 5.0
"#,
        ))
        .unwrap();
        let scenario = &project.scenarios[0];
        let measurements = vec![
            sweep_measurement("corner_001", 950.0, 0.12, 0.52, true),
            sweep_measurement("corner_002", 1000.0, -0.03, 0.67, false),
            sweep_measurement("corner_003", 1050.0, 0.06, 0.58, true),
        ];
        let mut findings = Vec::new();

        push_sweep_margin_summaries(&mut findings, scenario, &measurements);

        let summary = findings
            .iter()
            .find(|finding| finding.id == ANALOG_MONTE_CARLO_YIELD_SUMMARY)
            .unwrap();
        assert_eq!(summary.severity, Severity::Critical);
        assert_eq!(summary.measured["criteria_passed"], false);
        assert_eq!(summary.measured["passed"], false);
        assert_eq!(summary.limit["minimum_yield_percent"], 95.0);
        assert_eq!(summary.limit["minimum_p5_margin"], 0.0);
        assert!(summary.message.contains("criteria failed"));
        assert!(
            summary
                .message
                .contains("yield 66.667% is below required 95.000%")
        );
        assert!(summary.message.contains("p5 margin"));
    }

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-12,
            "expected {expected}, got {actual}"
        );
    }

    fn sweep_measurement(
        corner_name: &str,
        parameter_value: f64,
        margin: f64,
        measured: f64,
        passed: bool,
    ) -> SweepAssertionMeasurement {
        SweepAssertionMeasurement {
            sweep_name: "rc_tolerance".to_string(),
            corner_name: corner_name.to_string(),
            parameters: BTreeMap::from([("RIN_VALUE".to_string(), parameter_value)]),
            component_values: BTreeMap::from([("RLOAD.value_ohm".to_string(), 1100.0)]),
            model_sections: BTreeMap::from([("models/vendor.lib".to_string(), "typ".to_string())]),
            assertion: AnalogAssertionMeasurement {
                assertion_name: "filtered_rms_below".to_string(),
                probe_name: "v_filtered".to_string(),
                measured,
                limit: 0.64,
                margin,
                relation: "below",
                unit: "V".to_string(),
                quantity: "rms voltage".to_string(),
                passed,
            },
        }
    }
}
