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
    let dp_route = &imported["board"]["layout"]["routes"]["net_usb_dp"];
    assert_eq!(dp_route["segments"][0]["start"]["x_mm"], 0.0);
    assert_eq!(dp_route["segments"][0]["end"]["x_mm"], 1.0);
    assert_eq!(dp_route["segments"][0]["width_mm"], 0.15);
    assert_eq!(dp_route["segments"][0]["layer"], "F.Cu");
    assert_eq!(dp_route["vias"][0]["at"]["x_mm"], 0.5);
    assert_eq!(dp_route["vias"][0]["size_mm"], 0.6);
    assert_eq!(dp_route["vias"][0]["drill_mm"], 0.3);
    assert_eq!(dp_route["vias"][0]["layers"][1], "B.Cu");
    assert_eq!(
        imported["board"]["layout"]["routes"]["net_usb_dm"]["segments"][0]["end"]["y_mm"],
        0.4
    );
    assert_eq!(
        imported["board"]["layout"]["routes"]["net_usb_vbus"]["segments"][0]["end"]["x_mm"],
        1.5
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
    let route = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_route_geometry_j1")
        .expect("USB route geometry suggestion");
    assert_eq!(route["scenario"]["checks"][0], "USB_ROUTE_GEOMETRY_VALID");
    assert!(route["scenario"]["parameters"]["max_data_line_route_length_mm"].is_null());
    assert!(
        route["scenario"]["usb_routes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|usb_route| {
                usb_route["signal"] == "D+"
                    && usb_route["net"] == "net_usb_dp"
                    && usb_route["route_length_mm"] == 1.0
                    && usb_route["via_count"] == 1
            })
    );
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
    assert_eq!(
        imported["board"]["layout"]["routes"]["usb_dp"]["segments"][0]["end"]["x_mm"],
        1.0
    );
    assert_eq!(
        imported["board"]["layout"]["routes"]["usb_vbus"]["segments"][0]["width_mm"],
        0.3
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
    assert_eq!(suggestions["suggestions"].as_array().unwrap().len(), 7);
    assert!(
        suggestions["suggestions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|suggestion| suggestion["id"] == "usb_protection_placement_j1")
    );
    assert!(
        suggestions["suggestions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|suggestion| suggestion["id"] == "usb_route_geometry_j1")
    );
}
