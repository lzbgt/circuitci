use super::analog_sweeps::{
    AnalogMonteCarloCriteriaDraft, AnalogSweepComponentValueDraft, AnalogSweepModelSectionDraft,
    AnalogSweepParameterDraft, analog_load_sweep_candidates, analog_sweep_scenarios,
    append_analog_sweep_component_value, append_analog_sweep_model_section,
    append_analog_sweep_parameter, append_analog_sweep_preset,
    append_analog_sweep_with_component_value, append_analog_sweep_with_model_section,
    append_analog_sweep_with_parameter, remove_analog_sweep_component_value,
    remove_analog_sweep_model_section, remove_analog_sweep_parameter,
    set_analog_monte_carlo_criteria,
};

fn project_yaml() -> &'static str {
    "project:
  name: gui_sweep_test
  version: 0.1.0
board:
  components: {}
  nets:
    gnd:
      kind: ground
scenarios:
  - name: rc_run
    type: analog_transient
    analog:
      backend: auto
      netlist_source: file
      netlist: deck.cir
      model_files: []
      node_bindings:
        - { node: '0', net: gnd }
      pin_bindings: []
      analysis:
        type: tran
        stop_time_us: 1000
        max_step_us: 1
      stimuli: []
      probes:
        - name: v_out
          expression: V(out)
      assertions: []
"
}

fn generated_load_project_yaml() -> &'static str {
    "project:
  name: gui_load_sweep_test
  version: 0.1.0
board:
  components:
    RLOAD:
      model: generic.analog.resistor
      spice:
        primitive: resistor
        value_ohm: 1000.0
      pins:
        A: out
        B: gnd
    ILOAD:
      model: generic.analog.dc_current_source
      spice:
        primitive: dc_current_source
        dc_a: 0.01
      pins:
        P: out
        N: gnd
  nets:
    out:
      kind: digital_or_analog
    gnd:
      kind: ground
scenarios:
  - name: load_run
    type: analog_transient
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [RLOAD, ILOAD]
      model_files: []
      node_bindings:
        - { node: out, net: out }
        - { node: '0', net: gnd }
      pin_bindings: []
      analysis:
        type: tran
        stop_time_us: 1000
        max_step_us: 1
      stimuli: []
      probes:
        - name: v_out
          expression: V(out)
      assertions: []
"
}

fn monte_carlo_project_yaml() -> &'static str {
    "project:
  name: gui_monte_carlo_sweep_test
  version: 0.1.0
board:
  components: {}
  nets:
    gnd:
      kind: ground
scenarios:
  - name: rc_run
    type: analog_transient
    analog:
      backend: auto
      netlist_source: file
      netlist: deck.cir
      model_files: []
      node_bindings:
        - { node: '0', net: gnd }
      pin_bindings: []
      analysis:
        type: tran
        stop_time_us: 1000
        max_step_us: 1
      stimuli: []
      sweeps:
        - name: rc_monte_carlo
          monte_carlo:
            samples: 8
            seed: 7
            criteria:
              min_yield_percent: 95.0
              min_p5_margin: 0.1
            component_values:
              - component: RLOAD
                field: value_ohm
                nominal: 1000.0
                tolerance_percent: 5.0
              - component: CLOAD
                field: value_f
                nominal: 0.0000001
                tolerance_percent: 10.0
      probes:
        - name: v_out
          expression: V(out)
      assertions: []
"
}

#[test]
fn analog_sweep_scenarios_reports_monte_carlo_inputs() {
    let scenarios = analog_sweep_scenarios(monte_carlo_project_yaml()).unwrap();

    let sweep = &scenarios[0].sweeps[0];
    assert_eq!(sweep.name, "rc_monte_carlo");
    assert_eq!(sweep.corner_count, 8);
    let monte_carlo = sweep.monte_carlo.as_ref().unwrap();
    assert_eq!(monte_carlo.samples, 8);
    assert_eq!(monte_carlo.component_values[0].component, "RLOAD");
    assert_eq!(monte_carlo.component_values[0].field, "value_ohm");
    assert_eq!(monte_carlo.component_values[0].nominal, 1000.0);
    assert_eq!(monte_carlo.component_values[0].tolerance_percent, 5.0);
    assert_eq!(monte_carlo.component_values[0].distribution, "uniform");
    let criteria = monte_carlo.criteria.as_ref().unwrap();
    assert_eq!(criteria.min_yield_percent, Some(95.0));
    assert_eq!(criteria.min_p5_margin, Some(0.1));
    assert_eq!(criteria.min_p1_margin, None);
}

#[test]
fn set_monte_carlo_criteria_emits_valid_yaml() {
    let edited = set_analog_monte_carlo_criteria(
        monte_carlo_project_yaml(),
        &AnalogMonteCarloCriteriaDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "rc_monte_carlo".to_string(),
            min_yield_percent: "99.5".to_string(),
            min_p1_margin: "0.01".to_string(),
            min_p5_margin: "0.02".to_string(),
            min_p50_margin: "0.10".to_string(),
            min_p95_margin: "0.20".to_string(),
        },
    )
    .unwrap();

    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let criteria = project.scenarios[0].analog.as_ref().unwrap().sweeps[0]
        .monte_carlo
        .as_ref()
        .unwrap()
        .criteria
        .as_ref()
        .unwrap();
    assert_eq!(criteria.min_yield_percent, Some(99.5));
    assert_eq!(criteria.min_p1_margin, Some(0.01));
    assert_eq!(criteria.min_p5_margin, Some(0.02));
    assert_eq!(criteria.min_p50_margin, Some(0.10));
    assert_eq!(criteria.min_p95_margin, Some(0.20));
}

#[test]
fn clear_monte_carlo_criteria_removes_criteria() {
    let edited = set_analog_monte_carlo_criteria(
        monte_carlo_project_yaml(),
        &AnalogMonteCarloCriteriaDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "rc_monte_carlo".to_string(),
            min_yield_percent: String::new(),
            min_p1_margin: String::new(),
            min_p5_margin: String::new(),
            min_p50_margin: String::new(),
            min_p95_margin: String::new(),
        },
    )
    .unwrap();

    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    assert!(
        project.scenarios[0].analog.as_ref().unwrap().sweeps[0]
            .monte_carlo
            .as_ref()
            .unwrap()
            .criteria
            .is_none()
    );
}

#[test]
fn set_monte_carlo_criteria_rejects_invalid_yield_percent() {
    let error = set_analog_monte_carlo_criteria(
        monte_carlo_project_yaml(),
        &AnalogMonteCarloCriteriaDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "rc_monte_carlo".to_string(),
            min_yield_percent: "101".to_string(),
            min_p1_margin: String::new(),
            min_p5_margin: String::new(),
            min_p50_margin: String::new(),
            min_p95_margin: String::new(),
        },
    )
    .unwrap_err();

    assert!(error.to_string().contains("between 0 and 100"));
}

#[test]
fn set_monte_carlo_criteria_rejects_non_monte_carlo_sweep() {
    let edited = append_analog_sweep_with_parameter(
        project_yaml(),
        &AnalogSweepParameterDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "rc_tolerance".to_string(),
            parameter_name: "RIN_VALUE".to_string(),
            values_csv: "950, 1000, 1050".to_string(),
        },
    )
    .unwrap();
    let error = set_analog_monte_carlo_criteria(
        &edited,
        &AnalogMonteCarloCriteriaDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "rc_tolerance".to_string(),
            min_yield_percent: "95".to_string(),
            min_p1_margin: String::new(),
            min_p5_margin: "0".to_string(),
            min_p50_margin: String::new(),
            min_p95_margin: String::new(),
        },
    )
    .unwrap_err();

    assert!(error.to_string().contains("has no Monte Carlo block"));
}

#[test]
fn append_sweep_and_parameter_emit_valid_yaml() {
    let edited = append_analog_sweep_with_parameter(
        project_yaml(),
        &AnalogSweepParameterDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "rc_tolerance".to_string(),
            parameter_name: "RIN_VALUE".to_string(),
            values_csv: "950, 1000, 1050".to_string(),
        },
    )
    .unwrap();
    let edited = append_analog_sweep_parameter(
        &edited,
        &AnalogSweepParameterDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "rc_tolerance".to_string(),
            parameter_name: "COUT_VALUE".to_string(),
            values_csv: "9.5e-8, 1.0e-7".to_string(),
        },
    )
    .unwrap();

    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let sweep = &project.scenarios[0].analog.as_ref().unwrap().sweeps[0];
    assert_eq!(sweep.name, "rc_tolerance");
    assert_eq!(sweep.parameters.len(), 2);

    let scenarios = analog_sweep_scenarios(&edited).unwrap();
    assert_eq!(scenarios[0].sweeps[0].corner_count, 6);
}

#[test]
fn append_temperature_preset_emits_executable_sweep() {
    let edited = append_analog_sweep_preset(project_yaml(), "rc_run", "temperature").unwrap();

    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let sweep = &project.scenarios[0].analog.as_ref().unwrap().sweeps[0];
    assert_eq!(sweep.name, "temperature_corner");
    assert_eq!(sweep.parameters[0].name, "TEMP_C");
    assert_eq!(sweep.parameters[0].values, vec![-40.0, 25.0, 85.0]);

    let scenarios = analog_sweep_scenarios(&edited).unwrap();
    assert_eq!(scenarios[0].sweeps[0].corner_count, 3);
}

#[test]
fn append_rc_tolerance_preset_emits_nine_corners() {
    let edited = append_analog_sweep_preset(project_yaml(), "rc_run", "rc_tolerance").unwrap();

    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let sweep = &project.scenarios[0].analog.as_ref().unwrap().sweeps[0];
    assert_eq!(sweep.name, "rc_tolerance");
    assert_eq!(sweep.parameters.len(), 2);
    assert_eq!(sweep.parameters[0].name, "RIN_VALUE");
    assert_eq!(sweep.parameters[1].name, "COUT_VALUE");

    let scenarios = analog_sweep_scenarios(&edited).unwrap();
    assert_eq!(scenarios[0].sweeps[0].corner_count, 9);
}

#[test]
fn append_component_value_sweep_emits_valid_yaml() {
    let edited = append_analog_sweep_with_component_value(
        project_yaml(),
        &AnalogSweepComponentValueDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "load_corner".to_string(),
            component: "RLOAD".to_string(),
            field: "value_ohm".to_string(),
            values_csv: "900, 1000, 1100".to_string(),
        },
    )
    .unwrap();
    let edited = append_analog_sweep_component_value(
        &edited,
        &AnalogSweepComponentValueDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "load_corner".to_string(),
            component: "ILOAD".to_string(),
            field: "dc_a".to_string(),
            values_csv: "0.005, 0.01".to_string(),
        },
    )
    .unwrap();

    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let sweep = &project.scenarios[0].analog.as_ref().unwrap().sweeps[0];
    assert_eq!(sweep.component_values.len(), 2);
    assert_eq!(sweep.component_values[0].component, "RLOAD");

    let scenarios = analog_sweep_scenarios(&edited).unwrap();
    assert_eq!(scenarios[0].sweeps[0].corner_count, 6);
    assert_eq!(
        scenarios[0].sweeps[0].component_values[0].field,
        "value_ohm"
    );
}

#[test]
fn generated_load_sweep_candidates_project_component_fields() {
    let candidates =
        analog_load_sweep_candidates(generated_load_project_yaml(), "load_run").unwrap();

    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].component, "RLOAD");
    assert_eq!(candidates[0].field, "value_ohm");
    assert_eq!(candidates[0].values_csv, "900, 1000, 1100");
    assert_eq!(candidates[1].component, "ILOAD");
    assert_eq!(candidates[1].field, "dc_a");
    assert_eq!(candidates[1].values_csv, "0.009, 0.01, 0.011");
}

#[test]
fn remove_component_value_preserves_sweep_when_parameter_remains() {
    let edited = append_analog_sweep_with_parameter(
        project_yaml(),
        &AnalogSweepParameterDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "mixed_corner".to_string(),
            parameter_name: "RIN_VALUE".to_string(),
            values_csv: "950, 1050".to_string(),
        },
    )
    .unwrap();
    let edited = append_analog_sweep_component_value(
        &edited,
        &AnalogSweepComponentValueDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "mixed_corner".to_string(),
            component: "RLOAD".to_string(),
            field: "value_ohm".to_string(),
            values_csv: "900, 1100".to_string(),
        },
    )
    .unwrap();

    let edited = remove_analog_sweep_component_value(
        &edited,
        &AnalogSweepComponentValueDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "mixed_corner".to_string(),
            component: "RLOAD".to_string(),
            field: "value_ohm".to_string(),
            values_csv: String::new(),
        },
    )
    .unwrap();

    let scenarios = analog_sweep_scenarios(&edited).unwrap();
    assert_eq!(scenarios[0].sweeps[0].parameters.len(), 1);
    assert!(scenarios[0].sweeps[0].component_values.is_empty());
}

#[test]
fn append_model_section_emits_valid_corner_sweep() {
    let edited = append_analog_sweep_with_parameter(
        project_yaml(),
        &AnalogSweepParameterDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "model_corner".to_string(),
            parameter_name: "RIN_VALUE".to_string(),
            values_csv: "950, 1050".to_string(),
        },
    )
    .unwrap();
    let edited = append_analog_sweep_model_section(
        &edited,
        &AnalogSweepModelSectionDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "model_corner".to_string(),
            path: "models/vendor.lib".to_string(),
            sections_csv: "typ, slow, fast".to_string(),
        },
    )
    .unwrap();

    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let sweep = &project.scenarios[0].analog.as_ref().unwrap().sweeps[0];
    assert_eq!(sweep.model_sections[0].path, "models/vendor.lib");
    assert_eq!(
        sweep.model_sections[0].sections,
        vec!["typ", "slow", "fast"]
    );

    let scenarios = analog_sweep_scenarios(&edited).unwrap();
    assert_eq!(scenarios[0].sweeps[0].corner_count, 6);
    assert_eq!(
        scenarios[0].sweeps[0].model_sections[0].path,
        "models/vendor.lib"
    );
}

#[test]
fn append_sweep_with_model_section_emits_model_only_sweep() {
    let edited = append_analog_sweep_with_model_section(
        project_yaml(),
        &AnalogSweepModelSectionDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "model_corner".to_string(),
            path: "models/vendor.lib".to_string(),
            sections_csv: "typ, slow, fast".to_string(),
        },
    )
    .unwrap();

    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let sweep = &project.scenarios[0].analog.as_ref().unwrap().sweeps[0];
    assert!(sweep.parameters.is_empty());
    assert_eq!(sweep.model_sections[0].path, "models/vendor.lib");

    let scenarios = analog_sweep_scenarios(&edited).unwrap();
    assert_eq!(scenarios[0].sweeps[0].corner_count, 3);
}

#[test]
fn remove_model_section_preserves_sweep_when_parameter_remains() {
    let edited = append_analog_sweep_with_parameter(
        project_yaml(),
        &AnalogSweepParameterDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "model_corner".to_string(),
            parameter_name: "RIN_VALUE".to_string(),
            values_csv: "950, 1050".to_string(),
        },
    )
    .unwrap();
    let edited = append_analog_sweep_model_section(
        &edited,
        &AnalogSweepModelSectionDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "model_corner".to_string(),
            path: "models/vendor.lib".to_string(),
            sections_csv: "typ, slow".to_string(),
        },
    )
    .unwrap();

    let edited = remove_analog_sweep_model_section(
        &edited,
        &AnalogSweepModelSectionDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "model_corner".to_string(),
            path: "models/vendor.lib".to_string(),
            sections_csv: String::new(),
        },
    )
    .unwrap();

    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let sweep = &project.scenarios[0].analog.as_ref().unwrap().sweeps[0];
    assert!(sweep.model_sections.is_empty());
    assert_eq!(sweep.parameters.len(), 1);
}

#[test]
fn append_sweep_preset_rejects_duplicate_name() {
    let edited = append_analog_sweep_preset(project_yaml(), "rc_run", "load_1k").unwrap();

    let error = append_analog_sweep_preset(&edited, "rc_run", "load_1k")
        .unwrap_err()
        .to_string();

    assert!(error.contains("already exists"));
}

#[test]
fn sweep_parameter_remove_preserves_sweep_when_other_parameters_remain() {
    let edited = append_analog_sweep_with_parameter(
        project_yaml(),
        &AnalogSweepParameterDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "rc_tolerance".to_string(),
            parameter_name: "RIN_VALUE".to_string(),
            values_csv: "950, 1000, 1050".to_string(),
        },
    )
    .unwrap();
    let edited = append_analog_sweep_parameter(
        &edited,
        &AnalogSweepParameterDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "rc_tolerance".to_string(),
            parameter_name: "COUT_VALUE".to_string(),
            values_csv: "9.5e-8, 1.0e-7".to_string(),
        },
    )
    .unwrap();
    let edited = remove_analog_sweep_parameter(
        &edited,
        &AnalogSweepParameterDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "rc_tolerance".to_string(),
            parameter_name: "RIN_VALUE".to_string(),
            values_csv: String::new(),
        },
    )
    .unwrap();
    let scenarios = analog_sweep_scenarios(&edited).unwrap();

    assert_eq!(scenarios[0].sweeps[0].name, "rc_tolerance");
    assert_eq!(scenarios[0].sweeps[0].parameters.len(), 1);
    assert_eq!(scenarios[0].sweeps[0].parameters[0].name, "COUT_VALUE");
}

#[test]
fn sweep_parameter_remove_rejects_last_parameter() {
    let edited = append_analog_sweep_with_parameter(
        project_yaml(),
        &AnalogSweepParameterDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "rc_tolerance".to_string(),
            parameter_name: "RIN_VALUE".to_string(),
            values_csv: "950, 1000, 1050".to_string(),
        },
    )
    .unwrap();
    let error = remove_analog_sweep_parameter(
        &edited,
        &AnalogSweepParameterDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "rc_tolerance".to_string(),
            parameter_name: "RIN_VALUE".to_string(),
            values_csv: String::new(),
        },
    )
    .unwrap_err();

    assert!(error.to_string().contains("Remove the sweep instead"));
}

#[test]
fn sweep_parameter_rejects_invalid_spice_name() {
    let edited = append_analog_sweep_with_parameter(
        project_yaml(),
        &AnalogSweepParameterDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "rc_tolerance".to_string(),
            parameter_name: "RIN_VALUE".to_string(),
            values_csv: "950, 1000, 1050".to_string(),
        },
    )
    .unwrap();
    let error = append_analog_sweep_parameter(
        &edited,
        &AnalogSweepParameterDraft {
            scenario_name: "rc_run".to_string(),
            sweep_name: "rc_tolerance".to_string(),
            parameter_name: "1BAD".to_string(),
            values_csv: "1".to_string(),
        },
    )
    .unwrap_err();

    assert!(error.to_string().contains("SPICE parameter name"));
}
