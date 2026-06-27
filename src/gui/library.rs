use super::CircuitCiApp;
use super::kicad_symbol_library::{
    KiCadSymbolCatalogEntry, KiCadSymbolPin, import_kicad_symbol_file, kicad_symbol_catalog,
};
use super::library_observation_presets::create_component_observation_preset;
use super::sketch::{
    SketchNodeStyle, SketchSelection, add_component_with_ports, edit_component_model,
    edit_schematic_component_style, edit_schematic_node_positions,
    persisted_node_position_from_screen, persisted_node_position_from_screen_with_snap,
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
    simulation: Option<ModelSimulationEntry>,
}

impl ModelBrowserEntry {
    fn port_pairs(&self) -> Vec<(String, String)> {
        self.ports
            .iter()
            .map(|port| (port.id.clone(), port.kind.clone()))
            .collect()
    }
}

#[derive(Debug, Clone)]
struct ModelSimulationEntry {
    model_type: String,
    model_name: String,
    model_path: String,
    provenance: String,
    pin_order: Vec<String>,
    notes: Vec<String>,
}

impl ModelSimulationEntry {
    fn badge(&self) -> String {
        format!("SPICE {}", self.model_type)
    }

    fn search_text(&self) -> String {
        format!(
            "spice simulation generated observation {} {} {} {} {}",
            self.model_type,
            self.model_name,
            self.model_path,
            self.provenance,
            self.pin_order.join(" ")
        )
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

        self.kicad_symbol_browser(ui);
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
            if let Some(simulation) = &entry.simulation {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Simulation");
                    ui.strong(simulation.badge());
                    ui.label("model");
                    ui.monospace(&simulation.model_name);
                    ui.label("file");
                    ui.monospace(&simulation.model_path);
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label("Provenance");
                    ui.label(&simulation.provenance);
                    if !simulation.pin_order.is_empty() {
                        ui.label("pin order");
                        ui.monospace(simulation.pin_order.join(", "));
                    }
                });
                if let Some(note) = simulation.notes.first() {
                    ui.label(format!("Simulation note: {note}"));
                }
                let can_create_observation =
                    selected_component.as_ref().is_some_and(|component_id| {
                        component_uses_model(&self.project_yaml, component_id, &entry.id)
                    });
                if ui
                    .add_enabled(
                        can_create_observation,
                        egui::Button::new("Create Observation Preset"),
                    )
                    .clicked()
                    && let Some(component_id) = &selected_component
                {
                    self.apply_create_library_observation_preset(component_id);
                }
                if !can_create_observation {
                    ui.label("Select a placed component using this model to create a generated observation preset.");
                }
            } else {
                ui.label("Selected model has no generated-SPICE simulation face.");
            }
        }

        let filtered = filtered_entries(&entries, &self.model_search);
        ui.label(format!("{} of {} model(s)", filtered.len(), entries.len()));
        egui::ScrollArea::vertical()
            .max_height(280.0)
            .show(ui, |ui| {
                egui::Grid::new("model_browser_grid")
                    .num_columns(7)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Model");
                        ui.strong("Category");
                        ui.strong("Quality");
                        ui.strong("Simulation");
                        ui.strong("Ports");
                        ui.strong("Features");
                        ui.strong("Action");
                        ui.end_row();
                        for entry in filtered {
                            ui.monospace(&entry.id);
                            ui.label(&entry.category);
                            ui.label(format!("{} / {}", entry.source, entry.confidence));
                            if let Some(simulation) = &entry.simulation {
                                ui.label(simulation.badge());
                            } else {
                                ui.label("-");
                            }
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

    fn kicad_symbol_browser(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.strong("KiCad Symbol Library");
        ui.label(
            "Browse installed KiCad symbols or import a .kicad_sym file. Symbol placement is schematic display metadata; assign Board IR models separately for validation.",
        );
        ui.horizontal(|ui| {
            ui.label("Import file");
            ui.text_edit_singleline(&mut self.kicad_symbol_import_path);
            if ui.button("Import .kicad_sym").clicked() {
                self.import_kicad_symbol_library_file();
            }
        });
        if !self.imported_kicad_symbol_files.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.label("Imported");
                for path in &self.imported_kicad_symbol_files {
                    ui.monospace(path);
                }
            });
        }

        let (entries, diagnostics) = kicad_symbol_catalog(&self.imported_kicad_symbol_files);
        for diagnostic in diagnostics {
            ui.colored_label(egui::Color32::from_rgb(232, 160, 90), diagnostic);
        }
        ui.horizontal(|ui| {
            ui.label("Search");
            ui.text_edit_singleline(&mut self.kicad_symbol_search);
            if ui.button("Clear").clicked() {
                self.kicad_symbol_search.clear();
            }
        });

        let selected_component =
            selected_component_id(self.selected_sketch_item.as_ref()).map(str::to_string);
        let selected_entry = entries
            .iter()
            .find(|entry| entry.id == self.selected_kicad_symbol_id)
            .cloned();
        ui.horizontal_wrapped(|ui| {
            ui.label("Selected symbol");
            if self.selected_kicad_symbol_id.is_empty() {
                ui.label("none");
            } else {
                ui.monospace(&self.selected_kicad_symbol_id);
            }
            if let Some(entry) = &selected_entry {
                ui.label(format!("{} pin(s)", entry.pins.len()));
            }
        });
        ui.horizontal(|ui| {
            let can_apply =
                selected_component.is_some() && !self.selected_kicad_symbol_id.trim().is_empty();
            if ui
                .add_enabled(can_apply, egui::Button::new("Use Symbol For Selected"))
                .clicked()
                && let Some(component_id) = &selected_component
            {
                self.apply_selected_kicad_symbol(component_id.clone());
            }
            let can_insert = selected_entry.is_some() && !self.new_component_id.trim().is_empty();
            if ui
                .add_enabled(can_insert, egui::Button::new("Insert Symbol Component"))
                .clicked()
                && let Some(entry) = selected_entry.clone()
            {
                self.apply_insert_kicad_symbol_component_at_view(entry);
            }
        });
        ui.horizontal(|ui| {
            ui.label("Insert ID");
            ui.text_edit_singleline(&mut self.new_component_id);
            if ui.button("Next Symbol ID").clicked()
                && let Some(entry) = selected_entry.as_ref()
            {
                self.new_component_id = next_kicad_symbol_component_id(&self.project_yaml, entry)
                    .unwrap_or_else(|| self.new_component_id.clone());
            }
        });

        let filtered = filtered_kicad_symbol_entries(&entries, &self.kicad_symbol_search);
        ui.label(format!(
            "{} of {} KiCad symbol(s)",
            filtered.len(),
            entries.len()
        ));
        egui::ScrollArea::vertical()
            .max_height(220.0)
            .show(ui, |ui| {
                egui::Grid::new("kicad_symbol_browser_grid")
                    .num_columns(5)
                    .striped(true)
                    .show(ui, |ui| {
                        ui.strong("Symbol");
                        ui.strong("Library");
                        ui.strong("Pins");
                        ui.strong("Source");
                        ui.strong("Action");
                        ui.end_row();
                        for entry in filtered.into_iter().take(400) {
                            ui.monospace(&entry.name);
                            ui.label(&entry.library);
                            ui.label(entry.pins.len().to_string());
                            ui.label(compact_source_label(&entry.source));
                            if ui.button("Select").clicked() {
                                self.selected_kicad_symbol_id = entry.id.clone();
                                self.selected_library_model.clear();
                                self.new_component_model =
                                    "generic.schematic.imported_component".to_string();
                                self.new_component_id =
                                    next_kicad_symbol_component_id(&self.project_yaml, entry)
                                        .unwrap_or_else(|| self.new_component_id.clone());
                            }
                            ui.end_row();
                        }
                    });
            });
    }

    fn import_kicad_symbol_library_file(&mut self) {
        let path = self.kicad_symbol_import_path.trim();
        if path.is_empty() {
            self.status = "Enter a .kicad_sym path before importing symbols.".to_string();
            return;
        }
        match import_kicad_symbol_file(Path::new(path)) {
            Ok(entries) => {
                if !self
                    .imported_kicad_symbol_files
                    .iter()
                    .any(|entry| entry == path)
                {
                    self.imported_kicad_symbol_files.push(path.to_string());
                }
                self.status = format!("Imported {} KiCad symbol(s) from {path}.", entries.len());
            }
            Err(error) => self.record_error(error),
        }
    }

    fn apply_selected_kicad_symbol(&mut self, component_id: String) {
        match edit_schematic_component_symbol(
            &self.project_yaml,
            &component_id,
            &self.selected_kicad_symbol_id,
        ) {
            Ok(updated) => self.apply_edited_project_yaml(
                updated,
                &format!(
                    "Component {component_id} schematic symbol set to {}.",
                    self.selected_kicad_symbol_id
                ),
            ),
            Err(error) => self.record_error(error),
        }
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
                self.sketch_placement_orientation_controls(ui);
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

    fn apply_insert_kicad_symbol_component_at_view(&mut self, entry: KiCadSymbolCatalogEntry) {
        let canvas = self.sketch_last_canvas_rect.unwrap_or_else(|| {
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(960.0, 640.0))
        });
        self.apply_insert_kicad_symbol_component_at(entry, canvas, canvas.center());
    }

    fn apply_insert_kicad_symbol_component_at(
        &mut self,
        entry: KiCadSymbolCatalogEntry,
        canvas: egui::Rect,
        target: egui::Pos2,
    ) {
        if self.new_component_id.trim().is_empty() {
            self.new_component_id = next_kicad_symbol_component_id(&self.project_yaml, &entry)
                .unwrap_or_else(|| "U1".to_string());
        }
        let component_id = self.new_component_id.trim().to_string();
        let node_rect = egui::Rect::from_center_size(target, egui::vec2(180.0, 96.0));
        let (x, y) = persisted_node_position_from_screen_with_snap(
            canvas,
            target,
            node_rect,
            self.sketch_viewport(),
            self.sketch_snap_enabled,
            self.sketch_grid_step,
        );
        match insert_kicad_symbol_component_at(
            &self.project_yaml,
            &component_id,
            &entry,
            x,
            y,
            self.placement_node_style(),
        ) {
            Ok(updated) => {
                self.selected_kicad_symbol_id = entry.id.clone();
                self.selected_library_model.clear();
                self.new_component_model = "generic.schematic.imported_component".to_string();
                self.set_single_sketch_selection(Some(SketchSelection::Component(
                    component_id.clone(),
                )));
                self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Component {component_id} inserted with KiCad symbol {}.",
                        entry.id
                    ),
                );
                self.new_component_id = next_kicad_symbol_component_id(&self.project_yaml, &entry)
                    .unwrap_or(component_id);
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_insert_selected_library_model_at(
        &mut self,
        canvas: egui::Rect,
        target: egui::Pos2,
    ) {
        self.apply_insert_selected_library_model_at_with_snap(
            canvas,
            target,
            self.sketch_snap_enabled,
        );
    }

    pub(super) fn apply_insert_selected_library_model_at_with_snap(
        &mut self,
        canvas: egui::Rect,
        target: egui::Pos2,
        snap_enabled: bool,
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
        let (x, y) = if snap_enabled {
            persisted_node_position_from_screen_with_snap(
                canvas,
                target,
                node_rect,
                self.sketch_viewport(),
                snap_enabled,
                self.sketch_grid_step,
            )
        } else {
            persisted_node_position_from_screen(canvas, target, node_rect, self.sketch_viewport())
        };
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

    pub(super) fn apply_create_library_observation_preset(&mut self, component_id: &str) -> bool {
        match create_component_observation_preset(
            &self.project_yaml,
            Path::new(&self.project_path),
            component_id,
        ) {
            Ok(result) => {
                self.analog_generated_scenario = result.scenario_name.clone();
                self.apply_edited_project_yaml(
                    result.project_yaml,
                    &format!(
                        "Generated observation preset {} with {} voltage probe(s).",
                        result.scenario_name, result.probe_count
                    ),
                );
                true
            }
            Err(error) => {
                self.record_error(error);
                false
            }
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

fn insert_kicad_symbol_component_at(
    text: &str,
    component_id: &str,
    entry: &KiCadSymbolCatalogEntry,
    x: f64,
    y: f64,
    style: SketchNodeStyle,
) -> Result<String> {
    let ports = kicad_symbol_ports(entry);
    let inserted = add_component_with_ports(
        text,
        component_id,
        "generic.schematic.imported_component",
        &ports,
    )?;
    let positioned = edit_schematic_node_positions(
        &inserted,
        &[(
            SketchSelection::Component(component_id.trim().to_string()),
            x,
            y,
        )],
    )?;
    let styled = if style == SketchNodeStyle::default() {
        positioned
    } else {
        edit_schematic_component_style(&positioned, component_id, style)?
    };
    edit_schematic_component_symbol(&styled, component_id, &entry.id)
}

fn edit_schematic_component_symbol(
    text: &str,
    component_id: &str,
    symbol_id: &str,
) -> Result<String> {
    let component_id = component_id.trim();
    let symbol_id = symbol_id.trim();
    if component_id.is_empty() {
        anyhow::bail!("Component id must not be blank.");
    }
    if symbol_id.is_empty() {
        anyhow::bail!("KiCad symbol id must not be blank.");
    }
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    if !project.board.components.contains_key(component_id) {
        anyhow::bail!("Board IR component {component_id} was not found.");
    }
    let mut yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid YAML.")?;
    {
        let board = yaml
            .as_mapping_mut()
            .context("Board IR project must be a YAML object.")?
            .get_mut(serde_yaml_ng::Value::String("board".to_string()))
            .context("Board IR project is missing board.")?
            .as_mapping_mut()
            .context("Board IR field board must be an object.")?;
        let schematic =
            super::sketch::ensure_child_mapping_mut(board, "schematic", "board schematic")?;
        let symbols = super::sketch::ensure_child_mapping_mut(
            schematic,
            "component_symbols",
            "schematic component symbols",
        )?;
        symbols.insert(
            serde_yaml_ng::Value::String(component_id.to_string()),
            serde_yaml_ng::Value::String(symbol_id.to_string()),
        );
    }
    super::sketch::encode_edited_project_yaml(yaml)
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
            simulation: model_simulation_entry(model),
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
                "{} {} {} {} {} {} {}",
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
                entry.features.join(" "),
                entry
                    .simulation
                    .as_ref()
                    .map(ModelSimulationEntry::search_text)
                    .unwrap_or_default()
            )
            .to_ascii_lowercase();
            terms.iter().all(|term| haystack.contains(term))
        })
        .collect()
}

fn filtered_kicad_symbol_entries<'a>(
    entries: &'a [KiCadSymbolCatalogEntry],
    query: &str,
) -> Vec<&'a KiCadSymbolCatalogEntry> {
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
                "{} {} {} {}",
                entry.id,
                entry.library,
                entry.name,
                entry
                    .pins
                    .iter()
                    .map(|pin| format!("{} {} {}", pin.id, pin.name, pin.electrical_type))
                    .collect::<Vec<_>>()
                    .join(" ")
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

fn component_uses_model(project_yaml: &str, component_id: &str, model_id: &str) -> bool {
    let Ok(project) = serde_yaml_ng::from_str::<crate::board_ir::BoardProject>(project_yaml) else {
        return false;
    };
    project
        .board
        .components
        .get(component_id)
        .is_some_and(|component| component.model == model_id)
}

fn next_kicad_symbol_component_id(
    project_yaml: &str,
    entry: &KiCadSymbolCatalogEntry,
) -> Option<String> {
    let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(project_yaml).ok()?;
    let prefix = kicad_symbol_component_prefix(entry);
    for index in 1..10_000 {
        let candidate = format!("{prefix}{index}");
        if !project.board.components.contains_key(&candidate) {
            return Some(candidate);
        }
    }
    None
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

fn kicad_symbol_component_prefix(entry: &KiCadSymbolCatalogEntry) -> &'static str {
    let haystack = format!("{} {}", entry.library, entry.name).to_ascii_lowercase();
    if haystack.contains("connector") || haystack.contains("header") || haystack.contains("jack") {
        "J"
    } else if haystack.contains("resistor") || entry.name.starts_with('R') {
        "R"
    } else if haystack.contains("capacitor") || entry.name.starts_with('C') {
        "C"
    } else if haystack.contains("inductor") || entry.name.starts_with('L') {
        "L"
    } else if haystack.contains("diode") || entry.name.starts_with('D') {
        "D"
    } else if haystack.contains("transistor") || entry.name.starts_with('Q') {
        "Q"
    } else if haystack.contains("voltage") || haystack.contains("current") {
        "V"
    } else {
        "U"
    }
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

fn kicad_symbol_ports(entry: &KiCadSymbolCatalogEntry) -> Vec<(String, String)> {
    entry
        .pins
        .iter()
        .map(|pin| (pin.id.clone(), kicad_pin_kind_name(pin).to_string()))
        .collect()
}

fn kicad_pin_kind_name(pin: &KiCadSymbolPin) -> &'static str {
    match pin.electrical_type.as_str() {
        "input" => "digital_electrical_input",
        "output" => "digital_electrical_output",
        "bidirectional" | "tri_state" => "digital_electrical_io",
        "power_in" | "power_out" => "electrical_power",
        _ => "passive",
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

fn compact_source_label(source: &str) -> String {
    Path::new(source)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(source)
        .to_string()
}

fn model_features(model: &crate::library::ComponentModel) -> Vec<&'static str> {
    let mut features = Vec::new();
    if model.simulation.spice.is_some() {
        features.push("spice");
    }
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

fn model_simulation_entry(model: &crate::library::ComponentModel) -> Option<ModelSimulationEntry> {
    let spice = model.simulation.spice.as_ref()?;
    Some(ModelSimulationEntry {
        model_type: spice_model_type_label(&spice.model_type).to_string(),
        model_name: spice.model_name.clone(),
        model_path: spice.model_path.clone(),
        provenance: spice.provenance.clone(),
        pin_order: spice.pin_order.clone(),
        notes: spice.valid_operating_notes.clone(),
    })
}

fn spice_model_type_label(model_type: &crate::library::SpiceModelType) -> &'static str {
    match model_type {
        crate::library::SpiceModelType::BjtNpn => "BJT NPN",
        crate::library::SpiceModelType::BjtPnp => "BJT PNP",
        crate::library::SpiceModelType::MosfetN => "MOSFET N",
        crate::library::SpiceModelType::MosfetP => "MOSFET P",
        crate::library::SpiceModelType::Diode => "diode",
        crate::library::SpiceModelType::Subckt => "subckt",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ModelBrowserEntry, edit_schematic_component_symbol, filtered_entries,
        filtered_kicad_symbol_entries, insert_kicad_symbol_component_at,
        insert_library_model_component_at, kicad_symbol_ports, model_browser_entries,
        next_component_id, next_kicad_symbol_component_id,
    };
    use crate::gui::CircuitCiApp;
    use crate::gui::kicad_symbol_library::{KiCadSymbolCatalogEntry, KiCadSymbolPin};
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

    fn analog_project_yaml() -> &'static str {
        "project:
  name: model_browser_analog_test
  version: 0.1.0
libraries:
  - libs/generic/analog
  - libs/vendor/ti/reset_supervisors
board:
  components: {}
  nets: {}
"
    }

    fn kicad_resistor_entry() -> KiCadSymbolCatalogEntry {
        KiCadSymbolCatalogEntry {
            id: "Device:R".to_string(),
            library: "Device".to_string(),
            name: "R".to_string(),
            source: "Device.kicad_sym".to_string(),
            pins: vec![
                KiCadSymbolPin {
                    id: "1".to_string(),
                    name: "".to_string(),
                    electrical_type: "passive".to_string(),
                },
                KiCadSymbolPin {
                    id: "2".to_string(),
                    name: "".to_string(),
                    electrical_type: "passive".to_string(),
                },
            ],
        }
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
    fn model_browser_exposes_spice_simulation_readiness() {
        let entries = model_browser_entries(analog_project_yaml(), Path::new(".")).unwrap();
        let entry = entries
            .iter()
            .find(|entry| entry.id == "vendor.ti.tlv803ea29")
            .unwrap();
        let simulation = entry.simulation.as_ref().unwrap();

        assert!(entry.features.contains(&"spice"));
        assert_eq!(simulation.model_type, "subckt");
        assert_eq!(
            simulation.model_name,
            "CIRCUITCI_TLV803EA29_RESET_SUPERVISOR"
        );
        assert_eq!(
            simulation.model_path,
            "models/spice/generic/analog_behavioral.lib"
        );
        assert_eq!(simulation.pin_order, vec!["VDD", "GND", "RESET"]);
    }

    #[test]
    fn model_browser_filters_by_spice_metadata() {
        let entries = model_browser_entries(analog_project_yaml(), Path::new(".")).unwrap();
        let filtered = filtered_entries(&entries, "spice tlv803 subckt");

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "vendor.ti.tlv803ea29");
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
    fn kicad_symbol_filter_matches_library_symbol_and_pins() {
        let entries = vec![kicad_resistor_entry()];
        let filtered = filtered_kicad_symbol_entries(&entries, "device passive");

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "Device:R");
    }

    #[test]
    fn kicad_symbol_insert_at_persists_symbol_and_default_pin_nets() {
        let entry = kicad_resistor_entry();
        let edited = insert_kicad_symbol_component_at(
            project_yaml(),
            "R1",
            &entry,
            144.0,
            96.0,
            Default::default(),
        )
        .unwrap();
        let snapshot = load_project_snapshot_from_yaml(&edited).unwrap();
        let component = snapshot
            .components_detail
            .iter()
            .find(|component| component.id == "R1")
            .unwrap();

        assert_eq!(component.model, "generic.schematic.imported_component");
        assert_eq!(component.kicad_symbol_id.as_deref(), Some("Device:R"));
        assert!(component.pins.iter().any(|pin| pin.pin == "1"));
        assert!(component.pins.iter().any(|pin| pin.pin == "2"));
        assert!(edited.contains("component_symbols:"));
        assert!(edited.contains("R1: Device:R"));
    }

    #[test]
    fn kicad_symbol_can_be_applied_to_existing_component() {
        let with_component = super::insert_library_model_component(
            project_yaml(),
            "U1",
            &ModelBrowserEntry {
                id: "generic.schematic.imported_component".to_string(),
                category: "generic".to_string(),
                source: "builtin".to_string(),
                confidence: "low".to_string(),
                ports: Vec::new(),
                features: vec!["basic"],
                simulation: None,
            },
        )
        .unwrap();
        let edited =
            edit_schematic_component_symbol(&with_component, "U1", "Device:OpAmp").unwrap();
        let snapshot = load_project_snapshot_from_yaml(&edited).unwrap();
        let component = snapshot
            .components_detail
            .iter()
            .find(|component| component.id == "U1")
            .unwrap();

        assert_eq!(component.kicad_symbol_id.as_deref(), Some("Device:OpAmp"));
    }

    #[test]
    fn kicad_symbol_ports_keep_pin_numbers_and_kind_mapping() {
        let ports = kicad_symbol_ports(&KiCadSymbolCatalogEntry {
            id: "Device:Example".to_string(),
            library: "Device".to_string(),
            name: "Example".to_string(),
            source: "test".to_string(),
            pins: vec![
                KiCadSymbolPin {
                    id: "1".to_string(),
                    name: "IN".to_string(),
                    electrical_type: "input".to_string(),
                },
                KiCadSymbolPin {
                    id: "2".to_string(),
                    name: "VCC".to_string(),
                    electrical_type: "power_in".to_string(),
                },
            ],
        });

        assert_eq!(
            ports,
            vec![
                ("1".to_string(), "digital_electrical_input".to_string()),
                ("2".to_string(), "electrical_power".to_string())
            ]
        );
    }

    #[test]
    fn next_kicad_symbol_component_id_uses_symbol_prefix() {
        assert_eq!(
            next_kicad_symbol_component_id(project_yaml(), &kicad_resistor_entry()).unwrap(),
            "R1"
        );
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
            sketch_placement_mirrored: true,
            sketch_placement_pin_side: crate::gui::sketch::SketchPinSide::Right,
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
        assert!(component.style.mirrored);
        assert_eq!(
            component.style.pin_side,
            crate::gui::sketch::SketchPinSide::Right
        );
        assert!(!app.sketch_library_place_armed);
        assert_eq!(
            app.selected_sketch_item,
            Some(SketchSelection::Component("U1".to_string()))
        );
    }
}
