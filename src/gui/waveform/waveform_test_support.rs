use crate::gui::sketch::{ProjectSnapshot, SketchComponent, SketchNet, SketchNodeStyle, SketchPin};

pub(super) fn probe_snapshot() -> ProjectSnapshot {
    ProjectSnapshot {
        name: "probe_graph".to_string(),
        components: 1,
        nets: 2,
        scenarios: 1,
        libraries: Vec::new(),
        components_detail: vec![SketchComponent {
            id: "R1".to_string(),
            model: "generic.analog.resistor".to_string(),
            part_number: None,
            spice: None,
            position: None,
            pins: vec![
                SketchPin {
                    pin: "A".to_string(),
                    net: "out".to_string(),
                },
                SketchPin {
                    pin: "B".to_string(),
                    net: "gnd".to_string(),
                },
            ],
            style: SketchNodeStyle::default(),
            source_paths: Vec::new(),
        }],
        nets_detail: vec![
            SketchNet {
                id: "out".to_string(),
                kind: "digital_or_analog".to_string(),
                nominal_voltage: None,
                powered: None,
                connections: vec!["R1.A".to_string()],
                position: None,
            },
            SketchNet {
                id: "gnd".to_string(),
                kind: "ground".to_string(),
                nominal_voltage: Some(0.0),
                powered: Some(true),
                connections: vec!["R1.B".to_string()],
                position: None,
            },
        ],
        probes: Vec::new(),
        wire_routes: Default::default(),
        net_labels: Default::default(),
        component_labels: Default::default(),
    }
}
