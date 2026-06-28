mod common;

use common::{assert_yaml_file_valid, read_suggestion_report};
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
        connector_footprint["semantics"]["body_bounds"]["source"],
        "kicad_footprint_graphics"
    );
    assert_eq!(
        connector_footprint["semantics"]["body_bounds"]["min"]["x_mm"],
        -0.8
    );
    assert_eq!(
        connector_footprint["semantics"]["body_bounds"]["min"]["y_mm"],
        -0.9
    );
    assert_eq!(
        connector_footprint["semantics"]["body_bounds"]["max"]["x_mm"],
        0.4
    );
    assert_eq!(
        connector_footprint["semantics"]["body_bounds"]["max"]["y_mm"],
        1.2
    );
    assert_eq!(
        connector_footprint["semantics"]["courtyard_bounds"]["source"],
        "kicad_footprint_graphics"
    );
    assert_eq!(
        connector_footprint["semantics"]["courtyard_bounds"]["min"]["x_mm"],
        -0.8
    );
    assert_eq!(
        connector_footprint["semantics"]["courtyard_bounds"]["min"]["y_mm"],
        -0.9
    );
    assert_eq!(
        connector_footprint["semantics"]["courtyard_bounds"]["max"]["x_mm"],
        0.4
    );
    assert_eq!(
        connector_footprint["semantics"]["courtyard_bounds"]["max"]["y_mm"],
        1.0
    );
    assert!(connector_footprint["semantics"]["pin_1"].is_null());
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
    let suggestions = read_suggestion_report(&suggestions_path);
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
    let mapped_aperture_suggestions = read_suggestion_report(&mapped_aperture_suggestions_path);
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
fn import_kicad_pcb_preserves_pin_1_semantic_marker() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let project_path = dir.path().join("pin1.project.yaml");
    let pcb_path = dir.path().join("pin1.kicad_pcb");
    let output_path = dir.path().join("pin1.with_layout.project.yaml");
    let library_path = std::env::current_dir()
        .unwrap()
        .join("libs/generic")
        .display()
        .to_string();
    std::fs::write(
        &project_path,
        format!(
            r#"project:
  name: kicad_pin1_fixture
  version: 0.1.0
libraries:
  - {library_path}
board:
  components:
    U1:
      model: generic.schematic.imported_component
      pins: {{}}
  nets:
    NET_1:
      kind: digital_or_analog
scenarios: []
"#
        ),
    )
    .unwrap();
    std::fs::write(
        &pcb_path,
        r#"(kicad_pcb
  (version 20240108)
  (generator circuitci-test)
  (net 1 "NET_1")
  (footprint "Package:Pin1Fixture" (layer "F.Cu")
    (at 10 20 90)
    (property "Reference" "U1" (at 0 0 0) (layer "F.SilkS"))
    (fp_rect (start -1 -2) (end 1 2) (stroke (width 0.1) (type solid)) (fill none) (layer "F.Fab"))
    (pad "1" smd rect (at 0.5 0.25) (size 0.3 0.3) (layers "F.Cu" "F.Paste" "F.Mask") (net 1 "NET_1")
      (solder_mask_margin 0.04)
      (solder_paste_margin -0.02)
      (solder_paste_margin_ratio -0.1)
      (clearance 0.15)
      (zone_connect 2)
      (thermal_bridge_width 0.25)
      (thermal_gap 0.2))
  )
)
"#,
    )
    .unwrap();

    import_kicad_pcb(
        pcb_path.to_str().unwrap(),
        project_path.to_str().unwrap(),
        &output_path,
    );

    let schema: Value =
        serde_json::from_str(include_str!("../schemas/board_ir.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert_yaml_file_valid(&output_path, &validator);
    let imported: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&output_path).unwrap()).unwrap();
    let semantics = &imported["board"]["layout"]["footprints"]["U1"]["semantics"];
    assert_eq!(semantics["pin_1"]["source"], "kicad_pad_1");
    assert_eq!(semantics["pin_1"]["at"]["x_mm"], 9.75);
    assert_eq!(semantics["pin_1"]["at"]["y_mm"], 20.5);
    assert_eq!(
        semantics["body_bounds"]["source"],
        "kicad_footprint_graphics"
    );
    assert_eq!(semantics["body_bounds"]["min"]["x_mm"], 8.0);
    assert_eq!(semantics["body_bounds"]["min"]["y_mm"], 19.0);
    assert_eq!(semantics["body_bounds"]["max"]["x_mm"], 12.0);
    assert_eq!(semantics["body_bounds"]["max"]["y_mm"], 21.0);
    let pad_fabrication = &imported["board"]["layout"]["pads"]["U1"]["1"]["fabrication"];
    assert_eq!(pad_fabrication["source"], "kicad_pad_property");
    assert_eq!(pad_fabrication["solder_mask_margin_mm"], 0.04);
    assert_eq!(pad_fabrication["solder_paste_margin_mm"], -0.02);
    assert_eq!(pad_fabrication["solder_paste_margin_ratio"], -0.1);
    assert_eq!(pad_fabrication["clearance_mm"], 0.15);
    assert_eq!(pad_fabrication["zone_connect"], 2);
    assert_eq!(pad_fabrication["thermal_bridge_width_mm"], 0.25);
    assert_eq!(pad_fabrication["thermal_gap_mm"], 0.2);
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
