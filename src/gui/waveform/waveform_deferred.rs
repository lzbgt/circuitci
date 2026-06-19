use super::{CircuitCiApp, DeferredWaveformArtifact, WaveformLoadRequest};
use eframe::egui;
use std::collections::BTreeSet;

impl CircuitCiApp {
    pub(super) fn deferred_waveform_artifacts_ui(
        &mut self,
        ui: &mut egui::Ui,
        artifacts: &[DeferredWaveformArtifact],
        load_deferred_path: &mut Option<String>,
    ) {
        if artifacts.is_empty() {
            return;
        }
        self.prune_deferred_waveform_column_picks(artifacts);
        let visible_indexes =
            deferred_waveform_artifact_visible_indexes(artifacts, &self.waveform_deferred_filter);
        let visible_paths = visible_indexes
            .iter()
            .filter_map(|&index| artifacts.get(index))
            .map(|artifact| artifact.path.clone())
            .collect::<Vec<_>>();
        let matching_probe_requests = deferred_waveform_matching_probe_requests(
            artifacts,
            &visible_indexes,
            &self.waveform_deferred_filter,
        );
        let matching_probe_count = matching_probe_requests
            .iter()
            .map(WaveformLoadRequest::probe_count)
            .sum::<usize>();
        let remaining_probe_requests =
            deferred_waveform_remaining_probe_requests(artifacts, &visible_indexes);
        let remaining_probe_count = remaining_probe_requests
            .iter()
            .map(WaveformLoadRequest::probe_count)
            .sum::<usize>();
        let mut load_probe_requests = None;
        ui.horizontal_wrapped(|ui| {
            ui.menu_button(format!("Deferred Waveforms ({})", artifacts.len()), |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Find");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.waveform_deferred_filter)
                            .desired_width(180.0)
                            .hint_text("file, probe, detail"),
                    );
                    if response.changed() {
                        self.waveform_deferred_filter =
                            self.waveform_deferred_filter.trim_start().to_string();
                    }
                    if ui
                        .add_enabled(
                            !self.waveform_deferred_filter.trim().is_empty(),
                            egui::Button::new("Clear"),
                        )
                        .clicked()
                    {
                        self.waveform_deferred_filter.clear();
                    }
                    ui.label(format!(
                        "{} / {} visible",
                        visible_indexes.len(),
                        artifacts.len()
                    ));
                });
                if visible_indexes.is_empty() {
                    ui.label("No deferred waveform artifact matches the current filter.");
                    return;
                }
                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .show(ui, |ui| {
                        for &index in &visible_indexes {
                            let artifact = &artifacts[index];
                            let matching_probes = deferred_waveform_artifact_matching_probe_labels(
                                artifact,
                                &self.waveform_deferred_filter,
                            );
                            let picked_probes = deferred_waveform_artifact_picked_probe_labels(
                                artifact,
                                &self.waveform_deferred_column_picks,
                            );
                            ui.horizontal(|ui| {
                                if ui.button("Load").clicked() {
                                    *load_deferred_path = Some(artifact.path.clone());
                                    ui.close();
                                }
                                if ui
                                    .add_enabled(
                                        !matching_probes.is_empty(),
                                        egui::Button::new("Load Matching"),
                                    )
                                    .clicked()
                                {
                                    load_probe_requests =
                                        Some(vec![WaveformLoadRequest::selected_columns(
                                            artifact.path.clone(),
                                            matching_probes.clone(),
                                        )]);
                                    ui.close();
                                }
                                let unloaded_probes =
                                    deferred_waveform_artifact_unloaded_probe_labels(artifact);
                                if ui
                                    .add_enabled(
                                        !unloaded_probes.is_empty(),
                                        egui::Button::new("Load Remaining"),
                                    )
                                    .clicked()
                                {
                                    load_probe_requests =
                                        Some(vec![WaveformLoadRequest::selected_columns(
                                            artifact.path.clone(),
                                            unloaded_probes.clone(),
                                        )]);
                                    ui.close();
                                }
                                ui.menu_button("Columns", |ui| {
                                    ui.label(format!(
                                        "{} unloaded preview column(s)",
                                        unloaded_probes.len()
                                    ));
                                    if unloaded_probes.is_empty() {
                                        ui.label("All preview columns are already loaded.");
                                        return;
                                    }
                                    egui::ScrollArea::vertical()
                                        .max_height(160.0)
                                        .show(ui, |ui| {
                                            for probe in &artifact.probe_preview {
                                                let loaded = deferred_waveform_probe_is_loaded(
                                                    probe,
                                                    &artifact.loaded_probe_preview,
                                                );
                                                let key = deferred_waveform_column_pick_key(
                                                    artifact, probe,
                                                );
                                                if loaded {
                                                    let mut checked = true;
                                                    ui.add_enabled(
                                                        false,
                                                        egui::Checkbox::new(
                                                            &mut checked,
                                                            format!("{probe} loaded"),
                                                        ),
                                                    );
                                                    continue;
                                                }
                                                let mut checked = self
                                                    .waveform_deferred_column_picks
                                                    .contains(&key);
                                                if ui.checkbox(&mut checked, probe).changed() {
                                                    if checked {
                                                        self.waveform_deferred_column_picks
                                                            .insert(key);
                                                    } else {
                                                        self.waveform_deferred_column_picks
                                                            .remove(&key);
                                                    }
                                                }
                                            }
                                        });
                                    let picked = deferred_waveform_artifact_picked_probe_labels(
                                        artifact,
                                        &self.waveform_deferred_column_picks,
                                    );
                                    ui.horizontal(|ui| {
                                        if ui
                                            .add_enabled(
                                                !picked.is_empty(),
                                                egui::Button::new(format!(
                                                    "Load Selected ({})",
                                                    picked.len()
                                                )),
                                            )
                                            .clicked()
                                        {
                                            clear_deferred_waveform_column_picks(
                                                &mut self.waveform_deferred_column_picks,
                                                artifact,
                                                &picked,
                                            );
                                            load_probe_requests =
                                                Some(vec![WaveformLoadRequest::selected_columns(
                                                    artifact.path.clone(),
                                                    picked.clone(),
                                                )]);
                                            ui.close();
                                        }
                                        if ui
                                            .add_enabled(
                                                !picked.is_empty(),
                                                egui::Button::new("Clear"),
                                            )
                                            .clicked()
                                        {
                                            clear_deferred_waveform_column_picks(
                                                &mut self.waveform_deferred_column_picks,
                                                artifact,
                                                &picked,
                                            );
                                        }
                                    });
                                });
                                ui.monospace(&artifact.label);
                                let loaded_count = artifact.loaded_probe_preview.len();
                                let loaded = if loaded_count == 0 {
                                    String::new()
                                } else {
                                    format!("; {loaded_count} loaded")
                                };
                                ui.label(format!(
                                    "{}; ~{} row(s); {} trace(s){}",
                                    artifact.size_label, artifact.samples, artifact.probes, loaded
                                ));
                                if !picked_probes.is_empty() {
                                    ui.small(format!("{} picked", picked_probes.len()));
                                }
                            });
                            let preview = deferred_probe_preview_text(
                                &artifact.probe_preview,
                                &artifact.loaded_probe_preview,
                            );
                            if !preview.is_empty() {
                                ui.small(preview);
                            }
                            if !artifact.detail.is_empty() {
                                ui.small(&artifact.detail);
                            }
                        }
                    });
            });
            if ui.button("Load All Deferred").clicked() {
                self.load_deferred_waveforms();
            }
            if ui
                .add_enabled(
                    !visible_paths.is_empty() && visible_paths.len() < artifacts.len(),
                    egui::Button::new("Load Visible"),
                )
                .clicked()
            {
                self.load_deferred_waveform_paths(visible_paths);
            }
            if ui
                .add_enabled(
                    !matching_probe_requests.is_empty(),
                    egui::Button::new(format!("Load Matching Traces ({matching_probe_count})")),
                )
                .clicked()
            {
                load_probe_requests = Some(matching_probe_requests);
            }
            if ui
                .add_enabled(
                    !remaining_probe_requests.is_empty(),
                    egui::Button::new(format!("Load Remaining Preview ({remaining_probe_count})")),
                )
                .clicked()
            {
                load_probe_requests = Some(remaining_probe_requests);
            }
        });
        if let Some(requests) = load_probe_requests {
            self.load_deferred_waveform_requests(requests);
        }
    }

    fn prune_deferred_waveform_column_picks(&mut self, artifacts: &[DeferredWaveformArtifact]) {
        let valid = artifacts
            .iter()
            .flat_map(|artifact| {
                deferred_waveform_artifact_unloaded_probe_labels(artifact)
                    .into_iter()
                    .map(|probe| deferred_waveform_column_pick_key(artifact, &probe))
            })
            .collect::<BTreeSet<_>>();
        self.waveform_deferred_column_picks
            .retain(|pick| valid.contains(pick));
    }
}
fn deferred_probe_preview_text(probes: &[String], loaded_probes: &[String]) -> String {
    if probes.is_empty() {
        return String::new();
    }
    let visible = probes
        .iter()
        .take(6)
        .map(|probe| {
            if deferred_waveform_probe_is_loaded(probe, loaded_probes) {
                format!("{probe} (loaded)")
            } else {
                probe.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    if probes.len() > 6 {
        format!("{visible}, +{} more", probes.len() - 6)
    } else {
        visible
    }
}

pub(super) fn deferred_waveform_matching_probe_requests(
    artifacts: &[DeferredWaveformArtifact],
    visible_indexes: &[usize],
    query: &str,
) -> Vec<WaveformLoadRequest> {
    visible_indexes
        .iter()
        .filter_map(|&index| artifacts.get(index))
        .filter_map(|artifact| {
            let probes = deferred_waveform_artifact_matching_probe_labels(artifact, query);
            (!probes.is_empty())
                .then(|| WaveformLoadRequest::selected_columns(artifact.path.clone(), probes))
        })
        .collect()
}

pub(super) fn deferred_waveform_remaining_probe_requests(
    artifacts: &[DeferredWaveformArtifact],
    visible_indexes: &[usize],
) -> Vec<WaveformLoadRequest> {
    visible_indexes
        .iter()
        .filter_map(|&index| artifacts.get(index))
        .filter_map(|artifact| {
            let probes = deferred_waveform_artifact_unloaded_probe_labels(artifact);
            (!probes.is_empty())
                .then(|| WaveformLoadRequest::selected_columns(artifact.path.clone(), probes))
        })
        .collect()
}

fn deferred_waveform_artifact_matching_probe_labels(
    artifact: &DeferredWaveformArtifact,
    query: &str,
) -> Vec<String> {
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    artifact
        .probe_preview
        .iter()
        .filter(|probe| {
            probe.to_ascii_lowercase().contains(&query)
                && !deferred_waveform_probe_is_loaded(probe, &artifact.loaded_probe_preview)
        })
        .cloned()
        .collect()
}

pub(super) fn deferred_waveform_artifact_unloaded_probe_labels(
    artifact: &DeferredWaveformArtifact,
) -> Vec<String> {
    artifact
        .probe_preview
        .iter()
        .filter(|probe| !deferred_waveform_probe_is_loaded(probe, &artifact.loaded_probe_preview))
        .cloned()
        .collect()
}

pub(super) fn deferred_waveform_artifact_picked_probe_labels(
    artifact: &DeferredWaveformArtifact,
    picks: &BTreeSet<(String, String)>,
) -> Vec<String> {
    artifact
        .probe_preview
        .iter()
        .filter(|probe| {
            !deferred_waveform_probe_is_loaded(probe, &artifact.loaded_probe_preview)
                && picks.contains(&deferred_waveform_column_pick_key(artifact, probe))
        })
        .cloned()
        .collect()
}

fn deferred_waveform_probe_is_loaded(probe: &str, loaded_probes: &[String]) -> bool {
    loaded_probes
        .iter()
        .any(|loaded| loaded.trim().eq_ignore_ascii_case(probe.trim()))
}

fn deferred_waveform_column_pick_key(
    artifact: &DeferredWaveformArtifact,
    probe: &str,
) -> (String, String) {
    (artifact.path.clone(), probe.trim().to_string())
}

fn clear_deferred_waveform_column_picks(
    picks: &mut BTreeSet<(String, String)>,
    artifact: &DeferredWaveformArtifact,
    probes: &[String],
) {
    for probe in probes {
        picks.remove(&deferred_waveform_column_pick_key(artifact, probe));
    }
}

pub(super) fn deferred_waveform_artifact_visible_indexes(
    artifacts: &[DeferredWaveformArtifact],
    query: &str,
) -> Vec<usize> {
    let query = query.trim().to_ascii_lowercase();
    artifacts
        .iter()
        .enumerate()
        .filter_map(|(index, artifact)| {
            if query.is_empty() || deferred_waveform_artifact_search_text(artifact).contains(&query)
            {
                Some(index)
            } else {
                None
            }
        })
        .collect()
}

fn deferred_waveform_artifact_search_text(artifact: &DeferredWaveformArtifact) -> String {
    format!(
        "{} {} {} {} {} {}",
        artifact.label,
        artifact.path,
        artifact.size_label,
        artifact.samples,
        artifact.detail,
        artifact.probe_preview.join(" ")
    )
    .to_ascii_lowercase()
}
