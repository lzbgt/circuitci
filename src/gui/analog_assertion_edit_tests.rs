use super::analog::{
    AnalogAssertionDraft, AnalogAssertionRemoveDraft, AnalogAssertionReplaceDraft,
    AnalogScenarioDraft, append_analog_assertion, append_analog_transient_scenario,
    remove_analog_assertion, replace_analog_assertion,
};

fn editable_project_yaml() -> &'static str {
    "project:
  name: gui_analog_assertion_edit_test
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

fn scenario_draft() -> AnalogScenarioDraft {
    AnalogScenarioDraft {
        name: "gui_transient".to_string(),
        ground_net: "gnd".to_string(),
        probe_net: "out".to_string(),
        probe_name: "out_voltage".to_string(),
        stop_time_us: 100.0,
        max_step_us: 1.0,
    }
}

fn assertion_draft(name: &str, threshold: f64) -> AnalogAssertionDraft {
    AnalogAssertionDraft {
        scenario_name: "gui_transient".to_string(),
        assertion_name: name.to_string(),
        probe_name: "out_voltage".to_string(),
        aggregation: "sample".to_string(),
        relation: "above".to_string(),
        threshold,
        at_us: 50.0,
        start_us: 0.0,
        end_us: 100.0,
        time_limit_us: 50.0,
        duty_limit_percent: 50.0,
        count_limit: 1.0,
    }
}

fn project_with_two_assertions() -> String {
    let edited = append_analog_transient_scenario(editable_project_yaml(), &scenario_draft())
        .expect("scenario should be valid");
    let edited = append_analog_assertion(&edited, &assertion_draft("out_above_min", 1.0))
        .expect("first assertion should be valid");
    append_analog_assertion(&edited, &assertion_draft("out_above_warn", 2.0))
        .expect("second assertion should be valid")
}

#[test]
fn remove_analog_assertion_preserves_other_assertions_and_probe() {
    let edited = remove_analog_assertion(
        &project_with_two_assertions(),
        &AnalogAssertionRemoveDraft {
            scenario_name: "gui_transient".to_string(),
            assertion_name: "out_above_min".to_string(),
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
    assert_eq!(analog.assertions.len(), 1);
    assert_eq!(analog.assertions[0].name, "out_above_warn");
}

#[test]
fn replace_analog_assertion_updates_one_check() {
    let mut replacement = assertion_draft("out_below_limit", 4.2);
    replacement.relation = "below".to_string();
    replacement.aggregation = "max".to_string();
    replacement.start_us = 10.0;
    replacement.end_us = 90.0;
    let edited = replace_analog_assertion(
        &project_with_two_assertions(),
        &AnalogAssertionReplaceDraft {
            scenario_name: "gui_transient".to_string(),
            original_assertion_name: "out_above_min".to_string(),
            replacement,
        },
    )
    .unwrap();

    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let analog = project.scenarios[0].analog.as_ref().unwrap();
    let updated = analog
        .assertions
        .iter()
        .find(|assertion| assertion.name == "out_below_limit")
        .unwrap();
    assert_eq!(updated.threshold_v, Some(4.2));
    assert_eq!(updated.start_us, Some(10.0));
    assert_eq!(updated.end_us, Some(90.0));
    assert!(
        analog
            .assertions
            .iter()
            .any(|assertion| assertion.name == "out_above_warn")
    );
}

#[test]
fn replace_analog_assertion_rejects_duplicate_name() {
    let error = replace_analog_assertion(
        &project_with_two_assertions(),
        &AnalogAssertionReplaceDraft {
            scenario_name: "gui_transient".to_string(),
            original_assertion_name: "out_above_min".to_string(),
            replacement: assertion_draft("out_above_warn", 3.0),
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("already exists"));
}
