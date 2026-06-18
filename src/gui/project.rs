use super::sketch::{
    load_project_snapshot, load_project_snapshot_from_yaml, validate_board_ir_yaml_text,
};
use super::{CircuitCiApp, Stage};
use anyhow::Context;
use std::path::{Path, PathBuf};

const PROJECT_YAML_HISTORY_LIMIT: usize = 64;

impl CircuitCiApp {
    pub(super) fn load_project_summary(&mut self) {
        match load_project_snapshot(Path::new(&self.project_path)) {
            Ok(snapshot) => {
                let loaded_name = snapshot.name.clone();
                self.status = format!("Loaded {}", snapshot.name);
                self.project_snapshot = Some(snapshot);
                self.selected_sketch_item = None;
                if !self.project_yaml_dirty {
                    self.load_project_yaml();
                }
                self.push_diagnostic(&format!("Project summary loaded for {loaded_name}."));
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn load_project_yaml(&mut self) {
        match std::fs::read_to_string(Path::new(&self.project_path))
            .with_context(|| format!("Failed to read {}.", self.project_path))
            .and_then(|text| {
                validate_board_ir_yaml_text(&text)?;
                Ok(text)
            }) {
            Ok(text) => {
                self.project_yaml = text;
                self.project_yaml_dirty = false;
                self.clear_project_yaml_history();
                self.stage = Stage::Sketch;
                self.status = "Project YAML loaded.".to_string();
                self.push_diagnostic("Project YAML loaded into Sketch workspace.");
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn save_project_yaml(&mut self) {
        match validate_board_ir_yaml_text(&self.project_yaml).and_then(|()| {
            std::fs::write(Path::new(&self.project_path), &self.project_yaml)
                .with_context(|| format!("Failed to write {}.", self.project_path))
        }) {
            Ok(()) => {
                self.project_yaml_dirty = false;
                match load_project_snapshot_from_yaml(&self.project_yaml) {
                    Ok(snapshot) => {
                        self.project_snapshot = Some(snapshot);
                    }
                    Err(error) => {
                        self.record_error(error);
                        return;
                    }
                }
                self.status = "Project YAML saved.".to_string();
                self.push_diagnostic("Project YAML saved after schema parse validation.");
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn validate_project_yaml_text(&mut self) {
        match validate_board_ir_yaml_text(&self.project_yaml) {
            Ok(()) => {
                self.status = "Project YAML parses.".to_string();
                self.push_diagnostic("Project YAML parse validation passed.");
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn apply_edited_project_yaml(&mut self, updated: String, message: &str) {
        match load_project_snapshot_from_yaml(&updated) {
            Ok(snapshot) => {
                self.push_project_yaml_undo(self.project_yaml.clone());
                self.project_yaml = updated;
                self.project_yaml_dirty = true;
                self.project_snapshot = Some(snapshot);
                self.status = message.to_string();
                self.push_diagnostic(message);
            }
            Err(error) => self.record_error(error),
        }
    }

    pub(super) fn record_project_yaml_text_edit(&mut self, previous_yaml: String) {
        self.push_project_yaml_undo(previous_yaml);
        self.project_yaml_dirty = true;
        if let Ok(snapshot) = load_project_snapshot_from_yaml(&self.project_yaml) {
            self.project_snapshot = Some(snapshot);
        }
    }

    pub(super) fn undo_project_yaml_edit(&mut self) {
        let Some(previous) = self.project_yaml_undo.pop() else {
            return;
        };
        push_limited_history(
            &mut self.project_yaml_redo,
            self.project_yaml.clone(),
            PROJECT_YAML_HISTORY_LIMIT,
        );
        self.restore_project_yaml_history_entry(previous, "Undo applied.");
    }

    pub(super) fn redo_project_yaml_edit(&mut self) {
        let Some(next) = self.project_yaml_redo.pop() else {
            return;
        };
        push_limited_history(
            &mut self.project_yaml_undo,
            self.project_yaml.clone(),
            PROJECT_YAML_HISTORY_LIMIT,
        );
        self.restore_project_yaml_history_entry(next, "Redo applied.");
    }

    fn restore_project_yaml_history_entry(&mut self, yaml: String, message: &str) {
        self.project_yaml = yaml;
        self.project_yaml_dirty = true;
        if let Ok(snapshot) = load_project_snapshot_from_yaml(&self.project_yaml) {
            self.project_snapshot = Some(snapshot);
        }
        self.status = message.to_string();
        self.push_diagnostic(message);
    }

    fn push_project_yaml_undo(&mut self, previous_yaml: String) {
        if previous_yaml.is_empty() {
            return;
        }
        push_limited_history(
            &mut self.project_yaml_undo,
            previous_yaml,
            PROJECT_YAML_HISTORY_LIMIT,
        );
        self.project_yaml_redo.clear();
    }

    fn clear_project_yaml_history(&mut self) {
        self.project_yaml_undo.clear();
        self.project_yaml_redo.clear();
    }
}

pub(super) fn optional_path(text: &str) -> Option<PathBuf> {
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(Path::new(text).to_path_buf())
    }
}

pub(super) fn sanitized_project_name(path: &Path, fallback: &str) -> String {
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

fn push_limited_history(stack: &mut Vec<String>, value: String, limit: usize) {
    if stack.last().is_some_and(|last| last == &value) {
        return;
    }
    stack.push(value);
    if stack.len() > limit {
        stack.remove(0);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PROJECT_YAML_HISTORY_LIMIT, optional_path, push_limited_history, sanitized_project_name,
    };
    use crate::gui::CircuitCiApp;
    use crate::gui::sketch::{
        edit_component_model, edit_component_part_number, load_project_snapshot_from_yaml,
    };
    use std::path::Path;

    fn editable_project_yaml() -> &'static str {
        "project:
  name: gui_project_history_test
  version: 0.1.0
board:
  components:
    R1:
      model: generic.analog.resistor
      pins:
        A: net_a
        B: gnd
  nets:
    net_a:
      kind: digital_or_analog
    gnd:
      kind: ground
"
    }

    #[test]
    fn project_yaml_undo_redo_round_trips_validated_edit() {
        let mut app = CircuitCiApp {
            project_yaml: editable_project_yaml().to_string(),
            project_snapshot: Some(
                load_project_snapshot_from_yaml(editable_project_yaml()).unwrap(),
            ),
            ..CircuitCiApp::default()
        };
        let edited = edit_component_model(&app.project_yaml, "R1", "vendor.test.resistor").unwrap();
        app.apply_edited_project_yaml(edited, "edited");

        assert_eq!(app.project_yaml_undo.len(), 1);
        assert!(app.project_yaml.contains("vendor.test.resistor"));

        app.undo_project_yaml_edit();
        assert!(app.project_yaml.contains("generic.analog.resistor"));
        assert_eq!(app.project_yaml_redo.len(), 1);

        app.redo_project_yaml_edit();
        assert!(app.project_yaml.contains("vendor.test.resistor"));
        assert_eq!(app.project_yaml_undo.len(), 1);
    }

    #[test]
    fn project_yaml_branch_edit_clears_redo_stack() {
        let mut app = CircuitCiApp {
            project_yaml: editable_project_yaml().to_string(),
            project_snapshot: Some(
                load_project_snapshot_from_yaml(editable_project_yaml()).unwrap(),
            ),
            ..CircuitCiApp::default()
        };
        let edited = edit_component_model(&app.project_yaml, "R1", "vendor.test.resistor").unwrap();
        app.apply_edited_project_yaml(edited, "edited");
        app.undo_project_yaml_edit();
        assert_eq!(app.project_yaml_redo.len(), 1);

        let branched = edit_component_part_number(&app.project_yaml, "R1", "RC0603").unwrap();
        app.apply_edited_project_yaml(branched, "branched");
        assert!(app.project_yaml_redo.is_empty());
        assert!(app.project_yaml.contains("RC0603"));
    }

    #[test]
    fn project_yaml_history_is_capped() {
        let mut history = Vec::new();
        for index in 0..(PROJECT_YAML_HISTORY_LIMIT + 3) {
            push_limited_history(
                &mut history,
                format!("snapshot-{index}"),
                PROJECT_YAML_HISTORY_LIMIT,
            );
        }
        assert_eq!(history.len(), PROJECT_YAML_HISTORY_LIMIT);
        assert_eq!(history.first().unwrap(), "snapshot-3");
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
}
