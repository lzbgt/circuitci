use super::CircuitCiApp;
use super::project::PendingProjectAction;
use eframe::egui;
use std::path::{Path, PathBuf};

impl CircuitCiApp {
    pub(super) fn pick_project_path(&mut self) -> bool {
        pick_file_dialog(
            "Open Board IR Project",
            &self.project_path,
            &[("YAML", &["yaml", "yml"])],
        )
        .is_some_and(|path| {
            self.project_path = path_to_string(path);
            true
        })
    }

    pub(super) fn pick_and_request_project_load(&mut self, ctx: &egui::Context) {
        if self.pick_project_path() {
            self.request_project_action(
                PendingProjectAction::LoadProjectSummary {
                    path: self.project_path.clone(),
                },
                Some(ctx),
            );
        }
    }

    pub(super) fn pick_output_dir(&mut self) {
        if let Some(path) = pick_folder_dialog(&self.output_dir) {
            self.output_dir = path_to_string(path);
        }
    }

    pub(super) fn pick_import_schematic_path(&mut self) {
        if let Some(path) = pick_file_dialog(
            "Open KiCad Schematic",
            &self.import_schematic_path,
            &[("KiCad schematic", &["kicad_sch"])],
        ) {
            self.import_schematic_path = path_to_string(path);
        }
    }

    pub(super) fn pick_import_mapping_path(&mut self) {
        if let Some(path) = pick_file_dialog(
            "Open KiCad Mapping",
            &self.import_mapping_path,
            &[("YAML", &["yaml", "yml"])],
        ) {
            self.import_mapping_path = path_to_string(path);
        }
    }

    pub(super) fn pick_import_output_path(&mut self) {
        if let Some(path) = save_file_dialog(
            "Save Imported Board IR Project",
            &self.import_output_path,
            "imported.project.yaml",
            &[("YAML", &["yaml", "yml"])],
        ) {
            self.import_output_path = path_to_string(path);
        }
    }

    pub(super) fn pick_import_spice_deck_path(&mut self) {
        if let Some(path) = pick_file_dialog(
            "Open SPICE Deck",
            &self.import_spice_deck_path,
            &[("SPICE deck", &["cir", "sp", "spice", "net"])],
        ) {
            self.import_spice_deck_path = path_to_string(path);
        }
    }

    pub(super) fn pick_analog_model_file_path(&mut self) {
        if let Some(path) = pick_file_dialog(
            "Open SPICE Model Or Include",
            &self.analog_model_path,
            &[(
                "SPICE model/include",
                &["lib", "sub", "mod", "cir", "sp", "spice"],
            )],
        ) {
            self.analog_model_path = path_for_project_yaml(path, &self.project_path);
        }
    }

    pub(super) fn pick_import_spice_output_path(&mut self) {
        if let Some(path) = save_file_dialog(
            "Save Imported SPICE Project",
            &self.import_spice_output_path,
            "imported_spice.project.yaml",
            &[("YAML", &["yaml", "yml"])],
        ) {
            self.import_spice_output_path = path_to_string(path);
        }
    }

    pub(super) fn pick_import_pcb_path(&mut self) {
        if let Some(path) = pick_file_dialog(
            "Open KiCad PCB",
            &self.import_pcb_path,
            &[("KiCad PCB", &["kicad_pcb"])],
        ) {
            self.import_pcb_path = path_to_string(path);
        }
    }

    pub(super) fn pick_import_pcb_project_path(&mut self) {
        if let Some(path) = pick_file_dialog(
            "Open Board IR Project",
            &self.import_pcb_project_path,
            &[("YAML", &["yaml", "yml"])],
        ) {
            self.import_pcb_project_path = path_to_string(path);
        }
    }

    pub(super) fn pick_import_pcb_output_path(&mut self) {
        if let Some(path) = save_file_dialog(
            "Save PCB-Enriched Board IR Project",
            &self.import_pcb_output_path,
            "project_with_pcb.yaml",
            &[("YAML", &["yaml", "yml"])],
        ) {
            self.import_pcb_output_path = path_to_string(path);
        }
    }

    pub(super) fn pick_scope_snapshot_export_path(&mut self) -> Option<PathBuf> {
        save_file_dialog(
            "Export Scope Measurement Snapshots",
            &self.output_dir,
            "scope_measurement_snapshots.csv",
            &[("CSV", &["csv"])],
        )
    }
}

fn pick_file_dialog(
    title: &str,
    current_path: &str,
    filters: &[(&str, &[&str])],
) -> Option<PathBuf> {
    let mut dialog = file_dialog(title, current_path, filters);
    if let Some(file_name) = file_name_for_dialog(current_path) {
        dialog = dialog.set_file_name(file_name);
    }
    dialog.pick_file()
}

fn save_file_dialog(
    title: &str,
    current_path: &str,
    fallback_file_name: &str,
    filters: &[(&str, &[&str])],
) -> Option<PathBuf> {
    file_dialog(title, current_path, filters)
        .set_file_name(file_name_for_dialog(current_path).unwrap_or(fallback_file_name))
        .save_file()
}

fn pick_folder_dialog(current_path: &str) -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Choose Output Directory")
        .set_directory(directory_for_dialog(current_path))
        .pick_folder()
}

fn file_dialog(title: &str, current_path: &str, filters: &[(&str, &[&str])]) -> rfd::FileDialog {
    let mut dialog = rfd::FileDialog::new()
        .set_title(title)
        .set_directory(directory_for_dialog(current_path));
    for (label, extensions) in filters {
        dialog = dialog.add_filter(*label, extensions);
    }
    dialog
}

fn directory_for_dialog(current_path: &str) -> PathBuf {
    let path = Path::new(current_path.trim());
    if path.is_dir() {
        return path.to_path_buf();
    }
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn file_name_for_dialog(current_path: &str) -> Option<&str> {
    Path::new(current_path.trim())
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
}

fn path_to_string(path: PathBuf) -> String {
    path.to_string_lossy().into_owned()
}

fn path_for_project_yaml(path: PathBuf, project_path: &str) -> String {
    let project_dir = Path::new(project_path.trim())
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let Ok(canonical_project_dir) = project_dir.canonicalize() else {
        return path_to_string(path);
    };
    let Ok(canonical_path) = path.canonicalize() else {
        return path_to_string(path);
    };
    canonical_path
        .strip_prefix(&canonical_project_dir)
        .map(Path::to_path_buf)
        .map(path_to_string)
        .unwrap_or_else(|_| path_to_string(canonical_path))
}

#[cfg(test)]
mod tests {
    use super::{directory_for_dialog, file_name_for_dialog, path_for_project_yaml};
    use std::fs;
    use std::path::{Path, PathBuf};

    #[test]
    fn dialog_directory_uses_parent_for_file_paths() {
        assert_eq!(
            directory_for_dialog("out/gui/project.yaml"),
            Path::new("out/gui")
        );
        assert_eq!(directory_for_dialog("project.yaml"), PathBuf::from("."));
    }

    #[test]
    fn dialog_file_name_uses_last_path_segment() {
        assert_eq!(
            file_name_for_dialog("out/gui/project.yaml"),
            Some("project.yaml")
        );
        assert_eq!(file_name_for_dialog(""), None);
    }

    #[test]
    fn project_model_paths_are_relative_when_possible() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("project.yaml");
        let model_dir = dir.path().join("models");
        let model = model_dir.join("vendor.lib");
        fs::create_dir_all(&model_dir).unwrap();
        fs::write(&project, "").unwrap();
        fs::write(&model, "").unwrap();

        assert_eq!(
            path_for_project_yaml(model, project.to_str().unwrap()),
            "models/vendor.lib"
        );
    }
}
