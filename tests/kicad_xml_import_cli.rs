mod common;

use common::{
    assert_report_schema_valid, assert_yaml_file_valid, binary_available, run_validation,
};
use serde_json::Value;
use std::process::Command;

#[test]
fn import_kicad_netlist_generates_schema_valid_connectivity_project() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("imported_kicad.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-netlist",
            "examples/import_kicad_xml/board.net",
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_kicad_xml",
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&output, &validator);

    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(imported["project"]["import_source"], "kicad_xml_netlist");
    assert_eq!(imported["scenarios"], Value::Array(vec![]));
    assert_eq!(
        imported["board"]["components"]["R1"]["model"],
        "generic.schematic.imported_component"
    );
    assert_eq!(
        imported["board"]["components"]["U1"]["pins"]["7"],
        "net_reset_rc"
    );
    assert_eq!(imported["board"]["nets"]["gnd"]["kind"], "ground");
    assert_eq!(
        imported["board"]["nets"]["net_3v3"]["kind"],
        "digital_or_analog"
    );

    let report = run_validation(output.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert!(
        report["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|limitation| limitation["id"] == "SCHEMATIC_IMPORT_ONLY")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn import_kicad_netlist_applies_explicit_model_and_net_mapping() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("mapped_kicad.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-netlist",
            "examples/import_kicad_xml/board.net",
            "--mapping",
            "examples/import_kicad_xml/circuitci.kicad-map.yaml",
            "--output",
            output.to_str().unwrap(),
            "--name",
            "mapped_kicad_xml",
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&output, &validator);
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(
        imported["board"]["components"]["R1"]["model"],
        "generic.analog.resistor"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["A"],
        "net_3v3"
    );
    assert_eq!(
        imported["board"]["components"]["C1"]["model"],
        "generic.analog.capacitor"
    );
    assert_eq!(imported["board"]["components"]["C1"]["pins"]["B"], "gnd");
    assert_eq!(
        imported["board"]["components"]["V1"]["model"],
        "generic.analog.dc_voltage_source"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["spice"]["value_ohm"],
        10000.0
    );
    assert!(imported["board"]["components"]["R1"]["spice"]["value_ohm_from"].is_null());
    assert!(
        (imported["board"]["components"]["C1"]["spice"]["value_f"]
            .as_f64()
            .unwrap()
            - 100e-9)
            .abs()
            < 1e-18
    );
    assert!(imported["board"]["components"]["C1"]["spice"]["value_f_from"].is_null());
    assert_eq!(imported["board"]["components"]["V1"]["spice"]["dc_v"], 3.3);
    assert_eq!(imported["board"]["nets"]["net_3v3"]["kind"], "power");
    assert_eq!(imported["board"]["nets"]["net_3v3"]["nominal_voltage"], 3.3);
    assert_eq!(imported["board"]["nets"]["net_3v3"]["powered"], true);
    assert_eq!(
        imported["scenarios"][0]["name"],
        "kicad_mapped_rc_transient"
    );
    assert_eq!(
        imported["scenarios"][0]["analog"]["netlist_source"],
        "generated_from_board"
    );
    assert_eq!(
        imported["scenarios"][0]["analog"]["generated"]["components"],
        serde_json::json!(["V1", "R1", "D1", "C1"])
    );
    assert_eq!(
        imported["board"]["components"]["D1"]["model"],
        "vendor.onsemi.1n4148ws"
    );
    assert_eq!(
        imported["scenarios"][0]["analog"]["model_files"][0]["sha256"],
        "dee84e9189e05a9af600a0224a63cb6d01ebec4df27ff4ed12baeddd34869504"
    );
    assert!(
        imported["scenarios"][0]["analog"]["model_files"][0]["path"]
            .as_str()
            .unwrap()
            .ends_with("models/spice/onsemi/1n4148ws.lib")
    );
    assert_eq!(
        imported["scenarios"][0]["analog"]["assertions"][0]["name"],
        "rc_node_charges"
    );

    let report = run_validation(output.to_str().unwrap());
    assert_eq!(report["result"], "pass");
    assert!(!report["waveforms"].as_array().unwrap().is_empty());
    assert!(
        report["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|limitation| limitation["id"] == "SCHEMATIC_IMPORT_ONLY")
    );
    assert_report_schema_valid(&report);
}

#[test]
fn import_kicad_netlist_preserves_pin_electrical_types() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("typed.net");
    let mapping = dir.path().join("typed.kicad-map.yaml");
    let output = dir.path().join("typed.project.yaml");
    let repo = std::env::current_dir().unwrap();
    std::fs::write(
        &input,
        r#"<?xml version="1.0" encoding="utf-8"?>
<export version="E">
  <components>
    <comp ref="U1">
      <value>LogicBuffer</value>
      <libsource lib="Device" part="LogicBuffer"/>
    </comp>
  </components>
  <nets>
    <net code="1" name="IN">
      <node ref="U1" pin="1" pintype="input"/>
    </net>
    <net code="2" name="OUT">
      <node ref="U1" pin="2" pintype="output"/>
    </net>
  </nets>
</export>
"#,
    )
    .unwrap();
    std::fs::write(
        &mapping,
        format!(
            r#"
libraries:
  - {}
components:
  U1:
    model: generic.analog.resistor
    pin_map:
      "1": A
      "2": B
"#,
            repo.join("libs/generic").display()
        ),
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-netlist",
            input.to_str().unwrap(),
            "--mapping",
            mapping.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(
        imported["board"]["components"]["U1"]["source"]["kicad_pin_electrical_types"]["1"],
        "input"
    );
    assert_eq!(
        imported["board"]["components"]["U1"]["source"]["kicad_pin_electrical_types"]["2"],
        "output"
    );
    assert_eq!(
        imported["board"]["components"]["U1"]["source"]["board_pin_electrical_types"]["A"],
        "input"
    );
    assert_eq!(
        imported["board"]["components"]["U1"]["source"]["board_pin_electrical_types"]["B"],
        "output"
    );
}

#[test]
fn import_kicad_netlist_rejects_conflicting_pin_electrical_types() {
    assert_bad_kicad_mapping_for_input_contains(
        r#"<?xml version="1.0" encoding="utf-8"?>
<export version="E">
  <components>
    <comp ref="U1">
      <value>LogicBuffer</value>
      <libsource lib="Device" part="LogicBuffer"/>
    </comp>
  </components>
  <nets>
    <net code="1" name="IN">
      <node ref="U1" pin="1" pintype="input"/>
      <node ref="U1" pin="1" pintype="output"/>
    </net>
  </nets>
</export>
"#,
        r#"
components:
  U1:
    model: generic.schematic.imported_component
"#,
        "pin 1 has conflicting electrical types input and output",
    );
}

#[test]
fn import_kicad_netlist_applies_package_pin_alias() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let mapping = dir.path().join("alias.kicad-map.yaml");
    let output = dir.path().join("alias.project.yaml");
    std::fs::write(
        &mapping,
        format!(
            r#"
libraries:
  - {}
pin_aliases:
  two_terminal_ab:
    "1": A
    "2": B
components:
  R1:
    model: generic.analog.resistor
    pin_alias: two_terminal_ab
"#,
            std::env::current_dir()
                .unwrap()
                .join("libs/generic")
                .display()
        ),
    )
    .unwrap();

    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-netlist",
            "examples/import_kicad_xml/board.net",
            "--mapping",
            mapping.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--name",
            "alias_kicad_xml",
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    assert_eq!(
        imported["board"]["components"]["R1"]["model"],
        "generic.analog.resistor"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["A"],
        "net_3v3"
    );
    assert_eq!(
        imported["board"]["components"]["R1"]["pins"]["B"],
        "net_reset_rc"
    );
    assert!(imported["board"]["components"]["R1"]["pins"]["1"].is_null());
    assert!(imported["board"]["components"]["R1"]["pins"]["2"].is_null());
}

#[test]
fn import_kicad_netlist_maps_mosfet_soa_scenario() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("mapped_kicad_mosfet.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-netlist",
            "examples/import_kicad_mosfet/board.net",
            "--mapping",
            "examples/import_kicad_mosfet/circuitci.kicad-map.yaml",
            "--output",
            output.to_str().unwrap(),
            "--name",
            "mapped_kicad_mosfet",
        ])
        .status()
        .unwrap();
    assert!(status.success());

    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&output, &validator);
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let analog = &imported["scenarios"][0]["analog"];
    assert_eq!(
        imported["board"]["components"]["M1"]["model"],
        "vendor.onsemi.fdmc86184"
    );
    assert_eq!(
        imported["board"]["components"]["M1"]["pins"]["D"],
        "net_switched"
    );
    assert_eq!(imported["board"]["components"]["M1"]["pins"]["S"], "gnd");
    assert!(imported["board"]["components"]["M1"]["pins"]["B"].is_null());
    assert_eq!(
        analog["generated"]["components"],
        serde_json::json!(["VDD", "VGATE", "RLOAD", "M1"])
    );
    assert_eq!(analog["operating_conditions"]["allow_pulse_ratings"], true);
    assert_eq!(
        analog["model_files"][0]["sha256"],
        "c22b2f13d52a4545933f3d97588e0d626562e4813bda3ead62f103bd64e19c01"
    );
    assert!(
        analog["model_files"][0]["path"]
            .as_str()
            .unwrap()
            .ends_with("models/spice/onsemi/fdmc86184.lib")
    );

    let validate_out = dir.path().join("validate");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            output.to_str().unwrap(),
            "--profile",
            "iot_basic_v0",
            "--output",
            validate_out.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(validate_out.join("report.json")).unwrap())
            .unwrap();
    assert!(
        report["limitations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|limitation| limitation["id"] == "SCHEMATIC_IMPORT_ONLY")
    );
    let artifacts = report["artifacts"].as_array().unwrap();
    assert!(artifacts.iter().any(|artifact| {
        artifact
            .as_str()
            .unwrap()
            .ends_with("models/spice/onsemi/fdmc86184.lib")
    }));
    let generated_deck = artifacts
        .iter()
        .filter_map(|artifact| artifact.as_str())
        .find(|artifact| artifact.ends_with("generated_board.cir"))
        .expect("generated deck artifact");
    let generated_deck_text = std::fs::read_to_string(generated_deck).unwrap();
    assert!(generated_deck_text.contains("M1 cci_m1_d net_gate 0 0 ONSEMI_FDMC86184"));
    if binary_available("ngspice") {
        assert_eq!(report["result"], "fail");
        assert!(
            report["failures"]
                .as_array()
                .unwrap()
                .iter()
                .any(|failure| {
                    failure["id"] == "SPICE_OPERATING_LIMIT"
                        && failure["measured"]["component"] == "M1"
                        && failure["measured"]["rating"] == "SOA"
                        && failure["measured"]["soa_margin_ratio"].as_f64().unwrap() > 1.0
                        && failure["limit"]["soa_curve"] == "forward_bias_100us"
                })
        );
    } else {
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn import_kicad_netlist_rejects_duplicate_pin_assignment() {
    let dir = tempfile::tempdir().unwrap();
    let netlist = dir.path().join("bad.net");
    let output = dir.path().join("bad.project.yaml");
    std::fs::write(
        &netlist,
        r#"
<export>
  <components><comp ref="R1"><value>10k</value></comp></components>
  <nets>
    <net code="1" name="A"><node ref="R1" pin="1"/></net>
    <net code="2" name="B"><node ref="R1" pin="1"/></net>
  </nets>
</export>
"#,
    )
    .unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-netlist",
            netlist.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(!status.success());
    assert!(!output.exists());
}

#[test]
fn import_kicad_netlist_rejects_incomplete_real_model_pin_map() {
    assert_bad_kicad_mapping(
        r#"
components:
  R1:
    model: generic.analog.resistor
    pin_map:
      "1": A
"#,
    );
}

#[test]
fn import_kicad_netlist_rejects_unknown_mapped_model_pin() {
    assert_bad_kicad_mapping(
        r#"
components:
  R1:
    model: generic.analog.resistor
    pin_map:
      "1": A
      "2": Z
"#,
    );
}

#[test]
fn import_kicad_netlist_rejects_mapping_typos() {
    assert_bad_kicad_mapping(
        r#"
nets:
  +3V3:
    nominal_votlage: 3.3
"#,
    );
}

#[test]
fn import_kicad_netlist_rejects_unknown_pin_alias() {
    assert_bad_kicad_mapping_contains(
        r#"
components:
  R1:
    model: generic.analog.resistor
    pin_alias: missing_alias
"#,
        "references unknown pin_alias missing_alias",
    );
}

#[test]
fn import_kicad_netlist_rejects_pin_alias_and_direct_pin_map() {
    assert_bad_kicad_mapping_contains(
        r#"
pin_aliases:
  two_terminal_ab:
    "1": A
    "2": B
components:
  R1:
    model: generic.analog.resistor
    pin_alias: two_terminal_ab
    pin_map: { "1": A, "2": B }
"#,
        "cannot declare both pin_alias and pin_map",
    );
}

#[test]
fn import_kicad_netlist_rejects_empty_pin_alias() {
    assert_bad_kicad_mapping_contains(
        r#"
pin_aliases:
  empty: {}
"#,
        "pin_alias empty must declare at least one pin",
    );
}

#[test]
fn import_kicad_netlist_rejects_duplicate_target_in_pin_alias() {
    assert_bad_kicad_mapping_contains(
        r#"
pin_aliases:
  bad_duplicate:
    "1": A
    "2": A
"#,
        "maps more than one imported pin to model pin A",
    );
}

#[test]
fn import_kicad_netlist_rejects_passive_source_and_numeric_value() {
    assert_bad_kicad_mapping_contains(
        r#"
libsource_rules:
  - lib: Device
    part: R
    model: generic.analog.resistor
    pin_map: { "1": A, "2": B }
    spice:
      primitive: resistor
      value_ohm: 10000.0
      value_ohm_from: schematic_value
"#,
        "cannot declare both spice.value_ohm and spice.value_ohm_from",
    );
}

#[test]
fn import_kicad_netlist_rejects_wrong_passive_source_selector() {
    assert_bad_kicad_mapping_contains(
        r#"
libsource_rules:
  - lib: Device
    part: R
    model: generic.analog.resistor
    pin_map: { "1": A, "2": B }
    spice:
      primitive: resistor
      value_f_from: schematic_value
"#,
        "value_f_from only with primitive capacitor",
    );
}

#[test]
fn import_kicad_netlist_rejects_wrong_passive_quantity_field() {
    assert_bad_kicad_mapping_contains(
        r#"
libsource_rules:
  - lib: Device
    part: R
    model: generic.analog.resistor
    pin_map: { "1": A, "2": B }
    spice:
      primitive: resistor
      value_f: 0.0000001
"#,
        "primitive resistor may declare only spice.value_ohm",
    );
}

#[test]
fn import_kicad_netlist_rejects_annotated_resistor_value() {
    assert_bad_kicad_mapping_for_input_contains(
        &std::fs::read_to_string("examples/import_kicad_xml/board.net")
            .unwrap()
            .replace("<value>10k</value>", "<value>10k 1%</value>"),
        r#"
libsource_rules:
  - lib: Device
    part: R
    model: generic.analog.resistor
    pin_map: { "1": A, "2": B }
    spice:
      primitive: resistor
      value_ohm_from: schematic_value
"#,
        "not a strict positive ohm value",
    );
}

#[test]
fn import_kicad_netlist_rejects_plain_numeric_capacitance_value() {
    assert_bad_kicad_mapping_for_input_contains(
        &std::fs::read_to_string("examples/import_kicad_xml/board.net")
            .unwrap()
            .replace("<value>100n</value>", "<value>100</value>"),
        r#"
libsource_rules:
  - lib: Device
    part: C
    model: generic.analog.capacitor
    pin_map: { "1": A, "2": B }
    spice:
      primitive: capacitor
      value_f_from: schematic_value
"#,
        "not a strict positive farad value",
    );
}

#[test]
fn import_kicad_netlist_rejects_generated_scenario_without_assertions() {
    assert_bad_kicad_mapping(
        r#"
components:
  V1:
    model: generic.analog.dc_voltage_source
    pin_map: { "1": P, "2": N }
    spice: { primitive: dc_voltage_source, dc_v: 3.3 }
analog_scenarios:
  - name: no_assertions
    components: [V1]
    ground_net: GND
    analysis: { type: tran, stop_time_us: 100.0, max_step_us: 1.0 }
    stimuli:
      - { name: source, description: explicit source }
    probes:
      - { name: vdd, expression: V(net_3v3), quantity: voltage }
    assertions: []
"#,
    );
}

#[test]
fn import_kicad_netlist_rejects_generated_component_without_spice_metadata() {
    assert_bad_kicad_mapping(
        r#"
components:
  R1:
    model: generic.analog.resistor
    pin_map: { "1": A, "2": B }
analog_scenarios:
  - name: missing_spice
    components: [R1]
    ground_net: GND
    analysis: { type: tran, stop_time_us: 100.0, max_step_us: 1.0 }
    stimuli:
      - { name: source, description: intentionally incomplete }
    probes:
      - { name: rc, expression: V(net_reset_rc), quantity: voltage }
    assertions:
      - { name: must_have_assertion, probe: rc, at_us: 100.0, relation: above, threshold_v: 0.1 }
"#,
    );
}

#[test]
fn import_kicad_netlist_rejects_model_backed_component_without_model_file() {
    assert_bad_kicad_mapping_contains(
        r#"
components:
  D1:
    model: vendor.onsemi.1n4148ws
    pin_map: { "1": A, "2": K }
analog_scenarios:
  - name: missing_model_file
    components: [D1]
    ground_net: GND
    analysis: { type: tran, stop_time_us: 100.0, max_step_us: 1.0 }
    stimuli:
      - { name: diode, description: explicit diode model should require model file }
    probes:
      - { name: reset, expression: V(net_reset_rc), quantity: voltage }
    assertions:
      - { name: reset_sample, probe: reset, at_us: 100.0, relation: above, threshold_v: 0.1 }
"#,
        "scenario.model_files does not declare it",
    );
}

#[test]
fn import_kicad_netlist_rejects_model_file_without_sha() {
    assert_bad_kicad_mapping_contains(
        r#"
components:
  D1:
    model: vendor.onsemi.1n4148ws
    pin_map: { "1": A, "2": K }
analog_scenarios:
  - name: missing_sha
    components: [D1]
    ground_net: GND
    model_files:
      - path: ../../models/spice/onsemi/1n4148ws.lib
    analysis: { type: tran, stop_time_us: 100.0, max_step_us: 1.0 }
    stimuli:
      - { name: diode, description: explicit diode model should require sha }
    probes:
      - { name: reset, expression: V(net_reset_rc), quantity: voltage }
    assertions:
      - { name: reset_sample, probe: reset, at_us: 100.0, relation: above, threshold_v: 0.1 }
"#,
        "must declare sha256",
    );
}

#[test]
fn import_kicad_netlist_rejects_model_file_sha_mismatch() {
    assert_bad_kicad_mapping_contains(
        r#"
components:
  D1:
    model: vendor.onsemi.1n4148ws
    pin_map: { "1": A, "2": K }
analog_scenarios:
  - name: wrong_sha
    components: [D1]
    ground_net: GND
    model_files:
      - path: ../../models/spice/onsemi/1n4148ws.lib
        sha256: 0000000000000000000000000000000000000000000000000000000000000000
    analysis: { type: tran, stop_time_us: 100.0, max_step_us: 1.0 }
    stimuli:
      - { name: diode, description: explicit diode model should reject wrong sha }
    probes:
      - { name: reset, expression: V(net_reset_rc), quantity: voltage }
    assertions:
      - { name: reset_sample, probe: reset, at_us: 100.0, relation: above, threshold_v: 0.1 }
"#,
        "SHA-256 mismatch",
    );
}

fn assert_bad_kicad_mapping(mapping: &str) {
    let output = bad_kicad_mapping_output(mapping);
    assert!(!output.status.success());
}

fn assert_bad_kicad_mapping_contains(mapping: &str, expected: &str) {
    let output = bad_kicad_mapping_output(mapping);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected),
        "expected stderr to contain {expected:?}, got:\n{stderr}"
    );
}

fn assert_bad_kicad_mapping_for_input_contains(input: &str, mapping: &str, expected: &str) {
    let output = bad_kicad_mapping_output_for_input(input, mapping);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(expected),
        "expected stderr to contain {expected:?}, got:\n{stderr}"
    );
}

fn bad_kicad_mapping_output(mapping: &str) -> std::process::Output {
    bad_kicad_mapping_output_for_input(
        &std::fs::read_to_string("examples/import_kicad_xml/board.net").unwrap(),
        mapping,
    )
}

fn bad_kicad_mapping_output_for_input(input: &str, mapping: &str) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    let input_path = dir.path().join("board.net");
    let mapping_path = dir.path().join("bad.kicad-map.yaml");
    let output = dir.path().join("bad.project.yaml");
    let repo = std::env::current_dir().unwrap();
    let mapping = if mapping.contains("libraries:") {
        mapping.to_string()
    } else {
        format!(
            "libraries:\n  - {}\n  - {}\n{}",
            repo.join("libs/generic").display(),
            repo.join("libs/vendor/onsemi/diodes").display(),
            mapping
        )
    }
    .replace("../../models", &repo.join("models").to_string_lossy());
    std::fs::write(&input_path, input).unwrap();
    std::fs::write(&mapping_path, mapping).unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-netlist",
            input_path.to_str().unwrap(),
            "--mapping",
            mapping_path.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!output.exists());
    result
}
