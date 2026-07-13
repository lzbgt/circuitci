use super::{
    SpiceImportOptions, import_spice_with_progress, import_spice_with_progress_and_cancel,
    logical_lines, parse_spice_deck, parse_spice_number,
};

#[test]
fn spice_suffix_numbers_parse() {
    assert_eq!(parse_spice_number("10k"), Some(10_000.0));
    assert!((parse_spice_number("100n").unwrap() - 100e-9).abs() < 1e-18);
    assert!((parse_spice_number("1u").unwrap() - 1e-6).abs() < 1e-18);
    assert!((parse_spice_number("1u").unwrap() - 1e-6).abs() < 1e-18);
    assert_eq!(parse_spice_number("2meg"), Some(2e6));
    assert_eq!(parse_spice_number("3.3"), Some(3.3));
}

#[test]
fn continuation_lines_are_joined() {
    let lines = logical_lines("R1 a b\n+ 10k\n").unwrap();
    assert_eq!(lines, vec!["R1 a b 10k"]);
}

#[test]
fn malformed_continuation_fails_closed() {
    assert!(logical_lines("+ orphan\n").is_err());
}

#[test]
fn parser_discovers_basic_elements() {
    let dir = tempfile::tempdir().unwrap();
    let deck = dir.path().join("rc.cir");
    std::fs::write(
        &deck,
        "* test
V1 in 0 DC 3.3
R1 in out 10k
C1 out 0 100n
.tran 1u 10u
.end
",
    )
    .unwrap();
    let parsed = parse_spice_deck(&deck).unwrap();
    assert_eq!(parsed.elements.len(), 3);
    assert!(parsed.nodes.contains("in"));
    assert!(parsed.nodes.contains("out"));
    assert!(parsed.nodes.contains("0"));
    assert_eq!(parsed.tran.unwrap().stop_time_us, 10.0);
}

#[test]
fn parser_uses_transient_timing_from_control_block() {
    let dir = tempfile::tempdir().unwrap();
    let deck = dir.path().join("control.cir");
    std::fs::write(
        &deck,
        "* control block deck
V1 in 0 DC 3.3
R1 in 0 1k
.control
tran 2u 20u
meas tran avg_in AVG V(in) FROM=0 TO=20u
run
write waveform.raw
.endc
.end
",
    )
    .unwrap();
    let parsed = parse_spice_deck(&deck).unwrap();
    assert_eq!(parsed.elements.len(), 2);
    let tran = parsed.tran.as_ref().unwrap();
    assert_eq!(tran.max_step_us, 2.0);
    assert_eq!(tran.stop_time_us, 20.0);
    assert_eq!(parsed.measures.len(), 1);
    assert_eq!(parsed.measures[0].mode, "tran");
    assert_eq!(parsed.measures[0].name, "avg_in");
    assert_eq!(
        parsed.measures[0].statement,
        "meas tran avg_in AVG V(in) FROM=0 TO=20u"
    );
}

#[test]
fn parser_rejects_unclosed_control_block() {
    let dir = tempfile::tempdir().unwrap();
    let deck = dir.path().join("control.cir");
    std::fs::write(
        &deck,
        "V1 in 0 DC 3.3
.control
tran 1u 10u
",
    )
    .unwrap();
    let error = parse_spice_deck(&deck).unwrap_err();
    assert!(error.to_string().contains("missing a closing .endc"));
}

#[test]
fn import_spice_with_progress_emits_phases() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("imported.project.yaml");
    let mut stages = Vec::new();

    import_spice_with_progress(
        &SpiceImportOptions {
            input: "examples/import_spice_rc/deck.cir".into(),
            output,
            name: "progress_spice".to_string(),
            backend: "auto".to_string(),
            stop_time_us: 1000.0,
            max_step_us: 1.0,
        },
        |stage, _detail| stages.push(stage.to_string()),
    )
    .unwrap();

    for expected in [
        "Parsing SPICE deck",
        "Preparing output",
        "Building Board IR",
        "Serializing Board IR",
    ] {
        assert!(stages.iter().any(|stage| stage == expected), "{expected}");
    }
}

#[test]
fn import_spice_cancellation_stops_before_write() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("canceled.project.yaml");
    let error = import_spice_with_progress_and_cancel(
        &SpiceImportOptions {
            input: "examples/import_spice_rc/deck.cir".into(),
            output: output.clone(),
            name: "canceled_spice".to_string(),
            backend: "auto".to_string(),
            stop_time_us: 1000.0,
            max_step_us: 1.0,
        },
        |_, _| {},
        || true,
    )
    .unwrap_err();

    assert!(error.to_string().contains("canceled"));
    assert!(!output.exists());
}
