use std::process::Command;

#[test]
fn inspect_easyeda_pro_reports_structure_and_encoded_payloads() {
    std::fs::create_dir_all("out").unwrap();
    let dir = tempfile::tempdir_in("out").unwrap();
    let eprj2 = dir.path().join("fixture.eprj2");
    let output = dir.path().join("easyeda_report.md");
    let structure = r#"{"boards":{"board1":{"uuid":"board1","title":"Board A"}},"schematics":{"sch1":{"uuid":"sch1","name":"Main Schematic"}},"sheets":{"sheet1":{"uuid":"sheet1","title":"Power"}},"pcbs":{"pcb1":{"uuid":"pcb1","title":"PCB A"}}}"#;
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
    assert!(stdout.contains("1 encoded history payloads"));

    let report = std::fs::read_to_string(output).unwrap();
    assert!(report.contains("Demo Project"));
    assert!(report.contains("Board A"));
    assert!(report.contains("Main Schematic"));
    assert!(report.contains("PCB A"));
    assert!(report.contains("encoded/non-JSON"));
    assert!(report.contains("pad, via, route, zone, and net geometry as unavailable"));
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
