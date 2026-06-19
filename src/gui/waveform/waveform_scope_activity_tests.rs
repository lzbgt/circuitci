use super::{parse_waveform_csv_text, scope_snapshots_csv, scope_snapshots_markdown};
use crate::gui::sketch_canvas_hits::RuntimeScopeActivityTarget;
use crate::gui::{CircuitCiApp, ScopeProbeTarget};
use std::f64::consts::PI;
use std::fs;
use std::path::Path;

#[test]
fn scope_activity_sample_snapshot_captures_target_trace_at_cursor() {
    let waveform =
        parse_waveform_csv_text("time,v(out),i(load)\n0,0,0.1\n0.000001,2,0.3\n", "startup")
            .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        waveform_cursor_a_us: 0.5,
        ..Default::default()
    };

    assert!(
        app.capture_scope_activity_sample_snapshot(ScopeProbeTarget {
            scenario_name: "startup".to_string(),
            probe_name: "i(load)".to_string(),
        })
    );

    assert_eq!(app.waveform_measurement_snapshots.len(), 1);
    let snapshot = &app.waveform_measurement_snapshots[0];
    assert_eq!(snapshot.label, "Scope Activity 1");
    assert_eq!(snapshot.source, "scope activity");
    assert_eq!(snapshot.trace_label, "i(load)");
    assert_eq!(snapshot.time_a_us, Some(0.5));
    assert_eq!(snapshot.time_b_us, None);
    assert!((snapshot.value_a.unwrap() - 0.2).abs() < 1.0e-12);
    assert_eq!(snapshot.unit, "A");
    assert!(
        app.status
            .contains("Captured Scope Activity sample for i(load)")
    );
}

#[test]
fn scope_activity_sample_snapshot_reports_missing_target() {
    let waveform = parse_waveform_csv_text("time,v(out)\n0,0\n0.000001,2\n", "startup").unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        waveform_cursor_a_us: 0.5,
        ..Default::default()
    };

    assert!(
        !app.capture_scope_activity_sample_snapshot(ScopeProbeTarget {
            scenario_name: "startup".to_string(),
            probe_name: "i(load)".to_string(),
        })
    );

    assert!(app.waveform_measurement_snapshots.is_empty());
    assert!(app.status.contains("is not loaded yet"));
}

#[test]
fn scope_activity_frequency_snapshot_captures_peak_and_period() {
    let samples = (0..128)
        .map(|index| {
            let time_s = index as f64 / 64_000.0;
            let value = (2.0 * PI * 1_000.0 * time_s).sin();
            format!("{time_s:.9},{value:.9}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let waveform =
        parse_waveform_csv_text(&format!("time,v(out)\n{samples}\n"), "startup").unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        ..Default::default()
    };

    assert!(
        app.capture_scope_activity_frequency_snapshot(ScopeProbeTarget {
            scenario_name: "startup".to_string(),
            probe_name: "v(out)".to_string(),
        })
    );

    assert_eq!(app.waveform_measurement_snapshots.len(), 1);
    let snapshot = &app.waveform_measurement_snapshots[0];
    assert_eq!(snapshot.label, "Scope Activity Freq 1");
    assert_eq!(snapshot.source, "scope activity frequency");
    assert_eq!(snapshot.trace_label, "v(out)");
    assert!(snapshot.time_a_us.is_none());
    assert!((snapshot.value_a.unwrap() - 1_000.0).abs() < 80.0);
    assert!((snapshot.value_b.unwrap() - (1.0 / snapshot.value_a.unwrap())).abs() < 1.0e-12);
    assert_eq!(snapshot.unit, "Hz / s");
    assert!(
        app.status
            .contains("Captured Scope Activity frequency for v(out)")
    );
}

#[test]
fn scope_activity_snap_visible_frequency_captures_available_targets() {
    let rows = (0..128)
        .map(|index| {
            let time_s = index as f64 / 64_000.0;
            let value = (2.0 * PI * 1_000.0 * time_s).sin();
            format!("{time_s:.9},{value:.9},1.0")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let waveform =
        parse_waveform_csv_text(&format!("time,v(out),v(flat)\n{rows}\n"), "startup").unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        ..Default::default()
    };
    let targets = vec![
        RuntimeScopeActivityTarget {
            label: "out".to_string(),
            target: ScopeProbeTarget {
                scenario_name: "startup".to_string(),
                probe_name: "v(out)".to_string(),
            },
        },
        RuntimeScopeActivityTarget {
            label: "flat".to_string(),
            target: ScopeProbeTarget {
                scenario_name: "startup".to_string(),
                probe_name: "v(flat)".to_string(),
            },
        },
    ];

    assert_eq!(
        app.capture_visible_scope_activity_frequency_snapshots_from_sketch(&targets, &[0, 1]),
        1
    );

    assert_eq!(app.waveform_measurement_snapshots.len(), 1);
    assert_eq!(
        app.waveform_measurement_snapshots[0].source,
        "scope activity frequency"
    );
    assert_eq!(
        app.status,
        "Captured 1 visible Scope Activity frequency snapshot(s); 1 unavailable."
    );
}

#[test]
fn scope_activity_visible_observation_rows_include_samples_and_frequencies() {
    let rows = (0..128)
        .map(|index| {
            let time_s = index as f64 / 64_000.0;
            let value = (2.0 * PI * 1_000.0 * time_s).sin();
            format!("{time_s:.9},{value:.9},1.0")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let waveform =
        parse_waveform_csv_text(&format!("time,v(out),v(flat)\n{rows}\n"), "startup").unwrap();
    let app = CircuitCiApp {
        waveforms: vec![waveform],
        waveform_cursor_a_us: 500.0,
        ..Default::default()
    };
    let targets = vec![
        RuntimeScopeActivityTarget {
            label: "out".to_string(),
            target: ScopeProbeTarget {
                scenario_name: "startup".to_string(),
                probe_name: "v(out)".to_string(),
            },
        },
        RuntimeScopeActivityTarget {
            label: "flat".to_string(),
            target: ScopeProbeTarget {
                scenario_name: "startup".to_string(),
                probe_name: "v(flat)".to_string(),
            },
        },
    ];

    let rows = app.visible_scope_activity_observation_snapshots(&targets, &[0, 1]);

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].source, "scope activity");
    assert_eq!(rows[0].trace_label, "v(out)");
    assert_eq!(rows[1].source, "scope activity frequency");
    assert_eq!(rows[1].trace_label, "v(out)");
    assert_eq!(rows[2].source, "scope activity");
    assert_eq!(rows[2].trace_label, "v(flat)");
    assert!(app.waveform_measurement_snapshots.is_empty());

    let csv = scope_snapshots_csv(&rows);
    assert!(csv.contains("scope activity,v(out)"));
    assert!(csv.contains("scope activity frequency,v(out)"));
    assert!(csv.contains("scope activity,v(flat)"));
    let markdown = scope_snapshots_markdown(&rows);
    assert!(markdown.contains("| Scope Activity 1 |"));
    assert!(markdown.contains("| Scope Activity Freq 2 |"));
}

#[test]
fn visible_scope_activity_report_bundle_exports_filtered_observations() {
    let rows = (0..128)
        .map(|index| {
            let time_s = index as f64 / 64_000.0;
            let out = (2.0 * PI * 1_000.0 * time_s).sin();
            let flat = 1.0;
            format!("{time_s:.9},{out:.9},{flat:.9}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let waveform =
        parse_waveform_csv_text(&format!("time,v(out),v(flat)\n{rows}\n"), "startup").unwrap();
    let base_dir = std::env::temp_dir().join(format!(
        "circuitci_scope_activity_visible_bundle_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&base_dir);
    fs::create_dir_all(&base_dir).unwrap();
    let mut app = CircuitCiApp {
        output_dir: base_dir.to_string_lossy().into_owned(),
        waveforms: vec![waveform],
        waveform_cursor_a_us: 500.0,
        ..Default::default()
    };
    let targets = vec![
        RuntimeScopeActivityTarget {
            label: "out".to_string(),
            target: ScopeProbeTarget {
                scenario_name: "startup".to_string(),
                probe_name: "v(out)".to_string(),
            },
        },
        RuntimeScopeActivityTarget {
            label: "flat".to_string(),
            target: ScopeProbeTarget {
                scenario_name: "startup".to_string(),
                probe_name: "v(flat)".to_string(),
            },
        },
    ];

    assert_eq!(
        app.export_visible_scope_activity_report_bundle(&targets, &[0]),
        2
    );

    let bundle = app.waveform_recent_report_bundles.first().unwrap();
    let csv = fs::read_to_string(Path::new(bundle).join("measurement_snapshots.csv")).unwrap();
    let markdown = fs::read_to_string(Path::new(bundle).join("measurement_snapshots.md")).unwrap();
    assert!(csv.contains("scope activity,v(out)"));
    assert!(csv.contains("scope activity frequency,v(out)"));
    assert!(!csv.contains("v(flat)"));
    assert!(markdown.contains("| Scope Activity 1 |"));
    assert!(markdown.contains("| Scope Activity Freq 2 |"));
    assert!(Path::new(bundle).join("index.html").exists());
    assert!(Path::new(bundle).join("artifact_manifest.csv").exists());
    assert!(app.waveform_measurement_snapshots.is_empty());
    assert!(
        app.status
            .contains("Exported visible Scope Activity report bundle with 2 observation row(s)")
    );

    fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn scope_activity_recent_bundle_rows_prune_and_label_existing_bundles() {
    let base_dir = std::env::temp_dir().join(format!(
        "circuitci_scope_activity_recent_bundles_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&base_dir);
    let latest = base_dir.join("scope_report_bundle_latest");
    let older = base_dir.join("scope_report_bundle_older");
    let missing = base_dir.join("scope_report_bundle_missing");
    fs::create_dir_all(&latest).unwrap();
    fs::create_dir_all(&older).unwrap();
    let mut app = CircuitCiApp {
        waveform_recent_report_bundles: vec![
            latest.to_string_lossy().into_owned(),
            missing.to_string_lossy().into_owned(),
            older.to_string_lossy().into_owned(),
        ],
        ..Default::default()
    };

    let rows = app.scope_activity_recent_bundle_rows();

    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].label, "scope_report_bundle_latest");
    assert_eq!(rows[0].path, latest.to_string_lossy().into_owned());
    assert_eq!(rows[1].label, "scope_report_bundle_older");
    assert_eq!(
        app.waveform_recent_report_bundles,
        vec![
            latest.to_string_lossy().into_owned(),
            older.to_string_lossy().into_owned()
        ]
    );
    assert!(
        app.status
            .contains("Pruned 1 stale scope report bundle entry(s)")
    );

    fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn scope_activity_target_observation_rows_copy_one_trace_only() {
    let rows = (0..128)
        .map(|index| {
            let time_s = index as f64 / 64_000.0;
            let value = (2.0 * PI * 1_000.0 * time_s).sin();
            format!("{time_s:.9},{value:.9},1.0")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let waveform =
        parse_waveform_csv_text(&format!("time,v(out),v(flat)\n{rows}\n"), "startup").unwrap();
    let app = CircuitCiApp {
        waveforms: vec![waveform],
        waveform_cursor_a_us: 500.0,
        ..Default::default()
    };

    let out_rows = app.scope_activity_target_observation_snapshots(&ScopeProbeTarget {
        scenario_name: "startup".to_string(),
        probe_name: "v(out)".to_string(),
    });
    assert_eq!(out_rows.len(), 2);
    assert_eq!(out_rows[0].source, "scope activity");
    assert_eq!(out_rows[1].source, "scope activity frequency");
    assert!(out_rows.iter().all(|row| row.trace_label == "v(out)"));

    let flat_rows = app.scope_activity_target_observation_snapshots(&ScopeProbeTarget {
        scenario_name: "startup".to_string(),
        probe_name: "v(flat)".to_string(),
    });
    assert_eq!(flat_rows.len(), 1);
    assert_eq!(flat_rows[0].source, "scope activity");
    assert_eq!(flat_rows[0].trace_label, "v(flat)");

    assert!(app.waveform_measurement_snapshots.is_empty());
}

#[test]
fn scope_activity_target_report_bundle_exports_one_trace_observations() {
    let samples = (0..128)
        .map(|index| {
            let time_s = index as f64 / 64_000.0;
            let out = (2.0 * PI * 1_000.0 * time_s).sin();
            let timing = (2.0 * PI * 500.0 * time_s).sin();
            format!("{time_s:.9},{out:.9},{timing:.9}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let waveform =
        parse_waveform_csv_text(&format!("time,v(out),v(timing)\n{samples}\n"), "startup").unwrap();
    let base_dir = std::env::temp_dir().join(format!(
        "circuitci_scope_activity_row_bundle_test_{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&base_dir);
    fs::create_dir_all(&base_dir).unwrap();
    let mut app = CircuitCiApp {
        output_dir: base_dir.to_string_lossy().into_owned(),
        waveforms: vec![waveform],
        selected_probe: 0,
        waveform_cursor_a_us: 500.0,
        ..Default::default()
    };

    assert_eq!(
        app.export_scope_activity_target_report_bundle(ScopeProbeTarget {
            scenario_name: "startup".to_string(),
            probe_name: "v(out)".to_string(),
        }),
        2
    );

    let mut bundles = fs::read_dir(&base_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    bundles.sort();
    assert_eq!(bundles.len(), 1);
    let bundle = &bundles[0];
    let csv = fs::read_to_string(bundle.join("measurement_snapshots.csv")).unwrap();
    let markdown = fs::read_to_string(bundle.join("measurement_snapshots.md")).unwrap();
    assert!(csv.contains("scope activity,v(out)"));
    assert!(csv.contains("scope activity frequency,v(out)"));
    assert!(!csv.contains("v(timing)"));
    assert!(markdown.contains("| Scope Activity 1 |"));
    assert!(markdown.contains("| Scope Activity Freq 2 |"));
    assert!(bundle.join("index.html").exists());
    assert!(bundle.join("artifact_manifest.csv").exists());
    assert!(app.waveform_measurement_snapshots.is_empty());
    assert_eq!(app.waveform_recent_report_bundles.len(), 1);
    assert!(
        app.status
            .contains("Exported Scope Activity report bundle with 2 observation row(s) for v(out)")
    );

    fs::remove_dir_all(&base_dir).unwrap();
}

#[test]
fn scope_activity_target_report_bundle_and_open_handles_empty_observations() {
    let mut app = CircuitCiApp::default();

    assert_eq!(
        app.export_scope_activity_target_report_bundle_and_open(ScopeProbeTarget {
            scenario_name: "startup".to_string(),
            probe_name: "v(out)".to_string(),
        }),
        0
    );

    assert!(app.waveform_recent_report_bundles.is_empty());
    assert!(
        app.status
            .contains("No Scope Activity observations are available to bundle for v(out)")
    );
}

#[test]
fn scope_activity_snap_visible_captures_current_visible_targets() {
    let waveform = parse_waveform_csv_text(
        "time,v(out),v(timing),i(load)\n0,0,1,0.1\n0.000001,2,3,0.3\n",
        "startup",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        waveform_cursor_a_us: 0.5,
        ..Default::default()
    };
    let targets = vec![
        RuntimeScopeActivityTarget {
            label: "out".to_string(),
            target: ScopeProbeTarget {
                scenario_name: "startup".to_string(),
                probe_name: "v(out)".to_string(),
            },
        },
        RuntimeScopeActivityTarget {
            label: "timing".to_string(),
            target: ScopeProbeTarget {
                scenario_name: "startup".to_string(),
                probe_name: "v(timing)".to_string(),
            },
        },
        RuntimeScopeActivityTarget {
            label: "load".to_string(),
            target: ScopeProbeTarget {
                scenario_name: "startup".to_string(),
                probe_name: "i(load)".to_string(),
            },
        },
    ];

    assert_eq!(
        app.capture_visible_scope_activity_snapshots_from_sketch(&targets, &[0, 2]),
        2
    );

    assert_eq!(app.waveform_measurement_snapshots.len(), 2);
    assert_eq!(app.waveform_measurement_snapshots[0].trace_label, "v(out)");
    assert_eq!(app.waveform_measurement_snapshots[1].trace_label, "i(load)");
    assert_eq!(app.waveform_measurement_snapshots[0].value_a, Some(1.0));
    assert!((app.waveform_measurement_snapshots[1].value_a.unwrap() - 0.2).abs() < 1.0e-12);
    assert_eq!(
        app.status,
        "Captured 2 visible Scope Activity sample snapshot(s)."
    );
}
