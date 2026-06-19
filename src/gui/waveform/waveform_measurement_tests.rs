use super::waveform_test_support::probe_snapshot;
use super::{
    ScopeSnapshotGroupMode, ScopeSnapshotSortKey, ScopeSnapshotSourceFilter, ScopeTriggerEdge,
    WaveformTraceRef, cleanup_old_scope_report_bundle_dirs, interpolated_value,
    old_scope_report_bundle_dirs, parse_waveform_csv_text, scope_cursor_legend_rows,
    scope_region_stats_rows, scope_report_bundle_artifact_detail_rows,
    scope_report_bundle_changed_artifacts, scope_report_bundle_index_path,
    scope_report_bundle_integrity_details, scope_report_bundle_integrity_details_csv,
    scope_report_bundle_integrity_details_markdown, scope_report_bundle_missing_artifacts,
    scope_snapshot_visible_indexes, scope_snapshot_visible_indexes_sorted, scope_snapshots_csv,
    scope_snapshots_markdown, scope_trigger_events, scope_visible_trace_refs,
    unique_scope_report_bundle_dir, waveform_measurement, waveform_probe_value_for_badge,
};
use crate::gui::sketch::SketchSelection;
use crate::gui::sketch_probes::{SketchProbe, SketchProbeQuantity, SketchProbeTarget};
use crate::gui::{CircuitCiApp, ScopeProbeTarget};
use std::fs;

#[test]
fn scope_cursor_snapshots_capture_selected_and_pinned_traces() {
    let waveform = parse_waveform_csv_text(
        "time,v(out),i(load)\n0,0,0.1\n0.000001,2,0.3\n",
        "waveform.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        selected_probe: 0,
        waveform_cursor_a_us: 0.0,
        waveform_cursor_b_us: 1.0,
        waveform_pinned_traces: vec![WaveformTraceRef {
            waveform_index: 0,
            probe_index: 1,
        }],
        ..Default::default()
    };

    app.capture_scope_cursor_snapshots();

    assert_eq!(app.waveform_measurement_snapshots.len(), 2);
    let selected = &app.waveform_measurement_snapshots[0];
    assert_eq!(selected.label, "Cursor 1");
    assert_eq!(selected.source, "cursor selected");
    assert_eq!(
        selected.trace,
        Some(WaveformTraceRef {
            waveform_index: 0,
            probe_index: 0,
        })
    );
    assert_eq!(selected.trace_label, "v(out)");
    assert_eq!(selected.unit, "V");
    assert_eq!(selected.time_a_us, Some(0.0));
    assert_eq!(selected.time_b_us, Some(1.0));
    assert_eq!(selected.value_a, Some(0.0));
    assert_eq!(selected.value_b, Some(2.0));
    assert_eq!(selected.delta_value, Some(2.0));

    let pinned = &app.waveform_measurement_snapshots[1];
    assert_eq!(pinned.source, "cursor pinned");
    assert_eq!(
        pinned.trace,
        Some(WaveformTraceRef {
            waveform_index: 0,
            probe_index: 1,
        })
    );
    assert_eq!(pinned.trace_label, "i(load)");
    assert_eq!(pinned.unit, "A");
    assert!((pinned.delta_value.unwrap() - 0.2).abs() < 1.0e-12);
}

#[test]
fn scope_trigger_snapshots_capture_selected_event() {
    let waveform =
        parse_waveform_csv_text("time,v(out)\n0,0\n0.000001,2\n0.000002,0\n", "waveform.csv")
            .unwrap();
    let event = scope_trigger_events(&waveform, 0, 1.0, ScopeTriggerEdge::Rising)[0];
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        selected_probe: 0,
        ..Default::default()
    };

    app.capture_scope_trigger_snapshot(event);

    assert_eq!(app.waveform_measurement_snapshots.len(), 1);
    let snapshot = &app.waveform_measurement_snapshots[0];
    assert_eq!(snapshot.label, "Trigger 1");
    assert_eq!(snapshot.source, "trigger");
    assert_eq!(
        snapshot.trace,
        Some(WaveformTraceRef {
            waveform_index: 0,
            probe_index: 0,
        })
    );
    assert_eq!(snapshot.trace_label, "v(out)");
    assert_eq!(snapshot.event_edge.as_deref(), Some("rising"));
    assert_eq!(snapshot.time_a_us, Some(0.5));
    assert_eq!(snapshot.value_a, Some(1.0));
    assert_eq!(snapshot.unit, "V");
}

#[test]
fn scope_snapshot_jump_restores_trace_cursors_and_time_window() {
    let waveform = parse_waveform_csv_text(
        "time,v(out),i(load)\n0,0,0.1\n0.000001,2,0.3\n0.000002,0,0.1\n",
        "waveform.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        selected_probe: 0,
        waveform_cursor_a_us: 0.0,
        waveform_cursor_b_us: 2.0,
        waveform_pinned_traces: vec![WaveformTraceRef {
            waveform_index: 0,
            probe_index: 1,
        }],
        ..Default::default()
    };
    app.capture_scope_cursor_snapshots();
    app.selected_probe = 0;
    app.waveform_cursor_a_us = 0.0;
    app.waveform_cursor_b_us = 0.0;
    app.set_waveform_time_window(0.0, 2.0);

    assert!(app.activate_scope_measurement_snapshot(1, false));

    assert_eq!(app.selected_probe, 1);
    assert_eq!(app.waveform_cursor_a_us, 0.0);
    assert_eq!(app.waveform_cursor_b_us, 2.0);
    assert_eq!(app.visible_waveform_time_window(), Some((0.0, 2.0)));
    assert_eq!(app.status, "Restored scope snapshot Cursor 1.");
}

#[test]
fn scope_snapshot_focus_selects_originating_schematic_context() {
    let waveform = parse_waveform_csv_text(
        "time,v(out)\n0,0\n0.000001,2\n0.000002,0\n",
        "out/gui/tran_main/waveform.csv",
    )
    .unwrap();
    let event = scope_trigger_events(&waveform, 0, 1.0, ScopeTriggerEdge::Rising)[0];
    let mut snapshot = probe_snapshot();
    snapshot.probes.push(SketchProbe {
        scenario_name: "tran_main".to_string(),
        probe_name: "out_voltage".to_string(),
        expression: "V(out)".to_string(),
        quantity: SketchProbeQuantity::Voltage,
        target: SketchProbeTarget::Net("out".to_string()),
        assertion_names: Vec::new(),
    });
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        project_snapshot: Some(snapshot),
        selected_waveform: 0,
        selected_probe: 0,
        ..Default::default()
    };
    app.capture_scope_trigger_snapshot(event);

    assert!(app.activate_scope_measurement_snapshot(0, true));

    assert_eq!(app.selected_probe, 0);
    assert_eq!(app.waveform_cursor_a_us, 0.5);
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Net("out".to_string()))
    );
    assert_eq!(
        app.pending_scope_probe,
        Some(ScopeProbeTarget {
            scenario_name: "tran_main".to_string(),
            probe_name: "out_voltage".to_string(),
        })
    );
    assert_eq!(
        app.status,
        "Focused schematic context for scope snapshot Trigger 1."
    );
}

#[test]
fn scope_snapshot_markers_follow_visible_traces_only() {
    let waveform = parse_waveform_csv_text(
        "time,v(out),i(load)\n0,0,0.1\n0.000001,2,0.3\n",
        "waveform.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        selected_probe: 0,
        waveform_cursor_a_us: 0.0,
        waveform_cursor_b_us: 1.0,
        waveform_pinned_traces: vec![WaveformTraceRef {
            waveform_index: 0,
            probe_index: 1,
        }],
        ..Default::default()
    };
    app.capture_scope_cursor_snapshots();
    let selected_trace = WaveformTraceRef {
        waveform_index: 0,
        probe_index: 0,
    };
    let pinned_trace = WaveformTraceRef {
        waveform_index: 0,
        probe_index: 1,
    };

    let selected_only = app.scope_snapshot_markers(&[selected_trace]);
    assert_eq!(selected_only.len(), 1);
    assert_eq!(selected_only[0].trace, selected_trace);
    assert_eq!(selected_only[0].label, "Cursor 1");
    assert_eq!(selected_only[0].time_a_us, Some(0.0));
    assert_eq!(selected_only[0].time_b_us, Some(1.0));

    let both = app.scope_snapshot_markers(&[selected_trace, pinned_trace]);
    assert_eq!(both.len(), 2);
    assert_eq!(both[1].trace, pinned_trace);
    assert_eq!(both[1].source, "cursor pinned");
}

#[test]
fn waveform_parser_rejects_non_increasing_time() {
    let error = parse_waveform_csv_text(
        "time v(out)
1e-6 1.0
1e-6 2.0
",
        "waveform.csv",
    )
    .unwrap_err();
    assert!(error.to_string().contains("non-increasing time"));
}

#[test]
fn interpolation_returns_linear_value_between_samples() {
    let value = interpolated_value(&[0.0, 1.0e-6, 2.0e-6], &[0.0, 2.0, 4.0], 1.5e-6).unwrap();
    assert!((value - 3.0).abs() < 1.0e-12);
}

#[test]
fn waveform_probe_value_for_badge_matches_probe_expression() {
    let waveform = parse_waveform_csv_text(
        "time v(out)
0.0 0.0
1e-6 3.3
",
        "waveform.csv",
    )
    .unwrap();
    let probe = SketchProbe {
        scenario_name: "gui_transient".to_string(),
        probe_name: "out_voltage".to_string(),
        expression: "V(out)".to_string(),
        quantity: SketchProbeQuantity::Voltage,
        target: SketchProbeTarget::Net("out".to_string()),
        assertion_names: Vec::new(),
    };
    let value = waveform_probe_value_for_badge(&[waveform], 0, 0.5, &probe).unwrap();
    assert!((value - 1.65).abs() < 1.0e-12);
}

#[test]
fn quick_assertion_margin_is_relative_with_zero_floor() {
    assert!((super::quick_assertion_margin(5.0) - 0.05).abs() < 1.0e-12);
    assert_eq!(super::quick_assertion_margin(0.0), 1.0e-9);
}

#[test]
fn waveform_measurement_reports_cursor_delta_and_ranges() {
    let waveform = parse_waveform_csv_text(
        "time v(out)
0.0 0.0
1e-6 2.0
2e-6 1.0
",
        "waveform.csv",
    )
    .unwrap();
    let measurement = waveform_measurement(&waveform, 0, 0.5, 1.5).unwrap();
    assert!((measurement.cursor_a.value - 1.0).abs() < 1.0e-12);
    assert!((measurement.cursor_b.value - 1.5).abs() < 1.0e-12);
    assert!((measurement.delta_t_s - 1.0e-6).abs() < 1.0e-18);
    assert!((measurement.delta_value - 0.5).abs() < 1.0e-12);
    assert_eq!(measurement.full_min, 0.0);
    assert_eq!(measurement.full_max, 2.0);
    assert_eq!(measurement.window_max, 2.0);
}

#[test]
fn scope_cursor_legend_rows_include_selected_and_pinned_traces() {
    let waveform = parse_waveform_csv_text(
        "time v(out) i(load)
0.0 0.0 0.001
1e-6 2.0 0.003
2e-6 4.0 0.005
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
    let rows = scope_cursor_legend_rows(&[waveform], &traces, 0.5, 1.5);
    assert_eq!(rows.len(), 2);
    assert!(rows[0].selected);
    assert!(!rows[1].selected);
    assert_eq!(rows[0].label, "v(out)");
    assert_eq!(rows[0].unit, "V");
    assert!((rows[0].cursor_a_value - 1.0).abs() < 1.0e-12);
    assert!((rows[0].cursor_b_value - 3.0).abs() < 1.0e-12);
    assert!((rows[0].delta_value - 2.0).abs() < 1.0e-12);
    assert_eq!(rows[1].label, "i(load)");
    assert_eq!(rows[1].unit, "A");
    assert!((rows[1].cursor_a_value - 0.002).abs() < 1.0e-12);
    assert!((rows[1].cursor_b_value - 0.004).abs() < 1.0e-12);
}

#[test]
fn scope_region_stats_rows_compute_time_weighted_statistics() {
    let waveform = parse_waveform_csv_text(
        "time v(out) i(load)
0.0 0.0 0.001
1e-6 2.0 0.003
2e-6 0.0 0.005
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

    let rows = scope_region_stats_rows(&[waveform], &traces, 0.0, 2.0);

    assert_eq!(rows.len(), 2);
    assert!(rows[0].selected);
    assert!(!rows[1].selected);
    assert_eq!(rows[0].label, "v(out)");
    assert_eq!(rows[0].unit, "V");
    assert!((rows[0].min - 0.0).abs() < 1.0e-12);
    assert!((rows[0].max - 2.0).abs() < 1.0e-12);
    assert!((rows[0].mean - 1.0).abs() < 1.0e-12);
    assert!((rows[0].rms - (4.0_f64 / 3.0).sqrt()).abs() < 1.0e-12);
    assert_eq!(rows[1].label, "i(load)");
    assert_eq!(rows[1].unit, "A");
    assert!((rows[1].mean - 0.003).abs() < 1.0e-12);
    assert!((rows[1].rms - (31.0e-6_f64 / 3.0).sqrt()).abs() < 1.0e-15);
}

#[test]
fn scope_region_stats_rows_include_interpolated_region_edges() {
    let waveform = parse_waveform_csv_text(
        "time v(out)
0.0 0.0
1e-6 2.0
2e-6 0.0
",
        "scope.csv",
    )
    .unwrap();
    let traces = vec![WaveformTraceRef {
        waveform_index: 0,
        probe_index: 0,
    }];

    let rows = scope_region_stats_rows(&[waveform], &traces, 0.5, 1.5);

    assert_eq!(rows.len(), 1);
    assert!((rows[0].min - 1.0).abs() < 1.0e-12);
    assert!((rows[0].max - 2.0).abs() < 1.0e-12);
    assert!((rows[0].mean - 1.5).abs() < 1.0e-12);
    assert!((rows[0].rms - (7.0_f64 / 3.0).sqrt()).abs() < 1.0e-12);
}

#[test]
fn scope_region_stat_snapshots_capture_stats_and_restore_context() {
    let waveform = parse_waveform_csv_text(
        "time v(out) i(load)
0.0 0.0 0.001
1e-6 2.0 0.003
2e-6 0.0 0.005
",
        "scope.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        selected_probe: 0,
        waveform_cursor_a_us: 0.0,
        waveform_cursor_b_us: 2.0,
        waveform_pinned_traces: vec![WaveformTraceRef {
            waveform_index: 0,
            probe_index: 1,
        }],
        ..Default::default()
    };
    let traces = scope_visible_trace_refs(
        &app.waveforms,
        app.selected_waveform,
        app.selected_probe,
        &app.waveform_pinned_traces,
    );
    let rows = scope_region_stats_rows(&app.waveforms, &traces, 0.0, 2.0);

    app.capture_scope_region_stat_snapshots(&rows, 0.0, 2.0);

    assert_eq!(app.waveform_measurement_snapshots.len(), 2);
    let selected = &app.waveform_measurement_snapshots[0];
    assert_eq!(selected.label, "Region 1");
    assert_eq!(selected.source, "region selected");
    assert_eq!(
        selected.trace,
        Some(WaveformTraceRef {
            waveform_index: 0,
            probe_index: 0,
        })
    );
    assert_eq!(selected.trace_label, "v(out)");
    assert_eq!(selected.time_a_us, Some(0.0));
    assert_eq!(selected.time_b_us, Some(2.0));
    assert_eq!(selected.value_a, Some(0.0));
    assert_eq!(selected.value_b, Some(2.0));
    assert!((selected.delta_value.unwrap() - 1.0).abs() < 1.0e-12);
    assert!((selected.rms_value.unwrap() - (4.0_f64 / 3.0).sqrt()).abs() < 1.0e-12);

    let pinned = &app.waveform_measurement_snapshots[1];
    assert_eq!(pinned.source, "region pinned");
    assert_eq!(pinned.trace_label, "i(load)");
    assert!((pinned.rms_value.unwrap() - (31.0e-6_f64 / 3.0).sqrt()).abs() < 1.0e-15);

    app.selected_probe = 0;
    app.waveform_cursor_a_us = 0.0;
    app.waveform_cursor_b_us = 0.0;
    app.set_waveform_time_window(0.0, 1.0);

    assert!(app.activate_scope_measurement_snapshot(1, false));

    assert_eq!(app.selected_probe, 1);
    assert_eq!(app.waveform_cursor_a_us, 0.0);
    assert_eq!(app.waveform_cursor_b_us, 2.0);
    assert_eq!(app.visible_waveform_time_window(), Some((0.0, 2.0)));
    assert_eq!(app.status, "Restored scope snapshot Region 1.");
}

#[test]
fn scope_snapshot_csv_exports_cursor_trigger_and_region_rows() {
    let waveform = parse_waveform_csv_text(
        "time v(out) i(load)
0.0 0.0 0.001
1e-6 2.0 0.003
2e-6 0.0 0.005
",
        "scope.csv",
    )
    .unwrap();
    let event = scope_trigger_events(&waveform, 0, 1.0, ScopeTriggerEdge::Rising)[0];
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        selected_probe: 0,
        waveform_cursor_a_us: 0.0,
        waveform_cursor_b_us: 2.0,
        ..Default::default()
    };
    app.capture_scope_cursor_snapshots();
    app.capture_scope_trigger_snapshot(event);
    let traces = scope_visible_trace_refs(
        &app.waveforms,
        app.selected_waveform,
        app.selected_probe,
        &app.waveform_pinned_traces,
    );
    let rows = scope_region_stats_rows(&app.waveforms, &traces, 0.0, 2.0);
    app.waveform_measurement_snapshots[0].trace_label = "quoted, \"trace\"".to_string();
    app.waveform_measurement_snapshots[0].note = "settling, \"fast\"".to_string();
    app.capture_scope_region_stat_snapshots(&rows, 0.0, 2.0);

    let csv = scope_snapshots_csv(&app.waveform_measurement_snapshots);

    assert!(csv.starts_with(
        "label,note,source,trace,time_a_s,time_b_s,value_a_or_min,value_b_or_max,delta_or_mean,rms,event_edge,unit\n"
    ));
    assert!(
        csv.contains(
            "Cursor 1,\"settling, \"\"fast\"\"\",cursor selected,\"quoted, \"\"trace\"\"\""
        )
    );
    assert!(csv.contains("Trigger 2,,trigger rising,v(out),5.000000e-7"));
    assert!(csv.contains("Region 3,,region selected,v(out),0.000000e0,2.000000e-6,0.000000e0,2.000000e0,1.000000e0,1.154701e0,,V"));
}

#[test]
fn scope_snapshot_markdown_exports_report_table_rows() {
    let waveform = parse_waveform_csv_text(
        "time v(out)
0.0 0.0
1e-6 2.0
",
        "scope.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        selected_probe: 0,
        waveform_cursor_a_us: 0.0,
        waveform_cursor_b_us: 1.0,
        ..Default::default()
    };
    app.capture_scope_cursor_snapshots();
    app.waveform_measurement_snapshots[0].label = "Startup | edge".to_string();
    app.waveform_measurement_snapshots[0].note = "settles\nquickly".to_string();

    let markdown = scope_snapshots_markdown(&app.waveform_measurement_snapshots);

    assert!(markdown.starts_with("## Scope Measurement Snapshots\n\n"));
    assert!(markdown.contains(
        "| Label | Note | Source | Trace | A/Event/Min | B/Max | Delta/Mean | RMS | Unit |"
    ));
    assert!(markdown.contains(
        "| Startup \\| edge | settles<br>quickly | cursor selected | v(out) | 0.000000e0 @ 0.000000e0 s | 2.000000e0 @ 1.000000e-6 s | 2.000000e0 | - | V |"
    ));
    assert_eq!(
        scope_snapshots_markdown(&[]),
        "## Scope Measurement Snapshots\n\n_No measurement snapshots matched the current filters._\n"
    );
}

#[test]
fn scope_snapshot_filters_match_source_and_text_for_visible_rows() {
    let waveform = parse_waveform_csv_text(
        "time v(out) i(load)
0.0 0.0 0.001
1e-6 2.0 0.003
2e-6 0.0 0.005
",
        "scope.csv",
    )
    .unwrap();
    let event = scope_trigger_events(&waveform, 0, 1.0, ScopeTriggerEdge::Rising)[0];
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        selected_probe: 0,
        waveform_cursor_a_us: 0.0,
        waveform_cursor_b_us: 2.0,
        waveform_pinned_traces: vec![WaveformTraceRef {
            waveform_index: 0,
            probe_index: 1,
        }],
        ..Default::default()
    };
    app.capture_scope_cursor_snapshots();
    app.capture_scope_trigger_snapshot(event);
    let traces = scope_visible_trace_refs(
        &app.waveforms,
        app.selected_waveform,
        app.selected_probe,
        &app.waveform_pinned_traces,
    );
    let rows = scope_region_stats_rows(&app.waveforms, &traces, 0.0, 2.0);
    app.capture_scope_region_stat_snapshots(&rows, 0.0, 2.0);
    app.waveform_measurement_snapshots[4].note = "load channel observed".to_string();

    assert_eq!(
        scope_snapshot_visible_indexes(
            &app.waveform_measurement_snapshots,
            "",
            ScopeSnapshotSourceFilter::All,
        ),
        vec![0, 1, 2, 3, 4]
    );
    assert_eq!(
        scope_snapshot_visible_indexes(
            &app.waveform_measurement_snapshots,
            "",
            ScopeSnapshotSourceFilter::Cursor,
        ),
        vec![0, 1]
    );
    assert_eq!(
        scope_snapshot_visible_indexes(
            &app.waveform_measurement_snapshots,
            "load",
            ScopeSnapshotSourceFilter::All,
        ),
        vec![1, 4]
    );
    assert_eq!(
        scope_snapshot_visible_indexes(
            &app.waveform_measurement_snapshots,
            "load",
            ScopeSnapshotSourceFilter::Region,
        ),
        vec![4]
    );
    assert_eq!(
        scope_snapshot_visible_indexes(
            &app.waveform_measurement_snapshots,
            "observed",
            ScopeSnapshotSourceFilter::All,
        ),
        vec![4]
    );

    app.waveform_snapshot_filter = "load".to_string();
    app.waveform_snapshot_source_filter = ScopeSnapshotSourceFilter::Region;
    let visible = app.visible_scope_measurement_snapshot_indexes();
    let filtered: Vec<_> = visible
        .iter()
        .map(|&index| app.waveform_measurement_snapshots[index].clone())
        .collect();
    let csv = scope_snapshots_csv(&filtered);

    assert!(csv.contains("Region 4,load channel observed,region pinned,i(load)"));
    assert!(!csv.contains("v(out)"));
    assert_eq!(csv.lines().count(), 2);
}

#[test]
fn scope_snapshot_sorting_and_grouping_shape_visible_exports() {
    let waveform = parse_waveform_csv_text(
        "time v(out) i(load)
0.0 0.0 0.001
1e-6 2.0 0.003
2e-6 0.0 0.005
",
        "scope.csv",
    )
    .unwrap();
    let event = scope_trigger_events(&waveform, 0, 1.0, ScopeTriggerEdge::Rising)[0];
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        selected_probe: 0,
        waveform_cursor_a_us: 0.0,
        waveform_cursor_b_us: 2.0,
        waveform_pinned_traces: vec![WaveformTraceRef {
            waveform_index: 0,
            probe_index: 1,
        }],
        waveform_snapshot_sort_key: ScopeSnapshotSortKey::Trace,
        waveform_snapshot_group_mode: ScopeSnapshotGroupMode::Unit,
        ..Default::default()
    };
    app.capture_scope_cursor_snapshots();
    app.capture_scope_trigger_snapshot(event);
    let traces = scope_visible_trace_refs(
        &app.waveforms,
        app.selected_waveform,
        app.selected_probe,
        &app.waveform_pinned_traces,
    );
    let rows = scope_region_stats_rows(&app.waveforms, &traces, 0.0, 2.0);
    app.capture_scope_region_stat_snapshots(&rows, 0.0, 2.0);

    assert_eq!(
        scope_snapshot_visible_indexes_sorted(
            &app.waveform_measurement_snapshots,
            "",
            ScopeSnapshotSourceFilter::All,
            ScopeSnapshotSortKey::Trace,
            ScopeSnapshotGroupMode::Unit,
        ),
        vec![1, 4, 0, 2, 3]
    );
    let visible = app.visible_scope_measurement_snapshot_indexes();
    let filtered: Vec<_> = visible
        .iter()
        .map(|&index| app.waveform_measurement_snapshots[index].clone())
        .collect();
    let csv = scope_snapshots_csv(&filtered);
    let labels: Vec<_> = csv
        .lines()
        .skip(1)
        .filter_map(|line| line.split(',').next())
        .collect();

    assert_eq!(
        labels,
        vec!["Cursor 1", "Region 4", "Cursor 1", "Trigger 3", "Region 4"]
    );
    assert!(scope_snapshots_markdown(&filtered).contains("| Region 4 |"));
    assert_eq!(
        scope_snapshot_visible_indexes_sorted(
            &app.waveform_measurement_snapshots,
            "",
            ScopeSnapshotSourceFilter::All,
            ScopeSnapshotSortKey::Newest,
            ScopeSnapshotGroupMode::None,
        ),
        vec![4, 3, 2, 1, 0]
    );
}

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
    assert!(readme.contains("- `artifact_manifest.csv`"));
    assert!(readme.contains("- `artifact_integrity_details.csv`"));
    assert!(readme.contains("- `artifact_integrity_details.md`"));
    assert!(readme.contains("optional"));
    assert!(readme.contains("## Artifact Metadata"));
    assert!(readme.contains("| scope_plot.svg |"));
    assert!(readme.contains("| measurement_snapshots.csv |"));
    assert!(readme.contains("SHA-256"));
    assert!(index.contains("<title>CircuitCI Scope Report Bundle</title>"));
    assert!(index.contains("href=\"scope_plot.svg\""));
    assert!(index.contains("href=\"measurement_snapshots.csv\""));
    assert!(index.contains("href=\"measurement_snapshots.md\""));
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
