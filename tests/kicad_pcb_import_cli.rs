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
    let connector_dp_pad = &imported["board"]["layout"]["pads"]["J1"]["D+"];
    assert_eq!(connector_dp_pad["at"]["x_mm"], 0.0);
    assert_eq!(connector_dp_pad["at"]["y_mm"], 0.0);
    assert_eq!(connector_dp_pad["net"], "net_usb_dp");
    assert_eq!(connector_dp_pad["layers"][0], "F.Cu");
    assert_eq!(connector_dp_pad["kind"], "smd");
    assert_eq!(connector_dp_pad["shape"], "rect");
    assert_eq!(connector_dp_pad["size"]["x_mm"], 0.3);
    assert_eq!(connector_dp_pad["size"]["y_mm"], 0.3);
    assert_eq!(
        imported["board"]["layout"]["pads"]["J1"]["SHIELD"]["net"],
        "gnd"
    );
    assert_eq!(
        imported["board"]["layout"]["pads"]["UESD"]["D1-"]["at"]["x_mm"],
        1.0
    );
    assert_eq!(
        imported["board"]["layout"]["pads"]["UESD"]["D1-"]["at"]["y_mm"],
        0.4
    );
    assert_eq!(
        imported["board"]["layout"]["pads"]["UVBUS"]["IO"]["net"],
        "net_usb_vbus"
    );
    assert_eq!(
        imported["board"]["layout"]["pads"]["UVBUS"]["IO"]["shape"],
        "rect"
    );
    assert!(imported["board"]["layout"]["pads"]["H1"].is_null());
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
    let ground_zones = imported["board"]["layout"]["zones"]["gnd"]
        .as_array()
        .unwrap();
    assert_eq!(ground_zones.len(), 1);
    assert_eq!(ground_zones[0]["layer"], "F.Cu");
    assert_eq!(ground_zones[0]["polygon"].as_array().unwrap().len(), 4);
    assert_eq!(ground_zones[0]["polygon"][0]["x_mm"], -1.0);
    assert_eq!(ground_zones[0]["polygon"][2]["y_mm"], 1.0);
    let filled_polygons = ground_zones[0]["filled_polygons"].as_array().unwrap();
    assert_eq!(filled_polygons.len(), 1);
    assert_eq!(filled_polygons[0].as_array().unwrap().len(), 4);
    assert_eq!(filled_polygons[0][0]["x_mm"], -0.9);
    assert_eq!(filled_polygons[0][2]["y_mm"], 0.9);
    let dp_rule = &imported["board"]["layout"]["constraints"]["net_rules"]["net_usb_dp"];
    assert_eq!(dp_rule["net_class"], "USB_HS");
    assert_eq!(dp_rule["track_width_mm"], 0.15);
    assert_eq!(dp_rule["diff_pair_width_mm"], 0.15);
    assert_eq!(dp_rule["diff_pair_gap_mm"], 0.15);
    assert_eq!(dp_rule["length_max_mm"], 25.0);
    assert_eq!(dp_rule["skew_max_mm"], 0.5);
    let dm_rule = &imported["board"]["layout"]["constraints"]["net_rules"]["net_usb_dm"];
    assert_eq!(dm_rule["net_class"], "USB_HS");
    assert_eq!(dm_rule["length_max_mm"], 25.0);
    assert_eq!(dm_rule["skew_max_mm"], 0.5);

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
    let connector = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_connector_protection_j1")
        .expect("USB connector protection suggestion");
    assert_eq!(
        connector["scenario"]["parameters"]["require_shield_ground"],
        true
    );
    assert_eq!(
        connector["scenario"]["usb_connectors"][0]["shield_net"],
        "gnd"
    );
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
    assert_eq!(
        route["scenario"]["parameters"]["max_data_line_route_length_mm"],
        25.0
    );
    assert_eq!(
        route["scenario"]["parameters"]["max_data_pair_length_mismatch_mm"],
        0.5
    );
    assert!(route["scenario"]["parameters"]["max_data_line_width_delta_mm"].is_null());
    assert!(route["scenario"]["parameters"]["max_data_pair_gap_delta_mm"].is_null());
    assert_eq!(
        route["scenario"]["parameters"]["require_route_pad_contact_evidence"],
        true
    );
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
                    && usb_route["expected_data_line_width_mm"] == 0.15
                    && usb_route["measured_data_line_width_mm"] == 0.15
                    && usb_route["data_line_width_delta_mm"] == 0.0
                    && usb_route["connector_pad"]["component"] == "J1"
                    && usb_route["connector_pad"]["pin"] == "D+"
                    && usb_route["connector_pad"]["net"] == "net_usb_dp"
                    && usb_route["connector_pad"]["x_mm"] == 0.0
                    && usb_route["connector_pad"]["layers"][0] == "F.Cu"
                    && usb_route["connector_pad"]["kind"] == "smd"
                    && usb_route["connector_pad"]["shape"] == "rect"
                    && usb_route["connector_pad"]["size"]["x_mm"] == 0.3
                    && usb_route["protection_pad"]["component"] == "UESD"
                    && usb_route["protection_pad"]["pin"] == "D1+"
                    && usb_route["protection_pad"]["net"] == "net_usb_dp"
                    && usb_route["protection_pad"]["x_mm"] == 1.0
                    && usb_route["protection_pad"]["layers"][0] == "F.Cu"
                    && usb_route["protection_pad"]["shape"] == "rect"
                    && usb_route["connector_pad_to_route_distance_mm"] == 0.0
                    && usb_route["protection_pad_to_route_distance_mm"] == 0.0
                    && usb_route["connector_to_protection_pad_route_distance_mm"] == 1.0
            })
    );
    assert!(
        route["scenario"]["usb_routes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|usb_route| {
                usb_route["signal"] == "D-"
                    && usb_route["connector_pad"]["pin"] == "D-"
                    && usb_route["protection_pad"]["pin"] == "D1-"
                    && usb_route["connector_to_protection_pad_route_distance_mm"] == 1.0
            })
    );
    let route_pair = &route["scenario"]["usb_route_pairs"].as_array().unwrap()[0];
    assert_eq!(route_pair["expected_data_pair_gap_mm"], 0.15);
    assert!((route_pair["measured_data_pair_gap_mm"].as_f64().unwrap() - 0.25).abs() < 1.0e-9);
    assert!((route_pair["data_pair_gap_delta_mm"].as_f64().unwrap() - 0.1).abs() < 1.0e-9);
    let vbus_route = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_vbus_route_j1")
        .expect("USB VBUS route suggestion");
    assert_eq!(
        vbus_route["scenario"]["parameters"]["require_vbus_route_pad_contact_evidence"],
        true
    );
    let vbus = &vbus_route["scenario"]["usb_routes"][0];
    assert_eq!(vbus["connector_pad"]["pin"], "VBUS");
    assert_eq!(vbus["connector_pad"]["net"], "net_usb_vbus");
    assert_eq!(vbus["connector_pad"]["shape"], "rect");
    assert_eq!(vbus["connector_pad"]["size"]["y_mm"], 0.3);
    assert_eq!(vbus["protection_pad"]["component"], "UVBUS");
    assert_eq!(vbus["protection_pad"]["pin"], "IO");
    assert_eq!(vbus["protection_pad"]["net"], "net_usb_vbus");
    assert_eq!(vbus["protection_pad"]["kind"], "smd");
    assert_eq!(vbus["connector_to_protection_pad_route_distance_mm"], 1.5);
    let return_path = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_return_path_j1")
        .expect("USB return-path suggestion");
    assert_eq!(
        return_path["scenario"]["checks"][0],
        "USB_RETURN_PATH_VALID"
    );
    assert!(
        return_path["scenario"]["parameters"]["max_data_line_unreferenced_length_mm"].is_null()
    );
    assert!(
        return_path["scenario"]["parameters"]["max_data_via_to_ground_stitch_distance_mm"]
            .is_null()
    );
    assert!(return_path["scenario"]["parameters"]["require_filled_zone_coverage"].is_null());
    assert!(
        return_path["scenario"]["parameters"]["min_data_line_filled_zone_edge_clearance_mm"]
            .is_null()
    );
    assert!(
        return_path["scenario"]["parameters"]["require_ground_zone_contact_evidence"].is_null()
    );
    assert!(
        return_path["scenario"]["usb_routes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|usb_route| {
                (usb_route["unreferenced_route_length_mm"].as_f64().unwrap() - 0.0).abs() < 1.0e-9
                    && (usb_route["filled_unreferenced_route_length_mm"]
                        .as_f64()
                        .unwrap()
                        - 0.0)
                        .abs()
                        < 1.0e-9
                    && usb_route["unreferenced_segments"]
                        .as_array()
                        .unwrap()
                        .is_empty()
                    && usb_route["filled_unreferenced_segments"]
                        .as_array()
                        .unwrap()
                        .is_empty()
                    && usb_route["filled_zone_edge_clearance_min_mm"]
                        .as_f64()
                        .unwrap()
                        > 0.0
                    && !usb_route["filled_zone_edge_clearance_segments"]
                        .as_array()
                        .unwrap()
                        .is_empty()
                    && usb_route["ground_zone_contacts"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|contact| {
                            contact["contact_kind"] == "pad"
                                && contact["component"] == "J1"
                                && contact["pad"] == "GND"
                                && contact["net"] == "gnd"
                        })
                    && usb_route["filled_ground_zone_contacts"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|contact| {
                            contact["contact_kind"] == "pad"
                                && contact["component"] == "J1"
                                && contact["pad"] == "GND"
                                && contact["net"] == "gnd"
                        })
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
    assert_eq!(
        imported["board"]["layout"]["pads"]["J1"]["D+"]["net"],
        "usb_dp"
    );
    assert_eq!(
        imported["board"]["layout"]["pads"]["UVBUS"]["IO"]["net"],
        "usb_vbus"
    );
    assert_eq!(
        imported["board"]["layout"]["zones"]["gnd"][0]["polygon"][2]["x_mm"],
        2.0
    );
    assert_eq!(
        imported["board"]["layout"]["zones"]["gnd"][0]["filled_polygons"][0][2]["x_mm"],
        1.9
    );
    assert_eq!(
        imported["board"]["layout"]["constraints"]["net_rules"]["usb_dp"]["length_max_mm"],
        25.0
    );
    assert_eq!(
        imported["board"]["layout"]["constraints"]["net_rules"]["usb_dm"]["skew_max_mm"],
        0.5
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
    assert_eq!(suggestions["suggestions"].as_array().unwrap().len(), 9);
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
    assert!(
        suggestions["suggestions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|suggestion| suggestion["id"] == "usb_vbus_route_j1")
    );
    assert!(
        suggestions["suggestions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|suggestion| suggestion["id"] == "usb_return_path_j1")
    );
    let route = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_route_geometry_j1")
        .unwrap();
    assert_eq!(
        route["scenario"]["parameters"]["max_data_line_route_length_mm"],
        25.0
    );
    assert_eq!(
        route["scenario"]["parameters"]["max_data_pair_length_mismatch_mm"],
        0.5
    );
    assert!(route["scenario"]["parameters"]["max_data_line_width_delta_mm"].is_null());
    assert!(route["scenario"]["parameters"]["max_data_pair_gap_delta_mm"].is_null());
    assert!(
        route["scenario"]["usb_routes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|usb_route| usb_route["expected_data_line_width_mm"] == 0.15)
    );
    assert!(
        route["scenario"]["usb_routes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|usb_route| {
                usb_route["measured_data_line_width_mm"] == 0.15
                    && usb_route["data_line_width_delta_mm"] == 0.0
            })
    );
    assert_eq!(
        route["scenario"]["usb_route_pairs"][0]["expected_data_pair_gap_mm"],
        0.15
    );
    assert_eq!(
        route["scenario"]["usb_route_pairs"][0]["measured_data_pair_gap_mm"],
        0.25
    );
    assert!(
        (route["scenario"]["usb_route_pairs"][0]["data_pair_gap_delta_mm"]
            .as_f64()
            .unwrap()
            - 0.1)
            .abs()
            < 1.0e-9
    );
    let vbus_route = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_vbus_route_j1")
        .unwrap();
    assert_eq!(vbus_route["scenario"]["checks"][0], "USB_VBUS_ROUTE_VALID");
    assert_eq!(
        vbus_route["scenario"]["parameters"]["max_vbus_route_length_mm"],
        20.0
    );
    assert!(vbus_route["scenario"]["parameters"]["max_vbus_via_count"].is_null());
    assert_eq!(
        vbus_route["scenario"]["parameters"]["min_vbus_route_width_mm"],
        0.3
    );
    assert_eq!(
        vbus_route["scenario"]["parameters"]["require_vbus_route_pad_contact_evidence"],
        true
    );
    let vbus = &vbus_route["scenario"]["usb_routes"][0];
    assert_eq!(vbus["signal"], "VBUS");
    assert_eq!(vbus["net"], "usb_vbus");
    assert_eq!(vbus["route_length_mm"], 1.5);
    assert_eq!(vbus["via_count"], 0);
    assert_eq!(vbus["expected_vbus_route_width_mm"], 0.3);
    assert_eq!(vbus["measured_vbus_route_width_min_mm"], 0.3);
    assert_eq!(vbus["protection_component"], "UVBUS");
    assert_eq!(vbus["connector_pad"]["component"], "J1");
    assert_eq!(vbus["connector_pad"]["pin"], "VBUS");
    assert_eq!(vbus["connector_pad"]["net"], "usb_vbus");
    assert_eq!(vbus["connector_pad"]["shape"], "rect");
    assert_eq!(vbus["connector_pad"]["size"]["x_mm"], 0.3);
    assert_eq!(vbus["protection_pad"]["component"], "UVBUS");
    assert_eq!(vbus["protection_pad"]["pin"], "IO");
    assert_eq!(vbus["protection_pad"]["net"], "usb_vbus");
    assert_eq!(vbus["protection_pad"]["kind"], "smd");
    assert_eq!(vbus["connector_pad_to_route_distance_mm"], 0.0);
    assert_eq!(vbus["protection_pad_to_route_distance_mm"], 0.0);
    assert_eq!(vbus["connector_to_protection_pad_route_distance_mm"], 1.5);
    let return_path = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_return_path_j1")
        .unwrap();
    assert_eq!(
        return_path["scenario"]["checks"][0],
        "USB_RETURN_PATH_VALID"
    );
    assert!(
        return_path["scenario"]["parameters"]["max_data_line_unreferenced_length_mm"].is_null()
    );
    assert!(
        return_path["scenario"]["parameters"]["max_data_via_to_ground_stitch_distance_mm"]
            .is_null()
    );
    assert!(return_path["scenario"]["parameters"]["require_filled_zone_coverage"].is_null());
    assert!(
        return_path["scenario"]["parameters"]["min_data_line_filled_zone_edge_clearance_mm"]
            .is_null()
    );
    assert!(
        return_path["scenario"]["parameters"]["require_ground_zone_contact_evidence"].is_null()
    );
    assert!(
        return_path["scenario"]["usb_routes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|usb_route| {
                usb_route["unreferenced_route_length_mm"] == 0.0
                    && usb_route["filled_unreferenced_route_length_mm"] == 0.0
                    && usb_route["unreferenced_segments"]
                        .as_array()
                        .unwrap()
                        .is_empty()
                    && usb_route["filled_unreferenced_segments"]
                        .as_array()
                        .unwrap()
                        .is_empty()
                    && usb_route["filled_zone_edge_clearance_min_mm"]
                        .as_f64()
                        .unwrap()
                        > 0.0
                    && !usb_route["filled_zone_edge_clearance_segments"]
                        .as_array()
                        .unwrap()
                        .is_empty()
                    && usb_route["ground_zone_contacts"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|contact| {
                            contact["contact_kind"] == "pad"
                                && contact["component"] == "J1"
                                && contact["pad"] == "GND"
                                && contact["net"] == "gnd"
                        })
                    && usb_route["filled_ground_zone_contacts"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|contact| {
                            contact["contact_kind"] == "pad"
                                && contact["component"] == "J1"
                                && contact["pad"] == "GND"
                                && contact["net"] == "gnd"
                        })
            })
    );
}
