use super::{
    WaveformMathDraft, WaveformProbeQuantity, append_derived_waveform_probe,
    derived_waveform_quantity, find_scope_probe, interpolated_value, parse_waveform_csv_text,
    runtime_probe_activity_for_selection, runtime_probe_lines_for_selection, sanitized_probe_name,
    scope_plot_size, waveform_measurement, waveform_probe_quantity_from_label,
    waveform_probe_value_for_badge, waveform_time_range_for_view,
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
