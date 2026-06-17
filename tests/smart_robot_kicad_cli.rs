mod common;

use common::{assert_report_schema_valid, assert_yaml_file_valid};
use serde_json::Value;
use std::process::Command;

#[test]
fn smart_robot_motion_core_kicad_schematic_imports_connectivity() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("motion_core_imported.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "demos/smart_robot/kicad/motion_core/root.kicad_sch",
            "--mapping",
            "demos/smart_robot/kicad/motion_core/circuitci.kicad-map.yaml",
            "--output",
            output.to_str().unwrap(),
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
    assert_eq!(imported["project"]["import_source"], "kicad_schematic");
    assert_eq!(
        imported["board"]["components"]["MOD1"]["model"],
        "vendor.sipeed.licheerv_nano_w"
    );
    assert_eq!(
        imported["board"]["components"]["U1"]["model"],
        "vendor.artery.at32f435_motion_core"
    );
    assert_eq!(
        imported["board"]["components"]["U3"]["pins"]["CANH"],
        "net_robot_canh"
    );
    assert_eq!(
        imported["board"]["components"]["UCAN_ESD1"]["pins"]["CANH"],
        "net_robot_canh"
    );
    assert_eq!(
        imported["board"]["components"]["RT_CAN1"]["spice"]["value_ohm"],
        120.0
    );
    assert_eq!(imported["board"]["nets"]["net_3v3_logic"]["kind"], "power");
    assert_eq!(
        imported["board"]["nets"]["net_3v3_logic"]["nominal_voltage"],
        3.3
    );
    assert_eq!(
        imported["board"]["components"]["MOD1"]["source"]["board_pin_electrical_types"]["UART0_TX_A16"],
        "output"
    );
    assert_eq!(
        imported["board"]["components"]["U1"]["source"]["board_pin_electrical_types"]["CAN_RX"],
        "input"
    );
}

#[test]
fn smart_robot_wheel_actuator_kicad_schematic_imports_connectivity() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("wheel_actuator_imported.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "demos/smart_robot/kicad/wheel_actuator/root.kicad_sch",
            "--mapping",
            "demos/smart_robot/kicad/wheel_actuator/circuitci.kicad-map.yaml",
            "--output",
            output.to_str().unwrap(),
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
    assert_eq!(imported["project"]["import_source"], "kicad_schematic");
    assert_eq!(
        imported["board"]["components"]["U1"]["model"],
        "vendor.artery.at32m416_motor_control"
    );
    assert_eq!(
        imported["board"]["components"]["U2"]["model"],
        "vendor.ti.drv8323"
    );
    assert_eq!(
        imported["board"]["components"]["U2"]["pins"]["INHA"],
        "net_drv_inha"
    );
    assert_eq!(
        imported["board"]["components"]["PWR_STAGE"]["pins"]["PHASE_U"],
        "net_phase_u"
    );
    assert_eq!(
        imported["board"]["components"]["REGEN1"]["model"],
        "demo.smart_robot.regen_clamp_design_envelope"
    );
    assert_eq!(
        imported["board"]["components"]["REGEN1"]["pins"]["BUS"],
        "net_vwheel_sw"
    );
    assert_eq!(
        imported["board"]["components"]["RSHUNT_U"]["pins"]["A"],
        "net_phase_u"
    );
    assert_eq!(
        imported["board"]["components"]["RSHUNT_U"]["pins"]["B"],
        "net_cur_u"
    );
    assert_eq!(
        imported["board"]["components"]["RSHUNT_U"]["spice"]["value_ohm"],
        0.005
    );
    assert_eq!(
        imported["board"]["components"]["M1"]["pins"]["PHASE_U"],
        "net_phase_u"
    );
    assert_eq!(
        imported["board"]["components"]["JACT1"]["pins"]["CANH"],
        "net_robot_canh"
    );
    assert_eq!(
        imported["board"]["components"]["UCAN_ESD1"]["pins"]["CANH"],
        "net_robot_canh"
    );
    assert_eq!(
        imported["board"]["components"]["RT_CAN1"]["spice"]["value_ohm"],
        120.0
    );
    assert_eq!(imported["board"]["nets"]["net_vwheel_sw"]["kind"], "power");
    assert_eq!(
        imported["board"]["nets"]["net_vwheel_sw"]["nominal_voltage"],
        7.4
    );
    assert_eq!(
        imported["board"]["components"]["U1"]["source"]["board_pin_electrical_types"]["PWM_UH"],
        "output"
    );
    assert_eq!(
        imported["board"]["components"]["U2"]["source"]["board_pin_electrical_types"]["INHA"],
        "input"
    );
    assert!(
        imported["scenarios"]
            .as_array()
            .unwrap()
            .iter()
            .any(|scenario| {
                scenario["name"] == "wheel_actuator_selected_load_evidence_gate"
                    && scenario["checks"][0] == "MODEL_QUALITY_REQUIRED"
                    && scenario["parameters"]["components"][0] == "M1"
                    && scenario["parameters"]["components"][1] == "REGEN1"
            }),
        "wheel KiCad import should preserve the model-quality sign-off gate: {imported:#?}"
    );
    assert!(
        imported["scenarios"]
            .as_array()
            .unwrap()
            .iter()
            .any(|scenario| {
                scenario["name"] == "wheel_actuator_bus_cable_budget"
                    && scenario["checks"][0] == "LOAD_CABLE_CURRENT_VALID"
                    && scenario["target"]["component"] == "PWR_STAGE"
                    && scenario["target"]["power_pin"] == "VM"
            }),
        "wheel KiCad import should preserve the cable-current sign-off gate: {imported:#?}"
    );
    assert!(
        imported["scenarios"]
            .as_array()
            .unwrap()
            .iter()
            .any(|scenario| {
                scenario["name"] == "wheel_actuator_bus_cable_thermal_derating"
                    && scenario["checks"][0] == "LOAD_CABLE_THERMAL_DERATING_VALID"
                    && scenario["target"]["component"] == "PWR_STAGE"
                    && scenario["target"]["power_pin"] == "VM"
            }),
        "wheel KiCad import should preserve the cable-thermal sign-off gate: {imported:#?}"
    );
    assert!(
        imported["scenarios"]
            .as_array()
            .unwrap()
            .iter()
            .any(|scenario| {
                scenario["name"] == "wheel_actuator_bus_cable_voltage_drop"
                    && scenario["checks"][0] == "LOAD_CABLE_VOLTAGE_DROP_VALID"
                    && scenario["target"]["component"] == "PWR_STAGE"
                    && scenario["target"]["power_pin"] == "VM"
            }),
        "wheel KiCad import should preserve the cable voltage-drop sign-off gate: {imported:#?}"
    );
}

#[test]
fn smart_robot_wheel_actuator_kicad_schematic_validates_model_quality_gate() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let imported_project = dir.path().join("wheel_actuator_imported.project.yaml");
    let report_dir = dir.path().join("report");
    let import_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "demos/smart_robot/kicad/wheel_actuator/root.kicad_sch",
            "--mapping",
            "demos/smart_robot/kicad/wheel_actuator/circuitci.kicad-map.yaml",
            "--output",
            imported_project.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(import_status.success());

    let validate_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            imported_project.to_str().unwrap(),
            "--output",
            report_dir.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(validate_status.success());

    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(report_dir.join("report.json")).unwrap())
            .unwrap();
    assert_eq!(report["result"], "fail");
    assert_report_schema_valid(&report);
    let failures = report["failures"].as_array().unwrap();
    let model_quality_components = failures
        .iter()
        .filter(|finding| finding["id"] == "MODEL_QUALITY_REQUIRED")
        .map(|finding| finding["component"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(model_quality_components, vec!["M1", "REGEN1"]);
    assert!(failures.iter().any(|finding| {
        finding["id"] == "VALIDATION_INPUT_MISSING"
            && finding["scenario"] == "wheel_actuator_bus_cable_budget"
            && finding["limit"]["required_input"] == "cable_current_rating_A"
    }));
    assert!(failures.iter().any(|finding| {
        finding["id"] == "VALIDATION_INPUT_MISSING"
            && finding["scenario"] == "wheel_actuator_bus_cable_thermal_derating"
            && finding["limit"]["required_input"] == "cable_temperature_rise_test_current_A"
    }));
    assert!(failures.iter().any(|finding| {
        finding["id"] == "VALIDATION_INPUT_MISSING"
            && finding["scenario"] == "wheel_actuator_bus_cable_voltage_drop"
            && finding["limit"]["required_input"] == "cable_loop_resistance_ohm"
    }));
}

#[test]
fn smart_robot_pmu_kicad_schematic_imports_connectivity() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("pmu_imported.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "demos/smart_robot/kicad/pmu/root.kicad_sch",
            "--mapping",
            "demos/smart_robot/kicad/pmu/circuitci.kicad-map.yaml",
            "--output",
            output.to_str().unwrap(),
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
    assert_eq!(imported["project"]["import_source"], "kicad_schematic");
    assert_eq!(
        imported["board"]["components"]["UCHG"]["model"],
        "vendor.ti.bq25798"
    );
    assert_eq!(
        imported["board"]["components"]["UCHG"]["pins"]["VBUS"],
        "net_vbus_pd"
    );
    assert_eq!(
        imported["board"]["components"]["UCHG"]["pins"]["SYS"],
        "net_sys_pmu"
    );
    assert_eq!(
        imported["board"]["components"]["U5V"]["pins"]["VSENSE"],
        "net_5v_sys"
    );
    assert_eq!(
        imported["board"]["components"]["U3V3"]["pins"]["VOS"],
        "net_3v3_logic"
    );
    assert_eq!(
        imported["board"]["components"]["U_SERVO_SW"]["pins"]["VOUT"],
        "net_vservo_sw"
    );
    assert_eq!(
        imported["board"]["components"]["U_WHEEL_SW"]["pins"]["VOUT"],
        "net_vwheel_sw"
    );
    assert_eq!(
        imported["board"]["components"]["SERVO_LOAD"]["pins"]["VCC"],
        "net_vservo_sw"
    );
    assert_eq!(
        imported["board"]["components"]["WHEEL_LOAD"]["pins"]["VCC"],
        "net_vwheel_sw"
    );
    assert_eq!(
        imported["board"]["components"]["L3V3"]["spice"]["value_h"],
        0.0000022
    );
    assert_eq!(
        imported["board"]["components"]["C3V3_OUT"]["spice"]["value_f"],
        0.000022
    );
    assert_eq!(imported["board"]["nets"]["net_vbus_pd"]["kind"], "power");
    assert_eq!(
        imported["board"]["nets"]["net_vbus_pd"]["nominal_voltage"],
        20.0
    );
    assert_eq!(
        imported["board"]["nets"]["net_vservo_sw"]["supply_current_limit_A"],
        4.0
    );
    assert_eq!(
        imported["board"]["components"]["UCHG"]["source"]["board_pin_electrical_types"]["SDA"],
        "bidirectional"
    );
    assert_eq!(
        imported["board"]["components"]["U3V3"]["source"]["board_pin_electrical_types"]["PG"],
        "output"
    );
    assert!(
        imported["scenarios"]
            .as_array()
            .unwrap()
            .iter()
            .any(|scenario| {
                scenario["name"] == "pmu_estop_switch_selected_part_gate"
                    && scenario["checks"][0] == "MODEL_QUALITY_REQUIRED"
                    && scenario["parameters"]["components"][0] == "U_SERVO_SW"
                    && scenario["parameters"]["components"][1] == "U_WHEEL_SW"
            }),
        "PMU KiCad import should preserve the e-stop switch selected-part gate: {imported:#?}"
    );
    assert!(
        imported["scenarios"]
            .as_array()
            .unwrap()
            .iter()
            .any(|scenario| {
                scenario["name"] == "pmu_servo_switch_budget"
                    && scenario["checks"][0] == "POWER_SWITCH_BUDGET_VALID"
                    && scenario["target"]["component"] == "SERVO_LOAD"
                    && scenario["parameters"]["switch_component"] == "U_SERVO_SW"
            }),
        "PMU KiCad import should preserve the servo switch budget gate: {imported:#?}"
    );
    assert!(
        imported["scenarios"]
            .as_array()
            .unwrap()
            .iter()
            .any(|scenario| {
                scenario["name"] == "pmu_wheel_switch_budget"
                    && scenario["checks"][0] == "POWER_SWITCH_BUDGET_VALID"
                    && scenario["target"]["component"] == "WHEEL_LOAD"
                    && scenario["parameters"]["switch_component"] == "U_WHEEL_SW"
            }),
        "PMU KiCad import should preserve the wheel switch budget gate: {imported:#?}"
    );
    assert!(
        imported["scenarios"]
            .as_array()
            .unwrap()
            .iter()
            .any(|scenario| {
                scenario["name"] == "pmu_servo_switch_reverse_current"
                    && scenario["checks"][0] == "POWER_SWITCH_REVERSE_CURRENT_VALID"
                    && scenario["parameters"]["switch_component"] == "U_SERVO_SW"
            }),
        "PMU KiCad import should preserve the servo switch reverse-current gate: {imported:#?}"
    );
    assert!(
        imported["scenarios"]
            .as_array()
            .unwrap()
            .iter()
            .any(|scenario| {
                scenario["name"] == "pmu_wheel_switch_reverse_current"
                    && scenario["checks"][0] == "POWER_SWITCH_REVERSE_CURRENT_VALID"
                    && scenario["parameters"]["switch_component"] == "U_WHEEL_SW"
            }),
        "PMU KiCad import should preserve the wheel switch reverse-current gate: {imported:#?}"
    );
    assert!(
        imported["scenarios"]
            .as_array()
            .unwrap()
            .iter()
            .any(|scenario| {
                scenario["name"] == "pmu_servo_switch_inrush"
                    && scenario["checks"][0] == "POWER_SWITCH_INRUSH_VALID"
                    && scenario["parameters"]["switch_component"] == "U_SERVO_SW"
            }),
        "PMU KiCad import should preserve the servo switch inrush gate: {imported:#?}"
    );
    assert!(
        imported["scenarios"]
            .as_array()
            .unwrap()
            .iter()
            .any(|scenario| {
                scenario["name"] == "pmu_wheel_switch_inrush"
                    && scenario["checks"][0] == "POWER_SWITCH_INRUSH_VALID"
                    && scenario["parameters"]["switch_component"] == "U_WHEEL_SW"
            }),
        "PMU KiCad import should preserve the wheel switch inrush gate: {imported:#?}"
    );
}

#[test]
fn smart_robot_pmu_kicad_schematic_validates_estop_switch_gate() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let imported_project = dir.path().join("pmu_imported.project.yaml");
    let import_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "demos/smart_robot/kicad/pmu/root.kicad_sch",
            "--mapping",
            "demos/smart_robot/kicad/pmu/circuitci.kicad-map.yaml",
            "--output",
            imported_project.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(import_status.success());

    let report_dir = dir.path().join("pmu_report");
    let validate_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            imported_project.to_str().unwrap(),
            "--output",
            report_dir.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(validate_status.success());

    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(report_dir.join("report.json")).unwrap())
            .unwrap();
    assert_eq!(report["result"], "fail");
    assert_report_schema_valid(&report);
    let failures = report["failures"].as_array().unwrap();
    let model_quality_components = failures
        .iter()
        .filter(|finding| finding["id"] == "MODEL_QUALITY_REQUIRED")
        .map(|finding| finding["component"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(model_quality_components, vec!["U_WHEEL_SW"]);
    assert!(
        failures.iter().any(|finding| {
            finding["id"] == "VALIDATION_INPUT_MISSING"
                && finding["scenario"] == "pmu_wheel_switch_budget"
                && finding["limit"]["required_input"] == "power_switch.current_limit_A"
        }),
        "imported PMU schematic must preserve the wheel switch current-limit blocker: {failures:#?}"
    );
    assert!(
        failures.iter().any(|finding| {
            finding["id"] == "VALIDATION_INPUT_MISSING"
                && finding["scenario"] == "pmu_wheel_switch_reverse_current"
                && finding["limit"]["required_input"]
                    == "power_switch.reverse_current_blocking_mode"
        }),
        "imported PMU schematic must preserve the wheel switch reverse-current blocker: {failures:#?}"
    );
    assert!(
        failures.iter().any(|finding| {
            finding["id"] == "VALIDATION_INPUT_MISSING"
                && finding["scenario"] == "pmu_wheel_switch_inrush"
                && finding["limit"]["required_input"] == "power_switch.max_inrush_current_A"
        }),
        "imported PMU schematic must preserve the wheel switch inrush blocker: {failures:#?}"
    );
    assert!(
        !failures.iter().any(|finding| finding["scenario"]
            .as_str()
            .is_some_and(|scenario| scenario.starts_with("pmu_servo_switch_"))),
        "imported PMU schematic must preserve the source-backed TPS25948 servo switch selection: {failures:#?}"
    );
}

#[test]
fn smart_robot_servo_payload_kicad_schematic_imports_connectivity() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("servo_payload_imported.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "demos/smart_robot/kicad/servo_payload/root.kicad_sch",
            "--mapping",
            "demos/smart_robot/kicad/servo_payload/circuitci.kicad-map.yaml",
            "--output",
            output.to_str().unwrap(),
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
    assert_eq!(imported["project"]["import_source"], "kicad_schematic");
    assert_eq!(
        imported["board"]["components"]["U1"]["model"],
        "vendor.artery.at32f435_motion_core"
    );
    assert_eq!(
        imported["board"]["components"]["U2"]["model"],
        "vendor.nxp.pca9685"
    );
    assert_eq!(
        imported["board"]["components"]["U1"]["pins"]["I2C2_SCL"],
        "net_pca9685_scl"
    );
    assert_eq!(
        imported["board"]["components"]["U2"]["pins"]["SCL"],
        "net_pca9685_scl"
    );
    assert_eq!(
        imported["board"]["components"]["U2"]["pins"]["PWM0"],
        "net_servo0_pwm"
    );
    assert_eq!(
        imported["board"]["components"]["SV0"]["pins"]["PWM"],
        "net_servo0_pwm"
    );
    assert_eq!(
        imported["board"]["components"]["JSV0"]["pins"]["SIG"],
        "net_servo0_pwm"
    );
    assert_eq!(
        imported["board"]["components"]["SV3"]["pins"]["VCC"],
        "net_vservo_sw"
    );
    assert_eq!(
        imported["board"]["components"]["JSV3"]["pins"]["SIG"],
        "net_servo3_pwm"
    );
    assert_eq!(imported["board"]["nets"]["net_3v3_logic"]["kind"], "power");
    assert_eq!(
        imported["board"]["nets"]["net_vservo_sw"]["supply_current_limit_A"],
        4.0
    );
    assert_eq!(
        imported["board"]["components"]["U1"]["source"]["board_pin_electrical_types"]["I2C2_SDA"],
        "bidirectional"
    );
    assert_eq!(
        imported["board"]["components"]["U2"]["source"]["board_pin_electrical_types"]["PWM3"],
        "output"
    );
    assert_eq!(
        imported["board"]["components"]["SV0"]["source"]["board_pin_electrical_types"]["PWM"],
        "input"
    );
}

#[test]
fn smart_robot_wheel_actuator_kicad_pcb_imports_layout_evidence() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let schematic_project = dir.path().join("wheel_actuator_imported.project.yaml");
    let enriched_project = dir.path().join("wheel_actuator_with_pcb.project.yaml");

    let schematic_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "demos/smart_robot/kicad/wheel_actuator/root.kicad_sch",
            "--mapping",
            "demos/smart_robot/kicad/wheel_actuator/circuitci.kicad-map.yaml",
            "--output",
            schematic_project.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(schematic_status.success());

    let pcb_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-pcb",
            "demos/smart_robot/kicad/wheel_actuator/wheel_actuator.kicad_pcb",
            "--project",
            schematic_project.to_str().unwrap(),
            "--output",
            enriched_project.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(pcb_status.success());

    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&enriched_project, &validator);

    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&enriched_project).unwrap()).unwrap();
    assert_eq!(
        imported["board"]["layout"]["placements"]["JACT1"]["x_mm"],
        10.0
    );
    assert_eq!(
        imported["board"]["layout"]["placements"]["PWR_STAGE"]["x_mm"],
        61.0
    );
    assert_eq!(
        imported["board"]["layout"]["placements"]["M1"]["side"],
        "top"
    );
    assert_eq!(
        imported["board"]["layout"]["pads"]["JACT1"]["CANH"]["net"],
        "net_robot_canh"
    );
    assert_eq!(
        imported["board"]["layout"]["pads"]["JACT1"]["CANH"]["kind"],
        "thru_hole"
    );
    assert_eq!(
        imported["board"]["layout"]["pads"]["PWR_STAGE"]["VM"]["net"],
        "net_vwheel_sw"
    );
    assert_eq!(
        imported["board"]["layout"]["pads"]["RSHUNT_U"]["B"]["net"],
        "net_cur_u"
    );
    assert_eq!(
        imported["board"]["layout"]["pads"]["M1"]["PHASE_U"]["net"],
        "net_phase_u"
    );
    assert_eq!(
        imported["board"]["layout"]["routes"]["net_robot_canh"]["segments"][0]["width_mm"],
        0.20
    );
    assert_eq!(
        imported["board"]["layout"]["routes"]["net_robot_canh"]["segments"][2]["end"]["x_mm"],
        22.0
    );
    assert_eq!(
        imported["board"]["layout"]["routes"]["net_vwheel_sw"]["segments"][0]["width_mm"],
        1.50
    );
    assert_eq!(
        imported["board"]["layout"]["routes"]["net_vwheel_sw"]["vias"][0]["drill_mm"],
        0.35
    );
    assert_eq!(
        imported["board"]["layout"]["routes"]["net_phase_u"]["segments"][0]["width_mm"],
        1.20
    );
    assert_eq!(
        imported["board"]["layout"]["routes"]["net_cur_u"]["segments"][0]["start"]["x_mm"],
        66.0
    );
    assert_eq!(
        imported["board"]["layout"]["routes"]["net_cur_u"]["segments"][0]["width_mm"],
        0.15
    );
    assert_eq!(
        imported["board"]["layout"]["constraints"]["net_rules"]["net_robot_canh"]["diff_pair_gap_mm"],
        0.20
    );
    assert_eq!(
        imported["board"]["layout"]["constraints"]["net_rules"]["net_robot_canh"]["length_max_mm"],
        80.0
    );
    assert_eq!(
        imported["board"]["layout"]["constraints"]["net_rules"]["net_vwheel_sw"]["track_width_mm"],
        1.50
    );
    assert_eq!(
        imported["board"]["layout"]["zones"]["gnd"][0]["filled_polygons"][0][2]["x_mm"],
        92.0
    );
    assert_eq!(
        imported["board"]["layout"]["outline"]["segments"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
}

#[test]
fn smart_robot_wheel_actuator_kicad_pcb_validates_layout_scenarios() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let schematic_project = dir.path().join("wheel_actuator_imported.project.yaml");
    let enriched_project = dir.path().join("wheel_actuator_with_pcb.project.yaml");
    let layout_validation_project = dir
        .path()
        .join("wheel_actuator_with_pcb_bus_layout_scenarios.project.yaml");
    let report_dir = dir.path().join("report");

    let schematic_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "demos/smart_robot/kicad/wheel_actuator/root.kicad_sch",
            "--mapping",
            "demos/smart_robot/kicad/wheel_actuator/circuitci.kicad-map.yaml",
            "--output",
            schematic_project.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(schematic_status.success());

    let pcb_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-pcb",
            "demos/smart_robot/kicad/wheel_actuator/wheel_actuator.kicad_pcb",
            "--project",
            schematic_project.to_str().unwrap(),
            "--output",
            enriched_project.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(pcb_status.success());

    let mut imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&enriched_project).unwrap()).unwrap();
    let source: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string("demos/smart_robot/circuitci/wheel_actuator/project.yaml")
            .unwrap(),
    )
    .unwrap();
    let mut bus_layout_scenarios: Vec<Value> = source["scenarios"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|scenario| {
            matches!(
                scenario["name"].as_str(),
                Some("wheel_actuator_can_endpoint_termination")
                    | Some("wheel_actuator_can_esd_route_placement")
                    | Some("wheel_actuator_can_termination_route_placement")
                    | Some("wheel_actuator_phase_route_current")
                    | Some("wheel_actuator_vbat_regen_route_current")
                    | Some("wheel_actuator_current_sense_placement")
            )
        })
        .cloned()
        .collect();
    assert_eq!(bus_layout_scenarios.len(), 6);
    for scenario in &mut bus_layout_scenarios {
        let parameters = scenario["parameters"].as_object_mut().unwrap();
        if parameters.get("line_a_net").is_some() {
            parameters.insert(
                "line_a_net".to_string(),
                Value::String("net_robot_canh".into()),
            );
        }
        if parameters.get("line_b_net").is_some() {
            parameters.insert(
                "line_b_net".to_string(),
                Value::String("net_robot_canl".into()),
            );
        }
        if let Some(route_nets) = parameters
            .get_mut("route_nets")
            .and_then(Value::as_array_mut)
        {
            for route_net in route_nets {
                if let Some(net) = route_net.as_str() {
                    *route_net = Value::String(format!("net_{net}"));
                }
            }
        }
        for list_name in ["phase_route_nets", "sense_route_nets"] {
            if let Some(route_nets) = parameters.get_mut(list_name).and_then(Value::as_array_mut) {
                for route_net in route_nets {
                    if let Some(net) = route_net.as_str() {
                        *route_net = Value::String(format!("net_{net}"));
                    }
                }
            }
        }
    }
    imported["scenarios"] = Value::Array(bus_layout_scenarios);
    std::fs::write(
        &layout_validation_project,
        serde_yaml_ng::to_string(&imported).unwrap(),
    )
    .unwrap();

    let validate_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            layout_validation_project.to_str().unwrap(),
            "--output",
            report_dir.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(validate_status.success());

    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(report_dir.join("report.json")).unwrap())
            .unwrap();
    assert_eq!(report["result"], "pass");
    assert_report_schema_valid(&report);
    assert!(
        report["failures"].as_array().unwrap().is_empty(),
        "CAD-derived wheel layout scenarios should validate cleanly: {report:#?}"
    );
}
