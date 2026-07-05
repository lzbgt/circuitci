use super::{
    WaveformFootprintSortKey, WaveformFootprintSourceFilter, WaveformLoadDiagnostic,
    WaveformTraceRef, parse_waveform_csv_text, waveform_footprint_csv,
    waveform_footprint_largest_unload_targets, waveform_footprint_rows,
    waveform_footprint_rows_with_diagnostics, waveform_footprint_source_summaries,
    waveform_footprint_summary_csv, waveform_footprint_summary_markdown,
    waveform_footprint_unload_targets, waveform_load_deferred_artifacts,
};
use crate::gui::CircuitCiApp;

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

    assert!(csv.starts_with(
        "waveform,source,path,samples,probes,values,estimated_bytes,estimated_size\n"
    ));
    assert!(csv.contains("\"/tmp/run/quoted,\"\"scope\"\".csv\""));
    assert!(csv.contains(",runtime_only,\"/tmp/run/quoted,\"\"scope\"\".csv\",2,2,6,48,48 B\n"));
    assert!(!csv.contains("plain.csv"));
}

#[test]
fn waveform_footprint_rows_classify_loaded_source_type() {
    let full_path = "/tmp/run/full.csv";
    let selected_path = "/tmp/run/selected.csv";
    let runtime_path = "/tmp/run/runtime.csv";
    let full = parse_waveform_csv_text("time,v(out),i(load)\n0,1,0.1\n0.000001,2,0.2\n", full_path)
        .unwrap();
    let selected =
        parse_waveform_csv_text("time,i(load)\n0,0.1\n0.000001,0.2\n", selected_path).unwrap();
    let runtime =
        parse_waveform_csv_text("time,v(runtime)\n0,3\n0.000001,4\n", runtime_path).unwrap();
    let diagnostics = vec![
        WaveformLoadDiagnostic::loaded(full_path.to_string(), Some(256), 2, 2, 3),
        WaveformLoadDiagnostic::loaded_selected(
            selected_path.to_string(),
            Some(512),
            2,
            1,
            4,
            vec!["i(load)".to_string()],
        ),
    ];
    let waveforms = vec![full, selected, runtime];
    let rows = waveform_footprint_rows_with_diagnostics(
        &waveforms,
        &diagnostics,
        "",
        WaveformFootprintSourceFilter::All,
        WaveformFootprintSortKey::Label,
        false,
    );

    assert_eq!(
        rows.iter()
            .map(|row| (row.path.as_str(), row.source.csv_label()))
            .collect::<Vec<_>>(),
        vec![
            (full_path, "full_csv"),
            (runtime_path, "runtime_only"),
            (selected_path, "selected_columns"),
        ]
    );
    assert_eq!(
        waveform_footprint_source_summaries(&rows)
            .iter()
            .map(|summary| {
                (
                    summary.source.csv_label(),
                    summary.count,
                    summary.estimated_bytes,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("full_csv", 1, 48),
            ("selected_columns", 1, 32),
            ("runtime_only", 1, 32),
        ]
    );
    let summaries = waveform_footprint_source_summaries(&rows);
    assert_eq!(
        waveform_footprint_summary_csv(&summaries, rows.len(), 112),
        "source,count,estimated_bytes,estimated_size\n\
total,3,112,112 B\n\
full_csv,1,48,48 B\n\
selected_columns,1,32,32 B\n\
runtime_only,1,32,32 B\n"
    );
    let markdown = waveform_footprint_summary_markdown(&summaries, rows.len(), 112);
    assert!(markdown.starts_with("## Loaded Waveform Footprint Summary\n\n"));
    assert!(markdown.contains("| Total | 3 | 112 | 112 B |"));
    assert!(markdown.contains("| Selected Columns | 1 | 32 | 32 B |"));
    assert_eq!(
        waveform_footprint_rows_with_diagnostics(
            &waveforms,
            &diagnostics,
            "selected_columns",
            WaveformFootprintSourceFilter::All,
            WaveformFootprintSortKey::Label,
            false,
        )
        .iter()
        .map(|row| row.path.as_str())
        .collect::<Vec<_>>(),
        vec![selected_path]
    );
    assert_eq!(
        waveform_footprint_rows_with_diagnostics(
            &waveforms,
            &diagnostics,
            "",
            WaveformFootprintSourceFilter::SelectedColumns,
            WaveformFootprintSortKey::Label,
            false,
        )
        .iter()
        .map(|row| row.path.as_str())
        .collect::<Vec<_>>(),
        vec![selected_path]
    );
    let csv = waveform_footprint_csv(&rows);
    assert!(csv.contains(",full_csv,/tmp/run/full.csv,"));
    assert!(csv.contains(",selected_columns,/tmp/run/selected.csv,"));
    assert!(csv.contains(",runtime_only,/tmp/run/runtime.csv,"));
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
