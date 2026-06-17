mod common;

use common::assert_yaml_file_valid;
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
