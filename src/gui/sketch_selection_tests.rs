use super::CircuitCiApp;
use super::sketch::{
    SketchSelection, SketchViewport, layout_sketch_graph_viewport, load_project_snapshot_from_yaml,
};
use super::sketch_canvas_interaction::SketchSelectionBoxMode;
use eframe::egui;

fn selection_test_graph() -> (
    String,
    super::sketch::ProjectSnapshot,
    super::sketch::SketchGraph,
) {
    let yaml = "project:
  name: gui_selection_test
  version: 0.1.0
board:
  components:
    U1:
      model: generic.ic
    U2:
      model: generic.ic
  nets:
    sig:
      kind: digital_or_analog
  schematic:
    node_positions:
      component:U1:
        x: 20.0
        y: 30.0
      component:U2:
        x: 260.0
        y: 30.0
      net:sig:
        x: 180.0
        y: 170.0
"
    .to_string();
    let snapshot = load_project_snapshot_from_yaml(&yaml).unwrap();
    let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(640.0, 320.0));
    let graph = layout_sketch_graph_viewport(
        canvas,
        &snapshot,
        SketchViewport {
            pan: egui::Vec2::ZERO,
            zoom: 1.0,
        },
    );
    (yaml, snapshot, graph)
}

#[test]
fn selection_box_modes_replace_add_and_subtract() {
    let (yaml, snapshot, graph) = selection_test_graph();
    let mut app = CircuitCiApp {
        project_yaml: yaml,
        project_snapshot: Some(snapshot),
        ..CircuitCiApp::default()
    };

    app.apply_marquee_selection(
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(210.0, 140.0)),
        &graph,
        SketchSelectionBoxMode::Replace,
    );
    assert_eq!(app.selected_sketch_items.len(), 1);
    assert!(
        app.selected_sketch_items
            .contains(&SketchSelection::Component("U1".to_string()))
    );

    app.apply_marquee_selection(
        egui::Rect::from_min_max(egui::pos2(230.0, 0.0), egui::pos2(560.0, 140.0)),
        &graph,
        SketchSelectionBoxMode::Add,
    );
    assert_eq!(app.selected_sketch_items.len(), 2);
    assert!(
        app.selected_sketch_items
            .contains(&SketchSelection::Component("U2".to_string()))
    );

    app.apply_marquee_selection(
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(210.0, 140.0)),
        &graph,
        SketchSelectionBoxMode::Subtract,
    );
    assert_eq!(
        app.selected_sketch_items,
        [SketchSelection::Component("U2".to_string())]
            .into_iter()
            .collect()
    );
}

#[test]
fn selection_lasso_modes_replace_add_and_subtract() {
    let (yaml, snapshot, graph) = selection_test_graph();
    let mut app = CircuitCiApp {
        project_yaml: yaml,
        project_snapshot: Some(snapshot),
        ..CircuitCiApp::default()
    };

    app.apply_lasso_selection(
        &[
            egui::pos2(0.0, 0.0),
            egui::pos2(210.0, 0.0),
            egui::pos2(210.0, 140.0),
            egui::pos2(0.0, 140.0),
        ],
        &graph,
        SketchSelectionBoxMode::Replace,
    );
    assert_eq!(
        app.selected_sketch_items,
        [SketchSelection::Component("U1".to_string())]
            .into_iter()
            .collect()
    );

    app.apply_lasso_selection(
        &[
            egui::pos2(230.0, 0.0),
            egui::pos2(560.0, 0.0),
            egui::pos2(560.0, 140.0),
            egui::pos2(230.0, 140.0),
        ],
        &graph,
        SketchSelectionBoxMode::Add,
    );
    assert_eq!(app.selected_sketch_items.len(), 2);
    assert!(
        app.selected_sketch_items
            .contains(&SketchSelection::Component("U2".to_string()))
    );

    app.apply_lasso_selection(
        &[
            egui::pos2(0.0, 0.0),
            egui::pos2(210.0, 0.0),
            egui::pos2(210.0, 140.0),
            egui::pos2(0.0, 140.0),
        ],
        &graph,
        SketchSelectionBoxMode::Subtract,
    );
    assert_eq!(
        app.selected_sketch_items,
        [SketchSelection::Component("U2".to_string())]
            .into_iter()
            .collect()
    );
}
