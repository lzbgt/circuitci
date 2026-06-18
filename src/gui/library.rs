use super::CircuitCiApp;
use super::sketch::{
    SketchNodeStyle, SketchSelection, add_component_with_ports, edit_component_model,
    edit_schematic_component_style, edit_schematic_node_positions,
    persisted_node_position_from_screen_with_snap,
};
use anyhow::{Context, Result};
use eframe::egui;
use std::path::Path;

#[derive(Debug, Clone)]
struct ModelPortEntry {
    id: String,
    kind: String,
    required: bool,
}

#[derive(Debug, Clone)]
struct ModelBrowserEntry {
    id: String,
    category: String,
    source: String,
    confidence: String,
    ports: Vec<ModelPortEntry>,
    features: Vec<&'static str>,
}

impl ModelBrowserEntry {
    fn port_pairs(&self) -> Vec<(String, String)> {
        self.ports
            .iter()
            .map(|port| (port.id.clone(), port.kind.clone()))
            .collect()
    }
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

        let selected_entry = entries
            .iter()
            .find(|entry| entry.id == self.selected_library_model)
            .cloned();

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

        ui.horizontal(|ui| {
            ui.label("Insert ID");
            ui.text_edit_singleline(&mut self.new_component_id);
            let can_insert = selected_entry.is_some() && !self.new_component_id.trim().is_empty();
            if ui
                .add_enabled(can_insert, egui::Button::new("Insert Selected Model"))
                .clicked()
                && let Some(entry) = selected_entry.clone()
            {
                self.apply_insert_selected_library_model(entry);
            }
        });
        if let Some(entry) = &selected_entry {
            let ports = entry
                .ports
                .iter()
                .take(8)
                .map(|port| {
                    if port.required {
                        format!("{}:{}*", port.id, port.kind)
                    } else {
                        format!("{}:{}", port.id, port.kind)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            if ports.is_empty() {
                ui.label("Selected model declares no ports.");
            } else {
                ui.label(format!("Selected model ports: {ports}"));
            }
        }

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
                            ui.label(entry.ports.len().to_string());
                            ui.label(entry.features.join(", "));
                            if ui.button("Select").clicked() {
                                self.selected_library_model = entry.id.clone();
                                self.new_component_model = entry.id.clone();
                                self.new_component_id =
                                    next_component_id(&self.project_yaml, entry)
                                        .unwrap_or_else(|| self.new_component_id.clone());
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

    fn apply_insert_selected_library_model(&mut self, entry: ModelBrowserEntry) {
        match insert_library_model_component(&self.project_yaml, &self.new_component_id, &entry) {
            Ok(updated) => {
                let component_id = self.new_component_id.trim().to_string();
                self.selected_library_model = entry.id.clone();
                self.new_component_model = entry.id.clone();
                self.set_single_sketch_selection(Some(SketchSelection::Component(
                    component_id.clone(),
                )));
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Component {component_id} inserted from {}.", entry.id),
                );
                self.new_component_id =
                    next_component_id(&self.project_yaml, &entry).unwrap_or(component_id);
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn sketch_library_placement_panel(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Library Placement")
            .default_open(false)
            .show(ui, |ui| {
                if self.selected_library_model.trim().is_empty() {
                    ui.label("Select a model in the Library stage to place it here.");
                    if ui.button("Open Library").clicked() {
                        self.stage = super::Stage::Library;
                    }
                    return;
                }

                ui.horizontal_wrapped(|ui| {
                    ui.label("Model");
                    ui.monospace(&self.selected_library_model);
                });
                ui.horizontal(|ui| {
                    ui.label("ID");
                    ui.text_edit_singleline(&mut self.new_component_id);
                    if ui.button("Next").clicked() {
                        self.refresh_next_library_component_id();
                    }
                });
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !self.project_yaml.trim().is_empty()
                                && !self.new_component_id.trim().is_empty(),
                            egui::Button::new("Insert At View"),
                        )
                        .clicked()
                    {
                        self.apply_insert_selected_library_model_at_view();
                    }
                    let place_label = if self.sketch_library_place_armed {
                        "Click Canvas To Place"
                    } else {
                        "Place On Canvas"
                    };
                    let can_place = !self.project_yaml.trim().is_empty()
                        && !self.new_component_id.trim().is_empty();
                    let place_response = ui.add_enabled(
                        can_place,
                        egui::Button::new(place_label).sense(egui::Sense::click_and_drag()),
                    );
                    if place_response.drag_started() {
                        self.sketch_library_place_armed = true;
                        self.sketch_palette_place_armed = false;
                        self.sketch_net_label_place_armed = false;
                        self.status = format!(
                            "Drag to blank schematic space to place {}.",
                            self.selected_library_model
                        );
                    } else if place_response.clicked() {
                        self.sketch_library_place_armed = !self.sketch_library_place_armed;
                        if self.sketch_library_place_armed {
                            self.sketch_palette_place_armed = false;
                            self.sketch_net_label_place_armed = false;
                            self.status = format!(
                                "Click blank schematic space to place {}.",
                                self.selected_library_model
                            );
                        }
                    }
                    place_response.on_hover_text(format!(
                        "Click to arm placement, or drag {} onto blank schematic space.",
                        self.selected_library_model
                    ));
                    if self.sketch_library_place_armed && ui.button("Cancel").clicked() {
                        self.sketch_library_place_armed = false;
                        self.status = "Library placement canceled.".to_string();
                    }
                });
            });
    }

    pub(super) fn apply_insert_selected_library_model_at_view(&mut self) {
        let canvas = self.sketch_last_canvas_rect.unwrap_or_else(|| {
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(960.0, 640.0))
        });
        self.apply_insert_selected_library_model_at(canvas, canvas.center());
    }

    pub(super) fn apply_insert_selected_library_model_at(
        &mut self,
        canvas: egui::Rect,
        target: egui::Pos2,
    ) {
        let entry = match self.selected_library_entry() {
            Ok(entry) => entry,
            Err(error) => {
                self.record_error(error);
                return;
            }
        };
        if self.new_component_id.trim().is_empty() {
            self.new_component_id =
                next_component_id(&self.project_yaml, &entry).unwrap_or_else(|| "U1".to_string());
        }
        let component_id = self.new_component_id.trim().to_string();
        let node_rect = egui::Rect::from_center_size(target, egui::vec2(180.0, 92.0));
        let (x, y) = persisted_node_position_from_screen_with_snap(
            canvas,
            target,
            node_rect,
            self.sketch_viewport(),
            self.sketch_snap_enabled,
            self.sketch_grid_step,
        );
        match insert_library_model_component_at(
            &self.project_yaml,
            &component_id,
            &entry,
            x,
            y,
            self.placement_node_style(),
        ) {
            Ok(updated) => {
                self.selected_library_model = entry.id.clone();
                self.new_component_model = entry.id.clone();
                self.set_single_sketch_selection(Some(SketchSelection::Component(
                    component_id.clone(),
                )));
                self.apply_edited_project_yaml(
                    updated,
                    &format!("Component {component_id} inserted from {}.", entry.id),
                );
                self.new_component_id =
                    next_component_id(&self.project_yaml, &entry).unwrap_or(component_id);
                self.sketch_library_place_armed = false;
            }
            Err(error) => self.record_error(error),
        }
    }

    fn selected_library_entry(&self) -> Result<ModelBrowserEntry> {
        let selected = self.selected_library_model.trim();
        if selected.is_empty() {
            anyhow::bail!("Select a library model before placing a component.");
        }
        model_browser_entries(&self.project_yaml, Path::new(&self.project_path))?
            .into_iter()
            .find(|entry| entry.id == selected)
            .with_context(|| format!("Selected library model {selected} was not found."))
    }

    fn refresh_next_library_component_id(&mut self) {
        match self.selected_library_entry() {
            Ok(entry) => {
                if let Some(next) = next_component_id(&self.project_yaml, &entry) {
                    self.new_component_id = next;
                }
            }
            Err(error) => self.record_error(error),
        }
    }
}

fn insert_library_model_component(
    text: &str,
    component_id: &str,
    entry: &ModelBrowserEntry,
) -> Result<String> {
    add_component_with_ports(text, component_id, &entry.id, &entry.port_pairs())
}

fn insert_library_model_component_at(
    text: &str,
    component_id: &str,
    entry: &ModelBrowserEntry,
    x: f64,
    y: f64,
    style: SketchNodeStyle,
) -> Result<String> {
    let inserted = insert_library_model_component(text, component_id, entry)?;
    let positioned = edit_schematic_node_positions(
        &inserted,
        &[(
            SketchSelection::Component(component_id.trim().to_string()),
            x,
            y,
        )],
    )?;
    if style == SketchNodeStyle::default() {
        Ok(positioned)
    } else {
        edit_schematic_component_style(&positioned, component_id, style)
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
            ports: model
                .ports
                .iter()
                .map(|(id, port)| ModelPortEntry {
                    id: id.clone(),
                    kind: port_kind_name(&port.kind).to_string(),
                    required: port.required,
                })
                .collect(),
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
                "{} {} {} {} {} {}",
                entry.id,
                entry.category,
                entry.source,
                entry.confidence,
                entry
                    .ports
                    .iter()
                    .map(|port| format!("{} {}", port.id, port.kind))
                    .collect::<Vec<_>>()
                    .join(" "),
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

fn next_component_id(project_yaml: &str, entry: &ModelBrowserEntry) -> Option<String> {
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(project_yaml).ok()?;
    let prefix = component_prefix(entry);
    for index in 1..10_000 {
        let candidate = format!("{prefix}{index}");
        if !project.board.components.contains_key(&candidate) {
            return Some(candidate);
        }
    }
    None
}

fn component_prefix(entry: &ModelBrowserEntry) -> &'static str {
    let category = entry.category.to_ascii_lowercase();
    let id = entry.id.to_ascii_lowercase();
    if category.contains("connector") || id.contains("connector") || id.contains("jst") {
        "J"
    } else if category.contains("resistor") || id.contains("resistor") {
        "R"
    } else if category.contains("capacitor") || id.contains("capacitor") {
        "C"
    } else if category.contains("inductor") || id.contains("inductor") {
        "L"
    } else if category.contains("diode") || id.contains("diode") {
        "D"
    } else if category.contains("cable") || id.contains("cable") || id.contains("harness") {
        "W"
    } else if category.contains("motor") || id.contains("motor") {
        "M"
    } else {
        "U"
    }
}

fn port_kind_name(kind: &crate::library::PortKind) -> &'static str {
    match kind {
        crate::library::PortKind::ElectricalPower => "electrical_power",
        crate::library::PortKind::ElectricalGround => "electrical_ground",
        crate::library::PortKind::DigitalElectricalInput => "digital_electrical_input",
        crate::library::PortKind::DigitalElectricalOutput => "digital_electrical_output",
        crate::library::PortKind::DigitalElectricalIo => "digital_electrical_io",
        crate::library::PortKind::Passive => "passive",
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
    use super::{
        filtered_entries, insert_library_model_component_at, model_browser_entries,
        next_component_id,
    };
    use crate::gui::CircuitCiApp;
    use crate::gui::sketch::{SketchSelection, load_project_snapshot_from_yaml};
    use eframe::egui;
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

    #[test]
    fn model_browser_exposes_model_ports() {
        let entries = model_browser_entries(project_yaml(), Path::new(".")).unwrap();
        let entry = entries
            .iter()
            .find(|entry| entry.id == "vendor.ti.tps54331_5v")
            .unwrap();

        assert!(
            entry
                .ports
                .iter()
                .any(|port| port.id == "VIN" && port.kind == "electrical_power")
        );
        assert!(
            entry
                .ports
                .iter()
                .any(|port| port.id == "GND" && port.kind == "electrical_ground")
        );
    }

    #[test]
    fn next_component_id_uses_model_category_prefix() {
        let entries = model_browser_entries(project_yaml(), Path::new(".")).unwrap();
        let entry = entries
            .iter()
            .find(|entry| entry.id == "vendor.ti.tps54331_5v")
            .unwrap();

        assert_eq!(next_component_id(project_yaml(), entry).unwrap(), "U1");
    }

    #[test]
    fn library_model_insert_at_adds_position_and_default_pin_nets() {
        let entries = model_browser_entries(project_yaml(), Path::new(".")).unwrap();
        let entry = entries
            .iter()
            .find(|entry| entry.id == "vendor.ti.tps54331_5v")
            .unwrap();

        let edited = insert_library_model_component_at(
            project_yaml(),
            "U1",
            entry,
            144.0,
            96.0,
            Default::default(),
        )
        .unwrap();
        let snapshot = load_project_snapshot_from_yaml(&edited).unwrap();
        let component = snapshot
            .components_detail
            .iter()
            .find(|component| component.id == "U1")
            .unwrap();

        assert_eq!(component.model, "vendor.ti.tps54331_5v");
        assert!(component.pins.iter().any(|pin| pin.pin == "VIN"));
        assert_eq!(component.position.unwrap().x, 144.0);
        assert_eq!(component.position.unwrap().y, 96.0);
    }

    #[test]
    fn library_model_insert_at_persists_requested_schematic_rotation() {
        let entries = model_browser_entries(project_yaml(), Path::new(".")).unwrap();
        let entry = entries
            .iter()
            .find(|entry| entry.id == "vendor.ti.tps54331_5v")
            .unwrap();

        let edited = insert_library_model_component_at(
            project_yaml(),
            "U1",
            entry,
            144.0,
            96.0,
            crate::gui::sketch::SketchNodeStyle {
                rotation_deg: 270,
                ..Default::default()
            },
        )
        .unwrap();
        let snapshot = load_project_snapshot_from_yaml(&edited).unwrap();
        let component = snapshot
            .components_detail
            .iter()
            .find(|component| component.id == "U1")
            .unwrap();

        assert_eq!(component.style.rotation_deg, 270);
        assert!(edited.contains("component:U1:"));
        assert!(edited.contains("rotation_deg: 270"));
    }

    #[test]
    fn app_canvas_library_placement_inserts_at_clicked_position_and_disarms() {
        let canvas = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(640.0, 320.0));
        let mut app = CircuitCiApp {
            project_path: ".".to_string(),
            project_yaml: project_yaml().to_string(),
            selected_library_model: "vendor.ti.tps54331_5v".to_string(),
            new_component_id: "U1".to_string(),
            sketch_library_place_armed: true,
            sketch_placement_rotation_deg: 180,
            sketch_snap_enabled: false,
            ..Default::default()
        };

        app.apply_insert_selected_library_model_at(canvas, egui::pos2(300.0, 200.0));

        let snapshot = load_project_snapshot_from_yaml(&app.project_yaml).unwrap();
        let component = snapshot
            .components_detail
            .iter()
            .find(|component| component.id == "U1")
            .unwrap();
        let position = component.position.unwrap();
        assert_eq!(position.x, 210.0);
        assert_eq!(position.y, 154.0);
        assert_eq!(component.style.rotation_deg, 180);
        assert!(!app.sketch_library_place_armed);
        assert_eq!(
            app.selected_sketch_item,
            Some(SketchSelection::Component("U1".to_string()))
        );
    }
}
