use super::merge_waveform_load_diagnostics;
use super::waveform_export::{ScopePlotSvgOptions, ScopePlotSvgSizePreset};
use super::waveform_plot::decimated_trace_samples_for_plot;
use super::{ScopeTriggerEdge, ScopeTriggerJump};
use super::{
    WaveformFootprintSortKey, WaveformLoadDiagnostic, WaveformLoadPreviewFilter,
    WaveformLoadRequest, WaveformLoadStatusFilter, WaveformPlotLaneMode, WaveformPlotTrigger,
    WaveformPlotView, WaveformSnapshotChip, WaveformSnapshotMarker, WaveformTraceColor,
    WaveformTracePreset, WaveformTraceRef, WaveformTraceStyle, nearest_scope_cursor_target,
    plot_x_to_time_us, plot_y_to_value, scope_plot_size, scope_snapshot_chip_hit,
    scope_zoom_box_interaction,
};
use super::{
    WaveformMathDraft, WaveformProbeGroup, append_derived_waveform_probe,
    clear_deferred_waveform_column_picks,
    deferred_waveform_artifact_filtered_unloaded_probe_labels,
    deferred_waveform_artifact_picked_probe_labels,
    deferred_waveform_artifact_unloaded_probe_labels, deferred_waveform_artifact_visible_indexes,
    deferred_waveform_matching_probe_requests, deferred_waveform_remaining_probe_requests,
    load_report_waveforms_with_progress_and_cancel, load_waveform_csv_with_progress_and_cancel,
    load_waveform_paths_with_progress_and_cancel, load_waveform_requests_with_progress_and_cancel,
    parse_waveform_csv_text, scope_plot_svg, scope_trigger_event_rows, scope_trigger_events,
    select_deferred_waveform_column_picks, select_scope_trigger_event, waveform_footprint_rows,
    waveform_load_deferred_artifacts, waveform_load_deferred_paths,
    waveform_load_diagnostic_unloaded_preview_columns, waveform_load_diagnostic_visible_indexes,
    waveform_load_diagnostics_csv, waveform_load_preflight, waveform_probe_choices,
    waveform_probe_group_choices,
};
use crate::gui::CircuitCiApp;
use std::collections::BTreeSet;

#[test]
fn waveform_parser_accepts_ngspice_header_and_samples() {
    let text = "time v(out) i(load)
0.0 0.0 0.001
1e-6 3.3 0.002
";
    let waveform = parse_waveform_csv_text(text, "waveform.csv").unwrap();
    assert_eq!(waveform.time_s, vec![0.0, 1e-6]);
    assert_eq!(waveform.probes[0].label, "v(out)");
    assert_eq!(waveform.probes[0].values, vec![0.0, 3.3]);
    assert_eq!(waveform.probes[1].label, "i(load)");
    assert_eq!(waveform.probes[1].values, vec![0.001, 0.002]);
}

#[test]
fn waveform_csv_file_loader_reports_progress_for_large_files() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    use std::io::Write;

    writeln!(file, "time v(out)").unwrap();
    for index in 0..120_000 {
        writeln!(file, "{}e-9 {}", index, index % 11).unwrap();
    }
    let mut progress = Vec::new();

    let waveform = load_waveform_csv_with_progress_and_cancel(
        file.path(),
        "large.csv",
        |stage, detail| progress.push((stage, detail)),
        || false,
    )
    .unwrap();

    assert_eq!(waveform.time_s.len(), 120_000);
    assert!(
        progress
            .iter()
            .any(|(stage, detail)| *stage == "Loading waveforms" && detail.contains("large.csv"))
    );
}

#[test]
fn waveform_csv_file_loader_honors_cancellation() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    use std::io::Write;

    writeln!(file, "time v(out)").unwrap();
    writeln!(file, "0 0").unwrap();

    let error =
        load_waveform_csv_with_progress_and_cancel(file.path(), "cancel.csv", |_, _| {}, || true)
            .unwrap_err();

    assert!(crate::cancellation::is_canceled(&error));
}

#[test]
fn waveform_request_loader_reads_only_selected_probe_columns() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    use std::io::Write;

    writeln!(file, "time v(out) i(load) p(load)").unwrap();
    writeln!(file, "0 0 0.001 0").unwrap();
    writeln!(file, "1e-6 3.3 0.002 0.0066").unwrap();
    let path = file.path().to_string_lossy().into_owned();
    let requests = vec![WaveformLoadRequest::selected_columns(
        path,
        vec!["i(load)".to_string()],
    )];

    let (waveforms, diagnostics) =
        load_waveform_requests_with_progress_and_cancel(&requests, |_, _| {}, || false, false)
            .unwrap();

    assert_eq!(waveforms.len(), 1);
    assert_eq!(waveforms[0].time_s, vec![0.0, 1e-6]);
    assert_eq!(waveforms[0].probes.len(), 1);
    assert_eq!(waveforms[0].probes[0].label, "i(load)");
    assert_eq!(waveforms[0].probes[0].values, vec![0.001, 0.002]);
    assert!(diagnostics[0].loaded);
    assert_eq!(diagnostics[0].probes, 1);
    assert_eq!(diagnostics[0].probe_preview, vec!["i(load)"]);
    assert!(diagnostics[0].detail.contains("selected probe column"));
}

#[test]
fn waveform_request_loader_rejects_missing_selected_probe_columns() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    use std::io::Write;

    writeln!(file, "time v(out)").unwrap();
    writeln!(file, "0 0").unwrap();
    let path = file.path().to_string_lossy().into_owned();
    let requests = vec![WaveformLoadRequest::selected_columns(
        path,
        vec!["i(load)".to_string()],
    )];

    let (waveforms, diagnostics) =
        load_waveform_requests_with_progress_and_cancel(&requests, |_, _| {}, || false, false)
            .unwrap();

    assert!(waveforms.is_empty());
    assert!(!diagnostics[0].loaded);
    assert!(
        diagnostics[0]
            .detail
            .contains("does not contain requested probe column")
    );
    assert_eq!(diagnostics[0].probe_preview, vec!["i(load)"]);
    assert!(diagnostics[0].detail.contains("Selected probe column"));
}

#[test]
fn selected_deferred_waveform_load_preserves_deferred_placeholder() {
    let path = "/tmp/run/scope.csv".to_string();
    let mut diagnostics = vec![WaveformLoadDiagnostic {
        path: path.clone(),
        loaded: false,
        deferred: true,
        bytes: Some(60 * 1024 * 1024),
        samples: 1_200_000,
        probes: 3,
        probe_preview: vec![
            "v(out)".to_string(),
            "i(load)".to_string(),
            "p(load)".to_string(),
        ],
        elapsed_ms: 2,
        detail: "Deferred large waveform artifact".to_string(),
    }];

    merge_waveform_load_diagnostics(
        &mut diagnostics,
        vec![WaveformLoadDiagnostic::loaded_selected(
            path.clone(),
            Some(60 * 1024 * 1024),
            4,
            1,
            9,
            vec!["i(load)".to_string()],
        )],
    );

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(
        waveform_load_deferred_paths(&diagnostics),
        vec![path.clone()]
    );
    let artifacts = waveform_load_deferred_artifacts(&diagnostics);
    assert_eq!(artifacts.len(), 1);
    assert_eq!(
        artifacts[0].probe_preview,
        vec!["v(out)", "i(load)", "p(load)"]
    );
    assert_eq!(artifacts[0].loaded_probe_preview, vec!["i(load)"]);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.loaded
            && !diagnostic.deferred
            && diagnostic.probe_preview == vec!["i(load)".to_string()]
    }));

    merge_waveform_load_diagnostics(
        &mut diagnostics,
        vec![WaveformLoadDiagnostic::loaded(
            path.clone(),
            Some(60 * 1024 * 1024),
            4,
            3,
            12,
        )],
    );

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].loaded);
    assert!(!diagnostics[0].deferred);
    assert!(diagnostics[0].probe_preview.is_empty());
    assert!(waveform_load_deferred_paths(&diagnostics).is_empty());
}

#[test]
fn unloading_full_waveform_restores_deferred_placeholder() {
    let path = "/tmp/run/full_scope.csv";
    let waveform =
        parse_waveform_csv_text("time,v(out),i(load)\n0,0,0.1\n0.000001,1,0.2\n", path).unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        waveform_load_diagnostics: vec![WaveformLoadDiagnostic::loaded(
            path.to_string(),
            Some(256),
            2,
            2,
            4,
        )],
        waveform_pinned_traces: vec![WaveformTraceRef {
            waveform_index: 0,
            probe_index: 1,
        }],
        waveform_trace_presets: vec![WaveformTracePreset {
            name: "load current".to_string(),
            traces: vec![WaveformTraceRef {
                waveform_index: 0,
                probe_index: 1,
            }],
        }],
        waveform_trace_styles: vec![WaveformTraceStyle {
            trace: WaveformTraceRef {
                waveform_index: 0,
                probe_index: 1,
            },
            color: Some(WaveformTraceColor::Amber),
            visible: true,
        }],
        selected_probe: 1,
        ..Default::default()
    };

    app.unload_waveform_view(0);

    assert!(app.waveforms.is_empty());
    assert!(app.waveform_pinned_traces.is_empty());
    assert!(app.waveform_trace_presets.is_empty());
    assert!(app.waveform_trace_styles.is_empty());
    assert_eq!(app.selected_waveform, 0);
    assert_eq!(app.selected_probe, 0);
    let artifacts = waveform_load_deferred_artifacts(&app.waveform_load_diagnostics);
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].path, path);
    assert_eq!(artifacts[0].probe_preview, vec!["v(out)", "i(load)"]);
    assert!(artifacts[0].loaded_probe_preview.is_empty());
}

#[test]
fn unloading_selected_column_waveform_marks_columns_unloaded_again() {
    let selected_path = "/tmp/run/lazy_scope.csv";
    let other_path = "/tmp/run/other_scope.csv";
    let selected_waveform =
        parse_waveform_csv_text("time,i(load)\n0,0.1\n0.000001,0.2\n", selected_path).unwrap();
    let other_waveform =
        parse_waveform_csv_text("time,v(ref)\n0,3.3\n0.000001,3.2\n", other_path).unwrap();
    let selected_diagnostic = WaveformLoadDiagnostic::loaded_selected(
        selected_path.to_string(),
        Some(512),
        2,
        1,
        8,
        vec!["i(load)".to_string()],
    );
    let mut app = CircuitCiApp {
        waveforms: vec![selected_waveform, other_waveform],
        waveform_load_diagnostics: vec![
            WaveformLoadDiagnostic {
                path: selected_path.to_string(),
                loaded: false,
                deferred: true,
                bytes: Some(512),
                samples: 2,
                probes: 2,
                probe_preview: vec!["v(out)".to_string(), "i(load)".to_string()],
                elapsed_ms: 1,
                detail: "Deferred large waveform artifact".to_string(),
            },
            selected_diagnostic.clone(),
            WaveformLoadDiagnostic::loaded(other_path.to_string(), Some(128), 2, 1, 3),
        ],
        selected_waveform: 1,
        waveform_pinned_traces: vec![WaveformTraceRef {
            waveform_index: 1,
            probe_index: 0,
        }],
        waveform_trace_styles: vec![WaveformTraceStyle {
            trace: WaveformTraceRef {
                waveform_index: 1,
                probe_index: 0,
            },
            color: Some(WaveformTraceColor::Cyan),
            visible: true,
        }],
        ..Default::default()
    };

    app.unload_waveform_for_diagnostic(&selected_diagnostic);

    assert_eq!(app.waveforms.len(), 1);
    assert_eq!(app.waveforms[0].path, other_path);
    assert_eq!(app.selected_waveform, 0);
    assert_eq!(
        app.waveform_pinned_traces,
        vec![WaveformTraceRef {
            waveform_index: 0,
            probe_index: 0,
        }]
    );
    assert_eq!(
        app.waveform_trace_styles[0].trace,
        WaveformTraceRef {
            waveform_index: 0,
            probe_index: 0,
        }
    );
    assert!(
        !app.waveform_load_diagnostics
            .iter()
            .any(|diagnostic| diagnostic.loaded
                && !diagnostic.deferred
                && diagnostic.path == selected_path)
    );
    let artifacts = waveform_load_deferred_artifacts(&app.waveform_load_diagnostics);
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].loaded_probe_preview, Vec::<String>::new());
    assert_eq!(
        deferred_waveform_artifact_unloaded_probe_labels(&artifacts[0]),
        vec!["v(out)", "i(load)"]
    );
}

#[test]
fn unloading_partial_view_does_not_forget_full_loaded_diagnostic() {
    let path = "/tmp/run/mixed_scope.csv";
    let partial_waveform =
        parse_waveform_csv_text("time,i(load)\n0,0.1\n0.000001,0.2\n", path).unwrap();
    let full_waveform =
        parse_waveform_csv_text("time,v(out),i(load)\n0,0,0.1\n0.000001,1,0.2\n", path).unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![partial_waveform, full_waveform],
        waveform_load_diagnostics: vec![WaveformLoadDiagnostic::loaded(
            path.to_string(),
            Some(512),
            2,
            2,
            12,
        )],
        ..Default::default()
    };

    app.unload_waveform_view(0);

    assert_eq!(app.waveforms.len(), 1);
    assert_eq!(app.waveforms[0].probes.len(), 2);
    assert_eq!(app.waveform_load_diagnostics.len(), 1);
    assert!(app.waveform_load_diagnostics[0].loaded);
    assert!(!app.waveform_load_diagnostics[0].deferred);
    assert!(waveform_load_deferred_artifacts(&app.waveform_load_diagnostics).is_empty());

    let diagnostic = app.waveform_load_diagnostics[0].clone();
    app.unload_waveform_for_diagnostic(&diagnostic);

    assert!(app.waveforms.is_empty());
    let artifacts = waveform_load_deferred_artifacts(&app.waveform_load_diagnostics);
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].probe_preview, vec!["v(out)", "i(load)"]);
}

#[test]
fn failed_selected_deferred_waveform_load_preserves_deferred_placeholder() {
    let path = "/tmp/run/scope.csv".to_string();
    let mut diagnostics = vec![WaveformLoadDiagnostic {
        path: path.clone(),
        loaded: false,
        deferred: true,
        bytes: Some(60 * 1024 * 1024),
        samples: 1_200_000,
        probes: 2,
        probe_preview: vec!["v(out)".to_string(), "i(load)".to_string()],
        elapsed_ms: 2,
        detail: "Deferred large waveform artifact".to_string(),
    }];

    merge_waveform_load_diagnostics(
        &mut diagnostics,
        vec![WaveformLoadDiagnostic::skipped_selected(
            path.clone(),
            Some(60 * 1024 * 1024),
            8,
            vec!["p(load)".to_string()],
            "missing selected column".to_string(),
        )],
    );

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(waveform_load_deferred_paths(&diagnostics), vec![path]);
    assert!(diagnostics.iter().any(|diagnostic| {
        !diagnostic.loaded
            && !diagnostic.deferred
            && diagnostic.probe_preview == vec!["p(load)".to_string()]
            && diagnostic.detail.contains("Selected probe column")
    }));
}

#[test]
fn report_waveform_loader_records_loaded_and_skipped_diagnostics() {
    let temp_dir = tempfile::tempdir().unwrap();
    let waveform_path = temp_dir.path().join("scope.csv");
    std::fs::write(&waveform_path, "time v(out)\n0 0\n1e-6 3.3\n").unwrap();
    let missing_path = temp_dir.path().join("missing.csv");
    let report = crate::reports::ValidationReport::from_parts(
        "project".to_string(),
        "default".to_string(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![
            waveform_path.to_string_lossy().into_owned(),
            missing_path.to_string_lossy().into_owned(),
        ],
        "validate".to_string(),
    );

    let (waveforms, diagnostics) =
        load_report_waveforms_with_progress_and_cancel(&report, |_, _| {}, || false, false)
            .unwrap();

    assert_eq!(waveforms.len(), 1);
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics[0].loaded);
    assert!(!diagnostics[0].deferred);
    assert_eq!(diagnostics[0].samples, 2);
    assert_eq!(diagnostics[0].probes, 1);
    assert!(diagnostics[0].bytes.unwrap() > 0);
    assert!(diagnostics[0].detail.contains("Loaded 2 sample row"));
    assert!(!diagnostics[1].loaded);
    assert!(!diagnostics[1].deferred);
    assert_eq!(diagnostics[1].path, missing_path.to_string_lossy());
    assert!(
        diagnostics[1]
            .detail
            .contains("Failed to read waveform CSV")
    );
}

#[test]
fn waveform_load_preflight_estimates_rows_and_warns_for_large_artifacts() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    use std::io::Write;

    writeln!(file, "time v(out)").unwrap();
    for index in 0..10 {
        writeln!(file, "{}e-6 {}", index, index).unwrap();
    }
    let preflight = waveform_load_preflight(file.path());

    assert_eq!(preflight.estimated_rows, Some(10));
    assert_eq!(preflight.probe_preview, vec!["v(out)"]);
    assert!(!preflight.warning);
    assert!(preflight.summary.contains("10 data row"));

    let large_file = tempfile::NamedTempFile::new().unwrap();
    large_file.as_file().set_len(51 * 1024 * 1024).unwrap();
    let preflight = waveform_load_preflight(large_file.path());

    assert!(preflight.warning);
    assert!(preflight.summary.contains("MiB"));
}

#[test]
fn report_waveform_loader_emits_preflight_progress_before_parsing() {
    let temp_dir = tempfile::tempdir().unwrap();
    let waveform_path = temp_dir.path().join("scope.csv");
    std::fs::write(&waveform_path, "time v(out)\n0 0\n1e-6 3.3\n").unwrap();
    let report = crate::reports::ValidationReport::from_parts(
        "project".to_string(),
        "default".to_string(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![waveform_path.to_string_lossy().into_owned()],
        "validate".to_string(),
    );
    let mut progress = Vec::new();

    load_report_waveforms_with_progress_and_cancel(
        &report,
        |stage, detail| progress.push((stage, detail)),
        || false,
        false,
    )
    .unwrap();

    assert!(progress.iter().any(|(stage, detail)| {
        *stage == "Waveform preflight"
            && detail.contains("scope.csv")
            && detail.contains("2 data row")
    }));
}

#[test]
fn report_waveform_loader_defers_large_artifacts_until_requested() {
    let mut large_file = tempfile::NamedTempFile::new().unwrap();
    use std::io::Write;

    writeln!(large_file, "time v(out) i(load)").unwrap();
    large_file.as_file().set_len(51 * 1024 * 1024).unwrap();
    let path = large_file.path().to_string_lossy().into_owned();
    let mut progress = Vec::new();

    let (waveforms, diagnostics) = load_waveform_paths_with_progress_and_cancel(
        std::slice::from_ref(&path),
        |stage, detail| progress.push((stage, detail)),
        || false,
        true,
    )
    .unwrap();

    assert!(waveforms.is_empty());
    assert_eq!(diagnostics.len(), 1);
    assert!(!diagnostics[0].loaded);
    assert!(diagnostics[0].deferred);
    assert_eq!(diagnostics[0].probes, 2);
    assert_eq!(
        diagnostics[0].probe_preview,
        vec!["v(out)".to_string(), "i(load)".to_string()]
    );
    assert_eq!(
        waveform_load_deferred_paths(&diagnostics),
        vec![path.clone()]
    );
    assert!(
        progress
            .iter()
            .any(|(stage, detail)| *stage == "Deferred waveform artifact"
                && detail.contains(&path))
    );

    let (waveforms, diagnostics) = load_waveform_paths_with_progress_and_cancel(
        std::slice::from_ref(&path),
        |_, _| {},
        || false,
        false,
    )
    .unwrap();

    assert!(waveforms.is_empty());
    assert_eq!(diagnostics.len(), 1);
    assert!(!diagnostics[0].loaded);
    assert!(!diagnostics[0].deferred);
    assert!(diagnostics[0].detail.contains("Waveform CSV"));
}

#[test]
fn waveform_load_diagnostics_filter_and_csv_use_visible_rows() {
    let diagnostics = vec![
        WaveformLoadDiagnostic {
            path: "fast.csv".to_string(),
            loaded: true,
            deferred: false,
            bytes: Some(128),
            samples: 2,
            probes: 1,
            probe_preview: Vec::new(),
            elapsed_ms: 4,
            detail: "Loaded 2 sample row(s).".to_string(),
        },
        WaveformLoadDiagnostic {
            path: "slow.csv".to_string(),
            loaded: true,
            deferred: false,
            bytes: Some(2048),
            samples: 4000,
            probes: 3,
            probe_preview: Vec::new(),
            elapsed_ms: 180,
            detail: "Loaded 4000 sample row(s).".to_string(),
        },
        WaveformLoadDiagnostic {
            path: "huge.csv".to_string(),
            loaded: false,
            deferred: true,
            bytes: Some(60 * 1024 * 1024),
            samples: 1_200_000,
            probes: 2,
            probe_preview: vec!["v(out)".to_string(), "i(load)".to_string()],
            elapsed_ms: 2,
            detail: "Deferred large waveform artifact".to_string(),
        },
        WaveformLoadDiagnostic::loaded_selected(
            "huge.csv".to_string(),
            Some(60 * 1024 * 1024),
            32,
            1,
            15,
            vec!["i(load)".to_string()],
        ),
        WaveformLoadDiagnostic {
            path: "missing.csv".to_string(),
            loaded: false,
            deferred: false,
            bytes: None,
            samples: 0,
            probes: 0,
            probe_preview: Vec::new(),
            elapsed_ms: 12,
            detail: "Failed, \"missing\" file".to_string(),
        },
    ];

    assert_eq!(
        waveform_load_diagnostic_visible_indexes(
            &diagnostics,
            "missing",
            WaveformLoadStatusFilter::Skipped,
            WaveformLoadPreviewFilter::All,
            0.0,
            false,
        ),
        vec![4]
    );
    assert_eq!(
        waveform_load_diagnostic_visible_indexes(
            &diagnostics,
            "",
            WaveformLoadStatusFilter::Deferred,
            WaveformLoadPreviewFilter::All,
            0.0,
            false,
        ),
        vec![2]
    );
    assert_eq!(
        waveform_load_diagnostic_visible_indexes(
            &diagnostics,
            "i(load)",
            WaveformLoadStatusFilter::All,
            WaveformLoadPreviewFilter::All,
            0.0,
            false,
        ),
        vec![2, 3]
    );
    assert_eq!(
        waveform_load_diagnostic_visible_indexes(
            &diagnostics,
            "",
            WaveformLoadStatusFilter::Loaded,
            WaveformLoadPreviewFilter::All,
            10.0,
            true,
        ),
        vec![1, 3]
    );

    let visible_indexes = waveform_load_diagnostic_visible_indexes(
        &diagnostics,
        "",
        WaveformLoadStatusFilter::All,
        WaveformLoadPreviewFilter::All,
        10.0,
        true,
    );
    let csv = waveform_load_diagnostics_csv(&diagnostics, &visible_indexes);

    assert!(csv.starts_with(
        "status,path,size_bytes,samples,probes,elapsed_ms,preview_columns,loaded_preview_columns,unloaded_preview_columns,detail\n"
    ));
    assert!(csv.contains("loaded,slow.csv,2048,4000,3,180,,,"));
    assert!(
        csv.contains("loaded,huge.csv,62914560,32,1,15,i(load),i(load),,Loaded 32 sample row(s)")
    );
    assert!(csv.contains("skipped,missing.csv,,0,0,12,,,,\"Failed, \"\"missing\"\" file\""));
    assert!(!csv.contains("Deferred large waveform artifact"));
    assert!(!csv.contains("fast.csv"));

    let deferred_only = waveform_load_diagnostic_visible_indexes(
        &diagnostics,
        "",
        WaveformLoadStatusFilter::Deferred,
        WaveformLoadPreviewFilter::All,
        0.0,
        false,
    );
    let deferred_csv = waveform_load_diagnostics_csv(&diagnostics, &deferred_only);
    assert!(deferred_csv.contains(
        "deferred,huge.csv,62914560,1200000,2,2,v(out); i(load),i(load),v(out),Deferred large waveform artifact"
    ));
}

#[test]
fn waveform_load_diagnostics_preview_state_filters_deferred_rows() {
    let diagnostics = vec![
        WaveformLoadDiagnostic {
            path: "partial.csv".to_string(),
            loaded: false,
            deferred: true,
            bytes: Some(60 * 1024 * 1024),
            samples: 1_200_000,
            probes: 2,
            probe_preview: vec!["v(out)".to_string(), "i(load)".to_string()],
            elapsed_ms: 2,
            detail: "Deferred large waveform artifact".to_string(),
        },
        WaveformLoadDiagnostic::loaded_selected(
            "partial.csv".to_string(),
            Some(60 * 1024 * 1024),
            32,
            1,
            15,
            vec!["i(load)".to_string()],
        ),
        WaveformLoadDiagnostic {
            path: "complete.csv".to_string(),
            loaded: false,
            deferred: true,
            bytes: Some(64 * 1024 * 1024),
            samples: 1_400_000,
            probes: 1,
            probe_preview: vec!["p(load)".to_string()],
            elapsed_ms: 3,
            detail: "Deferred large waveform artifact".to_string(),
        },
        WaveformLoadDiagnostic::loaded_selected(
            "complete.csv".to_string(),
            Some(64 * 1024 * 1024),
            40,
            1,
            16,
            vec!["p(load)".to_string()],
        ),
        WaveformLoadDiagnostic::loaded("plain.csv".to_string(), Some(128), 2, 1, 4),
    ];

    assert_eq!(
        waveform_load_diagnostic_visible_indexes(
            &diagnostics,
            "",
            WaveformLoadStatusFilter::All,
            WaveformLoadPreviewFilter::HasUnloadedPreview,
            0.0,
            false,
        ),
        vec![0]
    );
    assert_eq!(
        waveform_load_diagnostic_visible_indexes(
            &diagnostics,
            "",
            WaveformLoadStatusFilter::All,
            WaveformLoadPreviewFilter::FullyLoadedPreview,
            0.0,
            false,
        ),
        vec![2]
    );
    assert_eq!(
        waveform_load_diagnostic_visible_indexes(
            &diagnostics,
            "",
            WaveformLoadStatusFilter::All,
            WaveformLoadPreviewFilter::NoPreview,
            0.0,
            false,
        ),
        vec![4]
    );
    assert_eq!(
        waveform_load_diagnostic_visible_indexes(
            &diagnostics,
            "",
            WaveformLoadStatusFilter::Loaded,
            WaveformLoadPreviewFilter::HasPreview,
            0.0,
            false,
        ),
        vec![1, 3]
    );
}

#[test]
fn waveform_load_diagnostics_unloaded_preview_columns_skip_selected_loads() {
    let diagnostics = vec![
        WaveformLoadDiagnostic {
            path: "huge.csv".to_string(),
            loaded: false,
            deferred: true,
            bytes: Some(60 * 1024 * 1024),
            samples: 1_200_000,
            probes: 3,
            probe_preview: vec![
                "v(out)".to_string(),
                "i(load)".to_string(),
                "p(load)".to_string(),
            ],
            elapsed_ms: 2,
            detail: "Deferred large waveform artifact".to_string(),
        },
        WaveformLoadDiagnostic::loaded_selected(
            "huge.csv".to_string(),
            Some(60 * 1024 * 1024),
            32,
            1,
            15,
            vec!["i(load)".to_string()],
        ),
    ];

    assert_eq!(
        waveform_load_diagnostic_unloaded_preview_columns(&diagnostics, &diagnostics[0]),
        vec!["v(out)", "p(load)"]
    );
    assert!(
        waveform_load_diagnostic_unloaded_preview_columns(&diagnostics, &diagnostics[1]).is_empty()
    );
}

#[test]
fn deferred_waveform_artifacts_project_selector_placeholders() {
    let diagnostics = vec![
        WaveformLoadDiagnostic {
            path: "/tmp/run/scope_a.csv".to_string(),
            loaded: false,
            deferred: true,
            bytes: Some(60 * 1024 * 1024),
            samples: 1_200_000,
            probes: 2,
            probe_preview: vec!["v(out)".to_string(), "i(load)".to_string()],
            elapsed_ms: 2,
            detail: "Deferred large waveform artifact".to_string(),
        },
        WaveformLoadDiagnostic {
            path: "/tmp/run/missing.csv".to_string(),
            loaded: false,
            deferred: false,
            bytes: None,
            samples: 0,
            probes: 0,
            probe_preview: Vec::new(),
            elapsed_ms: 3,
            detail: "Skipped".to_string(),
        },
    ];

    let artifacts = waveform_load_deferred_artifacts(&diagnostics);

    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].path, "/tmp/run/scope_a.csv");
    assert_eq!(artifacts[0].label, "scope_a.csv");
    assert_eq!(artifacts[0].size_label, "60.0 MiB");
    assert_eq!(artifacts[0].samples, 1_200_000);
    assert_eq!(artifacts[0].probes, 2);
    assert_eq!(artifacts[0].probe_preview, vec!["v(out)", "i(load)"]);
    assert!(artifacts[0].loaded_probe_preview.is_empty());
}

#[test]
fn deferred_waveform_artifact_filter_matches_probe_preview_and_metadata() {
    let diagnostics = vec![
        WaveformLoadDiagnostic {
            path: "/tmp/run/scope_voltage.csv".to_string(),
            loaded: false,
            deferred: true,
            bytes: Some(60 * 1024 * 1024),
            samples: 1_200_000,
            probes: 2,
            probe_preview: vec!["v(out)".to_string(), "v(ref)".to_string()],
            elapsed_ms: 2,
            detail: "Deferred large waveform artifact".to_string(),
        },
        WaveformLoadDiagnostic {
            path: "/tmp/run/scope_power.csv".to_string(),
            loaded: false,
            deferred: true,
            bytes: Some(80 * 1024 * 1024),
            samples: 2_400_000,
            probes: 2,
            probe_preview: vec!["i(load)".to_string(), "p(load)".to_string()],
            elapsed_ms: 3,
            detail: "Deferred large waveform artifact".to_string(),
        },
    ];
    let artifacts = waveform_load_deferred_artifacts(&diagnostics);

    assert_eq!(
        deferred_waveform_artifact_visible_indexes(&artifacts, ""),
        vec![0, 1]
    );
    assert_eq!(
        deferred_waveform_artifact_visible_indexes(&artifacts, "p(load)"),
        vec![1]
    );
    assert_eq!(
        deferred_waveform_artifact_visible_indexes(&artifacts, "voltage"),
        vec![0]
    );
    assert_eq!(
        deferred_waveform_artifact_visible_indexes(&artifacts, "not-present"),
        Vec::<usize>::new()
    );

    let visible_indexes = deferred_waveform_artifact_visible_indexes(&artifacts, "load");
    assert_eq!(
        deferred_waveform_matching_probe_requests(&artifacts, &visible_indexes, "LOAD"),
        vec![WaveformLoadRequest::selected_columns(
            "/tmp/run/scope_power.csv".to_string(),
            vec!["i(load)".to_string(), "p(load)".to_string()]
        )]
    );
    assert!(
        deferred_waveform_matching_probe_requests(&artifacts, &visible_indexes, "missing")
            .is_empty()
    );
}

#[test]
fn deferred_waveform_matching_probe_requests_skip_loaded_preview_columns() {
    let diagnostics = vec![
        WaveformLoadDiagnostic {
            path: "/tmp/run/scope_power.csv".to_string(),
            loaded: false,
            deferred: true,
            bytes: Some(80 * 1024 * 1024),
            samples: 2_400_000,
            probes: 3,
            probe_preview: vec![
                "i(load)".to_string(),
                "p(load)".to_string(),
                "p(aux)".to_string(),
            ],
            elapsed_ms: 3,
            detail: "Deferred large waveform artifact".to_string(),
        },
        WaveformLoadDiagnostic::loaded_selected(
            "/tmp/run/scope_power.csv".to_string(),
            Some(80 * 1024 * 1024),
            16,
            1,
            20,
            vec!["p(load)".to_string()],
        ),
    ];
    let artifacts = waveform_load_deferred_artifacts(&diagnostics);
    assert_eq!(artifacts[0].loaded_probe_preview, vec!["p(load)"]);

    let visible_indexes = deferred_waveform_artifact_visible_indexes(&artifacts, "p(");
    assert_eq!(
        deferred_waveform_matching_probe_requests(&artifacts, &visible_indexes, "p("),
        vec![WaveformLoadRequest::selected_columns(
            "/tmp/run/scope_power.csv".to_string(),
            vec!["p(aux)".to_string()]
        )]
    );
    assert!(
        deferred_waveform_matching_probe_requests(&artifacts, &visible_indexes, "p(load)")
            .is_empty()
    );
}

#[test]
fn deferred_waveform_remaining_probe_requests_skip_loaded_preview_columns() {
    let diagnostics = vec![
        WaveformLoadDiagnostic {
            path: "/tmp/run/scope_power.csv".to_string(),
            loaded: false,
            deferred: true,
            bytes: Some(80 * 1024 * 1024),
            samples: 2_400_000,
            probes: 4,
            probe_preview: vec![
                "i(load)".to_string(),
                "p(load)".to_string(),
                "p(aux)".to_string(),
                "v(out)".to_string(),
            ],
            elapsed_ms: 3,
            detail: "Deferred large waveform artifact".to_string(),
        },
        WaveformLoadDiagnostic::loaded_selected(
            "/tmp/run/scope_power.csv".to_string(),
            Some(80 * 1024 * 1024),
            16,
            2,
            20,
            vec!["p(load)".to_string(), "v(out)".to_string()],
        ),
    ];
    let artifacts = waveform_load_deferred_artifacts(&diagnostics);
    let visible_indexes = deferred_waveform_artifact_visible_indexes(&artifacts, "");

    assert_eq!(
        deferred_waveform_remaining_probe_requests(&artifacts, &visible_indexes),
        vec![WaveformLoadRequest::selected_columns(
            "/tmp/run/scope_power.csv".to_string(),
            vec!["i(load)".to_string(), "p(aux)".to_string()]
        )]
    );
}

#[test]
fn deferred_waveform_column_picker_uses_unloaded_preview_columns() {
    let diagnostics = vec![
        WaveformLoadDiagnostic {
            path: "/tmp/run/scope_power.csv".to_string(),
            loaded: false,
            deferred: true,
            bytes: Some(80 * 1024 * 1024),
            samples: 2_400_000,
            probes: 3,
            probe_preview: vec![
                "i(load)".to_string(),
                "p(load)".to_string(),
                "p(aux)".to_string(),
            ],
            elapsed_ms: 3,
            detail: "Deferred large waveform artifact".to_string(),
        },
        WaveformLoadDiagnostic::loaded_selected(
            "/tmp/run/scope_power.csv".to_string(),
            Some(80 * 1024 * 1024),
            16,
            1,
            20,
            vec!["p(load)".to_string()],
        ),
    ];
    let artifacts = waveform_load_deferred_artifacts(&diagnostics);
    let artifact = &artifacts[0];

    assert_eq!(
        deferred_waveform_artifact_unloaded_probe_labels(artifact),
        vec!["i(load)", "p(aux)"]
    );

    let picks = BTreeSet::from([
        (artifact.path.clone(), "p(load)".to_string()),
        (artifact.path.clone(), "p(aux)".to_string()),
        (artifact.path.clone(), "i(load)".to_string()),
    ]);
    assert_eq!(
        deferred_waveform_artifact_picked_probe_labels(artifact, &picks),
        vec!["i(load)", "p(aux)"]
    );
}

#[test]
fn deferred_waveform_column_picker_filters_and_selects_visible_unloaded_columns() {
    let diagnostics = vec![
        WaveformLoadDiagnostic {
            path: "/tmp/run/scope_power.csv".to_string(),
            loaded: false,
            deferred: true,
            bytes: Some(80 * 1024 * 1024),
            samples: 2_400_000,
            probes: 4,
            probe_preview: vec![
                "i(load)".to_string(),
                "p(load)".to_string(),
                "p(aux)".to_string(),
                "v(out)".to_string(),
            ],
            elapsed_ms: 3,
            detail: "Deferred large waveform artifact".to_string(),
        },
        WaveformLoadDiagnostic::loaded_selected(
            "/tmp/run/scope_power.csv".to_string(),
            Some(80 * 1024 * 1024),
            16,
            1,
            20,
            vec!["p(load)".to_string()],
        ),
    ];
    let artifacts = waveform_load_deferred_artifacts(&diagnostics);
    let artifact = &artifacts[0];
    let visible_unloaded =
        deferred_waveform_artifact_filtered_unloaded_probe_labels(artifact, "P(");

    assert_eq!(visible_unloaded, vec!["p(aux)"]);

    let mut picks = BTreeSet::new();
    select_deferred_waveform_column_picks(&mut picks, artifact, &visible_unloaded);
    assert_eq!(
        deferred_waveform_artifact_picked_probe_labels(artifact, &picks),
        vec!["p(aux)"]
    );
    clear_deferred_waveform_column_picks(&mut picks, artifact, &visible_unloaded);
    assert!(deferred_waveform_artifact_picked_probe_labels(artifact, &picks).is_empty());

    let empty = deferred_waveform_artifact_filtered_unloaded_probe_labels(artifact, "p(load)");
    assert!(empty.is_empty());
}

#[test]
fn scope_plot_decimator_keeps_exact_small_windows() {
    let waveform = parse_waveform_csv_text(
        "time v(out)
0.0 0.0
1e-6 1.0
2e-6 0.5
3e-6 2.0
",
        "scope.csv",
    )
    .unwrap();

    let samples = decimated_trace_samples_for_plot(&waveform, &waveform.probes[0], 0.0, 3e-6, 32);

    assert_eq!(
        samples,
        vec![(0.0, 0.0), (1e-6, 1.0), (2e-6, 0.5), (3e-6, 2.0)]
    );
}

#[test]
fn scope_plot_decimator_preserves_bucket_extrema_for_dense_traces() {
    let mut csv = String::from("time v(out)\n");
    for index in 0..=1000 {
        let value = match index {
            123 => 10.0,
            124 => -7.0,
            500 => 8.0,
            777 => 6.0,
            _ => 0.0,
        };
        csv.push_str(&format!("{}e-6 {value}\n", index));
    }
    let waveform = parse_waveform_csv_text(&csv, "scope.csv").unwrap();

    let samples =
        decimated_trace_samples_for_plot(&waveform, &waveform.probes[0], 0.0, 1000e-6, 10);

    assert!(samples.len() <= 22);
    assert!(
        samples
            .iter()
            .any(|sample| sample.0 == 123e-6 && sample.1 == 10.0)
    );
    assert!(
        samples
            .iter()
            .any(|sample| sample.0 == 124e-6 && sample.1 == -7.0)
    );
    assert!(
        samples
            .iter()
            .any(|sample| sample.0 == 500e-6 && sample.1 == 8.0)
    );
    assert!(
        samples
            .iter()
            .any(|sample| sample.0 == 777e-6 && sample.1 == 6.0)
    );
    assert!(samples.windows(2).all(|window| window[0].0 <= window[1].0));
}

#[test]
fn scope_plot_svg_exports_visible_runtime_annotations() {
    let waveform = parse_waveform_csv_text(
        "time v(out) i(load)
0.0 0.0 0.001
1e-6 3.3 0.002
2e-6 1.0 0.004
",
        "scope.csv",
    )
    .unwrap();
    let traces = vec![
        WaveformTraceRef {
            waveform_index: 0,
            probe_index: 0,
        },
        WaveformTraceRef {
            waveform_index: 0,
            probe_index: 1,
        },
    ];
    let markers = vec![WaveformSnapshotMarker {
        snapshot_index: 0,
        trace: traces[0],
        label: "Startup".to_string(),
        note: "captured".to_string(),
        source: "cursor selected".to_string(),
        trace_label: "v(out)".to_string(),
        time_a_us: Some(0.0),
        time_b_us: Some(2.0),
        value_a: Some(0.0),
        value_b: Some(1.0),
        event_edge: None,
    }];

    let svg = scope_plot_svg(
        &[waveform],
        &traces,
        0.0,
        2.0,
        WaveformPlotView {
            visible_window_us: Some((0.0, 2.0)),
            visible_value_window: None,
            lane_mode: WaveformPlotLaneMode::ByUnit,
            trigger: Some(WaveformPlotTrigger {
                threshold: 1.5,
                events_us: &[1.0],
            }),
            snapshot_markers: &markers,
        },
        &[],
        ScopePlotSvgOptions::default(),
    )
    .unwrap();

    assert!(svg.starts_with("<svg "));
    assert!(svg.contains("CircuitCI Scope Plot"));
    assert!(svg.contains("v(out)"));
    assert!(svg.contains("i(load)"));
    assert!(svg.contains(">A<"));
    assert!(svg.contains(">B<"));
    assert!(svg.contains(">T<"));
    assert!(svg.contains("Startup A"));
    assert!(svg.contains("Startup B"));
}

#[test]
fn scope_plot_svg_options_control_report_size_and_annotations() {
    let waveform = parse_waveform_csv_text(
        "time v(out)
0.0 0.0
1e-6 3.3
",
        "scope.csv",
    )
    .unwrap();
    let trace = WaveformTraceRef {
        waveform_index: 0,
        probe_index: 0,
    };
    let markers = vec![WaveformSnapshotMarker {
        snapshot_index: 0,
        trace,
        label: "Edge".to_string(),
        note: String::new(),
        source: "trigger event".to_string(),
        trace_label: "v(out)".to_string(),
        time_a_us: Some(1.0),
        time_b_us: None,
        value_a: Some(3.3),
        value_b: None,
        event_edge: Some("rising".to_string()),
    }];

    let svg = scope_plot_svg(
        &[waveform],
        &[trace],
        0.0,
        1.0,
        WaveformPlotView {
            visible_window_us: Some((0.0, 1.0)),
            visible_value_window: None,
            lane_mode: WaveformPlotLaneMode::Shared,
            trigger: Some(WaveformPlotTrigger {
                threshold: 1.5,
                events_us: &[1.0],
            }),
            snapshot_markers: &markers,
        },
        &[],
        ScopePlotSvgOptions {
            size_preset: ScopePlotSvgSizePreset::Compact,
            include_cursors: false,
            include_trigger: false,
            include_snapshots: false,
        },
    )
    .unwrap();

    assert!(svg.contains(r#"width="720" height="405""#));
    assert!(!svg.contains(">A<"));
    assert!(!svg.contains(">B<"));
    assert!(!svg.contains(">T<"));
    assert!(!svg.contains("Edge rising"));
}

#[test]
fn scope_plot_svg_decimates_dense_trace_points() {
    let mut csv = String::from("time v(out)\n");
    for index in 0..=5000 {
        let value = match index {
            1250 => 9.0,
            1251 => -8.0,
            3750 => 7.0,
            _ => 0.0,
        };
        csv.push_str(&format!("{}e-6 {value}\n", index));
    }
    let waveform = parse_waveform_csv_text(&csv, "dense.csv").unwrap();
    let trace = WaveformTraceRef {
        waveform_index: 0,
        probe_index: 0,
    };

    let svg = scope_plot_svg(
        &[waveform],
        &[trace],
        0.0,
        5000.0,
        WaveformPlotView {
            visible_window_us: Some((0.0, 5000.0)),
            visible_value_window: None,
            lane_mode: WaveformPlotLaneMode::Shared,
            trigger: None,
            snapshot_markers: &[],
        },
        &[],
        ScopePlotSvgOptions {
            size_preset: ScopePlotSvgSizePreset::Compact,
            ..ScopePlotSvgOptions::default()
        },
    )
    .unwrap();

    let points_attr = svg
        .split(r#"<polyline points=""#)
        .nth(1)
        .and_then(|tail| tail.split('"').next())
        .unwrap();
    let point_count = points_attr.split_whitespace().count();
    assert!(point_count <= 1260, "{point_count} points");
    assert!(point_count < 5001);
}

#[test]
fn waveform_probe_filter_matches_label_expression_and_kind() {
    let mut waveform = parse_waveform_csv_text(
        "time,v(out),i(load),temp\n0,1,0.1,25\n0.000001,2,0.2,26\n",
        "waveform.csv",
    )
    .unwrap();
    append_derived_waveform_probe(
        &mut waveform,
        &WaveformMathDraft {
            left_probe: 0,
            right_probe: 1,
            operation: "product".to_string(),
            label: "load_power".to_string(),
        },
    )
    .unwrap();

    let labels = |query: &str| {
        waveform_probe_choices(&waveform, query)
            .into_iter()
            .map(|choice| choice.label)
            .collect::<Vec<_>>()
    };

    assert_eq!(labels("out"), vec!["v(out)", "load_power"]);
    assert_eq!(labels("current"), vec!["i(load)"]);
    assert_eq!(labels("derived"), vec!["load_power"]);
    assert_eq!(labels("temp"), vec!["temp"]);
    assert!(labels("missing").is_empty());
}

#[test]
fn waveform_footprint_rows_filter_and_sort_loaded_views() {
    let small = parse_waveform_csv_text("time,v(out)\n0,1\n0.000001,2\n", "small.csv").unwrap();
    let large = parse_waveform_csv_text(
        "time,v(bus),i(load),temp\n0,12,0.1,25\n0.000001,11,0.2,26\n0.000002,10,0.3,27\n",
        "large.csv",
    )
    .unwrap();
    let waveforms = vec![small, large];

    let rows = waveform_footprint_rows(
        &waveforms,
        "",
        WaveformFootprintSortKey::EstimatedBytes,
        true,
    );

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].label, "large.csv");
    assert_eq!(rows[0].samples, 3);
    assert_eq!(rows[0].probes, 3);
    assert_eq!(rows[0].values, 12);
    assert_eq!(rows[0].estimated_bytes, 12 * std::mem::size_of::<f64>());
    assert_eq!(rows[1].label, "small.csv");

    let filtered = waveform_footprint_rows(
        &waveforms,
        "i(load)",
        WaveformFootprintSortKey::Label,
        false,
    );

    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].label, "large.csv");
}

#[test]
fn waveform_probe_grouping_orders_visible_traces_by_quantity() {
    let mut waveform = parse_waveform_csv_text(
        "time,v(out),i(load),temp\n0,1,0.1,25\n0.000001,2,0.2,26\n",
        "waveform.csv",
    )
    .unwrap();
    append_derived_waveform_probe(
        &mut waveform,
        &WaveformMathDraft {
            left_probe: 0,
            right_probe: 1,
            operation: "product".to_string(),
            label: "load_power".to_string(),
        },
    )
    .unwrap();

    let choices = waveform_probe_choices(&waveform, "");
    let groups = waveform_probe_group_choices(&choices)
        .into_iter()
        .map(|(group, choices)| {
            (
                group,
                choices
                    .into_iter()
                    .map(|choice| choice.index)
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        groups,
        vec![
            (WaveformProbeGroup::Voltage, vec![0]),
            (WaveformProbeGroup::Current, vec![1]),
            (WaveformProbeGroup::Derived, vec![3]),
            (WaveformProbeGroup::Other, vec![2]),
        ]
    );
}

#[test]
fn scope_plot_size_prefers_oscilloscope_workspace() {
    assert_eq!(
        scope_plot_size(eframe::egui::vec2(1100.0, 640.0)),
        eframe::egui::vec2(1100.0, 640.0)
    );
    assert_eq!(
        scope_plot_size(eframe::egui::vec2(320.0, 180.0)),
        eframe::egui::vec2(560.0, 360.0)
    );
}

#[test]
fn scope_plot_x_maps_to_visible_time_window() {
    let plot_rect = eframe::egui::Rect::from_min_size(
        eframe::egui::pos2(100.0, 20.0),
        eframe::egui::vec2(400.0, 240.0),
    );
    assert!((plot_x_to_time_us(100.0, plot_rect, 10.0, 50.0) - 10.0).abs() < 1.0e-12);
    assert!((plot_x_to_time_us(300.0, plot_rect, 10.0, 50.0) - 30.0).abs() < 1.0e-12);
    assert!((plot_x_to_time_us(500.0, plot_rect, 10.0, 50.0) - 50.0).abs() < 1.0e-12);
    assert!((plot_x_to_time_us(800.0, plot_rect, 10.0, 50.0) - 50.0).abs() < 1.0e-12);
}

#[test]
fn scope_plot_y_maps_to_visible_value_window() {
    let plot_rect = eframe::egui::Rect::from_min_size(
        eframe::egui::pos2(100.0, 20.0),
        eframe::egui::vec2(400.0, 240.0),
    );
    assert!((plot_y_to_value(260.0, plot_rect, -2.0, 6.0) - -2.0).abs() < 1.0e-12);
    assert!((plot_y_to_value(140.0, plot_rect, -2.0, 6.0) - 2.0).abs() < 1.0e-12);
    assert!((plot_y_to_value(20.0, plot_rect, -2.0, 6.0) - 6.0).abs() < 1.0e-12);
    assert!((plot_y_to_value(-100.0, plot_rect, -2.0, 6.0) - 6.0).abs() < 1.0e-12);
}

#[test]
fn scope_zoom_box_maps_shared_axis_time_and_value_windows() {
    let plot_rect = eframe::egui::Rect::from_min_max(
        eframe::egui::pos2(100.0, 20.0),
        eframe::egui::pos2(500.0, 260.0),
    );

    let interaction = scope_zoom_box_interaction(
        eframe::egui::pos2(200.0, 80.0),
        eframe::egui::pos2(400.0, 200.0),
        plot_rect,
        10.0,
        50.0,
        Some((plot_rect, -2.0, 6.0)),
    );

    assert_eq!(interaction.time_window_us, Some((20.0, 40.0)));
    let (value_min, value_max) = interaction.value_window.unwrap();
    assert!((value_min - 0.0).abs() < 1.0e-12);
    assert!((value_max - 4.0).abs() < 1.0e-12);
}

#[test]
fn scope_zoom_box_maps_split_lanes_to_time_window_only() {
    let plot_rect = eframe::egui::Rect::from_min_max(
        eframe::egui::pos2(100.0, 20.0),
        eframe::egui::pos2(500.0, 260.0),
    );

    let interaction = scope_zoom_box_interaction(
        eframe::egui::pos2(150.0, 40.0),
        eframe::egui::pos2(450.0, 180.0),
        plot_rect,
        0.0,
        100.0,
        None,
    );

    assert_eq!(interaction.time_window_us, Some((12.5, 87.5)));
    assert_eq!(interaction.value_window, None);
}

#[test]
fn scope_zoom_box_ignores_tiny_regions() {
    let plot_rect = eframe::egui::Rect::from_min_max(
        eframe::egui::pos2(100.0, 20.0),
        eframe::egui::pos2(500.0, 260.0),
    );

    let interaction = scope_zoom_box_interaction(
        eframe::egui::pos2(150.0, 40.0),
        eframe::egui::pos2(154.0, 44.0),
        plot_rect,
        0.0,
        100.0,
        Some((plot_rect, -1.0, 1.0)),
    );

    assert_eq!(interaction.time_window_us, None);
    assert_eq!(interaction.value_window, None);
}

#[test]
fn scope_cursor_hit_test_prefers_nearest_cursor_handle() {
    let plot_rect = eframe::egui::Rect::from_min_size(
        eframe::egui::pos2(100.0, 20.0),
        eframe::egui::vec2(400.0, 240.0),
    );
    let near_a = eframe::egui::pos2(200.0, 26.0);
    let near_b = eframe::egui::pos2(400.0, 26.0);
    let far = eframe::egui::pos2(300.0, 180.0);
    assert_eq!(
        nearest_scope_cursor_target(near_a, plot_rect, 25.0, 75.0, 0.0, 100.0),
        Some(super::WaveformCursorTarget::A)
    );
    assert_eq!(
        nearest_scope_cursor_target(near_b, plot_rect, 25.0, 75.0, 0.0, 100.0),
        Some(super::WaveformCursorTarget::B)
    );
    assert_eq!(
        nearest_scope_cursor_target(far, plot_rect, 25.0, 75.0, 0.0, 100.0),
        None
    );
}

#[test]
fn scope_snapshot_chip_hit_prefers_topmost_chip() {
    let chip_rect = eframe::egui::Rect::from_min_size(
        eframe::egui::pos2(10.0, 10.0),
        eframe::egui::vec2(80.0, 20.0),
    );
    let chips = vec![
        WaveformSnapshotChip {
            snapshot_index: 1,
            label: "Cursor 1 A".to_string(),
            detail: String::new(),
            color: eframe::egui::Color32::WHITE,
            line_x: 10.0,
            dot: eframe::egui::pos2(10.0, 20.0),
            chip_rect,
            lane_rect: chip_rect,
        },
        WaveformSnapshotChip {
            snapshot_index: 2,
            label: "Trigger 2".to_string(),
            detail: String::new(),
            color: eframe::egui::Color32::WHITE,
            line_x: 12.0,
            dot: eframe::egui::pos2(12.0, 20.0),
            chip_rect,
            lane_rect: chip_rect,
        },
    ];

    assert_eq!(
        scope_snapshot_chip_hit(&chips, eframe::egui::pos2(24.0, 18.0)),
        Some(2)
    );
    assert_eq!(
        scope_snapshot_chip_hit(&chips, eframe::egui::pos2(200.0, 18.0)),
        None
    );
}

#[test]
fn scope_trigger_events_interpolate_selected_edges() {
    let waveform = parse_waveform_csv_text(
        "time,v(out)\n0,0\n0.000001,2\n0.000002,0\n0.000003,3\n",
        "waveform.csv",
    )
    .unwrap();

    let rising = scope_trigger_events(&waveform, 0, 1.0, ScopeTriggerEdge::Rising);
    assert_eq!(rising.len(), 2);
    assert!((rising[0].time_us - 0.5).abs() < 1.0e-12);
    assert_eq!(rising[0].edge, ScopeTriggerEdge::Rising);
    assert!((rising[1].time_us - 2.333333333333333).abs() < 1.0e-12);

    let falling = scope_trigger_events(&waveform, 0, 1.0, ScopeTriggerEdge::Falling);
    assert_eq!(falling.len(), 1);
    assert!((falling[0].time_us - 1.5).abs() < 1.0e-12);
    assert_eq!(falling[0].edge, ScopeTriggerEdge::Falling);

    let either = scope_trigger_events(&waveform, 0, 1.0, ScopeTriggerEdge::Either);
    assert_eq!(either.len(), 3);
    assert_eq!(either[0].edge, ScopeTriggerEdge::Rising);
    assert_eq!(either[1].edge, ScopeTriggerEdge::Falling);
    assert_eq!(either[2].edge, ScopeTriggerEdge::Rising);
}

#[test]
fn scope_trigger_jump_selects_next_previous_with_wraparound() {
    let waveform = parse_waveform_csv_text(
        "time,v(out)\n0,0\n0.000001,2\n0.000002,0\n0.000003,3\n",
        "waveform.csv",
    )
    .unwrap();
    let events = scope_trigger_events(&waveform, 0, 1.0, ScopeTriggerEdge::Either);

    assert_eq!(
        select_scope_trigger_event(&events, 0.75, ScopeTriggerJump::Next)
            .unwrap()
            .time_us,
        1.5
    );
    assert_eq!(
        select_scope_trigger_event(&events, 0.75, ScopeTriggerJump::Previous)
            .unwrap()
            .time_us,
        0.5
    );
    assert_eq!(
        select_scope_trigger_event(&events, 4.0, ScopeTriggerJump::Next)
            .unwrap()
            .time_us,
        0.5
    );
    assert_eq!(
        select_scope_trigger_event(&events, 0.0, ScopeTriggerJump::Previous)
            .unwrap()
            .time_us,
        2.333333333333333
    );
}

#[test]
fn scope_trigger_event_rows_include_exact_times_values_and_delta() {
    let waveform =
        parse_waveform_csv_text("time,v(out)\n0,0\n0.000001,2\n0.000002,0\n", "waveform.csv")
            .unwrap();
    let events = scope_trigger_events(&waveform, 0, 1.0, ScopeTriggerEdge::Either);
    let rows = scope_trigger_event_rows(&events, 0.75);

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].index, 1);
    assert_eq!(rows[0].edge, "rising");
    assert!((rows[0].time_s - 0.5e-6).abs() < 1.0e-18);
    assert!((rows[0].value - 1.0).abs() < 1.0e-12);
    assert!((rows[0].delta_t_s - -0.25e-6).abs() < 1.0e-18);
    assert_eq!(rows[1].index, 2);
    assert_eq!(rows[1].edge, "falling");
    assert!((rows[1].time_s - 1.5e-6).abs() < 1.0e-18);
    assert!((rows[1].delta_t_s - 0.75e-6).abs() < 1.0e-18);
}
