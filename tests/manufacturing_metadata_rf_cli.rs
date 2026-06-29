mod common;

use common::read_suggestion_report;
use serde_yaml_ng::Value;
use std::process::Command;

#[test]
fn import_manufacturing_metadata_applies_rf_antenna_constraints() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("without_rf_constraints.project.yaml");
    let metadata = dir.path().join("rf_constraints.csv");
    let output = dir.path().join("with_rf_constraints.project.yaml");
    let manifest_output = output.with_extension("manufacturing.json");
    let suggestions_output = dir.path().join("suggestions.yaml");
    let mut project_yaml: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string("examples/scenario_suggestions_rf_antenna_keepout/project.yaml")
            .unwrap(),
    )
    .unwrap();
    remove_rf_antenna_constraints(&mut project_yaml);
    std::fs::write(&input, serde_yaml_ng::to_string(&project_yaml).unwrap()).unwrap();
    let csv_rows = [
        [
            "field",
            "value",
            "unit",
            "source",
            "notes",
            "name",
            "antenna_net",
            "layer",
            "polygon",
            "feed_component",
            "feed_pin",
            "matching_components",
            "max_matching_component_distance_mm",
            "reference_net",
            "elements",
            "frequency_min_mhz",
            "frequency_max_mhz",
            "frequency_mhz",
            "measurement_method",
            "min_measurement_count",
            "max_frequency_step_mhz",
        ],
        [
            "rf_antenna_keepout",
            "1.0",
            "mm",
            "antenna_layout_guide_rev_a",
            "reviewed antenna keepout",
            "chip_antenna_clearance",
            "ANT",
            "F.Cu",
            "0:0;10:0;10:10;0:10",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
        ],
        [
            "rf_antenna_feed_path",
            "10.0",
            "mm",
            "antenna_layout_guide_rev_a",
            "reviewed feed path",
            "chip_antenna_feed",
            "ANT",
            "",
            "",
            "ANT1",
            "A",
            "C1|L1",
            "2.0",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
        ],
        [
            "rf_antenna_matching_network",
            "pi",
            "",
            "rf_matching_review_rev_a",
            "reviewed matching topology",
            "chip_antenna_pi_match",
            "ANT",
            "",
            "",
            "",
            "",
            "",
            "",
            "GND",
            "series:L1:RFOUT:ANT;shunt:C2:RFOUT;shunt:C1:ANT",
            "",
            "",
            "",
            "",
            "",
            "",
        ],
        [
            "rf_antenna_measurement",
            "12.0",
            "dB",
            "vna_sweep_rev_a",
            "reviewed S11 sweep point",
            "chip_antenna_s11_2400",
            "ANT",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "2400.0",
            "vna_s11",
            "",
            "",
        ],
        [
            "rf_antenna_measurement",
            "14.0",
            "dB",
            "vna_sweep_rev_a",
            "reviewed S11 sweep point",
            "chip_antenna_s11_2450",
            "ANT",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "2450.0",
            "vna_s11",
            "",
            "",
        ],
        [
            "rf_antenna_measurement",
            "11.5",
            "dB",
            "vna_sweep_rev_a",
            "reviewed S11 sweep point",
            "chip_antenna_s11_2500",
            "ANT",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "2500.0",
            "vna_s11",
            "",
            "",
        ],
        [
            "rf_antenna_performance_limit",
            "10.0",
            "dB",
            "antenna_module_datasheet_rev_b",
            "reviewed return-loss requirement",
            "chip_antenna_2g4_limit",
            "ANT",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "",
            "2400.0",
            "2500.0",
            "",
            "",
            "3",
            "50.0",
        ],
    ];
    std::fs::write(
        &metadata,
        csv_rows
            .iter()
            .map(|row| row.join(","))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n",
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-manufacturing-metadata",
            "--project",
            input.to_str().unwrap(),
            "--metadata",
            metadata.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        command_output.status.success(),
        "{}",
        String::from_utf8_lossy(&command_output.stderr)
    );
    assert!(String::from_utf8_lossy(&command_output.stdout).contains("7 applied fields"));

    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    common::assert_yaml_file_valid(&output, &validator);
    let enriched: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let rf_antenna = &enriched["board"]["layout"]["constraints"]["rf_antenna"];
    let keepouts = rf_antenna["keepouts"].as_sequence().unwrap();
    assert_eq!(keepouts.len(), 1);
    assert_eq!(keepouts[0]["name"], "chip_antenna_clearance");
    assert_eq!(keepouts[0]["antenna_net"], "ANT");
    assert_eq!(keepouts[0]["layer"], "F.Cu");
    assert_eq!(keepouts[0]["min_copper_clearance_mm"], 1.0);
    assert_eq!(keepouts[0]["polygon"].as_sequence().unwrap().len(), 4);
    let feed_paths = rf_antenna["feed_paths"].as_sequence().unwrap();
    assert_eq!(feed_paths.len(), 1);
    assert_eq!(feed_paths[0]["name"], "chip_antenna_feed");
    assert_eq!(feed_paths[0]["feed_component"], "ANT1");
    assert_eq!(feed_paths[0]["feed_pin"], "A");
    assert_eq!(feed_paths[0]["matching_components"][0], "C1");
    assert_eq!(feed_paths[0]["matching_components"][1], "L1");
    assert_eq!(feed_paths[0]["max_feed_route_length_mm"], 10.0);
    assert_eq!(feed_paths[0]["max_matching_component_distance_mm"], 2.0);
    let matching_networks = rf_antenna["matching_networks"].as_sequence().unwrap();
    assert_eq!(matching_networks.len(), 1);
    assert_eq!(matching_networks[0]["name"], "chip_antenna_pi_match");
    assert_eq!(matching_networks[0]["antenna_net"], "ANT");
    assert_eq!(matching_networks[0]["topology"], "pi");
    assert_eq!(matching_networks[0]["reference_net"], "GND");
    assert_eq!(matching_networks[0]["source"], "rf_matching_review_rev_a");
    let elements = matching_networks[0]["elements"].as_sequence().unwrap();
    assert_eq!(elements.len(), 3);
    assert_eq!(elements[0]["role"], "series");
    assert_eq!(elements[0]["component"], "L1");
    assert_eq!(elements[0]["input_net"], "RFOUT");
    assert_eq!(elements[0]["output_net"], "ANT");
    assert_eq!(elements[1]["role"], "shunt");
    assert_eq!(elements[1]["signal_net"], "RFOUT");
    let measurements = rf_antenna["measurements"].as_sequence().unwrap();
    assert_eq!(measurements.len(), 3);
    assert_eq!(measurements[0]["name"], "chip_antenna_s11_2400");
    assert_eq!(measurements[0]["antenna_net"], "ANT");
    assert_eq!(measurements[0]["frequency_mhz"], 2400.0);
    assert_eq!(measurements[0]["return_loss_db"], 12.0);
    assert_eq!(measurements[0]["measurement_method"], "vna_s11");
    assert_eq!(measurements[0]["notes"], "reviewed S11 sweep point");
    assert_eq!(measurements[2]["name"], "chip_antenna_s11_2500");
    let performance_limits = rf_antenna["performance_limits"].as_sequence().unwrap();
    assert_eq!(performance_limits.len(), 1);
    assert_eq!(performance_limits[0]["name"], "chip_antenna_2g4_limit");
    assert_eq!(performance_limits[0]["antenna_net"], "ANT");
    assert_eq!(performance_limits[0]["min_return_loss_db"], 10.0);
    assert_eq!(performance_limits[0]["frequency_min_mhz"], 2400.0);
    assert_eq!(performance_limits[0]["frequency_max_mhz"], 2500.0);
    assert_eq!(performance_limits[0]["min_measurement_count"], 3);
    assert_eq!(performance_limits[0]["max_frequency_step_mhz"], 50.0);

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(manifest_output).unwrap()).unwrap();
    let manifest_schema: serde_json::Value = serde_json::from_str(include_str!(
        "../schemas/manufacturing_metadata_import.schema.json"
    ))
    .unwrap();
    let manifest_validator = jsonschema::validator_for(&manifest_schema).unwrap();
    if let Err(error) = manifest_validator.validate(&manifest) {
        panic!("Manufacturing metadata import manifest failed schema validation: {error}");
    }
    assert_eq!(manifest["schema_version"], "0.28.0");
    assert_eq!(
        manifest["rows"][0]["board_field"],
        "layout.constraints.rf_antenna.keepouts[]"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["polygon"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    assert_eq!(
        manifest["rows"][1]["board_field"],
        "layout.constraints.rf_antenna.feed_paths[]"
    );
    assert_eq!(
        manifest["rows"][1]["normalized_value"]["max_matching_component_distance_mm"],
        2.0
    );
    assert_eq!(
        manifest["rows"][2]["board_field"],
        "layout.constraints.rf_antenna.matching_networks[]"
    );
    assert_eq!(manifest["rows"][2]["normalized_value"]["topology"], "pi");
    assert_eq!(
        manifest["rows"][2]["normalized_value"]["elements"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        manifest["rows"][3]["board_field"],
        "layout.constraints.rf_antenna.measurements[]"
    );
    assert_eq!(
        manifest["rows"][3]["normalized_value"]["return_loss_db"],
        12.0
    );
    assert_eq!(
        manifest["rows"][6]["board_field"],
        "layout.constraints.rf_antenna.performance_limits[]"
    );
    assert_eq!(
        manifest["rows"][6]["normalized_value"]["min_return_loss_db"],
        10.0
    );
    assert_eq!(
        manifest["rows"][6]["normalized_value"]["min_measurement_count"],
        3
    );
    assert_eq!(
        manifest["rows"][6]["normalized_value"]["max_frequency_step_mhz"],
        50.0
    );

    let suggest_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "suggest-scenarios",
            output.to_str().unwrap(),
            "--output",
            suggestions_output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(suggest_status.success());
    let suggestions = read_suggestion_report(&suggestions_output);
    assert_runnable(&suggestions, "rf_antenna_keepout_chip_antenna_clearance");
    assert_runnable(&suggestions, "rf_antenna_feed_path_chip_antenna_feed");
    assert_runnable(
        &suggestions,
        "rf_antenna_matching_topology_chip_antenna_pi_match",
    );
    let measured = assert_runnable(
        &suggestions,
        "rf_antenna_measured_performance_sweep_chip_antenna_2g4_limit",
    );
    assert_eq!(
        measured["scenario"]["parameters"]["min_return_loss_db"],
        10.0
    );
    assert_eq!(
        measured["scenario"]["parameters"]["frequency_min_mhz"],
        2400.0
    );
    assert_eq!(
        measured["scenario"]["parameters"]["frequency_max_mhz"],
        2500.0
    );
    assert_eq!(
        measured["scenario"]["parameters"]["min_measurement_count"],
        3
    );
    assert_eq!(
        measured["scenario"]["parameters"]["max_frequency_step_mhz"],
        50.0
    );
    assert_eq!(
        measured["scenario"]["parameters"]["rf_measurements"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
}

#[test]
fn import_manufacturing_metadata_applies_rf_measurement_conditions() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("without_rf_conditions.project.yaml");
    let metadata = dir.path().join("rf_conditions.csv");
    let output = dir.path().join("with_rf_conditions.project.yaml");
    let manifest_output = output.with_extension("manufacturing.json");
    let suggestions_output = dir.path().join("suggestions.yaml");
    std::fs::write(
        &input,
        r#"project:
  name: rf_condition_import_fixture
  version: "1"
libraries: []
board:
  components: {}
  nets:
    ANT:
      kind: digital_or_analog
"#,
    )
    .unwrap();
    std::fs::write(
        &metadata,
        "field,value,unit,source,notes,name,antenna_net,frequency_mhz,measurement_method,measurement_condition,required_measurement_condition,frequency_min_mhz,frequency_max_mhz,fixture,cable_setup,enclosure_profile\n\
         rf_antenna_measurement_condition,,,rf_lab_plan_rev_a,reviewed enclosed measurement condition,enclosed_usb_fixture,,,,,,,,usb_enclosure_fixture,100mm_u_fl_to_sma,product_enclosure_closed\n\
         rf_antenna_measurement,14.0,dB,vna_sweep_rev_c,reviewed enclosed S11 point,chip_antenna_s11_2440,ANT,2440.0,vna_s11,enclosed_usb_fixture,,,,,,\n\
         rf_antenna_performance_limit,10.0,dB,antenna_module_datasheet_rev_c,reviewed enclosed S11 limit,chip_antenna_2g4_enclosed_limit,ANT,,,,enclosed_usb_fixture,2400.0,2500.0,,,\n",
    )
    .unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-manufacturing-metadata",
            "--project",
            input.to_str().unwrap(),
            "--metadata",
            metadata.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        command_output.status.success(),
        "{}",
        String::from_utf8_lossy(&command_output.stderr)
    );
    assert!(String::from_utf8_lossy(&command_output.stdout).contains("3 applied fields"));

    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    common::assert_yaml_file_valid(&output, &validator);
    let enriched: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let rf_antenna = &enriched["board"]["layout"]["constraints"]["rf_antenna"];
    let conditions = rf_antenna["measurement_conditions"].as_sequence().unwrap();
    assert_eq!(conditions.len(), 1);
    assert_eq!(conditions[0]["name"], "enclosed_usb_fixture");
    assert_eq!(conditions[0]["fixture"], "usb_enclosure_fixture");
    assert_eq!(conditions[0]["cable_setup"], "100mm_u_fl_to_sma");
    assert_eq!(
        conditions[0]["enclosure_profile"],
        "product_enclosure_closed"
    );
    let measurements = rf_antenna["measurements"].as_sequence().unwrap();
    assert_eq!(
        measurements[0]["measurement_condition"],
        "enclosed_usb_fixture"
    );
    let limits = rf_antenna["performance_limits"].as_sequence().unwrap();
    assert_eq!(
        limits[0]["required_measurement_condition"],
        "enclosed_usb_fixture"
    );

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(manifest_output).unwrap()).unwrap();
    let manifest_schema: serde_json::Value = serde_json::from_str(include_str!(
        "../schemas/manufacturing_metadata_import.schema.json"
    ))
    .unwrap();
    let manifest_validator = jsonschema::validator_for(&manifest_schema).unwrap();
    if let Err(error) = manifest_validator.validate(&manifest) {
        panic!("Manufacturing metadata import manifest failed schema validation: {error}");
    }
    assert_eq!(manifest["schema_version"], "0.28.0");
    assert_eq!(
        manifest["rows"][0]["board_field"],
        "layout.constraints.rf_antenna.measurement_conditions[]"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["fixture"],
        "usb_enclosure_fixture"
    );

    let suggest_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "suggest-scenarios",
            output.to_str().unwrap(),
            "--output",
            suggestions_output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(suggest_status.success());
    let suggestions = read_suggestion_report(&suggestions_output);
    let measured = assert_runnable(
        &suggestions,
        "rf_antenna_measured_performance_chip_antenna_s11_2440_chip_antenna_2g4_enclosed_limit",
    );
    assert_eq!(
        measured["scenario"]["parameters"]["measurement_condition"],
        "enclosed_usb_fixture"
    );
}

fn remove_rf_antenna_constraints(project_yaml: &mut Value) {
    let constraints = project_yaml["board"]["layout"]["constraints"]
        .as_mapping_mut()
        .unwrap();
    constraints.remove(Value::String("rf_antenna".to_string()));
}

fn assert_runnable<'a>(suggestions: &'a serde_json::Value, id: &str) -> &'a serde_json::Value {
    let suggestion = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == id)
        .unwrap_or_else(|| panic!("missing suggestion {id}"));
    assert_eq!(suggestion["runnable"], true, "{id}");
    assert!(suggestion.get("required_inputs").is_none(), "{id}");
    suggestion
}
