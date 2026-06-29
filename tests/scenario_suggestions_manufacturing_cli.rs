use serde_json::Value;
use std::process::Command;

#[test]
fn suggest_scenarios_derives_controlled_impedance_templates() {
    let suggestions =
        run_suggest_scenarios("examples/scenario_suggestions_controlled_impedance/project.yaml");
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_controlled_impedance"
    );
    let suggested = suggestions["suggestions"].as_array().unwrap();
    assert_eq!(suggested.len(), 8);

    let single_ended = &suggested[0];
    assert_eq!(single_ended["id"], "controlled_impedance_rf");
    assert_eq!(
        single_ended["kind"],
        "manufacturing_controlled_impedance_rf"
    );
    assert_eq!(single_ended["runnable"], true);
    assert_eq!(single_ended["scenario"]["type"], "manufacturing");
    assert_eq!(
        single_ended["scenario"]["checks"][0],
        "CONTROLLED_IMPEDANCE_GEOMETRY_VALID"
    );
    let net = &single_ended["scenario"]["parameters"]["nets"][0];
    assert_eq!(net["net"], "RF");
    assert_eq!(net["source"], "fab_stackup_table_rev_a");
    assert_eq!(net["target_impedance_ohm"], 50.0);
    assert_eq!(net["expected_width_mm"], 0.20);
    assert_eq!(net["max_width_error_mm"], 0.03);
    assert!(single_ended.get("required_inputs").is_none());
    assert!(
        single_ended["reason"]
            .as_str()
            .unwrap()
            .contains("reviewed board.manufacturing.controlled_impedance")
    );

    let differential = &suggested[1];
    assert_eq!(differential["id"], "controlled_impedance_dp_dm");
    assert_eq!(
        differential["kind"],
        "manufacturing_controlled_impedance_dp_dm"
    );
    assert_eq!(differential["runnable"], true);
    assert_eq!(differential["scenario"]["type"], "manufacturing");
    assert_eq!(
        differential["scenario"]["checks"][0],
        "CONTROLLED_IMPEDANCE_GEOMETRY_VALID"
    );
    let pair = &differential["scenario"]["parameters"]["differential_pairs"][0];
    assert_eq!(pair["first_net"], "DP");
    assert_eq!(pair["second_net"], "DM");
    assert_eq!(pair["source"], "fab_stackup_table_rev_a");
    assert_eq!(pair["target_differential_impedance_ohm"], 90.0);
    assert_eq!(pair["expected_width_mm"], 0.15);
    assert_eq!(pair["expected_gap_mm"], 0.20);
    assert_eq!(pair["max_width_error_mm"], 0.02);
    assert_eq!(pair["max_gap_error_mm"], 0.03);
    assert!(differential.get("required_inputs").is_none());
    assert!(
        differential["reason"]
            .as_str()
            .unwrap()
            .contains("parallel same-layer gap evidence")
    );

    let single_stackup = &suggested[2];
    assert_eq!(single_stackup["id"], "controlled_impedance_stackup_rf");
    assert_eq!(
        single_stackup["kind"],
        "manufacturing_controlled_impedance_stackup_rf"
    );
    assert_eq!(single_stackup["runnable"], true);
    assert_eq!(single_stackup["scenario"]["type"], "manufacturing");
    assert_eq!(
        single_stackup["scenario"]["checks"][0],
        "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID"
    );
    let route = &single_stackup["scenario"]["parameters"]["routes"][0];
    assert_eq!(route["net"], "RF");
    assert_eq!(route["route_layer"], "F.Cu");
    assert_eq!(route["reference_layer"], "In1.GND");
    assert_eq!(route["dielectric_layer"], "prepreg_1");
    assert!(
        single_stackup["reason"]
            .as_str()
            .unwrap()
            .contains("explicit stackup copper/dielectric metadata")
    );

    let differential_stackup = &suggested[3];
    assert_eq!(
        differential_stackup["id"],
        "controlled_impedance_stackup_dp_dm"
    );
    assert_eq!(
        differential_stackup["kind"],
        "manufacturing_controlled_impedance_stackup_dp_dm"
    );
    assert_eq!(differential_stackup["runnable"], true);
    assert_eq!(
        differential_stackup["scenario"]["checks"][0],
        "CONTROLLED_IMPEDANCE_STACKUP_EVIDENCE_VALID"
    );
    let routes = differential_stackup["scenario"]["parameters"]["routes"]
        .as_array()
        .unwrap();
    assert_eq!(routes.len(), 2);
    assert_eq!(routes[0]["net"], "DP");
    assert_eq!(routes[1]["net"], "DM");
    for route in routes {
        assert_eq!(route["route_layer"], "F.Cu");
        assert_eq!(route["reference_layer"], "In1.GND");
        assert_eq!(route["dielectric_layer"], "prepreg_1");
    }

    let single_mask = &suggested[4];
    assert_eq!(single_mask["id"], "controlled_impedance_solder_mask_rf");
    assert_eq!(
        single_mask["kind"],
        "manufacturing_controlled_impedance_solder_mask_rf"
    );
    assert_eq!(single_mask["runnable"], true);
    assert_eq!(
        single_mask["scenario"]["checks"][0],
        "CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID"
    );
    let mask_route = &single_mask["scenario"]["parameters"]["routes"][0];
    assert_eq!(mask_route["net"], "RF");
    assert_eq!(mask_route["route_layer"], "F.Cu");
    assert_eq!(mask_route["solder_mask_layer"], "F.Mask");
    assert_eq!(mask_route["expected_solder_mask_state"], "covered");
    assert_eq!(mask_route["source"], "fab_solder_mask_review_rev_a");
    assert!(
        single_mask["reason"]
            .as_str()
            .unwrap()
            .contains("solder-mask loading policy")
    );

    let differential_mask = &suggested[5];
    assert_eq!(
        differential_mask["id"],
        "controlled_impedance_solder_mask_dp_dm"
    );
    assert_eq!(
        differential_mask["kind"],
        "manufacturing_controlled_impedance_solder_mask_dp_dm"
    );
    assert_eq!(differential_mask["runnable"], true);
    assert_eq!(
        differential_mask["scenario"]["checks"][0],
        "CONTROLLED_IMPEDANCE_SOLDER_MASK_LOADING_VALID"
    );
    let mask_routes = differential_mask["scenario"]["parameters"]["routes"]
        .as_array()
        .unwrap();
    assert_eq!(mask_routes.len(), 2);
    assert_eq!(mask_routes[0]["net"], "DP");
    assert_eq!(mask_routes[1]["net"], "DM");
    for route in mask_routes {
        assert_eq!(route["route_layer"], "F.Cu");
        assert_eq!(route["solder_mask_layer"], "F.Mask");
        assert_eq!(route["expected_solder_mask_state"], "covered");
        assert_eq!(route["source"], "fab_solder_mask_review_rev_a");
    }

    let single_coupon = &suggested[6];
    assert_eq!(single_coupon["id"], "controlled_impedance_coupon_rf_coupon");
    assert_eq!(
        single_coupon["kind"],
        "manufacturing_controlled_impedance_coupon_rf_coupon"
    );
    assert_eq!(single_coupon["runnable"], true);
    assert_eq!(
        single_coupon["scenario"]["checks"][0],
        "CONTROLLED_IMPEDANCE_COUPON_VALID"
    );
    assert_eq!(
        single_coupon["scenario"]["parameters"]["coupons"][0]["name"],
        "rf_coupon"
    );
    assert!(
        single_coupon["reason"]
            .as_str()
            .unwrap()
            .contains("reviewed measured impedance evidence")
    );

    let differential_coupon = &suggested[7];
    assert_eq!(
        differential_coupon["id"],
        "controlled_impedance_coupon_dp_dm_coupon"
    );
    assert_eq!(
        differential_coupon["kind"],
        "manufacturing_controlled_impedance_coupon_dp_dm_coupon"
    );
    assert_eq!(differential_coupon["runnable"], true);
    assert_eq!(
        differential_coupon["scenario"]["checks"][0],
        "CONTROLLED_IMPEDANCE_COUPON_VALID"
    );
    assert_eq!(
        differential_coupon["scenario"]["parameters"]["coupons"][0]["name"],
        "dp_dm_coupon"
    );
}

#[test]
fn suggest_scenarios_derives_adjacent_plane_return_path_template() {
    let suggestions = run_suggest_scenarios(
        "examples/scenario_suggestions_adjacent_plane_return_path/project.yaml",
    );
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_adjacent_plane_return_path"
    );
    let suggested = suggestions["suggestions"].as_array().unwrap();
    assert_eq!(suggested.len(), 1);

    let return_path = &suggested[0];
    assert_eq!(return_path["id"], "adjacent_plane_return_path_sig");
    assert_eq!(
        return_path["kind"],
        "manufacturing_adjacent_plane_return_path_sig"
    );
    assert_eq!(return_path["runnable"], true);
    assert_eq!(return_path["scenario"]["type"], "manufacturing");
    assert_eq!(
        return_path["scenario"]["checks"][0],
        "ADJACENT_PLANE_RETURN_PATH_VALID"
    );
    let route = &return_path["scenario"]["parameters"]["routes"][0];
    assert_eq!(route["net"], "SIG");
    assert_eq!(route["reference_net"], "GND");
    assert_eq!(route["reference_layer"], "In1.Cu");
    assert_eq!(route["max_unreferenced_length_mm"], 0.0);
    assert!(return_path.get("required_inputs").is_none());
    assert!(
        return_path["reason"]
            .as_str()
            .unwrap()
            .contains("sampled adjacent GND plane-zone coverage")
    );
}

#[test]
fn suggest_scenarios_derives_reference_plane_slot_crossing_template() {
    let suggestions = run_suggest_scenarios(
        "examples/scenario_suggestions_reference_plane_slot_crossing/project.yaml",
    );
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_reference_plane_slot_crossing"
    );
    let suggested = suggestions["suggestions"].as_array().unwrap();
    assert_eq!(suggested.len(), 1);

    let slot_crossing = &suggested[0];
    assert_eq!(slot_crossing["id"], "reference_plane_slot_crossing_sig");
    assert_eq!(
        slot_crossing["kind"],
        "manufacturing_reference_plane_slot_crossing_sig"
    );
    assert_eq!(slot_crossing["runnable"], true);
    assert_eq!(slot_crossing["scenario"]["type"], "manufacturing");
    assert_eq!(
        slot_crossing["scenario"]["checks"][0],
        "REFERENCE_PLANE_SLOT_CROSSING_VALID"
    );
    let route = &slot_crossing["scenario"]["parameters"]["routes"][0];
    assert_eq!(route["net"], "SIG");
    assert_eq!(route["reference_net"], "GND");
    assert_eq!(route["reference_layer"], "In1.Cu");
    assert_eq!(route["max_slot_crossings"], 0);
    assert!(slot_crossing.get("required_inputs").is_none());
    assert!(
        slot_crossing["reason"]
            .as_str()
            .unwrap()
            .contains("1 internal reference-plane gap")
    );
}

#[test]
fn suggest_scenarios_derives_return_path_stitching_via_template() {
    let suggestions = run_suggest_scenarios(
        "examples/scenario_suggestions_return_path_stitching_via/project.yaml",
    );
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_return_path_stitching_via"
    );
    let suggested = suggestions["suggestions"].as_array().unwrap();
    assert_eq!(suggested.len(), 1);

    let stitching = &suggested[0];
    assert_eq!(stitching["id"], "return_path_stitching_via_sig");
    assert_eq!(
        stitching["kind"],
        "manufacturing_return_path_stitching_via_sig"
    );
    assert_eq!(stitching["runnable"], true);
    assert_eq!(stitching["scenario"]["type"], "manufacturing");
    assert_eq!(
        stitching["scenario"]["checks"][0],
        "RETURN_PATH_STITCHING_VIA_VALID"
    );
    let route = &stitching["scenario"]["parameters"]["routes"][0];
    assert_eq!(route["net"], "SIG");
    assert_eq!(route["reference_net"], "GND");
    assert_eq!(route["max_stitch_via_distance_mm"], 1.0);
    assert!(stitching.get("required_inputs").is_none());
    assert!(
        stitching["reason"]
            .as_str()
            .unwrap()
            .contains("reviewed board.manufacturing.max_stitch_via_distance_mm")
    );
}

#[test]
fn suggest_scenarios_derives_rf_antenna_keepout_template() {
    let suggestions =
        run_suggest_scenarios("examples/scenario_suggestions_rf_antenna_keepout/project.yaml");
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_rf_antenna_keepout"
    );
    let suggested = suggestions["suggestions"].as_array().unwrap();
    assert!(!suggested.is_empty());

    let keepout = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "rf_antenna_keepout_chip_antenna_clearance")
        .expect("RF antenna keepout suggestion");
    assert_eq!(keepout["id"], "rf_antenna_keepout_chip_antenna_clearance");
    assert_eq!(
        keepout["kind"],
        "manufacturing_rf_antenna_keepout_chip_antenna_clearance"
    );
    assert_eq!(keepout["runnable"], true);
    assert_eq!(keepout["scenario"]["type"], "manufacturing");
    assert_eq!(keepout["scenario"]["checks"][0], "RF_ANTENNA_KEEPOUT_VALID");
    assert_eq!(
        keepout["scenario"]["parameters"]["keepouts"][0]["name"],
        "chip_antenna_clearance"
    );
    assert!(keepout.get("required_inputs").is_none());
    assert!(
        keepout["reason"]
            .as_str()
            .unwrap()
            .contains("reviewed polygon/source metadata")
    );

    let feed_path = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "rf_antenna_feed_path_chip_antenna_feed")
        .expect("RF antenna feed path suggestion");
    assert_eq!(feed_path["id"], "rf_antenna_feed_path_chip_antenna_feed");
    assert_eq!(
        feed_path["kind"],
        "manufacturing_rf_antenna_feed_path_chip_antenna_feed"
    );
    assert_eq!(feed_path["runnable"], true);
    assert_eq!(feed_path["scenario"]["type"], "manufacturing");
    assert_eq!(
        feed_path["scenario"]["checks"][0],
        "RF_ANTENNA_FEED_PATH_VALID"
    );
    assert_eq!(
        feed_path["scenario"]["parameters"]["feed_paths"][0]["name"],
        "chip_antenna_feed"
    );
    assert!(feed_path.get("required_inputs").is_none());
    assert!(
        feed_path["reason"]
            .as_str()
            .unwrap()
            .contains("imported route, pad, placement")
    );

    let matching = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "rf_antenna_matching_topology_chip_antenna_pi_match")
        .expect("RF antenna matching topology suggestion");
    assert_eq!(
        matching["kind"],
        "manufacturing_rf_antenna_matching_topology_chip_antenna_pi_match"
    );
    assert_eq!(matching["runnable"], true);
    assert_eq!(matching["scenario"]["type"], "manufacturing");
    assert_eq!(
        matching["scenario"]["checks"][0],
        "RF_ANTENNA_MATCHING_TOPOLOGY_VALID"
    );
    assert_eq!(
        matching["scenario"]["parameters"]["matching_networks"][0]["name"],
        "chip_antenna_pi_match"
    );
    assert!(matching.get("required_inputs").is_none());
    assert!(
        matching["reason"]
            .as_str()
            .unwrap()
            .contains("reviewed topology metadata")
    );

    let measurement = suggested
        .iter()
        .find(|suggestion| {
            suggestion["id"] == "rf_antenna_measured_performance_chip_antenna_s11_2440"
        })
        .expect("RF antenna measured-performance suggestion");
    assert_eq!(
        measurement["kind"],
        "manufacturing_rf_antenna_measured_performance_chip_antenna_s11_2440"
    );
    assert_eq!(measurement["runnable"], false);
    assert_eq!(measurement["scenario"]["type"], "manufacturing");
    assert_eq!(
        measurement["scenario"]["checks"][0],
        "RF_ANTENNA_MEASURED_PERFORMANCE_VALID"
    );
    assert_eq!(
        measurement["scenario"]["parameters"]["rf_measurements"][0]["name"],
        "chip_antenna_s11_2440"
    );
    assert!(
        measurement["required_inputs"][0]
            .as_str()
            .unwrap()
            .contains("parameters.min_return_loss_db")
    );
    assert!(
        measurement["reason"]
            .as_str()
            .unwrap()
            .contains("frequency, and return-loss evidence")
    );
}

#[test]
fn suggest_scenarios_derives_thermal_copper_area_template() {
    let suggestions =
        run_suggest_scenarios("examples/scenario_suggestions_thermal_copper_area/project.yaml");
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_thermal_copper_area"
    );
    let suggested = suggestions["suggestions"].as_array().unwrap();

    let thermal = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "thermal_copper_area_u1_switch_heat_spreader")
        .expect("thermal copper suggestion");
    assert_eq!(thermal["runnable"], true);
    assert_eq!(
        thermal["scenario"]["checks"][0],
        "THERMAL_COPPER_AREA_VALID"
    );
    assert_eq!(
        thermal["scenario"]["parameters"]["thermal_copper"][0]["name"],
        "u1_switch_heat_spreader"
    );
    assert!(thermal.get("required_inputs").is_none());
    assert!(
        thermal["reason"]
            .as_str()
            .unwrap()
            .contains("reviewed power-loss/source metadata")
    );

    let thermal_vias = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "thermal_via_stackup_u1_switch_heat_spreader")
        .expect("thermal via stackup suggestion");
    assert_eq!(thermal_vias["runnable"], true);
    assert_eq!(
        thermal_vias["scenario"]["checks"][0],
        "THERMAL_VIA_STACKUP_VALID"
    );
    assert_eq!(
        thermal_vias["scenario"]["parameters"]["thermal_copper"][0]["name"],
        "u1_switch_heat_spreader"
    );
    assert!(thermal_vias.get("required_inputs").is_none());
    assert!(
        thermal_vias["reason"]
            .as_str()
            .unwrap()
            .contains("reviewed via-count/copper-thickness policy")
    );
}

#[test]
fn suggest_scenarios_derives_thermal_via_plating_template() {
    let suggestions =
        run_suggest_scenarios("examples/scenario_suggestions_thermal_via_plating/project.yaml");
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_thermal_via_plating"
    );
    let suggested = suggestions["suggestions"].as_array().unwrap();

    let thermal_via_plating = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "thermal_via_plating_u1_heat_spreader")
        .expect("thermal via plating suggestion");
    assert_eq!(thermal_via_plating["runnable"], true);
    assert_eq!(
        thermal_via_plating["scenario"]["checks"][0],
        "THERMAL_VIA_PLATING_VALID"
    );
    assert_eq!(
        thermal_via_plating["scenario"]["parameters"]["thermal_copper"][0]["name"],
        "u1_heat_spreader"
    );
    assert!(thermal_via_plating.get("required_inputs").is_none());
    assert!(
        thermal_via_plating["reason"]
            .as_str()
            .unwrap()
            .contains("reviewed plated-via/drill policy")
    );

    let barrel = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "thermal_via_barrel_cross_section_u1_heat_spreader")
        .expect("thermal via barrel cross-section suggestion");
    assert_eq!(barrel["runnable"], true);
    assert_eq!(
        barrel["scenario"]["checks"][0],
        "THERMAL_VIA_BARREL_CROSS_SECTION_VALID"
    );
    assert_eq!(
        barrel["scenario"]["parameters"]["thermal_copper"][0]["name"],
        "u1_heat_spreader"
    );
    assert!(barrel.get("required_inputs").is_none());
    assert!(
        barrel["reason"]
            .as_str()
            .unwrap()
            .contains("reviewed via-barrel cross-section policy")
    );
}

#[test]
fn suggest_scenarios_derives_thermal_package_temperature_template() {
    let suggestions = run_suggest_scenarios(
        "examples/scenario_suggestions_thermal_package_temperature/project.yaml",
    );
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_thermal_package_temperature"
    );
    let suggested = suggestions["suggestions"].as_array().unwrap();

    let thermal_package = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "thermal_package_temperature_u1_regulator_loss")
        .expect("thermal package temperature suggestion");
    assert_eq!(thermal_package["runnable"], false);
    assert_eq!(
        thermal_package["scenario"]["checks"][0],
        "THERMAL_PACKAGE_TEMPERATURE_VALID"
    );
    assert_eq!(
        thermal_package["scenario"]["parameters"]["thermal_copper"][0]["name"],
        "u1_regulator_loss"
    );
    let required = thermal_package["required_inputs"].as_array().unwrap();
    assert!(required.iter().any(|item| {
        item.as_str()
            .unwrap()
            .contains("parameters.ambient_temperature_C")
    }));
    assert!(required.iter().any(|item| {
        item.as_str()
            .unwrap()
            .contains("parameters.max_temperature_rise_C")
    }));
}

#[test]
fn suggest_scenarios_derives_thermal_measured_temperature_template() {
    let suggestions = run_suggest_scenarios(
        "examples/scenario_suggestions_thermal_measured_temperature/project.yaml",
    );
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_thermal_measured_temperature"
    );
    let suggested = suggestions["suggestions"].as_array().unwrap();

    let measured_temperature = suggested
        .iter()
        .find(|suggestion| {
            suggestion["id"] == "thermal_measured_temperature_u1_hotspot_steady_state"
        })
        .expect("thermal measured temperature suggestion");
    assert_eq!(measured_temperature["runnable"], false);
    assert_eq!(
        measured_temperature["scenario"]["checks"][0],
        "THERMAL_MEASURED_TEMPERATURE_VALID"
    );
    assert_eq!(
        measured_temperature["scenario"]["parameters"]["thermal_measurements"][0]["name"],
        "u1_hotspot_steady_state"
    );
    let required = measured_temperature["required_inputs"].as_array().unwrap();
    assert!(required.iter().any(|item| {
        item.as_str()
            .unwrap()
            .contains("parameters.max_measured_temperature_C")
    }));
    assert!(required.iter().any(|item| {
        item.as_str()
            .unwrap()
            .contains("parameters.max_temperature_rise_C")
    }));
    assert!(required.iter().any(|item| {
        item.as_str()
            .unwrap()
            .contains("parameters.include_measurement_uncertainty")
    }));
}

#[test]
fn suggest_scenarios_derives_thermal_derating_environment_template() {
    let suggestions = run_suggest_scenarios(
        "examples/scenario_suggestions_thermal_derating_environment/project.yaml",
    );
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_thermal_derating_environment"
    );
    let suggested = suggestions["suggestions"].as_array().unwrap();

    let derating = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "thermal_derating_environment_u1_heat_spreader")
        .expect("thermal derating environment suggestion");
    assert_eq!(derating["runnable"], false);
    assert_eq!(
        derating["scenario"]["checks"][0],
        "THERMAL_DERATING_ENVIRONMENT_VALID"
    );
    assert_eq!(
        derating["scenario"]["parameters"]["thermal_copper"][0]["name"],
        "u1_heat_spreader"
    );
    let required = derating["required_inputs"].as_array().unwrap();
    assert!(required.iter().any(|item| {
        item.as_str()
            .unwrap()
            .contains("parameters.ambient_temperature_C")
    }));
    assert!(
        required
            .iter()
            .any(|item| { item.as_str().unwrap().contains("parameters.airflow_lfm") })
    );
    assert!(required.iter().any(|item| {
        item.as_str()
            .unwrap()
            .contains("parameters.enclosure_profile")
    }));
}

#[test]
fn suggest_scenarios_derives_manufacturing_artifact_templates() {
    let suggestions =
        run_suggest_scenarios("examples/scenario_suggestions_manufacturing_artifacts/project.yaml");
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_manufacturing_artifacts"
    );
    let suggested = suggestions["suggestions"].as_array().unwrap();
    assert_eq!(suggested.len(), 16);

    let drill_diameter = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "drill_diameter_valid")
        .expect("drill diameter suggestion");
    assert_eq!(drill_diameter["kind"], "manufacturing_drill_diameter");
    assert_eq!(drill_diameter["runnable"], true);
    assert_eq!(drill_diameter["scenario"]["type"], "manufacturing");
    assert_eq!(
        drill_diameter["scenario"]["checks"][0],
        "DRILL_DIAMETER_VALID"
    );
    assert_eq!(
        drill_diameter["scenario"]["parameters"]["fabrication_process"],
        "jlcpcb_drill_diameter_range_2026_06"
    );

    let drill_edge = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "drill_to_board_edge_clearance")
        .expect("drill edge suggestion");
    assert_eq!(drill_edge["runnable"], true);
    assert_eq!(
        drill_edge["scenario"]["checks"][0],
        "DRILL_TO_BOARD_EDGE_CLEARANCE_VALID"
    );
    assert_eq!(
        drill_edge["scenario"]["parameters"]["min_drill_edge_clearance_mm"],
        0.50
    );
    assert!(drill_edge.get("required_inputs").is_none());

    let slot_width = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "slot_width_valid")
        .expect("slot width suggestion");
    assert_eq!(slot_width["runnable"], true);
    assert_eq!(slot_width["scenario"]["checks"][0], "SLOT_WIDTH_VALID");
    assert_eq!(
        slot_width["scenario"]["parameters"]["fabrication_process"],
        "jlcpcb_slot_min_2026_06"
    );

    let slot_aspect_ratio = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "slot_aspect_ratio_valid")
        .expect("slot aspect ratio suggestion");
    assert_eq!(slot_aspect_ratio["runnable"], true);
    assert_eq!(
        slot_aspect_ratio["scenario"]["checks"][0],
        "SLOT_ASPECT_RATIO_VALID"
    );
    assert_eq!(
        slot_aspect_ratio["scenario"]["parameters"]["fabrication_process"],
        "jlcpcb_slot_min_2026_06"
    );

    let slot_edge = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "slot_to_board_edge_clearance")
        .expect("slot edge suggestion");
    assert_eq!(slot_edge["runnable"], true);
    assert_eq!(
        slot_edge["scenario"]["checks"][0],
        "SLOT_TO_BOARD_EDGE_CLEARANCE_VALID"
    );
    assert_eq!(
        slot_edge["scenario"]["parameters"]["min_slot_edge_clearance_mm"],
        0.50
    );
    assert!(slot_edge.get("required_inputs").is_none());

    let castellated_hole = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "castellated_hole_valid")
        .expect("castellated hole suggestion");
    assert_eq!(castellated_hole["runnable"], true);
    assert_eq!(
        castellated_hole["scenario"]["checks"][0],
        "CASTELLATED_HOLE_VALID"
    );
    assert_eq!(
        castellated_hole["scenario"]["parameters"]["fabrication_process"],
        "jlcpcb_castellated_hole_2026_06"
    );

    let annular_ring = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "drill_annular_ring_valid")
        .expect("annular ring suggestion");
    assert_eq!(annular_ring["runnable"], true);
    assert_eq!(
        annular_ring["scenario"]["checks"][0],
        "DRILL_ANNULAR_RING_VALID"
    );
    assert_eq!(
        annular_ring["scenario"]["parameters"]["fabrication_process"],
        "jlcpcb_double_sided_via_min_2026_06"
    );

    let copper_edge = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "copper_to_board_edge_clearance")
        .expect("copper edge suggestion");
    assert_eq!(copper_edge["runnable"], true);
    assert_eq!(
        copper_edge["scenario"]["checks"][0],
        "COPPER_TO_BOARD_EDGE_CLEARANCE_VALID"
    );
    assert_eq!(
        copper_edge["scenario"]["parameters"]["fabrication_process"],
        "jlcpcb_routed_edge_copper_clearance_2026_06"
    );

    let mask_opening = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "solder_mask_opening_valid")
        .expect("mask opening suggestion");
    assert_eq!(mask_opening["runnable"], true);
    assert_eq!(
        mask_opening["scenario"]["checks"][0],
        "SOLDER_MASK_OPENING_VALID"
    );
    assert_eq!(
        mask_opening["scenario"]["parameters"]["fabrication_process"],
        "jlcpcb_standard_2026_06"
    );

    let mask_dam = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "solder_mask_dam_valid")
        .expect("mask dam suggestion");
    assert_eq!(mask_dam["runnable"], true);
    assert_eq!(mask_dam["scenario"]["checks"][0], "SOLDER_MASK_DAM_VALID");
    assert_eq!(
        mask_dam["scenario"]["parameters"]["fabrication_process"],
        "jlcpcb_standard_2026_06"
    );

    let copper_spacing = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "copper_spacing_valid")
        .expect("copper spacing suggestion");
    assert_eq!(copper_spacing["runnable"], true);
    assert_eq!(
        copper_spacing["scenario"]["checks"][0],
        "COPPER_SPACING_VALID"
    );
    assert_eq!(
        copper_spacing["scenario"]["parameters"]["fabrication_process"],
        "jlcpcb_1oz_copper_spacing_2026_06"
    );

    let paste_opening = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "solder_paste_opening_valid")
        .expect("paste opening suggestion");
    assert_eq!(paste_opening["runnable"], true);
    assert_eq!(
        paste_opening["scenario"]["checks"][0],
        "SOLDER_PASTE_OPENING_VALID"
    );
    assert_eq!(
        paste_opening["scenario"]["parameters"]["min_paste_area_ratio"],
        0.70
    );
    assert_eq!(
        paste_opening["scenario"]["parameters"]["max_paste_area_ratio"],
        1.00
    );
    assert!(paste_opening.get("required_inputs").is_none());

    let paste_aperture = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "solder_paste_aperture_size_valid")
        .expect("paste aperture suggestion");
    assert_eq!(paste_aperture["runnable"], true);
    assert_eq!(
        paste_aperture["scenario"]["checks"][0],
        "SOLDER_PASTE_APERTURE_SIZE_VALID"
    );
    assert_eq!(
        paste_aperture["scenario"]["parameters"]["fabrication_process"],
        "jlcpcb_stencil_aperture_min_2026_06"
    );

    let paste_area_ratio = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "solder_paste_aperture_area_ratio_valid")
        .expect("paste aperture area ratio suggestion");
    assert_eq!(paste_area_ratio["runnable"], true);
    assert_eq!(
        paste_area_ratio["scenario"]["checks"][0],
        "SOLDER_PASTE_APERTURE_AREA_RATIO_VALID"
    );
    assert_eq!(
        paste_area_ratio["scenario"]["parameters"]["fabrication_process"],
        "jlcpcb_stencil_area_ratio_2026_06"
    );
    assert_eq!(
        paste_area_ratio["scenario"]["parameters"]["stencil_thickness_mm"],
        0.10
    );
    assert!(paste_area_ratio.get("required_inputs").is_none());

    let paste_ic_pin = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "solder_paste_ic_pin_aperture_valid")
        .expect("IC pin paste aperture suggestion");
    assert_eq!(paste_ic_pin["runnable"], true);
    assert_eq!(
        paste_ic_pin["scenario"]["checks"][0],
        "SOLDER_PASTE_IC_PIN_APERTURE_VALID"
    );
    assert_eq!(paste_ic_pin["scenario"]["target"]["component"], "U1");
    assert_eq!(paste_ic_pin["scenario"]["parameters"]["pin_pitch_mm"], 0.5);
    assert!(
        paste_ic_pin["reason"]
            .as_str()
            .unwrap()
            .contains("U1 on F.Paste")
    );

    let paste_spacing = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "solder_paste_spacing_valid")
        .expect("paste spacing suggestion");
    assert_eq!(paste_spacing["runnable"], true);
    assert_eq!(
        paste_spacing["scenario"]["checks"][0],
        "SOLDER_PASTE_SPACING_VALID"
    );
    assert_eq!(
        paste_spacing["scenario"]["parameters"]["min_solder_paste_spacing_mm"],
        0.15
    );
    assert!(paste_spacing.get("required_inputs").is_none());
}

#[test]
fn suggest_scenarios_derives_assembly_footprint_alignment_template() {
    let suggestions = run_suggest_scenarios(
        "examples/scenario_suggestions_assembly_footprint_alignment/project.yaml",
    );
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_assembly_footprint_alignment"
    );
    let suggested = suggestions["suggestions"].as_array().unwrap();
    assert_eq!(suggested.len(), 1);

    let alignment = &suggested[0];
    assert_eq!(alignment["id"], "assembly_footprint_alignment_u1");
    assert_eq!(
        alignment["kind"],
        "manufacturing_assembly_footprint_alignment"
    );
    assert_eq!(alignment["runnable"], true);
    assert_eq!(alignment["scenario"]["type"], "manufacturing");
    assert_eq!(
        alignment["scenario"]["checks"][0],
        "ASSEMBLY_FOOTPRINT_ALIGNMENT_VALID"
    );
    assert_eq!(alignment["scenario"]["target"]["component"], "U1");
    assert_eq!(
        alignment["scenario"]["parameters"]["rotation_tolerance_deg"],
        0.01
    );
    assert!(alignment.get("required_inputs").is_none());
    assert!(
        alignment["reason"]
            .as_str()
            .unwrap()
            .contains("JLC/EasyEDA assembly source evidence")
    );
}

#[test]
fn suggest_scenarios_derives_pin_1_orientation_template() {
    let suggestions =
        run_suggest_scenarios("examples/scenario_suggestions_pin_1_orientation/project.yaml");
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_pin_1_orientation"
    );
    let suggested = suggestions["suggestions"].as_array().unwrap();
    assert_eq!(suggested.len(), 1);

    let pin_1 = &suggested[0];
    assert_eq!(pin_1["id"], "pin_1_orientation_u1");
    assert_eq!(pin_1["kind"], "manufacturing_pin_1_orientation");
    assert_eq!(pin_1["runnable"], false);
    assert_eq!(pin_1["scenario"]["type"], "manufacturing");
    assert_eq!(pin_1["scenario"]["checks"][0], "PIN_1_ORIENTATION_VALID");
    assert_eq!(pin_1["scenario"]["target"]["component"], "U1");
    assert!(pin_1["scenario"]["parameters"]["expected_pin_1_direction_deg"].is_null());
    assert!(pin_1["scenario"]["parameters"]["max_pin_1_direction_error_deg"].is_null());
    assert_eq!(pin_1["required_inputs"].as_array().unwrap().len(), 2);
}

#[test]
fn suggest_scenarios_derives_broad_ic_stencil_pitch_template() {
    let suggestions =
        run_suggest_scenarios("examples/scenario_suggestions_ic_stencil_broad_pitch/project.yaml");
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_ic_stencil_broad_pitch"
    );
    let suggested = suggestions["suggestions"].as_array().unwrap();
    assert_eq!(suggested.len(), 4);

    let paste_ic_pin = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "solder_paste_ic_pin_aperture_valid")
        .expect("IC pin paste aperture suggestion");
    assert_eq!(paste_ic_pin["runnable"], true);
    assert_eq!(
        paste_ic_pin["scenario"]["checks"][0],
        "SOLDER_PASTE_IC_PIN_APERTURE_VALID"
    );
    assert_eq!(paste_ic_pin["scenario"]["target"]["component"], "U1");
    assert_eq!(paste_ic_pin["scenario"]["parameters"]["pin_pitch_mm"], 1.0);
    assert!(
        paste_ic_pin["reason"]
            .as_str()
            .unwrap()
            .contains("3 repeated 1.000 mm")
    );
}

#[test]
fn suggest_scenarios_derives_bga_stencil_pitch_template() {
    let suggestions =
        run_suggest_scenarios("examples/scenario_suggestions_bga_stencil_pitch/project.yaml");
    assert_eq!(
        suggestions["project"],
        "scenario_suggestions_bga_stencil_pitch"
    );
    let suggested = suggestions["suggestions"].as_array().unwrap();
    assert_eq!(suggested.len(), 4);

    let paste_bga = suggested
        .iter()
        .find(|suggestion| suggestion["id"] == "solder_paste_bga_aperture_valid")
        .expect("BGA paste aperture suggestion");
    assert_eq!(paste_bga["runnable"], true);
    assert_eq!(
        paste_bga["scenario"]["checks"][0],
        "SOLDER_PASTE_BGA_APERTURE_VALID"
    );
    assert_eq!(paste_bga["scenario"]["target"]["component"], "U1");
    assert_eq!(paste_bga["scenario"]["parameters"]["pin_pitch_mm"], 0.8);
    assert!(
        paste_bga["reason"]
            .as_str()
            .unwrap()
            .contains("2 horizontal and 2 vertical repeated 0.800 mm")
    );
    assert!(
        suggested
            .iter()
            .all(|suggestion| suggestion["id"] != "solder_paste_ic_pin_aperture_valid")
    );
}

fn run_suggest_scenarios(project: &str) -> Value {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("suggestions.yaml");
    let status = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "suggest-scenarios",
            project,
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    let suggestions: Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(output).unwrap()).unwrap();
    assert_suggestion_schema_valid(&suggestions);
    assert_runnable_suggestions_have_no_required_inputs(&suggestions);
    suggestions
}

fn assert_suggestion_schema_valid(suggestions: &Value) {
    let schema: Value = serde_json::from_str(include_str!(
        "../schemas/scenario_suggestion_report.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors: Vec<String> = validator
        .iter_errors(suggestions)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    assert!(errors.is_empty(), "suggestion schema errors: {errors:#?}");
}

fn assert_runnable_suggestions_have_no_required_inputs(suggestions: &Value) {
    for suggestion in suggestions["suggestions"].as_array().unwrap() {
        if suggestion["runnable"].as_bool().unwrap() {
            assert!(
                suggestion.get("required_inputs").is_none(),
                "runnable suggestion {} has required_inputs",
                suggestion["id"]
            );
        }
    }
}
