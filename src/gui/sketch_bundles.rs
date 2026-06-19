use eframe::egui;
use std::collections::{BTreeMap, BTreeSet};

use super::CircuitCiApp;
use super::sketch::{ProjectSnapshot, SketchGraph, SketchSelection, with_opacity};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SketchNetBundle {
    pub(super) label: String,
    pub(super) members: Vec<String>,
}

#[derive(Debug, Clone)]
pub(super) struct SketchNetBundleBadge {
    pub(super) bundle: SketchNetBundle,
    pub(super) rect: egui::Rect,
    pub(super) spine_x: f32,
    pub(super) y_min: f32,
    pub(super) y_max: f32,
    pub(super) member_points: Vec<egui::Pos2>,
}

impl CircuitCiApp {
    pub(super) fn sketch_overlay_panel(&mut self, ui: &mut egui::Ui, snapshot: &ProjectSnapshot) {
        egui::CollapsingHeader::new("Circuit View")
            .default_open(true)
            .show(ui, |ui| {
                let bundle_count = derive_net_bundles(snapshot).len();
                ui.checkbox(
                    &mut self.sketch_net_bundles_visible,
                    format!("Show derived net bundles ({bundle_count})"),
                )
                .on_hover_text(
                    "Derived bundles are navigation overlays for similarly named nets. They are not components, pins, or required circuit connections.",
                );
                ui.label("Default view shows the connected schematic network; enable overlays only when auditing imported buses or grouped nets.");
                ui.separator();
                ui.checkbox(
                    &mut self.sketch_runtime_scope_overlay_visible,
                    "Show runtime scope activity",
                )
                .on_hover_text(
                    "Shows transient tinting and clickable scope chips for loaded waveform traces. This is a runtime observation overlay, not a circuit element.",
                );
            });
    }

    pub(super) fn select_net_bundle(&mut self, bundle: &SketchNetBundle) {
        self.selected_sketch_items = bundle
            .members
            .iter()
            .cloned()
            .map(SketchSelection::Net)
            .collect();
        self.selected_sketch_item = self.selected_sketch_items.iter().next().cloned();
        self.status = format!(
            "Selected net bundle {} ({} nets).",
            bundle.label,
            bundle.members.len()
        );
    }
}

pub(super) fn derive_net_bundles(snapshot: &ProjectSnapshot) -> Vec<SketchNetBundle> {
    let mut groups = BTreeMap::<String, BTreeSet<String>>::new();
    for net in &snapshot.nets_detail {
        if let Some(label) = net_bundle_label(&net.id) {
            groups.entry(label).or_default().insert(net.id.clone());
        }
    }
    groups
        .into_iter()
        .filter_map(|(label, members)| {
            (members.len() >= 2).then(|| SketchNetBundle {
                label,
                members: members.into_iter().collect(),
            })
        })
        .collect()
}

pub(super) fn find_net_bundle(snapshot: &ProjectSnapshot, label: &str) -> Option<SketchNetBundle> {
    derive_net_bundles(snapshot)
        .into_iter()
        .find(|bundle| bundle.label == label)
}

pub(super) fn layout_net_bundle_badges(
    snapshot: &ProjectSnapshot,
    graph: &SketchGraph,
) -> Vec<SketchNetBundleBadge> {
    derive_net_bundles(snapshot)
        .into_iter()
        .filter_map(|bundle| layout_net_bundle_badge(graph, bundle))
        .collect()
}

pub(super) fn net_bundle_graph_bounds(
    graph: &SketchGraph,
    bundle: &SketchNetBundle,
) -> Option<egui::Rect> {
    let mut bounds: Option<egui::Rect> = None;
    for member in &bundle.members {
        if let Some(node) = graph
            .nodes
            .iter()
            .find(|node| node.selection == SketchSelection::Net(member.clone()))
        {
            bounds = Some(bounds.map_or(node.rect, |current| current.union(node.rect)));
        }
    }
    bounds.map(|bounds| bounds.expand(64.0))
}

pub(super) fn hit_test_net_bundle_badge(
    badges: &[SketchNetBundleBadge],
    position: egui::Pos2,
) -> Option<&SketchNetBundleBadge> {
    badges
        .iter()
        .find(|badge| badge.rect.expand(4.0).contains(position))
}

pub(super) fn draw_net_bundle_overlay(
    painter: &egui::Painter,
    badge: &SketchNetBundleBadge,
    hovered: bool,
    opacity: f32,
) {
    let opacity = opacity.clamp(0.0, 1.0);
    let color = if hovered {
        egui::Color32::from_rgb(255, 196, 87)
    } else {
        egui::Color32::from_rgb(91, 202, 197)
    };
    let color = with_opacity(color, opacity);
    let spine_stroke = egui::Stroke::new(if hovered { 4.0 } else { 3.0 }, color);
    painter.line_segment(
        [
            egui::pos2(badge.spine_x, badge.y_min),
            egui::pos2(badge.spine_x, badge.y_max),
        ],
        spine_stroke,
    );
    let tap_stroke = egui::Stroke::new(
        if hovered { 1.5 } else { 1.0 },
        egui::Color32::from_rgba_unmultiplied(
            color.r(),
            color.g(),
            color.b(),
            ((150.0 * opacity).round()) as u8,
        ),
    );
    for point in &badge.member_points {
        painter.line_segment([*point, egui::pos2(badge.spine_x, point.y)], tap_stroke);
    }
    painter.rect_filled(
        badge.rect,
        4.0,
        egui::Color32::from_rgba_unmultiplied(
            24,
            58,
            60,
            (((if hovered { 238.0 } else { 210.0 }) * opacity).round()) as u8,
        ),
    );
    painter.rect_stroke(
        badge.rect,
        4.0,
        egui::Stroke::new(1.0, color),
        egui::StrokeKind::Inside,
    );
    painter.text(
        badge.rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("{} [{}]", badge.bundle.label, badge.bundle.members.len()),
        egui::FontId::monospace(11.0),
        with_opacity(egui::Color32::from_rgb(215, 252, 249), opacity),
    );
}

pub(super) fn net_bundle_tooltip(ui: &mut egui::Ui, badge: &SketchNetBundleBadge) {
    ui.strong(format!("Net bundle {}", badge.bundle.label));
    ui.label(format!("{} scalar nets", badge.bundle.members.len()));
    ui.separator();
    for member in badge.bundle.members.iter().take(12) {
        ui.monospace(member);
    }
    if badge.bundle.members.len() > 12 {
        ui.label(format!("{} more", badge.bundle.members.len() - 12));
    }
    ui.separator();
    ui.label("Derived visual grouping only; Board IR keeps scalar nets.");
    ui.label("Click to multi-select member nets. Use the Object Navigator to fit bundles.");
}

fn layout_net_bundle_badge(
    graph: &SketchGraph,
    bundle: SketchNetBundle,
) -> Option<SketchNetBundleBadge> {
    let mut member_points = bundle
        .members
        .iter()
        .filter_map(|member| {
            graph
                .nodes
                .iter()
                .find(|node| node.selection == SketchSelection::Net(member.clone()))
                .map(|node| node.rect.center())
        })
        .collect::<Vec<_>>();
    if member_points.len() < 2 {
        return None;
    }
    member_points.sort_by(|left, right| {
        left.y
            .total_cmp(&right.y)
            .then_with(|| left.x.total_cmp(&right.x))
    });
    let y_min = member_points
        .iter()
        .map(|point| point.y)
        .fold(f32::INFINITY, f32::min);
    let y_max = member_points
        .iter()
        .map(|point| point.y)
        .fold(f32::NEG_INFINITY, f32::max);
    let spine_x = member_points
        .iter()
        .map(|point| point.x)
        .fold(f32::NEG_INFINITY, f32::max)
        + 44.0;
    let label_width = (bundle.label.len() as f32 * 7.0 + 42.0).clamp(76.0, 180.0);
    let label_center = egui::pos2(spine_x, y_min - 22.0);
    let rect = egui::Rect::from_center_size(label_center, egui::vec2(label_width, 22.0));
    Some(SketchNetBundleBadge {
        bundle,
        rect,
        spine_x,
        y_min,
        y_max,
        member_points,
    })
}

fn net_bundle_label(net_id: &str) -> Option<String> {
    let trimmed = net_id.trim();
    if trimmed.is_empty() {
        return None;
    }
    bracket_bus_label(trimmed)
        .or_else(|| protocol_pair_label(trimmed))
        .or_else(|| dot_namespace_label(trimmed))
}

fn bracket_bus_label(net_id: &str) -> Option<String> {
    let open = net_id.rfind('[')?;
    if !net_id.ends_with(']') {
        return None;
    }
    let index = &net_id[open + 1..net_id.len() - 1];
    if index.is_empty() || !index.chars().all(|character| character.is_ascii_digit()) {
        return None;
    }
    let prefix = net_id[..open].trim();
    (!prefix.is_empty()).then(|| format!("{prefix}[]"))
}

fn dot_namespace_label(net_id: &str) -> Option<String> {
    let (prefix, member) = net_id.rsplit_once('.')?;
    (!prefix.trim().is_empty() && !member.trim().is_empty()).then(|| prefix.to_string())
}

fn protocol_pair_label(net_id: &str) -> Option<String> {
    let lower = net_id.to_ascii_lowercase();
    paired_suffix_label(net_id, &lower, &[("_canh", "CAN"), ("_canl", "CAN")])
        .or_else(|| paired_suffix_label(net_id, &lower, &[("_scl", "I2C"), ("_sda", "I2C")]))
        .or_else(|| {
            paired_suffix_label(
                net_id,
                &lower,
                &[
                    ("_dp", "USB"),
                    ("_dm", "USB"),
                    ("_dplus", "USB"),
                    ("_dminus", "USB"),
                ],
            )
        })
        .or_else(|| rs485_pair_label(net_id, &lower))
        .or_else(|| match lower.as_str() {
            "canh" | "canl" => Some("CAN".to_string()),
            "scl" | "sda" => Some("I2C".to_string()),
            "d+" | "dp" | "d-" | "dm" => Some("USB".to_string()),
            _ => None,
        })
}

fn paired_suffix_label(net_id: &str, lower: &str, suffixes: &[(&str, &str)]) -> Option<String> {
    for (suffix, family) in suffixes {
        if lower.ends_with(suffix) {
            let prefix = trim_group_separator(&net_id[..net_id.len() - suffix.len()]);
            return Some(if prefix.is_empty() {
                (*family).to_string()
            } else {
                prefix.to_string()
            });
        }
    }
    None
}

fn rs485_pair_label(net_id: &str, lower: &str) -> Option<String> {
    for suffix in ["_a", "_b", "-a", "-b"] {
        if lower.ends_with(suffix) {
            let prefix = trim_group_separator(&net_id[..net_id.len() - suffix.len()]);
            let prefix_lower = prefix.to_ascii_lowercase();
            if prefix_lower.contains("rs485") || prefix_lower.contains("485") {
                return Some(prefix.to_string());
            }
        }
    }
    None
}

fn trim_group_separator(value: &str) -> &str {
    value.trim_matches(|character| matches!(character, '_' | '-' | '.' | ':' | '/'))
}

#[cfg(test)]
mod tests {
    use super::{derive_net_bundles, net_bundle_label};
    use crate::gui::sketch::{ProjectSnapshot, SketchNet};

    fn snapshot_with_nets(nets: &[&str]) -> ProjectSnapshot {
        ProjectSnapshot {
            name: "bundles".to_string(),
            components: 0,
            nets: nets.len(),
            scenarios: 0,
            libraries: Vec::new(),
            components_detail: Vec::new(),
            nets_detail: nets
                .iter()
                .map(|net| SketchNet {
                    id: (*net).to_string(),
                    kind: "digital_or_analog".to_string(),
                    nominal_voltage: None,
                    powered: None,
                    connections: Vec::new(),
                    position: None,
                })
                .collect(),
            probes: Vec::new(),
            wire_routes: Default::default(),
            net_labels: Default::default(),
            component_labels: Default::default(),
        }
    }

    #[test]
    fn derives_dot_and_bracket_bus_bundles() {
        let bundles = derive_net_bundles(&snapshot_with_nets(&[
            "PORT.RESET_RC",
            "PORT.SPARE",
            "DATA[0]",
            "DATA[1]",
            "lonely[0]",
        ]));
        assert!(bundles.iter().any(|bundle| {
            bundle.label == "PORT"
                && bundle.members == vec!["PORT.RESET_RC".to_string(), "PORT.SPARE".to_string()]
        }));
        assert!(bundles.iter().any(|bundle| {
            bundle.label == "DATA[]"
                && bundle.members == vec!["DATA[0]".to_string(), "DATA[1]".to_string()]
        }));
        assert!(!bundles.iter().any(|bundle| bundle.label == "lonely[]"));
    }

    #[test]
    fn derives_common_interface_pair_bundles() {
        let bundles = derive_net_bundles(&snapshot_with_nets(&[
            "robot_canh",
            "robot_canl",
            "I2C2_SCL",
            "I2C2_SDA",
            "usb_dp",
            "usb_dm",
            "rs485_servo_a",
            "rs485_servo_b",
        ]));
        for label in ["robot", "I2C2", "usb", "rs485_servo"] {
            assert!(
                bundles.iter().any(|bundle| bundle.label == label),
                "missing bundle {label}: {bundles:?}"
            );
        }
    }

    #[test]
    fn skips_unstructured_scalar_names() {
        assert_eq!(net_bundle_label("GPIO1"), None);
        assert_eq!(net_bundle_label("STATUS_LED"), None);
        assert_eq!(net_bundle_label("MOTOR_A"), None);
    }
}
