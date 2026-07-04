use super::*;
use std::path::Path;

#[test]
fn append_analog_pss_scenario_emits_planning_yaml() {
    let draft = AnalogPssScenarioDraft {
        name: "gui_pss".to_string(),
        ground_net: "gnd".to_string(),
        probe_net: "out".to_string(),
        probe_name: "out_pss".to_string(),
        mode: "driven".to_string(),
        frequency_guess_hz: 100_000.0,
        stabilization_time_us: 100.0,
        periods: 3,
        drive_source: "V1".to_string(),
    };
    let project = parse_appended(
        append_analog_pss_scenario_with_project_path(
            periodic_project_yaml(),
            Path::new("examples/generated_pss/project.yaml"),
            &draft,
        )
        .unwrap(),
    );
    let scenario = &project.scenarios[0];
    assert_eq!(scenario.scenario_type, "analog_pss");
    assert_eq!(scenario.checks, vec!["SPICE_PSS_ANALYSIS".to_string()]);
    let analog = scenario.analog.as_ref().unwrap();
    assert_eq!(analog.analysis.analysis_type, "pss");
    assert_eq!(analog.analysis.pss_mode.as_deref(), Some("driven"));
    assert_eq!(analog.analysis.pss_frequency_guess_hz, Some(100_000.0));
    assert_eq!(analog.analysis.pss_stabilization_time_us, Some(100.0));
    assert_eq!(analog.analysis.pss_periods, Some(3));
    assert_eq!(
        analog.analysis.pss_output_expression.as_deref(),
        Some("V(out)")
    );
    assert_eq!(analog.analysis.pss_drive_sources, vec!["V1".to_string()]);
}

#[test]
fn append_analog_phase_noise_scenario_emits_planning_yaml() {
    let draft = AnalogPhaseNoiseScenarioDraft {
        name: "gui_phase_noise".to_string(),
        ground_net: "gnd".to_string(),
        probe_net: "out".to_string(),
        probe_name: "out_pn".to_string(),
        mode: "autonomous".to_string(),
        carrier_frequency_hz: 100_000.0,
        offset_start_hz: 10.0,
        offset_stop_hz: 1_000_000.0,
        points_per_decade: 20,
        drive_source: String::new(),
    };
    let project = parse_appended(
        append_analog_phase_noise_scenario_with_project_path(
            periodic_project_yaml(),
            Path::new("examples/generated_phase_noise/project.yaml"),
            &draft,
        )
        .unwrap(),
    );
    let scenario = &project.scenarios[0];
    assert_eq!(scenario.scenario_type, "analog_phase_noise");
    assert_eq!(
        scenario.checks,
        vec!["SPICE_PHASE_NOISE_ANALYSIS".to_string()]
    );
    let analog = scenario.analog.as_ref().unwrap();
    assert_eq!(analog.analysis.analysis_type, "phase_noise");
    assert_eq!(
        analog.analysis.phase_noise_mode.as_deref(),
        Some("autonomous")
    );
    assert_eq!(
        analog.analysis.phase_noise_carrier_frequency_hz,
        Some(100_000.0)
    );
    assert_eq!(analog.analysis.phase_noise_offset_start_hz, Some(10.0));
    assert_eq!(
        analog.analysis.phase_noise_offset_stop_hz,
        Some(1_000_000.0)
    );
    assert_eq!(analog.analysis.phase_noise_points_per_decade, Some(20));
    assert_eq!(
        analog.analysis.phase_noise_output_expression.as_deref(),
        Some("V(out)")
    );
    assert!(analog.analysis.phase_noise_drive_sources.is_empty());
}

#[test]
fn append_analog_periodic_ac_scenario_emits_planning_yaml() {
    let draft = AnalogPeriodicAcScenarioDraft {
        name: "gui_pac".to_string(),
        ground_net: "gnd".to_string(),
        probe_net: "out".to_string(),
        probe_name: "out_pac".to_string(),
        mode: "pxf".to_string(),
        carrier_frequency_hz: 100_000.0,
        start_frequency_hz: 10.0,
        stop_frequency_hz: 1_000_000.0,
        points_per_decade: 20,
        input_source: "V1".to_string(),
        sidebands: 4,
        drive_source: "V1".to_string(),
    };
    let project = parse_appended(
        append_analog_periodic_ac_scenario_with_project_path(
            periodic_project_yaml(),
            Path::new("examples/generated_periodic_ac/project.yaml"),
            &draft,
        )
        .unwrap(),
    );
    let scenario = &project.scenarios[0];
    assert_eq!(scenario.scenario_type, "analog_periodic_ac");
    assert_eq!(
        scenario.checks,
        vec!["SPICE_PERIODIC_AC_ANALYSIS".to_string()]
    );
    let analog = scenario.analog.as_ref().unwrap();
    assert_eq!(analog.analysis.analysis_type, "pac");
    assert_eq!(analog.analysis.pac_mode.as_deref(), Some("pxf"));
    assert_eq!(analog.analysis.pac_carrier_frequency_hz, Some(100_000.0));
    assert_eq!(analog.analysis.pac_start_frequency_hz, Some(10.0));
    assert_eq!(analog.analysis.pac_stop_frequency_hz, Some(1_000_000.0));
    assert_eq!(analog.analysis.pac_points_per_decade, Some(20));
    assert_eq!(
        analog.analysis.pac_output_expression.as_deref(),
        Some("V(out)")
    );
    assert_eq!(analog.analysis.pac_input_source.as_deref(), Some("V1"));
    assert_eq!(analog.analysis.pac_sidebands, Some(4));
    assert_eq!(analog.analysis.pac_drive_sources, vec!["V1".to_string()]);
}

#[test]
fn append_analog_pss_scenario_rejects_driven_without_source() {
    let draft = AnalogPssScenarioDraft {
        name: "gui_pss".to_string(),
        ground_net: "gnd".to_string(),
        probe_net: "out".to_string(),
        probe_name: "out_pss".to_string(),
        mode: "driven".to_string(),
        frequency_guess_hz: 100_000.0,
        stabilization_time_us: 100.0,
        periods: 3,
        drive_source: String::new(),
    };
    let error = append_analog_pss_scenario_with_project_path(
        periodic_project_yaml(),
        Path::new("examples/generated_pss/project.yaml"),
        &draft,
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("PSS drive source must not be blank")
    );
}

fn parse_appended(text: String) -> crate::board_ir::BoardProject {
    serde_yaml_ng::from_str(&text).unwrap()
}

fn periodic_project_yaml() -> &'static str {
    r#"
project:
  name: periodic_run_setup_test
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
