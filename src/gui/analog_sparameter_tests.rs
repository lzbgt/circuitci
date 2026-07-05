use super::analog::{
    AnalogSParameterAssertionDraft, AnalogSParameterNetworkAssertionDraft,
    AnalogSParameterNoiseAssertionDraft, AnalogSParameterReflectionDraft, AnalogScenarioDraft,
    append_analog_sparameter_assertion, append_analog_sparameter_network_assertion,
    append_analog_sparameter_noise_assertion, append_analog_transient_scenario,
    unique_analog_sparameter_assertion_name, unique_analog_sparameter_network_assertion_name,
    unique_analog_sparameter_noise_assertion_name,
};

fn sparameter_project_yaml() -> &'static str {
    "project: { name: gui_sparameter_editor_test, version: 0.1.0 }
board:
  components:
    R1:
      model: generic.analog.resistor
      pins: { A: port1, B: port2 }
      spice: { primitive: resistor, value_ohm: 50 }
    R2:
      model: generic.analog.resistor
      pins: { A: port2, B: gnd }
      spice: { primitive: resistor, value_ohm: 50 }
  nets:
    port1: { kind: digital_or_analog }
    port2: { kind: digital_or_analog }
    gnd: { kind: ground }
scenarios:
  - name: two_port_sparameter
    type: analog_sparameter
    checks: [SPICE_S_PARAMETER_ANALYSIS]
    analog:
      backend: xyce
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [R1, R2]
      model_files: []
      node_bindings:
        - { node: port1, net: port1 }
        - { node: port2, net: port2 }
        - { node: '0', net: gnd }
      pin_bindings:
        - { node: port1, endpoint: { component: R1, pin: A } }
        - { node: port2, endpoint: { component: R1, pin: B } }
        - { node: port2, endpoint: { component: R2, pin: A } }
        - { node: '0', endpoint: { component: R2, pin: B } }
      analysis:
        type: sparam
        start_frequency_hz: 1000000.0
        stop_frequency_hz: 1000000000.0
        points_per_decade: 20
        s_parameter_ports:
          - { name: p1, positive_node: port1, negative_node: '0', reference_impedance_ohm: 50.0 }
          - { name: p2, positive_node: port2, negative_node: '0', reference_impedance_ohm: 50.0 }
      stimuli:
        - { name: two_port_sweep, description: Planned two-port S-parameter sweep. }
      probes:
        - { name: s11, expression: 'S(p1,p1)' }
      assertions: []
"
}

fn sparameter_network_assertion_draft() -> AnalogSParameterNetworkAssertionDraft {
    AnalogSParameterNetworkAssertionDraft {
        scenario_name: "two_port_sparameter".to_string(),
        assertion_name: "stable_rollet_k".to_string(),
        metric: "rollet_k_min".to_string(),
        relation: "above".to_string(),
        threshold: 1.0,
        source_reflection: None,
        load_reflection: None,
    }
}

fn sparameter_assertion_draft() -> AnalogSParameterAssertionDraft {
    AnalogSParameterAssertionDraft {
        scenario_name: "two_port_sparameter".to_string(),
        assertion_name: "s11_return_loss_floor".to_string(),
        parameter: "s11".to_string(),
        metric: "return_loss_db".to_string(),
        aggregation: "min".to_string(),
        relation: "above".to_string(),
        threshold: 10.0,
    }
}

fn sparameter_noise_assertion_draft() -> AnalogSParameterNoiseAssertionDraft {
    AnalogSParameterNoiseAssertionDraft {
        scenario_name: "two_port_sparameter".to_string(),
        assertion_name: "rf_noise_figure_ceiling".to_string(),
        metric: "noise_figure_db_max".to_string(),
        relation: "below".to_string(),
        threshold: 3.0,
    }
}

fn non_sparameter_project_yaml() -> &'static str {
    "project:
  name: gui_analog_editor_test
  version: 0.1.0
board:
  components:
    V1:
      model: generic.analog.dc_voltage_source
      spice: { primitive: dc_voltage_source, dc_v: 5.0 }
      pins: { P: rail_5v, N: gnd }
    R1:
      model: generic.analog.resistor
      spice: { primitive: resistor, value_ohm: 1000.0 }
      pins: { A: rail_5v, B: out }
  nets:
    rail_5v: { kind: power, nominal_voltage: 5, powered: true }
    out: { kind: digital_or_analog }
    gnd: { kind: ground }
"
}

fn non_sparameter_draft() -> AnalogScenarioDraft {
    AnalogScenarioDraft {
        name: "gui_transient".to_string(),
        ground_net: "gnd".to_string(),
        probe_net: "out".to_string(),
        probe_name: "out_voltage".to_string(),
        stop_time_us: 100.0,
        max_step_us: 1.0,
    }
}
#[test]
fn append_sparameter_network_assertion_emits_analysis_yaml() {
    let edited = append_analog_sparameter_network_assertion(
        sparameter_project_yaml(),
        &sparameter_network_assertion_draft(),
    )
    .unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let assertions = &project.scenarios[0]
        .analog
        .as_ref()
        .unwrap()
        .analysis
        .s_parameter_network_assertions;
    assert_eq!(assertions.len(), 1);
    assert_eq!(assertions[0].name, "stable_rollet_k");
    assert_eq!(
        assertions[0].metric,
        crate::board_ir::AnalogSParameterNetworkMetric::RolletKMin
    );
    assert_eq!(
        assertions[0].relation,
        crate::board_ir::AnalogRelation::Above
    );
    assert_eq!(assertions[0].threshold, 1.0);
}

#[test]
fn append_sparameter_network_gain_assertion_emits_source_load_reflections() {
    let mut draft = sparameter_network_assertion_draft();
    draft.assertion_name = "transducer_gain_floor".to_string();
    draft.metric = "transducer_gain_db_min".to_string();
    draft.threshold = 3.0;
    draft.source_reflection = Some(AnalogSParameterReflectionDraft {
        real: 0.2,
        imaginary: -0.1,
    });
    draft.load_reflection = Some(AnalogSParameterReflectionDraft {
        real: -0.15,
        imaginary: 0.05,
    });

    let edited =
        append_analog_sparameter_network_assertion(sparameter_project_yaml(), &draft).unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let analysis = &project.scenarios[0].analog.as_ref().unwrap().analysis;

    let source = analysis.s_parameter_source_reflection.unwrap();
    assert_eq!(source.real, 0.2);
    assert_eq!(source.imaginary, -0.1);
    let load = analysis.s_parameter_load_reflection.unwrap();
    assert_eq!(load.real, -0.15);
    assert_eq!(load.imaginary, 0.05);
    let assertion = &analysis.s_parameter_network_assertions[0];
    assert_eq!(
        assertion.metric,
        crate::board_ir::AnalogSParameterNetworkMetric::TransducerGainDbMin
    );
    assert_eq!(assertion.threshold, 3.0);
}

#[test]
fn append_sparameter_network_gain_assertion_requires_source_reflection() {
    let mut draft = sparameter_network_assertion_draft();
    draft.assertion_name = "available_gain_floor".to_string();
    draft.metric = "available_gain_db_min".to_string();

    let error =
        append_analog_sparameter_network_assertion(sparameter_project_yaml(), &draft).unwrap_err();

    assert!(error.to_string().contains("requires source reflection"));
}

#[test]
fn append_sparameter_network_assertion_rejects_invalid_reflection() {
    let mut draft = sparameter_network_assertion_draft();
    draft.source_reflection = Some(AnalogSParameterReflectionDraft {
        real: 1.0,
        imaginary: 0.0,
    });

    let error =
        append_analog_sparameter_network_assertion(sparameter_project_yaml(), &draft).unwrap_err();

    assert!(error.to_string().contains("magnitude must be below 1"));
}

#[test]
fn unique_sparameter_network_assertion_name_suffixes_collisions() {
    let edited = append_analog_sparameter_network_assertion(
        sparameter_project_yaml(),
        &sparameter_network_assertion_draft(),
    )
    .unwrap();
    let name = unique_analog_sparameter_network_assertion_name(
        &edited,
        "two_port_sparameter",
        "stable_rollet_k",
    )
    .unwrap();
    assert_eq!(name, "stable_rollet_k_2");
}

#[test]
fn append_sparameter_noise_assertion_emits_analysis_yaml() {
    let edited = append_analog_sparameter_noise_assertion(
        sparameter_project_yaml(),
        &sparameter_noise_assertion_draft(),
    )
    .unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let assertions = &project.scenarios[0]
        .analog
        .as_ref()
        .unwrap()
        .analysis
        .s_parameter_noise_assertions;
    assert_eq!(assertions.len(), 1);
    assert_eq!(assertions[0].name, "rf_noise_figure_ceiling");
    assert_eq!(
        assertions[0].metric,
        crate::board_ir::AnalogSParameterNoiseMetric::NoiseFigureDbMax
    );
    assert_eq!(
        assertions[0].relation,
        crate::board_ir::AnalogRelation::Below
    );
    assert_eq!(assertions[0].threshold, 3.0);
}

#[test]
fn append_sparameter_noise_assertion_rejects_invalid_metric() {
    let mut draft = sparameter_noise_assertion_draft();
    draft.metric = "ordinary_noise_density".to_string();

    let error =
        append_analog_sparameter_noise_assertion(sparameter_project_yaml(), &draft).unwrap_err();

    assert!(error.to_string().contains("Unsupported S-parameter noise"));
}

#[test]
fn unique_sparameter_noise_assertion_name_suffixes_collisions() {
    let edited = append_analog_sparameter_noise_assertion(
        sparameter_project_yaml(),
        &sparameter_noise_assertion_draft(),
    )
    .unwrap();
    let name = unique_analog_sparameter_noise_assertion_name(
        &edited,
        "two_port_sparameter",
        "rf_noise_figure_ceiling",
    )
    .unwrap();
    assert_eq!(name, "rf_noise_figure_ceiling_2");
}

#[test]
fn append_sparameter_assertion_emits_analysis_yaml() {
    let edited = append_analog_sparameter_assertion(
        sparameter_project_yaml(),
        &sparameter_assertion_draft(),
    )
    .unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    let assertions = &project.scenarios[0]
        .analog
        .as_ref()
        .unwrap()
        .analysis
        .s_parameter_assertions;
    assert_eq!(assertions.len(), 1);
    assert_eq!(assertions[0].name, "s11_return_loss_floor");
    assert_eq!(assertions[0].parameter, "s11");
    assert_eq!(
        assertions[0].metric,
        crate::board_ir::AnalogSParameterMetric::ReturnLossDb
    );
    assert_eq!(
        assertions[0].aggregation,
        crate::board_ir::AnalogSParameterAggregation::Min
    );
    assert_eq!(
        assertions[0].relation,
        crate::board_ir::AnalogRelation::Above
    );
    assert_eq!(assertions[0].threshold, 10.0);
}

#[test]
fn unique_sparameter_assertion_name_suffixes_collisions() {
    let edited = append_analog_sparameter_assertion(
        sparameter_project_yaml(),
        &sparameter_assertion_draft(),
    )
    .unwrap();
    let name = unique_analog_sparameter_assertion_name(
        &edited,
        "two_port_sparameter",
        "s11_return_loss_floor",
    )
    .unwrap();
    assert_eq!(name, "s11_return_loss_floor_2");
}

#[test]
fn append_sparameter_assertion_rejects_incompatible_metric_parameter() {
    let mut assertion = sparameter_assertion_draft();
    assertion.parameter = "s21".to_string();
    let error =
        append_analog_sparameter_assertion(sparameter_project_yaml(), &assertion).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("requires a reflection parameter")
    );
}

#[test]
fn append_sparameter_assertion_accepts_impedance_metric_on_reflection_term() {
    let mut assertion = sparameter_assertion_draft();
    assertion.assertion_name = "s11_impedance_ceiling".to_string();
    assertion.metric = "impedance_magnitude_ohm".to_string();
    assertion.aggregation = "max".to_string();
    assertion.relation = "below".to_string();
    assertion.threshold = 75.0;
    let edited = append_analog_sparameter_assertion(sparameter_project_yaml(), &assertion).unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    assert_eq!(
        project.scenarios[0]
            .analog
            .as_ref()
            .unwrap()
            .analysis
            .s_parameter_assertions[0]
            .metric,
        crate::board_ir::AnalogSParameterMetric::ImpedanceMagnitudeOhm
    );
}

#[test]
fn append_sparameter_assertion_accepts_mismatch_loss_on_reflection_term() {
    let mut assertion = sparameter_assertion_draft();
    assertion.assertion_name = "s11_mismatch_loss_ceiling".to_string();
    assertion.metric = "mismatch_loss_db".to_string();
    assertion.aggregation = "max".to_string();
    assertion.relation = "below".to_string();
    assertion.threshold = 0.5;
    let edited = append_analog_sparameter_assertion(sparameter_project_yaml(), &assertion).unwrap();
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(&edited).unwrap();
    assert_eq!(
        project.scenarios[0]
            .analog
            .as_ref()
            .unwrap()
            .analysis
            .s_parameter_assertions[0]
            .metric,
        crate::board_ir::AnalogSParameterMetric::MismatchLossDb
    );
}

#[test]
fn append_sparameter_network_assertion_rejects_non_sparameter_scenario() {
    let edited =
        append_analog_transient_scenario(non_sparameter_project_yaml(), &non_sparameter_draft())
            .unwrap();
    let mut network_draft = sparameter_network_assertion_draft();
    network_draft.scenario_name = "gui_transient".to_string();
    let error = append_analog_sparameter_network_assertion(&edited, &network_draft).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("require an analog_sparameter scenario")
    );
}
