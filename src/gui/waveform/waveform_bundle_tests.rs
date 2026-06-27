use super::{
    WaveformTraceRef, cleanup_old_scope_report_bundle_dirs, old_scope_report_bundle_dirs,
    parse_operating_point_csv_text, parse_waveform_csv_text,
    scope_report_bundle_artifact_detail_rows, scope_report_bundle_changed_artifacts,
    scope_report_bundle_index_path, scope_report_bundle_integrity_details,
    scope_report_bundle_integrity_details_csv, scope_report_bundle_integrity_details_markdown,
    scope_report_bundle_integrity_projected_details, scope_report_bundle_missing_artifacts,
    unique_scope_report_bundle_dir,
};
use crate::gui::CircuitCiApp;
use crate::reports::{Finding, ValidationReport};
use serde_json::json;
use std::fs;

#[test]
fn scope_report_bundle_exports_filtered_snapshots_and_plot_svg() {
    let waveform = parse_waveform_csv_text(
        "time,v(out),i(load)\n0,0,0.1\n0.000001,2,0.3\n",
        "waveform.csv",
    )
    .unwrap();
    let base_dir = std::env::temp_dir().join(format!(
        "circuitci_scope_bundle_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&base_dir);
    fs::create_dir_all(&base_dir).unwrap();

    let mut app = CircuitCiApp {
        output_dir: base_dir.to_string_lossy().into_owned(),
        waveforms: vec![waveform],
        selected_probe: 0,
        waveform_cursor_a_us: 0.0,
        waveform_cursor_b_us: 1.0,
        waveform_pinned_traces: vec![WaveformTraceRef {
            waveform_index: 0,
            probe_index: 1,
        }],
        waveform_snapshot_filter: "v(out)".to_string(),
        waveform_split_trace_units: true,
        waveform_plot_export_trigger: false,
        ..Default::default()
    };
    app.capture_scope_cursor_snapshots();
    let visible = app.visible_scope_measurement_snapshot_indexes();
    let filtered: Vec<_> = visible
        .iter()
        .map(|&index| app.waveform_measurement_snapshots[index].clone())
        .collect();
    app.export_scope_report_bundle(&filtered);

    let mut bundles = fs::read_dir(&base_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    bundles.sort();
    assert_eq!(bundles.len(), 1);
    let bundle = &bundles[0];
    let svg = fs::read_to_string(bundle.join("scope_plot.svg")).unwrap();
    let csv = fs::read_to_string(bundle.join("measurement_snapshots.csv")).unwrap();
    let markdown = fs::read_to_string(bundle.join("measurement_snapshots.md")).unwrap();
    let margin_csv = fs::read_to_string(bundle.join("sweep_margin_summaries.csv")).unwrap();
    let margin_markdown = fs::read_to_string(bundle.join("sweep_margin_summaries.md")).unwrap();
    let readme = fs::read_to_string(bundle.join("README.md")).unwrap();
    let manifest = fs::read_to_string(bundle.join("artifact_manifest.csv")).unwrap();
    let integrity_csv = fs::read_to_string(bundle.join("artifact_integrity_details.csv")).unwrap();
    let integrity_markdown =
        fs::read_to_string(bundle.join("artifact_integrity_details.md")).unwrap();
    let index_path = scope_report_bundle_index_path(bundle);
    let index = fs::read_to_string(index_path).unwrap();

    assert!(svg.contains("CircuitCI Scope Plot"));
    assert!(csv.contains("Cursor 1"));
    assert!(csv.contains("v(out)"));
    assert!(!csv.contains("i(load)"));
    assert!(markdown.contains("| Cursor 1 |"));
    assert!(margin_csv.starts_with("scenario,assertion,probe,sweep,corner,inputs,passed"));
    assert!(margin_markdown.contains("| Scenario | Assertion | Probe | Sweep | Corner |"));
    assert!(readme.contains("# CircuitCI Scope Report Bundle"));
    assert!(readme.contains("- Rows: 1"));
    assert!(readme.contains("- Search: v(out)"));
    assert!(readme.contains("- Include trigger markers: no"));
    assert!(readme.contains("- Split units: yes"));
    assert!(readme.contains("- Selected trace: v(out)"));
    assert!(readme.contains("## Loaded Waveform Footprint Summary"));
    assert!(readme.contains("| Total | 1 | 48 | 48 B |"));
    assert!(readme.contains("| Runtime Only | 1 | 48 | 48 B |"));
    assert!(readme.contains("- `index.html`"));
    assert!(readme.contains("- `scope_plot.svg`"));
    assert!(readme.contains("- `sweep_margin_summaries.csv`"));
    assert!(readme.contains("- Rows: 0"));
    assert!(readme.contains("- `artifact_manifest.csv`"));
    assert!(readme.contains("- `artifact_integrity_details.csv`"));
    assert!(readme.contains("- `artifact_integrity_details.md`"));
    assert!(readme.contains("optional"));
    assert!(readme.contains("## Artifact Metadata"));
    assert!(readme.contains("| scope_plot.svg |"));
    assert!(readme.contains("| measurement_snapshots.csv |"));
    assert!(readme.contains("| sweep_margin_summaries.csv |"));
    assert!(readme.contains("SHA-256"));
    assert!(index.contains("<title>CircuitCI Scope Report Bundle</title>"));
    assert!(index.contains("href=\"scope_plot.svg\""));
    assert!(index.contains("href=\"measurement_snapshots.csv\""));
    assert!(index.contains("href=\"measurement_snapshots.md\""));
    assert!(index.contains("href=\"sweep_margin_summaries.csv\""));
    assert!(index.contains("href=\"sweep_margin_summaries.md\""));
    assert!(index.contains("href=\"README.md\""));
    assert!(index.contains("href=\"artifact_manifest.csv\""));
    assert!(index.contains("href=\"artifact_integrity_details.csv\""));
    assert!(index.contains("href=\"artifact_integrity_details.md\""));
    assert!(index.contains("<h2>Artifact Metadata</h2>"));
    assert!(index.contains("<h2>Loaded Waveform Footprint Summary</h2>"));
    assert!(index.contains("<td>Total</td><td class=\"number\">1</td>"));
    assert!(index.contains("<td>Runtime Only</td><td class=\"number\">1</td>"));
    assert!(manifest.contains("path,label,size_bytes,sha256"));
    assert!(manifest.contains("scope_plot.svg,scope_plot.svg"));
    assert!(manifest.contains("sweep_margin_summaries.csv,sweep_margin_summaries.csv"));
    assert!(manifest.contains("sweep_margin_summaries.md,sweep_margin_summaries.md"));
    assert!(manifest.contains("index.html,index.html"));
    assert!(manifest.contains("README.md,README.md"));
    assert!(!manifest.contains("artifact_integrity_details.csv"));
    assert!(!manifest.contains("artifact_integrity_details.md"));
    assert!(integrity_csv.starts_with(
        "artifact,state,expected_size_bytes,current_size_bytes,expected_sha256,current_sha256,path\n"
    ));
    assert!(integrity_csv.contains("scope_plot.svg,OK,"));
    assert!(integrity_markdown.contains(
        "| Artifact | State | Expected size bytes | Current size bytes | Expected SHA-256 | Current SHA-256 | Path |"
    ));
    assert!(integrity_markdown.contains("| scope_plot.svg | OK |"));
    assert!(scope_report_bundle_missing_artifacts(bundle).is_empty());
    assert!(
        scope_report_bundle_changed_artifacts(bundle)
            .unwrap()
            .is_empty()
    );
    fs::remove_file(bundle.join("measurement_snapshots.md")).unwrap();
    assert_eq!(
        scope_report_bundle_missing_artifacts(bundle),
        vec!["measurement_snapshots.md"]
    );
    assert_eq!(
        app.waveform_recent_report_bundles,
        vec![bundle.to_string_lossy().into_owned()]
    );
    assert!(app.status.contains("Exported scope report bundle"));

    fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn scope_report_bundle_exports_dc_operating_points_without_waveforms() {
    let operating_point = parse_operating_point_csv_text(
        "vin,midpoint\n5.0,2.375\n",
        "out/analog/divider_dc_bias/divider_tolerance_corner_007/operating_point.csv",
    )
    .unwrap();
    let base_dir = std::env::temp_dir().join(format!(
        "circuitci_scope_dc_bundle_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&base_dir);
    fs::create_dir_all(&base_dir).unwrap();

    let mut app = CircuitCiApp {
        output_dir: base_dir.to_string_lossy().into_owned(),
        operating_points: vec![operating_point],
        report: Some(report(vec![dc_sweep_margin(
            "divider_dc_bias",
            "midpoint_low",
            "midpoint",
            "divider_tolerance",
            "corner_007",
        )])),
        ..Default::default()
    };

    app.export_scope_report_bundle(&[]);

    let bundle = std::path::Path::new(&app.waveform_recent_report_bundles[0]);
    let svg = fs::read_to_string(bundle.join("scope_plot.svg")).unwrap();
    let snapshot_csv = fs::read_to_string(bundle.join("measurement_snapshots.csv")).unwrap();
    let op_csv = fs::read_to_string(bundle.join("operating_points.csv")).unwrap();
    let op_markdown = fs::read_to_string(bundle.join("operating_points.md")).unwrap();
    let readme = fs::read_to_string(bundle.join("README.md")).unwrap();
    let index = fs::read_to_string(bundle.join("index.html")).unwrap();
    let manifest = fs::read_to_string(bundle.join("artifact_manifest.csv")).unwrap();

    assert!(svg.contains("CircuitCI DC Operating-Point Bundle"));
    assert!(snapshot_csv.starts_with("label,note,source,trace"));
    assert!(op_csv.starts_with("scenario,sweep,corner,probe,value,worst,artifact\n"));
    assert!(op_csv.contains("divider_dc_bias,divider_tolerance,corner_007,midpoint"));
    assert!(op_csv.contains(",limiting,"));
    assert!(op_markdown.contains("| divider_dc_bias | divider_tolerance | corner_007 |"));
    assert!(op_markdown.contains("| midpoint |"));
    assert!(op_markdown.contains("| limiting |"));
    assert!(readme.contains("## DC Operating Points"));
    assert!(readme.contains("- Artifacts: 1"));
    assert!(readme.contains("- Values: 2"));
    assert!(readme.contains("- Rows: 0"));
    assert!(index.contains("<h2>DC Operating Points</h2>"));
    assert!(index.contains("href=\"operating_points.csv\""));
    assert!(index.contains("<td>midpoint</td>"));
    assert!(index.contains("<td>limiting</td>"));
    assert!(manifest.contains("operating_points.csv,operating_points.csv"));
    assert!(manifest.contains("operating_points.md,operating_points.md"));
    assert!(scope_report_bundle_missing_artifacts(bundle).is_empty());
    assert!(
        scope_report_bundle_changed_artifacts(bundle)
            .unwrap()
            .is_empty()
    );
    assert!(app.status.contains("Exported scope report bundle"));

    fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn scope_compare_report_bundle_exports_selected_and_pinned_traces() {
    let nominal = parse_waveform_csv_text(
        "time,v(filtered)\n0,0\n0.000001,0.5\n",
        "rc_nominal/waveform.csv",
    )
    .unwrap();
    let worst = parse_waveform_csv_text(
        "time,v(filtered)\n0,0\n0.000001,0.44\n",
        "rc_worst/waveform.csv",
    )
    .unwrap();
    let base_dir = std::env::temp_dir().join(format!(
        "circuitci_scope_compare_bundle_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&base_dir);
    fs::create_dir_all(&base_dir).unwrap();

    let mut app = CircuitCiApp {
        output_dir: base_dir.to_string_lossy().into_owned(),
        waveforms: vec![nominal, worst],
        selected_waveform: 0,
        selected_probe: 0,
        waveform_cursor_a_us: 0.0,
        waveform_cursor_b_us: 1.0,
        waveform_pinned_traces: vec![WaveformTraceRef {
            waveform_index: 1,
            probe_index: 0,
        }],
        report: Some(report(vec![sweep_margin(
            "rc_run",
            "v_filtered_rms",
            "v_filtered",
            "rc_tolerance",
            "corner_009",
        )])),
        ..Default::default()
    };

    assert_eq!(app.export_scope_compare_report_bundle(false), 2);
    assert_eq!(app.waveform_recent_report_bundles.len(), 1);

    let bundle = std::path::Path::new(&app.waveform_recent_report_bundles[0]);
    let csv = fs::read_to_string(bundle.join("measurement_snapshots.csv")).unwrap();
    let markdown = fs::read_to_string(bundle.join("measurement_snapshots.md")).unwrap();
    let margin_csv = fs::read_to_string(bundle.join("sweep_margin_summaries.csv")).unwrap();
    let margin_markdown = fs::read_to_string(bundle.join("sweep_margin_summaries.md")).unwrap();
    let svg = fs::read_to_string(bundle.join("scope_plot.svg")).unwrap();
    let readme = fs::read_to_string(bundle.join("README.md")).unwrap();
    let index = fs::read_to_string(bundle.join("index.html")).unwrap();

    assert!(csv.contains("Compare Set"));
    assert!(csv.contains("compare selected"));
    assert!(csv.contains("compare pinned"));
    assert!(csv.contains("Runtime compare bundle row"));
    assert!(csv.contains("v(filtered)"));
    assert!(markdown.contains("| Compare Set |"));
    assert!(margin_csv.contains("rc_run,v_filtered_rms,v_filtered,rc_tolerance,corner_009"));
    assert!(margin_csv.contains("R1.value_ohm=1050"));
    assert!(margin_csv.contains("0.01 V"));
    assert!(margin_markdown.contains("| rc_run | v_filtered_rms | v_filtered |"));
    assert!(margin_markdown.contains("R1.value_ohm=1050"));
    assert!(readme.contains("## Sweep Margin Summaries"));
    assert!(readme.contains("- Rows: 1"));
    assert!(index.contains("<h2>Sweep Margin Summaries</h2>"));
    assert!(index.contains("rc_tolerance"));
    assert!(svg.contains("CircuitCI Scope Plot"));
    assert!(app.status.contains("Exported scope compare report bundle"));

    fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn scope_compare_report_bundle_requires_multiple_traces() {
    let waveform =
        parse_waveform_csv_text("time,v(out)\n0,0\n0.000001,1\n", "waveform.csv").unwrap();
    let base_dir = std::env::temp_dir().join(format!(
        "circuitci_scope_compare_bundle_guard_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&base_dir);
    fs::create_dir_all(&base_dir).unwrap();

    let mut app = CircuitCiApp {
        output_dir: base_dir.to_string_lossy().into_owned(),
        waveforms: vec![waveform],
        selected_probe: 0,
        ..Default::default()
    };

    assert_eq!(app.export_scope_compare_report_bundle(false), 0);
    assert!(app.waveform_recent_report_bundles.is_empty());
    assert!(app.status.contains("pin at least two visible scope traces"));

    fs::remove_dir_all(&base_dir).unwrap();
}

fn sweep_margin(
    scenario: &str,
    assertion: &str,
    probe: &str,
    sweep: &str,
    corner: &str,
) -> Finding {
    let mut finding = Finding::info("ANALOG_SWEEP_MARGIN_SUMMARY", scenario, "summary");
    finding
        .measured
        .insert("assertion".to_string(), json!(assertion));
    finding.measured.insert("probe".to_string(), json!(probe));
    finding
        .measured
        .insert("analog_sweep".to_string(), json!(sweep));
    finding
        .measured
        .insert("analog_corner".to_string(), json!(corner));
    finding.measured.insert(
        "analog_component_values".to_string(),
        json!({ "R1.value_ohm": 1050.0 }),
    );
    finding
        .measured
        .insert("measured_value".to_string(), json!(0.61));
    finding
        .measured
        .insert("measured_unit".to_string(), json!("V"));
    finding.measured.insert("margin".to_string(), json!(0.01));
    finding.measured.insert("passed".to_string(), json!(true));
    finding
        .measured
        .insert("evaluated_corners".to_string(), json!(9));
    finding.limit.insert("relation".to_string(), json!("below"));
    finding.limit.insert("limit_value".to_string(), json!(0.62));
    finding.limit.insert("limit_unit".to_string(), json!("V"));
    finding
}

fn dc_sweep_margin(
    scenario: &str,
    assertion: &str,
    probe: &str,
    sweep: &str,
    corner: &str,
) -> Finding {
    let mut finding = sweep_margin(scenario, assertion, probe, sweep, corner);
    finding
        .measured
        .insert("quantity".to_string(), json!("operating point"));
    finding
}

fn report(infos: Vec<Finding>) -> ValidationReport {
    ValidationReport::from_parts(
        "project".to_string(),
        "profile".to_string(),
        infos,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        "validate".to_string(),
    )
}

#[test]
fn scope_report_bundle_refresh_recreates_missing_artifacts() {
    let waveform =
        parse_waveform_csv_text("time,v(out)\n0,0\n0.000001,2\n", "waveform.csv").unwrap();
    let base_dir = std::env::temp_dir().join(format!(
        "circuitci_scope_bundle_refresh_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&base_dir);
    fs::create_dir_all(&base_dir).unwrap();

    let mut app = CircuitCiApp {
        output_dir: base_dir.to_string_lossy().into_owned(),
        waveforms: vec![waveform],
        selected_probe: 0,
        waveform_cursor_a_us: 0.0,
        waveform_cursor_b_us: 1.0,
        ..Default::default()
    };
    app.capture_scope_cursor_snapshots();
    let visible = app.visible_scope_measurement_snapshot_indexes();
    let filtered: Vec<_> = visible
        .iter()
        .map(|&index| app.waveform_measurement_snapshots[index].clone())
        .collect();
    app.export_scope_report_bundle(&filtered);

    let bundle = app.waveform_recent_report_bundles[0].clone();
    let bundle_path = std::path::Path::new(&bundle);
    fs::remove_file(bundle_path.join("index.html")).unwrap();
    assert_eq!(
        scope_report_bundle_missing_artifacts(bundle_path),
        vec!["index.html"]
    );

    app.preview_scope_report_bundle_refresh(&bundle);
    assert_eq!(app.waveform_bundle_refresh_preview, Some(bundle.clone()));
    app.confirm_scope_report_bundle_refresh(&filtered);

    assert!(scope_report_bundle_missing_artifacts(bundle_path).is_empty());
    assert!(bundle_path.join("index.html").is_file());
    assert_eq!(app.waveform_bundle_refresh_preview, None);
    assert!(app.status.contains("Refreshed scope report bundle"));
    let clean_rows = scope_report_bundle_artifact_detail_rows(bundle_path);
    let clean_plot_row = clean_rows
        .iter()
        .find(|row| row.0 == "scope_plot.svg")
        .unwrap();
    assert_eq!(clean_plot_row.1, "OK");
    assert_eq!(clean_plot_row.2, clean_plot_row.3);
    assert_eq!(clean_plot_row.4, clean_plot_row.5);

    fs::write(bundle_path.join("scope_plot.svg"), "<svg>changed</svg>").unwrap();
    assert_eq!(
        scope_report_bundle_changed_artifacts(bundle_path).unwrap(),
        vec!["scope_plot.svg".to_string()]
    );
    let changed_rows = scope_report_bundle_artifact_detail_rows(bundle_path);
    let changed_plot_row = changed_rows
        .iter()
        .find(|row| row.0 == "scope_plot.svg")
        .unwrap();
    assert_eq!(changed_plot_row.1, "Changed");
    assert_ne!(changed_plot_row.2, changed_plot_row.3);
    assert_ne!(changed_plot_row.4, changed_plot_row.5);
    let changed_details = scope_report_bundle_integrity_details(bundle_path);
    let changed_csv = scope_report_bundle_integrity_details_csv(&changed_details);
    assert!(changed_csv.starts_with(
        "artifact,state,expected_size_bytes,current_size_bytes,expected_sha256,current_sha256,path\n"
    ));
    assert!(changed_csv.contains("scope_plot.svg,Changed,"));
    assert!(changed_csv.contains(changed_plot_row.4.as_deref().unwrap()));
    assert!(changed_csv.contains(changed_plot_row.5.as_deref().unwrap()));
    let changed_markdown = scope_report_bundle_integrity_details_markdown(&changed_details);
    assert!(changed_markdown.contains(
        "| Artifact | State | Expected size bytes | Current size bytes | Expected SHA-256 | Current SHA-256 | Path |"
    ));
    assert!(changed_markdown.contains("| scope_plot.svg | Changed |"));
    assert!(changed_markdown.contains(changed_plot_row.4.as_deref().unwrap()));
    assert!(changed_markdown.contains(changed_plot_row.5.as_deref().unwrap()));
    let problem_details = scope_report_bundle_integrity_projected_details(&changed_details, true);
    assert!(
        problem_details
            .rows
            .iter()
            .any(|row| row.label == "scope_plot.svg")
    );
    assert!(
        problem_details
            .rows
            .iter()
            .all(|row| row.state.is_problem())
    );
    let problem_csv = scope_report_bundle_integrity_details_csv(&problem_details);
    assert!(problem_csv.contains("scope_plot.svg,Changed,"));
    assert!(!problem_csv.contains("index.html,OK,"));
    let problem_markdown = scope_report_bundle_integrity_details_markdown(&problem_details);
    assert!(problem_markdown.contains("| scope_plot.svg | Changed |"));
    assert!(!problem_markdown.contains("| index.html | OK |"));
    let all_details = scope_report_bundle_integrity_projected_details(&changed_details, false);
    assert_eq!(all_details.rows.len(), changed_details.rows.len());
    app.preview_scope_report_bundle_refresh(&bundle);
    assert_eq!(app.waveform_bundle_refresh_preview, Some(bundle.clone()));
    app.confirm_scope_report_bundle_refresh(&filtered);
    assert!(
        scope_report_bundle_changed_artifacts(bundle_path)
            .unwrap()
            .is_empty()
    );

    fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn scope_report_bundle_dir_uses_collision_suffix() {
    let base_dir = std::env::temp_dir().join(format!(
        "circuitci_scope_bundle_collision_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&base_dir);
    fs::create_dir_all(base_dir.join("scope_report_bundle_42")).unwrap();

    let next = unique_scope_report_bundle_dir(&base_dir, 42);

    assert_eq!(next, base_dir.join("scope_report_bundle_42_02"));
    fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn scope_report_bundle_cleanup_removes_only_old_bundle_dirs() {
    let base_dir = std::env::temp_dir().join(format!(
        "circuitci_scope_bundle_cleanup_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&base_dir);
    fs::create_dir_all(&base_dir).unwrap();
    for timestamp in [100_u128, 200, 300, 400] {
        fs::create_dir_all(base_dir.join(format!("scope_report_bundle_{timestamp}"))).unwrap();
    }
    fs::create_dir_all(base_dir.join("scope_report_bundle_300_02")).unwrap();
    fs::create_dir_all(base_dir.join("unrelated")).unwrap();

    let old = old_scope_report_bundle_dirs(&base_dir, 3).unwrap();
    assert_eq!(
        old,
        vec![
            base_dir.join("scope_report_bundle_200"),
            base_dir.join("scope_report_bundle_100"),
        ]
    );
    let removed = cleanup_old_scope_report_bundle_dirs(&base_dir, 3).unwrap();

    assert_eq!(removed, 2);
    assert!(base_dir.join("scope_report_bundle_400").exists());
    assert!(base_dir.join("scope_report_bundle_300_02").exists());
    assert!(base_dir.join("scope_report_bundle_300").exists());
    assert!(!base_dir.join("scope_report_bundle_200").exists());
    assert!(!base_dir.join("scope_report_bundle_100").exists());
    assert!(base_dir.join("unrelated").exists());
    fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn scope_report_bundle_recent_pruning_removes_missing_folders() {
    let base_dir = std::env::temp_dir().join(format!(
        "circuitci_scope_bundle_recent_prune_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&base_dir);
    let existing = base_dir.join("scope_report_bundle_200");
    let missing = base_dir.join("scope_report_bundle_100");
    fs::create_dir_all(&existing).unwrap();

    let mut app = CircuitCiApp {
        waveform_recent_report_bundles: vec![
            missing.to_string_lossy().into_owned(),
            existing.to_string_lossy().into_owned(),
        ],
        waveform_bundle_refresh_preview: Some(missing.to_string_lossy().into_owned()),
        waveform_bundle_integrity_details: Some(missing.to_string_lossy().into_owned()),
        waveform_bundle_cleanup_preview: vec![
            missing.to_string_lossy().into_owned(),
            existing.to_string_lossy().into_owned(),
        ],
        ..Default::default()
    };

    let pruned = app.prune_missing_scope_report_bundles();

    assert_eq!(pruned, 1);
    assert_eq!(
        app.waveform_recent_report_bundles,
        vec![existing.to_string_lossy().into_owned()]
    );
    assert_eq!(app.waveform_bundle_refresh_preview, None);
    assert_eq!(app.waveform_bundle_integrity_details, None);
    assert_eq!(
        app.waveform_bundle_cleanup_preview,
        vec![existing.to_string_lossy().into_owned()]
    );
    fs::remove_dir_all(&base_dir).unwrap();
}
