use eframe::egui;

use super::CircuitCiApp;
use super::sketch::{self, ProjectSnapshot, SketchSelection};
use super::sketch_bundles::{derive_net_bundles, find_net_bundle, net_bundle_graph_bounds};
use super::sketch_probes::SketchProbeTarget;

const MAX_NAVIGATOR_ROWS: usize = 96;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SketchNavigatorTarget {
    Component(String),
    Net(String),
    Bundle(String),
    Wire { net_id: String, source: String },
    Probe { scenario: String, probe: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SketchNavigatorRow {
    kind: &'static str,
    label: String,
    detail: String,
    target: SketchNavigatorTarget,
}

impl SketchNavigatorTarget {
    fn selection(&self, snapshot: &ProjectSnapshot) -> Option<SketchSelection> {
        match self {
            Self::Component(id) => Some(SketchSelection::Component(id.clone())),
            Self::Bundle(label) => find_net_bundle(snapshot, label)
                .and_then(|bundle| bundle.members.first().cloned())
                .map(SketchSelection::Net),
            Self::Net(id) | Self::Wire { net_id: id, .. } => Some(SketchSelection::Net(id.clone())),
            Self::Probe { scenario, probe } => snapshot
                .probes
                .iter()
                .find(|candidate| {
                    candidate.scenario_name == *scenario && candidate.probe_name == *probe
                })
                .map(|candidate| match &candidate.target {
                    SketchProbeTarget::Component(id) => SketchSelection::Component(id.clone()),
                    SketchProbeTarget::Net(id) => SketchSelection::Net(id.clone()),
                }),
        }
    }
}

impl CircuitCiApp {
    pub(super) fn sketch_navigator_panel(&mut self, ui: &mut egui::Ui, snapshot: &ProjectSnapshot) {
        ui.collapsing("Object Navigator", |ui| {
            ui.horizontal(|ui| {
                ui.label("Search");
                ui.text_edit_singleline(&mut self.sketch_navigator_query);
                if ui.button("Clear").clicked() {
                    self.sketch_navigator_query.clear();
                }
            });
            let rows = sketch_navigator_rows(snapshot, &self.sketch_navigator_query);
            ui.label(format!(
                "{} of {} matching item(s)",
                rows.len().min(MAX_NAVIGATOR_ROWS),
                rows.len()
            ));
            egui::ScrollArea::vertical()
                .max_height(170.0)
                .show(ui, |ui| {
                    egui::Grid::new("sketch_object_navigator_rows")
                        .num_columns(5)
                        .striped(true)
                        .show(ui, |ui| {
                            for row in rows.iter().take(MAX_NAVIGATOR_ROWS) {
                                ui.monospace(row.kind);
                                ui.label(&row.label);
                                ui.label(&row.detail);
                                if ui.button("Select").clicked() {
                                    self.select_navigator_target(snapshot, &row.target);
                                }
                                if ui.button("Fit").clicked() {
                                    self.select_navigator_target(snapshot, &row.target);
                                    self.sketch_navigator_fit_target = Some(row.target.clone());
                                }
                                ui.end_row();
                            }
                        });
                });
        });
    }

    fn select_navigator_target(
        &mut self,
        snapshot: &ProjectSnapshot,
        target: &SketchNavigatorTarget,
    ) {
        if let SketchNavigatorTarget::Bundle(label) = target
            && let Some(bundle) = find_net_bundle(snapshot, label)
        {
            self.select_net_bundle(&bundle);
            return;
        }
        if let Some(selection) = target.selection(snapshot) {
            self.set_single_sketch_selection(Some(selection.clone()));
            self.status = match selection {
                SketchSelection::Component(id) => format!("Selected component {id}."),
                SketchSelection::Net(id) => format!("Selected net {id}."),
                SketchSelection::Overflow(label) => format!("Selected {label}."),
            };
        }
        if let SketchNavigatorTarget::Probe { scenario, probe } = target {
            self.analog_probe_scenario = scenario.clone();
            self.analog_assertion_scenario = scenario.clone();
            self.analog_assertion_probe = probe.clone();
            self.status = format!("Selected probe {probe} from scenario {scenario}.");
        }
    }

    pub(super) fn fit_sketch_navigator_target(
        &mut self,
        canvas: egui::Rect,
        snapshot: &ProjectSnapshot,
        target: &SketchNavigatorTarget,
    ) {
        let graph = sketch::layout_sketch_graph(canvas, snapshot);
        let Some(bounds) = navigator_target_bounds(&graph, snapshot, target) else {
            self.status =
                "Navigator target is not visible in the current sketch layout.".to_string();
            return;
        };
        fit_viewport_to_bounds(self, canvas, bounds);
    }
}

pub(super) fn sketch_navigator_rows(
    snapshot: &ProjectSnapshot,
    query: &str,
) -> Vec<SketchNavigatorRow> {
    let mut rows = Vec::new();
    for component in &snapshot.components_detail {
        rows.push(SketchNavigatorRow {
            kind: "component",
            label: component.id.clone(),
            detail: format!(
                "{} / {} pins{}",
                component.model,
                component.pins.len(),
                component
                    .part_number
                    .as_ref()
                    .map(|part| format!(" / {part}"))
                    .unwrap_or_default()
            ),
            target: SketchNavigatorTarget::Component(component.id.clone()),
        });
        for pin in &component.pins {
            rows.push(SketchNavigatorRow {
                kind: "wire",
                label: format!("{}.{}", component.id, pin.pin),
                detail: format!("net {}", pin.net),
                target: SketchNavigatorTarget::Wire {
                    net_id: pin.net.clone(),
                    source: format!("{}.{}", component.id, pin.pin),
                },
            });
        }
    }
    for bundle in derive_net_bundles(snapshot) {
        rows.push(SketchNavigatorRow {
            kind: "bundle",
            label: bundle.label.clone(),
            detail: format!(
                "{} nets: {}",
                bundle.members.len(),
                compact_members(&bundle.members)
            ),
            target: SketchNavigatorTarget::Bundle(bundle.label),
        });
    }
    for net in &snapshot.nets_detail {
        rows.push(SketchNavigatorRow {
            kind: "net",
            label: net.id.clone(),
            detail: format!("{} / {} conn", net.kind, net.connections.len()),
            target: SketchNavigatorTarget::Net(net.id.clone()),
        });
    }
    for probe in &snapshot.probes {
        rows.push(SketchNavigatorRow {
            kind: "probe",
            label: format!("{}:{}", probe.scenario_name, probe.probe_name),
            detail: format!("{} {}", probe.quantity.label(), probe.expression),
            target: SketchNavigatorTarget::Probe {
                scenario: probe.scenario_name.clone(),
                probe: probe.probe_name.clone(),
            },
        });
    }
    filter_navigator_rows(rows, query)
}

fn filter_navigator_rows(rows: Vec<SketchNavigatorRow>, query: &str) -> Vec<SketchNavigatorRow> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return rows;
    }
    rows.into_iter()
        .filter(|row| {
            row.kind.contains(&query)
                || row.label.to_lowercase().contains(&query)
                || row.detail.to_lowercase().contains(&query)
        })
        .collect()
}

fn navigator_target_bounds(
    graph: &sketch::SketchGraph,
    snapshot: &ProjectSnapshot,
    target: &SketchNavigatorTarget,
) -> Option<egui::Rect> {
    match target {
        SketchNavigatorTarget::Component(id) => graph
            .nodes
            .iter()
            .find(|node| node.selection == SketchSelection::Component(id.clone()))
            .map(|node| node.rect.expand(36.0)),
        SketchNavigatorTarget::Net(id) => graph
            .nodes
            .iter()
            .find(|node| node.selection == SketchSelection::Net(id.clone()))
            .map(|node| node.rect.expand(36.0)),
        SketchNavigatorTarget::Bundle(label) => {
            let bundle = find_net_bundle(snapshot, label)?;
            net_bundle_graph_bounds(graph, &bundle)
        }
        SketchNavigatorTarget::Wire { net_id, source } => graph
            .edges
            .iter()
            .find(|edge| edge.net_id == *net_id && edge.source == *source)
            .map(|edge| {
                egui::Rect::from_two_pos(edge.start, edge.end)
                    .expand(48.0)
                    .union(egui::Rect::from_center_size(
                        edge.label_position(),
                        egui::vec2(80.0, 24.0),
                    ))
            }),
        SketchNavigatorTarget::Probe { scenario, probe } => graph
            .probe_badges
            .iter()
            .find(|badge| {
                badge.probe.scenario_name == *scenario && badge.probe.probe_name == *probe
            })
            .map(|badge| badge.rect.expand(64.0)),
    }
}

fn compact_members(members: &[String]) -> String {
    let mut values = members.iter().take(4).cloned().collect::<Vec<_>>();
    if members.len() > values.len() {
        values.push(format!("{} more", members.len() - values.len()));
    }
    values.join(", ")
}

fn fit_viewport_to_bounds(app: &mut CircuitCiApp, canvas: egui::Rect, bounds: egui::Rect) {
    let padding = 80.0;
    let available =
        (canvas.size() - egui::vec2(padding * 2.0, padding * 2.0)).max(egui::Vec2::splat(1.0));
    let content = bounds.size().max(egui::Vec2::splat(1.0));
    let zoom = (available.x / content.x)
        .min(available.y / content.y)
        .clamp(0.25, 4.0);
    let fitted_size = content * zoom;
    let target_min = canvas.min + egui::vec2(padding, padding) + (available - fitted_size) / 2.0;
    app.sketch_zoom = zoom;
    app.sketch_pan = target_min - canvas.min - (bounds.min - canvas.min) * zoom;
}

trait SketchEdgeLabelPosition {
    fn label_position(&self) -> egui::Pos2;
}

impl SketchEdgeLabelPosition for sketch::SketchEdge {
    fn label_position(&self) -> egui::Pos2 {
        super::sketch::edge_label_position(self)
    }
}

#[cfg(test)]
mod tests {
    use super::{SketchNavigatorTarget, sketch_navigator_rows};
    use crate::gui::sketch::{
        ProjectSnapshot, SketchComponent, SketchNet, SketchNodeStyle, SketchPin, SketchPosition,
    };
    use crate::gui::sketch_probes::{SketchProbe, SketchProbeQuantity, SketchProbeTarget};

    fn snapshot() -> ProjectSnapshot {
        ProjectSnapshot {
            name: "navigator".to_string(),
            components: 1,
            nets: 3,
            scenarios: 1,
            libraries: Vec::new(),
            components_detail: vec![SketchComponent {
                id: "R1".to_string(),
                model: "generic.analog.resistor".to_string(),
                part_number: Some("RC0603".to_string()),
                pins: vec![
                    SketchPin {
                        pin: "A".to_string(),
                        net: "rail".to_string(),
                    },
                    SketchPin {
                        pin: "B".to_string(),
                        net: "out".to_string(),
                    },
                ],
                position: Some(SketchPosition { x: 10.0, y: 20.0 }),
                style: SketchNodeStyle::default(),
            }],
            nets_detail: vec![
                SketchNet {
                    id: "rail".to_string(),
                    kind: "power".to_string(),
                    nominal_voltage: Some(5.0),
                    powered: Some(true),
                    connections: vec!["R1.A".to_string()],
                    position: None,
                },
                SketchNet {
                    id: "robot_canh".to_string(),
                    kind: "digital_or_analog".to_string(),
                    nominal_voltage: None,
                    powered: None,
                    connections: Vec::new(),
                    position: None,
                },
                SketchNet {
                    id: "robot_canl".to_string(),
                    kind: "digital_or_analog".to_string(),
                    nominal_voltage: None,
                    powered: None,
                    connections: Vec::new(),
                    position: None,
                },
            ],
            probes: vec![SketchProbe {
                scenario_name: "tran".to_string(),
                probe_name: "rail_v".to_string(),
                expression: "V(rail)".to_string(),
                quantity: SketchProbeQuantity::Voltage,
                target: SketchProbeTarget::Net("rail".to_string()),
                assertion_names: Vec::new(),
            }],
        }
    }

    #[test]
    fn sketch_navigator_lists_components_wires_nets_and_probes() {
        let rows = sketch_navigator_rows(&snapshot(), "");
        assert!(rows.iter().any(|row| {
            row.kind == "component"
                && row.label == "R1"
                && row.target == SketchNavigatorTarget::Component("R1".to_string())
        }));
        assert!(rows.iter().any(|row| {
            row.kind == "wire"
                && row.label == "R1.A"
                && row.target
                    == SketchNavigatorTarget::Wire {
                        net_id: "rail".to_string(),
                        source: "R1.A".to_string(),
                    }
        }));
        assert!(rows.iter().any(|row| {
            row.kind == "net"
                && row.label == "rail"
                && row.target == SketchNavigatorTarget::Net("rail".to_string())
        }));
        assert!(rows.iter().any(|row| {
            row.kind == "bundle"
                && row.label == "robot"
                && row.target == SketchNavigatorTarget::Bundle("robot".to_string())
        }));
        assert!(rows.iter().any(|row| {
            row.kind == "probe"
                && row.label == "tran:rail_v"
                && row.target
                    == SketchNavigatorTarget::Probe {
                        scenario: "tran".to_string(),
                        probe: "rail_v".to_string(),
                    }
        }));
    }

    #[test]
    fn sketch_navigator_filters_by_kind_label_and_detail() {
        let by_kind = sketch_navigator_rows(&snapshot(), "probe");
        assert_eq!(by_kind.len(), 1);
        assert_eq!(by_kind[0].label, "tran:rail_v");

        let by_label = sketch_navigator_rows(&snapshot(), "R1.A");
        assert_eq!(by_label.len(), 1);
        assert_eq!(by_label[0].kind, "wire");

        let by_detail = sketch_navigator_rows(&snapshot(), "RC0603");
        assert_eq!(by_detail.len(), 1);
        assert_eq!(by_detail[0].kind, "component");

        let by_bundle = sketch_navigator_rows(&snapshot(), "robot");
        assert!(by_bundle.iter().any(|row| row.kind == "bundle"));
    }
}
