mod common;

use common::assert_yaml_file_valid;
use serde_json::Value;
use std::process::Command;

#[test]
fn import_kicad_pcb_adds_layout_placements_for_suggestions() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let schematic_project = dir.path().join("usb_connector_imported.project.yaml");
    let enriched_project = dir.path().join("usb_connector_with_layout.project.yaml");
    let suggestions_path = dir.path().join("suggestions.yaml");

    let schematic_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-schematic",
            "examples/import_kicad_usb_connector_protection_suggestions/root.kicad_sch",
            "--mapping",
            "examples/import_kicad_usb_connector_protection_suggestions/circuitci.kicad-map.yaml",
            "--output",
            schematic_project.to_str().unwrap(),
            "--name",
            "kicad_usb_connector_protection_suggestions",
        ])
        .status()
        .unwrap();
    assert!(schematic_status.success());

    let pcb_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-pcb",
            "examples/import_kicad_usb_connector_protection_suggestions/board.kicad_pcb",
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
    assert_eq!(imported["board"]["layout"]["placements"]["J1"]["x_mm"], 0.0);
    assert_eq!(
        imported["board"]["layout"]["placements"]["J1"]["side"],
        "top"
    );
    assert_eq!(
        imported["board"]["layout"]["placements"]["UESD"]["x_mm"],
        1.0
    );
    assert_eq!(
        imported["board"]["layout"]["placements"]["UVBUS"]["x_mm"],
        1.5
    );
    assert!(imported["board"]["layout"]["placements"]["H1"].is_null());

    let suggest_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "suggest-scenarios",
            enriched_project.to_str().unwrap(),
            "--output",
            suggestions_path.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(suggest_status.success());
    let suggestions: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&suggestions_path).unwrap()).unwrap();
    let placement = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_protection_placement_j1")
        .expect("USB protection placement suggestion");
    assert_eq!(
        placement["scenario"]["checks"][0],
        "USB_PROTECTION_PLACEMENT_VALID"
    );
    assert!(
        placement["scenario"]["parameters"]["max_connector_to_protection_distance_mm"].is_null()
    );
    let clamps = placement["scenario"]["protection_clamps"]
        .as_array()
        .unwrap();
    assert!(clamps.iter().any(|clamp| {
        clamp["component"] == "UESD"
            && clamp["protected_net"] == "net_usb_dp"
            && clamp["distance_to_target_mm"] == 1.0
    }));
    assert!(clamps.iter().any(|clamp| {
        clamp["component"] == "UESD"
            && clamp["protected_net"] == "net_usb_dm"
            && clamp["distance_to_target_mm"] == 1.0
    }));
    assert!(clamps.iter().any(|clamp| {
        clamp["component"] == "UVBUS"
            && clamp["protected_net"] == "net_usb_vbus"
            && clamp["distance_to_target_mm"] == 1.5
    }));
}

#[test]
fn import_kicad_pcb_rewrites_relative_libraries_for_output_location() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let enriched_project = dir.path().join("usb_connector_with_layout.project.yaml");
    let suggestions_path = dir.path().join("suggestions.yaml");

    let pcb_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-pcb",
            "examples/import_kicad_usb_connector_protection_suggestions/board.kicad_pcb",
            "--project",
            "examples/scenario_suggestions_usb_connector_protection/project.yaml",
            "--output",
            enriched_project.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(pcb_status.success());

    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&enriched_project).unwrap()).unwrap();
    let libraries = imported["libraries"].as_array().unwrap();
    assert!(
        libraries
            .iter()
            .all(|library| { std::path::Path::new(library.as_str().unwrap()).is_absolute() })
    );

    let suggest_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "suggest-scenarios",
            enriched_project.to_str().unwrap(),
            "--output",
            suggestions_path.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(suggest_status.success());
    let suggestions: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&suggestions_path).unwrap()).unwrap();
    assert_eq!(suggestions["suggestions"].as_array().unwrap().len(), 6);
    assert!(
        suggestions["suggestions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|suggestion| suggestion["id"] == "usb_protection_placement_j1")
    );
}
