mod common;

use common::{
    assert_no_generated_solver_artifacts, assert_report_schema_valid, assert_yaml_file_valid,
    binary_available, embedded_backend_project, run_validation, run_validation_with_env,
    run_validation_with_path, run_validation_with_path_and_env,
};
use serde_json::Value;
use std::process::Command;

#[test]
fn um_stm32l4_physical_spice_requires_backend() {
    let report = run_validation("examples/um_stm32l4_usb_downloader_physical_spice/project.yaml");
    assert_eq!(report["result"], "fail");
    if binary_available("ngspice") {
        let failures = report["failures"].as_array().unwrap();
        assert!(
            failures
                .iter()
                .any(|failure| failure["id"] == "SPICE_TRANSIENT_ANALYSIS")
        );
        assert!(failures.iter().any(|failure| {
            failure["measured"]
                .as_object()
                .is_some_and(|measured| measured.contains_key("nrst"))
        }));
        assert!(failures.iter().any(|failure| {
            failure["measured"]
                .as_object()
                .is_some_and(|measured| measured.contains_key("q2_collector"))
        }));
        assert!(failures.iter().any(|failure| {
            failure["measured"]
                .as_object()
                .is_some_and(|measured| measured.contains_key("q2_base"))
        }));
        assert!(failures.iter().any(|failure| {
            failure["measured"]
                .as_object()
                .is_some_and(|measured| measured.contains_key("q2_base_drive_current"))
        }));
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
        assert!(
            report["failures"][0]["message"]
                .as_str()
                .unwrap()
                .contains("Physical analog simulation requires ngspice, Xyce")
        );
    }
    let artifacts = report["artifacts"].as_array().unwrap();
    assert!(artifacts.iter().any(|artifact| {
        artifact
            .as_str()
            .unwrap()
            .ends_with("models/spice/onsemi/1n4148ws.lib")
    }));
    assert!(artifacts.iter().any(|artifact| {
        artifact
            .as_str()
            .unwrap()
            .ends_with("models/spice/onsemi/ss8050_ss8550.lib")
    }));
    assert_report_schema_valid(&report);
}

#[test]
fn generated_mosfet_low_side_switch_passes_when_ngspice_available() {
    let report = run_validation("examples/good_mosfet_low_side_switch/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/onsemi/nds7002a.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn import_spice_generates_schema_valid_file_backed_project() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let output = dir.path().join("imported.project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            "examples/import_spice_rc/deck.cir",
            "--output",
            output.to_str().unwrap(),
            "--name",
            "import_spice_rc",
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
    assert_eq!(imported["project"]["name"], "import_spice_rc");
    assert_eq!(imported["scenarios"][0]["analog"]["netlist_source"], "file");
    assert_eq!(
        imported["scenarios"][0]["analog"]["assertions"],
        Value::Array(vec![])
    );
    assert!(imported["board"]["components"].get("R1").is_some());
    assert_eq!(
        imported["board"]["components"]["L1"]["spice"]["primitive"],
        "inductor"
    );
    assert!(
        (imported["board"]["components"]["L1"]["spice"]["value_h"]
            .as_f64()
            .unwrap()
            - 0.0000022)
            .abs()
            < 1.0e-18
    );
    assert!(imported["board"]["components"].get("D1").is_some());
    let model_file = &imported["scenarios"][0]["analog"]["model_files"][0];
    assert!(
        model_file["path"]
            .as_str()
            .unwrap()
            .ends_with("examples/import_spice_rc/models/imported_switch.model")
    );
    assert_eq!(model_file["sha256"].as_str().unwrap().len(), 64);

    let report = run_validation(output.to_str().unwrap());
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["infos"][0]["id"], "ANALOG_ASSERTIONS_ABSENT");
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn import_spice_rejects_malformed_element_line() {
    let dir = tempfile::tempdir().unwrap();
    let deck = dir.path().join("bad.cir");
    let output = dir.path().join("bad.project.yaml");
    std::fs::write(&deck, "R1 only_one_node\n.end\n").unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-spice",
            deck.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(!status.success());
    assert!(!output.exists());
}

#[test]
fn generated_mosfet_low_side_switch_runs_with_embedded_ngspice_when_available() {
    let (_project_dir, project_path) =
        embedded_backend_project("examples/good_mosfet_low_side_switch/project.yaml");
    let report = run_validation(project_path.to_str().unwrap());
    if report["result"] == "pass" {
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("waveform.csv") })
        );
    } else {
        assert_eq!(
            report["failures"][0]["id"],
            "ANALOG_EMBEDDED_SOLVER_UNAVAILABLE"
        );
    }
    assert_report_schema_valid(&report);
}

#[test]
fn explicit_embedded_ngspice_does_not_fallback_when_configured_library_is_missing() {
    let (_project_dir, project_path) =
        embedded_backend_project("examples/good_mosfet_low_side_switch/project.yaml");
    let missing_library = tempfile::tempdir()
        .unwrap()
        .path()
        .join("missing-libngspice.dylib");
    let report = run_validation_with_env(
        project_path.to_str().unwrap(),
        &[("CIRCUITCI_LIBNGSPICE", missing_library.to_str().unwrap())],
    );
    assert_eq!(report["result"], "fail");
    assert_eq!(
        report["failures"][0]["id"],
        "ANALOG_EMBEDDED_SOLVER_UNAVAILABLE"
    );
    assert_report_schema_valid(&report);
}

#[test]
fn generated_mosfet_overcurrent_fails_operating_limits() {
    let report = run_validation("examples/bad_mosfet_overcurrent/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "fail");
        let failures = report["failures"].as_array().unwrap();
        let operating_failures: Vec<&Value> = failures
            .iter()
            .filter(|failure| failure["id"] == "SPICE_OPERATING_LIMIT")
            .collect();
        assert_eq!(operating_failures.len(), 2);
        assert!(operating_failures.iter().any(|failure| {
            failure["measured"]["rating"] == "ID_continuous"
                && failure["measured"]["unit"] == "A"
                && failure["measured"]["component"] == "M1"
                && failure["measured"]["time_of_max_us"].as_f64().is_some()
                && failure["limit"]["rating_value"] == 0.28
        }));
        assert!(operating_failures.iter().any(|failure| {
            failure["measured"]["rating"] == "PD"
                && failure["measured"]["unit"] == "W"
                && failure["measured"]["component"] == "M1"
                && failure["measured"]["time_of_max_us"].as_f64().is_some()
                && failure["limit"]["rating_value"] == 0.3
        }));
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_mosfet_high_ambient_derates_power_limit() {
    let report = run_validation("examples/bad_mosfet_high_ambient_derating/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "fail");
        let failures = report["failures"].as_array().unwrap();
        assert!(failures.iter().any(|failure| {
            failure["id"] == "SPICE_OPERATING_LIMIT"
                && failure["measured"]["rating"] == "PD"
                && failure["measured"]["component"] == "M1"
                && failure["measured"]["scenario_temperature_c"] == 100.0
                && failure["limit"]["rating_value"] == 0.3
                && failure["limit"]["effective_limit"] == 0.12
                && failure["limit"]["derating_per_c"] == 0.0024
        }));
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_bjt_overcurrent_fails_operating_limits() {
    let report = run_validation("examples/bad_bjt_overcurrent/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "fail");
        let failures = report["failures"].as_array().unwrap();
        let operating_failures: Vec<&Value> = failures
            .iter()
            .filter(|failure| failure["id"] == "SPICE_OPERATING_LIMIT")
            .collect();
        assert!(
            operating_failures.iter().any(|failure| {
                failure["measured"]["rating"] == "IC"
                    && failure["measured"]["unit"] == "A"
                    && failure["measured"]["component"] == "Q1"
                    && failure["measured"]["time_of_max_us"].as_f64().is_some()
                    && failure["limit"]["rating_value"] == 1.5
            }),
            "expected an SS8050 collector-current operating-limit failure"
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_diode_temperature_derating_requires_metadata() {
    let path_without_ngspice = tempfile::tempdir().unwrap();
    let report = run_validation_with_path(
        "examples/bad_diode_missing_derating/project.yaml",
        path_without_ngspice.path(),
    );
    assert_eq!(report["result"], "fail");
    assert_eq!(report["summary"]["critical"], 1);
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "SPICE_OPERATING_LIMIT");
    assert_eq!(failure["measured"]["component"], "D1");
    assert_eq!(failure["measured"]["rating"], "PD or Ptot");
    assert_eq!(failure["limit"]["temperature_derating_required"], true);
    assert_report_schema_valid(&report);
}

#[test]
fn generated_mosfet_pulse_rating_requires_width_and_duty() {
    let path_without_ngspice = tempfile::tempdir().unwrap();
    let report = run_validation_with_path(
        "examples/bad_mosfet_unqualified_pulse_rating/project.yaml",
        path_without_ngspice.path(),
    );
    assert_eq!(report["result"], "fail");
    assert_eq!(report["summary"]["critical"], 1);
    let failure = &report["failures"][0];
    assert_eq!(failure["id"], "SPICE_OPERATING_LIMIT");
    assert_eq!(failure["measured"]["component"], "M1");
    assert_eq!(failure["measured"]["missing_pulse_rating"][0], "ID_pulsed");
    assert_eq!(failure["limit"]["pulse_width_and_duty_required"], true);
    assert_report_schema_valid(&report);
}

#[test]
fn generated_mosfet_qualified_pulse_current_passes_when_ngspice_available() {
    let report = run_validation("examples/good_mosfet_qualified_pulse_current/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/onsemi/fdmc86184.lib")
        }));
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_mosfet_pulse_duty_fails_with_pulse_evidence() {
    let report = run_validation("examples/bad_mosfet_pulse_duty/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "fail");
        let failure = report["failures"]
            .as_array()
            .unwrap()
            .iter()
            .find(|failure| {
                failure["id"] == "SPICE_OPERATING_LIMIT"
                    && failure["measured"]["rating"] == "ID_continuous"
            })
            .expect("expected continuous current pulse-duty finding");
        assert_eq!(failure["id"], "SPICE_OPERATING_LIMIT");
        assert_eq!(failure["measured"]["component"], "M1");
        assert_eq!(failure["measured"]["rating"], "ID_continuous");
        assert_eq!(failure["limit"]["pulse_rating"], "ID_pulsed");
        assert_eq!(failure["limit"]["pulse_rating_value"], 266.0);
        assert_eq!(failure["limit"]["pulse_width_us"], 300.0);
        assert_eq!(failure["limit"]["pulse_duty_cycle_max"], 0.02);
        assert!(failure["measured"]["pulse_duration_us"].as_f64().unwrap() > 300.0);
        assert!(failure["measured"]["pulse_duty_cycle"].as_f64().unwrap() > 0.02);
        assert!(failure["measured"]["max_abs"].as_f64().unwrap() < 266.0);
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_mosfet_soa_violation_reports_digitized_curve_evidence() {
    let report = run_validation("examples/bad_mosfet_soa_violation/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "fail");
        let failures = report["failures"].as_array().unwrap();
        assert!(failures.iter().any(|failure| {
            failure["id"] == "SPICE_OPERATING_LIMIT"
                && failure["measured"]["rating"] == "SOA"
                && failure["measured"]["component"] == "M1"
                && failure["measured"]["soa_margin_ratio"].as_f64().unwrap() > 1.0
                && failure["measured"]["vds_v"].as_f64().unwrap() > 40.0
                && failure["measured"]["id_a"].as_f64().unwrap() > 12.0
                && failure["limit"]["soa_curve"] == "forward_bias_100us"
                && failure["limit"]["curve_pulse_width_us"] == 100.0
                && failure["limit"]["interpolation"] == "log_log"
                && failure["limit"]["digitization_confidence"] == "low"
                && failure["limit"]["digitization_warning"]
                    .as_str()
                    .unwrap()
                    .contains("screening evidence")
        }));
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_pmos_overcurrent_preserves_signed_datasheet_rating() {
    let report = run_validation("examples/bad_pmos_overcurrent/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "fail");
        let failures = report["failures"].as_array().unwrap();
        assert!(failures.iter().any(|failure| {
            failure["id"] == "SPICE_OPERATING_LIMIT"
                && failure["measured"]["rating"] == "ID_continuous"
                && failure["measured"]["unit"] == "A"
                && failure["measured"]["component"] == "M1"
                && failure["measured"]["time_of_max_us"].as_f64().is_some()
                && failure["limit"]["rating_value"] == -0.13
                && failure["limit"]["max_abs"] == 0.13
        }));
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_pmos_high_side_switch_passes_when_ngspice_available() {
    let report = run_validation("examples/good_pmos_high_side_switch/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/onsemi/bss84.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_diode_switching_passes_when_ngspice_available() {
    let report = run_validation("examples/good_diode_switching/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/onsemi/1n4148ws.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_diode_overcurrent_fails_operating_limits() {
    let report = run_validation("examples/bad_diode_overcurrent/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "fail");
        let failures = report["failures"].as_array().unwrap();
        assert!(failures.iter().any(|failure| {
            failure["id"] == "SPICE_OPERATING_LIMIT"
                && failure["measured"]["rating"] == "IF_AV"
                && failure["measured"]["unit"] == "A"
                && failure["measured"]["component"] == "D1"
                && failure["measured"]["time_of_max_us"].as_f64().is_some()
                && failure["limit"]["rating_value"] == 0.15
        }));
        assert!(failures.iter().any(|failure| {
            failure["id"] == "SPICE_OPERATING_LIMIT"
                && failure["measured"]["rating"] == "PD"
                && failure["measured"]["unit"] == "W"
                && failure["measured"]["component"] == "D1"
                && failure["measured"]["time_of_max_us"].as_f64().is_some()
                && failure["limit"]["rating_value"] == 0.2
        }));
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_diode_reverse_voltage_fails_operating_limits() {
    let report = run_validation("examples/bad_diode_reverse_voltage/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "fail");
        let failures = report["failures"].as_array().unwrap();
        assert!(failures.iter().any(|failure| {
            failure["id"] == "SPICE_OPERATING_LIMIT"
                && failure["measured"]["rating"] == "VRRM"
                && failure["measured"]["unit"] == "V"
                && failure["measured"]["component"] == "D1"
                && failure["measured"]["time_of_max_us"].as_f64().is_some()
                && failure["limit"]["rating_value"].as_f64() == Some(100.0)
        }));
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_diode_offset_reverse_voltage_uses_terminal_difference() {
    let report = run_validation("examples/bad_diode_reverse_voltage_offset/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "fail");
        let failures = report["failures"].as_array().unwrap();
        assert!(failures.iter().any(|failure| {
            failure["id"] == "SPICE_OPERATING_LIMIT"
                && failure["measured"]["rating"] == "VRRM"
                && failure["measured"]["expression"] == "max(0,V(cathode,anode))"
                && failure["measured"]["component"] == "D1"
                && failure["limit"]["rating_value"].as_f64() == Some(100.0)
        }));
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_subckt_rc_delay_passes_when_ngspice_available() {
    let report = run_validation("examples/good_subckt_rc_delay/project.yaml");
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
        assert!(report["failures"].as_array().unwrap().is_empty());
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/rc_delay_subckt.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_mosfet_without_body_policy_fails_closed() {
    let report = run_validation("examples/bad_mosfet_missing_body_policy/project.yaml");
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_TRANSIENT_ANALYSIS");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("body_pin_policy=tie_to_source_when_absent")
    );
    assert!(report["waveforms"].as_array().unwrap().is_empty());
    assert_no_generated_solver_artifacts(&report);
    assert_report_schema_valid(&report);
}

#[test]
fn generated_mosfet_model_file_requires_sha_pin() {
    let report = run_validation("examples/bad_mosfet_model_missing_sha/project.yaml");
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "SPICE_TRANSIENT_ANALYSIS");
    assert!(
        report["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("has no SHA-256 pin")
    );
    assert!(report["waveforms"].as_array().unwrap().is_empty());
    assert_no_generated_solver_artifacts(&report);
    assert_report_schema_valid(&report);
}

#[test]
fn generated_mosfet_operating_limits_require_datasheet_ratings() {
    let path_without_ngspice = tempfile::tempdir().unwrap();
    let report = run_validation_with_path(
        "examples/bad_mosfet_missing_operating_ratings/project.yaml",
        path_without_ngspice.path(),
    );
    assert_eq!(report["result"], "fail");
    let failures = report["failures"].as_array().unwrap();
    assert_eq!(
        failures
            .iter()
            .filter(|failure| failure["id"] == "SPICE_OPERATING_LIMIT")
            .count(),
        4
    );
    assert!(failures.iter().any(|failure| {
        failure["measured"]["component"] == "M1"
            && failure["measured"]["model"] == "fixture.no_rating_mosfet"
            && failure["measured"]["quantity"] == "current"
    }));
    assert!(report["waveforms"].as_array().unwrap().is_empty());
    let artifacts = report["artifacts"].as_array().unwrap();
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.as_str().unwrap().ends_with("generated_board.cir"))
    );
    for suffix in ["circuitci_ngspice.cir", "ngspice.log", "waveform.csv"] {
        assert!(
            !artifacts
                .iter()
                .any(|artifact| artifact.as_str().unwrap().ends_with(suffix)),
            "unexpected solver artifact {suffix} in {artifacts:#?}"
        );
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_diode_operating_limits_require_datasheet_ratings() {
    let path_without_ngspice = tempfile::tempdir().unwrap();
    let report = run_validation_with_path(
        "examples/bad_diode_missing_operating_ratings/project.yaml",
        path_without_ngspice.path(),
    );
    assert_eq!(report["result"], "fail");
    let failures = report["failures"].as_array().unwrap();
    assert_eq!(
        failures
            .iter()
            .filter(|failure| failure["id"] == "SPICE_OPERATING_LIMIT")
            .count(),
        3
    );
    assert!(failures.iter().any(|failure| {
        failure["measured"]["component"] == "D1"
            && failure["measured"]["model"] == "fixture.no_rating_diode"
            && failure["measured"]["quantity"] == "current"
    }));
    assert!(report["waveforms"].as_array().unwrap().is_empty());
    let artifacts = report["artifacts"].as_array().unwrap();
    assert!(
        artifacts
            .iter()
            .any(|artifact| artifact.as_str().unwrap().ends_with("generated_board.cir"))
    );
    for suffix in ["circuitci_ngspice.cir", "ngspice.log", "waveform.csv"] {
        assert!(
            !artifacts
                .iter()
                .any(|artifact| artifact.as_str().unwrap().ends_with(suffix)),
            "unexpected solver artifact {suffix} in {artifacts:#?}"
        );
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_contract_errors_do_not_require_solver_on_path() {
    let path_without_ngspice = tempfile::tempdir().unwrap();
    let missing_body = run_validation_with_path(
        "examples/bad_mosfet_missing_body_policy/project.yaml",
        path_without_ngspice.path(),
    );
    assert_eq!(missing_body["result"], "fail");
    assert_eq!(
        missing_body["failures"][0]["id"],
        "SPICE_TRANSIENT_ANALYSIS"
    );
    assert!(
        missing_body["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("body_pin_policy=tie_to_source_when_absent")
    );
    assert!(missing_body["waveforms"].as_array().unwrap().is_empty());
    assert_no_generated_solver_artifacts(&missing_body);
    assert_report_schema_valid(&missing_body);

    let missing_sha = run_validation_with_path(
        "examples/bad_mosfet_model_missing_sha/project.yaml",
        path_without_ngspice.path(),
    );
    assert_eq!(missing_sha["result"], "fail");
    assert_eq!(missing_sha["failures"][0]["id"], "SPICE_TRANSIENT_ANALYSIS");
    assert!(
        missing_sha["failures"][0]["message"]
            .as_str()
            .unwrap()
            .contains("has no SHA-256 pin")
    );
    assert!(missing_sha["waveforms"].as_array().unwrap().is_empty());
    assert_no_generated_solver_artifacts(&missing_sha);
    assert_report_schema_valid(&missing_sha);
}

#[test]
fn generated_valid_netlist_is_artifact_even_without_solver() {
    let path_without_ngspice = tempfile::tempdir().unwrap();
    let missing_library = path_without_ngspice.path().join("missing-libngspice.dylib");
    let report = run_validation_with_path_and_env(
        "examples/good_mosfet_low_side_switch/project.yaml",
        path_without_ngspice.path(),
        &[("CIRCUITCI_LIBNGSPICE", missing_library.to_str().unwrap())],
    );
    assert_eq!(report["result"], "fail");
    assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    let artifacts = report["artifacts"].as_array().unwrap();
    assert_eq!(
        artifacts
            .iter()
            .filter(|artifact| artifact.as_str().unwrap().ends_with("generated_board.cir"))
            .count(),
        1
    );
    assert_report_schema_valid(&report);
}

#[test]
fn generated_subckt_wrong_pin_order_fails_waveform_assertion() {
    let report = run_validation("examples/bad_subckt_wrong_pin_order/project.yaml");
    assert_eq!(report["result"], "fail");
    if binary_available("ngspice") {
        assert_eq!(report["failures"][0]["id"], "SPICE_TRANSIENT_ANALYSIS");
        assert!(
            report["failures"][0]["message"]
                .as_str()
                .unwrap()
                .contains("output_should_still_be_charging_after_four_us failed")
        );
        assert_eq!(report["failures"][0]["measured"]["output_unit"], "V");
        assert!(!report["waveforms"].as_array().unwrap().is_empty());
        let artifacts = report["artifacts"].as_array().unwrap();
        assert!(artifacts.iter().any(|artifact| {
            artifact
                .as_str()
                .unwrap()
                .ends_with("models/spice/generic/rc_delay_subckt.lib")
        }));
        assert!(
            artifacts
                .iter()
                .any(|artifact| { artifact.as_str().unwrap().ends_with("generated_board.cir") })
        );
    } else {
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}

#[test]
fn generated_model_paths_do_not_depend_on_process_cwd() {
    let out_dir = tempfile::tempdir().unwrap();
    let cwd = tempfile::tempdir().unwrap();
    let project_path = std::env::current_dir()
        .unwrap()
        .join("examples/good_mosfet_low_side_switch/project.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .current_dir(cwd.path())
        .args([
            "validate",
            project_path.to_str().unwrap(),
            "--profile",
            "iot_basic_v0",
            "--output",
            out_dir.path().to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(out_dir.path().join("report.json")).unwrap())
            .unwrap();
    if binary_available("ngspice") {
        assert_eq!(report["result"], "pass");
    } else {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["failures"][0]["id"], "ANALOG_BACKEND_UNAVAILABLE");
    }
    assert_report_schema_valid(&report);
}
