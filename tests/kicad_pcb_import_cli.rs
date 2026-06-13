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
    assert_eq!(
        connector_footprint["entry_direction"]["source"],
        "kicad_footprint_property"
    );
    assert_eq!(connector_footprint["entry_direction"]["offset_deg"], 0.0);
    assert_eq!(
        connector_footprint["entry_clearance"]["source"],
        "kicad_footprint_property"
    );
    assert_eq!(connector_footprint["entry_clearance"]["depth_mm"], 2.5);
    assert_eq!(
        imported["board"]["layout"]["footprints"]["UESD"]["rectangles"][0]["kind"],
        "fabrication"
    );
    assert_eq!(
        imported["board"]["layout"]["footprints"]["UESD"]["rectangles"][0]["layer"],
        "F.Fab"
    );
    assert_eq!(
        imported["board"]["layout"]["footprints"]["UESD"]["rectangles"][0]["start"]["x_mm"],
        0.9
    );
    assert_eq!(
        imported["board"]["layout"]["footprints"]["UESD"]["rectangles"][0]["end"]["y_mm"],
        0.7
    );
    assert_eq!(
        imported["board"]["layout"]["footprints"]["UVBUS"]["rectangles"][0]["start"]["x_mm"],
        1.38
    );
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
    assert_eq!(outline_segments.len(), 59);
    assert_eq!(outline_segments[0]["layer"], "Edge.Cuts");
    assert_eq!(outline_segments[0]["source_primitive"], "gr_line");
    assert_eq!(outline_segments[0]["source_primitive_index"], 0);
    assert_eq!(outline_segments[0]["sample_index"], 0);
    assert_eq!(outline_segments[0]["sample_count"], 1);
    assert_eq!(outline_segments[0]["start"]["x_mm"], -0.4);
    assert_eq!(outline_segments[0]["end"]["x_mm"], 2.0);
    assert_eq!(outline_segments[4]["source_primitive"], "gr_rect");
    assert_eq!(outline_segments[4]["source_primitive_index"], 4);
    assert_eq!(outline_segments[4]["sample_index"], 0);
    assert_eq!(outline_segments[4]["sample_count"], 4);
    assert_eq!(outline_segments[4]["boundary_role"], "cutout");
    assert_eq!(outline_segments[4]["start"]["x_mm"], 1.55);
    assert_eq!(outline_segments[4]["start"]["y_mm"], 1.05);
    assert_eq!(outline_segments[7]["source_primitive"], "gr_rect");
    assert_eq!(outline_segments[7]["source_primitive_index"], 4);
    assert_eq!(outline_segments[7]["sample_index"], 3);
    assert_eq!(outline_segments[7]["sample_count"], 4);
    assert_eq!(outline_segments[7]["end"]["x_mm"], 1.55);
    assert_eq!(outline_segments[7]["end"]["y_mm"], 1.05);
    assert_eq!(outline_segments[8]["source_primitive"], "gr_poly");
    assert_eq!(outline_segments[8]["source_primitive_index"], 5);
    assert_eq!(outline_segments[8]["sample_index"], 0);
    assert_eq!(outline_segments[8]["sample_count"], 3);
    assert_eq!(outline_segments[8]["boundary_role"], "cutout");
    assert_eq!(outline_segments[8]["start"]["x_mm"], 1.1);
    assert_eq!(outline_segments[8]["start"]["y_mm"], 1.05);
    assert_eq!(outline_segments[10]["source_primitive"], "gr_poly");
    assert_eq!(outline_segments[10]["source_primitive_index"], 5);
    assert_eq!(outline_segments[10]["sample_index"], 2);
    assert_eq!(outline_segments[10]["sample_count"], 3);
    assert_eq!(outline_segments[10]["end"]["x_mm"], 1.1);
    assert_eq!(outline_segments[10]["end"]["y_mm"], 1.05);
    assert_eq!(outline_segments[11]["source_primitive"], "gr_circle");
    assert_eq!(outline_segments[11]["source_primitive_index"], 6);
    assert_eq!(outline_segments[11]["sample_index"], 0);
    assert_eq!(outline_segments[11]["sample_count"], 32);
    assert_eq!(outline_segments[11]["start"]["x_mm"], 1.9);
    assert_eq!(outline_segments[11]["start"]["y_mm"], 1.2);
    assert_eq!(outline_segments[43]["source_primitive"], "gr_arc");
    assert_eq!(outline_segments[43]["source_primitive_index"], 7);
    assert_eq!(outline_segments[43]["sample_index"], 0);
    assert_eq!(outline_segments[43]["sample_count"], 16);
    assert_eq!(outline_segments[43]["start"]["x_mm"], 1.6);
    assert_eq!(outline_segments[43]["start"]["y_mm"], 1.4);
    assert_eq!(outline_segments[58]["source_primitive"], "gr_arc");
    assert_eq!(outline_segments[58]["source_primitive_index"], 7);
    assert_eq!(outline_segments[58]["sample_index"], 15);
    assert_eq!(outline_segments[58]["sample_count"], 16);
    assert!((outline_segments[58]["end"]["x_mm"].as_f64().unwrap() - 2.0).abs() < 1e-12);
    assert!((outline_segments[58]["end"]["y_mm"].as_f64().unwrap() - 1.4).abs() < 1e-12);
    let ground_zones = imported["board"]["layout"]["zones"]["gnd"]
        .as_array()
        .unwrap();
    assert_eq!(ground_zones.len(), 1);
    assert_eq!(ground_zones[0]["layer"], "F.Cu");
    assert_eq!(ground_zones[0]["island_id"], "F_Cu_GND_zone_0");
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
    assert_eq!(nearest_edge["connector_entry_direction_offset_deg"], 0.0);
    assert_eq!(
        nearest_edge["connector_entry_direction_offset_source"],
        "footprint_property"
    );
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
    assert_eq!(nearest_clearance["clearance_mm"], 0.5);
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
        "footprint_rectangle"
    );
    assert_eq!(
        nearest_clearance["component_footprint_graphic_layer"],
        "F.Fab"
    );
    assert_eq!(
        nearest_clearance["component_footprint_graphic_kind"],
        "fabrication"
    );
    let entry_clearance = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_connector_entry_clearance_j1")
        .expect("USB connector entry-clearance suggestion");
    assert_eq!(entry_clearance["runnable"], true);
    assert!(entry_clearance.get("required_inputs").is_none());
    assert_eq!(
        entry_clearance["scenario"]["checks"][0],
        "USB_CONNECTOR_ENTRY_CLEARANCE_VALID"
    );
    assert_eq!(
        entry_clearance["scenario"]["parameters"]["entry_direction_deg"],
        0.0
    );
    let entry_evidence = &entry_clearance["scenario"]["usb_connectors"][0]["entry_clearance"];
    assert_eq!(entry_evidence["entry_direction_deg"], 0.0);
    assert_eq!(
        entry_evidence["entry_direction_source"],
        "footprint_property_offset"
    );
    assert_eq!(entry_evidence["entry_direction_offset_deg"], 0.0);
    assert_eq!(
        entry_clearance["scenario"]["usb_connectors"][0]["footprint"]["entry_direction"]["source"],
        "kicad_footprint_property"
    );
    assert_eq!(
        entry_clearance["scenario"]["usb_connectors"][0]["footprint"]["entry_direction"]["offset_deg"],
        0.0
    );
    assert_eq!(
        entry_clearance["scenario"]["usb_connectors"][0]["footprint"]["entry_clearance"]["source"],
        "kicad_footprint_property"
    );
    assert_eq!(
        entry_clearance["scenario"]["usb_connectors"][0]["footprint"]["entry_clearance"]["depth_mm"],
        2.5
    );
    assert_eq!(
        entry_clearance["scenario"]["usb_connectors"][0]["footprint"]["entry_clearance"]["width_mm"],
        1.4
    );
    assert_eq!(
        entry_evidence["entry_clearance_depth_source"],
        "footprint_property_depth"
    );
    assert_eq!(
        entry_evidence["suggested_min_cable_entry_clearance_depth_mm"],
        2.5
    );
    assert_eq!(
        entry_evidence["entry_clearance_width_source"],
        "footprint_property_width"
    );
    assert_eq!(
        entry_evidence["suggested_cable_entry_clearance_width_mm"],
        1.4
    );
    assert_eq!(
        entry_clearance["scenario"]["usb_connectors"][0]["footprint"]["entry_aperture"]["source"],
        "kicad_footprint_property"
    );
    assert_eq!(
        entry_clearance["scenario"]["usb_connectors"][0]["footprint"]["entry_aperture"]["width_mm"],
        1.0
    );
    assert_eq!(
        entry_evidence["entry_aperture_source"],
        "footprint_property_aperture"
    );
    assert_eq!(entry_evidence["connector_front_projection_mm"], 0.4);
    assert_eq!(entry_evidence["entry_aperture_front_projection_mm"], 0.4);
    assert_eq!(
        entry_evidence["entry_aperture_center_lateral_projection_mm"],
        0.0
    );
    assert_eq!(entry_evidence["entry_aperture_width_mm"], 1.0);
    assert_eq!(
        entry_evidence["aperture_min_effective_clearance_width_mm"],
        1.0
    );
    assert_eq!(entry_evidence["nearest_obstruction"]["component"], "UESD");
    assert_eq!(
        entry_evidence["nearest_obstruction"]["obstruction_depth_mm"],
        0.5
    );
    assert_eq!(
        entry_evidence["nearest_obstruction"]["obstruction_lateral_offset_mm"],
        -0.1
    );
    assert_eq!(
        entry_evidence["nearest_obstruction"]["obstruction_reference"],
        "footprint_rectangle"
    );
    assert_eq!(
        entry_evidence["nearest_obstruction"]["obstruction_footprint_graphic_kind"],
        "fabrication"
    );
    assert_eq!(
        entry_clearance["scenario"]["parameters"]["min_cable_entry_clearance_depth_mm"],
        2.5
    );
    assert_eq!(
        entry_clearance["scenario"]["parameters"]["cable_entry_clearance_width_mm"],
        1.4
    );
    let route = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_route_geometry_j1")
        .expect("USB route geometry suggestion");
    assert_eq!(route["runnable"], true);
    assert!(route.get("required_inputs").is_none());
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
    assert!(route["scenario"]["parameters"]["require_route_pad_contact_evidence"].is_null());
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
    assert_eq!(vbus_route["runnable"], false);
    assert!(
        vbus_route["required_inputs"][0]
            .as_str()
            .unwrap()
            .contains("max_vbus_route_length_mm")
    );
    assert_eq!(
        vbus_route["scenario"]["parameters"]["require_vbus_route_pad_contact_evidence"],
        serde_json::Value::Null
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
    assert_eq!(return_path["runnable"], false);
    assert!(
        return_path["required_inputs"][0]
            .as_str()
            .unwrap()
            .contains("usb_return_path.max_data_line_unreferenced_length_mm")
    );
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

    let board_without_aperture = dir.path().join("board_without_aperture.kicad_pcb");
    let board_text = std::fs::read_to_string(
        "examples/import_kicad_usb_connector_protection_suggestions/board.kicad_pcb",
    )
    .unwrap()
    .lines()
    .filter(|line| {
        !line.contains("CircuitCI_EntryAperture")
            && !line.contains("CircuitCI_EntryDirection")
            && !line.contains("CircuitCI_EntryClearance")
    })
    .collect::<Vec<_>>()
    .join("\n");
    std::fs::write(&board_without_aperture, board_text).unwrap();
    let mapped_aperture_project = dir
        .path()
        .join("usb_connector_mapping_aperture.project.yaml");
    import_kicad_pcb(
        board_without_aperture.to_str().unwrap(),
        schematic_project.to_str().unwrap(),
        &mapped_aperture_project,
    );
    let mapped_aperture_suggestions_path = dir.path().join("mapping_aperture.suggestions.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "suggest-scenarios",
            mapped_aperture_project.to_str().unwrap(),
            "--output",
            mapped_aperture_suggestions_path.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let mapped_aperture_suggestions: Value = serde_yaml_ng::from_str(
        &std::fs::read_to_string(&mapped_aperture_suggestions_path).unwrap(),
    )
    .unwrap();
    let mapped_entry_clearance = mapped_aperture_suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_connector_entry_clearance_j1")
        .expect("USB connector entry-clearance suggestion from mapping aperture");
    assert_eq!(mapped_entry_clearance["runnable"], true);
    assert!(mapped_entry_clearance.get("required_inputs").is_none());
    let mapped_entry_evidence =
        &mapped_entry_clearance["scenario"]["usb_connectors"][0]["entry_clearance"];
    assert_eq!(
        mapped_entry_clearance["scenario"]["usb_connectors"][0]["footprint"]["entry_aperture"]["source"],
        "kicad_mapping"
    );
    assert_eq!(
        mapped_entry_clearance["scenario"]["usb_connectors"][0]["footprint"]["entry_direction"]["source"],
        "kicad_mapping"
    );
    assert_eq!(
        mapped_entry_clearance["scenario"]["usb_connectors"][0]["footprint"]["entry_clearance"]["source"],
        "kicad_mapping"
    );
    assert_eq!(
        mapped_entry_clearance["scenario"]["usb_connectors"][0]["footprint"]["entry_clearance"]["depth_mm"],
        2.0
    );
    assert_eq!(
        mapped_entry_clearance["scenario"]["usb_connectors"][0]["footprint"]["entry_clearance"]["width_mm"],
        1.3
    );
    assert_eq!(
        mapped_entry_evidence["entry_direction_source"],
        "kicad_mapping_offset"
    );
    assert_eq!(mapped_entry_evidence["entry_direction_offset_deg"], 0.0);
    assert_eq!(
        mapped_entry_evidence["entry_clearance_depth_source"],
        "kicad_mapping_depth"
    );
    assert_eq!(
        mapped_entry_evidence["suggested_min_cable_entry_clearance_depth_mm"],
        2.0
    );
    assert_eq!(
        mapped_entry_clearance["scenario"]["parameters"]["min_cable_entry_clearance_depth_mm"],
        2.0
    );
    assert_eq!(
        mapped_entry_evidence["entry_clearance_width_source"],
        "kicad_mapping_width"
    );
    assert_eq!(
        mapped_entry_evidence["suggested_cable_entry_clearance_width_mm"],
        1.3
    );
    assert_eq!(
        mapped_entry_clearance["scenario"]["parameters"]["cable_entry_clearance_width_mm"],
        1.3
    );
    assert_eq!(
        mapped_entry_evidence["entry_aperture_source"],
        "kicad_mapping_aperture"
    );
    assert_eq!(mapped_entry_evidence["entry_aperture_width_mm"], 1.2);
    assert_eq!(
        mapped_entry_evidence["aperture_min_effective_clearance_width_mm"],
        1.2
    );
}

#[test]
fn import_kicad_pcb_rejects_invalid_entry_aperture_properties() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let source_pcb = std::fs::read_to_string(
        "examples/import_kicad_usb_connector_protection_suggestions/board.kicad_pcb",
    )
    .unwrap();
    let width_property =
        "(property \"CircuitCI_EntryApertureWidthMM\" \"1.0\" (at 0 1.6 0) (layer \"F.Fab\"))";
    let malformed_cases = [
        (
            "duplicate_aperture_width.kicad_pcb",
            source_pcb.replace(width_property, &format!("{width_property}\n    {width_property}")),
            "duplicate aperture property",
        ),
        (
            "nonpositive_aperture_width.kicad_pcb",
            source_pcb.replace(
                width_property,
                "(property \"CircuitCI_EntryApertureWidthMM\" \"0.0\" (at 0 1.6 0) (layer \"F.Fab\"))",
            ),
            "must be greater than zero",
        ),
    ];

    for (file_name, pcb_contents, expected_error) in malformed_cases {
        let pcb_path = dir.path().join(file_name);
        let output_path = dir.path().join(format!("{file_name}.project.yaml"));
        std::fs::write(&pcb_path, pcb_contents).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
            .args([
                "import-kicad-pcb",
                pcb_path.to_str().unwrap(),
                "--project",
                "examples/import_kicad_usb_connector_protection_suggestions/project.yaml",
                "--output",
                output_path.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            !output.status.success(),
            "malformed KiCad PCB aperture fixture {file_name} unexpectedly imported"
        );
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.contains(expected_error),
            "expected {expected_error:?} in stderr for {file_name}, got:\n{stderr}"
        );
    }
}

#[test]
fn import_kicad_pcb_rejects_invalid_entry_direction_properties() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let source_pcb = std::fs::read_to_string(
        "examples/import_kicad_usb_connector_protection_suggestions/board.kicad_pcb",
    )
    .unwrap();
    let direction_property =
        "(property \"CircuitCI_EntryDirectionOffsetDeg\" \"0.0\" (at 0 1.0 0) (layer \"F.Fab\"))";
    let malformed_cases = [
        (
            "duplicate_entry_direction.kicad_pcb",
            source_pcb.replace(
                direction_property,
                &format!("{direction_property}\n    {direction_property}"),
            ),
            "duplicate entry-direction property",
        ),
        (
            "nonfinite_entry_direction.kicad_pcb",
            source_pcb.replace(
                direction_property,
                "(property \"CircuitCI_EntryDirectionOffsetDeg\" \"inf\" (at 0 1.0 0) (layer \"F.Fab\"))",
            ),
            "must be finite",
        ),
    ];

    for (file_name, pcb_contents, expected_error) in malformed_cases {
        let pcb_path = dir.path().join(file_name);
        let output_path = dir.path().join(format!("{file_name}.project.yaml"));
        std::fs::write(&pcb_path, pcb_contents).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
            .args([
                "import-kicad-pcb",
                pcb_path.to_str().unwrap(),
                "--project",
                "examples/import_kicad_usb_connector_protection_suggestions/project.yaml",
                "--output",
                output_path.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            !output.status.success(),
            "malformed KiCad PCB entry-direction fixture {file_name} unexpectedly imported"
        );
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.contains(expected_error),
            "expected error containing {expected_error:?}, got:\n{stderr}"
        );
    }
}

#[test]
fn import_kicad_pcb_rejects_invalid_entry_clearance_properties() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let source_pcb = std::fs::read_to_string(
        "examples/import_kicad_usb_connector_protection_suggestions/board.kicad_pcb",
    )
    .unwrap();
    let clearance_depth_property =
        "(property \"CircuitCI_EntryClearanceDepthMM\" \"2.5\" (at 0 1.1 0) (layer \"F.Fab\"))";
    let clearance_width_property =
        "(property \"CircuitCI_EntryClearanceWidthMM\" \"1.4\" (at 0 1.15 0) (layer \"F.Fab\"))";
    let malformed_cases = [
        (
            "duplicate_entry_clearance.kicad_pcb",
            source_pcb.replace(
                clearance_depth_property,
                &format!("{clearance_depth_property}\n    {clearance_depth_property}"),
            ),
            "duplicate entry-clearance property",
        ),
        (
            "nonpositive_entry_clearance.kicad_pcb",
            source_pcb.replace(
                clearance_depth_property,
                "(property \"CircuitCI_EntryClearanceDepthMM\" \"0.0\" (at 0 1.1 0) (layer \"F.Fab\"))",
            ),
            "must be greater than zero",
        ),
        (
            "duplicate_entry_clearance_width.kicad_pcb",
            source_pcb.replace(
                clearance_width_property,
                &format!("{clearance_width_property}\n    {clearance_width_property}"),
            ),
            "duplicate entry-clearance property",
        ),
        (
            "nonpositive_entry_clearance_width.kicad_pcb",
            source_pcb.replace(
                clearance_width_property,
                "(property \"CircuitCI_EntryClearanceWidthMM\" \"0.0\" (at 0 1.15 0) (layer \"F.Fab\"))",
            ),
            "must be greater than zero",
        ),
    ];

    for (file_name, pcb_contents, expected_error) in malformed_cases {
        let pcb_path = dir.path().join(file_name);
        let output_path = dir.path().join(format!("{file_name}.project.yaml"));
        std::fs::write(&pcb_path, pcb_contents).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
            .args([
                "import-kicad-pcb",
                pcb_path.to_str().unwrap(),
                "--project",
                "examples/import_kicad_usb_connector_protection_suggestions/project.yaml",
                "--output",
                output_path.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            !output.status.success(),
            "malformed KiCad PCB entry-clearance fixture {file_name} unexpectedly imported"
        );
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.contains(expected_error),
            "expected error containing {expected_error:?}, got:\n{stderr}"
        );
    }
}

#[test]
fn import_kicad_pcb_rejects_degenerate_rectangular_board_outlines() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let source_pcb = std::fs::read_to_string(
        "examples/import_kicad_usb_connector_protection_suggestions/board.kicad_pcb",
    )
    .unwrap();
    let rect_outline = "(gr_rect (start 1.55 1.05) (end 1.70 1.20) (stroke (width 0.05) (type solid)) (fill none) (layer \"Edge.Cuts\"))";
    let malformed_cases = [
        (
            "zero_width_rect_outline.kicad_pcb",
            source_pcb.replace(
                rect_outline,
                "(gr_rect (start 1.55 1.05) (end 1.55 1.20) (stroke (width 0.05) (type solid)) (fill none) (layer \"Edge.Cuts\"))",
            ),
        ),
        (
            "zero_height_rect_outline.kicad_pcb",
            source_pcb.replace(
                rect_outline,
                "(gr_rect (start 1.55 1.05) (end 1.70 1.05) (stroke (width 0.05) (type solid)) (fill none) (layer \"Edge.Cuts\"))",
            ),
        ),
    ];

    for (file_name, pcb_contents) in malformed_cases {
        let pcb_path = dir.path().join(file_name);
        let output_path = dir.path().join(format!("{file_name}.project.yaml"));
        std::fs::write(&pcb_path, pcb_contents).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
            .args([
                "import-kicad-pcb",
                pcb_path.to_str().unwrap(),
                "--project",
                "examples/import_kicad_usb_connector_protection_suggestions/project.yaml",
                "--output",
                output_path.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            !output.status.success(),
            "degenerate KiCad PCB rectangular outline fixture {file_name} unexpectedly imported"
        );
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.contains("Edge.Cuts gr_rect") && stderr.contains("zero width or height"),
            "expected rectangular outline error for {file_name}, got:\n{stderr}"
        );
    }
}

#[test]
fn import_kicad_pcb_rejects_malformed_polygon_board_outlines() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let source_pcb = std::fs::read_to_string(
        "examples/import_kicad_usb_connector_protection_suggestions/board.kicad_pcb",
    )
    .unwrap();
    let polygon_outline = "(gr_poly (pts (xy 1.10 1.05) (xy 1.25 1.05) (xy 1.175 1.20)) (stroke (width 0.05) (type solid)) (fill none) (layer \"Edge.Cuts\"))";
    let malformed_cases = [
        (
            "missing_pts_poly_outline.kicad_pcb",
            source_pcb.replace(
                polygon_outline,
                "(gr_poly (stroke (width 0.05) (type solid)) (fill none) (layer \"Edge.Cuts\"))",
            ),
            "missing pts list",
        ),
        (
            "too_few_points_poly_outline.kicad_pcb",
            source_pcb.replace(
                polygon_outline,
                "(gr_poly (pts (xy 1.10 1.05) (xy 1.25 1.05)) (stroke (width 0.05) (type solid)) (fill none) (layer \"Edge.Cuts\"))",
            ),
            "fewer than three points",
        ),
    ];

    for (file_name, pcb_contents, expected_error) in malformed_cases {
        let pcb_path = dir.path().join(file_name);
        let output_path = dir.path().join(format!("{file_name}.project.yaml"));
        std::fs::write(&pcb_path, pcb_contents).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
            .args([
                "import-kicad-pcb",
                pcb_path.to_str().unwrap(),
                "--project",
                "examples/import_kicad_usb_connector_protection_suggestions/project.yaml",
                "--output",
                output_path.to_str().unwrap(),
            ])
            .output()
            .unwrap();
        assert!(
            !output.status.success(),
            "malformed KiCad PCB polygon outline fixture {file_name} unexpectedly imported"
        );
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.contains("Edge.Cuts gr_poly") && stderr.contains(expected_error),
            "expected polygon outline error containing {expected_error:?} for {file_name}, got:\n{stderr}"
        );
    }
}

#[test]
fn import_kicad_pcb_component_clearance_check_uses_imported_layout() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let enriched_project = dir.path().join("usb_connector_checks.project.yaml");
    import_kicad_pcb(
        "examples/import_kicad_usb_connector_protection_suggestions/board.kicad_pcb",
        "examples/import_kicad_usb_connector_protection_suggestions/project_checks.yaml",
        &enriched_project,
    );

    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&enriched_project, &validator);

    let report_dir = dir.path().join("usb_connector_checks.report");
    let validate_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            enriched_project.to_str().unwrap(),
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
    assert_eq!(report["summary"]["critical"], 2);
    let failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "USB_CONNECTOR_COMPONENT_CLEARANCE_VALID")
        .expect("USB connector component-clearance finding");
    assert_eq!(failure["component"], "J1");
    assert_eq!(failure["measured"]["nearby_component"], "UESD");
    assert_eq!(
        failure["measured"]["connector_clearance_reference"],
        "footprint_polygon"
    );
    assert_eq!(
        failure["measured"]["connector_footprint_graphic_layer"],
        "F.Fab"
    );
    assert_eq!(
        failure["measured"]["connector_footprint_graphic_kind"],
        "fabrication"
    );
    assert_eq!(
        failure["measured"]["nearby_component_clearance_reference"],
        "footprint_rectangle"
    );
    assert_eq!(
        failure["measured"]["nearby_component_footprint_graphic_layer"],
        "F.Fab"
    );
    assert_eq!(
        failure["measured"]["nearby_component_footprint_graphic_kind"],
        "fabrication"
    );
    let clearance = failure["measured"]["connector_to_component_clearance_mm"]
        .as_f64()
        .unwrap();
    assert!((clearance - 0.5).abs() < 1e-12);
    assert_eq!(
        failure["limit"]["min_connector_to_component_clearance_mm"],
        0.7
    );
    let entry_failure = report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "USB_CONNECTOR_ENTRY_CLEARANCE_VALID")
        .expect("USB connector entry-clearance finding");
    assert_eq!(entry_failure["component"], "J1");
    assert_eq!(entry_failure["measured"]["obstructing_component"], "UESD");
    assert_eq!(entry_failure["measured"]["entry_direction_deg"], 0.0);
    assert_eq!(
        entry_failure["measured"]["entry_direction_source"],
        "footprint_property_offset"
    );
    assert_eq!(entry_failure["measured"]["entry_direction_offset_deg"], 0.0);
    assert_eq!(
        entry_failure["measured"]["entry_aperture_source"],
        "footprint_property_aperture"
    );
    assert_eq!(entry_failure["measured"]["entry_aperture_width_mm"], 1.0);
    assert_eq!(
        entry_failure["measured"]["aperture_min_effective_clearance_width_mm"],
        1.0
    );
    assert_eq!(
        entry_failure["measured"]["effective_cable_entry_clearance_width_mm"],
        1.0
    );
    assert_eq!(
        entry_failure["measured"]["obstruction_reference"],
        "footprint_rectangle"
    );
    assert_eq!(
        entry_failure["measured"]["obstruction_footprint_graphic_kind"],
        "fabrication"
    );
    assert_eq!(
        entry_failure["limit"]["min_cable_entry_clearance_depth_mm"],
        0.8
    );
    assert_eq!(
        entry_failure["limit"]["cable_entry_clearance_width_mm"],
        1.0
    );

    let pcb_text = std::fs::read_to_string(
        "examples/import_kicad_usb_connector_protection_suggestions/board.kicad_pcb",
    )
    .unwrap();
    let property_offset = "(property \"CircuitCI_EntryDirectionOffsetDeg\" \"0.0\"";
    assert_eq!(pcb_text.matches(property_offset).count(), 1);
    let nonzero_property_pcb = dir
        .path()
        .join("board_with_nonzero_entry_direction_property.kicad_pcb");
    std::fs::write(
        &nonzero_property_pcb,
        pcb_text.replace(
            property_offset,
            "(property \"CircuitCI_EntryDirectionOffsetDeg\" \"10.0\"",
        ),
    )
    .unwrap();
    let property_direction_project = dir
        .path()
        .join("usb_connector_property_direction_checks.yaml");
    import_kicad_pcb(
        nonzero_property_pcb.to_str().unwrap(),
        "examples/import_kicad_usb_connector_protection_suggestions/project_checks.yaml",
        &property_direction_project,
    );
    assert_yaml_file_valid(&property_direction_project, &validator);

    let property_direction_report_dir = dir.path().join("property_direction_checks.report");
    let property_direction_validate_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            property_direction_project.to_str().unwrap(),
            "--profile",
            "iot_basic_v0",
            "--output",
            property_direction_report_dir.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(property_direction_validate_status.success());
    let property_direction_report: Value = serde_json::from_str(
        &std::fs::read_to_string(property_direction_report_dir.join("report.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(property_direction_report["result"], "fail");
    let property_direction_entry_failure = property_direction_report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "USB_CONNECTOR_ENTRY_CLEARANCE_VALID")
        .expect("USB connector entry-clearance finding with nonzero footprint-property direction");
    assert_eq!(
        property_direction_entry_failure["measured"]["entry_direction_source"],
        "footprint_property_offset"
    );
    assert_eq!(
        property_direction_entry_failure["measured"]["entry_direction_offset_deg"],
        10.0
    );
    assert_eq!(
        property_direction_entry_failure["measured"]["entry_direction_deg"],
        10.0
    );
    assert_eq!(
        property_direction_entry_failure["measured"]["obstructing_component"],
        "UVBUS"
    );
    assert_report_schema_valid(&property_direction_report);

    let stripped_pcb = dir
        .path()
        .join("board_without_entry_direction_or_aperture.kicad_pcb");
    let stripped_pcb_text = pcb_text
        .lines()
        .filter(|line| {
            !line.contains("CircuitCI_EntryAperture") && !line.contains("CircuitCI_EntryDirection")
        })
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&stripped_pcb, stripped_pcb_text).unwrap();
    let mapping_aperture_project = dir
        .path()
        .join("usb_connector_mapping_aperture_checks.yaml");
    import_kicad_pcb(
        stripped_pcb.to_str().unwrap(),
        "examples/import_kicad_usb_connector_protection_suggestions/project_checks.yaml",
        &mapping_aperture_project,
    );
    assert_yaml_file_valid(&mapping_aperture_project, &validator);

    let mapping_aperture_report_dir = dir.path().join("mapping_aperture_checks.report");
    let mapping_validate_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            mapping_aperture_project.to_str().unwrap(),
            "--profile",
            "iot_basic_v0",
            "--output",
            mapping_aperture_report_dir.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(mapping_validate_status.success());
    let mapping_report: Value = serde_json::from_str(
        &std::fs::read_to_string(mapping_aperture_report_dir.join("report.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(mapping_report["result"], "fail");
    let mapping_entry_failure = mapping_report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "USB_CONNECTOR_ENTRY_CLEARANCE_VALID")
        .expect("USB connector entry-clearance finding with mapping aperture");
    assert_eq!(
        mapping_entry_failure["measured"]["entry_direction_source"],
        "kicad_mapping_offset"
    );
    assert_eq!(
        mapping_entry_failure["measured"]["entry_direction_offset_deg"],
        0.0
    );
    assert_eq!(
        mapping_entry_failure["measured"]["entry_aperture_source"],
        "kicad_mapping_aperture"
    );
    assert_eq!(
        mapping_entry_failure["measured"]["entry_aperture_width_mm"],
        1.2
    );
    assert_eq!(
        mapping_entry_failure["measured"]["aperture_min_effective_clearance_width_mm"],
        1.2
    );
    assert_eq!(
        mapping_entry_failure["measured"]["effective_cable_entry_clearance_width_mm"],
        1.2
    );
    assert_eq!(
        mapping_entry_failure["limit"]["cable_entry_clearance_width_mm"],
        1.0
    );
    assert_report_schema_valid(&mapping_report);

    let mapping_project_text = std::fs::read_to_string(
        "examples/import_kicad_usb_connector_protection_suggestions/project_checks.yaml",
    )
    .unwrap();
    assert_eq!(mapping_project_text.matches("offset_deg: 0.0").count(), 1);
    let nonzero_mapping_project_checks = dir
        .path()
        .join("project_checks_mapping_entry_direction_offset.yaml");
    std::fs::write(
        &nonzero_mapping_project_checks,
        mapping_project_text.replace("offset_deg: 0.0", "offset_deg: 10.0"),
    )
    .unwrap();
    let mapping_direction_project = dir
        .path()
        .join("usb_connector_mapping_direction_checks.yaml");
    import_kicad_pcb(
        stripped_pcb.to_str().unwrap(),
        nonzero_mapping_project_checks.to_str().unwrap(),
        &mapping_direction_project,
    );
    assert_yaml_file_valid(&mapping_direction_project, &validator);

    let mapping_direction_report_dir = dir.path().join("mapping_direction_checks.report");
    let mapping_direction_validate_status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "validate",
            mapping_direction_project.to_str().unwrap(),
            "--profile",
            "iot_basic_v0",
            "--output",
            mapping_direction_report_dir.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(mapping_direction_validate_status.success());
    let mapping_direction_report: Value = serde_json::from_str(
        &std::fs::read_to_string(mapping_direction_report_dir.join("report.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(mapping_direction_report["result"], "fail");
    let mapping_direction_entry_failure = mapping_direction_report["failures"]
        .as_array()
        .unwrap()
        .iter()
        .find(|failure| failure["id"] == "USB_CONNECTOR_ENTRY_CLEARANCE_VALID")
        .expect("USB connector entry-clearance finding with nonzero mapping direction");
    assert_eq!(
        mapping_direction_entry_failure["measured"]["entry_direction_source"],
        "kicad_mapping_offset"
    );
    assert_eq!(
        mapping_direction_entry_failure["measured"]["entry_direction_offset_deg"],
        10.0
    );
    assert_eq!(
        mapping_direction_entry_failure["measured"]["entry_direction_deg"],
        10.0
    );
    assert_eq!(
        mapping_direction_entry_failure["measured"]["entry_aperture_source"],
        "kicad_mapping_aperture"
    );
    assert_eq!(
        mapping_direction_entry_failure["measured"]["obstructing_component"],
        "UVBUS"
    );
    assert_report_schema_valid(&mapping_direction_report);
    assert_report_schema_valid(&report);
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
        imported["board"]["layout"]["zones"]["gnd"][0]["island_id"],
        "F_Cu_GND_zone_0"
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
    assert_eq!(
        imported["board"]["layout"]["constraints"]["usb_connector"]["max_connector_to_protection_distance_mm"],
        2.0
    );
    assert_eq!(
        imported["board"]["layout"]["constraints"]["usb_connector"]["max_connector_rotation_error_deg"],
        181.0
    );
    assert_eq!(
        imported["board"]["layout"]["constraints"]["usb_route"]["max_data_line_via_count"],
        1
    );
    assert_eq!(
        imported["board"]["layout"]["constraints"]["usb_route"]["max_data_pair_gap_delta_mm"],
        0.12
    );
    assert_eq!(
        imported["board"]["layout"]["constraints"]["usb_return_path"]["max_data_line_unreferenced_length_mm"],
        0.0
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
    assert_eq!(suggestions["suggestions"].as_array().unwrap().len(), 14);
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
            .any(|suggestion| suggestion["id"] == "usb_connector_entry_clearance_j1")
    );
    assert!(
        suggestions["suggestions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|suggestion| suggestion["id"] == "usb_return_path_j1")
    );
    let placement = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_protection_placement_j1")
        .unwrap();
    assert_eq!(placement["runnable"], true);
    assert!(placement.get("required_inputs").is_none());
    assert_eq!(
        placement["scenario"]["parameters"]["max_connector_to_protection_distance_mm"],
        2.0
    );
    let orientation = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_connector_orientation_j1")
        .unwrap();
    assert_eq!(orientation["runnable"], true);
    assert!(orientation.get("required_inputs").is_none());
    assert_eq!(
        orientation["scenario"]["parameters"]["expected_connector_rotation_deg"],
        180.0
    );
    assert_eq!(
        orientation["scenario"]["parameters"]["max_connector_rotation_error_deg"],
        181.0
    );
    let edge_proximity = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_connector_edge_proximity_j1")
        .unwrap();
    assert_eq!(edge_proximity["runnable"], true);
    assert!(edge_proximity.get("required_inputs").is_none());
    assert_eq!(
        edge_proximity["scenario"]["parameters"]["max_connector_to_board_edge_distance_mm"],
        0.1
    );
    let body_overhang = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_connector_body_overhang_j1")
        .unwrap();
    assert_eq!(body_overhang["runnable"], true);
    assert!(body_overhang.get("required_inputs").is_none());
    assert_eq!(
        body_overhang["scenario"]["parameters"]["max_connector_body_overhang_mm"],
        0.5
    );
    let component_clearance = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_connector_component_clearance_j1")
        .unwrap();
    assert_eq!(component_clearance["runnable"], true);
    assert!(component_clearance.get("required_inputs").is_none());
    assert_eq!(
        component_clearance["scenario"]["parameters"]["min_connector_to_component_clearance_mm"],
        0.5
    );
    let route = suggestions["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|suggestion| suggestion["id"] == "usb_route_geometry_j1")
        .unwrap();
    assert_eq!(route["runnable"], true);
    assert!(route.get("required_inputs").is_none());
    assert_eq!(
        route["scenario"]["parameters"]["max_data_line_route_length_mm"],
        25.0
    );
    assert_eq!(
        route["scenario"]["parameters"]["max_data_pair_length_mismatch_mm"],
        0.5
    );
    assert_eq!(
        route["scenario"]["parameters"]["max_data_line_via_count"],
        1
    );
    assert_eq!(
        route["scenario"]["parameters"]["max_data_line_width_delta_mm"],
        0.02
    );
    assert_eq!(
        route["scenario"]["parameters"]["max_connector_to_protection_route_distance_mm"],
        2.0
    );
    assert_eq!(
        route["scenario"]["parameters"]["max_component_to_route_distance_mm"],
        0.05
    );
    assert_eq!(
        route["scenario"]["parameters"]["max_data_pair_via_count_delta"],
        1
    );
    assert_eq!(
        route["scenario"]["parameters"]["max_data_pair_gap_delta_mm"],
        0.12
    );
    assert_eq!(
        route["scenario"]["parameters"]["require_route_pad_contact_evidence"],
        true
    );
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
    assert_eq!(vbus_route["runnable"], true);
    assert!(vbus_route.get("required_inputs").is_none());
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
        serde_json::Value::Null
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
    assert_eq!(return_path["runnable"], true);
    assert!(return_path.get("required_inputs").is_none());
    assert_eq!(
        return_path["scenario"]["checks"][0],
        "USB_RETURN_PATH_VALID"
    );
    assert_eq!(
        return_path["scenario"]["parameters"]["max_data_line_unreferenced_length_mm"],
        0.0
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
fn import_kicad_pcb_sampled_edge_segments_feed_usb_layout_checks() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    assert_sampled_usb_board_edge_fixture(
        dir.path(),
        "circle",
        "examples/import_kicad_usb_curved_board_edge_suggestions/board_circle.kicad_pcb",
        32,
        SampledEdgeExpectation {
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
            body_overhang_limit_mm: None,
        },
    );
    assert_sampled_usb_board_edge_fixture(
        dir.path(),
        "arc",
        "examples/import_kicad_usb_curved_board_edge_suggestions/board_arc.kicad_pcb",
        16,
        SampledEdgeExpectation {
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
            body_overhang_limit_mm: None,
        },
    );
    assert_sampled_usb_board_edge_fixture(
        dir.path(),
        "rect",
        "examples/import_kicad_usb_curved_board_edge_suggestions/board_rect.kicad_pcb",
        4,
        SampledEdgeExpectation {
            start_x_mm: 0.5,
            start_y_mm: -0.5,
            end_x_mm: 0.5,
            end_y_mm: 0.5,
            outward_normal_deg: 0.0,
            body_overhang_mm: 0.06,
            source_primitive: "gr_rect",
            source_primitive_index: 0,
            sample_index: 1,
            sample_count: 4,
            body_overhang_limit_mm: Some(0.05),
        },
    );
    assert_sampled_usb_board_edge_fixture(
        dir.path(),
        "poly",
        "examples/import_kicad_usb_curved_board_edge_suggestions/board_poly.kicad_pcb",
        4,
        SampledEdgeExpectation {
            start_x_mm: 0.5,
            start_y_mm: -0.5,
            end_x_mm: 0.5,
            end_y_mm: 0.5,
            outward_normal_deg: 0.0,
            body_overhang_mm: 0.06,
            source_primitive: "gr_poly",
            source_primitive_index: 0,
            sample_index: 1,
            sample_count: 4,
            body_overhang_limit_mm: Some(0.05),
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

struct SampledEdgeExpectation {
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
    body_overhang_limit_mm: Option<f64>,
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

fn assert_sampled_usb_board_edge_fixture(
    dir: &Path,
    name: &str,
    board_path: &str,
    expected_outline_segments: usize,
    expected: SampledEdgeExpectation,
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
    assert_suggestion_uses_sampled_edge(&suggestions, "usb_connector_orientation_j1", &expected);
    assert_suggestion_uses_sampled_edge(&suggestions, "usb_connector_edge_proximity_j1", &expected);
    assert_suggestion_uses_sampled_edge(&suggestions, "usb_connector_body_overhang_j1", &expected);

    let check_project = dir.join(format!("{name}.checks.project.yaml"));
    let check_source = if let Some(limit_mm) = expected.body_overhang_limit_mm {
        let checks_text = std::fs::read_to_string(
            "examples/import_kicad_usb_curved_board_edge_suggestions/project_checks.yaml",
        )
        .unwrap();
        let tuned_checks = dir.join(format!("{name}.project_checks.yaml"));
        std::fs::write(
            &tuned_checks,
            checks_text.replace(
                "max_connector_body_overhang_mm: 0.2",
                &format!("max_connector_body_overhang_mm: {limit_mm}"),
            ),
        )
        .unwrap();
        tuned_checks
    } else {
        Path::new("examples/import_kicad_usb_curved_board_edge_suggestions/project_checks.yaml")
            .to_path_buf()
    };
    import_kicad_pcb(board_path, check_source.to_str().unwrap(), &check_project);
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
    if let Some(limit_mm) = expected.body_overhang_limit_mm {
        assert_eq!(report["result"], "fail");
        assert_eq!(report["summary"]["critical"], 1);
        let failure = report["failures"]
            .as_array()
            .unwrap()
            .iter()
            .find(|failure| failure["id"] == "USB_CONNECTOR_BODY_OVERHANG_VALID")
            .expect("USB body-overhang finding");
        assert_close(
            failure["measured"]["connector_body_overhang_mm"]
                .as_f64()
                .unwrap(),
            expected.body_overhang_mm,
        );
        assert_eq!(
            failure["measured"]["board_edge_source_primitive"],
            expected.source_primitive
        );
        assert_eq!(
            failure["measured"]["board_edge_source_primitive_index"],
            expected.source_primitive_index
        );
        assert_eq!(
            failure["measured"]["board_edge_sample_index"],
            expected.sample_index
        );
        assert_eq!(
            failure["measured"]["board_edge_sample_count"],
            expected.sample_count
        );
        assert_close(
            failure["measured"]["outward_normal_deg"].as_f64().unwrap(),
            expected.outward_normal_deg,
        );
        assert_eq!(failure["limit"]["max_connector_body_overhang_mm"], limit_mm);
        assert_report_schema_valid(&report);
    } else {
        assert_eq!(report["result"], "pass");
        assert_eq!(report["summary"]["critical"], 0);
    }
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

fn assert_suggestion_uses_sampled_edge(
    suggestions: &Value,
    suggestion_id: &str,
    expected: &SampledEdgeExpectation,
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
