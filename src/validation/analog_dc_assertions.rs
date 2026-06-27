use crate::board_ir::{
    AnalogAggregation, AnalogAssertion, AnalogQuantity, AnalogRelation, Scenario,
};
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::SPICE_DC_ANALYSIS;
use super::analog_assertions::{AnalogAssertionMeasurement, quantity_name};
use super::analog_util::normalize_artifact_path;
use super::common::validation_input_missing;

pub(super) fn validate_dc_assertion_contract(assertion: &AnalogAssertion) -> Result<(), String> {
    if assertion.aggregation != AnalogAggregation::OperatingPoint {
        return Err("analog_dc assertions require operating_point aggregation".to_string());
    }
    if assertion.at_us.is_some()
        || assertion.start_us.is_some()
        || assertion.end_us.is_some()
        || assertion.time_limit_us.is_some()
        || assertion.at_hz.is_some()
        || assertion.frequency_limit_hz.is_some()
        || assertion.duty_limit_percent.is_some()
        || assertion.count_limit.is_some()
        || assertion.overshoot_limit_percent.is_some()
        || assertion.reference_probe.is_some()
        || assertion.reference_threshold_v.is_some()
        || assertion.reference_threshold_a.is_some()
        || assertion.reference_threshold_w.is_some()
        || assertion.threshold_vs.is_some()
        || assertion.threshold_c.is_some()
        || assertion.threshold_j.is_some()
        || assertion.threshold_db.is_some()
        || assertion.threshold_deg.is_some()
        || assertion.threshold_v_per_sqrt_hz.is_some()
        || assertion.target_v.is_some()
        || assertion.target_a.is_some()
        || assertion.target_w.is_some()
        || assertion.tolerance_v.is_some()
        || assertion.tolerance_a.is_some()
        || assertion.tolerance_w.is_some()
    {
        return Err(
            "operating_point aggregation must only declare the probe-unit threshold".to_string(),
        );
    }
    Ok(())
}

pub(super) fn evaluate_dc_assertions(
    scenario: &Scenario,
    operating_point: &Path,
    findings: &mut Vec<Finding>,
) -> Vec<AnalogAssertionMeasurement> {
    let mut measurements = Vec::new();
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before DC assertion evaluation");
    let values = match parse_operating_point_csv(operating_point) {
        Ok(values) => values,
        Err(message) => {
            findings.push(Finding::critical(
                SPICE_DC_ANALYSIS,
                &scenario.name,
                format!(
                    "Failed to parse operating-point response {}: {message}",
                    operating_point.display()
                ),
            ));
            return measurements;
        }
    };
    for assertion in &analog.assertions {
        let Some(probe) = analog
            .probes
            .iter()
            .find(|probe| probe.name == assertion.probe)
        else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog DC assertion {} references unknown probe {}.",
                    assertion.name, assertion.probe
                ),
            );
            continue;
        };
        let probe_key = sanitize_csv_column(&probe.name);
        let Some(measured) = values.get(&probe_key).copied() else {
            let mut finding = Finding::critical(
                SPICE_DC_ANALYSIS,
                &scenario.name,
                format!(
                    "DC assertion {} could not be evaluated because probe {} is missing from operating_point.csv.",
                    assertion.name, probe.name
                ),
            );
            finding.measured.insert(
                "operating_point".to_string(),
                json!(normalize_artifact_path(operating_point)),
            );
            findings.push(finding);
            continue;
        };
        let Some((limit, unit)) = comparison_limit(assertion, &probe.quantity) else {
            validation_input_missing(
                findings,
                scenario,
                format!(
                    "Analog DC assertion {} is missing a finite {} threshold.",
                    assertion.name,
                    quantity_name(&probe.quantity)
                ),
            );
            continue;
        };
        let (relation, margin, passed) = match assertion.relation {
            AnalogRelation::Below => ("below", limit - measured, measured < limit),
            AnalogRelation::Above => ("above", measured - limit, measured > limit),
        };
        measurements.push(AnalogAssertionMeasurement {
            assertion_name: assertion.name.clone(),
            probe_name: assertion.probe.clone(),
            measured,
            limit,
            margin,
            relation,
            unit,
            quantity: "operating point".to_string(),
            passed,
        });
        if !passed {
            let mut finding = Finding::critical(
                SPICE_DC_ANALYSIS,
                &scenario.name,
                format!(
                    "DC assertion {} failed: operating point probe {} measured {:.6} {}, expected {relation} {:.6} {}.",
                    assertion.name, assertion.probe, measured, unit, limit, unit
                ),
            );
            finding
                .measured
                .insert(assertion.probe.clone(), json!(measured));
            finding
                .measured
                .insert("quantity".to_string(), json!("operating point"));
            finding.measured.insert(
                "operating_point".to_string(),
                json!(normalize_artifact_path(operating_point)),
            );
            finding
                .limit
                .insert(format!("{relation}_{unit}"), json!(limit));
            if assertion.suggested_fixes.is_empty() {
                finding.suggested_fixes.push(
                    "Adjust bias values, load conditions, or the device model so the simulated DC operating point meets the declared limit."
                        .to_string(),
                );
            } else {
                finding
                    .suggested_fixes
                    .extend(assertion.suggested_fixes.iter().cloned());
            }
            findings.push(finding);
        }
    }
    measurements
}

fn comparison_limit(
    assertion: &AnalogAssertion,
    quantity: &AnalogQuantity,
) -> Option<(f64, &'static str)> {
    let (value, unit) = match quantity {
        AnalogQuantity::Voltage => (assertion.threshold_v, "V"),
        AnalogQuantity::Current => (assertion.threshold_a, "A"),
        AnalogQuantity::Power => (assertion.threshold_w, "W"),
    };
    value
        .filter(|value| value.is_finite())
        .map(|value| (value, unit))
}

fn parse_operating_point_csv(path: &Path) -> Result<BTreeMap<String, f64>, String> {
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "Failed to read operating-point CSV {}: {error}",
            path.display()
        )
    })?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| format!("Operating-point CSV {} is empty.", path.display()))?;
    let value_line = lines
        .next()
        .ok_or_else(|| format!("Operating-point CSV {} has no value row.", path.display()))?;
    let names: Vec<_> = header.split(',').map(str::trim).collect();
    let values: Vec<_> = value_line.split(',').map(str::trim).collect();
    if names.is_empty() || names.len() != values.len() {
        return Err(format!(
            "Operating-point CSV {} has mismatched header/value columns.",
            path.display()
        ));
    }
    let mut parsed = BTreeMap::new();
    for (name, value) in names.iter().zip(values.iter()) {
        if name.is_empty() {
            return Err(format!(
                "Operating-point CSV {} contains a blank probe column.",
                path.display()
            ));
        }
        let value = value.parse::<f64>().map_err(|_| {
            format!(
                "Operating-point CSV {} has non-numeric value {}.",
                path.display(),
                value
            )
        })?;
        if !value.is_finite() {
            return Err(format!(
                "Operating-point CSV {} has non-finite value {}.",
                path.display(),
                value
            ));
        }
        parsed.insert((*name).to_string(), value);
    }
    Ok(parsed)
}

fn sanitize_csv_column(name: &str) -> String {
    let mut output = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            output.push(ch);
        } else {
            output.push('_');
        }
    }
    if output.is_empty() {
        "probe".to_string()
    } else {
        output
    }
}

#[cfg(test)]
mod tests {
    use super::{evaluate_dc_assertions, validate_dc_assertion_contract};
    use crate::board_ir::{AnalogAggregation, BoardProject};
    use crate::reports::Finding;

    #[test]
    fn operating_point_assertion_passes_and_records_margin() {
        let project: BoardProject = serde_yaml_ng::from_str(
            r#"
project:
  name: dc_assertion
  version: 0.1.0
board:
  components: {}
  nets: {}
scenarios:
  - name: bias
    type: analog_dc
    checks: [SPICE_DC_ANALYSIS]
    analog:
      backend: auto
      model_files: []
      node_bindings: []
      pin_bindings: []
      analysis: { type: op }
      stimuli: []
      probes:
        - { name: midpoint, expression: V(midpoint) }
      assertions:
        - name: midpoint_above_2v
          probe: midpoint
          aggregation: operating_point
          relation: above
          threshold_v: 2.0
"#,
        )
        .unwrap();
        let scenario = &project.scenarios[0];
        let assertion = &scenario.analog.as_ref().unwrap().assertions[0];
        validate_dc_assertion_contract(assertion).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("operating_point.csv");
        std::fs::write(&path, "midpoint\n2.5\n").unwrap();
        let mut findings = Vec::<Finding>::new();
        let measurements = evaluate_dc_assertions(scenario, &path, &mut findings);
        assert!(findings.is_empty());
        assert_eq!(measurements.len(), 1);
        assert_eq!(measurements[0].assertion_name, "midpoint_above_2v");
        assert_eq!(measurements[0].margin, 0.5);
        assert!(measurements[0].passed);
    }

    #[test]
    fn dc_contract_rejects_transient_aggregation() {
        let project: BoardProject = serde_yaml_ng::from_str(
            r#"
project:
  name: dc_contract
  version: 0.1.0
board:
  components: {}
  nets: {}
scenarios:
  - name: bias
    type: analog_dc
    checks: [SPICE_DC_ANALYSIS]
    analog:
      backend: auto
      model_files: []
      node_bindings: []
      pin_bindings: []
      analysis: { type: op }
      stimuli: []
      probes:
        - { name: midpoint, expression: V(midpoint) }
      assertions:
        - name: midpoint_sample
          probe: midpoint
          aggregation: sample
          relation: above
          at_us: 1.0
          threshold_v: 2.0
"#,
        )
        .unwrap();
        let assertion = &project.scenarios[0].analog.as_ref().unwrap().assertions[0];
        assert_eq!(assertion.aggregation, AnalogAggregation::Sample);
        let error = validate_dc_assertion_contract(assertion).unwrap_err();
        assert!(error.contains("operating_point"));
    }
}
