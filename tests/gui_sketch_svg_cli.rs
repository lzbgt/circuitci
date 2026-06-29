#![cfg(feature = "gui")]

use std::process::Command;

#[test]
fn export_sketch_svg_writes_headless_visual_artifact() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("sketch.svg");
    let command_output = Command::new(env!("CARGO_BIN_EXE_circuitci"))
        .args([
            "export-sketch-svg",
            "examples/ne555_astable_scope_smoke/project.yaml",
            "--output",
            output.to_str().unwrap(),
            "--width",
            "960",
            "--height",
            "540",
        ])
        .output()
        .unwrap();

    assert!(
        command_output.status.success(),
        "export-sketch-svg failed: stdout={} stderr={}",
        String::from_utf8_lossy(&command_output.stdout),
        String::from_utf8_lossy(&command_output.stderr)
    );
    let svg = std::fs::read_to_string(output).unwrap();
    assert!(svg.starts_with("<svg "));
    assert!(svg.contains("CircuitCI Sketch - ne555_astable_scope"));
    assert!(svg.contains(r#"role="img""#));
    assert!(svg.contains(r#"data-kind="component" data-id="RTIM""#));
    assert!(svg.contains(r#"data-net="out""#));
    assert!(svg.contains(r#"data-pin-label="RTIM:A""#));
    assert!(svg.contains(r#"<circle "#));
}
