use super::{
    WaveformTraceColor, WaveformTracePreset, WaveformTraceRef, WaveformTraceStyle,
    parse_waveform_csv_text, scope_trace_color_for_style, scope_visible_styled_trace_refs,
    scope_visible_trace_refs,
};
use crate::gui::{CircuitCiApp, ScopeProbeTarget, Stage};
use crate::reports::{Finding, ValidationReport};
use serde_json::json;

#[test]
fn scope_probe_target_pin_for_compare_adds_loaded_trace_once() {
    let waveform = parse_waveform_csv_text(
        "time,v(out),i(load)
0,0,0.1
0.000001,1,0.2
",
        "waveform.csv",
    )
    .unwrap();
    let target = ScopeProbeTarget {
        scenario_name: "waveform.csv".to_string(),
        probe_name: "i(load)".to_string(),
    };
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        ..Default::default()
    };

    assert!(app.pin_scope_probe_target_for_compare(target.clone()));
    assert!(app.scope_probe_target_pinned_for_compare(&target));
    assert_eq!(
        app.waveform_pinned_traces,
        vec![WaveformTraceRef {
            waveform_index: 0,
            probe_index: 1
        }]
    );

    assert!(app.pin_scope_probe_target_for_compare(target));
    assert_eq!(app.waveform_pinned_traces.len(), 1);
}

#[test]
fn scope_probe_target_pin_for_compare_reports_missing_loaded_trace() {
    let waveform = parse_waveform_csv_text(
        "time,v(out)
0,0
0.000001,1
",
        "waveform.csv",
    )
    .unwrap();
    let target = ScopeProbeTarget {
        scenario_name: "waveform.csv".to_string(),
        probe_name: "i(load)".to_string(),
    };
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        ..Default::default()
    };

    assert!(!app.pin_scope_probe_target_for_compare(target));
    assert!(app.waveform_pinned_traces.is_empty());
    assert!(app.status.contains("is not loaded yet"));
}

#[test]
fn scope_probe_target_unpin_for_compare_removes_loaded_trace() {
    let waveform = parse_waveform_csv_text(
        "time,v(out),i(load)
0,0,0.1
0.000001,1,0.2
",
        "waveform.csv",
    )
    .unwrap();
    let target = ScopeProbeTarget {
        scenario_name: "waveform.csv".to_string(),
        probe_name: "i(load)".to_string(),
    };
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        waveform_pinned_traces: vec![WaveformTraceRef {
            waveform_index: 0,
            probe_index: 1,
        }],
        ..Default::default()
    };

    assert!(app.unpin_scope_probe_target_for_compare(target.clone()));

    assert!(!app.scope_probe_target_pinned_for_compare(&target));
    assert!(app.waveform_pinned_traces.is_empty());
    assert!(app.status.contains("Unpinned scope trace i(load)"));
}

#[test]
fn clear_scope_compare_pins_from_sketch_removes_all_pins() {
    let mut app = CircuitCiApp {
        waveform_pinned_traces: vec![
            WaveformTraceRef {
                waveform_index: 0,
                probe_index: 1,
            },
            WaveformTraceRef {
                waveform_index: 0,
                probe_index: 2,
            },
        ],
        ..Default::default()
    };

    assert_eq!(app.clear_scope_compare_pins_from_sketch(), 2);

    assert!(app.waveform_pinned_traces.is_empty());
    assert!(app.status.contains("Cleared 2 pinned scope trace"));
}

#[test]
fn pin_selected_sweep_corner_traces_pins_matching_probe_from_loaded_corners() {
    let corner_001 = parse_waveform_csv_text(
        "time,v(filtered),i(load)
0,0,0.1
0.000001,1,0.2
",
        "out/analog/rc_lowpass_sine_transient/rc_tolerance_corner_001/waveform.csv",
    )
    .unwrap();
    let corner_002 = parse_waveform_csv_text(
        "time,v(filtered),i(load)
0,0,0.1
0.000001,2,0.2
",
        "out/analog/rc_lowpass_sine_transient/rc_tolerance_corner_002/waveform.csv",
    )
    .unwrap();
    let other_sweep = parse_waveform_csv_text(
        "time,v(filtered),i(load)
0,0,0.1
0.000001,3,0.2
",
        "out/analog/rc_lowpass_sine_transient/load_corner_001/waveform.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![corner_001, corner_002, other_sweep],
        selected_waveform: 0,
        selected_probe: 0,
        ..Default::default()
    };

    assert!(app.pin_selected_sweep_corner_traces());

    assert_eq!(
        app.waveform_pinned_traces,
        vec![WaveformTraceRef {
            waveform_index: 1,
            probe_index: 0
        }]
    );
    assert!(app.status.contains("1 loaded sweep-corner"));
}

#[test]
fn pin_selected_bode_sweep_corner_traces_pins_matching_probe_from_loaded_corners() {
    let corner_001 = parse_waveform_csv_text(
        "frequency_hz,filtered_mag_db,filtered_phase_deg\n10,0,0\n1000,-3,-45\n",
        "out/analog/rc_lowpass_bode_ac/rc_tolerance_corner_001/bode.csv",
    )
    .unwrap();
    let corner_002 = parse_waveform_csv_text(
        "frequency_hz,filtered_mag_db,filtered_phase_deg\n10,-0.1,-1\n1000,-2.8,-42\n",
        "out/analog/rc_lowpass_bode_ac/rc_tolerance_corner_002/bode.csv",
    )
    .unwrap();
    let other_sweep = parse_waveform_csv_text(
        "frequency_hz,filtered_mag_db,filtered_phase_deg\n10,-0.2,-2\n1000,-2.6,-40\n",
        "out/analog/rc_lowpass_bode_ac/load_corner_001/bode.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![corner_001, corner_002, other_sweep],
        selected_waveform: 0,
        selected_probe: 0,
        ..Default::default()
    };

    assert!(app.pin_selected_sweep_corner_traces());

    assert_eq!(
        app.waveform_pinned_traces,
        vec![WaveformTraceRef {
            waveform_index: 1,
            probe_index: 0
        }]
    );
    assert!(app.status.contains("1 loaded sweep-corner"));
}

#[test]
fn pin_selected_sweep_worst_corner_traces_uses_report_margin_probe_and_corner() {
    let corner_001 = parse_waveform_csv_text(
        "time,v(filtered),i(load)
0,0,0.1
0.000001,1,0.2
",
        "out/analog/rc_lowpass_sine_transient/rc_tolerance_corner_001/waveform.csv",
    )
    .unwrap();
    let corner_002 = parse_waveform_csv_text(
        "time,v(filtered),i(load)
0,0,0.1
0.000001,2,0.2
",
        "out/analog/rc_lowpass_sine_transient/rc_tolerance_corner_002/waveform.csv",
    )
    .unwrap();
    let corner_003 = parse_waveform_csv_text(
        "time,v(filtered),i(load)
0,0,0.1
0.000001,3,0.2
",
        "out/analog/rc_lowpass_sine_transient/rc_tolerance_corner_003/waveform.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![corner_001, corner_002, corner_003],
        selected_waveform: 0,
        selected_probe: 0,
        report: Some(report(vec![
            sweep_margin(
                "rc_lowpass_sine_transient",
                "rc_tolerance",
                "corner_002",
                "v_filtered",
            ),
            sweep_margin(
                "rc_lowpass_sine_transient",
                "rc_tolerance",
                "corner_003",
                "i_load",
            ),
        ])),
        ..Default::default()
    };

    assert!(app.pin_selected_sweep_worst_corner_traces());

    assert_eq!(
        app.waveform_pinned_traces,
        vec![WaveformTraceRef {
            waveform_index: 1,
            probe_index: 0
        }]
    );
    assert!(app.status.contains("1 worst-corner"));
}

#[test]
fn pin_selected_bode_worst_corner_traces_matches_report_probe_base_name() {
    let corner_001 = parse_waveform_csv_text(
        "frequency_hz,filtered_mag_db,filtered_phase_deg\n10,0,0\n1000,-3,-45\n",
        "out/analog/rc_lowpass_bode_ac/rc_tolerance_corner_001/bode.csv",
    )
    .unwrap();
    let corner_002 = parse_waveform_csv_text(
        "frequency_hz,filtered_mag_db,filtered_phase_deg\n10,-0.1,-1\n1000,-2.8,-42\n",
        "out/analog/rc_lowpass_bode_ac/rc_tolerance_corner_002/bode.csv",
    )
    .unwrap();
    let corner_003 = parse_waveform_csv_text(
        "frequency_hz,filtered_mag_db,filtered_phase_deg\n10,-0.2,-2\n1000,-2.6,-40\n",
        "out/analog/rc_lowpass_bode_ac/rc_tolerance_corner_003/bode.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![corner_001, corner_002, corner_003],
        selected_waveform: 0,
        selected_probe: 0,
        report: Some(report(vec![
            sweep_margin(
                "rc_lowpass_bode_ac",
                "rc_tolerance",
                "corner_002",
                "filtered",
            ),
            sweep_margin("rc_lowpass_bode_ac", "rc_tolerance", "corner_003", "input"),
        ])),
        ..Default::default()
    };

    assert!(app.pin_selected_sweep_worst_corner_traces());

    assert_eq!(
        app.waveform_pinned_traces,
        vec![WaveformTraceRef {
            waveform_index: 1,
            probe_index: 0
        }]
    );
    assert!(app.status.contains("1 worst-corner"));
}

#[test]
fn pin_selected_sweep_worst_corner_traces_reports_missing_summary() {
    let waveform = parse_waveform_csv_text(
        "time,v(filtered)
0,0
0.000001,1
",
        "out/analog/rc_lowpass_sine_transient/rc_tolerance_corner_001/waveform.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        selected_waveform: 0,
        selected_probe: 0,
        ..Default::default()
    };

    assert!(!app.pin_selected_sweep_worst_corner_traces());

    assert!(app.waveform_pinned_traces.is_empty());
    assert!(app.status.contains("No loaded worst-corner trace"));
}

#[test]
fn save_pinned_scope_compare_from_sketch_saves_pinned_traces_only() {
    let waveform = parse_waveform_csv_text(
        "time,v(out),i(load),v(ref)
0,0,0.1,3.3
0.000001,1,0.2,3.2
",
        "waveform.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        selected_probe: 0,
        waveform_pinned_traces: vec![
            WaveformTraceRef {
                waveform_index: 0,
                probe_index: 1,
            },
            WaveformTraceRef {
                waveform_index: 0,
                probe_index: 2,
            },
        ],
        waveform_trace_preset_name: "sketch compare".to_string(),
        ..Default::default()
    };

    assert!(app.save_pinned_scope_compare_from_sketch());

    assert_eq!(
        app.waveform_trace_presets,
        vec![WaveformTracePreset {
            name: "sketch compare".to_string(),
            traces: vec![
                WaveformTraceRef {
                    waveform_index: 0,
                    probe_index: 1,
                },
                WaveformTraceRef {
                    waveform_index: 0,
                    probe_index: 2,
                },
            ],
        }]
    );
    assert!(app.waveform_trace_preset_name.is_empty());
    assert!(
        app.status
            .contains("Saved scope compare set sketch compare")
    );
}

fn sweep_margin(scenario: &str, sweep: &str, corner: &str, probe: &str) -> Finding {
    let mut finding = Finding::info("ANALOG_SWEEP_MARGIN_SUMMARY", scenario, "summary");
    finding
        .measured
        .insert("analog_sweep".to_string(), json!(sweep));
    finding
        .measured
        .insert("analog_corner".to_string(), json!(corner));
    finding.measured.insert("probe".to_string(), json!(probe));
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
fn save_pinned_scope_compare_from_sketch_reports_empty_after_prune() {
    let waveform = parse_waveform_csv_text(
        "time,v(out)
0,0
0.000001,1
",
        "waveform.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        waveform_pinned_traces: vec![WaveformTraceRef {
            waveform_index: 0,
            probe_index: 9,
        }],
        ..Default::default()
    };

    assert!(!app.save_pinned_scope_compare_from_sketch());

    assert!(app.waveform_pinned_traces.is_empty());
    assert!(app.waveform_trace_presets.is_empty());
    assert!(app.status.contains("Pin at least one"));
}

#[test]
fn load_scope_compare_preset_from_sketch_restores_saved_set() {
    let waveform = parse_waveform_csv_text(
        "time,v(out),i(load),v(ref)
0,0,0.1,3.3
0.000001,1,0.2,3.2
",
        "waveform.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        selected_probe: 2,
        waveform_trace_presets: vec![WaveformTracePreset {
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
        }],
        ..Default::default()
    };

    assert!(app.load_scope_compare_preset_from_sketch(0));

    assert_eq!(app.selected_probe, 0);
    assert_eq!(
        app.waveform_pinned_traces,
        vec![WaveformTraceRef {
            waveform_index: 0,
            probe_index: 1,
        }]
    );
    assert!(app.status.contains("Loaded scope compare set startup"));
}

#[test]
fn delete_scope_compare_preset_from_sketch_removes_saved_set() {
    let mut app = CircuitCiApp {
        waveform_trace_presets: vec![WaveformTracePreset {
            name: "startup".to_string(),
            traces: vec![WaveformTraceRef {
                waveform_index: 0,
                probe_index: 1,
            }],
        }],
        ..Default::default()
    };

    assert!(app.delete_scope_compare_preset_from_sketch(0));

    assert!(app.waveform_trace_presets.is_empty());
    assert!(app.status.contains("Deleted scope compare set startup"));
}

#[test]
fn open_pinned_scope_compare_selects_first_valid_pin_and_opens_scopes() {
    let waveform = parse_waveform_csv_text(
        "time,v(out),i(load),v(ref)
0,0,0.1,3.3
0.000001,1,0.2,3.2
",
        "waveform.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        selected_probe: 0,
        waveform_pinned_traces: vec![
            WaveformTraceRef {
                waveform_index: 0,
                probe_index: 1,
            },
            WaveformTraceRef {
                waveform_index: 0,
                probe_index: 2,
            },
        ],
        stage: Stage::Sketch,
        waveform_playing: true,
        ..Default::default()
    };

    assert!(app.open_pinned_scope_compare());

    assert_eq!(app.stage, Stage::Simulation);
    assert_eq!(app.selected_waveform, 0);
    assert_eq!(app.selected_probe, 1);
    assert!(!app.waveform_playing);
    assert_eq!(app.current_scope_compare_traces().len(), 2);
    assert!(app.status.contains("2 pinned trace"));
}

#[test]
fn open_pinned_scope_compare_prunes_stale_pins_before_opening() {
    let waveform = parse_waveform_csv_text(
        "time,v(out)
0,0
0.000001,1
",
        "waveform.csv",
    )
    .unwrap();
    let mut app = CircuitCiApp {
        waveforms: vec![waveform],
        waveform_pinned_traces: vec![WaveformTraceRef {
            waveform_index: 99,
            probe_index: 0,
        }],
        stage: Stage::Sketch,
        ..Default::default()
    };

    assert!(!app.open_pinned_scope_compare());

    assert_eq!(app.stage, Stage::Sketch);
    assert!(app.waveform_pinned_traces.is_empty());
    assert!(app.status.contains("Pin at least one"));
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
fn scope_visible_traces_do_not_mix_time_and_frequency_axes() {
    let transient = parse_waveform_csv_text(
        "time,v(out)\n0,0\n0.000001,1\n",
        "out/gui/tran_main/waveform.csv",
    )
    .unwrap();
    let bode = parse_waveform_csv_text(
        "frequency_hz,v(out)_mag_db,v(out)_phase_deg\n10,0,0\n1000,-3,-45\n",
        "out/gui/ac_main/bode.csv",
    )
    .unwrap();
    let pinned = vec![
        WaveformTraceRef {
            waveform_index: 0,
            probe_index: 0,
        },
        WaveformTraceRef {
            waveform_index: 1,
            probe_index: 1,
        },
    ];

    assert_eq!(
        scope_visible_trace_refs(&[transient, bode], 1, 0, &pinned),
        vec![
            WaveformTraceRef {
                waveform_index: 1,
                probe_index: 0,
            },
            WaveformTraceRef {
                waveform_index: 1,
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
