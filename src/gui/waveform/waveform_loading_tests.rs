use super::merge_waveform_load_diagnostics;
use super::{
    WaveformFootprintSortKey, WaveformLoadDiagnostic, WaveformLoadPreviewFilter,
    WaveformLoadRequest, WaveformLoadStatusFilter, WaveformTraceColor, WaveformTracePreset,
    WaveformTraceRef, WaveformTraceStyle,
};
use super::{
    clear_deferred_waveform_column_picks,
    deferred_waveform_artifact_filtered_unloaded_probe_labels,
    deferred_waveform_artifact_picked_probe_labels,
    deferred_waveform_artifact_unloaded_probe_labels, deferred_waveform_artifact_visible_indexes,
    deferred_waveform_matching_probe_requests, deferred_waveform_remaining_probe_requests,
    load_report_waveforms_with_progress_and_cancel, load_waveform_csv_with_progress_and_cancel,
    load_waveform_paths_with_progress_and_cancel, load_waveform_requests_with_progress_and_cancel,
    parse_waveform_csv_text, select_deferred_waveform_column_picks, waveform_footprint_csv,
    waveform_footprint_largest_unload_targets, waveform_footprint_rows,
    waveform_footprint_unload_targets, waveform_load_deferred_artifacts,
    waveform_load_deferred_paths, waveform_load_diagnostic_unloaded_preview_columns,
    waveform_load_diagnostic_visible_indexes, waveform_load_diagnostics_csv,
    waveform_load_preflight,
};
use crate::gui::CircuitCiApp;
use std::collections::BTreeSet;
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
fn waveform_footprint_csv_exports_visible_rows() {
    let plain =
        parse_waveform_csv_text("time,v(out)\n0,1\n0.000001,2\n", "/tmp/run/plain.csv").unwrap();
    let quoted = parse_waveform_csv_text(
        "time,v(out),i(load)\n0,1,0.1\n0.000001,2,0.2\n",
        "/tmp/run/quoted,\"scope\".csv",
    )
    .unwrap();
    let rows = waveform_footprint_rows(
        &[plain, quoted],
        "scope",
        WaveformFootprintSortKey::EstimatedBytes,
        true,
    );
    let csv = waveform_footprint_csv(&rows);

    assert!(
        csv.starts_with("waveform,path,samples,probes,values,estimated_bytes,estimated_size\n")
    );
    assert!(csv.contains("\"/tmp/run/quoted,\"\"scope\"\".csv\""));
    assert!(csv.contains(",2,2,6,48,48 B\n"));
    assert!(!csv.contains("plain.csv"));
}

#[test]
fn waveform_footprint_bulk_unload_uses_preview_targets() {
    let small_path = "/tmp/run/small.csv";
    let large_path = "/tmp/run/large.csv";
    let extra_path = "/tmp/run/extra.csv";
    let small = parse_waveform_csv_text("time,v(out)\n0,1\n0.000001,2\n", small_path).unwrap();
    let large = parse_waveform_csv_text(
        "time,v(bus),i(load),temp\n0,12,0.1,25\n0.000001,11,0.2,26\n0.000002,10,0.3,27\n",
        large_path,
    )
    .unwrap();
    let extra = parse_waveform_csv_text("time,v(ref)\n0,3.3\n0.000001,3.2\n", extra_path).unwrap();
    let rows = waveform_footprint_rows(
        &[small.clone(), large.clone(), extra.clone()],
        "load",
        WaveformFootprintSortKey::EstimatedBytes,
        true,
    );
    let targets = waveform_footprint_unload_targets(&rows);
    let mut app = CircuitCiApp {
        waveforms: vec![small, large, extra],
        waveform_load_diagnostics: vec![
            WaveformLoadDiagnostic::loaded(small_path.to_string(), Some(128), 2, 1, 1),
            WaveformLoadDiagnostic::loaded(large_path.to_string(), Some(512), 3, 3, 2),
            WaveformLoadDiagnostic::loaded(extra_path.to_string(), Some(128), 2, 1, 1),
        ],
        selected_waveform: 2,
        waveform_pinned_traces: vec![WaveformTraceRef {
            waveform_index: 2,
            probe_index: 0,
        }],
        ..Default::default()
    };

    let removed = app.unload_waveform_footprint_targets(&targets);

    assert_eq!(removed, 1);
    assert_eq!(app.waveforms.len(), 2);
    assert_eq!(app.waveforms[0].path, small_path);
    assert_eq!(app.waveforms[1].path, extra_path);
    assert_eq!(app.selected_waveform, 1);
    assert_eq!(
        app.waveform_pinned_traces,
        vec![WaveformTraceRef {
            waveform_index: 1,
            probe_index: 0,
        }]
    );
    let artifacts = waveform_load_deferred_artifacts(&app.waveform_load_diagnostics);
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].path, large_path);

    assert_eq!(app.unload_waveform_footprint_targets(&targets), 0);
}

#[test]
fn waveform_footprint_largest_unload_targets_reduce_to_budget() {
    let small_path = "/tmp/run/small.csv";
    let medium_path = "/tmp/run/medium.csv";
    let large_path = "/tmp/run/large.csv";
    let small = parse_waveform_csv_text("time,v(out)\n0,1\n0.000001,2\n", small_path).unwrap();
    let medium = parse_waveform_csv_text(
        "time,v(out),i(load)\n0,1,0.1\n0.000001,2,0.2\n",
        medium_path,
    )
    .unwrap();
    let large = parse_waveform_csv_text(
        "time,v(bus),i(load),temp\n0,12,0.1,25\n0.000001,11,0.2,26\n0.000002,10,0.3,27\n",
        large_path,
    )
    .unwrap();
    let rows = waveform_footprint_rows(
        &[small.clone(), medium.clone(), large.clone()],
        "",
        WaveformFootprintSortKey::EstimatedBytes,
        true,
    );
    let total_bytes = rows.iter().map(|row| row.estimated_bytes).sum::<usize>();
    let budget_bytes = total_bytes - rows[0].estimated_bytes;
    let targets = waveform_footprint_largest_unload_targets(&rows, budget_bytes, total_bytes);
    let mut app = CircuitCiApp {
        waveforms: vec![small, medium, large],
        waveform_load_diagnostics: vec![
            WaveformLoadDiagnostic::loaded(small_path.to_string(), Some(128), 2, 1, 1),
            WaveformLoadDiagnostic::loaded(medium_path.to_string(), Some(256), 2, 2, 2),
            WaveformLoadDiagnostic::loaded(large_path.to_string(), Some(512), 3, 3, 2),
        ],
        selected_waveform: 0,
        ..Default::default()
    };

    assert_eq!(targets.len(), 1);
    assert_eq!(app.unload_waveform_footprint_targets(&targets), 1);
    assert_eq!(
        app.waveforms
            .iter()
            .map(|waveform| waveform.path.as_str())
            .collect::<Vec<_>>(),
        vec![small_path, medium_path]
    );
}
