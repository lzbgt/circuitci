use crate::board_ir::{BoardProject, ComponentSpec};
use crate::library::PowerMux;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DerivedPowerMuxSelection {
    pub input_name: String,
    pub input_pin: String,
    pub input_net: String,
    pub output_net: String,
}

pub fn derive_selected_power_mux_input_from_powered_nets(
    project: &BoardProject,
    component: &ComponentSpec,
    mux: &PowerMux,
) -> Option<DerivedPowerMuxSelection> {
    let output_net_name = resolve_power_pin_net(component, &mux.output_pin)?;
    let output_net = project.board.nets.get(output_net_name)?;
    if output_net.powered != Some(true) {
        return None;
    }

    let mut powered_inputs = Vec::new();
    for input in &mux.inputs {
        let input_net_name = resolve_power_pin_net(component, &input.input_pin)?;
        let input_net = project.board.nets.get(input_net_name)?;
        match input_net.powered {
            Some(true) => powered_inputs.push(DerivedPowerMuxSelection {
                input_name: input.name.clone(),
                input_pin: input.input_pin.clone(),
                input_net: input_net_name.to_string(),
                output_net: output_net_name.to_string(),
            }),
            Some(false) => {}
            None => return None,
        }
    }

    if powered_inputs.len() == 1 {
        powered_inputs.pop()
    } else {
        None
    }
}

fn resolve_power_pin_net<'a>(component: &'a ComponentSpec, pin_name: &str) -> Option<&'a str> {
    component
        .power_domains
        .get(pin_name)
        .or_else(|| component.pins.get(pin_name))
        .or(component.power_domain.as_ref())
        .map(String::as_str)
}
