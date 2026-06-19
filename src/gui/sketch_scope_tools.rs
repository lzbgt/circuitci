use eframe::egui;

use super::CircuitCiApp;
use super::sketch::SketchSelection;
use super::sketch_probes::SketchProbeQuantity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::gui) enum SketchScopeProbeTool {
    Voltage,
    Current,
    Power,
}

impl SketchScopeProbeTool {
    pub(super) const ALL: [Self; 3] = [Self::Voltage, Self::Current, Self::Power];

    pub(super) fn button_label(self) -> &'static str {
        match self {
            Self::Voltage => "V",
            Self::Current => "I",
            Self::Power => "P",
        }
    }

    pub(super) fn shortcut_key(self) -> egui::Key {
        match self {
            Self::Voltage => egui::Key::V,
            Self::Current => egui::Key::I,
            Self::Power => egui::Key::P,
        }
    }

    pub(super) fn shortcut_label(self) -> &'static str {
        self.button_label()
    }

    pub(super) fn from_unmodified_shortcut(input: &egui::InputState) -> Option<Self> {
        if !input.modifiers.is_none() {
            return None;
        }
        Self::ALL
            .into_iter()
            .find(|tool| input.key_pressed(tool.shortcut_key()))
    }

    fn status_label(self) -> &'static str {
        match self {
            Self::Voltage => "Scope voltage",
            Self::Current => "Scope current",
            Self::Power => "Scope power",
        }
    }

    fn target_hint(self) -> &'static str {
        match self {
            Self::Voltage => "click a net, wire, pin, or net label",
            Self::Current | Self::Power => "click a component, pin, or component label",
        }
    }

    fn armed_status(self) -> String {
        format!(
            "{} armed: {}. Press {} again or Esc to cancel.",
            self.status_label(),
            self.target_hint(),
            self.shortcut_label()
        )
    }

    pub(super) fn accepts_selection(self, selection: &SketchSelection) -> bool {
        matches!(
            (self, selection),
            (Self::Voltage, SketchSelection::Net(_))
                | (Self::Current | Self::Power, SketchSelection::Component(_))
        )
    }

    pub(super) fn target_label(self, selection: Option<&SketchSelection>) -> String {
        match selection {
            Some(SketchSelection::Net(net_id)) if self == Self::Voltage => {
                format!("Scope V -> {net_id}")
            }
            Some(SketchSelection::Component(component_id))
                if matches!(self, Self::Current | Self::Power) =>
            {
                format!("Scope {} -> {component_id}", self.button_label())
            }
            Some(SketchSelection::Net(net_id)) => {
                format!("{} cannot attach to net {net_id}", self.status_label())
            }
            Some(SketchSelection::Component(component_id)) => {
                format!(
                    "{} cannot attach to component {component_id}",
                    self.status_label()
                )
            }
            Some(SketchSelection::Overflow(label)) => {
                format!("{} cannot attach to bundle {label}", self.status_label())
            }
            None => format!("{}: {}", self.status_label(), self.target_hint()),
        }
    }
}

impl CircuitCiApp {
    pub(super) fn schematic_scope_probe_tool_controls(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.label("Scope Tool");
        for tool in SketchScopeProbeTool::ALL {
            let active = self.sketch_scope_probe_tool == Some(tool);
            if ui
                .selectable_label(active, tool.button_label())
                .on_hover_text(format!(
                    "{}: {}. Shortcut: {}",
                    tool.status_label(),
                    tool.target_hint(),
                    tool.shortcut_label()
                ))
                .clicked()
            {
                self.toggle_scope_probe_tool(tool);
            }
        }
        if self.sketch_scope_probe_tool.is_some() && ui.button("Off").clicked() {
            self.cancel_scope_probe_tool();
        }
    }

    pub(super) fn arm_scope_probe_tool(&mut self, tool: SketchScopeProbeTool) {
        self.sketch_scope_probe_tool = Some(tool);
        self.sketch_palette_place_armed = false;
        self.sketch_library_place_armed = false;
        self.sketch_net_label_place_armed = false;
        self.wire_from_component = None;
        self.sketch_wire_draft.clear();
        self.status = tool.armed_status();
    }

    pub(super) fn toggle_scope_probe_tool(&mut self, tool: SketchScopeProbeTool) {
        if self.sketch_scope_probe_tool == Some(tool) {
            self.cancel_scope_probe_tool();
        } else {
            self.arm_scope_probe_tool(tool);
        }
    }

    pub(super) fn cancel_scope_probe_tool(&mut self) {
        self.sketch_scope_probe_tool = None;
        self.status = "Scope probe tool canceled.".to_string();
    }

    pub(super) fn scope_probe_tool_armed(&self) -> bool {
        self.sketch_scope_probe_tool.is_some()
    }

    pub(super) fn active_scope_probe_tool(&self) -> Option<SketchScopeProbeTool> {
        self.sketch_scope_probe_tool
    }

    pub(super) fn apply_scope_probe_tool_to_selection(
        &mut self,
        selection: Option<SketchSelection>,
    ) -> bool {
        let Some(tool) = self.sketch_scope_probe_tool else {
            return false;
        };
        let Some(selection) = selection else {
            self.status = tool.armed_status();
            return true;
        };
        match (tool, selection) {
            (SketchScopeProbeTool::Voltage, SketchSelection::Net(net_id)) => {
                self.open_or_create_scope_voltage_probe_for_net(&net_id);
                self.sketch_scope_probe_tool = None;
                true
            }
            (SketchScopeProbeTool::Current, SketchSelection::Component(component_id)) => {
                self.open_or_create_scope_component_probe(
                    &component_id,
                    SketchProbeQuantity::Current,
                );
                self.sketch_scope_probe_tool = None;
                true
            }
            (SketchScopeProbeTool::Power, SketchSelection::Component(component_id)) => {
                self.open_or_create_scope_component_probe(
                    &component_id,
                    SketchProbeQuantity::Power,
                );
                self.sketch_scope_probe_tool = None;
                true
            }
            (SketchScopeProbeTool::Voltage, _) => {
                self.status =
                    "Scope voltage tool needs a net, wire, pin, or net label.".to_string();
                true
            }
            (SketchScopeProbeTool::Current, _) => {
                self.status =
                    "Scope current tool needs a component, pin, or component label.".to_string();
                true
            }
            (SketchScopeProbeTool::Power, _) => {
                self.status =
                    "Scope power tool needs a component, pin, or component label.".to_string();
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SketchScopeProbeTool;
    use crate::gui::CircuitCiApp;
    use crate::gui::sketch::SketchSelection;
    use eframe::egui;

    #[test]
    fn scope_probe_tool_shortcuts_are_plain_v_i_p() {
        assert_eq!(SketchScopeProbeTool::Voltage.shortcut_key(), egui::Key::V);
        assert_eq!(SketchScopeProbeTool::Current.shortcut_key(), egui::Key::I);
        assert_eq!(SketchScopeProbeTool::Power.shortcut_key(), egui::Key::P);
        assert_eq!(SketchScopeProbeTool::Voltage.shortcut_label(), "V");
        assert_eq!(SketchScopeProbeTool::Current.shortcut_label(), "I");
        assert_eq!(SketchScopeProbeTool::Power.shortcut_label(), "P");
    }

    #[test]
    fn scope_probe_tool_accepts_matching_targets() {
        assert!(
            SketchScopeProbeTool::Voltage.accepts_selection(&SketchSelection::Net("out".into()))
        );
        assert!(
            SketchScopeProbeTool::Current
                .accepts_selection(&SketchSelection::Component("V1".into()))
        );
        assert!(
            SketchScopeProbeTool::Power
                .accepts_selection(&SketchSelection::Component("RLOAD".into()))
        );
        assert!(
            !SketchScopeProbeTool::Voltage
                .accepts_selection(&SketchSelection::Component("V1".into()))
        );
        assert!(
            !SketchScopeProbeTool::Current.accepts_selection(&SketchSelection::Net("out".into()))
        );
    }

    #[test]
    fn scope_probe_tool_target_labels_explain_valid_and_invalid_hover() {
        assert_eq!(
            SketchScopeProbeTool::Voltage.target_label(Some(&SketchSelection::Net("out".into()))),
            "Scope V -> out"
        );
        assert_eq!(
            SketchScopeProbeTool::Current
                .target_label(Some(&SketchSelection::Component("V1".into()))),
            "Scope I -> V1"
        );
        assert!(
            SketchScopeProbeTool::Power
                .target_label(Some(&SketchSelection::Net("out".into())))
                .contains("cannot attach")
        );
        assert!(
            SketchScopeProbeTool::Voltage
                .target_label(None)
                .contains("click a net")
        );
    }

    #[test]
    fn scope_probe_tool_toggle_status_mentions_shortcut_cancel() {
        let mut app = CircuitCiApp::default();
        app.toggle_scope_probe_tool(SketchScopeProbeTool::Voltage);
        assert_eq!(
            app.sketch_scope_probe_tool,
            Some(SketchScopeProbeTool::Voltage)
        );
        assert!(app.status.contains("Scope voltage armed"));
        assert!(app.status.contains("Press V again or Esc"));

        app.toggle_scope_probe_tool(SketchScopeProbeTool::Voltage);
        assert_eq!(app.sketch_scope_probe_tool, None);
        assert_eq!(app.status, "Scope probe tool canceled.");

        app.toggle_scope_probe_tool(SketchScopeProbeTool::Power);
        assert_eq!(
            app.sketch_scope_probe_tool,
            Some(SketchScopeProbeTool::Power)
        );
        assert!(app.status.contains("click a component"));
        assert!(app.status.contains("Press P again or Esc"));
    }
}
