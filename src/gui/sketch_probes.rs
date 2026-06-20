use eframe::egui;

use super::kicad_symbol_library::{KiCadProbeSymbolKind, draw_kicad_probe_symbol};
use super::sketch::{
    ProjectSnapshot, SketchNode, SketchSelection, encode_edited_project_yaml,
    ensure_child_mapping_mut, validated_graph_id, with_opacity,
};
use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub(super) struct SketchProbe {
    pub(super) element_id: Option<String>,
    pub(super) scenario_name: String,
    pub(super) probe_name: String,
    pub(super) expression: String,
    pub(super) quantity: SketchProbeQuantity,
    pub(super) target: SketchProbeTarget,
    pub(super) assertion_names: Vec<String>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SketchProbeAttachmentKind {
    Node,
    Pin,
    Wire,
}

#[derive(Debug, Clone)]
pub(super) struct SketchProbeElementDraft {
    pub(super) element_id: String,
    pub(super) scenario_name: String,
    pub(super) probe_name: String,
    pub(super) target: SketchProbeTarget,
    pub(super) attachment: SketchProbeAttachmentKind,
    pub(super) source: Option<String>,
}

#[derive(Debug)]
pub(super) struct SketchProbeBadge {
    pub(super) probe: SketchProbe,
    pub(super) rect: egui::Rect,
    pub(super) anchor: egui::Pos2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SketchProbeStatus {
    Unasserted,
    Unknown,
    Pass,
    Fail,
}

impl SketchProbeStatus {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Unasserted => "no assertions",
            Self::Unknown => "assertions not evaluated",
            Self::Pass => "assertions passed",
            Self::Fail => "assertion failed",
        }
    }
}

pub(super) fn derive_project_probes(project: &crate::board_ir::BoardProject) -> Vec<SketchProbe> {
    let branch_targets = component_branch_targets(project);
    let schematic_probe_elements = project
        .board
        .schematic
        .probe_elements
        .iter()
        .filter_map(|(element_id, element)| {
            let target = schematic_probe_element_target(project, element)?;
            Some((
                (element.scenario.as_str(), element.probe.as_str()),
                (element_id, target),
            ))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
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
            let element_id = schematic_probe_elements
                .get(&(scenario.name.as_str(), probe.name.as_str()))
                .and_then(|(element_id, element_target)| {
                    (element_target == &target).then(|| (*element_id).clone())
                });
            probes.push(SketchProbe {
                element_id,
                scenario_name: scenario.name.clone(),
                probe_name: probe.name.clone(),
                expression: probe.expression.clone(),
                quantity,
                target,
                assertion_names: analog
                    .assertions
                    .iter()
                    .filter(|assertion| assertion.probe == probe.name)
                    .map(|assertion| assertion.name.clone())
                    .collect(),
            });
        }
    }
    probes
}

pub(super) fn upsert_schematic_probe_element(
    text: &str,
    draft: &SketchProbeElementDraft,
) -> Result<String> {
    let element_id = validated_graph_id(&draft.element_id, "probe element")?;
    let scenario_name = validated_graph_id(&draft.scenario_name, "scenario")?;
    let probe_name = validated_graph_id(&draft.probe_name, "probe")?;
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let scenario = project
        .scenarios
        .iter()
        .find(|scenario| scenario.name == scenario_name)
        .with_context(|| format!("Scenario {scenario_name} was not found."))?;
    let analog = scenario
        .analog
        .as_ref()
        .with_context(|| format!("Scenario {scenario_name} is not an analog scenario."))?;
    if !analog.probes.iter().any(|probe| probe.name == probe_name) {
        anyhow::bail!("Analog probe {probe_name} was not found in scenario {scenario_name}.");
    }
    match &draft.target {
        SketchProbeTarget::Net(net_id) => {
            if !project.board.nets.contains_key(net_id) {
                anyhow::bail!("Probe target net {net_id} was not found.");
            }
        }
        SketchProbeTarget::Component(component_id) => {
            if !project.board.components.contains_key(component_id) {
                anyhow::bail!("Probe target component {component_id} was not found.");
            }
        }
    }
    if let Some(source) = &draft.source {
        validate_probe_source(source)?;
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
        let schematic = ensure_child_mapping_mut(board, "schematic", "board schematic")?;
        let elements =
            ensure_child_mapping_mut(schematic, "probe_elements", "schematic probe elements")?;
        let mut target = serde_yaml_ng::Mapping::new();
        match &draft.target {
            SketchProbeTarget::Net(net_id) => {
                target.insert(key("kind"), serde_yaml_ng::Value::String("net".to_string()));
                target.insert(key("id"), serde_yaml_ng::Value::String(net_id.clone()));
            }
            SketchProbeTarget::Component(component_id) => {
                target.insert(
                    key("kind"),
                    serde_yaml_ng::Value::String("component".to_string()),
                );
                target.insert(
                    key("id"),
                    serde_yaml_ng::Value::String(component_id.clone()),
                );
            }
        }
        target.insert(
            key("attach"),
            serde_yaml_ng::Value::String(draft.attachment.as_str().to_string()),
        );
        if let Some(source) = &draft.source {
            target.insert(key("source"), serde_yaml_ng::Value::String(source.clone()));
        }
        let mut element = serde_yaml_ng::Mapping::new();
        element.insert(
            key("scenario"),
            serde_yaml_ng::Value::String(scenario_name.to_string()),
        );
        element.insert(
            key("probe"),
            serde_yaml_ng::Value::String(probe_name.to_string()),
        );
        element.insert(key("target"), serde_yaml_ng::Value::Mapping(target));
        elements.insert(
            serde_yaml_ng::Value::String(element_id.to_string()),
            serde_yaml_ng::Value::Mapping(element),
        );
    }
    encode_edited_project_yaml(yaml)
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
        let (rect, anchor) = probe_badge_geometry(node.rect, *offset_index, probe.quantity);
        *offset_index += 1;
        badges.push(SketchProbeBadge {
            probe: probe.clone(),
            rect,
            anchor,
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

pub(super) fn probe_assertion_status(
    report: Option<&crate::reports::ValidationReport>,
    probe: &SketchProbe,
) -> SketchProbeStatus {
    if probe.assertion_names.is_empty() {
        return SketchProbeStatus::Unasserted;
    }
    let Some(report) = report else {
        return SketchProbeStatus::Unknown;
    };
    let scenario_failures: Vec<_> = report
        .failures
        .iter()
        .filter(|finding| finding.scenario == probe.scenario_name)
        .collect();
    let assertion_failed = probe.assertion_names.iter().any(|assertion_name| {
        scenario_failures
            .iter()
            .any(|finding| finding_mentions_assertion(finding, assertion_name))
    });
    if assertion_failed {
        SketchProbeStatus::Fail
    } else if scenario_failures
        .iter()
        .any(|finding| !finding.message.contains("assertion "))
    {
        SketchProbeStatus::Unknown
    } else {
        SketchProbeStatus::Pass
    }
}

pub(super) fn draw_probe_badge(
    painter: &egui::Painter,
    badge: &SketchProbeBadge,
    hovered: bool,
    status: SketchProbeStatus,
    opacity: f32,
) {
    let opacity = opacity.clamp(0.0, 1.0);
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
    let status_color = match status {
        SketchProbeStatus::Unasserted => egui::Color32::from_gray(150),
        SketchProbeStatus::Unknown => egui::Color32::from_rgb(230, 190, 90),
        SketchProbeStatus::Pass => egui::Color32::from_rgb(86, 190, 112),
        SketchProbeStatus::Fail => egui::Color32::from_rgb(232, 83, 83),
    };
    let stroke = egui::Stroke::new(
        if hovered { 2.1 } else { 1.5 },
        with_opacity(stroke_color, opacity),
    );
    painter.line_segment(
        [
            badge.anchor,
            egui::pos2(badge.rect.left(), badge.rect.center().y),
        ],
        egui::Stroke::new(1.2, with_opacity(stroke_color, opacity)),
    );
    painter.rect_filled(
        badge.rect.expand(2.0),
        2.0,
        with_opacity(egui::Color32::from_rgb(13, 18, 24), opacity * 0.72),
    );
    if !draw_kicad_probe_symbol(
        painter,
        probe_symbol_kind(badge.probe.quantity),
        badge.rect,
        stroke,
        with_opacity(fill, opacity),
    ) {
        draw_fallback_probe_symbol(
            painter,
            badge.rect,
            badge.probe.quantity,
            stroke,
            fill,
            opacity,
        );
    }
    painter.rect_stroke(
        badge.rect.expand(2.0),
        2.0,
        egui::Stroke::new(0.7, with_opacity(stroke_color, opacity * 0.55)),
        egui::StrokeKind::Inside,
    );
    painter.circle_filled(
        egui::pos2(badge.rect.right() - 1.5, badge.rect.top() + 1.5),
        3.0,
        with_opacity(status_color, opacity),
    );
    if status == SketchProbeStatus::Fail {
        painter.text(
            egui::pos2(badge.rect.right() - 1.5, badge.rect.top() + 1.2),
            egui::Align2::CENTER_CENTER,
            "!",
            egui::FontId::monospace(7.0),
            with_opacity(egui::Color32::WHITE, opacity),
        );
    }
}

fn probe_symbol_kind(quantity: SketchProbeQuantity) -> KiCadProbeSymbolKind {
    match quantity {
        SketchProbeQuantity::Voltage => KiCadProbeSymbolKind::Voltage,
        SketchProbeQuantity::Current => KiCadProbeSymbolKind::Current,
        SketchProbeQuantity::Power => KiCadProbeSymbolKind::Power,
    }
}

fn draw_fallback_probe_symbol(
    painter: &egui::Painter,
    rect: egui::Rect,
    quantity: SketchProbeQuantity,
    stroke: egui::Stroke,
    fill: egui::Color32,
    opacity: f32,
) {
    let color = with_opacity(fill, opacity);
    painter.circle_stroke(
        rect.center(),
        rect.height().min(rect.width()) * 0.32,
        stroke,
    );
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        quantity.label(),
        egui::FontId::monospace(12.0),
        color,
    );
}

fn finding_mentions_assertion(finding: &crate::reports::Finding, assertion_name: &str) -> bool {
    finding
        .message
        .contains(&format!("assertion {assertion_name} "))
        || finding
            .message
            .contains(&format!("assertion {assertion_name}."))
        || finding
            .message
            .contains(&format!("assertion {assertion_name}:"))
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

fn schematic_probe_element_target(
    project: &crate::board_ir::BoardProject,
    element: &crate::board_ir::SchematicProbeElement,
) -> Option<SketchProbeTarget> {
    match element.target.kind {
        crate::board_ir::SchematicProbeElementTargetKind::Net => project
            .board
            .nets
            .contains_key(&element.target.id)
            .then(|| SketchProbeTarget::Net(element.target.id.clone())),
        crate::board_ir::SchematicProbeElementTargetKind::Component => project
            .board
            .components
            .contains_key(&element.target.id)
            .then(|| SketchProbeTarget::Component(element.target.id.clone())),
    }
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

impl SketchProbeAttachmentKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Node => "node",
            Self::Pin => "pin",
            Self::Wire => "wire",
        }
    }
}

fn validate_probe_source(source: &str) -> Result<()> {
    let (component_id, pin_id) = source
        .split_once('.')
        .context("Probe element source must use component.pin form.")?;
    validated_graph_id(component_id, "probe source component")?;
    validated_graph_id(pin_id, "probe source pin")?;
    Ok(())
}

fn key(name: &str) -> serde_yaml_ng::Value {
    serde_yaml_ng::Value::String(name.to_string())
}

fn expression_without_whitespace(expression: &str) -> String {
    expression
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect()
}

fn probe_badge_geometry(
    node_rect: egui::Rect,
    offset_index: usize,
    quantity: SketchProbeQuantity,
) -> (egui::Rect, egui::Pos2) {
    let size = match quantity {
        SketchProbeQuantity::Voltage | SketchProbeQuantity::Current => egui::vec2(46.0, 34.0),
        SketchProbeQuantity::Power => egui::vec2(52.0, 36.0),
    };
    let y = node_rect.top() + 2.0 + offset_index as f32 * (size.y + 6.0);
    let center_y = (y + size.y * 0.5).clamp(node_rect.top(), node_rect.bottom());
    let anchor = egui::pos2(node_rect.right(), center_y);
    let rect = egui::Rect::from_min_size(egui::pos2(node_rect.right() + 8.0, y), size);
    (rect, anchor)
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

#[cfg(test)]
mod tests {
    use super::{
        SketchProbe, SketchProbeQuantity, SketchProbeStatus, SketchProbeTarget,
        probe_assertion_status,
    };
    use crate::reports::{Finding, ValidationReport};

    fn asserted_probe() -> SketchProbe {
        SketchProbe {
            element_id: None,
            scenario_name: "tran".to_string(),
            probe_name: "rail_voltage".to_string(),
            expression: "V(rail)".to_string(),
            quantity: SketchProbeQuantity::Voltage,
            target: SketchProbeTarget::Net("rail".to_string()),
            assertion_names: vec!["rail_voltage_min".to_string()],
        }
    }

    fn report(findings: Vec<Finding>) -> ValidationReport {
        ValidationReport::from_parts(
            "project".to_string(),
            "profile".to_string(),
            findings,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            "validate".to_string(),
        )
    }

    #[test]
    fn probe_status_is_unasserted_without_assertions() {
        let mut probe = asserted_probe();
        probe.assertion_names.clear();
        assert_eq!(
            probe_assertion_status(Some(&report(Vec::new())), &probe),
            SketchProbeStatus::Unasserted
        );
    }

    #[test]
    fn probe_status_is_unknown_before_validation() {
        let probe = asserted_probe();
        assert_eq!(
            probe_assertion_status(None, &probe),
            SketchProbeStatus::Unknown
        );
    }

    #[test]
    fn probe_status_fails_when_report_names_assertion() {
        let probe = asserted_probe();
        let finding = Finding::critical(
            "SPICE_TRANSIENT_ANALYSIS",
            "tran",
            "Analog assertion rail_voltage_min failed: sampled probe rail_voltage measured 4.0 V.",
        );
        assert_eq!(
            probe_assertion_status(Some(&report(vec![finding])), &probe),
            SketchProbeStatus::Fail
        );
    }

    #[test]
    fn probe_status_passes_when_asserted_scenario_has_no_failures() {
        let probe = asserted_probe();
        assert_eq!(
            probe_assertion_status(Some(&report(Vec::new())), &probe),
            SketchProbeStatus::Pass
        );
    }

    #[test]
    fn probe_status_is_unknown_when_scenario_failed_before_assertions() {
        let probe = asserted_probe();
        let finding = Finding::critical(
            "SPICE_TRANSIENT_ANALYSIS",
            "tran",
            "ngspice backend failed before waveform assertions were evaluated.",
        );
        assert_eq!(
            probe_assertion_status(Some(&report(vec![finding])), &probe),
            SketchProbeStatus::Unknown
        );
    }

    #[test]
    fn probe_status_passes_when_only_other_assertions_failed() {
        let probe = asserted_probe();
        let finding = Finding::critical(
            "SPICE_TRANSIENT_ANALYSIS",
            "tran",
            "Analog assertion out_voltage_max failed: sampled probe out_voltage measured 6.0 V.",
        );
        assert_eq!(
            probe_assertion_status(Some(&report(vec![finding])), &probe),
            SketchProbeStatus::Pass
        );
    }
}
