use std::process::Command;

#[test]
fn inspect_easyeda_pro_reports_structure_and_encoded_payloads() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let eprj2 = dir.path().join("fixture.eprj2");
    let output = dir.path().join("easyeda_report.md");
    let manifest_output = output.with_extension("json");
    let structure = r#"{"boards":{"board1":{"uuid":"board1","title":"Board A","pcb_uuid":"pcb1"}},"schematics":{"sch1":{"uuid":"sch1","name":"Main Schematic","sheet_uuid":"sheet1"}},"sheets":{"sheet1":{"uuid":"sheet1","title":"Power"}},"pcbs":{"pcb1":{"uuid":"pcb1","title":"PCB A"}}}"#;
    let sql = format!(
        "CREATE TABLE projects (uuid varchar, name varchar, branch_uuid varchar, ticket integer);
         CREATE TABLE branches (id integer, uuid varchar, name varchar, history_uuid varchar);
         CREATE TABLE project_structures (id integer, ticket integer, structure text);
         CREATE TABLE history_data (id integer, dataStr text);
         INSERT INTO projects VALUES ('project1', 'Demo Project', 'branch1', 7);
         INSERT INTO branches VALUES (1, 'branch1', 'main', 'history1');
         INSERT INTO project_structures VALUES (1, 42, '{}');
         INSERT INTO history_data VALUES (1, 'uQuXeaEWVPvQkrqXBaOCA==');
         INSERT INTO history_data VALUES (2, '{{\"plain\":true}}');",
        structure.replace('\'', "''")
    );
    let sqlite_output = Command::new("sqlite3")
        .arg(&eprj2)
        .arg(sql)
        .output()
        .unwrap();
    assert!(
        sqlite_output.status.success(),
        "{}",
        String::from_utf8_lossy(&sqlite_output.stderr)
    );

    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "inspect-easyeda-pro",
            eprj2.to_str().unwrap(),
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
    let stdout = String::from_utf8_lossy(&command_output.stdout);
    assert!(stdout.contains("1 projects"));
    assert!(stdout.contains("1 branches"));
    assert!(stdout.contains("1 structures"));
    assert!(stdout.contains("latest ticket 42"));
    assert!(stdout.contains("1 boards"));
    assert!(stdout.contains("1 schematics"));
    assert!(stdout.contains("1 sheets"));
    assert!(stdout.contains("1 PCBs"));
    assert!(stdout.contains("4 structure objects"));
    assert!(stdout.contains("1 encoded history payloads"));
    assert!(stdout.contains("manifest"));

    let report = std::fs::read_to_string(output).unwrap();
    assert!(report.contains("Demo Project"));
    assert!(report.contains("Board A"));
    assert!(report.contains("Main Schematic"));
    assert!(report.contains("PCB A"));
    assert!(report.contains("Object Evidence"));
    assert!(report.contains("pcb_uuid=pcb1"));
    assert!(report.contains("encoded/non-JSON"));
    assert!(report.contains("pad, via, route, zone, and net geometry as unavailable"));

    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(manifest_output).unwrap()).unwrap();
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../schemas/easyeda_pro_inspection.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    if let Err(error) = validator.validate(&manifest) {
        panic!("EasyEDA Pro manifest failed schema validation: {error}");
    }
    assert_eq!(manifest["schema_version"], "0.2.0");
    assert_eq!(manifest["source"]["sha256"].as_str().unwrap().len(), 64);
    assert_eq!(manifest["sqlite"]["tables"].as_array().unwrap().len(), 4);
    assert!(
        manifest["sqlite"]["tables"]
            .as_array()
            .unwrap()
            .iter()
            .any(|table| table["name"] == "history_data"
                && table["row_count"] == 2
                && table["columns"].as_array().unwrap().len() == 2)
    );
    assert_eq!(manifest["easyeda_pro"]["latest_structure"]["ticket"], 42);
    assert_eq!(
        manifest["easyeda_pro"]["latest_structure"]["boards"][0]["title"],
        "Board A"
    );
    let structure_objects = manifest["easyeda_pro"]["latest_structure"]["objects"]
        .as_array()
        .unwrap();
    assert_eq!(structure_objects.len(), 4);
    let board_object = structure_objects
        .iter()
        .find(|object| object["kind"] == "board" && object["uuid"] == "board1")
        .expect("board structure object evidence");
    assert_eq!(board_object["map_key"], "board1");
    assert_eq!(board_object["title"], "Board A");
    assert_eq!(board_object["sha256"].as_str().unwrap().len(), 64);
    assert!(
        board_object["field_names"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field == "pcb_uuid")
    );
    assert_eq!(board_object["references"][0]["field"], "pcb_uuid");
    assert_eq!(board_object["references"][0]["value"], "pcb1");
    assert_eq!(
        manifest["easyeda_pro"]["history_payloads"]["encoded_or_non_json"],
        1
    );
    let payloads = manifest["easyeda_pro"]["history_payloads"]["rows"]
        .as_array()
        .unwrap();
    assert_eq!(payloads.len(), 2);
    assert_eq!(payloads[0]["id"], 1);
    assert_eq!(payloads[0]["looks_like_json"], false);
    assert_eq!(payloads[0]["sha256"].as_str().unwrap().len(), 64);
    assert_eq!(payloads[1]["looks_like_json"], true);
    assert_eq!(
        manifest["importability"]["status"],
        "blocked_encoded_history_payloads"
    );
}

#[test]
fn inspect_easyeda_pro_rejects_non_sqlite_input() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let eprj2 = dir.path().join("not_sqlite.eprj2");
    let output = dir.path().join("easyeda_report.md");
    std::fs::write(&eprj2, "not sqlite").unwrap();

    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "inspect-easyeda-pro",
            eprj2.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(!command_output.status.success());
    let stderr = String::from_utf8_lossy(&command_output.stderr);
    assert!(stderr.contains("not a SQLite 3 database"));
}
