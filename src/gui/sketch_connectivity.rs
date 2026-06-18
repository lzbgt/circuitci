use std::collections::BTreeSet;

use super::sketch::{SketchPinAnchor, SketchSelection};

#[derive(Debug, Clone, Default)]
pub(super) struct SketchConnectivityHighlight {
    selected_nets: BTreeSet<String>,
}

impl SketchConnectivityHighlight {
    pub(super) fn from_selection<'a>(
        selected: impl IntoIterator<Item = &'a SketchSelection>,
    ) -> Self {
        Self {
            selected_nets: selected
                .into_iter()
                .filter_map(|selection| match selection {
                    SketchSelection::Net(net_id) => Some(net_id.clone()),
                    SketchSelection::Component(_) | SketchSelection::Overflow(_) => None,
                })
                .collect(),
        }
    }

    pub(super) fn net_selected(&self, net_id: &str) -> bool {
        self.selected_nets.contains(net_id)
    }

    pub(super) fn anchor_connected(&self, anchor: &SketchPinAnchor) -> bool {
        self.net_selected(&anchor.net)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui;

    #[test]
    fn connectivity_highlight_tracks_selected_nets_only() {
        let selections = [
            SketchSelection::Net("sig".to_string()),
            SketchSelection::Component("R1".to_string()),
        ];
        let highlight = SketchConnectivityHighlight::from_selection(&selections);

        assert!(highlight.net_selected("sig"));
        assert!(!highlight.net_selected("other"));
        assert!(highlight.anchor_connected(&SketchPinAnchor {
            component_id: "R1".to_string(),
            pin: "A".to_string(),
            net: "sig".to_string(),
            kind: "digital_or_analog".to_string(),
            pos: egui::pos2(0.0, 0.0),
            label_pos: egui::pos2(0.0, 0.0),
            label_align: egui::Align2::LEFT_CENTER,
        }));
    }
}
