use super::CircuitCiApp;
use super::sketch::{SketchSelection, edit_component_model};
use anyhow::{Context, Result};
use eframe::egui;
use std::path::Path;

#[derive(Debug, Clone)]
struct ModelBrowserEntry {
    id: String,
    category: String,
    source: String,
    confidence: String,
    ports: usize,
    features: Vec<&'static str>,
}

impl CircuitCiApp {
    pub(super) fn library_stage(&mut self, ui: &mut egui::Ui) {
        ui.heading("Library Binding");
        ui.separator();
        if let Some(snapshot) = &self.project_snapshot {
            if snapshot.libraries.is_empty() {
                ui.label("Project uses default library resolution.");
            } else {
                for library in &snapshot.libraries {
                    ui.monospace(library);
                }
            }
        }

        self.model_browser(ui);

        if !self.suggestions_yaml.is_empty() {
            ui.separator();
            ui.label("Suggested scenarios");
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.suggestions_yaml)
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(24)
                        .lock_focus(true),
                );
            });
        }
    }

    fn model_browser(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.strong("Component Model Browser");
        if self.project_yaml.trim().is_empty() {
            ui.label("Load project YAML to browse models from the active library set.");
            return;
        }

        let entries = match model_browser_entries(&self.project_yaml, Path::new(&self.project_path))
        {
            Ok(entries) => entries,
            Err(error) => {
                ui.label(format!("Model browser unavailable: {error}"));
                return;
            }
        };
        ui.horizontal(|ui| {
            ui.label("Search");
            ui.text_edit_singleline(&mut self.model_search);
            if ui.button("Clear").clicked() {
                self.model_search.clear();
            }
        });

        let selected_component =
            selected_component_id(self.selected_sketch_item.as_ref()).map(str::to_string);
        ui.horizontal_wrapped(|ui| {
            ui.label("Selected component");
            if let Some(component_id) = &selected_component {
                ui.monospace(component_id);
            } else {
                ui.label("none");
            }
            ui.label("Selected model");
            if self.selected_library_model.is_empty() {
                ui.label("none");
            } else {
                ui.monospace(&self.selected_library_model);
            }
        });

        ui.horizontal(|ui| {
            let can_apply =
                selected_component.is_some() && !self.selected_library_model.trim().is_empty();
            if ui
                .add_enabled(can_apply, egui::Button::new("Use For Selected Component"))
                .clicked()
                && let Some(component_id) = &selected_component
            {
                self.apply_selected_library_model(component_id.clone());
            }
            if ui
                .add_enabled(
                    !self.selected_library_model.trim().is_empty(),
                    egui::Button::new("Use For New Component"),
                )
                .clicked()
            {
                self.new_component_model = self.selected_library_model.clone();
            }
        });

        let filtered = filtered_entries(&entries, &self.model_search);
        ui.label(format!("{} of {} model(s)", filtered.len(), entries.len()));
        egui::ScrollArea::vertical()
            .max_height(280.0)
            .show(ui, |ui| {
                egui::Grid::new("model_browser_grid")
                    .num_columns(6)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Model");
                        ui.strong("Category");
                        ui.strong("Quality");
                        ui.strong("Ports");
                        ui.strong("Features");
                        ui.strong("Action");
                        ui.end_row();
                        for entry in filtered {
                            ui.monospace(&entry.id);
                            ui.label(&entry.category);
                            ui.label(format!("{} / {}", entry.source, entry.confidence));
                            ui.label(entry.ports.to_string());
                            ui.label(entry.features.join(", "));
                            if ui.button("Select").clicked() {
                                self.selected_library_model = entry.id.clone();
                            }
                            ui.end_row();
                        }
                    });
            });
    }

    fn apply_selected_library_model(&mut self, component_id: String) {
        match edit_component_model(
            &self.project_yaml,
            &component_id,
            &self.selected_library_model,
        ) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Component {component_id} model set to {}.",
                    self.selected_library_model
                ),
            ),
            Err(error) => self.record_error(error),
        }
    }
}

fn model_browser_entries(
    project_yaml: &str,
    project_path: &Path,
) -> Result<Vec<ModelBrowserEntry>> {
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(project_yaml).context("Project YAML is not valid Board IR.")?;
    let (library, findings) = crate::library::load_library(project_path, &project);
    let hard_failures: Vec<_> = findings
        .iter()
        .filter(|finding| finding.id == "LIBRARY_NOT_FOUND" || finding.id == "MODEL_LOAD_FAILED")
        .map(|finding| finding.message.clone())
        .collect();
    if !hard_failures.is_empty() {
        anyhow::bail!("{}", hard_failures.join("; "));
    }
    let mut entries: Vec<_> = library
        .iter()
        .map(|(id, model)| ModelBrowserEntry {
            id: id.to_string(),
            category: model.category.clone(),
            source: model.model_quality.source.clone(),
            confidence: model.model_quality.confidence.clone(),
            ports: model.ports.len(),
            features: model_features(model),
        })
        .collect();
    entries.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(entries)
}

fn filtered_entries<'a>(
    entries: &'a [ModelBrowserEntry],
    query: &str,
) -> Vec<&'a ModelBrowserEntry> {
    let terms: Vec<String> = query
        .split_whitespace()
        .map(|term| term.to_ascii_lowercase())
        .collect();
    entries
        .iter()
        .filter(|entry| {
            if terms.is_empty() {
                return true;
            }
            let haystack = format!(
                "{} {} {} {} {}",
                entry.id,
                entry.category,
                entry.source,
                entry.confidence,
                entry.features.join(" ")
            )
            .to_ascii_lowercase();
            terms.iter().all(|term| haystack.contains(term))
        })
        .collect()
}

fn selected_component_id(selection: Option<&SketchSelection>) -> Option<&str> {
    match selection {
        Some(SketchSelection::Component(component_id)) => Some(component_id.as_str()),
        _ => None,
    }
}

fn model_features(model: &crate::library::ComponentModel) -> Vec<&'static str> {
    let mut features = Vec::new();
    if model.power_conversion.is_some() {
        features.push("power");
    }
    if model.power_switch.is_some() {
        features.push("switch");
    }
    if model.battery_charger.is_some() {
        features.push("charger");
    }
    if model.power_mux.is_some() {
        features.push("mux");
    }
    if model.reset_supervisor.is_some() {
        features.push("reset");
    }
    if model.usb_connector.is_some() || model.connector.is_some() {
        features.push("connector");
    }
    if model.cable_assembly.is_some() {
        features.push("cable");
    }
    if !model.signal_conditioning.protection_clamps.is_empty() {
        features.push("protection");
    }
    if !model.clock_sources.is_empty() || model.crystal.is_some() {
        features.push("clock");
    }
    if model.motor_load.is_some() {
        features.push("motor");
    }
    if model.regen_absorber.is_some() {
        features.push("regen");
    }
    if model.motor_bridge.is_some() {
        features.push("bridge");
    }
    if model.datasheet.is_some() {
        features.push("datasheet");
    }
    if features.is_empty() {
        features.push("basic");
    }
    features
}

#[cfg(test)]
mod tests {
    use super::{filtered_entries, model_browser_entries};
    use std::path::Path;

    fn project_yaml() -> &'static str {
        "project:
  name: model_browser_test
  version: 0.1.0
libraries:
  - libs/vendor/ti/regulators
board:
  components: {}
  nets: {}
"
    }

    #[test]
    fn model_browser_loads_project_library_entries() {
        let entries = model_browser_entries(project_yaml(), Path::new(".")).unwrap();
        assert!(
            entries
                .iter()
                .any(|entry| entry.id == "vendor.ti.tps54331_5v")
        );
    }

    #[test]
    fn model_browser_filters_by_id_and_feature() {
        let entries = model_browser_entries(project_yaml(), Path::new(".")).unwrap();
        let filtered = filtered_entries(&entries, "tps power");
        assert!(
            filtered
                .iter()
                .any(|entry| entry.id == "vendor.ti.tps54331_5v")
        );
        assert!(
            filtered
                .iter()
                .all(|entry| entry.id.contains("tps") || entry.features.contains(&"power"))
        );
    }
}
