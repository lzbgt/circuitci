use super::{
    WaveformMathDraft, WaveformProbeQuantity, WaveformTraceRef, append_derived_waveform_probe,
    derived_waveform_quantity, find_scope_probe, interpolated_value, parse_waveform_csv_text,
    runtime_probe_activity_for_selection, runtime_probe_lines_for_selection, sanitized_probe_name,
    scope_cursor_legend_rows, scope_plot_size, scope_visible_trace_refs, waveform_measurement,
    waveform_probe_quantity_from_label, waveform_probe_value_for_badge,
    waveform_time_range_for_view, waveform_time_window_for_view, waveform_trace_bounds_in_window,
    zoom_time_window,
};
use super::{
    clamp_value_window, expanded_value_bounds, nearest_scope_cursor_target, plot_x_to_time_us,
    plot_y_to_value,
};
use crate::gui::sketch::{
    ProjectSnapshot, SketchComponent, SketchNet, SketchNodeStyle, SketchPin, SketchSelection,
};
use crate::gui::sketch_probes::{SketchProbe, SketchProbeQuantity, SketchProbeTarget};
use crate::gui::{CircuitCiApp, ScopeProbeTarget};

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
