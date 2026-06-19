use super::ScopeTriggerEdge;
use super::waveform_context::find_scope_probe;
use super::{
    WaveformMathDraft, WaveformPlotLaneMode, WaveformProbeQuantity, WaveformTracePreset,
    WaveformTraceRef, append_derived_waveform_probe, derived_waveform_quantity, interpolated_value,
    parse_waveform_csv_text, runtime_probe_activity_for_selection,
    runtime_probe_lines_for_selection, sanitized_probe_name, scope_cursor_legend_rows,
    scope_region_stats_rows, scope_snapshots_csv, scope_trace_lanes, scope_trigger_events,
    scope_visible_styled_trace_refs, scope_visible_trace_refs, waveform_measurement,
    waveform_probe_quantity_from_label, waveform_probe_value_for_badge,
    waveform_time_range_for_view, waveform_time_window_for_view, waveform_trace_bounds_in_window,
    zoom_time_window,
};
use super::{
    WaveformTraceColor, WaveformTraceStyle, clamp_value_window, expanded_value_bounds,
    scope_trace_color_for_style,
};
use crate::gui::sketch::{
    ProjectSnapshot, SketchComponent, SketchNet, SketchNodeStyle, SketchPin, SketchSelection,
};
use crate::gui::sketch_probes::{SketchProbe, SketchProbeQuantity, SketchProbeTarget};
use crate::gui::{CircuitCiApp, ScopeProbeTarget, SketchViewportCommand, Stage};

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
    app.capture_scope_region_stat_snapshots(&rows, 0.0, 2.0);

    let csv = scope_snapshots_csv(&app.waveform_measurement_snapshots);

    assert!(csv.starts_with(
        "label,source,trace,time_a_s,time_b_s,value_a_or_min,value_b_or_max,delta_or_mean,rms,event_edge,unit\n"
    ));
    assert!(csv.contains("Cursor 1,cursor selected,\"quoted, \"\"trace\"\"\""));
    assert!(csv.contains("Trigger 2,trigger rising,v(out),5.000000e-7"));
    assert!(csv.contains("Region 3,region selected,v(out),0.000000e0,2.000000e-6,0.000000e0,2.000000e0,1.000000e0,1.154701e0,,V"));
}

#[test]
fn scope_trace_lanes_split_visible_traces_by_inferred_unit() {
    let waveform = parse_waveform_csv_text(
        "time v(out) i(load) p(load) v(ref)
0.0 0.0 0.001 0.01 3.3
1e-6 2.0 0.003 0.02 3.2
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
        WaveformTraceRef {
            waveform_index: 0,
            probe_index: 2,
        },
        WaveformTraceRef {
            waveform_index: 0,
            probe_index: 3,
        },
    ];

    let lanes = scope_trace_lanes(&[waveform], &traces, WaveformPlotLaneMode::ByUnit);

    assert_eq!(lanes.len(), 3);
    assert_eq!(lanes[0].unit, "V");
    assert_eq!(lanes[0].traces, vec![traces[0], traces[3]]);
    assert_eq!(lanes[1].unit, "A");
    assert_eq!(lanes[1].traces, vec![traces[1]]);
    assert_eq!(lanes[2].unit, "W");
    assert_eq!(lanes[2].traces, vec![traces[2]]);
}

#[test]
fn scope_trace_lanes_shared_mode_keeps_mixed_overlay() {
    let waveform = parse_waveform_csv_text(
        "time v(out) i(load)
0.0 0.0 0.001
1e-6 2.0 0.003
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

    let lanes = scope_trace_lanes(&[waveform], &traces, WaveformPlotLaneMode::Shared);

    assert_eq!(lanes.len(), 1);
    assert_eq!(lanes[0].unit, "mixed");
    assert_eq!(lanes[0].traces, traces);
}

#[test]
fn derived_waveform_difference_is_selectable_probe() {
    let mut waveform = parse_waveform_csv_text(
        "time v(out) v(in)
0.0 0.0 5.0
1e-6 3.0 5.0
",
        "waveform.csv",
    )
    .unwrap();
    let index = append_derived_waveform_probe(
        &mut waveform,
        &WaveformMathDraft {
            left_probe: 1,
            right_probe: 0,
            operation: "difference".to_string(),
            label: "headroom".to_string(),
        },
    )
    .unwrap();
    assert_eq!(waveform.probes[index].label, "headroom");
    assert_eq!(waveform.probes[index].values, vec![5.0, 2.0]);
    assert!(waveform.probes[index].derived);
    let measurement = waveform_measurement(&waveform, index, 0.0, 1.0).unwrap();
    assert_eq!(measurement.full_min, 2.0);
    assert_eq!(measurement.full_max, 5.0);
}

#[test]
fn derived_waveform_product_composes_power_trace() {
    let mut waveform = parse_waveform_csv_text(
        "time v(load) i(load)
0.0 5.0 0.10
1e-6 4.0 0.25
",
        "waveform.csv",
    )
    .unwrap();
    let index = append_derived_waveform_probe(
        &mut waveform,
        &WaveformMathDraft {
            left_probe: 0,
            right_probe: 1,
            operation: "product".to_string(),
            label: String::new(),
        },
    )
    .unwrap();
    assert_eq!(waveform.probes[index].label, "v(load) * i(load)");
    assert_eq!(waveform.probes[index].values, vec![0.5, 1.0]);
    assert_eq!(
        waveform.probes[index].promoted_quantity,
        Some(WaveformProbeQuantity::Power)
    );
}

#[test]
fn derived_waveform_ratio_rejects_zero_denominator() {
    let mut waveform = parse_waveform_csv_text(
        "time v(out) i(load)
0.0 5.0 0.0
1e-6 4.0 0.25
",
        "waveform.csv",
    )
    .unwrap();
    let error = append_derived_waveform_probe(
        &mut waveform,
        &WaveformMathDraft {
            left_probe: 0,
            right_probe: 1,
            operation: "ratio".to_string(),
            label: "impedance".to_string(),
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("zero sample"));
}

#[test]
fn derived_waveform_quantity_infers_promotable_channels() {
    assert_eq!(
        waveform_probe_quantity_from_label("V(out)"),
        Some(WaveformProbeQuantity::Voltage)
    );
    assert_eq!(
        waveform_probe_quantity_from_label("I(VCCI_R1)"),
        Some(WaveformProbeQuantity::Current)
    );
    assert_eq!(
        derived_waveform_quantity(
            super::WaveformMathOperation::Difference,
            Some(WaveformProbeQuantity::Voltage),
            Some(WaveformProbeQuantity::Voltage),
        ),
        Some(WaveformProbeQuantity::Voltage)
    );
    assert_eq!(
        derived_waveform_quantity(
            super::WaveformMathOperation::Ratio,
            Some(WaveformProbeQuantity::Voltage),
            Some(WaveformProbeQuantity::Current),
        ),
        None
    );
    assert_eq!(sanitized_probe_name("V(out) * I(load)"), "V_out_I_load");
}

#[test]
fn runtime_probe_lines_match_hovered_net() {
    let waveform = parse_waveform_csv_text(
        "time v(out) i(R1)
0.0 0.0 0.001
1e-6 3.3 0.003
",
        "waveform.csv",
    )
    .unwrap();
    let snapshot = probe_snapshot();
    let lines = runtime_probe_lines_for_selection(
        &[waveform],
        0,
        0.5,
        &SketchSelection::Net("out".to_string()),
        &snapshot,
    );
    assert_eq!(lines.len(), 1);
    assert!(lines[0].contains("v(out)"));
    assert!(lines[0].contains("1.650000e0"));
}

#[test]
fn runtime_probe_lines_match_hovered_component() {
    let waveform = parse_waveform_csv_text(
        "time v(out) i(R1)
0.0 0.0 0.001
1e-6 3.3 0.003
",
        "waveform.csv",
    )
    .unwrap();
    let snapshot = probe_snapshot();
    let lines = runtime_probe_lines_for_selection(
        &[waveform],
        0,
        1.0,
        &SketchSelection::Component("R1".to_string()),
        &snapshot,
    );
    assert_eq!(lines.len(), 2);
    assert!(lines.iter().any(|line| line.contains("v(out)")));
    assert!(lines.iter().any(|line| line.contains("i(R1)")));
}

#[test]
fn runtime_probe_activity_normalizes_matching_probe_value() {
    let waveform = parse_waveform_csv_text(
        "time v(out) i(R1)
0.0 0.0 0.000
1e-6 3.0 0.003
",
        "waveform.csv",
    )
    .unwrap();
    let snapshot = probe_snapshot();
    let activity = runtime_probe_activity_for_selection(
        &[waveform],
        0,
        0.5,
        &SketchSelection::Net("out".to_string()),
        &snapshot,
    )
    .unwrap();
    assert!((activity - 0.5).abs() < 1.0e-12);
}

#[test]
fn runtime_probe_activity_ignores_unmatched_selection() {
    let waveform = parse_waveform_csv_text(
        "time v(other)
0.0 0.0
1e-6 3.0
",
        "waveform.csv",
    )
    .unwrap();
    let snapshot = probe_snapshot();
    let activity = runtime_probe_activity_for_selection(
        &[waveform],
        0,
        0.5,
        &SketchSelection::Net("out".to_string()),
        &snapshot,
    );
    assert_eq!(activity, None);
}

#[test]
fn waveform_time_range_for_view_returns_microseconds() {
    let waveform = parse_waveform_csv_text(
        "time v(out)
0.0 0.0
2e-6 1.0
",
        "waveform.csv",
    )
    .unwrap();
    assert_eq!(
        waveform_time_range_for_view(&[waveform], 0),
        Some((0.0, 2.0))
    );
}

#[test]
fn waveform_time_window_clamps_to_loaded_range() {
    let waveform = parse_waveform_csv_text(
        "time v(out)
0.0 0.0
1e-6 1.0
2e-6 2.0
",
        "waveform.csv",
    )
    .unwrap();

    assert_eq!(
        waveform_time_window_for_view(&[waveform], 0, Some(-5.0), Some(1.0)),
        Some((0.0, 2.0))
    );
}

#[test]
fn zoom_time_window_keeps_focus_ratio() {
    let (start, end) = zoom_time_window(0.0, 100.0, 25.0, 0.5);

    assert_eq!((start, end), (12.5, 62.5));
}

#[test]
fn value_window_expands_flat_traces_for_visible_y_scale() {
    let (min, max) = expanded_value_bounds(3.3, 3.3).unwrap();

    assert!(min < 3.3);
    assert!(max > 3.3);
    assert!((max - min - 0.33).abs() < 1.0e-12);
}

#[test]
fn value_window_clamps_zoomed_ranges_to_data_bounds() {
    assert_eq!(clamp_value_window(0.0, 10.0, 2.0, 8.0), Some((2.0, 8.0)));
    assert_eq!(clamp_value_window(0.0, 10.0, -4.0, 4.0), Some((0.0, 8.0)));
    assert_eq!(clamp_value_window(0.0, 10.0, 8.0, 14.0), Some((4.0, 10.0)));
}

#[test]
fn scope_view_history_restores_time_and_value_windows() {
    let waveform = parse_waveform_csv_text(
        "time v(out)
0.0 0.0
1e-6 2.0
2e-6 4.0
",
        "waveform.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        ..Default::default()
    };

    app.apply_waveform_view_change(|app| app.set_waveform_time_window(0.25, 1.75));
    app.apply_waveform_view_change(|app| app.set_waveform_value_window(1.0, 3.0));

    assert_eq!(app.waveform_view_back_stack.len(), 2);
    assert!(app.waveform_view_forward_stack.is_empty());
    assert_eq!(app.visible_waveform_time_window(), Some((0.25, 1.75)));
    assert_eq!(app.visible_waveform_value_window(), Some((1.0, 3.0)));

    app.restore_previous_waveform_view_window();
    assert_eq!(app.visible_waveform_time_window(), Some((0.25, 1.75)));
    assert_eq!(app.waveform_value_min, None);
    assert_eq!(app.waveform_value_max, None);
    assert_eq!(app.waveform_view_forward_stack.len(), 1);

    app.restore_previous_waveform_view_window();
    assert_eq!(app.visible_waveform_time_window(), Some((0.0, 2.0)));
    assert_eq!(app.waveform_view_forward_stack.len(), 2);

    app.restore_next_waveform_view_window();
    assert_eq!(app.visible_waveform_time_window(), Some((0.25, 1.75)));
    assert_eq!(app.waveform_value_min, None);
}

#[test]
fn scope_view_history_clears_forward_on_new_window_change() {
    let waveform = parse_waveform_csv_text(
        "time v(out)
0.0 0.0
1e-6 2.0
2e-6 4.0
",
        "waveform.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        ..Default::default()
    };

    app.apply_waveform_view_change(|app| app.set_waveform_time_window(0.25, 1.75));
    app.restore_previous_waveform_view_window();
    assert_eq!(app.waveform_view_forward_stack.len(), 1);

    app.apply_waveform_view_change(|app| app.set_waveform_time_window(0.5, 1.5));

    assert!(app.waveform_view_forward_stack.is_empty());
    assert_eq!(app.visible_waveform_time_window(), Some((0.5, 1.5)));
}

#[test]
fn scope_view_history_coalesces_plot_drag_windows() {
    let waveform = parse_waveform_csv_text(
        "time v(out)
0.0 0.0
1e-6 2.0
2e-6 4.0
",
        "waveform.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        ..Default::default()
    };

    app.waveform_view_drag_start = Some(app.waveform_view_window());
    app.set_waveform_time_window(0.25, 1.75);
    app.set_waveform_time_window(0.5, 1.5);
    app.commit_waveform_view_drag();

    assert_eq!(app.waveform_view_back_stack.len(), 1);
    assert_eq!(app.visible_waveform_time_window(), Some((0.5, 1.5)));

    app.restore_previous_waveform_view_window();

    assert_eq!(app.visible_waveform_time_window(), Some((0.0, 2.0)));
}

#[test]
fn waveform_trace_bounds_use_visible_window() {
    let waveform = parse_waveform_csv_text(
        "time v(out)
0.0 0.0
1e-6 10.0
2e-6 0.0
",
        "waveform.csv",
    )
    .unwrap();

    assert_eq!(
        waveform_trace_bounds_in_window(
            &[waveform],
            &[WaveformTraceRef {
                waveform_index: 0,
                probe_index: 0,
            }],
            0.0,
            0.5e-6,
        ),
        Some((0.0, 5.0))
    );
}

#[test]
fn scope_probe_selection_prefers_matching_scenario_path() {
    let first = parse_waveform_csv_text(
        "time out_voltage
0.0 1.0
1e-6 2.0
",
        "out/gui/other/waveform.csv",
    )
    .unwrap();
    let second = parse_waveform_csv_text(
        "time out_voltage current_probe
0.0 3.0 0.1
1e-6 4.0 0.2
",
        "out/gui/tran_main/waveform.csv",
    )
    .unwrap();

    assert_eq!(
        find_scope_probe(&[first, second], "tran_main", "out_voltage"),
        Some((1, 0))
    );
}

#[test]
fn pending_scope_probe_focus_selects_loaded_trace() {
    let waveform = parse_waveform_csv_text(
        "time out_voltage current_probe
0.0 1.0 0.1
1e-6 2.0 0.2
",
        "out/gui/tran_main/waveform.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        pending_scope_probe: Some(ScopeProbeTarget {
            scenario_name: "tran_main".to_string(),
            probe_name: "current_probe".to_string(),
        }),
        ..Default::default()
    };

    assert!(app.apply_pending_scope_probe_focus());
    assert_eq!(app.selected_waveform, 0);
    assert_eq!(app.selected_probe, 1);
}

#[test]
fn selected_scope_trace_focuses_originating_schematic_net() {
    let waveform = parse_waveform_csv_text(
        "time v(out)
0.0 1.0
1e-6 2.0
",
        "out/gui/tran_main/waveform.csv",
    )
    .unwrap();
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

    assert!(app.focus_selected_scope_schematic_context());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Net("out".to_string()))
    );
    assert!(
        app.selected_sketch_items
            .contains(&SketchSelection::Net("out".to_string()))
    );
    assert_eq!(
        app.pending_scope_probe,
        Some(ScopeProbeTarget {
            scenario_name: "tran_main".to_string(),
            probe_name: "out_voltage".to_string(),
        })
    );
}

#[test]
fn selected_scope_trace_focuses_originating_schematic_component() {
    let waveform = parse_waveform_csv_text(
        "time R1_power
0.0 0.1
1e-6 0.2
",
        "out/gui/tran_main/waveform.csv",
    )
    .unwrap();
    let mut snapshot = probe_snapshot();
    snapshot.probes.push(SketchProbe {
        scenario_name: "tran_main".to_string(),
        probe_name: "R1_power".to_string(),
        expression: "V(out)*I(VSENSE_R1)".to_string(),
        quantity: SketchProbeQuantity::Power,
        target: SketchProbeTarget::Component("R1".to_string()),
        assertion_names: Vec::new(),
    });
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        project_snapshot: Some(snapshot),
        selected_waveform: 0,
        selected_probe: 0,
        ..Default::default()
    };

    assert!(app.focus_selected_scope_schematic_context());
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("R1".to_string()))
    );
    assert!(
        app.selected_sketch_items
            .contains(&SketchSelection::Component("R1".to_string()))
    );
}

#[test]
fn scope_schematic_context_opens_sketch_with_selected_target() {
    let waveform = parse_waveform_csv_text(
        "time v(out)
0.0 1.0
1e-6 2.0
",
        "out/gui/tran_main/waveform.csv",
    )
    .unwrap();
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
        stage: Stage::Simulation,
        ..Default::default()
    };

    assert!(app.open_selected_scope_schematic_context(false));
    assert_eq!(app.stage, Stage::Sketch);
    assert_eq!(app.sketch_viewport_command, None);
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Net("out".to_string()))
    );
}

#[test]
fn scope_schematic_context_fit_queues_sketch_fit_selection() {
    let waveform = parse_waveform_csv_text(
        "time R1_power
0.0 0.1
1e-6 0.2
",
        "out/gui/tran_main/waveform.csv",
    )
    .unwrap();
    let mut snapshot = probe_snapshot();
    snapshot.probes.push(SketchProbe {
        scenario_name: "tran_main".to_string(),
        probe_name: "R1_power".to_string(),
        expression: "V(out)*I(VSENSE_R1)".to_string(),
        quantity: SketchProbeQuantity::Power,
        target: SketchProbeTarget::Component("R1".to_string()),
        assertion_names: Vec::new(),
    });
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        project_snapshot: Some(snapshot),
        stage: Stage::Simulation,
        ..Default::default()
    };

    assert!(app.open_selected_scope_schematic_context(true));
    assert_eq!(app.stage, Stage::Sketch);
    assert_eq!(
        app.sketch_viewport_command,
        Some(SketchViewportCommand::FitSelection)
    );
    assert_eq!(
        app.selected_sketch_item,
        Some(SketchSelection::Component("R1".to_string()))
    );
}

#[test]
fn scope_visible_traces_keep_selected_first_and_dedupe_pins() {
    let waveform = parse_waveform_csv_text(
        "time out_voltage current_probe aux_probe
0.0 1.0 0.1 5.0
1e-6 2.0 0.2 6.0
",
        "out/gui/tran_main/waveform.csv",
    )
    .unwrap();
    let pinned = vec![
        WaveformTraceRef {
            waveform_index: 0,
            probe_index: 1,
        },
        WaveformTraceRef {
            waveform_index: 0,
            probe_index: 0,
        },
        WaveformTraceRef {
            waveform_index: 99,
            probe_index: 0,
        },
    ];

    assert_eq!(
        scope_visible_trace_refs(&[waveform], 0, 0, &pinned),
        vec![
            WaveformTraceRef {
                waveform_index: 0,
                probe_index: 0,
            },
            WaveformTraceRef {
                waveform_index: 0,
                probe_index: 1,
            },
        ]
    );
}

#[test]
fn pinned_scope_trace_pruning_drops_invalid_loaded_refs() {
    let waveform = parse_waveform_csv_text(
        "time out_voltage current_probe
0.0 1.0 0.1
1e-6 2.0 0.2
",
        "out/gui/tran_main/waveform.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        waveform_pinned_traces: vec![
            WaveformTraceRef {
                waveform_index: 0,
                probe_index: 1,
            },
            WaveformTraceRef {
                waveform_index: 0,
                probe_index: 9,
            },
            WaveformTraceRef {
                waveform_index: 9,
                probe_index: 0,
            },
        ],
        ..Default::default()
    };

    app.prune_scope_trace_pins();

    assert_eq!(
        app.waveform_pinned_traces,
        vec![WaveformTraceRef {
            waveform_index: 0,
            probe_index: 1,
        }]
    );
}

#[test]
fn scope_trace_styles_hide_pinned_traces_but_keep_active_trace_visible() {
    let traces = vec![
        WaveformTraceRef {
            waveform_index: 0,
            probe_index: 0,
        },
        WaveformTraceRef {
            waveform_index: 0,
            probe_index: 1,
        },
        WaveformTraceRef {
            waveform_index: 0,
            probe_index: 2,
        },
    ];
    let styles = vec![
        WaveformTraceStyle {
            trace: traces[0],
            color: None,
            visible: false,
        },
        WaveformTraceStyle {
            trace: traces[1],
            color: None,
            visible: false,
        },
    ];

    assert_eq!(
        scope_visible_styled_trace_refs(&traces, &styles),
        vec![traces[0], traces[2]]
    );
}

#[test]
fn scope_trace_style_color_overrides_auto_palette() {
    let trace = WaveformTraceRef {
        waveform_index: 0,
        probe_index: 1,
    };
    let styles = vec![WaveformTraceStyle {
        trace,
        color: Some(WaveformTraceColor::Red),
        visible: true,
    }];

    assert_eq!(
        scope_trace_color_for_style(1, trace, &styles),
        WaveformTraceColor::Red.color()
    );
}

#[test]
fn scope_trace_style_pruning_drops_invalid_loaded_refs() {
    let waveform = parse_waveform_csv_text(
        "time out_voltage current_probe
0.0 1.0 0.1
1e-6 2.0 0.2
",
        "out/gui/tran_main/waveform.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        waveform_trace_styles: vec![
            WaveformTraceStyle {
                trace: WaveformTraceRef {
                    waveform_index: 0,
                    probe_index: 1,
                },
                color: Some(WaveformTraceColor::Green),
                visible: true,
            },
            WaveformTraceStyle {
                trace: WaveformTraceRef {
                    waveform_index: 0,
                    probe_index: 9,
                },
                color: Some(WaveformTraceColor::Red),
                visible: true,
            },
        ],
        ..Default::default()
    };

    app.prune_scope_trace_pins();

    assert_eq!(
        app.waveform_trace_styles,
        vec![WaveformTraceStyle {
            trace: WaveformTraceRef {
                waveform_index: 0,
                probe_index: 1,
            },
            color: Some(WaveformTraceColor::Green),
            visible: true,
        }]
    );
}

#[test]
fn pinned_scope_refs_shift_after_probe_removal() {
    let mut app = CircuitCiApp {
        waveform_pinned_traces: vec![
            WaveformTraceRef {
                waveform_index: 0,
                probe_index: 1,
            },
            WaveformTraceRef {
                waveform_index: 0,
                probe_index: 3,
            },
            WaveformTraceRef {
                waveform_index: 1,
                probe_index: 3,
            },
        ],
        ..Default::default()
    };

    app.shift_scope_trace_pins_after_probe_removal(0, 1);

    assert_eq!(
        app.waveform_pinned_traces,
        vec![
            WaveformTraceRef {
                waveform_index: 0,
                probe_index: 2,
            },
            WaveformTraceRef {
                waveform_index: 1,
                probe_index: 3,
            },
        ]
    );
}

#[test]
fn scope_trace_styles_shift_after_probe_removal() {
    let mut app = CircuitCiApp {
        waveform_trace_styles: vec![
            WaveformTraceStyle {
                trace: WaveformTraceRef {
                    waveform_index: 0,
                    probe_index: 1,
                },
                color: Some(WaveformTraceColor::Blue),
                visible: true,
            },
            WaveformTraceStyle {
                trace: WaveformTraceRef {
                    waveform_index: 0,
                    probe_index: 3,
                },
                color: None,
                visible: false,
            },
            WaveformTraceStyle {
                trace: WaveformTraceRef {
                    waveform_index: 1,
                    probe_index: 3,
                },
                color: Some(WaveformTraceColor::Cyan),
                visible: true,
            },
        ],
        ..Default::default()
    };

    app.shift_scope_trace_styles_after_probe_removal(0, 1);

    assert_eq!(
        app.waveform_trace_styles,
        vec![
            WaveformTraceStyle {
                trace: WaveformTraceRef {
                    waveform_index: 0,
                    probe_index: 2,
                },
                color: None,
                visible: false,
            },
            WaveformTraceStyle {
                trace: WaveformTraceRef {
                    waveform_index: 1,
                    probe_index: 3,
                },
                color: Some(WaveformTraceColor::Cyan),
                visible: true,
            },
        ]
    );
}

#[test]
fn scope_compare_preset_saves_replaces_and_restores_traces() {
    let waveform = parse_waveform_csv_text(
        "time,v(out),i(load),v(ref)\n0,1,0.1,2\n0.000001,2,0.2,3\n",
        "waveform.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        selected_probe: 0,
        waveform_pinned_traces: vec![WaveformTraceRef {
            waveform_index: 0,
            probe_index: 1,
        }],
        waveform_trace_preset_name: "startup".to_string(),
        ..Default::default()
    };

    app.save_current_scope_compare_preset(app.current_scope_compare_traces());

    assert_eq!(
        app.waveform_trace_presets,
        vec![WaveformTracePreset {
            name: "startup".to_string(),
            traces: vec![
                WaveformTraceRef {
                    waveform_index: 0,
                    probe_index: 0,
                },
                WaveformTraceRef {
                    waveform_index: 0,
                    probe_index: 1,
                },
            ],
        }]
    );

    app.selected_probe = 2;
    app.waveform_pinned_traces.clear();
    app.apply_scope_compare_preset(0);

    assert_eq!(app.selected_probe, 0);
    assert_eq!(
        app.waveform_pinned_traces,
        vec![WaveformTraceRef {
            waveform_index: 0,
            probe_index: 1,
        }]
    );

    app.selected_probe = 2;
    app.waveform_pinned_traces.clear();
    app.waveform_trace_preset_name = "startup".to_string();
    app.save_current_scope_compare_preset(app.current_scope_compare_traces());

    assert_eq!(app.waveform_trace_presets.len(), 1);
    assert_eq!(
        app.waveform_trace_presets[0].traces,
        vec![WaveformTraceRef {
            waveform_index: 0,
            probe_index: 2,
        }]
    );
}

#[test]
fn scope_compare_preset_load_prunes_stale_traces() {
    let waveform = parse_waveform_csv_text(
        "time,v(out),i(load)\n0,1,0.1\n0.000001,2,0.2\n",
        "waveform.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        selected_probe: 1,
        waveform_pinned_traces: vec![WaveformTraceRef {
            waveform_index: 0,
            probe_index: 0,
        }],
        waveform_trace_presets: vec![WaveformTracePreset {
            name: "valid subset".to_string(),
            traces: vec![
                WaveformTraceRef {
                    waveform_index: 0,
                    probe_index: 0,
                },
                WaveformTraceRef {
                    waveform_index: 0,
                    probe_index: 9,
                },
                WaveformTraceRef {
                    waveform_index: 0,
                    probe_index: 1,
                },
            ],
        }],
        ..Default::default()
    };

    app.apply_scope_compare_preset(0);

    assert_eq!(app.selected_probe, 0);
    assert_eq!(
        app.waveform_pinned_traces,
        vec![WaveformTraceRef {
            waveform_index: 0,
            probe_index: 1,
        }]
    );
}

#[test]
fn scope_compare_presets_shift_after_probe_removal() {
    let mut app = CircuitCiApp {
        waveform_trace_presets: vec![
            WaveformTracePreset {
                name: "drop removed".to_string(),
                traces: vec![
                    WaveformTraceRef {
                        waveform_index: 0,
                        probe_index: 1,
                    },
                    WaveformTraceRef {
                        waveform_index: 0,
                        probe_index: 3,
                    },
                ],
            },
            WaveformTracePreset {
                name: "other waveform".to_string(),
                traces: vec![WaveformTraceRef {
                    waveform_index: 1,
                    probe_index: 3,
                }],
            },
        ],
        ..Default::default()
    };

    app.shift_scope_trace_presets_after_probe_removal(0, 1);

    assert_eq!(
        app.waveform_trace_presets,
        vec![
            WaveformTracePreset {
                name: "drop removed".to_string(),
                traces: vec![WaveformTraceRef {
                    waveform_index: 0,
                    probe_index: 2,
                }],
            },
            WaveformTracePreset {
                name: "other waveform".to_string(),
                traces: vec![WaveformTraceRef {
                    waveform_index: 1,
                    probe_index: 3,
                }],
            },
        ]
    );
}

fn probe_snapshot() -> ProjectSnapshot {
    ProjectSnapshot {
        name: "probe_graph".to_string(),
        components: 1,
        nets: 2,
        scenarios: 1,
        libraries: Vec::new(),
        components_detail: vec![SketchComponent {
            id: "R1".to_string(),
            model: "generic.analog.resistor".to_string(),
            part_number: None,
            spice: None,
            position: None,
            pins: vec![
                SketchPin {
                    pin: "A".to_string(),
                    net: "out".to_string(),
                },
                SketchPin {
                    pin: "B".to_string(),
                    net: "gnd".to_string(),
                },
            ],
            style: SketchNodeStyle::default(),
            source_paths: Vec::new(),
        }],
        nets_detail: vec![
            SketchNet {
                id: "out".to_string(),
                kind: "digital_or_analog".to_string(),
                nominal_voltage: None,
                powered: None,
                connections: vec!["R1.A".to_string()],
                position: None,
            },
            SketchNet {
                id: "gnd".to_string(),
                kind: "ground".to_string(),
                nominal_voltage: Some(0.0),
                powered: Some(true),
                connections: vec!["R1.B".to_string()],
                position: None,
            },
        ],
        probes: Vec::new(),
        wire_routes: Default::default(),
        net_labels: Default::default(),
        component_labels: Default::default(),
    }
}
