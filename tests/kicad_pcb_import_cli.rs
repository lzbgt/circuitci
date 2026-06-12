mod common;

use common::{assert_report_schema_valid, assert_yaml_file_valid};
use serde_json::Value;
use std::path::Path;
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
        imported["board"]["layout"]["placements"]["J1"]["rotation_deg"],
        0.0
    );
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
    let connector_footprint = &imported["board"]["layout"]["footprints"]["J1"];
    assert_eq!(connector_footprint["rectangles"][0]["kind"], "fabrication");
    assert_eq!(connector_footprint["rectangles"][0]["layer"], "F.Fab");
    assert_eq!(connector_footprint["rectangles"][0]["start"]["x_mm"], -0.7);
    assert_eq!(connector_footprint["rectangles"][0]["end"]["y_mm"], 1.2);
    assert_eq!(connector_footprint["segments"][0]["kind"], "courtyard");
    assert_eq!(connector_footprint["segments"][0]["layer"], "F.CrtYd");
    assert_eq!(connector_footprint["segments"][0]["start"]["x_mm"], -0.8);
    assert_eq!(connector_footprint["segments"][0]["end"]["x_mm"], 0.4);
    assert_eq!(connector_footprint["polygons"][0]["kind"], "fabrication");
    assert_eq!(connector_footprint["polygons"][0]["layer"], "F.Fab");
    assert_eq!(
        connector_footprint["polygons"][0]["points"][0]["x_mm"],
        -0.8
    );
    assert_eq!(connector_footprint["polygons"][0]["points"][3]["y_mm"], 1.2);
    assert_eq!(connector_footprint["circles"][0]["kind"], "fabrication");
    assert_eq!(connector_footprint["circles"][0]["layer"], "F.Fab");
    assert_eq!(connector_footprint["circles"][0]["center"]["x_mm"], 0.2);
    assert_eq!(connector_footprint["circles"][0]["end"]["x_mm"], 0.3);
    assert_eq!(connector_footprint["arcs"][0]["kind"], "courtyard");
    assert_eq!(connector_footprint["arcs"][0]["layer"], "F.CrtYd");
    assert_eq!(connector_footprint["arcs"][0]["start"]["x_mm"], 0.0);
    assert_eq!(connector_footprint["arcs"][0]["mid"]["y_mm"], 1.0);
    assert!(imported["board"]["layout"]["footprints"]["H1"].is_null());
    let connector_dp_pad = &imported["board"]["layout"]["pads"]["J1"]["D+"];
    assert_eq!(connector_dp_pad["at"]["x_mm"], 0.0);
    assert_eq!(connector_dp_pad["at"]["y_mm"], 0.2);
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
        imported["board"]["layout"]["pads"]["J1"]["GND"]["at"]["y_mm"],
        1.02
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
    let outline_segments = imported["board"]["layout"]["outline"]["segments"]
        .as_array()
        .unwrap();
    assert_eq!(outline_segments.len(), 52);
    assert_eq!(outline_segments[0]["layer"], "Edge.Cuts");
    assert_eq!(outline_segments[0]["source_primitive"], "gr_line");
    assert_eq!(outline_segments[0]["source_primitive_index"], 0);
    assert_eq!(outline_segments[0]["sample_index"], 0);
    assert_eq!(outline_segments[0]["sample_count"], 1);
    assert_eq!(outline_segments[0]["start"]["x_mm"], -0.4);
    assert_eq!(outline_segments[0]["end"]["x_mm"], 2.0);
    assert_eq!(outline_segments[4]["source_primitive"], "gr_circle");
    assert_eq!(outline_segments[4]["source_primitive_index"], 4);
    assert_eq!(outline_segments[4]["sample_index"], 0);
    assert_eq!(outline_segments[4]["sample_count"], 32);
    assert_eq!(outline_segments[4]["start"]["x_mm"], 1.9);
    assert_eq!(outline_segments[4]["start"]["y_mm"], 1.2);
    assert_eq!(outline_segments[36]["source_primitive"], "gr_arc");
    assert_eq!(outline_segments[36]["source_primitive_index"], 5);
    assert_eq!(outline_segments[36]["sample_index"], 0);
    assert_eq!(outline_segments[36]["sample_count"], 16);
    assert_eq!(outline_segments[36]["start"]["x_mm"], 1.6);
    assert_eq!(outline_segments[36]["start"]["y_mm"], 1.4);
    assert_eq!(outline_segments[51]["source_primitive"], "gr_arc");
    assert_eq!(outline_segments[51]["source_primitive_index"], 5);
    assert_eq!(outline_segments[51]["sample_index"], 15);
    assert_eq!(outline_segments[51]["sample_count"], 16);
    assert!((outline_segments[51]["end"]["x_mm"].as_f64().unwrap() - 2.0).abs() < 1e-12);
    assert!((outline_segments[51]["end"]["y_mm"].as_f64().unwrap() - 1.4).abs() < 1e-12);
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
    let orientation = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_connector_orientation_j1")
        .expect("USB connector orientation suggestion");
    assert_eq!(
        orientation["scenario"]["checks"][0],
        "USB_CONNECTOR_ORIENTATION_VALID"
    );
    assert_eq!(
        orientation["scenario"]["parameters"]["expected_connector_rotation_deg"],
        180.0
    );
    assert!(orientation["scenario"]["parameters"]["max_connector_rotation_error_deg"].is_null());
    assert_eq!(
        orientation["scenario"]["usb_connectors"][0]["placement"]["rotation_deg"],
        0.0
    );
    let nearest_edge = &orientation["scenario"]["usb_connectors"][0]["nearest_board_edge"];
    assert_eq!(nearest_edge["layer"], "Edge.Cuts");
    assert_eq!(nearest_edge["start"]["x_mm"], -0.4);
    assert_eq!(nearest_edge["end"]["y_mm"], -1.0);
    assert_eq!(nearest_edge["distance_to_connector_mm"], 0.0);
    assert_eq!(
        nearest_edge["connector_edge_reference"],
        "footprint_polygon"
    );
    assert_eq!(nearest_edge["footprint_graphic_layer"], "F.Fab");
    assert_eq!(nearest_edge["footprint_graphic_kind"], "fabrication");
    assert_eq!(nearest_edge["outward_normal_deg"], 180.0);
    assert_eq!(nearest_edge["connector_rotation_error_deg"], 180.0);
    let edge_proximity = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_connector_edge_proximity_j1")
        .expect("USB connector edge proximity suggestion");
    assert_eq!(
        edge_proximity["scenario"]["checks"][0],
        "USB_CONNECTOR_EDGE_PROXIMITY_VALID"
    );
    assert!(
        edge_proximity["scenario"]["parameters"]["max_connector_to_board_edge_distance_mm"]
            .is_null()
    );
    assert_eq!(
        edge_proximity["scenario"]["usb_connectors"][0]["nearest_board_edge"]["distance_to_connector_mm"],
        0.0
    );
    assert_eq!(
        edge_proximity["scenario"]["usb_connectors"][0]["nearest_board_edge"]["connector_edge_reference"],
        "footprint_polygon"
    );
    assert_eq!(
        edge_proximity["scenario"]["usb_connectors"][0]["footprint"]["rectangles"][0]["kind"],
        "fabrication"
    );
    assert_eq!(
        edge_proximity["scenario"]["usb_connectors"][0]["footprint"]["rectangles"][0]["layer"],
        "F.Fab"
    );
    assert_eq!(
        edge_proximity["scenario"]["usb_connectors"][0]["footprint"]["segments"][0]["kind"],
        "courtyard"
    );
    assert_eq!(
        edge_proximity["scenario"]["usb_connectors"][0]["footprint"]["polygons"][0]["kind"],
        "fabrication"
    );
    assert_eq!(
        edge_proximity["scenario"]["usb_connectors"][0]["footprint"]["circles"][0]["kind"],
        "fabrication"
    );
    assert_eq!(
        edge_proximity["scenario"]["usb_connectors"][0]["footprint"]["arcs"][0]["kind"],
        "courtyard"
    );
    let body_overhang = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_connector_body_overhang_j1")
        .expect("USB connector body overhang suggestion");
    assert_eq!(
        body_overhang["scenario"]["checks"][0],
        "USB_CONNECTOR_BODY_OVERHANG_VALID"
    );
    assert!(body_overhang["scenario"]["parameters"]["max_connector_body_overhang_mm"].is_null());
    assert_eq!(
        body_overhang["scenario"]["usb_connectors"][0]["nearest_board_edge"]["connector_edge_reference"],
        "footprint_polygon"
    );
    assert_eq!(
        body_overhang["scenario"]["usb_connectors"][0]["nearest_board_edge"]["footprint_graphic_layer"],
        "F.Fab"
    );
    assert_eq!(
        body_overhang["scenario"]["usb_connectors"][0]["nearest_board_edge"]["footprint_graphic_kind"],
        "fabrication"
    );
    let overhang = body_overhang["scenario"]["usb_connectors"][0]["nearest_board_edge"]
        ["connector_body_overhang_mm"]
        .as_f64()
        .unwrap();
    assert!((overhang - 0.4).abs() < 1e-12);
    let component_clearance = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_connector_component_clearance_j1")
        .expect("USB connector component-clearance suggestion");
    assert_eq!(
        component_clearance["scenario"]["checks"][0],
        "USB_CONNECTOR_COMPONENT_CLEARANCE_VALID"
    );
    assert!(
        component_clearance["scenario"]["parameters"]["min_connector_to_component_clearance_mm"]
            .is_null()
    );
    assert_eq!(
        component_clearance["scenario"]["usb_connectors"][0]["footprint"]["polygons"][0]["kind"],
        "fabrication"
    );
    let nearest_clearance =
        &component_clearance["scenario"]["usb_connectors"][0]["nearest_component_clearance"];
    assert_eq!(nearest_clearance["component"], "UESD");
    assert_eq!(nearest_clearance["clearance_mm"], 0.6);
    assert_eq!(
        nearest_clearance["connector_clearance_reference"],
        "footprint_polygon"
    );
    assert_eq!(
        nearest_clearance["connector_footprint_graphic_layer"],
        "F.Fab"
    );
    assert_eq!(
        nearest_clearance["connector_footprint_graphic_kind"],
        "fabrication"
    );
    assert_eq!(
        nearest_clearance["component_clearance_reference"],
        "placement_center"
    );
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
                    && usb_route["connector_pad"]["y_mm"] == 0.2
                    && usb_route["connector_pad"]["layers"][0] == "F.Cu"
                    && usb_route["connector_pad"]["kind"] == "smd"
                    && usb_route["connector_pad"]["shape"] == "rect"
                    && usb_route["connector_pad"]["size"]["x_mm"] == 0.3
                    && usb_route["protection_pad"]["component"] == "UESD"
                    && usb_route["protection_pad"]["pin"] == "D1+"
                    && usb_route["protection_pad"]["net"] == "net_usb_dp"
                    && usb_route["protection_pad"]["x_mm"] == 1.0
                    && usb_route["protection_pad"]["y_mm"] == 0.2
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
                                && contact["y_mm"] == 1.02
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
                                && contact["y_mm"] == 1.02
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
    assert_eq!(suggestions["suggestions"].as_array().unwrap().len(), 13);
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
            .any(|suggestion| suggestion["id"] == "usb_connector_edge_proximity_j1")
    );
    assert!(
        suggestions["suggestions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|suggestion| suggestion["id"] == "usb_connector_body_overhang_j1")
    );
    assert!(
        suggestions["suggestions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|suggestion| suggestion["id"] == "usb_connector_component_clearance_j1")
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
                                && contact["y_mm"] == 1.02
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
                                && contact["y_mm"] == 1.02
                        })
            })
    );
}

#[test]
fn import_kicad_pcb_curved_edge_segments_feed_usb_layout_checks() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    assert_curved_usb_board_edge_fixture(
        dir.path(),
        "circle",
        "examples/import_kicad_usb_curved_board_edge_suggestions/board_circle.kicad_pcb",
        32,
        CurvedEdgeExpectation {
            start_x_mm: 1.0,
            start_y_mm: 0.0,
            end_x_mm: 0.9807852804032304,
            end_y_mm: 0.19509032201612825,
            outward_normal_deg: 5.625,
            body_overhang_mm: 0.04764876029325276,
            source_primitive: "gr_circle",
            source_primitive_index: 0,
            sample_index: 0,
            sample_count: 32,
        },
    );
    assert_curved_usb_board_edge_fixture(
        dir.path(),
        "arc",
        "examples/import_kicad_usb_curved_board_edge_suggestions/board_arc.kicad_pcb",
        16,
        CurvedEdgeExpectation {
            start_x_mm: 0.09754516100806417,
            start_y_mm: 0.4903926402016152,
            end_x_mm: 0.0,
            end_y_mm: 0.5,
            outward_normal_deg: 84.375,
            body_overhang_mm: 0.06755245482669661,
            source_primitive: "gr_arc",
            source_primitive_index: 0,
            sample_index: 7,
            sample_count: 16,
        },
    );
}

#[test]
fn import_kicad_pcb_cutout_edges_are_not_usb_entry_edges() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let board_path = "examples/import_kicad_usb_cutout_board_edge_suggestions/board.kicad_pcb";
    let enriched_project = dir.path().join("cutout.project.yaml");
    import_kicad_pcb(
        board_path,
        "examples/import_kicad_usb_cutout_board_edge_suggestions/project.yaml",
        &enriched_project,
    );

    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&enriched_project, &validator);
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&enriched_project).unwrap()).unwrap();
    let outline_segments = imported["board"]["layout"]["outline"]["segments"]
        .as_array()
        .unwrap();
    assert_eq!(outline_segments.len(), 36);
    assert_eq!(outline_segments[0]["boundary_role"], "external");
    assert_eq!(outline_segments[0]["contour_index"], 0);
    assert_eq!(outline_segments[0]["source_primitive"], "gr_line");
    assert_eq!(outline_segments[4]["boundary_role"], "cutout");
    assert_eq!(outline_segments[4]["contour_index"], 1);
    assert_eq!(outline_segments[4]["source_primitive"], "gr_circle");
    assert!(
        outline_segments
            .iter()
            .skip(4)
            .all(|segment| segment["boundary_role"] == "cutout")
    );

    let suggestions_path = dir.path().join("cutout.suggestions.yaml");
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
    assert_cutout_suggestion_uses_external_edge(&suggestions, "usb_connector_orientation_j1");
    assert_cutout_suggestion_uses_external_edge(&suggestions, "usb_connector_edge_proximity_j1");
    assert_cutout_suggestion_uses_external_edge(&suggestions, "usb_connector_body_overhang_j1");

    let check_project = dir.path().join("cutout.checks.project.yaml");
    import_kicad_pcb(
        board_path,
        "examples/import_kicad_usb_cutout_board_edge_suggestions/project_checks.yaml",
        &check_project,
    );
    let report_dir = dir.path().join("cutout.report");
    let validate_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            check_project.to_str().unwrap(),
            "--profile",
            "iot_basic_v0",
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
    assert_eq!(report["summary"]["critical"], 1);
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "USB_CONNECTOR_EDGE_PROXIMITY_VALID")
        .unwrap();
    assert_eq!(
        failure["measured"]["connector_to_board_edge_distance_mm"],
        4.7
    );
    assert_eq!(failure["measured"]["board_edge_boundary_role"], "external");
    assert_eq!(failure["measured"]["board_edge_contour_index"], 0);
    assert_eq!(
        failure["measured"]["board_edge_source_primitive"],
        "gr_line"
    );
    assert_eq!(failure["measured"]["board_edge_source_primitive_index"], 1);
    assert_report_schema_valid(&report);
}

struct CurvedEdgeExpectation {
    start_x_mm: f64,
    start_y_mm: f64,
    end_x_mm: f64,
    end_y_mm: f64,
    outward_normal_deg: f64,
    body_overhang_mm: f64,
    source_primitive: &'static str,
    source_primitive_index: usize,
    sample_index: usize,
    sample_count: usize,
}

fn assert_cutout_suggestion_uses_external_edge(suggestions: &Value, suggestion_id: &str) {
    let suggestion = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == suggestion_id)
        .unwrap_or_else(|| panic!("missing suggestion {suggestion_id}"));
    let edge = &suggestion["scenario"]["usb_connectors"][0]["nearest_board_edge"];
    assert_eq!(edge["boundary_role"], "external");
    assert_eq!(edge["contour_index"], 0);
    assert_eq!(edge["source_primitive"], "gr_line");
    assert_eq!(edge["source_primitive_index"], 1);
    assert_eq!(edge["sample_index"], 0);
    assert_eq!(edge["sample_count"], 1);
    assert_eq!(edge["start"]["x_mm"], 10.0);
    assert_eq!(edge["start"]["y_mm"], 0.0);
    assert_eq!(edge["end"]["x_mm"], 10.0);
    assert_eq!(edge["end"]["y_mm"], 10.0);
    assert_eq!(edge["distance_to_connector_mm"], 4.7);
    assert_eq!(edge["connector_edge_reference"], "footprint_polygon");
}

fn assert_curved_usb_board_edge_fixture(
    dir: &Path,
    name: &str,
    board_path: &str,
    expected_outline_segments: usize,
    expected: CurvedEdgeExpectation,
) {
    let enriched_project = dir.join(format!("{name}.project.yaml"));
    import_kicad_pcb(
        board_path,
        "examples/import_kicad_usb_curved_board_edge_suggestions/project.yaml",
        &enriched_project,
    );

    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&enriched_project, &validator);
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&enriched_project).unwrap()).unwrap();
    assert_eq!(
        imported["board"]["layout"]["outline"]["segments"]
            .as_array()
            .unwrap()
            .len(),
        expected_outline_segments
    );

    let suggestions_path = dir.join(format!("{name}.suggestions.yaml"));
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
    assert_suggestion_uses_curved_edge(&suggestions, "usb_connector_orientation_j1", &expected);
    assert_suggestion_uses_curved_edge(&suggestions, "usb_connector_edge_proximity_j1", &expected);
    assert_suggestion_uses_curved_edge(&suggestions, "usb_connector_body_overhang_j1", &expected);

    let check_project = dir.join(format!("{name}.checks.project.yaml"));
    import_kicad_pcb(
        board_path,
        "examples/import_kicad_usb_curved_board_edge_suggestions/project_checks.yaml",
        &check_project,
    );
    let report_dir = dir.join(format!("{name}.report"));
    let validate_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            check_project.to_str().unwrap(),
            "--profile",
            "iot_basic_v0",
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
    assert_eq!(report["summary"]["critical"], 0);
}

fn import_kicad_pcb(board_path: &str, project_path: &str, output_path: &Path) {
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "import-kicad-pcb",
            board_path,
            "--project",
            project_path,
            "--output",
            output_path.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
}

fn assert_suggestion_uses_curved_edge(
    suggestions: &Value,
    suggestion_id: &str,
    expected: &CurvedEdgeExpectation,
) {
    let suggestion = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == suggestion_id)
        .unwrap_or_else(|| panic!("missing suggestion {suggestion_id}"));
    let edge = &suggestion["scenario"]["usb_connectors"][0]["nearest_board_edge"];
    assert_close(edge["start"]["x_mm"].as_f64().unwrap(), expected.start_x_mm);
    assert_close(edge["start"]["y_mm"].as_f64().unwrap(), expected.start_y_mm);
    assert_close(edge["end"]["x_mm"].as_f64().unwrap(), expected.end_x_mm);
    assert_close(edge["end"]["y_mm"].as_f64().unwrap(), expected.end_y_mm);
    assert_eq!(edge["layer"], "Edge.Cuts");
    assert_eq!(edge["source_primitive"], expected.source_primitive);
    assert_eq!(
        edge["source_primitive_index"],
        expected.source_primitive_index
    );
    assert_eq!(edge["sample_index"], expected.sample_index);
    assert_eq!(edge["sample_count"], expected.sample_count);
    assert_eq!(edge["distance_to_connector_mm"], 0.0);
    assert_eq!(edge["connector_edge_reference"], "footprint_polygon");
    assert_eq!(edge["footprint_graphic_layer"], "F.Fab");
    assert_eq!(edge["footprint_graphic_kind"], "fabrication");
    assert_close(
        edge["outward_normal_deg"].as_f64().unwrap(),
        expected.outward_normal_deg,
    );
    assert_close(
        edge["connector_body_overhang_mm"].as_f64().unwrap(),
        expected.body_overhang_mm,
    );
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1.0e-12,
        "expected {actual} to be close to {expected}"
    );
}
