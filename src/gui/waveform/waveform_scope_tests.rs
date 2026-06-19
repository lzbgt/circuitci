use super::waveform_context::find_scope_probe;
use super::waveform_test_support::probe_snapshot;
use super::{RuntimeScopeProbeEdgeStep, clamp_value_window, expanded_value_bounds};
use super::{
    WaveformMathDraft, WaveformPlotLaneMode, WaveformProbeQuantity, WaveformTraceRef,
    append_derived_waveform_probe, derived_waveform_quantity, parse_waveform_csv_text,
    runtime_probe_activity_for_selection, runtime_probe_lines_for_selection,
    runtime_scope_probe_edge_jump, runtime_scope_probe_sample_label,
    runtime_scope_probe_sparkline_points, runtime_scope_probe_target_for_selection,
    sanitized_probe_name, scope_trace_lanes, waveform_measurement,
    waveform_probe_quantity_from_label, waveform_time_range_for_view,
    waveform_time_window_for_view, waveform_trace_bounds_in_window, zoom_time_window,
};
use crate::gui::sketch::{
    SketchSelection, SketchViewport, layout_sketch_graph_viewport, runtime_scope_chip_rect,
};
use crate::gui::sketch_canvas_hits::{
    SketchCanvasHitContext, hover_targets, runtime_scope_activity_targets,
};
use crate::gui::sketch_probes::{SketchProbe, SketchProbeQuantity, SketchProbeTarget};
use crate::gui::{CircuitCiApp, ScopeProbeTarget, SketchViewportCommand, Stage};
use eframe::egui;

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
fn runtime_scope_probe_target_matches_loaded_node_trace() {
    let waveform = parse_waveform_csv_text(
        "time v(out) i(R1)
0.0 0.0 0.001
1e-6 3.3 0.003
",
        "scope/run/gui_transient/waveform.csv",
    )
    .unwrap();
    let snapshot = probe_snapshot();

    assert_eq!(
        runtime_scope_probe_target_for_selection(
            std::slice::from_ref(&waveform),
            0,
            &SketchSelection::Net("out".to_string()),
            &snapshot,
        ),
        Some(ScopeProbeTarget {
            scenario_name: "scope/run/gui_transient/waveform.csv".to_string(),
            probe_name: "v(out)".to_string(),
        })
    );
    assert_eq!(
        runtime_scope_probe_target_for_selection(
            &[waveform],
            0,
            &SketchSelection::Component("R1".to_string()),
            &snapshot,
        ),
        Some(ScopeProbeTarget {
            scenario_name: "scope/run/gui_transient/waveform.csv".to_string(),
            probe_name: "i(R1)".to_string(),
        })
    );
}

#[test]
fn runtime_scope_overlay_visibility_gates_chip_hover_but_keeps_activity_count() {
    let waveform = parse_waveform_csv_text(
        "time v(out) i(R1)
0.0 0.0 0.001
1e-6 3.3 0.003
",
        "scope/run/gui_transient/waveform.csv",
    )
    .unwrap();
    let snapshot = probe_snapshot();
    let graph = layout_sketch_graph_viewport(
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(500.0, 320.0)),
        &snapshot,
        SketchViewport {
            pan: egui::Vec2::ZERO,
            zoom: 1.0,
        },
    );
    let component = graph
        .nodes
        .iter()
        .find(|node| node.selection == SketchSelection::Component("R1".to_string()))
        .unwrap();
    let chip_center = runtime_scope_chip_rect(component).center();
    let visible_context = SketchCanvasHitContext {
        graph: &graph,
        hierarchy_view: None,
        bundle_badges: &[],
        hierarchy_connector_badges: &[],
        net_label_badges: &[],
        component_label_badges: &[],
        minimap: None,
        waveforms: std::slice::from_ref(&waveform),
        selected_waveform: 0,
        waveform_cursor_a_us: 0.5,
        snapshot: &snapshot,
        runtime_scope_overlay_visible: true,
    };

    let targets = runtime_scope_activity_targets(&visible_context);
    assert_eq!(targets.len(), 2);
    assert_eq!(
        targets
            .iter()
            .map(|row| (row.label.as_str(), row.target.probe_name.as_str()))
            .collect::<Vec<_>>(),
        vec![("R1", "i(R1)"), ("out", "v(out)")]
    );
    assert_eq!(
        hover_targets(&visible_context, Some(chip_center))
            .runtime_scope_node
            .map(|node| &node.selection),
        Some(&SketchSelection::Component("R1".to_string()))
    );

    let hidden_context = SketchCanvasHitContext {
        runtime_scope_overlay_visible: false,
        ..visible_context
    };

    assert_eq!(runtime_scope_activity_targets(&hidden_context).len(), 2);
    assert!(
        hover_targets(&hidden_context, Some(chip_center))
            .runtime_scope_node
            .is_none()
    );
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
fn runtime_scope_probe_sample_label_reports_value_unit_and_time() {
    let waveform = parse_waveform_csv_text(
        "time v(out) i(R1)
0.0 0.0 0.001
1e-6 3.3 0.003
",
        "waveform.csv",
    )
    .unwrap();
    let sample = runtime_scope_probe_sample_label(
        &[waveform],
        0,
        0.5,
        &ScopeProbeTarget {
            scenario_name: "waveform.csv".to_string(),
            probe_name: "v(out)".to_string(),
        },
    )
    .unwrap();

    assert!(sample.contains("1.650000e0"));
    assert!(sample.contains("V"));
    assert!(sample.contains("5.000000e-7 s"));
}

#[test]
fn runtime_scope_probe_sparkline_points_normalize_trace_shape() {
    let waveform = parse_waveform_csv_text(
        "time v(out) v(flat)
0.0 0.0 2.5
1e-6 1.0 2.5
2e-6 0.0 2.5
",
        "waveform.csv",
    )
    .unwrap();
    let waveforms = [waveform];
    let target = ScopeProbeTarget {
        scenario_name: "waveform.csv".to_string(),
        probe_name: "v(out)".to_string(),
    };
    let flat_target = ScopeProbeTarget {
        scenario_name: "waveform.csv".to_string(),
        probe_name: "v(flat)".to_string(),
    };

    let points = runtime_scope_probe_sparkline_points(&waveforms, 0, &target, 5).unwrap();
    let flat_points = runtime_scope_probe_sparkline_points(&waveforms, 0, &flat_target, 5).unwrap();

    assert_eq!(points.len(), 5);
    assert_eq!(points[0], (0.0, 0.0));
    assert_eq!(points[2], (0.5, 1.0));
    assert_eq!(points[4], (1.0, 0.0));
    assert!(
        flat_points
            .iter()
            .all(|(_, y)| (*y - 0.5).abs() < f32::EPSILON)
    );
}

#[test]
fn runtime_scope_probe_edge_jump_selects_previous_next_and_wraps() {
    let waveform = parse_waveform_csv_text(
        "time v(out)
0.0 0.0
1e-6 2.0
2e-6 0.0
3e-6 2.0
",
        "waveform.csv",
    )
    .unwrap();
    let target = ScopeProbeTarget {
        scenario_name: "waveform.csv".to_string(),
        probe_name: "v(out)".to_string(),
    };
    let waveforms = [waveform];

    let next = runtime_scope_probe_edge_jump(
        &waveforms,
        0,
        0.75,
        &target,
        RuntimeScopeProbeEdgeStep::Next,
    )
    .unwrap();
    let previous = runtime_scope_probe_edge_jump(
        &waveforms,
        0,
        0.75,
        &target,
        RuntimeScopeProbeEdgeStep::Previous,
    )
    .unwrap();
    let wrapped_next =
        runtime_scope_probe_edge_jump(&waveforms, 0, 4.0, &target, RuntimeScopeProbeEdgeStep::Next)
            .unwrap();

    assert_eq!(next.time_us, 1.5);
    assert_eq!(previous.time_us, 0.5);
    assert_eq!(wrapped_next.time_us, 0.5);
    assert!(next.label.contains("falling edge"));
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
