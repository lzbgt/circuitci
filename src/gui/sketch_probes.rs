use eframe::egui;

use super::sketch::{ProjectSnapshot, SketchNode, SketchSelection};

#[derive(Debug, Clone)]
pub(super) struct SketchProbe {
    pub(super) scenario_name: String,
    pub(super) probe_name: String,
    pub(super) expression: String,
    pub(super) quantity: SketchProbeQuantity,
    pub(super) target: SketchProbeTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SketchProbeQuantity {
    Voltage,
    Current,
    Power,
}

impl SketchProbeQuantity {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Voltage => "V",
            Self::Current => "I",
            Self::Power => "P",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum SketchProbeTarget {
    Component(String),
    Net(String),
}

#[derive(Debug)]
pub(super) struct SketchProbeBadge {
    pub(super) probe: SketchProbe,
    pub(super) rect: egui::Rect,
}

pub(super) fn derive_project_probes(project: &crate::board_ir::BoardProject) -> Vec<SketchProbe> {
    let branch_targets = component_branch_targets(project);
    let mut probes = Vec::new();
    for scenario in &project.scenarios {
        let Some(analog) = scenario.analog.as_ref() else {
            continue;
        };
        let node_to_net: std::collections::BTreeMap<_, _> = analog
            .node_bindings
            .iter()
            .map(|binding| (binding.node.as_str(), binding.net.as_str()))
            .collect();
        for probe in &analog.probes {
            let quantity = sketch_probe_quantity(&probe.quantity);
            let target = match quantity {
                SketchProbeQuantity::Voltage => {
                    voltage_probe_target(&probe.expression, &node_to_net)
                }
                SketchProbeQuantity::Current | SketchProbeQuantity::Power => {
                    component_probe_target(&probe.expression, &branch_targets)
                }
            };
            let Some(target) = target else {
                continue;
            };
            probes.push(SketchProbe {
                scenario_name: scenario.name.clone(),
                probe_name: probe.name.clone(),
                expression: probe.expression.clone(),
                quantity,
                target,
            });
        }
    }
    probes
}

pub(super) fn layout_probe_badges(
    snapshot: &ProjectSnapshot,
    nodes: &[SketchNode],
) -> Vec<SketchProbeBadge> {
    let mut target_counts = std::collections::BTreeMap::<SketchProbeTarget, usize>::new();
    let mut badges = Vec::new();
    for probe in &snapshot.probes {
        let Some(node) = nodes
            .iter()
            .find(|node| match (&node.selection, &probe.target) {
                (SketchSelection::Component(node_id), SketchProbeTarget::Component(target_id)) => {
                    node_id == target_id
                }
                (SketchSelection::Net(node_id), SketchProbeTarget::Net(target_id)) => {
                    node_id == target_id
                }
                _ => false,
            })
        else {
            continue;
        };
        let offset_index = target_counts.entry(probe.target.clone()).or_insert(0);
        let rect = probe_badge_rect(node.rect, *offset_index, probe.quantity);
        *offset_index += 1;
        badges.push(SketchProbeBadge {
            probe: probe.clone(),
            rect,
        });
    }
    badges
}

pub(super) fn hit_test_probe_badge(
    badges: &[SketchProbeBadge],
    position: egui::Pos2,
) -> Option<&SketchProbeBadge> {
    badges
        .iter()
        .find(|badge| badge.rect.expand(2.0).contains(position))
}

pub(super) fn draw_probe_badge(painter: &egui::Painter, badge: &SketchProbeBadge, hovered: bool) {
    let fill = match badge.probe.quantity {
        SketchProbeQuantity::Voltage => egui::Color32::from_rgb(52, 100, 166),
        SketchProbeQuantity::Current => egui::Color32::from_rgb(136, 86, 154),
        SketchProbeQuantity::Power => egui::Color32::from_rgb(180, 112, 40),
    };
    let stroke_color = if hovered {
        egui::Color32::from_rgb(255, 226, 145)
    } else {
        egui::Color32::from_gray(36)
    };
    painter.rect_filled(badge.rect, 3.0, fill);
    painter.rect_stroke(
        badge.rect,
        3.0,
        egui::Stroke::new(if hovered { 2.0 } else { 1.0 }, stroke_color),
        egui::StrokeKind::Inside,
    );
    painter.text(
        badge.rect.center(),
        egui::Align2::CENTER_CENTER,
        badge.probe.quantity.label(),
        egui::FontId::monospace(11.0),
        egui::Color32::WHITE,
    );
}

fn sketch_probe_quantity(quantity: &crate::board_ir::AnalogQuantity) -> SketchProbeQuantity {
    match quantity {
        crate::board_ir::AnalogQuantity::Voltage => SketchProbeQuantity::Voltage,
        crate::board_ir::AnalogQuantity::Current => SketchProbeQuantity::Current,
        crate::board_ir::AnalogQuantity::Power => SketchProbeQuantity::Power,
    }
}

fn voltage_probe_target(
    expression: &str,
    node_to_net: &std::collections::BTreeMap<&str, &str>,
) -> Option<SketchProbeTarget> {
    let normalized = expression_without_whitespace(expression);
    let inner = normalized.strip_prefix("V(")?.strip_suffix(')')?;
    let node = inner.split(',').next()?.trim();
    node_to_net
        .get(node)
        .map(|net| SketchProbeTarget::Net((*net).to_string()))
}

fn component_probe_target(
    expression: &str,
    branch_targets: &std::collections::BTreeMap<String, String>,
) -> Option<SketchProbeTarget> {
    let normalized = expression_without_whitespace(expression).to_ascii_lowercase();
    branch_targets
        .iter()
        .find(|(branch, _)| normalized.contains(&format!("i({branch})")))
        .map(|(_, component_id)| SketchProbeTarget::Component(component_id.clone()))
}

fn component_branch_targets(
    project: &crate::board_ir::BoardProject,
) -> std::collections::BTreeMap<String, String> {
    let mut targets = std::collections::BTreeMap::new();
    for component_id in project.board.components.keys() {
        for prefix in ["V", "I"] {
            targets.insert(
                spice_element_name(prefix, component_id).to_ascii_lowercase(),
                component_id.clone(),
            );
        }
        for prefix in ["R", "C", "L", "D", "Q", "M"] {
            targets.insert(
                generated_current_sense_name(prefix, component_id).to_ascii_lowercase(),
                component_id.clone(),
            );
        }
    }
    targets
}

fn expression_without_whitespace(expression: &str) -> String {
    expression
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn probe_badge_rect(
    node_rect: egui::Rect,
    offset_index: usize,
    quantity: SketchProbeQuantity,
) -> egui::Rect {
    let size = match quantity {
        SketchProbeQuantity::Voltage => egui::vec2(22.0, 18.0),
        SketchProbeQuantity::Current => egui::vec2(22.0, 18.0),
        SketchProbeQuantity::Power => egui::vec2(24.0, 18.0),
    };
    let x = node_rect.right() - size.x - 6.0;
    let y = node_rect.top() + 6.0 + offset_index as f32 * (size.y + 3.0);
    egui::Rect::from_min_size(egui::pos2(x, y), size)
}

fn generated_current_sense_name(device_prefix: &str, component_id: &str) -> String {
    format!("VCCI_{}", spice_element_name(device_prefix, component_id))
}

fn spice_element_name(prefix: &str, component_id: &str) -> String {
    let suffix = spice_element_suffix(component_id);
    if suffix.starts_with(prefix) {
        suffix
    } else {
        format!("{prefix}{suffix}")
    }
}

fn spice_element_suffix(component_id: &str) -> String {
    let mut suffix = String::new();
    for character in component_id.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            suffix.push(character);
        } else {
            suffix.push('_');
        }
    }
    if suffix.is_empty() {
        suffix.push('X');
    }
    suffix
}
