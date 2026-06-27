use crate::board_ir::Scenario;
use crate::reports::Finding;
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
) {
    for finding in findings.iter_mut().skip(start) {
        tag_corner_finding(finding, run_plan);
    }
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
    let monte_carlo_sweeps: std::collections::BTreeSet<&str> = analog
        .sweeps
        .iter()
        .filter(|sweep| sweep.monte_carlo.is_some())
        .map(|sweep| sweep.name.as_str())
        .collect();
    if monte_carlo_sweeps.is_empty() {
        return;
    }
    let mut grouped: BTreeMap<(String, String), Vec<&SweepAssertionMeasurement>> = BTreeMap::new();
    for measurement in measurements {
        if monte_carlo_sweeps.contains(measurement.sweep_name.as_str()) {
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
        let mut finding = Finding::info(
            ANALOG_MONTE_CARLO_YIELD_SUMMARY,
            &scenario.name,
            format!(
                "Monte Carlo sweep {sweep_name} yield for assertion {assertion_name} is {:.3}% ({}/{} pass); mean margin {:.6} {} with stddev {:.6} {}, worst {:.6} {} at {}.",
                stats.yield_percent,
                stats.passed_samples,
                stats.evaluated_samples,
                stats.mean_margin,
                worst.assertion.unit,
                stats.stddev_margin,
                worst.assertion.unit,
                stats.min_margin,
                worst.assertion.unit,
                worst.corner_name
            ),
        );
        insert_common_summary_fields(&mut finding, &sweep_name, &assertion_name, worst);
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
            .limit
            .insert("minimum_margin".to_string(), json!(0.0));
        findings.push(finding);
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
        Some(Self {
            evaluated_samples,
            passed_samples,
            failed_samples,
            yield_percent,
            mean_margin,
            stddev_margin: variance.sqrt(),
            min_margin: worst.assertion.margin,
            max_margin,
            worst,
        })
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
        push_sweep_margin_summaries, tag_corner_finding,
    };
    use crate::board_ir::{AnalogSweepComponentField, BoardProject};
    use crate::reports::Finding;
    use crate::validation::analog_assertions::AnalogAssertionMeasurement;
    use crate::validation::analog_runner::{ModelSectionOverride, ParameterOverride};
    use crate::validation::analog_spice::{AnalogRunPlan, ComponentValueOverride};
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
        assert_eq!(summary.limit["minimum_margin"], 0.0);
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
                unit: "V",
                quantity: "rms voltage".to_string(),
                passed,
            },
        }
    }
}
