use super::CircuitCiApp;
use anyhow::{Context, Result};
use eframe::egui;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
struct SpiceDeckChoice {
    scenario: String,
    path: String,
    resolved_path: PathBuf,
}

impl CircuitCiApp {
    pub(super) fn spice_deck_editor(&mut self, ui: &mut egui::Ui) {
        ui.collapsing("File-backed SPICE Deck", |ui| {
            if self.project_yaml.trim().is_empty() {
                ui.label("Load a project or import a SPICE deck first.");
                return;
            }
            let choices =
                match spice_deck_choices(&self.project_yaml, Path::new(&self.project_path)) {
                    Ok(choices) => choices,
                    Err(error) => {
                        ui.label(format!("SPICE deck choices unavailable: {error}"));
                        return;
                    }
                };
            if choices.is_empty() {
                ui.label("No file-backed analog run setup is declared in this project.");
                return;
            }
            if !choices
                .iter()
                .any(|choice| choice.scenario == self.spice_deck_scenario)
            {
                self.spice_deck_scenario = choices[0].scenario.clone();
                self.spice_deck_path = choices[0].resolved_path.to_string_lossy().into_owned();
                self.spice_deck_text.clear();
                self.spice_deck_dirty = false;
            }

            egui::Grid::new("spice_deck_editor_grid")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Run setup");
                    egui::ComboBox::from_id_salt("spice_deck_scenario")
                        .selected_text(&self.spice_deck_scenario)
                        .show_ui(ui, |ui| {
                            for choice in &choices {
                                if ui
                                    .selectable_value(
                                        &mut self.spice_deck_scenario,
                                        choice.scenario.clone(),
                                        &choice.scenario,
                                    )
                                    .clicked()
                                {
                                    self.spice_deck_path =
                                        choice.resolved_path.to_string_lossy().into_owned();
                                    self.spice_deck_text.clear();
                                    self.spice_deck_dirty = false;
                                }
                            }
                        });
                    ui.end_row();

                    ui.label("Deck path");
                    ui.monospace(
                        selected_spice_choice(&choices, &self.spice_deck_scenario)
                            .map_or(self.spice_deck_path.as_str(), |choice| choice.path.as_str()),
                    );
                    ui.end_row();
                });

            let selected = selected_spice_choice(&choices, &self.spice_deck_scenario).cloned();
            ui.horizontal(|ui| {
                if ui.button("Load Deck").clicked()
                    && let Some(choice) = &selected
                {
                    self.load_spice_deck_text(choice.resolved_path.clone());
                }
                if ui
                    .add_enabled(
                        selected.is_some() && self.spice_deck_dirty,
                        egui::Button::new("Save Deck"),
                    )
                    .clicked()
                    && let Some(choice) = &selected
                {
                    self.save_spice_deck_text(choice.resolved_path.clone(), false);
                }
                if ui
                    .add_enabled(
                        selected.is_some() && !self.spice_deck_text.is_empty(),
                        egui::Button::new("Save + Run"),
                    )
                    .clicked()
                    && let Some(choice) = &selected
                {
                    self.save_spice_deck_text(choice.resolved_path.clone(), true);
                }
                if self.spice_deck_dirty {
                    ui.label("unsaved deck edits");
                }
            });

            if self.spice_deck_text.is_empty() {
                ui.label("Load the deck to edit file-backed SPICE source in place.");
            } else {
                let response = egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut self.spice_deck_text)
                                .font(egui::TextStyle::Monospace)
                                .desired_rows(16)
                                .lock_focus(true),
                        )
                    });
                if response.inner.changed() {
                    self.spice_deck_dirty = true;
                }
            }
        });
    }

    fn load_spice_deck_text(&mut self, path: PathBuf) {
        match std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read SPICE deck {}.", path.display()))
        {
            Ok(text) => {
                self.spice_deck_path = path.to_string_lossy().into_owned();
                self.spice_deck_text = text;
                self.spice_deck_dirty = false;
                self.status = format!("Loaded SPICE deck {}.", self.spice_deck_path);
                self.push_diagnostic("SPICE deck loaded into the observation editor.");
            }
            Err(error) => self.record_error(error),
        }
    }

    fn save_spice_deck_text(&mut self, path: PathBuf, run_after_save: bool) {
        match std::fs::write(&path, &self.spice_deck_text)
            .with_context(|| format!("Failed to write SPICE deck {}.", path.display()))
        {
            Ok(()) => {
                self.spice_deck_path = path.to_string_lossy().into_owned();
                self.spice_deck_dirty = false;
                self.status = format!("Saved SPICE deck {}.", self.spice_deck_path);
                self.push_diagnostic("SPICE deck saved.");
                if run_after_save {
                    self.validate_project();
                }
            }
            Err(error) => self.record_error(error),
        }
    }
}

fn spice_deck_choices(project_yaml: &str, project_path: &Path) -> Result<Vec<SpiceDeckChoice>> {
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(project_yaml).context("Project YAML is not valid Board IR.")?;
    let project_dir = project_path.parent().unwrap_or_else(|| Path::new("."));
    let mut choices = Vec::new();
    for scenario in project.scenarios {
        let Some(analog) = scenario.analog else {
            continue;
        };
        if analog.netlist_source != crate::board_ir::AnalogNetlistSource::File {
            continue;
        }
        let Some(path) = analog.netlist else {
            continue;
        };
        let raw_path = Path::new(&path);
        let resolved_path = if raw_path.is_absolute() {
            raw_path.to_path_buf()
        } else {
            normalize_path(&project_dir.join(raw_path))
        };
        choices.push(SpiceDeckChoice {
            scenario: scenario.name,
            path,
            resolved_path,
        });
    }
    choices.sort_by(|left, right| left.scenario.cmp(&right.scenario));
    Ok(choices)
}

fn selected_spice_choice<'a>(
    choices: &'a [SpiceDeckChoice],
    scenario: &str,
) -> Option<&'a SpiceDeckChoice> {
    choices.iter().find(|choice| choice.scenario == scenario)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::spice_deck_choices;
    use std::path::Path;

    #[test]
    fn spice_deck_choices_resolve_file_backed_analog_scenarios() {
        let project = "project:
  name: spice_editor_test
  version: 0.1.0
board:
  components: {}
  nets: {}
scenarios:
  - name: imported_spice_transient
    type: analog_transient
    checks: [SPICE_TRANSIENT_ANALYSIS]
    analog:
      backend: auto
      netlist_source: file
      netlist: decks/filter.cir
      model_files: []
      node_bindings: []
      pin_bindings: []
      analysis: { type: tran, stop_time_us: 20.0, max_step_us: 1.0 }
      stimuli: []
      probes: []
      assertions: []
";
        let choices = spice_deck_choices(project, Path::new("out/gui/project.yaml")).unwrap();
        assert_eq!(choices.len(), 1);
        assert_eq!(choices[0].scenario, "imported_spice_transient");
        assert_eq!(choices[0].path, "decks/filter.cir");
        assert_eq!(
            choices[0].resolved_path,
            Path::new("out/gui/decks/filter.cir")
        );
    }

    #[test]
    fn spice_deck_choices_ignore_generated_scenarios() {
        let project = "project:
  name: generated_only
  version: 0.1.0
board:
  components: {}
  nets: {}
scenarios:
  - name: generated_transient
    type: analog_transient
    checks: [SPICE_TRANSIENT_ANALYSIS]
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated: { components: [], ground_net: gnd }
      model_files: []
      node_bindings: []
      pin_bindings: []
      analysis: { type: tran, stop_time_us: 20.0, max_step_us: 1.0 }
      stimuli: []
      probes: []
      assertions: []
";
        let choices = spice_deck_choices(project, Path::new("project.yaml")).unwrap();
        assert!(choices.is_empty());
    }
}
