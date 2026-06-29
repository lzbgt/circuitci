mod common;

use common::manufacturing_metadata::assert_runnable;
use common::read_suggestion_report;
use serde_yaml_ng::Value;
use std::process::Command;

#[test]
fn import_manufacturing_metadata_applies_solver_convergence_rows() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("without_solver_convergence.project.yaml");
    let output = dir.path().join("with_solver_convergence.project.yaml");
    let manifest_output = output.with_extension("manufacturing.json");
    let metadata = dir.path().join("solver_convergence.csv");
    let mut project_yaml: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string("examples/scenario_suggestions_controlled_impedance/project.yaml")
            .unwrap(),
    )
    .unwrap();
    let run_logs =
        project_yaml["board"]["manufacturing"]["controlled_impedance"]["solver_run_logs"]
            .as_sequence_mut()
            .unwrap();
    run_logs.clear();
    std::fs::write(&input, serde_yaml_ng::to_string(&project_yaml).unwrap()).unwrap();
    let rows = [
        [
            "field",
            "value",
            "source",
            "name",
            "solver",
            "solver_version",
            "run_id",
            "artifact_uri",
            "artifact_sha256",
            "random_seed",
            "numeric_tolerance_policy",
            "max_residual_error",
            "max_iterations",
            "precision_policy_source",
            "precision_policy_artifact_uri",
            "precision_policy_artifact_sha256",
            "floating_point_precision",
            "min_significant_digits",
            "max_roundoff_error_ohm",
            "min_convergence_sample_count",
            "max_convergence_impedance_delta_ohm",
            "required_stopping_criteria",
            "require_monotonic_residual_decrease",
            "solver_run_log",
            "iteration",
            "solved_impedance_ohm",
            "residual_error",
            "stopping_criteria",
        ],
        [
            "controlled_impedance_solver_run_log",
            "",
            "si_solver_run_review_rev_a",
            "reviewed_2d_field_solver_run_log_rf",
            "reviewed_2d_field_solver",
            "2026.07",
            "rf_solver_run_2026_07_a",
            "artifacts/solver/rf_solver_run_2026_07_a.log",
            "aaaabbbbccccddddeeeeffff0000111122223333444455556666777788889999",
            "seed_2026_07_rf",
            "si_solver_tolerance_rev_a",
            "0.000001",
            "120",
            "si_solver_precision_review_rev_a",
            "artifacts/solver/precision_policy_rev_a.json",
            "22223333444455556666777788889999aaaabbbbccccddddeeeeffff00001111",
            "ieee754_binary64",
            "12",
            "0.005",
            "2",
            "0.04",
            "residual_and_delta",
            "true",
            "",
            "",
            "",
            "",
            "",
        ],
        [
            "controlled_impedance_solver_convergence_sample",
            "",
            "si_solver_convergence_review_rev_a",
            "rf_solver_converged_92",
            "",
            "",
            "",
            "artifacts/solver/rf_solver_converged_92.json",
            "77776666555544443333222211110000ffffeeeeddddccccbbbbaaaa99998888",
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
            "",
            "",
            "reviewed_2d_field_solver_run_log_rf",
            "92",
            "50.81",
            "0.0000004",
            "residual_and_delta",
        ],
        [
            "controlled_impedance_solver_convergence_sample",
            "",
            "si_solver_convergence_review_rev_a",
            "rf_solver_converged_96",
            "",
            "",
            "",
            "artifacts/solver/rf_solver_converged_96.json",
            "6666555544443333222211110000ffffeeeeddddccccbbbbaaaa999988887777",
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
            "",
            "",
            "reviewed_2d_field_solver_run_log_rf",
            "96",
            "50.80",
            "0.0000003",
            "residual_and_delta",
        ],
    ];
    std::fs::write(
        &metadata,
        rows.iter()
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

    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    common::assert_yaml_file_valid(&output, &validator);
    let enriched: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let run_log = &enriched["board"]["manufacturing"]["controlled_impedance"]["solver_run_logs"][0];
    assert_eq!(run_log["name"], "reviewed_2d_field_solver_run_log_rf");
    assert_eq!(
        run_log["precision_policy_source"],
        "si_solver_precision_review_rev_a"
    );
    assert_eq!(run_log["floating_point_precision"], "ieee754_binary64");
    assert_eq!(run_log["min_significant_digits"], 12);
    assert_eq!(run_log["max_roundoff_error_ohm"], 0.005);
    assert_eq!(run_log["min_convergence_sample_count"], 2);
    assert_eq!(run_log["max_convergence_impedance_delta_ohm"], 0.04);
    assert_eq!(run_log["required_stopping_criteria"], "residual_and_delta");
    assert_eq!(run_log["require_monotonic_residual_decrease"], true);
    assert_eq!(
        run_log["convergence_samples"].as_sequence().unwrap().len(),
        2
    );
    assert_eq!(
        run_log["convergence_samples"][0]["name"],
        "rf_solver_converged_92"
    );
    assert_eq!(
        run_log["convergence_samples"][1]["stopping_criteria"],
        "residual_and_delta"
    );

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(manifest_output).unwrap()).unwrap();
    assert_eq!(manifest["schema_version"], "0.41.0");
    assert_eq!(
        manifest["rows"][2]["board_field"],
        "controlled_impedance.solver_run_logs[].convergence_samples[]"
    );
    assert_eq!(
        manifest["rows"][2]["raw_columns"]["solver_run_log"],
        "reviewed_2d_field_solver_run_log_rf"
    );
}

#[test]
fn import_manufacturing_metadata_applies_solver_result_rows() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("without_solver_result.project.yaml");
    let metadata = dir.path().join("solver_result.csv");
    let output = dir.path().join("with_solver_result.project.yaml");
    let library_output = dir
        .path()
        .join("with_solver_result_and_material_library.project.yaml");
    let runtime_output = dir
        .path()
        .join("with_solver_result_and_runtime_allowlist.project.yaml");
    let entitlement_output = dir
        .path()
        .join("with_solver_result_and_entitlement.project.yaml");
    let environment_output = dir
        .path()
        .join("with_solver_result_and_environment.project.yaml");
    let run_log_output = dir
        .path()
        .join("with_solver_result_and_run_log.project.yaml");
    let process_output = dir
        .path()
        .join("with_solver_result_and_material_process.project.yaml");
    let qualified_output = dir
        .path()
        .join("with_solver_result_and_qualification.project.yaml");
    let manifest_output = output.with_extension("manufacturing.json");
    let runtime_manifest_output = runtime_output.with_extension("manufacturing.json");
    let entitlement_manifest_output = entitlement_output.with_extension("manufacturing.json");
    let environment_manifest_output = environment_output.with_extension("manufacturing.json");
    let run_log_manifest_output = run_log_output.with_extension("manufacturing.json");
    let library_manifest_output = library_output.with_extension("manufacturing.json");
    let process_manifest_output = process_output.with_extension("manufacturing.json");
    let suggestions_output = dir.path().join("suggestions.yaml");
    let project_yaml: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string("examples/scenario_suggestions_controlled_impedance/project.yaml")
            .unwrap(),
    )
    .unwrap();
    std::fs::write(&input, serde_yaml_ng::to_string(&project_yaml).unwrap()).unwrap();
    std::fs::write(
        &metadata,
        "field,value,unit,source,notes,name,result_type,net,target_impedance_ohm,max_impedance_error_ohm,solver,solver_version,solver_artifact_uri,solver_artifact_sha256,solver_input_deck_uri,solver_input_deck_sha256,stackup_revision,route_layer,reference_layer,dielectric_layer,solved_width_mm,max_route_width_delta_mm,input_stackup_revision,input_route_layer,input_reference_layer,input_dielectric_layer,input_width_mm,frequency_mhz,input_frequency_mhz,min_solver_sample_count,max_solver_frequency_step_mhz,required_solver_corners,solver_result_name,corner,copper_roughness_model,copper_roughness_um,input_copper_roughness_model,input_copper_roughness_um,etch_compensation_model,etch_compensation_um,input_etch_compensation_model,input_etch_compensation_um,solver_artifact_signature_uri,solver_artifact_signature_sha256,solver_artifact_signer,solver_output_schema,solver_output_schema_version,solver_output_schema_uri,solver_output_schema_sha256,solver_config_lock_uri,solver_config_lock_sha256,solver_config_lock_tool,solver_config_lock_revision,solver_runtime_allowlist,solver_runtime_profile,solver_runtime_options,solver_material_library,solver_material_library_revision,solver_material_library_artifact_uri,solver_material_library_artifact_sha256,input_material_library,input_material_library_revision,stackup_signoff_source,fabricator_stackup_revision,stackup_signoff_artifact_uri,stackup_signoff_artifact_sha256,solver_entitlement,solver_entitlement_features,solver_execution_environment,solver_environment_fingerprint,solver_environment_components,solver_run_log,solver_run_id,solver_random_seed,solver_numeric_tolerance_policy,solver_residual_error,solver_iterations\n\
         controlled_impedance_solver_result,50.6,ohm,solver report,reviewed solver evidence,rf_solver_result,single_ended,RF,50.0,2.0,reviewed_2d_field_solver,2026.07,artifacts/solver/rf_solver_result.json,0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef,artifacts/solver/rf_solver_input_deck.json,fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210,stackup_rev_b,F.Cu,In1.GND,prepreg_1,0.20,0.03,stackup_rev_b,F.Cu,In1.GND,prepreg_1,0.20,2400,2400,4,500,nominal;high_dk,,,huray,1.5,huray,1.5,fabricator_finished_width_bias,8.0,fabricator_finished_width_bias,8.0,artifacts/solver/rf_solver_result.sig,1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef,si_review_key_2026,circuitci_controlled_impedance_solver_result,1.0,artifacts/solver/controlled_impedance_solver_result_schema_v1.json,55556666777788889999aaaabbbbccccddddeeeeffff00001111222233334444,artifacts/solver/reviewed_2d_field_solver_config_lock_rev_c.json,6666777788889999aaaabbbbccccddddeeeeffff000011112222333344445555,reviewed_2d_field_solver,config_lock_rev_c,reviewed_2d_field_solver_runtime_lock_c,production_si,quasi_static;finite_thickness_copper;huray_roughness;fabricator_etch_bias,reviewed_stackup_materials,rev_b,artifacts/solver/material_library_rev_b.json,abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd,reviewed_stackup_materials,rev_b,fabricator_stackup_review_rev_b,stackup_rev_b,artifacts/fabricator/stackup_signoff_rev_b.pdf,111122223333444455556666777788889999aaaabbbbccccddddeeeeffff0000,reviewed_2d_field_solver_si_entitlement,2d_field_solver;lossy_copper_roughness,reviewed_2d_field_solver_env_lock,env_fp_2026_07_a,solver_binary;material_library;config_lock,reviewed_2d_field_solver_run_log_rf,rf_solver_run_2026_07_a,seed_2026_07_rf,si_solver_tolerance_rev_a,0.0000004,84\n\
         controlled_impedance_solver_sample,50.6,ohm,solver report,nominal sample,rf_solver_nominal_2400,,,,,,,,,,,,,,,,,,,,,,2400,,,,,rf_solver_result,nominal,,,,,,,,,,,,,,,,,,,,,,,,,,,\n\
         controlled_impedance_solver_sample,50.7,ohm,solver report,nominal sample,rf_solver_nominal_2900,,,,,,,,,,,,,,,,,,,,,,2900,,,,,rf_solver_result,nominal,,,,,,,,,,,,,,,,,,,,,,,,,,,\n\
         controlled_impedance_solver_sample,49.5,ohm,solver report,high dk sample,rf_solver_high_dk_2400,,,,,,,,,,,,,,,,,,,,,,2400,,,,,rf_solver_result,high_dk,,,,,,,,,,,,,,,,,,,,,,,,,,,\n\
         controlled_impedance_solver_sample,49.6,ohm,solver report,high dk sample,rf_solver_high_dk_2900,,,,,,,,,,,,,,,,,,,,,,2900,,,,,rf_solver_result,high_dk,,,,,,,,,,,,,,,,,,,,,,,,,\n",
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

    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    common::assert_yaml_file_valid(&output, &validator);
    let enriched: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let result = &enriched["board"]["manufacturing"]["controlled_impedance"]["solver_results"][0];
    assert_eq!(result["name"], "rf_solver_result");
    assert_eq!(result["source"], "solver report");
    assert_eq!(result["solver_version"], "2026.07");
    assert_eq!(
        result["solver_artifact_uri"],
        "artifacts/solver/rf_solver_result.json"
    );
    assert_eq!(
        result["solver_artifact_sha256"],
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
    assert_eq!(
        result["solver_artifact_signature_uri"],
        "artifacts/solver/rf_solver_result.sig"
    );
    assert_eq!(
        result["solver_artifact_signature_sha256"],
        "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
    );
    assert_eq!(result["solver_artifact_signer"], "si_review_key_2026");
    assert_eq!(
        result["solver_output_schema"],
        "circuitci_controlled_impedance_solver_result"
    );
    assert_eq!(result["solver_output_schema_version"], "1.0");
    assert_eq!(
        result["solver_output_schema_uri"],
        "artifacts/solver/controlled_impedance_solver_result_schema_v1.json"
    );
    assert_eq!(
        result["solver_output_schema_sha256"],
        "55556666777788889999aaaabbbbccccddddeeeeffff00001111222233334444"
    );
    assert_eq!(
        result["solver_config_lock_uri"],
        "artifacts/solver/reviewed_2d_field_solver_config_lock_rev_c.json"
    );
    assert_eq!(
        result["solver_config_lock_sha256"],
        "6666777788889999aaaabbbbccccddddeeeeffff000011112222333344445555"
    );
    assert_eq!(
        result["solver_config_lock_tool"],
        "reviewed_2d_field_solver"
    );
    assert_eq!(result["solver_config_lock_revision"], "config_lock_rev_c");
    assert_eq!(
        result["solver_runtime_allowlist"],
        "reviewed_2d_field_solver_runtime_lock_c"
    );
    assert_eq!(result["solver_runtime_profile"], "production_si");
    assert_eq!(result["solver_runtime_options"][2], "huray_roughness");
    assert_eq!(
        result["solver_entitlement"],
        "reviewed_2d_field_solver_si_entitlement"
    );
    assert_eq!(
        result["solver_entitlement_features"][1],
        "lossy_copper_roughness"
    );
    assert_eq!(
        result["solver_execution_environment"],
        "reviewed_2d_field_solver_env_lock"
    );
    assert_eq!(result["solver_environment_fingerprint"], "env_fp_2026_07_a");
    assert_eq!(result["solver_environment_components"][2], "config_lock");
    assert_eq!(
        result["solver_run_log"],
        "reviewed_2d_field_solver_run_log_rf"
    );
    assert_eq!(result["solver_run_id"], "rf_solver_run_2026_07_a");
    assert_eq!(result["solver_random_seed"], "seed_2026_07_rf");
    assert_eq!(
        result["solver_numeric_tolerance_policy"],
        "si_solver_tolerance_rev_a"
    );
    assert_eq!(result["solver_residual_error"], 0.0000004);
    assert_eq!(result["solver_iterations"], 84);
    assert_eq!(
        result["solver_input_deck_uri"],
        "artifacts/solver/rf_solver_input_deck.json"
    );
    assert_eq!(
        result["solver_input_deck_sha256"],
        "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210"
    );
    assert_eq!(result["solved_impedance_ohm"], 50.6);
    assert_eq!(result["stackup_revision"], "stackup_rev_b");
    assert_eq!(result["input_width_mm"], 0.20);
    assert_eq!(result["copper_roughness_model"], "huray");
    assert_eq!(result["copper_roughness_um"], 1.5);
    assert_eq!(result["input_copper_roughness_model"], "huray");
    assert_eq!(result["input_copper_roughness_um"], 1.5);
    assert_eq!(
        result["etch_compensation_model"],
        "fabricator_finished_width_bias"
    );
    assert_eq!(result["etch_compensation_um"], 8.0);
    assert_eq!(
        result["input_etch_compensation_model"],
        "fabricator_finished_width_bias"
    );
    assert_eq!(result["input_etch_compensation_um"], 8.0);
    assert_eq!(
        result["solver_material_library"],
        "reviewed_stackup_materials"
    );
    assert_eq!(result["solver_material_library_revision"], "rev_b");
    assert_eq!(
        result["solver_material_library_artifact_uri"],
        "artifacts/solver/material_library_rev_b.json"
    );
    assert_eq!(
        result["solver_material_library_artifact_sha256"],
        "abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
    );
    assert_eq!(
        result["input_material_library"],
        "reviewed_stackup_materials"
    );
    assert_eq!(result["input_material_library_revision"], "rev_b");
    assert_eq!(
        result["stackup_signoff_source"],
        "fabricator_stackup_review_rev_b"
    );
    assert_eq!(result["fabricator_stackup_revision"], "stackup_rev_b");
    assert_eq!(
        result["stackup_signoff_artifact_uri"],
        "artifacts/fabricator/stackup_signoff_rev_b.pdf"
    );
    assert_eq!(
        result["stackup_signoff_artifact_sha256"],
        "111122223333444455556666777788889999aaaabbbbccccddddeeeeffff0000"
    );
    assert_eq!(result["min_solver_sample_count"], 4);
    assert_eq!(result["max_solver_frequency_step_mhz"], 500.0);
    assert_eq!(result["required_solver_corners"][0], "nominal");
    assert_eq!(result["samples"][0]["name"], "rf_solver_nominal_2400");
    assert_eq!(result["samples"][0]["corner"], "nominal");

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
    assert_eq!(manifest["schema_version"], "0.41.0");
    assert_eq!(
        manifest["rows"][0]["board_field"],
        "controlled_impedance.solver_results[]"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver"],
        "reviewed_2d_field_solver"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_artifact_sha256"],
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_artifact_signature_sha256"],
        "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_artifact_signer"],
        "si_review_key_2026"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_output_schema_version"],
        "1.0"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_output_schema_sha256"],
        "55556666777788889999aaaabbbbccccddddeeeeffff00001111222233334444"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_config_lock_tool"],
        "reviewed_2d_field_solver"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_config_lock_revision"],
        "config_lock_rev_c"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_runtime_allowlist"],
        "reviewed_2d_field_solver_runtime_lock_c"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_runtime_options"][3],
        "fabricator_etch_bias"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_entitlement"],
        "reviewed_2d_field_solver_si_entitlement"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_entitlement_features"][0],
        "2d_field_solver"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_execution_environment"],
        "reviewed_2d_field_solver_env_lock"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_environment_components"][1],
        "material_library"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_run_log"],
        "reviewed_2d_field_solver_run_log_rf"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_numeric_tolerance_policy"],
        "si_solver_tolerance_rev_a"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_iterations"],
        84
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_input_deck_uri"],
        "artifacts/solver/rf_solver_input_deck.json"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["solver_material_library_revision"],
        "rev_b"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["input_material_library"],
        "reviewed_stackup_materials"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["stackup_signoff_artifact_sha256"],
        "111122223333444455556666777788889999aaaabbbbccccddddeeeeffff0000"
    );
    assert_eq!(
        manifest["rows"][1]["board_field"],
        "controlled_impedance.solver_results[].samples[]"
    );
    assert_eq!(
        manifest["rows"][1]["normalized_value"]["solved_impedance_ohm"],
        50.6
    );

    std::fs::write(
        &metadata,
        "field,value,source,name,solver,solver_config_lock_revision,runtime_profile,allowlist_revision,artifact_uri,artifact_sha256,allowed_options\n\
         controlled_impedance_solver_runtime_allowlist,reviewed,solver runtime review,reviewed_2d_field_solver_runtime_lock_c,reviewed_2d_field_solver,config_lock_rev_c,production_si,runtime_allowlist_rev_a,artifacts/solver/runtime_allowlist_rev_a.json,777788889999aaaabbbbccccddddeeeeffff0000111122223333444455556666,quasi_static;finite_thickness_copper;huray_roughness;fabricator_etch_bias\n",
    )
    .unwrap();
    let runtime_import = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-manufacturing-metadata",
            "--project",
            output.to_str().unwrap(),
            "--metadata",
            metadata.to_str().unwrap(),
            "--output",
            runtime_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        runtime_import.status.success(),
        "{}",
        String::from_utf8_lossy(&runtime_import.stderr)
    );
    common::assert_yaml_file_valid(&runtime_output, &validator);
    let enriched_with_runtime: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&runtime_output).unwrap()).unwrap();
    let allowlist = &enriched_with_runtime["board"]["manufacturing"]["controlled_impedance"]["solver_runtime_allowlists"]
        [0];
    assert_eq!(allowlist["name"], "reviewed_2d_field_solver_runtime_lock_c");
    assert_eq!(allowlist["runtime_profile"], "production_si");
    assert_eq!(allowlist["allowed_options"][3], "fabricator_etch_bias");
    let runtime_manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(runtime_manifest_output).unwrap()).unwrap();
    if let Err(error) = manifest_validator.validate(&runtime_manifest) {
        panic!("Runtime allowlist metadata import manifest failed schema validation: {error}");
    }
    assert_eq!(
        runtime_manifest["rows"][0]["board_field"],
        "controlled_impedance.solver_runtime_allowlists[]"
    );
    assert_eq!(
        runtime_manifest["rows"][0]["normalized_value"]["artifact_sha256"],
        "777788889999aaaabbbbccccddddeeeeffff0000111122223333444455556666"
    );

    std::fs::write(
        &metadata,
        "field,value,source,name,solver,solver_version,entitlement_id,entitlement_revision,artifact_uri,artifact_sha256,licensed_features\n\
         controlled_impedance_solver_entitlement,reviewed,solver entitlement review,reviewed_2d_field_solver_si_entitlement,reviewed_2d_field_solver,2026.07,si_solver_floating_pool_a,entitlement_rev_2026_07,artifacts/solver/license_entitlement_rev_2026_07.json,88889999aaaabbbbccccddddeeeeffff00001111222233334444555566667777,2d_field_solver;lossy_copper_roughness\n",
    )
    .unwrap();
    let entitlement_import = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-manufacturing-metadata",
            "--project",
            runtime_output.to_str().unwrap(),
            "--metadata",
            metadata.to_str().unwrap(),
            "--output",
            entitlement_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        entitlement_import.status.success(),
        "{}",
        String::from_utf8_lossy(&entitlement_import.stderr)
    );
    common::assert_yaml_file_valid(&entitlement_output, &validator);
    let enriched_with_entitlement: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&entitlement_output).unwrap()).unwrap();
    let entitlement = &enriched_with_entitlement["board"]["manufacturing"]["controlled_impedance"]
        ["solver_entitlements"][0];
    assert_eq!(
        entitlement["name"],
        "reviewed_2d_field_solver_si_entitlement"
    );
    assert_eq!(entitlement["solver_version"], "2026.07");
    assert_eq!(
        entitlement["licensed_features"][1],
        "lossy_copper_roughness"
    );
    let entitlement_manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(entitlement_manifest_output).unwrap())
            .unwrap();
    if let Err(error) = manifest_validator.validate(&entitlement_manifest) {
        panic!("Solver entitlement metadata import manifest failed schema validation: {error}");
    }
    assert_eq!(
        entitlement_manifest["rows"][0]["board_field"],
        "controlled_impedance.solver_entitlements[]"
    );
    assert_eq!(
        entitlement_manifest["rows"][0]["normalized_value"]["licensed_features"][0],
        "2d_field_solver"
    );

    std::fs::write(
        &metadata,
        "field,value,source,name,solver,solver_version,environment_id,environment_revision,artifact_uri,artifact_sha256,reproducibility_fingerprint,locked_components\n\
         controlled_impedance_solver_execution_environment,reviewed,solver environment review,reviewed_2d_field_solver_env_lock,reviewed_2d_field_solver,2026.07,si_solver_env_a,env_rev_2026_07,artifacts/solver/execution_environment_rev_2026_07.json,9999aaaabbbbccccddddeeeeffff000011112222333344445555666677778888,env_fp_2026_07_a,solver_binary;material_library;config_lock\n",
    )
    .unwrap();
    let environment_import = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-manufacturing-metadata",
            "--project",
            entitlement_output.to_str().unwrap(),
            "--metadata",
            metadata.to_str().unwrap(),
            "--output",
            environment_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        environment_import.status.success(),
        "{}",
        String::from_utf8_lossy(&environment_import.stderr)
    );
    common::assert_yaml_file_valid(&environment_output, &validator);
    let enriched_with_environment: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&environment_output).unwrap()).unwrap();
    let environment = &enriched_with_environment["board"]["manufacturing"]["controlled_impedance"]
        ["solver_execution_environments"][0];
    assert_eq!(environment["name"], "reviewed_2d_field_solver_env_lock");
    assert_eq!(environment["environment_revision"], "env_rev_2026_07");
    assert_eq!(environment["locked_components"][2], "config_lock");
    let environment_manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(environment_manifest_output).unwrap())
            .unwrap();
    if let Err(error) = manifest_validator.validate(&environment_manifest) {
        panic!(
            "Solver execution environment metadata import manifest failed schema validation: {error}"
        );
    }
    assert_eq!(
        environment_manifest["rows"][0]["board_field"],
        "controlled_impedance.solver_execution_environments[]"
    );
    assert_eq!(
        environment_manifest["rows"][0]["normalized_value"]["reproducibility_fingerprint"],
        "env_fp_2026_07_a"
    );

    std::fs::write(
        &metadata,
        "field,value,source,name,solver,solver_version,run_id,artifact_uri,artifact_sha256,random_seed,numeric_tolerance_policy,max_residual_error,max_iterations\n\
         controlled_impedance_solver_run_log,reviewed,solver run review,reviewed_2d_field_solver_run_log_rf,reviewed_2d_field_solver,2026.07,rf_solver_run_2026_07_a,artifacts/solver/rf_solver_run_2026_07_a.log,aaaabbbbccccddddeeeeffff0000111122223333444455556666777788889999,seed_2026_07_rf,si_solver_tolerance_rev_a,0.000001,120\n",
    )
    .unwrap();
    let run_log_import = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-manufacturing-metadata",
            "--project",
            environment_output.to_str().unwrap(),
            "--metadata",
            metadata.to_str().unwrap(),
            "--output",
            run_log_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        run_log_import.status.success(),
        "{}",
        String::from_utf8_lossy(&run_log_import.stderr)
    );
    common::assert_yaml_file_valid(&run_log_output, &validator);
    let enriched_with_run_log: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&run_log_output).unwrap()).unwrap();
    let run_log = &enriched_with_run_log["board"]["manufacturing"]["controlled_impedance"]["solver_run_logs"]
        [0];
    assert_eq!(run_log["name"], "reviewed_2d_field_solver_run_log_rf");
    assert_eq!(run_log["run_id"], "rf_solver_run_2026_07_a");
    assert_eq!(run_log["max_iterations"], 120);
    let run_log_manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(run_log_manifest_output).unwrap()).unwrap();
    if let Err(error) = manifest_validator.validate(&run_log_manifest) {
        panic!("Solver run-log metadata import manifest failed schema validation: {error}");
    }
    assert_eq!(
        run_log_manifest["rows"][0]["board_field"],
        "controlled_impedance.solver_run_logs[]"
    );
    assert_eq!(
        run_log_manifest["rows"][0]["normalized_value"]["numeric_tolerance_policy"],
        "si_solver_tolerance_rev_a"
    );

    std::fs::write(
        &metadata,
        "field,value,source,name,material_library,material_library_revision,artifact_uri,artifact_sha256,corners,dielectric_layers,materials,content_fields,fabricator_stackup_revision,acceptance_artifact_uri,acceptance_artifact_sha256,accepted_by,accepted_corners,accepted_dielectric_layers,accepted_materials\n\
         controlled_impedance_solver_material_library,reviewed,solver material library,reviewed_stackup_materials_rev_b,reviewed_stackup_materials,rev_b,artifacts/solver/material_library_rev_b.json,abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd,nominal;high_dk,prepreg_1,FR-4 prepreg,corner;dielectric_layer;material;dielectric_constant;nominal_dielectric_constant,,,,,,,\n\
         controlled_impedance_solver_material_acceptance,reviewed,fabricator material acceptance,reviewed_stackup_materials_rev_b_acceptance,reviewed_stackup_materials,rev_b,,,,,,,stackup_rev_b,artifacts/fabricator/material_acceptance_rev_b.pdf,22223333444455556666777788889999aaaabbbbccccddddeeeeffff00001111,fabricator_si_review,nominal;high_dk,prepreg_1,FR-4 prepreg\n",
    )
    .unwrap();
    let library_import = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-manufacturing-metadata",
            "--project",
            run_log_output.to_str().unwrap(),
            "--metadata",
            metadata.to_str().unwrap(),
            "--output",
            library_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        library_import.status.success(),
        "{}",
        String::from_utf8_lossy(&library_import.stderr)
    );
    common::assert_yaml_file_valid(&library_output, &validator);
    let enriched_with_library: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&library_output).unwrap()).unwrap();
    let library = enriched_with_library["board"]["manufacturing"]["controlled_impedance"]
        ["solver_material_libraries"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|library| library["name"] == "reviewed_stackup_materials_rev_b")
        .unwrap();
    assert_eq!(library["name"], "reviewed_stackup_materials_rev_b");
    assert_eq!(library["material_library"], "reviewed_stackup_materials");
    assert_eq!(library["material_library_revision"], "rev_b");
    assert_eq!(library["corners"][1], "high_dk");
    assert_eq!(library["content_fields"][3], "dielectric_constant");
    let acceptance = enriched_with_library["board"]["manufacturing"]["controlled_impedance"]
        ["solver_material_acceptances"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|acceptance| acceptance["name"] == "reviewed_stackup_materials_rev_b_acceptance")
        .unwrap();
    assert_eq!(
        acceptance["acceptance_artifact_uri"],
        "artifacts/fabricator/material_acceptance_rev_b.pdf"
    );
    assert_eq!(acceptance["accepted_corners"][1], "high_dk");
    let library_manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(library_manifest_output).unwrap()).unwrap();
    if let Err(error) = manifest_validator.validate(&library_manifest) {
        panic!("Material-library metadata import manifest failed schema validation: {error}");
    }
    assert_eq!(
        library_manifest["rows"][0]["board_field"],
        "controlled_impedance.solver_material_libraries[]"
    );
    assert_eq!(
        library_manifest["rows"][0]["normalized_value"]["artifact_sha256"],
        "abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
    );
    assert_eq!(
        library_manifest["rows"][0]["normalized_value"]["content_fields"][3],
        "dielectric_constant"
    );
    assert_eq!(
        library_manifest["rows"][1]["board_field"],
        "controlled_impedance.solver_material_acceptances[]"
    );
    assert_eq!(
        library_manifest["rows"][1]["normalized_value"]["acceptance_artifact_sha256"],
        "22223333444455556666777788889999aaaabbbbccccddddeeeeffff00001111"
    );

    std::fs::write(
        &metadata,
        "field,value,source,name,material_library,material_library_revision,fabricator_stackup_revision,dielectric_layer,material,process_lot,material_lot,process_revision,drift_artifact_uri,drift_artifact_sha256,accepted_dielectric_constant,measured_dielectric_constant,max_dielectric_constant_delta,accepted_thickness_mm,measured_thickness_mm,max_thickness_delta_mm\n\
         controlled_impedance_solver_material_process,reviewed,fabricator material lot drift,reviewed_stackup_materials_rev_b_lot_a,reviewed_stackup_materials,rev_b,stackup_rev_b,prepreg_1,FR-4 prepreg,lot_2026_06_b,fr4_prepreg_lot_8,lamination_rev_d,artifacts/fabricator/material_lot_drift_rev_b.pdf,3333444455556666777788889999aaaabbbbccccddddeeeeffff000011112222,4.1,4.12,0.05,0.18,0.181,0.005\n",
    )
    .unwrap();
    let process_import = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-manufacturing-metadata",
            "--project",
            library_output.to_str().unwrap(),
            "--metadata",
            metadata.to_str().unwrap(),
            "--output",
            process_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        process_import.status.success(),
        "{}",
        String::from_utf8_lossy(&process_import.stderr)
    );
    common::assert_yaml_file_valid(&process_output, &validator);
    let enriched_with_process: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&process_output).unwrap()).unwrap();
    let process = enriched_with_process["board"]["manufacturing"]["controlled_impedance"]
        ["solver_material_processes"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|process| process["name"] == "reviewed_stackup_materials_rev_b_lot_a")
        .unwrap();
    assert_eq!(process["process_lot"], "lot_2026_06_b");
    assert_eq!(process["measured_dielectric_constant"], 4.12);
    let process_manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(process_manifest_output).unwrap()).unwrap();
    if let Err(error) = manifest_validator.validate(&process_manifest) {
        panic!("Material-process metadata import manifest failed schema validation: {error}");
    }
    assert_eq!(
        process_manifest["rows"][0]["board_field"],
        "controlled_impedance.solver_material_processes[]"
    );
    assert_eq!(
        process_manifest["rows"][0]["normalized_value"]["drift_artifact_sha256"],
        "3333444455556666777788889999aaaabbbbccccddddeeeeffff000011112222"
    );

    std::fs::write(
        &metadata,
        "field,value,unit,source,notes,name,solver,solver_version,qualification_artifact_uri,qualification_artifact_sha256\n\
         controlled_impedance_solver_qualification,qualified,,si tool qualification,reviewed tool/version qualification,reviewed_2d_field_solver_2026_07,reviewed_2d_field_solver,2026.07,artifacts/solver/reviewed_2d_field_solver_2026_07_qualification.pdf,11223344556677889900aabbccddeeff11223344556677889900aabbccddeeff\n",
    )
    .unwrap();
    let qualification_import = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-manufacturing-metadata",
            "--project",
            process_output.to_str().unwrap(),
            "--metadata",
            metadata.to_str().unwrap(),
            "--output",
            qualified_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        qualification_import.status.success(),
        "{}",
        String::from_utf8_lossy(&qualification_import.stderr)
    );

    let suggest_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "suggest-scenarios",
            qualified_output.to_str().unwrap(),
            "--output",
            suggestions_output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(suggest_status.success());
    let suggestions = read_suggestion_report(&suggestions_output);
    assert_runnable(
        &suggestions,
        "controlled_impedance_solver_result_rf_solver_result",
    );
}

#[test]
fn import_manufacturing_metadata_applies_solver_material_corner_rows() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir
        .path()
        .join("without_solver_material_corners.project.yaml");
    let metadata = dir.path().join("solver_material_corners.csv");
    let output = dir.path().join("with_solver_material_corners.project.yaml");
    let manifest_output = output.with_extension("manufacturing.json");
    let suggestions_output = dir.path().join("suggestions.yaml");
    let project_yaml: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string("examples/scenario_suggestions_controlled_impedance/project.yaml")
            .unwrap(),
    )
    .unwrap();
    std::fs::write(&input, serde_yaml_ng::to_string(&project_yaml).unwrap()).unwrap();
    std::fs::write(
        &metadata,
        "field,value,unit,source,notes,solver_result_name,name,corner,dielectric_layer,material,nominal_dielectric_constant,material_library,material_library_revision\n\
         controlled_impedance_solver_material_corner,4.1,,solver material library,nominal material corner,rf_solver_result,rf_solver_nominal_material,nominal,prepreg_1,FR-4 prepreg,4.1,reviewed_stackup_materials,rev_a\n\
         controlled_impedance_solver_material_corner,4.4,,solver material library,high-dk material corner,rf_solver_result,rf_solver_high_dk_material,high_dk,prepreg_1,FR-4 prepreg,4.1,reviewed_stackup_materials,rev_a\n",
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

    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    common::assert_yaml_file_valid(&output, &validator);
    let enriched: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let result = &enriched["board"]["manufacturing"]["controlled_impedance"]["solver_results"][0];
    assert_eq!(
        result["material_corners"][0]["name"],
        "rf_solver_nominal_material"
    );
    assert_eq!(result["material_corners"][0]["corner"], "nominal");
    assert_eq!(result["material_corners"][0]["dielectric_constant"], 4.1);
    assert_eq!(
        result["material_corners"][0]["nominal_dielectric_constant"],
        4.1
    );
    assert_eq!(result["material_corners"][1]["corner"], "high_dk");
    assert_eq!(result["material_corners"][1]["dielectric_constant"], 4.4);

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
    assert_eq!(manifest["schema_version"], "0.41.0");
    assert_eq!(
        manifest["rows"][0]["board_field"],
        "controlled_impedance.solver_results[].material_corners[]"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["material_library_revision"],
        "rev_a"
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
    assert_runnable(
        &suggestions,
        "controlled_impedance_solver_result_rf_solver_result",
    );
}

#[test]
fn import_manufacturing_metadata_applies_solver_qualification_rows() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("without_solver_qualification.project.yaml");
    let metadata = dir.path().join("solver_qualification.csv");
    let output = dir.path().join("with_solver_qualification.project.yaml");
    let manifest_output = output.with_extension("manufacturing.json");
    let suggestions_output = dir.path().join("suggestions.yaml");
    let project_yaml: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string("examples/scenario_suggestions_controlled_impedance/project.yaml")
            .unwrap(),
    )
    .unwrap();
    std::fs::write(&input, serde_yaml_ng::to_string(&project_yaml).unwrap()).unwrap();
    std::fs::write(
        &metadata,
        "field,value,unit,source,notes,name,solver,solver_version,qualification_artifact_uri,qualification_artifact_sha256\n\
         controlled_impedance_solver_qualification,qualified,,si tool qualification,reviewed tool/version qualification,reviewed_2d_field_solver_2026_06,reviewed_2d_field_solver,2026.06,artifacts/solver/reviewed_2d_field_solver_2026_06_qualification.pdf,00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff\n",
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

    let enriched: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let qualification =
        &enriched["board"]["manufacturing"]["controlled_impedance"]["solver_qualifications"][0];
    assert_eq!(qualification["name"], "reviewed_2d_field_solver_2026_06");
    assert_eq!(qualification["source"], "si tool qualification");
    assert_eq!(qualification["solver"], "reviewed_2d_field_solver");
    assert_eq!(qualification["solver_version"], "2026.06");
    assert_eq!(
        qualification["qualification_artifact_sha256"],
        "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff"
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
    assert_eq!(manifest["schema_version"], "0.41.0");
    assert_eq!(
        manifest["rows"][0]["board_field"],
        "controlled_impedance.solver_qualifications[]"
    );
    assert_eq!(
        manifest["rows"][0]["normalized_value"]["qualification_artifact_uri"],
        "artifacts/solver/reviewed_2d_field_solver_2026_06_qualification.pdf"
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
    assert_runnable(
        &suggestions,
        "controlled_impedance_solver_result_rf_solver_result",
    );
}

#[test]
fn import_manufacturing_metadata_applies_solver_rerun_rows() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("without_solver_reruns.project.yaml");
    let output = dir.path().join("with_solver_reruns.project.yaml");
    let manifest_output = output.with_extension("manufacturing.json");
    let metadata = dir.path().join("solver_reruns.csv");
    let mut project_yaml: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string("examples/scenario_suggestions_controlled_impedance/project.yaml")
            .unwrap(),
    )
    .unwrap();
    let run_logs =
        project_yaml["board"]["manufacturing"]["controlled_impedance"]["solver_run_logs"]
            .as_sequence_mut()
            .unwrap();
    run_logs.clear();
    std::fs::write(&input, serde_yaml_ng::to_string(&project_yaml).unwrap()).unwrap();
    let metadata_csv = [
        "field,value,source,name,solver,solver_version,run_id,artifact_uri,artifact_sha256,random_seed,numeric_tolerance_policy,max_residual_error,max_iterations,min_rerun_count,max_rerun_impedance_delta_ohm,solver_run_log,solved_impedance_ohm,residual_error,iterations",
        "controlled_impedance_solver_run_log,,si_solver_run_review_rev_a,reviewed_2d_field_solver_run_log_rf,reviewed_2d_field_solver,2026.07,rf_solver_run_2026_07_a,artifacts/solver/rf_solver_run_2026_07_a.log,aaaabbbbccccddddeeeeffff0000111122223333444455556666777788889999,seed_2026_07_rf,si_solver_tolerance_rev_a,0.000001,120,2,0.05,,,,",
        "controlled_impedance_solver_rerun,50.82,si_solver_rerun_review_rev_a,rf_solver_rerun_a,,,rf_solver_run_2026_07_a_rerun_1,artifacts/solver/rf_solver_run_2026_07_a_rerun_1.log,9999888877776666555544443333222211110000ffffeeeeddddccccbbbbaaaa,seed_2026_07_rf,,,,,,reviewed_2d_field_solver_run_log_rf,50.82,0.0000005,86",
        "controlled_impedance_solver_rerun,50.79,si_solver_rerun_review_rev_a,rf_solver_rerun_b,,,rf_solver_run_2026_07_a_rerun_2,artifacts/solver/rf_solver_run_2026_07_a_rerun_2.log,888877776666555544443333222211110000ffffeeeeddddccccbbbbaaaa9999,seed_2026_07_rf,,,,,,reviewed_2d_field_solver_run_log_rf,50.79,0.0000004,83",
    ]
    .join("\n");
    std::fs::write(&metadata, format!("{metadata_csv}\n")).unwrap();

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

    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    common::assert_yaml_file_valid(&output, &validator);
    let enriched: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output).unwrap()).unwrap();
    let run_log = &enriched["board"]["manufacturing"]["controlled_impedance"]["solver_run_logs"][0];
    assert_eq!(run_log["name"], "reviewed_2d_field_solver_run_log_rf");
    assert_eq!(run_log["min_rerun_count"], 2);
    assert_eq!(run_log["max_rerun_impedance_delta_ohm"], 0.05);
    assert_eq!(run_log["reruns"].as_sequence().unwrap().len(), 2);
    assert_eq!(run_log["reruns"][0]["name"], "rf_solver_rerun_a");
    assert_eq!(run_log["reruns"][1]["solved_impedance_ohm"], 50.79);

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(manifest_output).unwrap()).unwrap();
    assert_eq!(manifest["schema_version"], "0.41.0");
    assert_eq!(
        manifest["rows"][2]["board_field"],
        "controlled_impedance.solver_run_logs[].reruns[]"
    );
    assert_eq!(
        manifest["rows"][2]["raw_columns"]["solver_run_log"],
        "reviewed_2d_field_solver_run_log_rf"
    );
}
