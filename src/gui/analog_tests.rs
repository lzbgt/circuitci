use super::analog::{
    AnalogAcScenarioDraft, AnalogAssertionDraft, AnalogAssertionUiStatus, AnalogCurrentProbeDraft,
    AnalogDcScenarioDraft, AnalogExpressionProbeDraft, AnalogNoiseScenarioDraft,
    AnalogPowerProbeDraft, AnalogProbeAssertionsRemoveDraft, AnalogProbeDraft,
    AnalogProbeRemoveDraft, AnalogScenarioDraft, analog_probe_assertion_summaries,
    append_analog_ac_scenario_with_project_path, append_analog_assertion,
    append_analog_current_probe, append_analog_dc_scenario_with_project_path,
    append_analog_expression_probe, append_analog_noise_scenario_with_project_path,
    append_analog_power_probe, append_analog_transient_scenario,
    append_analog_transient_scenario_with_project_path, append_analog_voltage_probe,
    remove_analog_assertions_for_probe, remove_analog_probe, unique_analog_assertion_name,
};
use crate::reports::{Finding, ValidationReport};
use std::path::Path;

fn editable_project_yaml() -> &'static str {
    "project:
  name: gui_analog_editor_test
  version: 0.1.0
board:
  components:
    V1:
      model: generic.analog.dc_voltage_source
      spice:
        primitive: dc_voltage_source
        dc_v: 5.0
      pins:
        P: rail_5v
        N: gnd
    R1:
      model: generic.analog.resistor
      spice:
        primitive: resistor
        value_ohm: 1000.0
      pins:
        A: rail_5v
        B: out
  nets:
    rail_5v:
      kind: power
      nominal_voltage: 5
      powered: true
    out:
      kind: digital_or_analog
    gnd:
      kind: ground
"
}

fn draft() -> AnalogScenarioDraft {
    AnalogScenarioDraft {
        name: "gui_transient".to_string(),
        ground_net: "gnd".to_string(),
        probe_net: "out".to_string(),
        probe_name: "out_voltage".to_string(),
        stop_time_us: 100.0,
        max_step_us: 1.0,
    }
}

fn assertion_draft() -> AnalogAssertionDraft {
    AnalogAssertionDraft {
        scenario_name: "gui_transient".to_string(),
        assertion_name: "out_above_min".to_string(),
        probe_name: "out_voltage".to_string(),
        reference_probe: String::new(),
        aggregation: "sample".to_string(),
        relation: "above".to_string(),
        threshold: 1.0,
        reference_threshold: 0.0,
        target: 0.0,
        tolerance: 0.1,
        at_us: 50.0,
        at_hz: 1000.0,
        start_us: 0.0,
        end_us: 100.0,
        time_limit_us: 50.0,
        frequency_limit_hz: 1000.0,
        duty_limit_percent: 50.0,
        count_limit: 1.0,
        overshoot_limit_percent: 10.0,
    }
}

#[test]
fn append_analog_assertion_emits_phase_delay_yaml() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let edited = append_analog_voltage_probe(
        &edited,
        &AnalogProbeDraft {
            scenario_name: "gui_transient".to_string(),
            net_id: "rail_5v".to_string(),
            probe_name: "input_voltage".to_string(),
        },
    )
    .unwrap();
    let mut assertion = assertion_draft();
    assertion.assertion_name = "out_lags_input".to_string();
    assertion.aggregation = "rising_phase_delay".to_string();
    assertion.relation = "below".to_string();
    assertion.probe_name = "out_voltage".to_string();
    assertion.reference_probe = "input_voltage".to_string();
    assertion.threshold = 0.0;
    assertion.reference_threshold = 0.0;
    assertion.start_us = 0.0;
    assertion.end_us = 100.0;
    assertion.time_limit_us = 10.0;

    let edited = append_analog_assertion(&edited, &assertion).unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let assertion = &project.scenarios[0].analog.as_ref().unwrap().assertions[0];
    assert_eq!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::RisingPhaseDelay
    );
    assert_eq!(assertion.reference_probe.as_deref(), Some("input_voltage"));
    assert_eq!(assertion.reference_threshold_v, Some(0.0));
    assert_eq!(assertion.threshold_v, Some(0.0));
    assert_eq!(assertion.time_limit_us, Some(10.0));
}

#[test]
fn append_analog_assertion_emits_setup_time_yaml() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let edited = append_analog_voltage_probe(
        &edited,
        &AnalogProbeDraft {
            scenario_name: "gui_transient".to_string(),
            net_id: "rail_5v".to_string(),
            probe_name: "clock_voltage".to_string(),
        },
    )
    .unwrap();
    let mut assertion = assertion_draft();
    assertion.assertion_name = "out_setup_before_clock".to_string();
    assertion.aggregation = "rising_setup_time".to_string();
    assertion.relation = "above".to_string();
    assertion.probe_name = "out_voltage".to_string();
    assertion.reference_probe = "clock_voltage".to_string();
    assertion.threshold = 0.5;
    assertion.reference_threshold = 1.2;
    assertion.start_us = 0.0;
    assertion.end_us = 100.0;
    assertion.time_limit_us = 5.0;

    let edited = append_analog_assertion(&edited, &assertion).unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let assertion = &project.scenarios[0].analog.as_ref().unwrap().assertions[0];
    assert_eq!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::RisingSetupTime
    );
    assert_eq!(assertion.reference_probe.as_deref(), Some("clock_voltage"));
    assert_eq!(assertion.reference_threshold_v, Some(1.2));
    assert_eq!(assertion.threshold_v, Some(0.5));
    assert_eq!(assertion.time_limit_us, Some(5.0));
}

fn probe_draft() -> AnalogProbeDraft {
    AnalogProbeDraft {
        scenario_name: "gui_transient".to_string(),
        net_id: "rail_5v".to_string(),
        probe_name: "rail_5v_voltage".to_string(),
    }
}

fn current_probe_draft() -> AnalogCurrentProbeDraft {
    AnalogCurrentProbeDraft {
        scenario_name: "gui_transient".to_string(),
        component_id: "V1".to_string(),
        probe_name: "v1_current".to_string(),
    }
}

fn power_probe_draft() -> AnalogPowerProbeDraft {
    AnalogPowerProbeDraft {
        scenario_name: "gui_transient".to_string(),
        component_id: "R1".to_string(),
        probe_name: "r1_power".to_string(),
    }
}

#[test]
fn append_analog_transient_scenario_emits_valid_yaml() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    assert_eq!(project.scenarios.len(), 1);
    let scenario = &project.scenarios[0];
    assert_eq!(scenario.name, "gui_transient");
    assert_eq!(scenario.scenario_type, "analog_transient");
    let analog = scenario.analog.as_ref().unwrap();
    assert_eq!(analog.generated.as_ref().unwrap().ground_net, "gnd");
    assert_eq!(analog.probes[0].expression, "V(out)");
    assert!(analog.assertions.is_empty());
}

#[test]
fn append_analog_ac_scenario_emits_valid_yaml() {
    let draft = AnalogAcScenarioDraft {
        name: "gui_bode".to_string(),
        ground_net: "gnd".to_string(),
        probe_net: "out".to_string(),
        probe_name: "out_gain".to_string(),
        start_frequency_hz: 10.0,
        stop_frequency_hz: 100_000.0,
        points_per_decade: 20,
    };
    let edited = append_analog_ac_scenario_with_project_path(
        editable_project_yaml(),
        Path::new("examples/generated_ac/project.yaml"),
        &draft,
    )
    .unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    assert_eq!(project.scenarios.len(), 1);
    let scenario = &project.scenarios[0];
    assert_eq!(scenario.name, "gui_bode");
    assert_eq!(scenario.scenario_type, "analog_ac");
    assert_eq!(scenario.checks, vec!["SPICE_AC_ANALYSIS".to_string()]);
    let analog = scenario.analog.as_ref().unwrap();
    assert_eq!(analog.generated.as_ref().unwrap().ground_net, "gnd");
    assert_eq!(analog.analysis.analysis_type, "ac");
    assert_eq!(analog.analysis.start_frequency_hz, Some(10.0));
    assert_eq!(analog.analysis.stop_frequency_hz, Some(100_000.0));
    assert_eq!(analog.analysis.points_per_decade, Some(20));
    assert_eq!(analog.probes[0].expression, "V(out)");
    assert!(analog.assertions.is_empty());
}

#[test]
fn append_analog_dc_scenario_emits_valid_yaml() {
    let draft = AnalogDcScenarioDraft {
        name: "gui_bias".to_string(),
        ground_net: "gnd".to_string(),
        probe_net: "out".to_string(),
        probe_name: "out_bias".to_string(),
    };
    let edited = append_analog_dc_scenario_with_project_path(
        editable_project_yaml(),
        Path::new("examples/generated_dc/project.yaml"),
        &draft,
    )
    .unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    assert_eq!(project.scenarios.len(), 1);
    let scenario = &project.scenarios[0];
    assert_eq!(scenario.name, "gui_bias");
    assert_eq!(scenario.scenario_type, "analog_dc");
    assert_eq!(scenario.checks, vec!["SPICE_DC_ANALYSIS".to_string()]);
    let analog = scenario.analog.as_ref().unwrap();
    assert_eq!(analog.generated.as_ref().unwrap().ground_net, "gnd");
    assert_eq!(analog.analysis.analysis_type, "op");
    assert_eq!(analog.probes[0].expression, "V(out)");
    assert!(analog.assertions.is_empty());
}

#[test]
fn append_analog_noise_scenario_emits_valid_yaml() {
    let draft = AnalogNoiseScenarioDraft {
        name: "gui_noise".to_string(),
        ground_net: "gnd".to_string(),
        probe_net: "out".to_string(),
        output_probe_name: "onoise".to_string(),
        input_probe_name: "inoise".to_string(),
        input_source: "V1".to_string(),
        start_frequency_hz: 10.0,
        stop_frequency_hz: 100_000.0,
        points_per_decade: 20,
    };
    let edited = append_analog_noise_scenario_with_project_path(
        editable_project_yaml(),
        Path::new("examples/generated_noise/project.yaml"),
        &draft,
    )
    .unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    assert_eq!(project.scenarios.len(), 1);
    let scenario = &project.scenarios[0];
    assert_eq!(scenario.name, "gui_noise");
    assert_eq!(scenario.scenario_type, "analog_noise");
    assert_eq!(scenario.checks, vec!["SPICE_NOISE_ANALYSIS".to_string()]);
    let analog = scenario.analog.as_ref().unwrap();
    assert_eq!(analog.generated.as_ref().unwrap().ground_net, "gnd");
    assert_eq!(analog.analysis.analysis_type, "noise");
    assert_eq!(analog.analysis.start_frequency_hz, Some(10.0));
    assert_eq!(analog.analysis.stop_frequency_hz, Some(100_000.0));
    assert_eq!(analog.analysis.points_per_decade, Some(20));
    assert_eq!(analog.analysis.noise_output_node.as_deref(), Some("out"));
    assert_eq!(analog.analysis.noise_input_source.as_deref(), Some("V1"));
    assert_eq!(analog.probes[0].name, "onoise");
    assert_eq!(analog.probes[0].expression, "V(out)");
    assert_eq!(analog.probes[1].name, "inoise");
    assert_eq!(analog.probes[1].expression, "V(rail_5v)");
    assert!(analog.assertions.is_empty());
}

#[test]
fn append_analog_assertion_emits_operating_point_yaml() {
    let draft = AnalogDcScenarioDraft {
        name: "gui_bias".to_string(),
        ground_net: "gnd".to_string(),
        probe_net: "out".to_string(),
        probe_name: "out_bias".to_string(),
    };
    let edited = append_analog_dc_scenario_with_project_path(
        editable_project_yaml(),
        Path::new("examples/generated_dc/project.yaml"),
        &draft,
    )
    .unwrap();
    let mut assertion = assertion_draft();
    assertion.scenario_name = "gui_bias".to_string();
    assertion.assertion_name = "out_bias_above_min".to_string();
    assertion.probe_name = "out_bias".to_string();
    assertion.aggregation = "operating_point".to_string();
    assertion.relation = "above".to_string();
    assertion.threshold = 2.4;
    let edited = append_analog_assertion(&edited, &assertion).unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let assertion = &project.scenarios[0].analog.as_ref().unwrap().assertions[0];
    assert_eq!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::OperatingPoint
    );
    assert_eq!(assertion.threshold_v, Some(2.4));
    assert_eq!(assertion.at_us, None);
    assert_eq!(assertion.start_us, None);
    assert_eq!(assertion.end_us, None);
}

#[test]
fn append_analog_transient_scenario_infers_model_pack_file() {
    let yaml = r#"
project:
  name: gui_model_pack_setup_test
  version: 0.1.0
libraries:
  - ../../libs/generic/analog
board:
  components:
    VCC:
      model: generic.analog.dc_voltage_source
      pins: {P: vcc_5v, N: gnd}
      spice: {primitive: dc_voltage_source, dc_v: 5.0}
    VIN:
      model: generic.analog.dc_voltage_source
      pins: {P: input, N: gnd}
      spice: {primitive: dc_voltage_source, dc_v: 1.0}
    XU1:
      model: generic.analog.ideal_opamp
      pins: {INP: input, INN: output, VCC: vcc_5v, VEE: gnd, OUT: output}
    RLOAD:
      model: generic.analog.resistor
      pins: {A: output, B: gnd}
      spice: {primitive: resistor, value_ohm: 10000.0}
  nets:
    vcc_5v: {kind: power, nominal_voltage: 5.0, powered: true}
    input: {kind: digital_or_analog}
    output: {kind: digital_or_analog}
    gnd: {kind: ground}
"#;
    let draft = AnalogScenarioDraft {
        name: "generated_model_pack".to_string(),
        ground_net: "gnd".to_string(),
        probe_net: "output".to_string(),
        probe_name: "output_voltage".to_string(),
        stop_time_us: 10.0,
        max_step_us: 0.1,
    };

    let edited = append_analog_transient_scenario_with_project_path(
        yaml,
        Path::new("examples/good_ideal_opamp_buffer/project.yaml"),
        &draft,
    )
    .unwrap();

    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let model_files = &project.scenarios[0].analog.as_ref().unwrap().model_files;
    assert_eq!(model_files.len(), 1);
    assert_eq!(
        model_files[0].path,
        "../../models/spice/generic/analog_behavioral.lib"
    );
    assert_eq!(
        model_files[0].sha256.as_deref(),
        Some("6d4392ef2b9d911fe02d705c2516deefb290ab2dc7a826b3ed65fe23a200b19b")
    );
}

#[test]
fn append_analog_transient_rejects_duplicate_scenario() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let error = append_analog_transient_scenario(&edited, &draft()).unwrap_err();
    assert!(error.to_string().contains("already exists"));
}

#[test]
fn append_analog_transient_rejects_missing_probe_net() {
    let mut draft = draft();
    draft.probe_net = "missing".to_string();
    let error = append_analog_transient_scenario(editable_project_yaml(), &draft).unwrap_err();
    assert!(error.to_string().contains("missing"));
}

#[test]
fn append_analog_assertion_emits_valid_yaml() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let edited = append_analog_assertion(&edited, &assertion_draft()).unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let assertion = &project.scenarios[0].analog.as_ref().unwrap().assertions[0];
    assert_eq!(assertion.name, "out_above_min");
    assert_eq!(assertion.threshold_v, Some(1.0));
    assert_eq!(assertion.at_us, Some(50.0));
}

#[test]
fn append_analog_assertion_emits_settling_time_yaml() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let mut assertion = assertion_draft();
    assertion.assertion_name = "out_settles".to_string();
    assertion.aggregation = "settling_time".to_string();
    assertion.relation = "below".to_string();
    assertion.target = 5.0;
    assertion.tolerance = 0.05;
    assertion.start_us = 0.0;
    assertion.end_us = 100.0;
    assertion.time_limit_us = 20.0;

    let edited = append_analog_assertion(&edited, &assertion).unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let assertion = &project.scenarios[0].analog.as_ref().unwrap().assertions[0];
    assert_eq!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::SettlingTime
    );
    assert_eq!(assertion.target_v, Some(5.0));
    assert_eq!(assertion.tolerance_v, Some(0.05));
    assert_eq!(assertion.time_limit_us, Some(20.0));
    assert_eq!(assertion.threshold_v, None);
}

#[test]
fn append_analog_assertion_emits_overshoot_yaml() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let mut assertion = assertion_draft();
    assertion.assertion_name = "out_overshoot".to_string();
    assertion.aggregation = "overshoot_percent".to_string();
    assertion.relation = "below".to_string();
    assertion.target = 5.0;
    assertion.overshoot_limit_percent = 8.0;
    assertion.start_us = 0.0;
    assertion.end_us = 100.0;

    let edited = append_analog_assertion(&edited, &assertion).unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let assertion = &project.scenarios[0].analog.as_ref().unwrap().assertions[0];
    assert_eq!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::OvershootPercent
    );
    assert_eq!(assertion.target_v, Some(5.0));
    assert_eq!(assertion.overshoot_limit_percent, Some(8.0));
    assert_eq!(assertion.threshold_v, None);
}

#[test]
fn append_analog_assertion_emits_crossing_time_yaml() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let mut draft = assertion_draft();
    draft.assertion_name = "out_rises_soon".to_string();
    draft.aggregation = "rising_crossing_time".to_string();
    draft.relation = "below".to_string();
    draft.start_us = 0.0;
    draft.end_us = 100.0;
    draft.time_limit_us = 25.0;
    let edited = append_analog_assertion(&edited, &draft).unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let assertion = &project.scenarios[0].analog.as_ref().unwrap().assertions[0];
    assert_eq!(assertion.name, "out_rises_soon");
    assert_eq!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::RisingCrossingTime
    );
    assert_eq!(assertion.threshold_v, Some(1.0));
    assert_eq!(assertion.start_us, Some(0.0));
    assert_eq!(assertion.end_us, Some(100.0));
    assert_eq!(assertion.time_limit_us, Some(25.0));
    assert_eq!(assertion.at_us, None);
}

#[test]
fn append_analog_assertion_emits_duty_cycle_yaml() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let mut draft = assertion_draft();
    draft.assertion_name = "out_duty_ok".to_string();
    draft.aggregation = "duty_cycle".to_string();
    draft.relation = "below".to_string();
    draft.start_us = 0.0;
    draft.end_us = 100.0;
    draft.duty_limit_percent = 55.0;
    let edited = append_analog_assertion(&edited, &draft).unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let assertion = &project.scenarios[0].analog.as_ref().unwrap().assertions[0];
    assert_eq!(assertion.name, "out_duty_ok");
    assert_eq!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::DutyCycle
    );
    assert_eq!(assertion.threshold_v, Some(1.0));
    assert_eq!(assertion.start_us, Some(0.0));
    assert_eq!(assertion.end_us, Some(100.0));
    assert_eq!(assertion.duty_limit_percent, Some(55.0));
    assert_eq!(assertion.at_us, None);
}

#[test]
fn append_analog_assertion_emits_crossing_count_yaml() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let mut draft = assertion_draft();
    draft.assertion_name = "out_no_recross".to_string();
    draft.aggregation = "crossing_count".to_string();
    draft.relation = "below".to_string();
    draft.start_us = 10.0;
    draft.end_us = 100.0;
    draft.count_limit = 1.0;
    let edited = append_analog_assertion(&edited, &draft).unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let assertion = &project.scenarios[0].analog.as_ref().unwrap().assertions[0];
    assert_eq!(assertion.name, "out_no_recross");
    assert_eq!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::CrossingCount
    );
    assert_eq!(assertion.threshold_v, Some(1.0));
    assert_eq!(assertion.start_us, Some(10.0));
    assert_eq!(assertion.end_us, Some(100.0));
    assert_eq!(assertion.count_limit, Some(1.0));
    assert_eq!(assertion.at_us, None);
}

#[test]
fn append_analog_assertion_emits_integral_threshold_yaml() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let mut draft = assertion_draft();
    draft.assertion_name = "out_volt_seconds".to_string();
    draft.aggregation = "integral".to_string();
    draft.relation = "below".to_string();
    draft.start_us = 0.0;
    draft.end_us = 100.0;
    draft.threshold = 2.5e-4;
    let edited = append_analog_assertion(&edited, &draft).unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let assertion = &project.scenarios[0].analog.as_ref().unwrap().assertions[0];
    assert_eq!(assertion.name, "out_volt_seconds");
    assert_eq!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::Integral
    );
    assert_eq!(assertion.threshold_vs, Some(2.5e-4));
    assert_eq!(assertion.threshold_v, None);
    assert_eq!(assertion.start_us, Some(0.0));
    assert_eq!(assertion.end_us, Some(100.0));
}

#[test]
fn append_analog_assertion_emits_power_energy_threshold_yaml() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let edited = append_analog_expression_probe(
        &edited,
        &AnalogExpressionProbeDraft {
            scenario_name: "gui_transient".to_string(),
            probe_name: "load_power".to_string(),
            expression: "V(out)*I(V1)".to_string(),
            quantity: "power".to_string(),
        },
    )
    .unwrap();
    let mut draft = assertion_draft();
    draft.assertion_name = "load_energy_limit".to_string();
    draft.probe_name = "load_power".to_string();
    draft.aggregation = "energy".to_string();
    draft.relation = "below".to_string();
    draft.start_us = 0.0;
    draft.end_us = 100.0;
    draft.threshold = 1.0e-6;
    let edited = append_analog_assertion(&edited, &draft).unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let analog = project.scenarios[0].analog.as_ref().unwrap();
    let assertion = &analog.assertions[0];
    assert_eq!(assertion.name, "load_energy_limit");
    assert_eq!(
        assertion.aggregation,
        crate::board_ir::AnalogAggregation::Energy
    );
    assert_eq!(assertion.threshold_j, Some(1.0e-6));
    assert_eq!(assertion.threshold_w, None);
}

#[test]
fn append_analog_assertion_rejects_duplicate_assertion() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let edited = append_analog_assertion(&edited, &assertion_draft()).unwrap();
    let error = append_analog_assertion(&edited, &assertion_draft()).unwrap_err();
    assert!(error.to_string().contains("already exists"));
}

#[test]
fn append_analog_assertion_rejects_out_of_range_sample() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let mut draft = assertion_draft();
    draft.at_us = 101.0;
    let error = append_analog_assertion(&edited, &draft).unwrap_err();
    assert!(error.to_string().contains("stop time"));
}

#[test]
fn analog_probe_assertion_summaries_show_pass_status_after_clean_report() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let edited = append_analog_assertion(&edited, &assertion_draft()).unwrap();
    let report = ValidationReport::from_parts(
        "project".to_string(),
        "profile".to_string(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        "validate".to_string(),
    );
    let rows =
        analog_probe_assertion_summaries(&edited, Some(&report), "gui_transient", "out_voltage")
            .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].status, AnalogAssertionUiStatus::Pass);
    assert_eq!(rows[0].threshold, "1.000000 V");
    assert_eq!(rows[0].timing, "at 50.000000 us");
}

#[test]
fn analog_probe_assertion_summaries_show_matching_failure() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let edited = append_analog_assertion(&edited, &assertion_draft()).unwrap();
    let failure = Finding::critical(
        "SPICE_TRANSIENT_ANALYSIS",
        "gui_transient",
        "Analog assertion out_above_min failed: sampled probe out_voltage measured 0.5 V.",
    );
    let report = ValidationReport::from_parts(
        "project".to_string(),
        "profile".to_string(),
        vec![failure],
        Vec::new(),
        Vec::new(),
        Vec::new(),
        "validate".to_string(),
    );
    let rows =
        analog_probe_assertion_summaries(&edited, Some(&report), "gui_transient", "out_voltage")
            .unwrap();
    assert_eq!(rows[0].status, AnalogAssertionUiStatus::Fail);
    assert!(rows[0].failure_message.as_ref().unwrap().contains("failed"));
}

#[test]
fn analog_probe_assertion_summaries_are_unknown_before_report() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let edited = append_analog_assertion(&edited, &assertion_draft()).unwrap();
    let rows =
        analog_probe_assertion_summaries(&edited, None, "gui_transient", "out_voltage").unwrap();
    assert_eq!(rows[0].status, AnalogAssertionUiStatus::Unknown);
}

#[test]
fn append_analog_voltage_probe_emits_valid_yaml() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let edited = append_analog_voltage_probe(&edited, &probe_draft()).unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let probes = &project.scenarios[0].analog.as_ref().unwrap().probes;
    assert!(probes.iter().any(|probe| {
        probe.name == "rail_5v_voltage"
            && probe.expression == "V(rail_5v)"
            && probe.quantity == crate::board_ir::AnalogQuantity::Voltage
    }));
}

#[test]
fn append_analog_voltage_probe_rejects_missing_node_binding() {
    let mut edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    edited = edited.replace("    - node: rail_5v\n      net: rail_5v\n", "");
    let error = append_analog_voltage_probe(&edited, &probe_draft()).unwrap_err();
    assert!(error.to_string().contains("node binding"));
}

#[test]
fn append_analog_expression_probe_emits_valid_yaml() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let edited = append_analog_expression_probe(
        &edited,
        &AnalogExpressionProbeDraft {
            scenario_name: "gui_transient".to_string(),
            probe_name: "load_power_math".to_string(),
            expression: "V(rail_5v,out)*I(VCCI_R1)".to_string(),
            quantity: "power".to_string(),
        },
    )
    .unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let probes = &project.scenarios[0].analog.as_ref().unwrap().probes;
    assert!(probes.iter().any(|probe| {
        probe.name == "load_power_math"
            && probe.expression == "V(rail_5v,out)*I(VCCI_R1)"
            && probe.quantity == crate::board_ir::AnalogQuantity::Power
    }));
}

#[test]
fn append_analog_expression_probe_rejects_quantity_mismatch() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let error = append_analog_expression_probe(
        &edited,
        &AnalogExpressionProbeDraft {
            scenario_name: "gui_transient".to_string(),
            probe_name: "bad_math".to_string(),
            expression: "V(out)".to_string(),
            quantity: "current".to_string(),
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("not consistent"));
}

#[test]
fn remove_analog_probe_drops_referencing_assertions() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let edited = append_analog_assertion(&edited, &assertion_draft()).unwrap();
    let edited = remove_analog_probe(
        &edited,
        &AnalogProbeRemoveDraft {
            scenario_name: "gui_transient".to_string(),
            probe_name: "out_voltage".to_string(),
        },
    )
    .unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let analog = project.scenarios[0].analog.as_ref().unwrap();
    assert!(
        analog
            .probes
            .iter()
            .all(|probe| probe.name != "out_voltage")
    );
    assert!(analog.assertions.is_empty());
}

#[test]
fn remove_analog_probe_rejects_missing_probe() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let error = remove_analog_probe(
        &edited,
        &AnalogProbeRemoveDraft {
            scenario_name: "gui_transient".to_string(),
            probe_name: "missing_probe".to_string(),
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("was not found"));
}

#[test]
fn remove_analog_assertions_for_probe_preserves_probe() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let edited = append_analog_assertion(&edited, &assertion_draft()).unwrap();
    let edited = remove_analog_assertions_for_probe(
        &edited,
        &AnalogProbeAssertionsRemoveDraft {
            scenario_name: "gui_transient".to_string(),
            probe_name: "out_voltage".to_string(),
        },
    )
    .unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let analog = project.scenarios[0].analog.as_ref().unwrap();
    assert!(
        analog
            .probes
            .iter()
            .any(|probe| probe.name == "out_voltage")
    );
    assert!(analog.assertions.is_empty());
}

#[test]
fn remove_analog_assertions_for_probe_rejects_probe_without_assertions() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let error = remove_analog_assertions_for_probe(
        &edited,
        &AnalogProbeAssertionsRemoveDraft {
            scenario_name: "gui_transient".to_string(),
            probe_name: "out_voltage".to_string(),
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("No analog assertions"));
}

#[test]
fn unique_analog_assertion_name_suffixes_collisions() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let edited = append_analog_assertion(&edited, &assertion_draft()).unwrap();
    let name = unique_analog_assertion_name(&edited, "gui_transient", "out_above_min").unwrap();
    assert_eq!(name, "out_above_min_2");
}

#[test]
fn append_analog_current_probe_emits_source_branch_current() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let edited =
        append_analog_current_probe(&edited, Path::new("project.yaml"), &current_probe_draft())
            .unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let probes = &project.scenarios[0].analog.as_ref().unwrap().probes;
    assert!(probes.iter().any(|probe| {
        probe.name == "v1_current"
            && probe.expression == "I(V1)"
            && probe.quantity == crate::board_ir::AnalogQuantity::Current
    }));
}

#[test]
fn append_analog_current_probe_emits_generated_semiconductor_sense_current() {
    let project_yaml = "project:
  name: gui_analog_current_editor_test
  version: 0.1.0
libraries:
  - libs/generic/analog
board:
  components:
    D-2:
      model: generic.analog.switching_diode
      pins:
        A: rail_5v
        K: out
  nets:
    rail_5v:
      kind: power
      nominal_voltage: 5
      powered: true
    out:
      kind: digital_or_analog
    gnd:
      kind: ground
";
    let edited = append_analog_transient_scenario(project_yaml, &draft()).unwrap();
    let draft = AnalogCurrentProbeDraft {
        scenario_name: "gui_transient".to_string(),
        component_id: "D-2".to_string(),
        probe_name: "d2_current".to_string(),
    };
    let edited = append_analog_current_probe(&edited, Path::new("project.yaml"), &draft).unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let probes = &project.scenarios[0].analog.as_ref().unwrap().probes;
    assert!(probes.iter().any(|probe| {
        probe.name == "d2_current"
            && probe.expression == "I(VCCI_D_2)"
            && probe.quantity == crate::board_ir::AnalogQuantity::Current
    }));
}

#[test]
fn append_analog_current_probe_emits_passive_branch_current_sense() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let draft = AnalogCurrentProbeDraft {
        scenario_name: "gui_transient".to_string(),
        component_id: "R1".to_string(),
        probe_name: "r1_current".to_string(),
    };
    let edited = append_analog_current_probe(&edited, Path::new("project.yaml"), &draft).unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let probes = &project.scenarios[0].analog.as_ref().unwrap().probes;
    assert!(probes.iter().any(|probe| {
        probe.name == "r1_current"
            && probe.expression == "I(VCCI_R1)"
            && probe.quantity == crate::board_ir::AnalogQuantity::Current
    }));
}

#[test]
fn append_analog_power_probe_emits_passive_branch_power() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let edited =
        append_analog_power_probe(&edited, Path::new("project.yaml"), &power_probe_draft())
            .unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let probes = &project.scenarios[0].analog.as_ref().unwrap().probes;
    assert!(probes.iter().any(|probe| {
        probe.name == "r1_power"
            && probe.expression == "V(rail_5v,out)*I(VCCI_R1)"
            && probe.quantity == crate::board_ir::AnalogQuantity::Power
    }));
}

#[test]
fn append_analog_power_probe_emits_source_branch_power() {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    let draft = AnalogPowerProbeDraft {
        scenario_name: "gui_transient".to_string(),
        component_id: "V1".to_string(),
        probe_name: "v1_power".to_string(),
    };
    let edited = append_analog_power_probe(&edited, Path::new("project.yaml"), &draft).unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let probes = &project.scenarios[0].analog.as_ref().unwrap().probes;
    assert!(probes.iter().any(|probe| {
        probe.name == "v1_power"
            && probe.expression == "V(rail_5v,0)*I(V1)"
            && probe.quantity == crate::board_ir::AnalogQuantity::Power
    }));
}

#[test]
fn append_analog_power_probe_rejects_missing_pin_binding() {
    let mut edited = append_analog_transient_scenario(editable_project_yaml(), &draft()).unwrap();
    edited = edited.replace(
        "    - node: out\n      endpoint:\n        component: R1\n        pin: B\n",
        "",
    );
    let error = append_analog_power_probe(&edited, Path::new("project.yaml"), &power_probe_draft())
        .unwrap_err();
    assert!(error.to_string().contains("pin binding"));
}
