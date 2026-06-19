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
    fn button_label(self) -> &'static str {
        match self {
            Self::Voltage => "V",
            Self::Current => "I",
            Self::Power => "P",
        }
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
}

impl CircuitCiApp {
    pub(super) fn schematic_scope_probe_tool_controls(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.label("Scope Tool");
        for tool in [
            SketchScopeProbeTool::Voltage,
            SketchScopeProbeTool::Current,
            SketchScopeProbeTool::Power,
        ] {
            let active = self.sketch_scope_probe_tool == Some(tool);
            if ui
                .selectable_label(active, tool.button_label())
                .on_hover_text(format!("{}: {}", tool.status_label(), tool.target_hint()))
                .clicked()
            {
                if active {
                    self.cancel_scope_probe_tool();
                } else {
                    self.arm_scope_probe_tool(tool);
                }
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
        self.status = format!("{} armed: {}.", tool.status_label(), tool.target_hint());
    }

    pub(super) fn cancel_scope_probe_tool(&mut self) {
        self.sketch_scope_probe_tool = None;
        self.status = "Scope probe tool canceled.".to_string();
    }

    pub(super) fn scope_probe_tool_armed(&self) -> bool {
        self.sketch_scope_probe_tool.is_some()
    }

    pub(super) fn apply_scope_probe_tool_to_selection(
        &mut self,
        selection: Option<SketchSelection>,
    ) -> bool {
        let Some(tool) = self.sketch_scope_probe_tool else {
            return false;
        };
        let Some(selection) = selection else {
            self.status = format!("{} armed: {}.", tool.status_label(), tool.target_hint());
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
