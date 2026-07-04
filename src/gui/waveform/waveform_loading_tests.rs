use super::merge_waveform_load_diagnostics;
use super::waveform_io::{
    parse_distortion_spectrum_csv_text, parse_fourier_summary_csv_text,
    parse_sensitivity_summary_csv_text,
};
use super::{
    WaveformFootprintSortKey, WaveformFootprintSourceFilter, WaveformLoadDiagnostic,
    WaveformLoadPreviewFilter, WaveformLoadRequest, WaveformLoadStatusFilter, WaveformTraceColor,
    WaveformTracePreset, WaveformTraceRef, WaveformTraceStyle, WaveformXAxis,
};
use super::{
    clear_deferred_waveform_column_picks,
    deferred_waveform_artifact_filtered_unloaded_probe_labels,
    deferred_waveform_artifact_picked_probe_labels,
    deferred_waveform_artifact_unloaded_probe_labels, deferred_waveform_artifact_visible_indexes,
    deferred_waveform_matching_probe_requests, deferred_waveform_remaining_probe_requests,
    load_report_waveforms_with_progress_and_cancel, load_waveform_csv_with_progress_and_cancel,
    load_waveform_paths_with_progress_and_cancel, load_waveform_requests_with_progress_and_cancel,
    parse_hb_spectrum_csv_text, parse_waveform_csv_text, select_deferred_waveform_column_picks,
    waveform_footprint_csv, waveform_footprint_largest_unload_targets, waveform_footprint_rows,
    waveform_footprint_rows_with_diagnostics, waveform_footprint_source_summaries,
    waveform_footprint_summary_csv, waveform_footprint_summary_markdown,
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
fn waveform_csv_loader_maps_bode_artifacts_to_frequency_axis() {
    let waveform = parse_waveform_csv_text(
        "frequency_hz,v(input)_mag_db,v(input)_phase_deg,v(filtered)_mag_db,v(filtered)_phase_deg,v(filtered)_mag\n10,0,0,-0.00017,-0.36,0.99998\n1000,0,0,-1.445,-32.14,0.8467\n10000,0,0,-16.07,-80.95,0.1573\n",
        "bode.csv",
    )
    .unwrap();

    assert_eq!(waveform.x_axis, WaveformXAxis::FrequencyHz);
    assert_eq!(waveform.time_s, vec![10.0e-6, 1000.0e-6, 10000.0e-6]);
    assert_eq!(waveform.probes.len(), 7);
    assert_eq!(waveform.probes[0].label, "v(input) magnitude dB");
    assert_eq!(waveform.probes[1].label, "v(input) phase deg");
    assert_eq!(super::probe_unit(&waveform.probes[0].label), "dB");
    assert_eq!(super::probe_unit(&waveform.probes[1].label), "deg");
    assert_eq!(super::probe_unit(&waveform.probes[4].label), "ratio");
    let input_delay = waveform
        .probes
        .iter()
        .find(|probe| probe.label == "v(input) group delay s")
        .unwrap();
    assert!(input_delay.derived);
    assert_eq!(input_delay.values, vec![0.0, 0.0, 0.0]);
    assert_eq!(super::probe_unit(&input_delay.label), "s");
    let filtered_delay = waveform
        .probes
        .iter()
        .find(|probe| probe.label == "v(filtered) group delay s")
        .unwrap();
    assert!(filtered_delay.derived);
    assert!(filtered_delay.values.iter().all(|value| *value > 0.0));
}

#[test]
fn waveform_csv_loader_adds_derived_s_parameter_margin_traces() {
    let waveform = parse_waveform_csv_text(
        "frequency_hz,reference_impedance_ohm,s11_mag_db,s11_phase_deg,s11_mag_linear,s21_mag_db,s21_phase_deg,s21_mag_linear,s12_mag_db,s12_phase_deg,s12_mag_linear,s22_mag_db,s22_phase_deg,s22_mag_linear\n1e6,50,-6.02059991328,0,0.5,6.02059991328,0,2.0,-40,0,0.01,-7.95880017344,0,0.4\n1e9,50,-13.97940008672,0,0.2,3.52182518111,0,1.5,-33.9794000867,0,0.02,-10.4575749056,0,0.3\n",
        "s_parameters.csv",
    )
    .unwrap();

    assert_eq!(waveform.x_axis, WaveformXAxis::FrequencyHz);
    assert_eq!(waveform.time_s, vec![1.0, 1000.0]);
    assert_eq!(waveform.probes[2].label, "s11 linear magnitude");
    let labels: Vec<_> = waveform
        .probes
        .iter()
        .map(|probe| probe.label.as_str())
        .collect();
    assert!(!labels.contains(&"reference_impedance_ohm"));
    assert!(labels.contains(&"s11 return loss dB"));
    assert!(labels.contains(&"s11 VSWR"));
    assert!(labels.contains(&"s11 mismatch loss dB"));
    assert!(labels.contains(&"s11 impedance real ohm"));
    assert!(labels.contains(&"s11 impedance imaginary ohm"));
    assert!(labels.contains(&"s11 impedance magnitude ohm"));
    assert!(labels.contains(&"s21 insertion loss dB"));
    assert!(labels.contains(&"s12 insertion loss dB"));
    assert!(labels.contains(&"s22 return loss dB"));
    assert!(labels.contains(&"two-port reciprocity error"));
    assert!(labels.contains(&"two-port passivity singular value"));
    assert!(labels.contains(&"two-port stability delta magnitude"));
    assert!(labels.contains(&"two-port Rollet K"));
    assert!(labels.contains(&"two-port maximum available gain dB"));
    assert!(labels.contains(&"two-port maximum stable gain dB"));
    assert!(labels.contains(&"s21 group delay s"));
    let s11_return_loss = waveform
        .probes
        .iter()
        .find(|probe| probe.label == "s11 return loss dB")
        .unwrap();
    assert!(s11_return_loss.derived);
    assert_eq!(s11_return_loss.values, vec![6.02059991328, 13.97940008672]);
    assert_eq!(super::probe_unit(&s11_return_loss.label), "dB");
    let s11_vswr = waveform
        .probes
        .iter()
        .find(|probe| probe.label == "s11 VSWR")
        .unwrap();
    assert!(s11_vswr.derived);
    assert!((s11_vswr.values[0] - 3.0).abs() < 1.0e-12);
    assert!((s11_vswr.values[1] - 1.5).abs() < 1.0e-12);
    assert_eq!(super::probe_unit(&s11_vswr.label), "ratio");
    let s11_mismatch_loss = waveform
        .probes
        .iter()
        .find(|probe| probe.label == "s11 mismatch loss dB")
        .unwrap();
    assert!(s11_mismatch_loss.derived);
    let expected_mismatch_loss_0 = -10.0_f64 * (1.0_f64 - 0.5_f64 * 0.5_f64).log10();
    let expected_mismatch_loss_1 = -10.0_f64 * (1.0_f64 - 0.2_f64 * 0.2_f64).log10();
    assert!((s11_mismatch_loss.values[0] - expected_mismatch_loss_0).abs() < 1.0e-12);
    assert!((s11_mismatch_loss.values[1] - expected_mismatch_loss_1).abs() < 1.0e-12);
    assert_eq!(super::probe_unit(&s11_mismatch_loss.label), "dB");
    let s11_impedance_real = waveform
        .probes
        .iter()
        .find(|probe| probe.label == "s11 impedance real ohm")
        .unwrap();
    assert!(s11_impedance_real.derived);
    assert!((s11_impedance_real.values[0] - 150.0).abs() < 1.0e-12);
    assert!((s11_impedance_real.values[1] - 75.0).abs() < 1.0e-12);
    assert_eq!(super::probe_unit(&s11_impedance_real.label), "ohm");
    let s11_impedance_imaginary = waveform
        .probes
        .iter()
        .find(|probe| probe.label == "s11 impedance imaginary ohm")
        .unwrap();
    assert!(s11_impedance_imaginary.derived);
    assert_eq!(s11_impedance_imaginary.values, vec![0.0, 0.0]);
    assert_eq!(super::probe_unit(&s11_impedance_imaginary.label), "ohm");
    let s11_impedance_magnitude = waveform
        .probes
        .iter()
        .find(|probe| probe.label == "s11 impedance magnitude ohm")
        .unwrap();
    assert!(s11_impedance_magnitude.derived);
    assert!((s11_impedance_magnitude.values[0] - 150.0).abs() < 1.0e-12);
    assert!((s11_impedance_magnitude.values[1] - 75.0).abs() < 1.0e-12);
    assert_eq!(super::probe_unit(&s11_impedance_magnitude.label), "ohm");
    let s21_insertion_loss = waveform
        .probes
        .iter()
        .find(|probe| probe.label == "s21 insertion loss dB")
        .unwrap();
    assert_eq!(
        s21_insertion_loss.values,
        vec![-6.02059991328, -3.52182518111]
    );
    assert_eq!(super::probe_unit(&s21_insertion_loss.label), "dB");
    let reciprocity = waveform
        .probes
        .iter()
        .find(|probe| probe.label == "two-port reciprocity error")
        .unwrap();
    assert!(reciprocity.derived);
    assert!((reciprocity.values[0] - 1.99).abs() < 1.0e-12);
    assert!((reciprocity.values[1] - 1.48).abs() < 1.0e-12);
    assert_eq!(super::probe_unit(&reciprocity.label), "ratio");
    let passivity = waveform
        .probes
        .iter()
        .find(|probe| probe.label == "two-port passivity singular value")
        .unwrap();
    assert!(passivity.derived);
    assert!(passivity.values[0] > 2.0);
    assert!(passivity.values[1] > 1.5);
    assert_eq!(super::probe_unit(&passivity.label), "ratio");
    let stability_delta = waveform
        .probes
        .iter()
        .find(|probe| probe.label == "two-port stability delta magnitude")
        .unwrap();
    assert!(stability_delta.derived);
    assert!((stability_delta.values[0] - 0.18).abs() < 1.0e-12);
    assert!((stability_delta.values[1] - 0.03).abs() < 1.0e-12);
    assert_eq!(super::probe_unit(&stability_delta.label), "ratio");
    let rollet_k = waveform
        .probes
        .iter()
        .find(|probe| probe.label == "two-port Rollet K")
        .unwrap();
    assert!(rollet_k.derived);
    assert!((rollet_k.values[0] - 15.56).abs() < 1.0e-12);
    assert!((rollet_k.values[1] - 14.515).abs() < 1.0e-12);
    assert_eq!(super::probe_unit(&rollet_k.label), "ratio");
    let maximum_available_gain = waveform
        .probes
        .iter()
        .find(|probe| probe.label == "two-port maximum available gain dB")
        .unwrap();
    assert!(maximum_available_gain.derived);
    let expected_mag_0 =
        10.0_f64 * (200.0_f64 * (15.56_f64 - (15.56_f64 * 15.56_f64 - 1.0).sqrt())).log10();
    let expected_mag_1 =
        10.0_f64 * (75.0_f64 * (14.515_f64 - (14.515_f64 * 14.515_f64 - 1.0).sqrt())).log10();
    assert!((maximum_available_gain.values[0] - expected_mag_0).abs() < 1.0e-12);
    assert!((maximum_available_gain.values[1] - expected_mag_1).abs() < 1.0e-12);
    assert_eq!(super::probe_unit(&maximum_available_gain.label), "dB");
    let maximum_stable_gain = waveform
        .probes
        .iter()
        .find(|probe| probe.label == "two-port maximum stable gain dB")
        .unwrap();
    assert!(maximum_stable_gain.derived);
    assert!((maximum_stable_gain.values[0] - 10.0_f64 * 200.0_f64.log10()).abs() < 1.0e-12);
    assert!((maximum_stable_gain.values[1] - 10.0_f64 * 75.0_f64.log10()).abs() < 1.0e-12);
    assert_eq!(super::probe_unit(&maximum_stable_gain.label), "dB");
    let s21_group_delay = waveform
        .probes
        .iter()
        .find(|probe| probe.label == "s21 group delay s")
        .unwrap();
    assert!(s21_group_delay.derived);
    assert_eq!(s21_group_delay.values, vec![0.0, 0.0]);
    assert_eq!(super::probe_unit(&s21_group_delay.label), "s");
}

#[test]
fn waveform_csv_loader_skips_vswr_when_reflection_magnitude_reaches_unity() {
    let waveform = parse_waveform_csv_text(
        "frequency_hz,s11_mag_db,s11_phase_deg,s11_mag_linear\n1e6,0,0,1.0\n1e9,-6.02059991328,0,0.5\n",
        "s_parameters.csv",
    )
    .unwrap();

    let labels: Vec<_> = waveform
        .probes
        .iter()
        .map(|probe| probe.label.as_str())
        .collect();
    assert!(labels.contains(&"s11 return loss dB"));
    assert!(!labels.contains(&"s11 VSWR"));
    assert!(!labels.contains(&"s11 mismatch loss dB"));
    assert!(!labels.contains(&"two-port reciprocity error"));
    assert!(!labels.contains(&"two-port passivity singular value"));
    assert!(!labels.contains(&"two-port stability delta magnitude"));
    assert!(!labels.contains(&"two-port Rollet K"));
    assert!(!labels.contains(&"two-port maximum available gain dB"));
    assert!(!labels.contains(&"two-port maximum stable gain dB"));
}

#[test]
fn waveform_request_loader_selects_raw_bode_header_names() {
    let mut file = tempfile::NamedTempFile::new().unwrap();
    use std::io::Write;

    writeln!(file, "frequency_hz,v(out)_mag_db,v(out)_phase_deg").unwrap();
    writeln!(file, "100,-0.1,-5").unwrap();
    writeln!(file, "1000,-3,-45").unwrap();
    let path = file.path().to_string_lossy().into_owned();
    let requests = vec![WaveformLoadRequest::selected_columns(
        path,
        vec!["v(out)_phase_deg".to_string()],
    )];

    let (waveforms, diagnostics) =
        load_waveform_requests_with_progress_and_cancel(&requests, |_, _| {}, || false, false)
            .unwrap();

    assert_eq!(waveforms.len(), 1);
    assert_eq!(waveforms[0].x_axis, WaveformXAxis::FrequencyHz);
    assert_eq!(waveforms[0].probes.len(), 1);
    assert_eq!(waveforms[0].probes[0].label, "v(out) phase deg");
    assert_eq!(waveforms[0].probes[0].values, vec![-5.0, -45.0]);
    assert!(diagnostics[0].loaded);
}

#[test]
fn waveform_loader_reads_noise_spectrum_as_frequency_density_traces() {
    let waveform = parse_waveform_csv_text(
        "frequency_hz,onoise_v_per_sqrt_hz,inoise_v_per_sqrt_hz\n10,2e-9,4e-9\n1000,3e-9,6e-9\n",
        "noise_spectrum.csv",
    )
    .unwrap();

    assert_eq!(waveform.x_axis, WaveformXAxis::FrequencyHz);
    assert_eq!(waveform.probes.len(), 2);
    assert_eq!(waveform.probes[0].label, "output noise density");
    assert_eq!(waveform.probes[1].label, "input noise density");
    assert_eq!(waveform.probes[0].values, vec![2.0e-9, 3.0e-9]);
    assert_eq!(super::probe_unit(&waveform.probes[0].label), "V/sqrt(Hz)");
}

#[test]
fn waveform_loader_reads_harmonic_balance_spectrum_as_frequency_traces() {
    let waveform = parse_hb_spectrum_csv_text(
        "output_expression,fundamental_frequency_hz,harmonic,frequency_hz,real,imaginary,magnitude,phase_deg\nV(out,0),1e5,0,0,0.5,0,0.5,0\nV(out,0),1e5,1,1e5,0.3,-0.4,0.5,-53.1301023542\nV(out,0),1e5,-1,-1e5,0.3,0.4,0.5,53.1301023542\n",
        "hb_spectrum.csv",
    )
    .unwrap();

    assert_eq!(waveform.x_axis, WaveformXAxis::FrequencyHz);
    assert_eq!(waveform.time_s, vec![0.0, 0.1]);
    assert_eq!(
        waveform
            .probes
            .iter()
            .map(|probe| probe.label.as_str())
            .collect::<Vec<_>>(),
        vec![
            "V(out,0) magnitude",
            "V(out,0) phase deg",
            "V(out,0) real",
            "V(out,0) imaginary",
        ]
    );
    assert_eq!(waveform.probes[0].values, vec![0.5, 0.5]);
    assert_eq!(waveform.probes[1].values, vec![0.0, -53.1301023542]);
    assert_eq!(super::probe_unit(&waveform.probes[0].label), "V");
    assert_eq!(super::probe_unit(&waveform.probes[1].label), "deg");
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
fn report_loader_includes_harmonic_balance_spectrum_artifacts() {
    let temp_dir = tempfile::tempdir().unwrap();
    let spectrum_path = temp_dir.path().join("hb_spectrum.csv");
    std::fs::write(
        &spectrum_path,
        "output_expression,fundamental_frequency_hz,harmonic,frequency_hz,real,imaginary,magnitude,phase_deg\nV(out),1e5,0,0,0.5,0,0.5,0\nV(out),1e5,1,1e5,0.3,-0.4,0.5,-53.13\n",
    )
    .unwrap();
    let report = crate::reports::ValidationReport::from_parts(
        "project".to_string(),
        "default".to_string(),
        Vec::new(),
        Vec::new(),
        vec![spectrum_path.to_string_lossy().into_owned()],
        Vec::new(),
        "validate".to_string(),
    );

    let (waveforms, diagnostics) =
        load_report_waveforms_with_progress_and_cancel(&report, |_, _| {}, || false, false)
            .unwrap();

    assert_eq!(waveforms.len(), 1);
    assert_eq!(waveforms[0].x_axis, WaveformXAxis::FrequencyHz);
    assert_eq!(waveforms[0].probes[0].label, "V(out) magnitude");
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].loaded);
    assert_eq!(diagnostics[0].samples, 2);
    assert_eq!(diagnostics[0].probes, 4);
}

#[test]
fn report_loader_includes_distortion_spectrum_artifacts() {
    let temp_dir = tempfile::tempdir().unwrap();
    let spectrum_path = temp_dir.path().join("distortion_spectrum.csv");
    std::fs::write(
        &spectrum_path,
        "component,frequency_hz,output_expression,real,imaginary,magnitude,phase_degrees\nh2,1e3,\"V(out,0)\",1e-3,0,1e-3,0\nh2,2e3,\"V(out,0)\",2e-3,0,2e-3,0\nh3,1e3,\"V(out,0)\",3e-4,4e-4,5e-4,53.13\nh3,2e3,\"V(out,0)\",6e-4,8e-4,1e-3,53.13\n",
    )
    .unwrap();
    let report = crate::reports::ValidationReport::from_parts(
        "project".to_string(),
        "default".to_string(),
        Vec::new(),
        Vec::new(),
        vec![spectrum_path.to_string_lossy().into_owned()],
        Vec::new(),
        "validate".to_string(),
    );

    let (waveforms, diagnostics) =
        load_report_waveforms_with_progress_and_cancel(&report, |_, _| {}, || false, false)
            .unwrap();

    assert_eq!(waveforms.len(), 1);
    assert_eq!(waveforms[0].x_axis, WaveformXAxis::FrequencyHz);
    assert_eq!(waveforms[0].time_s, vec![0.001, 0.002]);
    assert_eq!(waveforms[0].probes[0].label, "V(out,0) h2 magnitude");
    assert_eq!(waveforms[0].probes[4].label, "V(out,0) h3 magnitude");
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].loaded);
    assert_eq!(diagnostics[0].samples, 2);
    assert_eq!(diagnostics[0].probes, 8);
}

#[test]
fn waveform_loader_reads_fourier_summary_as_frequency_traces() {
    let waveform = parse_fourier_summary_csv_text(
        "output_expression,fundamental_frequency_hz,reported_harmonics,harmonic,frequency_hz,magnitude,phase_deg,normalized_magnitude,normalized_phase_deg,thd_percent,grid_size,interpolation_degree,periods\n\"V(out,0)\",1e5,5,0,0,0.509986,0,0,0,18.5435,200,1,1\n\"V(out,0)\",1e5,5,1,1e5,0.538779,-35.733,1,0,18.5435,200,1,1\n\"V(out,0)\",1e5,5,2,2e5,0.0124232,31.3212,0.0230581,67.0541,18.5435,200,1,1\n",
        "fourier_summary.csv",
    )
    .unwrap();

    assert_eq!(waveform.x_axis, WaveformXAxis::FrequencyHz);
    assert_eq!(waveform.time_s, vec![0.0, 0.1, 0.2]);
    assert_eq!(waveform.probes.len(), 4);
    assert_eq!(waveform.probes[0].label, "V(out,0) magnitude");
    assert_eq!(
        waveform.probes[0].values,
        vec![0.509986, 0.538779, 0.0124232]
    );
    assert_eq!(waveform.probes[1].label, "V(out,0) phase deg");
    assert_eq!(waveform.probes[2].label, "V(out,0) normalized magnitude");
    assert_eq!(waveform.probes[2].values, vec![0.0, 1.0, 0.0230581]);
    assert_eq!(waveform.probes[3].label, "V(out,0) normalized phase deg");
}

#[test]
fn report_loader_includes_fourier_summary_artifacts() {
    let temp_dir = tempfile::tempdir().unwrap();
    let summary_path = temp_dir.path().join("fourier_summary.csv");
    std::fs::write(
        &summary_path,
        "output_expression,fundamental_frequency_hz,reported_harmonics,harmonic,frequency_hz,magnitude,phase_deg,normalized_magnitude,normalized_phase_deg,thd_percent,grid_size,interpolation_degree,periods\nV(out),1e5,5,0,0,0.5,0,0,0,18.5,200,1,1\nV(out),1e5,5,1,1e5,0.3,-20,1,0,18.5,200,1,1\n",
    )
    .unwrap();
    let report = crate::reports::ValidationReport::from_parts(
        "project".to_string(),
        "default".to_string(),
        Vec::new(),
        Vec::new(),
        vec![summary_path.to_string_lossy().into_owned()],
        Vec::new(),
        "validate".to_string(),
    );

    let (waveforms, diagnostics) =
        load_report_waveforms_with_progress_and_cancel(&report, |_, _| {}, || false, false)
            .unwrap();

    assert_eq!(waveforms.len(), 1);
    assert_eq!(waveforms[0].x_axis, WaveformXAxis::FrequencyHz);
    assert_eq!(waveforms[0].probes[0].label, "V(out) magnitude");
    assert_eq!(waveforms[0].probes[2].label, "V(out) normalized magnitude");
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].loaded);
    assert_eq!(diagnostics[0].samples, 2);
    assert_eq!(diagnostics[0].probes, 4);
}

#[test]
fn waveform_loader_reads_ac_sensitivity_summary_as_frequency_traces() {
    let waveform = parse_sensitivity_summary_csv_text(
        "output_expression,mode,parameter,frequency_hz,sensitivity_real,sensitivity_imaginary,sensitivity_magnitude\nV(out),dc,r1,,-2.5e-4,0,2.5e-4\n\"V(out,0)\",ac,r1,1e2,-2.5e-4,1e-6,2.50002e-4\n\"V(out,0)\",ac,r1,1e3,-1.5e-4,2e-6,1.50013e-4\n\"V(out,0)\",ac,r2,1e2,2.5e-4,0,2.5e-4\n\"V(out,0)\",ac,r2,1e3,1.5e-4,-1e-6,1.50003e-4\n",
        "sensitivity_summary.csv",
    )
    .unwrap();

    assert_eq!(waveform.x_axis, WaveformXAxis::FrequencyHz);
    assert_eq!(waveform.time_s, vec![0.0001, 0.001]);
    assert_eq!(waveform.probes.len(), 6);
    assert_eq!(
        waveform.probes[0].label,
        "V(out,0) r1 sensitivity magnitude"
    );
    assert_eq!(waveform.probes[0].values, vec![2.50002e-4, 1.50013e-4]);
    assert_eq!(waveform.probes[1].label, "V(out,0) r1 sensitivity real");
    assert_eq!(waveform.probes[1].values, vec![-2.5e-4, -1.5e-4]);
    assert_eq!(
        waveform.probes[2].label,
        "V(out,0) r1 sensitivity imaginary"
    );
    assert_eq!(
        waveform.probes[3].label,
        "V(out,0) r2 sensitivity magnitude"
    );
}

#[test]
fn report_loader_includes_sensitivity_summary_artifacts() {
    let temp_dir = tempfile::tempdir().unwrap();
    let summary_path = temp_dir.path().join("sensitivity_summary.csv");
    std::fs::write(
        &summary_path,
        "output_expression,mode,parameter,frequency_hz,sensitivity_real,sensitivity_imaginary,sensitivity_magnitude\nV(out),ac,r1,1e2,-2.5e-4,1e-6,2.50002e-4\nV(out),ac,r1,1e3,-1.5e-4,2e-6,1.50013e-4\n",
    )
    .unwrap();
    let report = crate::reports::ValidationReport::from_parts(
        "project".to_string(),
        "default".to_string(),
        Vec::new(),
        Vec::new(),
        vec![summary_path.to_string_lossy().into_owned()],
        Vec::new(),
        "validate".to_string(),
    );

    let (waveforms, diagnostics) =
        load_report_waveforms_with_progress_and_cancel(&report, |_, _| {}, || false, false)
            .unwrap();

    assert_eq!(waveforms.len(), 1);
    assert_eq!(waveforms[0].x_axis, WaveformXAxis::FrequencyHz);
    assert_eq!(
        waveforms[0].probes[0].label,
        "V(out) r1 sensitivity magnitude"
    );
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].loaded);
    assert_eq!(diagnostics[0].samples, 2);
    assert_eq!(diagnostics[0].probes, 3);
}

#[test]
fn fourier_summary_parser_rejects_duplicate_frequency_rows() {
    let error = parse_fourier_summary_csv_text(
        "output_expression,fundamental_frequency_hz,reported_harmonics,harmonic,frequency_hz,magnitude,phase_deg,normalized_magnitude,normalized_phase_deg,thd_percent,grid_size,interpolation_degree,periods\nV(out),1e5,5,1,1e5,0.3,-20,1,0,18.5,200,1,1\nV(out),1e5,5,2,1e5,0.1,10,0.3,30,18.5,200,1,1\n",
        "fourier_summary.csv",
    )
    .unwrap_err();

    assert!(format!("{error:#}").contains("duplicate or non-increasing frequency"));
}

#[test]
fn sensitivity_summary_parser_rejects_mismatched_frequency_grids() {
    let error = parse_sensitivity_summary_csv_text(
        "output_expression,mode,parameter,frequency_hz,sensitivity_real,sensitivity_imaginary,sensitivity_magnitude\nV(out),ac,r1,1e2,-2.5e-4,1e-6,2.50002e-4\nV(out),ac,r2,1e3,1.5e-4,-1e-6,1.50003e-4\n",
        "sensitivity_summary.csv",
    )
    .unwrap_err();

    assert!(format!("{error:#}").contains("same frequency grid"));
}

#[test]
fn distortion_spectrum_parser_rejects_mismatched_frequency_grids() {
    let error = parse_distortion_spectrum_csv_text(
        "component,frequency_hz,output_expression,real,imaginary,magnitude,phase_degrees\nh2,1e3,\"V(out,0)\",1e-3,0,1e-3,0\nh3,2e3,\"V(out,0)\",6e-4,8e-4,1e-3,53.13\n",
        "distortion_spectrum.csv",
    )
    .unwrap_err();

    assert!(format!("{error:#}").contains("same frequency grid"));
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
