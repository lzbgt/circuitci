use crate::reports::{Finding, Limitation, ValidationReport};
use anyhow::{Context, Result};
use eframe::egui;
use std::path::Path;

pub fn run() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("CircuitCI")
            .with_inner_size([1280.0, 820.0]),
        ..Default::default()
    };
    eframe::run_native(
        "CircuitCI",
        options,
        Box::new(|_cc| Ok(Box::new(CircuitCiApp::default()))),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Project,
    Import,
    Sketch,
    Library,
    Simulation,
    Reports,
}

impl Stage {
    const ALL: [Self; 6] = [
        Self::Project,
        Self::Import,
        Self::Sketch,
        Self::Library,
        Self::Simulation,
        Self::Reports,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Project => "Project",
            Self::Import => "Import",
            Self::Sketch => "Sketch",
            Self::Library => "Library",
            Self::Simulation => "Simulation",
            Self::Reports => "Reports",
        }
    }
}

pub struct CircuitCiApp {
    project_path: String,
    output_dir: String,
    profile: String,
    import_schematic_path: String,
    import_mapping_path: String,
    import_output_path: String,
    import_project_name: String,
    import_default_model: String,
    import_pcb_path: String,
    import_pcb_project_path: String,
    import_pcb_output_path: String,
    stage: Stage,
    status: String,
    diagnostics: Vec<String>,
    report: Option<ValidationReport>,
    report_markdown: String,
    suggestions_yaml: String,
    project_yaml: String,
    project_yaml_dirty: bool,
    project_snapshot: Option<ProjectSnapshot>,
    waveforms: Vec<WaveformView>,
    selected_waveform: usize,
    selected_probe: usize,
}

impl Default for CircuitCiApp {
    fn default() -> Self {
        Self {
            project_path: "demos/smart_robot/circuitci/wheel_actuator/project.yaml".to_string(),
            output_dir: "out/gui".to_string(),
            profile: "default".to_string(),
            import_schematic_path: "demos/smart_robot/kicad/wheel_actuator/root.kicad_sch"
                .to_string(),
            import_mapping_path: "demos/smart_robot/kicad/wheel_actuator/circuitci.kicad-map.yaml"
                .to_string(),
            import_output_path: "out/gui_import/wheel_actuator_imported.project.yaml".to_string(),
            import_project_name: String::new(),
            import_default_model: "generic.schematic.imported_component".to_string(),
            import_pcb_path: "demos/smart_robot/kicad/wheel_actuator/wheel_actuator.kicad_pcb"
                .to_string(),
            import_pcb_project_path: "out/gui_import/wheel_actuator_imported.project.yaml"
                .to_string(),
            import_pcb_output_path: "out/gui_import/wheel_actuator_with_pcb.project.yaml"
                .to_string(),
            stage: Stage::Project,
            status: "Ready".to_string(),
            diagnostics: Vec::new(),
            report: None,
            report_markdown: String::new(),
            suggestions_yaml: String::new(),
            project_yaml: String::new(),
            project_yaml_dirty: false,
            project_snapshot: None,
            waveforms: Vec::new(),
            selected_waveform: 0,
            selected_probe: 0,
        }
    }
}

impl eframe::App for CircuitCiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.menu_bar(ctx);
        self.workflow_bar(ctx);
        self.left_panel(ctx);
        self.bottom_panel(ctx);
        self.central_panel(ctx);
    }
}

impl CircuitCiApp {
    fn menu_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::MenuBar::new().ui(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Import KiCad Schematic").clicked() {
                        self.import_kicad_schematic();
                        ui.close();
                    }
                    if ui.button("Import KiCad PCB").clicked() {
                        self.import_kicad_pcb();
                        ui.close();
                    }
                    if ui.button("Load Project").clicked() {
                        self.load_project_summary();
                        ui.close();
                    }
                    if ui.button("Load Project YAML").clicked() {
                        self.load_project_yaml();
                        ui.close();
                    }
                    if ui.button("Save Project YAML").clicked() {
                        self.save_project_yaml();
                        ui.close();
                    }
                    if ui.button("Validate").clicked() {
                        self.validate_project();
                        ui.close();
                    }
                    if ui.button("Suggest Scenarios").clicked() {
                        self.suggest_scenarios();
                        ui.close();
                    }
                });
                ui.menu_button("Workflow", |ui| {
                    for stage in Stage::ALL {
                        if ui.button(stage.label()).clicked() {
                            self.stage = stage;
                            ui.close();
                        }
                    }
                });
                ui.menu_button("Simulation", |ui| {
                    if ui.button("Run Validation + Analog Scenarios").clicked() {
                        self.validate_project();
                        self.stage = Stage::Simulation;
                        ui.close();
                    }
                });
                ui.menu_button("Help", |ui| {
                    ui.label("Native Rust desktop shell for CircuitCI.");
                    ui.label("The engine remains the CLI/library validation runtime.");
                });
            });
        });
    }

    fn workflow_bar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("workflow_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (index, stage) in Stage::ALL.iter().enumerate() {
                    let label = format!("{}. {}", index + 1, stage.label());
                    if ui.selectable_label(self.stage == *stage, label).clicked() {
                        self.stage = *stage;
                    }
                }
            });
        });
    }

    fn left_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("project_panel")
            .resizable(true)
            .default_width(310.0)
            .show(ctx, |ui| {
                ui.heading("CircuitCI");
                ui.separator();
                ui.label("Project");
                ui.text_edit_singleline(&mut self.project_path);
                ui.label("Output");
                ui.text_edit_singleline(&mut self.output_dir);
                ui.label("Profile");
                ui.text_edit_singleline(&mut self.profile);
                ui.horizontal(|ui| {
                    if ui.button("Load").clicked() {
                        self.load_project_summary();
                    }
                    if ui.button("Save").clicked() {
                        self.save_project_yaml();
                    }
                    if ui.button("Validate").clicked() {
                        self.validate_project();
                    }
                    if ui.button("Suggest").clicked() {
                        self.suggest_scenarios();
                    }
                });
                ui.separator();
                if let Some(snapshot) = &self.project_snapshot {
                    ui.label(format!("Name: {}", snapshot.name));
                    ui.label(format!("Components: {}", snapshot.components));
                    ui.label(format!("Nets: {}", snapshot.nets));
                    ui.label(format!("Scenarios: {}", snapshot.scenarios));
                    if self.project_yaml_dirty {
                        ui.label("YAML: unsaved edits");
                    }
                    if !snapshot.libraries.is_empty() {
                        ui.label("Libraries");
                        for library in &snapshot.libraries {
                            ui.monospace(library);
                        }
                    }
                } else {
                    ui.label("No project loaded.");
                }
            });
    }

    fn bottom_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("messages")
            .resizable(true)
            .default_height(120.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.strong("Status:");
                    ui.label(&self.status);
                });
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for diagnostic in self.diagnostics.iter().rev().take(40) {
                        ui.label(diagnostic);
                    }
                });
            });
    }

    fn central_panel(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| match self.stage {
            Stage::Project => self.project_stage(ui),
            Stage::Import => self.import_stage(ui),
            Stage::Sketch => self.sketch_stage(ui),
            Stage::Library => self.library_stage(ui),
            Stage::Simulation => self.simulation_stage(ui),
            Stage::Reports => self.reports_stage(ui),
        });
    }

    fn project_stage(&mut self, ui: &mut egui::Ui) {
        ui.heading("Project Workflow");
        ui.separator();
        ui.label("Load a Board IR project or an imported KiCad/EDA artifact, then run validation directly through the CircuitCI engine.");
        ui.add_space(8.0);
        if let Some(report) = &self.report {
            ui.horizontal(|ui| {
                ui.strong(format!("Result: {}", report.result));
                ui.label(format!(
                    "critical {} / warning {} / info {}",
                    report.summary.critical, report.summary.warning, report.summary.info
                ));
            });
        }
        ui.add_space(8.0);
        ui.label("Recommended flow");
        ui.label("Import or sketch -> bind library -> simulate/validate -> inspect reports -> revise design evidence.");
    }

    fn import_stage(&mut self, ui: &mut egui::Ui) {
        ui.heading("Import");
        ui.separator();
        ui.label("Import external CAD evidence into Board IR, then edit and validate the generated project.");
        ui.add_space(8.0);

        ui.group(|ui| {
            ui.strong("KiCad Schematic To Board IR");
            egui::Grid::new("kicad_schematic_import_grid")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("Schematic");
                    ui.text_edit_singleline(&mut self.import_schematic_path);
                    ui.end_row();
                    ui.label("Mapping");
                    ui.text_edit_singleline(&mut self.import_mapping_path);
                    ui.end_row();
                    ui.label("Output project");
                    ui.text_edit_singleline(&mut self.import_output_path);
                    ui.end_row();
                    ui.label("Project name");
                    ui.text_edit_singleline(&mut self.import_project_name);
                    ui.end_row();
                    ui.label("Default model");
                    ui.text_edit_singleline(&mut self.import_default_model);
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                if ui.button("Import Schematic").clicked() {
                    self.import_kicad_schematic();
                }
                if ui.button("Use As Project").clicked() {
                    self.project_path = self.import_output_path.clone();
                    self.load_project_summary();
                }
            });
        });

        ui.add_space(10.0);
        ui.group(|ui| {
            ui.strong("KiCad PCB Layout Evidence");
            egui::Grid::new("kicad_pcb_import_grid")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("PCB");
                    ui.text_edit_singleline(&mut self.import_pcb_path);
                    ui.end_row();
                    ui.label("Input project");
                    ui.text_edit_singleline(&mut self.import_pcb_project_path);
                    ui.end_row();
                    ui.label("Output project");
                    ui.text_edit_singleline(&mut self.import_pcb_output_path);
                    ui.end_row();
                });
            ui.horizontal(|ui| {
                if ui.button("Import PCB Evidence").clicked() {
                    self.import_kicad_pcb();
                }
                if ui.button("Use As Project").clicked() {
                    self.project_path = self.import_pcb_output_path.clone();
                    self.load_project_summary();
                }
            });
        });
    }

    fn sketch_stage(&mut self, ui: &mut egui::Ui) {
        ui.heading("Sketch Workspace");
        ui.separator();
        ui.label("Edit the Board IR YAML evidence directly, save it, then rerun validation and waveform observation through the same engine path.");
        ui.add_space(8.0);
        if let Some(snapshot) = &self.project_snapshot {
            egui::Grid::new("sketch_grid").striped(true).show(ui, |ui| {
                ui.label("Board graph");
                ui.label(format!(
                    "{} components, {} nets",
                    snapshot.components, snapshot.nets
                ));
                ui.end_row();
                ui.label("Scenario set");
                ui.label(format!("{} scenarios", snapshot.scenarios));
                ui.end_row();
            });
        }
        ui.horizontal(|ui| {
            if ui.button("Load YAML").clicked() {
                self.load_project_yaml();
            }
            if ui.button("Save YAML").clicked() {
                self.save_project_yaml();
            }
            if ui.button("Validate YAML").clicked() {
                self.validate_project_yaml_text();
            }
            if self.project_yaml_dirty {
                ui.label("Unsaved edits");
            }
        });
        ui.separator();
        if self.project_yaml.is_empty() {
            ui.label("Load a project to edit its Board IR YAML.");
        } else {
            let response = egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.project_yaml)
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(36)
                        .lock_focus(true),
                )
            });
            if response.inner.changed() {
                self.project_yaml_dirty = true;
            }
        }
    }

    fn library_stage(&mut self, ui: &mut egui::Ui) {
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

    fn simulation_stage(&mut self, ui: &mut egui::Ui) {
        ui.heading("Simulation And Observation");
        ui.separator();
        if self.report.is_some() {
            self.waveform_view(ui);
            ui.separator();
            let report = self.report.as_ref().expect("checked above");
            ui.label("Waveforms");
            if report.waveforms.is_empty() {
                ui.label("No waveform artifacts were emitted by the current scenario set.");
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
            ui.separator();
            self.findings_view(ui, report);
        } else {
            ui.label(
                "Run validation to observe SPICE waveforms, generated decks, and rule findings.",
            );
        }
    }

    fn reports_stage(&mut self, ui: &mut egui::Ui) {
        ui.heading("Reports");
        ui.separator();
        if self.report.is_some() {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut self.report_markdown)
                        .font(egui::TextStyle::Monospace)
                        .desired_rows(32)
                        .lock_focus(true),
                );
            });
        } else {
            ui.label("No report loaded.");
        }
    }

    fn findings_view(&self, ui: &mut egui::Ui, report: &ValidationReport) {
        finding_group(ui, "Critical", &report.failures);
        finding_group(ui, "Warnings", &report.warnings);
        finding_group(ui, "Info", &report.infos);
        limitation_group(ui, &report.limitations);
    }

    fn load_project_summary(&mut self) {
        match load_project_snapshot(Path::new(&self.project_path)) {
            Ok(snapshot) => {
                let loaded_name = snapshot.name.clone();
                self.status = format!("Loaded {}", snapshot.name);
                self.project_snapshot = Some(snapshot);
                if !self.project_yaml_dirty {
                    self.load_project_yaml();
                }
                self.push_diagnostic(&format!("Project summary loaded for {loaded_name}."));
            }
            Err(error) => self.record_error(error),
        }
    }

    fn import_kicad_schematic(&mut self) {
        let schematic = Path::new(&self.import_schematic_path).to_path_buf();
        let output = Path::new(&self.import_output_path).to_path_buf();
        let mapping = optional_path(&self.import_mapping_path);
        let name = if self.import_project_name.trim().is_empty() {
            sanitized_project_name(&schematic, "imported_kicad_project")
        } else {
            self.import_project_name.trim().to_string()
        };
        let options = crate::importers::kicad::KicadImportOptions {
            input: schematic,
            output: output.clone(),
            name,
            default_model: self.import_default_model.trim().to_string(),
            mapping,
        };
        match crate::importers::kicad_sch::import_kicad_schematic(&options) {
            Ok(()) => {
                self.project_path = output.to_string_lossy().into_owned();
                self.import_pcb_project_path = self.project_path.clone();
                self.status = "KiCad schematic imported.".to_string();
                self.push_diagnostic("KiCad schematic imported to Board IR.");
                self.load_project_summary();
            }
            Err(error) => self.record_error(error),
        }
    }

    fn import_kicad_pcb(&mut self) {
        let options = crate::importers::kicad_pcb::KicadPcbPlacementImportOptions {
            input: Path::new(&self.import_pcb_path).to_path_buf(),
            project: Path::new(&self.import_pcb_project_path).to_path_buf(),
            output: Path::new(&self.import_pcb_output_path).to_path_buf(),
        };
        match crate::importers::kicad_pcb::import_kicad_pcb_placements(&options) {
            Ok(summary) => {
                self.project_path = self.import_pcb_output_path.clone();
                self.status = "KiCad PCB evidence imported.".to_string();
                self.push_diagnostic(&format!(
                    "KiCad PCB imported: {} placements, {} pads, {} route segments, {} vias.",
                    summary.placements, summary.pads, summary.route_segments, summary.route_vias
                ));
                self.load_project_summary();
            }
            Err(error) => self.record_error(error),
        }
    }

    fn load_project_yaml(&mut self) {
        match std::fs::read_to_string(Path::new(&self.project_path))
            .with_context(|| format!("Failed to read {}.", self.project_path))
            .and_then(|text| {
                validate_board_ir_yaml_text(&text)?;
                Ok(text)
            }) {
            Ok(text) => {
                self.project_yaml = text;
                self.project_yaml_dirty = false;
                self.stage = Stage::Sketch;
                self.status = "Project YAML loaded.".to_string();
                self.push_diagnostic("Project YAML loaded into Sketch workspace.");
            }
            Err(error) => self.record_error(error),
        }
    }

    fn save_project_yaml(&mut self) {
        match validate_board_ir_yaml_text(&self.project_yaml).and_then(|()| {
            std::fs::write(Path::new(&self.project_path), &self.project_yaml)
                .with_context(|| format!("Failed to write {}.", self.project_path))
        }) {
            Ok(()) => {
                self.project_yaml_dirty = false;
                self.status = "Project YAML saved.".to_string();
                self.push_diagnostic("Project YAML saved after schema parse validation.");
                self.load_project_summary();
            }
            Err(error) => self.record_error(error),
        }
    }

    fn validate_project_yaml_text(&mut self) {
        match validate_board_ir_yaml_text(&self.project_yaml) {
            Ok(()) => {
                self.status = "Project YAML parses.".to_string();
                self.push_diagnostic("Project YAML parse validation passed.");
            }
            Err(error) => self.record_error(error),
        }
    }

    fn validate_project(&mut self) {
        match validate_from_gui(
            Path::new(&self.project_path),
            &self.profile,
            Path::new(&self.output_dir),
        ) {
            Ok((report, markdown)) => {
                let waveforms = load_report_waveforms(&report);
                let waveform_count = waveforms.len();
                self.status = format!("Validation {}", report.result);
                self.report_markdown = markdown;
                self.report = Some(report);
                self.waveforms = waveforms;
                self.selected_waveform = 0;
                self.selected_probe = 0;
                self.stage = if waveform_count == 0 {
                    Stage::Reports
                } else {
                    Stage::Simulation
                };
                self.push_diagnostic(&format!(
                    "Validation report written; loaded {waveform_count} waveform view(s)."
                ));
                self.load_project_summary();
            }
            Err(error) => self.record_error(error),
        }
    }

    fn suggest_scenarios(&mut self) {
        match suggest_from_gui(Path::new(&self.project_path), &self.profile) {
            Ok(yaml) => {
                self.status = "Scenario suggestions generated.".to_string();
                self.suggestions_yaml = yaml;
                self.stage = Stage::Library;
                self.push_diagnostic("Scenario suggestion YAML updated.");
                self.load_project_summary();
            }
            Err(error) => self.record_error(error),
        }
    }

    fn record_error(&mut self, error: anyhow::Error) {
        self.status = "Error".to_string();
        self.push_diagnostic(&format!("{error:#}"));
    }

    fn push_diagnostic(&mut self, message: &str) {
        self.diagnostics.push(message.to_string());
    }

    fn waveform_view(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.strong("Waveform Viewer");
            if self.waveforms.is_empty() {
                ui.label("No parsed CSV waveform is available.");
            }
        });
        if self.waveforms.is_empty() {
            return;
        }

        self.selected_waveform = self.selected_waveform.min(self.waveforms.len() - 1);
        ui.horizontal_wrapped(|ui| {
            for (index, waveform) in self.waveforms.iter().enumerate() {
                if ui
                    .selectable_label(self.selected_waveform == index, &waveform.label)
                    .clicked()
                {
                    self.selected_waveform = index;
                    self.selected_probe = 0;
                }
            }
        });

        let waveform = &self.waveforms[self.selected_waveform];
        if waveform.probes.is_empty() {
            ui.label("Waveform has no probe columns.");
            return;
        }

        self.selected_probe = self.selected_probe.min(waveform.probes.len() - 1);
        ui.horizontal_wrapped(|ui| {
            for (index, probe) in waveform.probes.iter().enumerate() {
                if ui
                    .selectable_label(self.selected_probe == index, &probe.label)
                    .clicked()
                {
                    self.selected_probe = index;
                }
            }
        });

        draw_waveform_plot(ui, waveform, self.selected_probe);
    }
}

#[derive(Debug, Clone)]
struct ProjectSnapshot {
    name: String,
    components: usize,
    nets: usize,
    scenarios: usize,
    libraries: Vec<String>,
}

#[derive(Debug, Clone)]
struct WaveformView {
    label: String,
    path: String,
    time_s: Vec<f64>,
    probes: Vec<WaveformProbe>,
}

#[derive(Debug, Clone)]
struct WaveformProbe {
    label: String,
    values: Vec<f64>,
}

fn load_project_snapshot(path: &Path) -> Result<ProjectSnapshot> {
    let project = crate::board_ir::load_project(path)?;
    Ok(ProjectSnapshot {
        name: project.project.name,
        components: project.board.components.len(),
        nets: project.board.nets.len(),
        scenarios: project.scenarios.len(),
        libraries: project
            .libraries
            .iter()
            .map(|library| library.to_string())
            .collect(),
    })
}

fn validate_board_ir_yaml_text(text: &str) -> Result<()> {
    let _project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    Ok(())
}

fn optional_path(text: &str) -> Option<std::path::PathBuf> {
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(Path::new(text).to_path_buf())
    }
}

fn sanitized_project_name(path: &Path, fallback: &str) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(sanitize_identifier)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

fn sanitize_identifier(value: &str) -> String {
    let mut result = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() || character == '_' || character == '-' {
            result.push(character);
        } else if !result.ends_with('_') {
            result.push('_');
        }
    }
    result.trim_matches('_').to_string()
}

fn validate_from_gui(
    project_path: &Path,
    profile: &str,
    output: &Path,
) -> Result<(ValidationReport, String)> {
    let command = format!(
        "circuitci-gui validate {} --profile {} --output {}",
        display_path(project_path),
        profile,
        display_path(output)
    );
    let report =
        crate::suite::validate_and_write_project_report(project_path, profile, output, command)?;
    let markdown = std::fs::read_to_string(output.join("report.md"))
        .with_context(|| format!("Failed to read {}.", output.join("report.md").display()))?;
    Ok((report, markdown))
}

fn load_report_waveforms(report: &ValidationReport) -> Vec<WaveformView> {
    report
        .waveforms
        .iter()
        .filter_map(|waveform| load_waveform_csv(Path::new(waveform), waveform).ok())
        .collect()
}

fn load_waveform_csv(path: &Path, label: &str) -> Result<WaveformView> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read waveform CSV {}.", path.display()))?;
    parse_waveform_csv_text(&text, label)
}

fn parse_waveform_csv_text(text: &str, label: &str) -> Result<WaveformView> {
    let mut time_s = Vec::new();
    let mut probe_labels = Vec::new();
    let mut probe_values: Vec<Vec<f64>> = Vec::new();

    for (line_index, line) in text.lines().enumerate() {
        let fields = split_waveform_fields(line);
        if fields.is_empty() {
            continue;
        }
        let Some(time) = parse_waveform_float(fields[0]) else {
            if time_s.is_empty() {
                probe_labels = fields
                    .iter()
                    .skip(1)
                    .map(|field| (*field).to_string())
                    .collect();
                continue;
            }
            anyhow::bail!(
                "Waveform row {} has non-numeric time value {}.",
                line_index + 1,
                fields[0]
            );
        };
        if let Some(previous) = time_s.last()
            && time <= *previous
        {
            anyhow::bail!(
                "Waveform row {} has non-increasing time value {}.",
                line_index + 1,
                fields[0]
            );
        }
        let probe_count = fields.len().saturating_sub(1);
        if probe_count == 0 {
            anyhow::bail!("Waveform row {} has no probe columns.", line_index + 1);
        }
        if probe_values.is_empty() {
            probe_values = vec![Vec::new(); probe_count];
            if probe_labels.len() != probe_count {
                probe_labels = (0..probe_count)
                    .map(|index| format!("probe_{}", index + 1))
                    .collect();
            }
        } else if probe_count < probe_values.len() {
            anyhow::bail!(
                "Waveform row {} has {} probe columns, expected at least {}.",
                line_index + 1,
                probe_count,
                probe_values.len()
            );
        }
        time_s.push(time);
        for (index, values) in probe_values.iter_mut().enumerate() {
            let value = parse_waveform_float(fields[index + 1]).with_context(|| {
                format!(
                    "Waveform row {} has non-numeric probe value {}.",
                    line_index + 1,
                    fields[index + 1]
                )
            })?;
            values.push(value);
        }
    }

    if time_s.is_empty() {
        anyhow::bail!("Waveform CSV has no numeric samples.");
    }

    let probes = probe_labels
        .into_iter()
        .zip(probe_values)
        .map(|(label, values)| WaveformProbe { label, values })
        .collect();
    Ok(WaveformView {
        label: label.to_string(),
        path: label.to_string(),
        time_s,
        probes,
    })
}

fn split_waveform_fields(line: &str) -> Vec<&str> {
    line.split(|character: char| character == ',' || character.is_whitespace())
        .filter(|field| !field.is_empty())
        .collect()
}

fn parse_waveform_float(value: &str) -> Option<f64> {
    value
        .parse::<f64>()
        .ok()
        .filter(|number| number.is_finite())
}

fn suggest_from_gui(project_path: &Path, profile: &str) -> Result<String> {
    let project = crate::board_ir::load_project(project_path)?;
    let (library, library_findings) = crate::library::load_library(project_path, &project);
    let bound = crate::library::bind_project(&project, library, library_findings);
    let profile = if profile.trim().is_empty() || profile == "default" {
        None
    } else {
        Some(profile)
    };
    let report = crate::scenario_suggestions::suggest_scenarios_for_profile(&bound, profile);
    serde_yaml_ng::to_string(&report).context("Failed to serialize scenario suggestions.")
}

fn draw_waveform_plot(ui: &mut egui::Ui, waveform: &WaveformView, probe_index: usize) {
    let probe = &waveform.probes[probe_index];
    let Some((x_min, x_max)) = min_max(&waveform.time_s) else {
        ui.label("Waveform has no time samples.");
        return;
    };
    let Some((y_min, y_max)) = min_max(&probe.values) else {
        ui.label("Selected probe has no samples.");
        return;
    };

    ui.label(format!(
        "{} samples from {}",
        waveform.time_s.len(),
        waveform.path
    ));
    let desired_size = egui::vec2(ui.available_width().max(360.0), 300.0);
    let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, egui::Color32::from_gray(16));

    let plot_rect = egui::Rect::from_min_max(
        rect.min + egui::vec2(56.0, 16.0),
        rect.max - egui::vec2(16.0, 38.0),
    );
    draw_plot_frame(&painter, plot_rect);

    let x_span = positive_span(x_min, x_max);
    let y_span = positive_span(y_min, y_max);
    let map_point = |x: f64, y: f64| -> egui::Pos2 {
        let x_ratio = ((x - x_min) / x_span).clamp(0.0, 1.0) as f32;
        let y_ratio = ((y - y_min) / y_span).clamp(0.0, 1.0) as f32;
        egui::pos2(
            plot_rect.left() + x_ratio * plot_rect.width(),
            plot_rect.bottom() - y_ratio * plot_rect.height(),
        )
    };

    for tick in 0..=4 {
        let ratio = tick as f32 / 4.0;
        let x = plot_rect.left() + ratio * plot_rect.width();
        painter.line_segment(
            [
                egui::pos2(x, plot_rect.top()),
                egui::pos2(x, plot_rect.bottom()),
            ],
            egui::Stroke::new(1.0, egui::Color32::from_gray(44)),
        );
        let y = plot_rect.top() + ratio * plot_rect.height();
        painter.line_segment(
            [
                egui::pos2(plot_rect.left(), y),
                egui::pos2(plot_rect.right(), y),
            ],
            egui::Stroke::new(1.0, egui::Color32::from_gray(44)),
        );
    }

    let points: Vec<_> = waveform
        .time_s
        .iter()
        .copied()
        .zip(probe.values.iter().copied())
        .map(|(x, y)| map_point(x, y))
        .collect();
    if points.len() >= 2 {
        painter.add(egui::Shape::line(
            points,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(93, 185, 255)),
        ));
    }

    let font = egui::FontId::monospace(12.0);
    painter.text(
        egui::pos2(plot_rect.left(), rect.bottom() - 22.0),
        egui::Align2::LEFT_CENTER,
        format!("t {:.3e}..{:.3e} s", x_min, x_max),
        font.clone(),
        egui::Color32::LIGHT_GRAY,
    );
    painter.text(
        egui::pos2(plot_rect.left(), rect.top() + 8.0),
        egui::Align2::LEFT_CENTER,
        format!("{} {:.3e}..{:.3e}", probe.label, y_min, y_max),
        font,
        egui::Color32::LIGHT_GRAY,
    );
}

fn draw_plot_frame(painter: &egui::Painter, rect: egui::Rect) {
    let stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(96));
    painter.line_segment([rect.left_top(), rect.right_top()], stroke);
    painter.line_segment([rect.right_top(), rect.right_bottom()], stroke);
    painter.line_segment([rect.right_bottom(), rect.left_bottom()], stroke);
    painter.line_segment([rect.left_bottom(), rect.left_top()], stroke);
}

fn min_max(values: &[f64]) -> Option<(f64, f64)> {
    let mut iter = values.iter().copied();
    let first = iter.next()?;
    let (min, max) = iter.fold((first, first), |(min, max), value| {
        (min.min(value), max.max(value))
    });
    Some((min, max))
}

fn positive_span(min: f64, max: f64) -> f64 {
    let span = max - min;
    if span.abs() < f64::EPSILON { 1.0 } else { span }
}

fn finding_group(ui: &mut egui::Ui, title: &str, findings: &[Finding]) {
    egui::CollapsingHeader::new(format!("{title} ({})", findings.len()))
        .default_open(!findings.is_empty())
        .show(ui, |ui| {
            for finding in findings {
                ui.group(|ui| {
                    ui.strong(&finding.id);
                    ui.label(&finding.scenario);
                    ui.label(&finding.message);
                    if !finding.suggested_fixes.is_empty() {
                        ui.label("Suggested fixes");
                        for fix in &finding.suggested_fixes {
                            ui.label(fix);
                        }
                    }
                });
            }
        });
}

fn limitation_group(ui: &mut egui::Ui, limitations: &[Limitation]) {
    egui::CollapsingHeader::new(format!("Limitations ({})", limitations.len())).show(ui, |ui| {
        for limitation in limitations {
            ui.group(|ui| {
                ui.strong(&limitation.id);
                ui.label(format!(
                    "{} / confidence {} / blocking {}",
                    limitation.scope, limitation.confidence, limitation.blocking
                ));
                ui.label(&limitation.message);
            });
        }
    });
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::{
        optional_path, parse_waveform_csv_text, sanitized_project_name, validate_board_ir_yaml_text,
    };
    use std::path::Path;

    #[test]
    fn board_ir_editor_accepts_minimal_project_yaml() {
        validate_board_ir_yaml_text(
            "project:
  name: gui_editor_test
  version: 0.1.0
board:
  components: {}
  nets: {}
",
        )
        .unwrap();
    }

    #[test]
    fn board_ir_editor_rejects_invalid_project_yaml() {
        let error = validate_board_ir_yaml_text(
            "project:
  name: gui_editor_test
",
        )
        .unwrap_err();
        assert!(error.to_string().contains("Board IR"));
    }

    #[test]
    fn optional_path_ignores_blank_mapping_path() {
        assert!(optional_path("  ").is_none());
        assert_eq!(
            optional_path("mapping.yaml").unwrap(),
            Path::new("mapping.yaml").to_path_buf()
        );
    }

    #[test]
    fn sanitized_project_name_uses_file_stem() {
        assert_eq!(
            sanitized_project_name(Path::new("some dir/root.kicad_sch"), "fallback"),
            "root"
        );
        assert_eq!(
            sanitized_project_name(Path::new("bad name!.kicad_sch"), "fallback"),
            "bad_name"
        );
    }

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
}
