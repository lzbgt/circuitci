use crate::board_ir::{
    AnalogMonteCarloCriteria, AnalogNetlistSource, AnalogScenario, AnalogSweepComponentField,
    Scenario,
};
use crate::library::BoundBoard;
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::analog_model_compiler::validate_model_compiler_provenance;
use super::analog_runner::{ModelSectionOverride, ParameterOverride};
use super::analog_sweep_sampling::monte_carlo_component_value_samples;
use super::analog_util::{component_value_parameter_name, push_artifact};
use super::spice_netlist::generate_board_netlist;

const MAX_ANALOG_SWEEP_CORNERS: usize = 64;

#[derive(Debug, Clone)]
pub(super) struct AnalogRunPlan {
    pub(super) sweep_name: Option<String>,
    pub(super) corner_name: String,
    pub(super) run_subdir: Option<String>,
    pub(super) parameter_overrides: Vec<ParameterOverride>,
    pub(super) component_value_overrides: Vec<ComponentValueOverride>,
    pub(super) model_section_overrides: Vec<ModelSectionOverride>,
}

#[derive(Debug, Clone)]
pub(super) struct ComponentValueOverride {
    pub(super) component: String,
    pub(super) field: AnalogSweepComponentField,
    pub(super) parameter_name: String,
    pub(super) value: f64,
}

impl AnalogRunPlan {
    fn nominal() -> Self {
        Self {
            sweep_name: None,
            corner_name: "nominal".to_string(),
            run_subdir: None,
            parameter_overrides: Vec::new(),
            component_value_overrides: Vec::new(),
            model_section_overrides: Vec::new(),
        }
    }

    pub(super) fn progress_label(&self) -> String {
        if let Some(sweep_name) = &self.sweep_name {
            let override_count = self.parameter_overrides.len()
                + self.component_value_overrides.len()
                + self.model_section_overrides.len();
            format!(
                "{} / {} with {} override(s).",
                sweep_name, self.corner_name, override_count
            )
        } else {
            "nominal input set.".to_string()
        }
    }

    pub(super) fn parameter_overrides_for_solver(&self) -> Vec<ParameterOverride> {
        let mut overrides = self.parameter_overrides.clone();
        overrides.extend(self.component_value_overrides.iter().map(|override_| {
            ParameterOverride {
                name: override_.parameter_name.clone(),
                value: override_.value,
            }
        }));
        overrides
    }
}

pub(super) fn analog_run_plans(analog: &AnalogScenario) -> Result<Vec<AnalogRunPlan>, String> {
    if analog.sweeps.is_empty() {
        return Ok(vec![AnalogRunPlan::nominal()]);
    }
    let mut plans = Vec::new();
    for sweep in &analog.sweeps {
        if sweep.name.trim().is_empty() {
            return Err("analog sweep names must not be empty.".to_string());
        }
        if sweep.parameters.is_empty()
            && sweep.component_values.is_empty()
            && sweep.model_sections.is_empty()
            && sweep.monte_carlo.is_none()
        {
            return Err(format!(
                "Analog sweep {} must declare at least one parameter, component value, model section, or Monte Carlo block.",
                sweep.name
            ));
        }
        let mut seen = BTreeSet::new();
        let mut parameter_values = Vec::new();
        for parameter in &sweep.parameters {
            if !valid_spice_parameter_name(&parameter.name) {
                return Err(format!(
                    "Analog sweep {} has invalid SPICE parameter name {}.",
                    sweep.name, parameter.name
                ));
            }
            if !seen.insert(parameter.name.clone()) {
                return Err(format!(
                    "Analog sweep {} declares parameter {} more than once.",
                    sweep.name, parameter.name
                ));
            }
            if parameter.values.is_empty() {
                return Err(format!(
                    "Analog sweep {} parameter {} must declare at least one value.",
                    sweep.name, parameter.name
                ));
            }
            if parameter.values.iter().any(|value| !value.is_finite()) {
                return Err(format!(
                    "Analog sweep {} parameter {} contains a non-finite value.",
                    sweep.name, parameter.name
                ));
            }
            parameter_values.push((parameter.name.clone(), parameter.values.clone()));
        }
        let mut component_value_seen = BTreeSet::new();
        let mut component_value_values = Vec::new();
        for component_value in &sweep.component_values {
            let component = component_value.component.trim();
            if component.is_empty() {
                return Err(format!(
                    "Analog sweep {} has a component value entry with an empty component.",
                    sweep.name
                ));
            }
            let key = (component.to_string(), component_value.field);
            if !component_value_seen.insert(key.clone()) {
                return Err(format!(
                    "Analog sweep {} declares component value {}.{} more than once.",
                    sweep.name,
                    key.0,
                    key.1.as_str()
                ));
            }
            if component_value.values.is_empty() {
                return Err(format!(
                    "Analog sweep {} component value {}.{} must declare at least one value.",
                    sweep.name,
                    component,
                    component_value.field.as_str()
                ));
            }
            if component_value.values.iter().any(|value| {
                !value.is_finite()
                    || component_value.field.requires_positive_value() && *value <= 0.0
            }) {
                return Err(format!(
                    "Analog sweep {} component value {}.{} contains a non-finite or out-of-range value.",
                    sweep.name,
                    component,
                    component_value.field.as_str()
                ));
            }
            component_value_values.push((
                component.to_string(),
                component_value.field,
                component_value.values.clone(),
            ));
        }
        let mut model_section_values = Vec::new();
        for model_section in &sweep.model_sections {
            if model_section.path.trim().is_empty() {
                return Err(format!(
                    "Analog sweep {} has a model section entry with an empty path.",
                    sweep.name
                ));
            }
            if model_section.sections.is_empty() {
                return Err(format!(
                    "Analog sweep {} model file {} must declare at least one section.",
                    sweep.name, model_section.path
                ));
            }
            for section in &model_section.sections {
                if !valid_spice_model_section_name(section) {
                    return Err(format!(
                        "Analog sweep {} model file {} has invalid section name {}.",
                        sweep.name, model_section.path, section
                    ));
                }
            }
            model_section_values.push((model_section.path.clone(), model_section.sections.clone()));
        }
        let mut parameter_combinations = Vec::new();
        expand_parameter_combinations(
            &parameter_values,
            0,
            Vec::new(),
            &mut parameter_combinations,
        )?;
        let mut model_section_combinations = Vec::new();
        expand_model_section_combinations(
            &model_section_values,
            0,
            Vec::new(),
            &mut model_section_combinations,
        )?;
        let mut component_value_combinations = Vec::new();
        expand_component_value_combinations(
            &component_value_values,
            0,
            Vec::new(),
            &mut component_value_combinations,
        )?;
        let monte_carlo_combinations = if let Some(monte_carlo) = &sweep.monte_carlo {
            validate_monte_carlo_criteria(&sweep.name, monte_carlo.criteria.as_ref())?;
            monte_carlo_component_value_samples(&sweep.name, monte_carlo, &component_value_seen)?
                .into_iter()
                .map(|sample| {
                    sample
                        .into_iter()
                        .map(|entry| ComponentValueOverride {
                            parameter_name: component_value_parameter_name(
                                &entry.component,
                                entry.field.as_str(),
                            ),
                            component: entry.component,
                            field: entry.field,
                            value: entry.value,
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        } else {
            vec![Vec::new()]
        };
        for (
            index,
            (
                parameter_overrides,
                component_value_overrides,
                monte_carlo_component_value_overrides,
                model_section_overrides,
            ),
        ) in
            parameter_combinations
                .into_iter()
                .flat_map(|parameter_overrides| {
                    component_value_combinations.iter().cloned().map(
                        move |component_value_overrides| {
                            (parameter_overrides.clone(), component_value_overrides)
                        },
                    )
                })
                .flat_map(|(parameter_overrides, component_value_overrides)| {
                    monte_carlo_combinations.iter().cloned().map(
                        move |monte_carlo_component_value_overrides| {
                            (
                                parameter_overrides.clone(),
                                component_value_overrides.clone(),
                                monte_carlo_component_value_overrides,
                            )
                        },
                    )
                })
                .flat_map(
                    |(
                        parameter_overrides,
                        component_value_overrides,
                        monte_carlo_component_value_overrides,
                    )| {
                        model_section_combinations.iter().cloned().map(
                            move |model_section_overrides| {
                                (
                                    parameter_overrides.clone(),
                                    component_value_overrides.clone(),
                                    monte_carlo_component_value_overrides.clone(),
                                    model_section_overrides,
                                )
                            },
                        )
                    },
                )
                .enumerate()
        {
            if plans.len() >= MAX_ANALOG_SWEEP_CORNERS {
                return Err(format!(
                    "Analog sweeps exceed the {MAX_ANALOG_SWEEP_CORNERS}-corner execution cap."
                ));
            }
            let corner_name = format!("corner_{:03}", index + 1);
            plans.push(AnalogRunPlan {
                sweep_name: Some(sweep.name.clone()),
                run_subdir: Some(format!("{}_{}", sweep.name, corner_name)),
                corner_name,
                parameter_overrides,
                component_value_overrides: component_value_overrides
                    .into_iter()
                    .chain(monte_carlo_component_value_overrides)
                    .collect(),
                model_section_overrides,
            });
        }
    }
    Ok(plans)
}

fn validate_monte_carlo_criteria(
    sweep_name: &str,
    criteria: Option<&AnalogMonteCarloCriteria>,
) -> Result<(), String> {
    let Some(criteria) = criteria else {
        return Ok(());
    };
    if let Some(min_yield_percent) = criteria.min_yield_percent
        && (!min_yield_percent.is_finite() || !(0.0..=100.0).contains(&min_yield_percent))
    {
        return Err(format!(
            "Analog sweep {sweep_name} Monte Carlo min_yield_percent must be between 0 and 100."
        ));
    }
    for (field, value) in [
        ("min_p1_margin", criteria.min_p1_margin),
        ("min_p5_margin", criteria.min_p5_margin),
        ("min_p50_margin", criteria.min_p50_margin),
        ("min_p95_margin", criteria.min_p95_margin),
    ] {
        if value.is_some_and(|value| !value.is_finite()) {
            return Err(format!(
                "Analog sweep {sweep_name} Monte Carlo {field} must be finite."
            ));
        }
    }
    Ok(())
}

fn expand_component_value_combinations(
    component_values: &[(String, AnalogSweepComponentField, Vec<f64>)],
    index: usize,
    current: Vec<ComponentValueOverride>,
    output: &mut Vec<Vec<ComponentValueOverride>>,
) -> Result<(), String> {
    if output.len() >= MAX_ANALOG_SWEEP_CORNERS {
        return Err(format!(
            "Analog sweeps exceed the {MAX_ANALOG_SWEEP_CORNERS}-corner execution cap."
        ));
    }
    let Some((component, field, values)) = component_values.get(index) else {
        output.push(current);
        return Ok(());
    };
    for value in values {
        let mut next = current.clone();
        next.push(ComponentValueOverride {
            component: component.clone(),
            field: *field,
            parameter_name: component_value_parameter_name(component, field.as_str()),
            value: *value,
        });
        expand_component_value_combinations(component_values, index + 1, next, output)?;
    }
    Ok(())
}

fn expand_parameter_combinations(
    parameters: &[(String, Vec<f64>)],
    index: usize,
    current: Vec<ParameterOverride>,
    output: &mut Vec<Vec<ParameterOverride>>,
) -> Result<(), String> {
    if output.len() >= MAX_ANALOG_SWEEP_CORNERS {
        return Err(format!(
            "Analog sweeps exceed the {MAX_ANALOG_SWEEP_CORNERS}-corner execution cap."
        ));
    }
    let Some((name, values)) = parameters.get(index) else {
        output.push(current);
        return Ok(());
    };
    for value in values {
        let mut next = current.clone();
        next.push(ParameterOverride {
            name: name.clone(),
            value: *value,
        });
        expand_parameter_combinations(parameters, index + 1, next, output)?;
    }
    Ok(())
}

fn expand_model_section_combinations(
    model_sections: &[(String, Vec<String>)],
    index: usize,
    current: Vec<ModelSectionOverride>,
    output: &mut Vec<Vec<ModelSectionOverride>>,
) -> Result<(), String> {
    if output.len() >= MAX_ANALOG_SWEEP_CORNERS {
        return Err(format!(
            "Analog sweeps exceed the {MAX_ANALOG_SWEEP_CORNERS}-corner execution cap."
        ));
    }
    let Some((path, sections)) = model_sections.get(index) else {
        output.push(current);
        return Ok(());
    };
    for section in sections {
        let mut next = current.clone();
        next.push(ModelSectionOverride {
            path: path.clone(),
            section: section.clone(),
        });
        expand_model_section_combinations(model_sections, index + 1, next, output)?;
    }
    Ok(())
}

fn valid_spice_parameter_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn valid_spice_model_section_name(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty()
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | ':'))
}

pub(super) fn netlist_source_name(source: &AnalogNetlistSource) -> &'static str {
    match source {
        AnalogNetlistSource::File => "file-backed",
        AnalogNetlistSource::GeneratedFromBoard => "generated-from-Board",
    }
}

pub(super) fn validate_netlist_source(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    artifacts: &mut Vec<String>,
) -> Option<Finding> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before netlist source validation");
    let netlist_finding = match analog.netlist_source {
        AnalogNetlistSource::File => {
            let Some(netlist) = &analog.netlist else {
                let mut finding = Finding::critical(
                    "ANALOG_NETLIST_UNAVAILABLE",
                    &scenario.name,
                    "analog.netlist is required when analog.netlist_source is file.",
                );
                finding
                    .limit
                    .insert("required_artifact".to_string(), json!("spice_netlist"));
                return Some(finding);
            };
            let netlist = bound.project.source_dir.join(netlist);
            if !netlist.is_file() {
                let mut finding = Finding::critical(
                    "ANALOG_NETLIST_UNAVAILABLE",
                    &scenario.name,
                    format!(
                        "SPICE netlist {} is required for physical analog simulation.",
                        netlist.display()
                    ),
                );
                finding
                    .limit
                    .insert("required_artifact".to_string(), json!("spice_netlist"));
                finding.suggested_fixes.push(
                    "Add a SPICE-compatible deck with device models for this board region."
                        .to_string(),
                );
                return Some(finding);
            }
            push_artifact(artifacts, &netlist);
            None
        }
        AnalogNetlistSource::GeneratedFromBoard => {
            if analog.generated.is_none() {
                return Some(Finding::critical(
                    "ANALOG_NETLIST_UNAVAILABLE",
                    &scenario.name,
                    "analog.generated is required when analog.netlist_source is generated_from_board.",
                ));
            }
            None
        }
    };
    if netlist_finding.is_some() {
        return netlist_finding;
    }
    validate_model_compiler_provenance(bound, scenario, artifacts)
}

pub(super) fn prepare_source_netlist(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    run_dir: &Path,
) -> Result<PathBuf, String> {
    let analog = scenario
        .analog
        .as_ref()
        .expect("analog was validated before source netlist preparation");
    match analog.netlist_source {
        AnalogNetlistSource::File => {
            let netlist = analog
                .netlist
                .as_ref()
                .ok_or_else(|| "analog.netlist is required for file netlist source.".to_string())?;
            Ok(bound.project.source_dir.join(netlist))
        }
        AnalogNetlistSource::GeneratedFromBoard => {
            let path = run_dir.join("generated_board.cir");
            generate_board_netlist(bound, analog, &path)?;
            Ok(path)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_ANALOG_SWEEP_CORNERS, analog_run_plans};
    use crate::board_ir::BoardProject;

    fn rc_sweep_project_yaml(parameter_yaml: &str) -> String {
        format!(
            r#"
project:
  name: sweep_test
  version: 0.1.0
board:
  name: sweep_test
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
          parameters:
{parameter_yaml}
      probes:
        - name: v_out
          expression: V(out)
      assertions: []
"#
        )
    }

    fn model_section_sweep_project_yaml(sweep_body_yaml: &str) -> String {
        format!(
            r#"
project:
  name: model_corner_test
  version: 0.1.0
board:
  name: model_corner_test
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
      model_files:
        - path: models/vendor.lib
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
        - name: model_corner
{sweep_body_yaml}
      probes:
        - name: v_out
          expression: V(out)
      assertions: []
"#
        )
    }

    #[test]
    fn analog_run_plans_expand_parameter_sweeps() {
        let yaml = rc_sweep_project_yaml(
            r#"            - name: RIN_VALUE
              values: [950.0, 1000.0]
            - name: COUT_VALUE
              values: [0.000000095, 0.0000001]
"#,
        );
        let project: BoardProject = serde_yaml_ng::from_str(&yaml).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();

        let plans = analog_run_plans(analog).unwrap();

        assert_eq!(plans.len(), 4);
        assert_eq!(plans[0].sweep_name.as_deref(), Some("rc_tolerance"));
        assert_eq!(plans[0].corner_name, "corner_001");
        assert_eq!(
            plans[0].run_subdir.as_deref(),
            Some("rc_tolerance_corner_001")
        );
        assert_eq!(plans[0].parameter_overrides[0].name, "RIN_VALUE");
        assert_eq!(plans[0].parameter_overrides[0].value, 950.0);
        assert_eq!(plans[0].parameter_overrides[1].name, "COUT_VALUE");
        assert_eq!(plans[0].parameter_overrides[1].value, 0.000000095);
        assert_eq!(plans[3].parameter_overrides[0].value, 1000.0);
        assert_eq!(plans[3].parameter_overrides[1].value, 0.0000001);
    }

    #[test]
    fn analog_run_plans_expand_model_section_sweeps() {
        let yaml = model_section_sweep_project_yaml(
            r#"          model_sections:
            - path: models/vendor.lib
              sections: [typ, slow, fast]
"#,
        );
        let project: BoardProject = serde_yaml_ng::from_str(&yaml).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();

        let plans = analog_run_plans(analog).unwrap();

        assert_eq!(plans.len(), 3);
        assert_eq!(plans[0].sweep_name.as_deref(), Some("model_corner"));
        assert_eq!(plans[0].corner_name, "corner_001");
        assert_eq!(
            plans[0].run_subdir.as_deref(),
            Some("model_corner_corner_001")
        );
        assert!(plans[0].parameter_overrides.is_empty());
        assert_eq!(
            plans[0].model_section_overrides[0].path,
            "models/vendor.lib"
        );
        assert_eq!(plans[0].model_section_overrides[0].section, "typ");
        assert_eq!(plans[2].model_section_overrides[0].section, "fast");
    }

    #[test]
    fn analog_run_plans_combine_parameter_and_model_section_sweeps() {
        let yaml = model_section_sweep_project_yaml(
            r#"          parameters:
            - name: R_LOAD
              values: [900.0, 1100.0]
          model_sections:
            - path: models/vendor.lib
              sections: [slow, fast]
"#,
        );
        let project: BoardProject = serde_yaml_ng::from_str(&yaml).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();

        let plans = analog_run_plans(analog).unwrap();

        assert_eq!(plans.len(), 4);
        assert_eq!(plans[0].parameter_overrides[0].value, 900.0);
        assert_eq!(plans[0].model_section_overrides[0].section, "slow");
        assert_eq!(plans[1].parameter_overrides[0].value, 900.0);
        assert_eq!(plans[1].model_section_overrides[0].section, "fast");
        assert_eq!(plans[2].parameter_overrides[0].value, 1100.0);
        assert_eq!(plans[2].model_section_overrides[0].section, "slow");
    }

    #[test]
    fn analog_run_plans_expand_component_value_sweeps() {
        let yaml = model_section_sweep_project_yaml(
            r#"          component_values:
            - component: RLOAD
              field: value_ohm
              values: [900.0, 1000.0]
            - component: ILOAD
              field: dc_a
              values: [0.01, 0.02]
"#,
        );
        let project: BoardProject = serde_yaml_ng::from_str(&yaml).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();

        let plans = analog_run_plans(analog).unwrap();

        assert_eq!(plans.len(), 4);
        assert!(plans[0].parameter_overrides.is_empty());
        assert_eq!(
            plans[0].component_value_overrides[0].parameter_name,
            "CCI_RLOAD_VALUE_OHM"
        );
        assert_eq!(plans[0].component_value_overrides[0].value, 900.0);
        assert_eq!(
            plans[0].component_value_overrides[1].parameter_name,
            "CCI_ILOAD_DC_A"
        );
        assert_eq!(plans[3].component_value_overrides[0].value, 1000.0);
        assert_eq!(plans[3].component_value_overrides[1].value, 0.02);
        let solver_overrides = plans[0].parameter_overrides_for_solver();
        assert_eq!(solver_overrides.len(), 2);
        assert_eq!(solver_overrides[0].name, "CCI_RLOAD_VALUE_OHM");
    }

    #[test]
    fn analog_run_plans_expand_monte_carlo_component_values() {
        let yaml = model_section_sweep_project_yaml(
            r#"          monte_carlo:
            samples: 4
            seed: 42
            component_values:
              - component: RLOAD
                field: value_ohm
                nominal: 1000.0
                tolerance_percent: 5.0
              - component: CLOAD
                field: value_f
                nominal: 0.0000001
                tolerance_percent: 10.0
"#,
        );
        let project: BoardProject = serde_yaml_ng::from_str(&yaml).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();

        let plans = analog_run_plans(analog).unwrap();

        assert_eq!(plans.len(), 4);
        assert_eq!(plans[0].sweep_name.as_deref(), Some("model_corner"));
        assert_eq!(
            plans[0].run_subdir.as_deref(),
            Some("model_corner_corner_001")
        );
        assert_eq!(plans[0].component_value_overrides.len(), 2);
        assert_eq!(
            plans[0].component_value_overrides[0].parameter_name,
            "CCI_RLOAD_VALUE_OHM"
        );
        assert!((950.0..=1050.0).contains(&plans[0].component_value_overrides[0].value));
        assert!((0.00000009..=0.00000011).contains(&plans[3].component_value_overrides[1].value));
    }

    #[test]
    fn analog_run_plans_reject_invalid_monte_carlo_criteria() {
        let yaml = model_section_sweep_project_yaml(
            r#"          monte_carlo:
            samples: 4
            seed: 42
            criteria:
              min_yield_percent: 101.0
            component_values:
              - component: RLOAD
                field: value_ohm
                nominal: 1000.0
                tolerance_percent: 5.0
"#,
        );
        let project: BoardProject = serde_yaml_ng::from_str(&yaml).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();

        let error = analog_run_plans(analog).unwrap_err();

        assert!(error.contains("min_yield_percent must be between 0 and 100"));
    }

    #[test]
    fn analog_run_plans_reject_invalid_parameter_names() {
        let yaml = rc_sweep_project_yaml(
            r#"            - name: 1BAD
              values: [1.0]
"#,
        );
        let project: BoardProject = serde_yaml_ng::from_str(&yaml).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();

        let error = analog_run_plans(analog).unwrap_err();

        assert!(error.contains("invalid SPICE parameter name"));
    }

    #[test]
    fn analog_run_plans_enforce_corner_cap() {
        let values = (0..=MAX_ANALOG_SWEEP_CORNERS)
            .map(|value| value.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let yaml = rc_sweep_project_yaml(&format!(
            r#"            - name: RIN_VALUE
              values: [{values}]
"#
        ));
        let project: BoardProject = serde_yaml_ng::from_str(&yaml).unwrap();
        let analog = project.scenarios[0].analog.as_ref().unwrap();

        let error = analog_run_plans(analog).unwrap_err();

        assert!(error.contains("64-corner execution cap"));
    }
}
