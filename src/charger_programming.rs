use crate::board_ir::{BoardProject, ComponentSpec, SpicePrimitive};
use crate::library::BatteryCharger;

#[derive(Debug, Clone)]
pub struct DerivedChargeCurrent {
    pub current_a: f64,
    pub resistor_component: String,
    pub resistor_ohm: f64,
    pub source: Option<String>,
}

pub fn derive_charge_current_from_programming_resistor(
    project: &BoardProject,
    charger_component: &ComponentSpec,
    charger: &BatteryCharger,
) -> Option<DerivedChargeCurrent> {
    let programming = charger.charge_current_programming.as_ref()?;
    if !programming.current_gain_v.is_finite() || programming.current_gain_v <= 0.0 {
        return None;
    }
    let programming_net = charger_component.pins.get(&programming.programming_pin)?;
    let reference_net = charger_component.pins.get(&programming.reference_pin)?;
    if programming_net == reference_net {
        return None;
    }

    let mut matched = None;
    for (component_id, component) in &project.board.components {
        let Some(spice) = component.spice.as_ref() else {
            continue;
        };
        if spice.primitive != SpicePrimitive::Resistor {
            continue;
        }
        let Some(resistor_ohm) = spice.value_ohm else {
            continue;
        };
        if !resistor_ohm.is_finite() || resistor_ohm <= 0.0 {
            continue;
        }
        if !component_connects_nets(component, programming_net, reference_net) {
            continue;
        }
        if matched.is_some() {
            return None;
        }
        let current_a = programming.current_gain_v / resistor_ohm;
        if !current_a.is_finite() || current_a < 0.0 {
            return None;
        }
        matched = Some(DerivedChargeCurrent {
            current_a,
            resistor_component: component_id.clone(),
            resistor_ohm,
            source: programming.source.clone(),
        });
    }
    matched
}

fn component_connects_nets(component: &ComponentSpec, net_a: &str, net_b: &str) -> bool {
    let mut has_a = false;
    let mut has_b = false;
    for net in component.pins.values() {
        if net == net_a {
            has_a = true;
        } else if net == net_b {
            has_b = true;
        }
    }
    has_a && has_b
}
