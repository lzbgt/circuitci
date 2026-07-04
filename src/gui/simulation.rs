use super::CircuitCiApp;
use super::analog::{
    AnalogProbeDraft, AnalogScenarioDraft, append_analog_transient_scenario_with_project_path,
    append_analog_voltage_probe,
};
use anyhow::{Context, Result};
use eframe::egui;
use std::path::Path;

impl CircuitCiApp {
    pub(super) fn simulation_stage(&mut self, ui: &mut egui::Ui) {
        self.scope_run_toolbar(ui);
        ui.separator();

        let available = ui.available_size();
        let side_width = (available.x * 0.30).clamp(320.0, 420.0);
        let gap = 8.0;
        if available.x >= 980.0 {
            let scope_size = egui::vec2(
                (available.x - side_width - gap).max(560.0),
                available.y.max(520.0),
            );
            ui.horizontal_top(|ui| {
                self.waveform_scope_view(ui, scope_size);
                ui.add_space(gap);
                ui.allocate_ui_with_layout(
                    egui::vec2(side_width, scope_size.y),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.scope_side_dock(ui),
                );
            });
        } else {
            self.waveform_scope_view(
                ui,
                egui::vec2(available.x.max(560.0), (available.y * 0.62).max(360.0)),
            );
            ui.separator();
            self.scope_side_dock(ui);
        }
    }

    fn scope_run_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.heading("Scopes");
            if ui.button("Model").clicked() {
                self.stage = super::Stage::Sketch;
            }
            if ui
                .add_enabled(
                    self.background_job_elapsed_secs().is_none() && self.project_snapshot.is_some(),
                    egui::Button::new("Run"),
                )
                .clicked()
            {
                self.run_scope_model();
            }
            self.scope_auto_probe_button(ui);
            self.scope_auto_probe_run_toggle(ui);
            if ui.button("Fit Time").clicked() {
                self.fit_waveform_time_window();
            }
            if let Some(elapsed_secs) = self.background_job_elapsed_secs() {
                let label = self.background_job_label().unwrap_or("job");
                ui.add(egui::Spinner::new());
                ui.label(format!("{label} running for {elapsed_secs:.1}s"));
                if ui
                    .add_enabled(
                        !self.background_job_cancel_requested(),
                        egui::Button::new("Cancel"),
                    )
                    .clicked()
                {
                    self.cancel_background_job();
                }
            }
            if let Some(report) = &self.report {
                ui.label(format!(
                    "result {}: critical {} / warning {} / info {}",
                    report.result,
                    report.summary.critical,
                    report.summary.warning,
                    report.summary.info
                ));
            }
        });
    }

    fn run_scope_model(&mut self) {
        self.run_model_with_scope_preparation();
    }

    pub(super) fn run_model_with_scope_preparation(&mut self) -> bool {
        if self.project_yaml_dirty {
            self.save_project_yaml();
            if self.project_yaml_dirty {
                return false;
            }
        }
        match self.prepare_auto_scope_probes_for_run() {
            Ok(true) => {
                self.save_project_yaml();
                if self.project_yaml_dirty {
                    return false;
                }
            }
            Ok(false) => {}
            Err(error) => {
                self.record_error(error);
                return false;
            }
        }
        match self.prepare_scope_run_inputs() {
            Ok(true) => {
                self.save_project_yaml();
                if self.project_yaml_dirty {
                    return false;
                }
            }
            Ok(false) => {}
            Err(error) => {
                self.record_error(error);
                return false;
            }
        }
        let had_background_job = self.background_job_elapsed_secs().is_some();
        self.validate_project();
        !had_background_job && self.background_job_elapsed_secs().is_some()
    }

    fn prepare_scope_run_inputs(&mut self) -> Result<bool> {
        let preparation = prepare_scope_run_yaml(
            &self.project_yaml,
            Path::new(&self.project_path),
            &self.analog_scenario_name,
            &self.analog_probe_name,
            self.analog_stop_time_us,
            self.analog_max_step_us,
        )?;
        let Some((updated, preparation)) = preparation else {
            return Ok(false);
        };
        self.remember_scope_probe_target(preparation.scenario_name(), preparation.probe_name());
        let status = preparation.status_message();
        self.apply_edited_project_yaml(updated, &status);
        Ok(true)
    }

    fn scope_side_dock(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            self.scope_run_readiness_panel(ui);
            ui.separator();
            self.waveform_controls_panel(ui);
            ui.separator();
            if let Some(snapshot) = self.project_snapshot.clone() {
                self.analog_scenario_editor(ui, &snapshot);
                self.analog_sweep_editor(ui);
                self.analog_generated_overview_panel(ui);
                self.analog_generated_settings_editor(ui);
                self.analog_generated_components_editor(ui);
                self.analog_stimulus_editor(ui);
                self.analog_model_file_manager(ui);
                self.selected_probe_assertions_panel(ui);
                self.analog_assertion_editor(ui);
                self.sparameter_assertion_editor(ui);
                self.sparameter_network_assertion_editor(ui);
                self.sparameter_noise_assertion_editor(ui);
            }
            self.spice_deck_editor(ui);
            self.scope_artifacts_and_findings(ui);
        });
    }

    fn scope_artifacts_and_findings(&mut self, ui: &mut egui::Ui) {
        let mut bundle_install_repair_report: Option<String> = None;
        if self.report.is_some() {
            ui.separator();
            let report = self.report.clone().expect("checked above");
            egui::CollapsingHeader::new("Artifacts")
                .default_open(false)
                .show(ui, |ui| {
                    ui.label("Waveforms");
                    if report.waveforms.is_empty() {
                        ui.label("No waveform artifacts were emitted by the current run setup.");
                    } else {
                        for waveform in &report.waveforms {
                            ui.monospace(waveform);
                        }
                    }
                    ui.add_space(8.0);
                    ui.label("Artifacts");
                    if report.artifacts.is_empty() {
                        ui.label("No artifacts were emitted.");
                    } else {
                        for artifact in &report.artifacts {
                            ui.monospace(artifact);
                        }
                    }
                    ui.add_space(8.0);
                    ui.label("Distortion summary");
                    if report.distortion_summaries.is_empty() {
                        ui.label("No distortion summary rows were emitted.");
                    } else {
                        for row in &report.distortion_summaries {
                            ui.monospace(format!(
                                "{} {} rows={} max={:.6e} at {:.6e} Hz artifact={}",
                                row.component,
                                row.output_expression,
                                row.row_count,
                                row.max_magnitude,
                                row.frequency_hz_at_max,
                                row.artifact
                            ));
                        }
                    }
                    ui.add_space(8.0);
                    ui.label("Pole-zero summary");
                    if report.pole_zero_summaries.is_empty() {
                        ui.label("No pole-zero summary rows were emitted.");
                    } else {
                        for row in &report.pole_zero_summaries {
                            ui.monospace(format!(
                                "{} {} real={:.6e}rad/s imag={:.6e}rad/s f={:.6e}Hz out={} ref={} src={} artifact={}",
                                row.root_kind,
                                row.root_index,
                                row.real_rad_per_s,
                                row.imaginary_rad_per_s,
                                row.frequency_hz,
                                row.output_node,
                                row.reference_node,
                                row.input_source,
                                row.artifact
                            ));
                        }
                    }
                    ui.add_space(8.0);
                    ui.label("Sensitivity summary");
                    if report.sensitivity_summaries.is_empty() {
                        ui.label("No sensitivity summary rows were emitted.");
                    } else {
                        for row in &report.sensitivity_summaries {
                            let frequency = row
                                .frequency_hz
                                .map(|value| format!("{value:.6e}Hz"))
                                .unwrap_or_else(|| "dc".to_string());
                            ui.monospace(format!(
                                "{} {} {} f={} real={:.6e} imag={:.6e} mag={:.6e} artifact={}",
                                row.output_expression,
                                row.mode,
                                row.parameter,
                                frequency,
                                row.sensitivity_real,
                                row.sensitivity_imaginary,
                                row.sensitivity_magnitude,
                                row.artifact
                            ));
                        }
                    }
                    ui.add_space(8.0);
                    ui.label("Transfer function summary");
                    if report.transfer_function_summaries.is_empty() {
                        ui.label("No transfer-function summary rows were emitted.");
                    } else {
                        for row in &report.transfer_function_summaries {
                            ui.monospace(format!(
                                "{} from {} gain={:.6e} rin={:.6e}ohm rout={:.6e}ohm artifact={}",
                                row.output_expression,
                                row.input_source,
                                row.transfer_function_gain,
                                row.input_resistance_ohm,
                                row.output_resistance_ohm,
                                row.artifact
                            ));
                        }
                    }
                    ui.add_space(8.0);
                    self.sparameter_assertion_failure_actions(ui, &report.failures);
                    ui.add_space(8.0);
                    ui.label("S-parameter summary");
                    if report.s_parameter_summaries.is_empty() {
                        ui.label("No S-parameter summary rows were emitted.");
                    } else {
                        for row in &report.s_parameter_summaries {
                            ui.monospace(format!(
                                "{} rows={} f={:.6e}..{:.6e}Hz mag_db={:.6e}..{:.6e} return_loss={} insertion_loss={} vswr={} mismatch_loss={} group_delay={} z_mag={} artifact={}",
                                row.parameter,
                                row.row_count,
                                row.min_frequency_hz,
                                row.max_frequency_hz,
                                row.min_mag_db,
                                row.max_mag_db,
                                optional_range_label(row.min_return_loss_db, row.max_return_loss_db),
                                optional_range_label(row.min_insertion_loss_db, row.max_insertion_loss_db),
                                optional_range_label(row.min_vswr, row.max_vswr),
                                optional_range_label(row.min_mismatch_loss_db, row.max_mismatch_loss_db),
                                optional_range_label(row.min_group_delay_s, row.max_group_delay_s),
                                optional_range_label(row.min_impedance_magnitude_ohm, row.max_impedance_magnitude_ohm),
                                row.artifact
                            ));
                        }
                    }
                    ui.add_space(8.0);
                    ui.label("S-parameter network summary");
                    if report.s_parameter_network_summaries.is_empty() {
                        ui.label("No S-parameter network summary rows were emitted.");
                    } else {
                        for row in &report.s_parameter_network_summaries {
                            ui.monospace(format!(
                                "ports={} rows={} f={:.6e}..{:.6e}Hz reciprocity_error={:.6e} at {:.6e}Hz passivity_singular={:.6e} at {:.6e}Hz rollet_k_min={} at {}Hz delta_mag_max={} at {}Hz mag_min_db={} at {}Hz msg_min_db={} at {}Hz unilateral_gain_min_db={} at {}Hz transducer_gain_min_db={} at {}Hz available_gain_min_db={} at {}Hz operating_gain_min_db={} at {}Hz artifact={}",
                                row.port_count,
                                row.row_count,
                                row.min_frequency_hz,
                                row.max_frequency_hz,
                                row.max_reciprocity_error_linear,
                                row.frequency_hz_at_max_reciprocity_error,
                                row.max_passivity_singular_value,
                                row.frequency_hz_at_max_passivity,
                                optional_value_label(row.min_rollet_k),
                                optional_value_label(row.frequency_hz_at_min_rollet_k),
                                optional_value_label(row.max_stability_delta_magnitude),
                                optional_value_label(
                                    row.frequency_hz_at_max_stability_delta_magnitude
                                ),
                                optional_value_label(row.min_maximum_available_gain_db),
                                optional_value_label(
                                    row.frequency_hz_at_min_maximum_available_gain
                                ),
                                optional_value_label(row.min_maximum_stable_gain_db),
                                optional_value_label(
                                    row.frequency_hz_at_min_maximum_stable_gain
                                ),
                                optional_value_label(row.min_maximum_unilateral_gain_db),
                                optional_value_label(
                                    row.frequency_hz_at_min_maximum_unilateral_gain
                                ),
                                optional_value_label(row.min_transducer_gain_db),
                                optional_value_label(row.frequency_hz_at_min_transducer_gain),
                                optional_value_label(row.min_available_gain_db),
                                optional_value_label(row.frequency_hz_at_min_available_gain),
                                optional_value_label(row.min_operating_gain_db),
                                optional_value_label(row.frequency_hz_at_min_operating_gain),
                                row.artifact
                            ));
                        }
                    }
                    ui.add_space(8.0);
                    ui.label("S-parameter noise summary");
                    if report.s_parameter_noise_summaries.is_empty() {
                        ui.label("No S-parameter noise summary rows were emitted.");
                    } else {
                        for row in &report.s_parameter_noise_summaries {
                            ui.monospace(format!(
                                "rows={} f={:.6e}..{:.6e}Hz nf_max={:.6e}dB at {:.6e}Hz nfmin_max={:.6e}dB at {:.6e}Hz rn_max={:.6e}ohm at {:.6e}Hz gamma_opt_max={:.6e} at {:.6e}Hz artifact={}",
                                row.row_count,
                                row.min_frequency_hz,
                                row.max_frequency_hz,
                                row.max_noise_figure_db,
                                row.frequency_hz_at_max_noise_figure,
                                row.max_minimum_noise_figure_db,
                                row.frequency_hz_at_max_minimum_noise_figure,
                                row.max_equivalent_noise_resistance_ohm,
                                row.frequency_hz_at_max_equivalent_noise_resistance,
                                row.max_optimum_source_reflection_magnitude,
                                row.frequency_hz_at_max_optimum_source_reflection_magnitude,
                                row.artifact
                            ));
                        }
                    }
                    ui.add_space(8.0);
                    ui.label("Fourier summary");
                    if report.fourier_summaries.is_empty() {
                        ui.label("No Fourier summary rows were emitted.");
                    } else {
                        for row in &report.fourier_summaries {
                            let thd = row
                                .thd_percent
                                .map(|value| format!("{value:.6e}%"))
                                .unwrap_or_else(|| "n/a".to_string());
                            ui.monospace(format!(
                                "{} h{} f={:.6e}Hz mag={:.6e} norm={:.6e} phase={:.6e}deg thd={} artifact={}",
                                row.output_expression,
                                row.harmonic,
                                row.frequency_hz,
                                row.magnitude,
                                row.normalized_magnitude,
                                row.phase_deg,
                                thd,
                                row.artifact
                            ));
                        }
                    }
                    ui.add_space(8.0);
                    ui.label("Model file provenance");
                    if report.model_file_provenance.is_empty() {
                        ui.label("No compiled model provenance was emitted.");
                    } else {
                        for provenance in &report.model_file_provenance {
                            ui.monospace(format!(
                                "{} [{}] scenario={} backend={} rebuild={} produced_by_circuitci={}",
                                provenance.model_file,
                                provenance.artifact_format,
                                provenance.scenario,
                                provenance.backend,
                                provenance.rebuild_mode,
                                provenance
                                    .produced_by_circuitci
                                    .map(|value| value.to_string())
                                    .unwrap_or_else(|| "unknown".to_string())
                            ));
                            ui.monospace(format!(
                                "source {} {}",
                                provenance.source_path, provenance.source_sha256_actual
                            ));
                            ui.monospace(format!(
                                "artifact {}",
                                provenance.artifact_sha256_actual
                            ));
                            if let Some(package_name) = &provenance.model_package_name {
                                ui.monospace(format!(
                                    "package {} {} artifact={} lock={}",
                                    package_name,
                                    provenance.model_package_version.as_deref().unwrap_or(""),
                                    provenance
                                        .model_package_artifact_id
                                        .as_deref()
                                        .unwrap_or(""),
                                    provenance.model_package_lock_path.as_deref().unwrap_or("")
                                ));
                                if let Some(registry_path) =
                                    &provenance.model_package_registry_path
                                {
                                    ui.monospace(format!(
                                        "registry {} entry={}",
                                        registry_path,
                                        provenance
                                            .model_package_registry_entry
                                            .as_deref()
                                            .unwrap_or("")
                                    ));
                                }
                            }
                        }
                    }
                    ui.add_space(8.0);
                    ui.label("Model package conformance");
                    if report.model_package_conformance_checks.is_empty() {
                        ui.label("No package conformance checks were emitted.");
                    } else {
                        for check in &report.model_package_conformance_checks {
                            ui.monospace(format!(
                                "{} [{}] solver={} result={} target={} {}",
                                check.check_name,
                                check.analysis,
                                check.solver,
                                check.result,
                                check.target_artifact_id,
                                check.target_artifact_sha256
                            ));
                            ui.monospace(format!(
                                "report {} artifact={}",
                                check.report, check.report_artifact_id
                            ));
                            if !check.artifacts.is_empty() {
                                ui.monospace(format!("evidence {}", check.artifacts.join(", ")));
                            }
                        }
                    }
                    ui.add_space(8.0);
                    ui.label("Model package bundle verification");
                    if report.model_package_bundle_verifications.is_empty() {
                        ui.label("No package bundle verification reports were emitted.");
                    } else {
                        for bundle in &report.model_package_bundle_verifications {
                            ui.monospace(format!(
                                "{} {} result={} artifacts={} conformance_checks={} findings={}",
                                bundle.package_name.as_deref().unwrap_or("<unknown-package>"),
                                bundle.package_version.as_deref().unwrap_or(""),
                                bundle.result,
                                bundle.artifact_count,
                                bundle.conformance_check_count,
                                bundle.finding_count
                            ));
                            ui.monospace(format!(
                                "bundle {} report {}",
                                bundle.bundle_path, bundle.report
                            ));
                            ui.monospace(format!(
                                "manifest {} {}",
                                bundle.manifest_path,
                                bundle.manifest_sha256_actual.as_deref().unwrap_or("")
                            ));
                            if let Some(lock_path) = &bundle.lock_path {
                                ui.monospace(format!(
                                    "lock {} {}",
                                    lock_path,
                                    bundle.lock_sha256_actual.as_deref().unwrap_or("")
                                ));
                            }
                            if let Some(registry_path) = &bundle.registry_path {
                                ui.monospace(format!(
                                    "registry {} {}",
                                    registry_path,
                                    bundle.registry_sha256_actual.as_deref().unwrap_or("")
                                ));
                            }
                        }
                    }
                    ui.add_space(8.0);
                    ui.label("Model package bundle install");
                    if report.model_package_bundle_installs.is_empty() {
                        ui.label("No package bundle install reports were emitted.");
                    } else {
                        for install in &report.model_package_bundle_installs {
                            ui.monospace(format!(
                                "{} {} result={} artifacts={} conformance_checks={} findings={}",
                                install.package_name.as_deref().unwrap_or("<unknown-package>"),
                                install.package_version.as_deref().unwrap_or(""),
                                install.result,
                                install.artifact_count,
                                install.conformance_check_count,
                                install.finding_count
                            ));
                            ui.monospace(format!(
                                "source {} install_dir {} report {}",
                                install.source_bundle, install.install_dir, install.report
                            ));
                            if let Some(registry_path) = &install.installed_registry_path {
                                ui.monospace(format!(
                                    "installed_registry {} {}",
                                    registry_path,
                                    install
                                        .installed_registry_sha256_actual
                                        .as_deref()
                                        .unwrap_or("")
                                ));
                            }
                            if let Some(entry) = &install.model_package_registry_entry {
                                ui.monospace(format!(
                                    "scenario_import registry={} sha={} entry={} lock={} lock_sha={} artifact={}",
                                    install.model_package_registry_path.as_deref().unwrap_or(""),
                                    install.model_package_registry_sha256.as_deref().unwrap_or(""),
                                    entry,
                                    install.model_package_lock_path.as_deref().unwrap_or(""),
                                    install.model_package_lock_sha256.as_deref().unwrap_or(""),
                                    install.model_package_artifact_id.as_deref().unwrap_or("")
                                ));
                            }
                            if let Some(command) = &install.repair_yaml_command {
                                ui.monospace(format!("repair_command {command}"));
                                if ui
                                    .add_enabled(
                                        self.background_job_elapsed_secs().is_none(),
                                        egui::Button::new("Repair YAML"),
                                    )
                                    .clicked()
                                {
                                    bundle_install_repair_report = Some(install.report.clone());
                                }
                            }
                        }
                    }
                    ui.add_space(8.0);
                    ui.label("Model package bundle import");
                    if report.model_package_bundle_imports.is_empty() {
                        ui.label("No package bundle import reports were emitted.");
                    } else {
                        for import in &report.model_package_bundle_imports {
                            ui.monospace(format!(
                                "{} {} result={} artifacts={} conformance_checks={} repair_applied={} repair_blocked={}",
                                import.package_name.as_deref().unwrap_or("<unknown-package>"),
                                import.package_version.as_deref().unwrap_or(""),
                                import.result,
                                import.bundle_artifacts,
                                import.conformance_checks,
                                import.repair_applied,
                                import.repair_blocked
                            ));
                            ui.monospace(format!(
                                "bundle {} project {} install_dir {} report {}",
                                import.bundle_path, import.project, import.install_dir, import.report
                            ));
                            if let Some(entry) = &import.model_package_registry_entry {
                                ui.monospace(format!(
                                    "scenario_import registry={} sha={} entry={} lock={} lock_sha={} artifact={}",
                                    import.model_package_registry_path.as_deref().unwrap_or(""),
                                    import.model_package_registry_sha256.as_deref().unwrap_or(""),
                                    entry,
                                    import.model_package_lock_path.as_deref().unwrap_or(""),
                                    import.model_package_lock_sha256.as_deref().unwrap_or(""),
                                    import.model_package_artifact_id.as_deref().unwrap_or("")
                                ));
                            }
                            if let Some(repaired_project) = &import.repaired_project {
                                ui.monospace(format!(
                                    "repaired_project {} report={}",
                                    repaired_project,
                                    import.repaired_validation_report.as_deref().unwrap_or("")
                                ));
                            }
                        }
                    }
                    ui.add_space(8.0);
                    ui.label("YAML repairs");
                    if report.yaml_repairs.is_empty() {
                        ui.label("No YAML repair reports were emitted.");
                    } else {
                        for repair in &report.yaml_repairs {
                            ui.monospace(format!(
                                "{} mode={} result={} proposed={} selected={} applied={} blocked={} skipped={}",
                                repair.finding,
                                repair.mode,
                                repair.result,
                                repair.proposed,
                                repair.selected,
                                repair.applied,
                                repair.blocked,
                                repair.skipped
                            ));
                            ui.monospace(format!(
                                "project {} -> {}",
                                repair.original_project,
                                repair.repaired_project.as_deref().unwrap_or("")
                            ));
                            ui.monospace(format!(
                                "report {} repaired_report={}",
                                repair.report,
                                repair.repaired_report.as_deref().unwrap_or("")
                            ));
                            ui.monospace(format!(
                                "proof original_removed={} no_new_criticals={} new_criticals={}",
                                repair
                                    .original_finding_removed
                                    .map(|value| value.to_string())
                                    .unwrap_or_else(|| "unknown".to_string()),
                                repair
                                    .no_new_criticals
                                    .map(|value| value.to_string())
                                    .unwrap_or_else(|| "unknown".to_string()),
                                repair.new_criticals
                            ));
                            if !repair.reason_codes.is_empty() {
                                ui.monospace(format!(
                                    "reason_codes {}",
                                    repair.reason_codes.join(", ")
                                ));
                            }
                        }
                    }
                });
            egui::CollapsingHeader::new("Findings")
                .default_open(false)
                .show(ui, |ui| {
                    self.findings_view(ui, &report);
                });
        } else {
            ui.separator();
            ui.label("Run validation to observe SPICE waveforms, generated decks, and findings.");
        }
        if let Some(report) = bundle_install_repair_report {
            self.repair_bundle_install_package_metadata(report);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ScopeRunPreparation {
    AddedScenario {
        scenario_name: String,
        probe_name: String,
        net_id: String,
    },
    AddedProbe {
        scenario_name: String,
        probe_name: String,
        net_id: String,
    },
}

impl ScopeRunPreparation {
    fn scenario_name(&self) -> &str {
        match self {
            Self::AddedScenario { scenario_name, .. } | Self::AddedProbe { scenario_name, .. } => {
                scenario_name
            }
        }
    }

    fn probe_name(&self) -> &str {
        match self {
            Self::AddedScenario { probe_name, .. } | Self::AddedProbe { probe_name, .. } => {
                probe_name
            }
        }
    }

    fn status_message(&self) -> String {
        match self {
            Self::AddedScenario {
                scenario_name,
                probe_name,
                net_id,
            } => format!(
                "Run created transient setup {scenario_name} with voltage probe {probe_name} on net {net_id}."
            ),
            Self::AddedProbe {
                scenario_name,
                probe_name,
                net_id,
            } => format!(
                "Run added voltage probe {probe_name} on net {net_id} to run setup {scenario_name}."
            ),
        }
    }
}

fn prepare_scope_run_yaml(
    text: &str,
    project_path: &Path,
    preferred_scenario_name: &str,
    preferred_probe_name: &str,
    stop_time_us: f64,
    max_step_us: f64,
) -> Result<Option<(String, ScopeRunPreparation)>> {
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    if project
        .scenarios
        .iter()
        .filter_map(|scenario| scenario.analog.as_ref())
        .any(|analog| !analog.probes.is_empty())
    {
        return Ok(None);
    }

    if let Some((scenario_name, net_id, probe_name)) =
        scope_probe_for_existing_analog_scenario(&project, preferred_probe_name)?
    {
        let draft = AnalogProbeDraft {
            scenario_name: scenario_name.clone(),
            net_id: net_id.clone(),
            probe_name: probe_name.clone(),
        };
        let updated = append_analog_voltage_probe(text, &draft)?;
        return Ok(Some((
            updated,
            ScopeRunPreparation::AddedProbe {
                scenario_name,
                probe_name,
                net_id,
            },
        )));
    }

    let ground_net = default_scope_ground_net(&project)?;
    let probe_net = default_scope_probe_net(&project)?;
    let scenario_name = unique_scope_scenario_name(&project, preferred_scenario_name);
    let probe_name = nonblank_id(preferred_probe_name, "probe_voltage");
    let draft = AnalogScenarioDraft {
        name: scenario_name.clone(),
        ground_net,
        probe_net: probe_net.clone(),
        probe_name: probe_name.clone(),
        stop_time_us,
        max_step_us,
    };
    let updated = append_analog_transient_scenario_with_project_path(text, project_path, &draft)?;
    Ok(Some((
        updated,
        ScopeRunPreparation::AddedScenario {
            scenario_name,
            probe_name,
            net_id: probe_net,
        },
    )))
}

fn scope_probe_for_existing_analog_scenario(
    project: &crate::board_ir::BoardProject,
    preferred_probe_name: &str,
) -> Result<Option<(String, String, String)>> {
    for scenario in &project.scenarios {
        let Some(analog) = scenario.analog.as_ref() else {
            continue;
        };
        let Some(net_id) = analog
            .node_bindings
            .iter()
            .map(|binding| binding.net.as_str())
            .find(|net_id| {
                project
                    .board
                    .nets
                    .get(*net_id)
                    .is_some_and(|net| net.kind != crate::board_ir::NetKind::Ground)
            })
            .or_else(|| {
                analog
                    .node_bindings
                    .first()
                    .map(|binding| binding.net.as_str())
            })
        else {
            anyhow::bail!(
                "Run setup {} has no node bindings; add a voltage probe manually after binding schematic nets.",
                scenario.name
            );
        };
        let probe_name = unique_scope_probe_name(analog, preferred_probe_name);
        return Ok(Some((
            scenario.name.clone(),
            net_id.to_string(),
            probe_name,
        )));
    }
    Ok(None)
}

fn default_scope_ground_net(project: &crate::board_ir::BoardProject) -> Result<String> {
    project
        .board
        .nets
        .iter()
        .find_map(|(id, net)| (net.kind == crate::board_ir::NetKind::Ground).then(|| id.clone()))
        .context("Run needs a ground net before it can create a default transient setup.")
}

fn default_scope_probe_net(project: &crate::board_ir::BoardProject) -> Result<String> {
    project
        .board
        .nets
        .iter()
        .find_map(|(id, net)| (net.kind != crate::board_ir::NetKind::Ground).then(|| id.clone()))
        .or_else(|| project.board.nets.keys().next().cloned())
        .context("Run needs at least one schematic net before it can create a default scope probe.")
}

fn unique_scope_scenario_name(
    project: &crate::board_ir::BoardProject,
    preferred_scenario_name: &str,
) -> String {
    let existing = project
        .scenarios
        .iter()
        .map(|scenario| scenario.name.as_str())
        .collect::<Vec<_>>();
    unique_id(preferred_scenario_name, "gui_transient", &existing)
}

fn unique_scope_probe_name(
    analog: &crate::board_ir::AnalogScenario,
    preferred_probe_name: &str,
) -> String {
    let existing = analog
        .probes
        .iter()
        .map(|probe| probe.name.as_str())
        .collect::<Vec<_>>();
    unique_id(preferred_probe_name, "probe_voltage", &existing)
}

fn unique_id(preferred: &str, fallback: &str, existing: &[&str]) -> String {
    let base = nonblank_id(preferred, fallback);
    if !existing.iter().any(|name| *name == base) {
        return base;
    }
    for suffix in 2.. {
        let candidate = format!("{base}_{suffix}");
        if !existing.iter().any(|name| *name == candidate) {
            return candidate;
        }
    }
    unreachable!("unbounded suffix search should always find a unique id")
}

fn nonblank_id(preferred: &str, fallback: &str) -> String {
    let trimmed = preferred.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn optional_range_label(min: Option<f64>, max: Option<f64>) -> String {
    match (min, max) {
        (Some(min), Some(max)) => format!("{min:.6e}..{max:.6e}"),
        _ => "n/a".to_string(),
    }
}

fn optional_value_label(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.6e}"))
        .unwrap_or_else(|| "n/a".to_string())
}

#[cfg(test)]
mod tests {
    use super::{ScopeRunPreparation, prepare_scope_run_yaml};
    use std::path::Path;

    const BASE_PROJECT: &str = "project:
  name: scope_run_auto_probe_test
  version: 0.1.0
board:
  components:
    V1:
      model: generic.analog.voltage_source
      spice:
        primitive: dc_voltage_source
        dc_voltage_v: 5.0
      pins:
        P: rail_5v
        N: gnd
    R1:
      model: generic.analog.resistor
      spice:
        primitive: resistor
        value_ohm: 1000.0
      pins:
        A: rail_5v
        B: out
    C1:
      model: generic.analog.capacitor
      spice:
        primitive: capacitor
        value_f: 0.000001
      pins:
        A: out
        B: gnd
  nets:
    gnd: {kind: ground}
    out: {kind: digital_or_analog}
    rail_5v: {kind: power, nominal_voltage: 5.0, powered: true}
scenarios: []
";

    #[test]
    fn scope_run_preparation_adds_generated_scenario_and_probe() {
        let (updated, preparation) = prepare_scope_run_yaml(
            BASE_PROJECT,
            Path::new("project.yaml"),
            "gui_transient",
            "probe_voltage",
            100.0,
            1.0,
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            preparation,
            ScopeRunPreparation::AddedScenario {
                scenario_name: "gui_transient".to_string(),
                probe_name: "probe_voltage".to_string(),
                net_id: "out".to_string(),
            }
        );
        let choices = crate::gui::analog::analog_scenario_choices(&updated).unwrap();
        assert_eq!(choices.len(), 1);
        assert_eq!(choices[0].name, "gui_transient");
        assert_eq!(choices[0].probes[0].name, "probe_voltage");
    }

    #[test]
    fn scope_run_preparation_adds_probe_to_existing_empty_analog_scenario() {
        let project = BASE_PROJECT.replace(
            "scenarios: []",
            "scenarios:
  - name: existing_transient
    type: analog
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated: {components: [V1, R1, C1], ground_net: gnd}
      model_files: []
      node_bindings:
        - {node: '0', net: gnd}
        - {node: out, net: out}
        - {node: rail_5v, net: rail_5v}
      pin_bindings:
        - {node: rail_5v, endpoint: {component: V1, pin: P}}
        - {node: '0', endpoint: {component: V1, pin: N}}
        - {node: rail_5v, endpoint: {component: R1, pin: A}}
        - {node: out, endpoint: {component: R1, pin: B}}
        - {node: out, endpoint: {component: C1, pin: A}}
        - {node: '0', endpoint: {component: C1, pin: B}}
      analysis: {type: tran, stop_time_us: 100.0, max_step_us: 1.0}
      stimuli: []
      probes: []
      assertions: []
",
        );
        let (updated, preparation) = prepare_scope_run_yaml(
            &project,
            Path::new("project.yaml"),
            "gui_transient",
            "probe_voltage",
            100.0,
            1.0,
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            preparation,
            ScopeRunPreparation::AddedProbe {
                scenario_name: "existing_transient".to_string(),
                probe_name: "probe_voltage".to_string(),
                net_id: "out".to_string(),
            }
        );
        let choices = crate::gui::analog::analog_scenario_choices(&updated).unwrap();
        assert_eq!(choices[0].probes[0].name, "probe_voltage");
    }

    #[test]
    fn scope_run_preparation_keeps_existing_scope_probe() {
        let project = BASE_PROJECT.replace(
            "scenarios: []",
            "scenarios:
  - name: existing_transient
    type: analog
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated: {components: [V1, R1, C1], ground_net: gnd}
      model_files: []
      node_bindings:
        - {node: '0', net: gnd}
        - {node: out, net: out}
      pin_bindings: []
      analysis: {type: tran, stop_time_us: 100.0, max_step_us: 1.0}
      stimuli: []
      probes:
        - {name: out_voltage, expression: V(out), quantity: voltage}
      assertions: []
",
        );

        assert!(
            prepare_scope_run_yaml(
                &project,
                Path::new("project.yaml"),
                "gui_transient",
                "probe_voltage",
                100.0,
                1.0,
            )
            .unwrap()
            .is_none()
        );
    }
}
