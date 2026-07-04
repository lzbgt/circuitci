use super::*;
use std::path::Path;

#[test]
fn append_analog_fourier_scenario_emits_valid_yaml() {
    let draft = AnalogFourierScenarioDraft {
        name: "gui_fourier".to_string(),
        ground_net: "gnd".to_string(),
        probe_net: "out".to_string(),
        probe_name: "out_fourier".to_string(),
        stop_time_us: 100.0,
        max_step_us: 0.5,
        fundamental_frequency_hz: 100_000.0,
        harmonics: 8,
    };
    let edited = append_analog_fourier_scenario_with_project_path(
        editable_project_yaml(),
        Path::new("examples/generated_fourier/project.yaml"),
        &draft,
    )
    .unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let scenario = &project.scenarios[0];

    assert_eq!(scenario.name, "gui_fourier");
    assert_eq!(scenario.scenario_type, "analog_fourier");
    assert_eq!(scenario.checks, vec!["SPICE_FOURIER_ANALYSIS".to_string()]);
    let analog = scenario.analog.as_ref().unwrap();
    assert_eq!(analog.backend, crate::board_ir::AnalogBackend::Auto);
    assert_eq!(analog.generated.as_ref().unwrap().ground_net, "gnd");
    assert_eq!(analog.analysis.analysis_type, "fourier");
    assert_eq!(analog.analysis.stop_time_us, 100.0);
    assert_eq!(analog.analysis.max_step_us, 0.5);
    assert_eq!(
        analog.analysis.fourier_fundamental_frequency_hz,
        Some(100_000.0)
    );
    assert_eq!(analog.analysis.fourier_harmonics, Some(8));
    assert_eq!(
        analog.analysis.fourier_output_expression.as_deref(),
        Some("V(out)")
    );
    assert_eq!(analog.probes[0].name, "out_fourier");
    assert_eq!(analog.probes[0].expression, "V(out)");
}

#[test]
fn append_analog_measure_scenario_emits_valid_transient_yaml() {
    let draft = AnalogMeasureScenarioDraft {
        name: "gui_measure".to_string(),
        ground_net: "gnd".to_string(),
        probe_net: "out".to_string(),
        probe_name: "out_measure".to_string(),
        mode: "tran".to_string(),
        template_name: "avg_out".to_string(),
        operation: "avg".to_string(),
        from: 10.0,
        to: 80.0,
        stop_time_us: 100.0,
        max_step_us: 0.5,
        start_frequency_hz: 10.0,
        stop_frequency_hz: 100_000.0,
        points_per_decade: 20,
    };
    let edited = append_analog_measure_scenario_with_project_path(
        editable_project_yaml(),
        Path::new("examples/generated_measure/project.yaml"),
        &draft,
    )
    .unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let scenario = &project.scenarios[0];
    assert_eq!(scenario.name, "gui_measure");
    assert_eq!(scenario.scenario_type, "analog_measure");
    assert_eq!(scenario.checks, vec!["SPICE_MEASURE_ANALYSIS".to_string()]);
    let analog = scenario.analog.as_ref().unwrap();
    assert_eq!(analog.backend, crate::board_ir::AnalogBackend::Auto);
    assert_eq!(analog.analysis.analysis_type, "measure");
    assert_eq!(analog.analysis.measure_mode.as_deref(), Some("tran"));
    assert_eq!(analog.analysis.stop_time_us, 100.0);
    assert_eq!(analog.analysis.max_step_us, 0.5);
    let template = &analog.analysis.measure_templates[0];
    assert_eq!(template.name, "avg_out");
    assert_eq!(template.operation, "avg");
    assert_eq!(template.expression, "V(out)");
    assert_eq!(template.from_us, Some(10.0));
    assert_eq!(template.to_us, Some(80.0));
    assert_eq!(template.from_hz, None);
    assert_eq!(analog.probes[0].name, "out_measure");
    assert_eq!(analog.probes[0].expression, "V(out)");
}

#[test]
fn append_analog_measure_scenario_emits_valid_ac_yaml() {
    let draft = AnalogMeasureScenarioDraft {
        name: "gui_measure_ac".to_string(),
        ground_net: "gnd".to_string(),
        probe_net: "out".to_string(),
        probe_name: "out_measure_ac".to_string(),
        mode: "ac".to_string(),
        template_name: "max_out".to_string(),
        operation: "max".to_string(),
        from: 100.0,
        to: 10_000.0,
        stop_time_us: 100.0,
        max_step_us: 0.5,
        start_frequency_hz: 10.0,
        stop_frequency_hz: 100_000.0,
        points_per_decade: 20,
    };
    let edited = append_analog_measure_scenario_with_project_path(
        editable_project_yaml(),
        Path::new("examples/generated_measure/project.yaml"),
        &draft,
    )
    .unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let analog = project.scenarios[0].analog.as_ref().unwrap();
    assert_eq!(analog.analysis.measure_mode.as_deref(), Some("ac"));
    assert_eq!(analog.analysis.start_frequency_hz, Some(10.0));
    assert_eq!(analog.analysis.stop_frequency_hz, Some(100_000.0));
    assert_eq!(analog.analysis.points_per_decade, Some(20));
    let template = &analog.analysis.measure_templates[0];
    assert_eq!(template.name, "max_out");
    assert_eq!(template.operation, "max");
    assert_eq!(template.from_hz, Some(100.0));
    assert_eq!(template.to_hz, Some(10_000.0));
    assert_eq!(template.from_us, None);
}

#[test]
fn append_analog_measure_scenario_rejects_window_outside_ac_sweep() {
    let draft = AnalogMeasureScenarioDraft {
        name: "gui_measure_ac".to_string(),
        ground_net: "gnd".to_string(),
        probe_net: "out".to_string(),
        probe_name: "out_measure_ac".to_string(),
        mode: "ac".to_string(),
        template_name: "max_out".to_string(),
        operation: "max".to_string(),
        from: 1.0,
        to: 10_000.0,
        stop_time_us: 100.0,
        max_step_us: 0.5,
        start_frequency_hz: 10.0,
        stop_frequency_hz: 100_000.0,
        points_per_decade: 20,
    };
    let error = append_analog_measure_scenario_with_project_path(
        editable_project_yaml(),
        Path::new("examples/generated_measure/project.yaml"),
        &draft,
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("AC measure window must fit inside the AC sweep range")
    );
}

fn editable_project_yaml() -> &'static str {
    r#"
project:
  name: fourier_run_setup_test
  version: 0.1.0
board:
  components:
    V1:
      model: generic.analog.dc_voltage_source
      spice:
        primitive: pulse_voltage_source
        initial_v: 0.0
        pulsed_v: 1.0
        delay_us: 0.0
        rise_us: 0.1
        fall_us: 0.1
        width_us: 5.0
        period_us: 10.0
      pins:
        P: out
        N: gnd
    R1:
      model: generic.analog.resistor
      spice:
        primitive: resistor
        value_ohm: 1000.0
      pins:
        A: out
        B: gnd
  nets:
    out:
      kind: digital_or_analog
    gnd:
      kind: ground
"#
}
