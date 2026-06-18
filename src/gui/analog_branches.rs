use anyhow::{Context, Result};
use std::path::Path;

pub(super) fn current_probe_expression(
    project: &crate::board_ir::BoardProject,
    project_path: &Path,
    component_id: &str,
) -> Result<String> {
    let component = project
        .board
        .components
        .get(component_id)
        .with_context(|| format!("Probe component {component_id} was not found."))?;
    if let Some(spice) = &component.spice {
        return primitive_current_probe_expression(component_id, &spice.primitive);
    }

    let (library, _findings) = crate::library::load_library(project_path, project);
    let model = library.get(&component.model).with_context(|| {
        format!(
            "Component {component_id} references model {}, but that model was not found in the active libraries.",
            component.model
        )
    })?;
    let spice = model.simulation.spice.as_ref().with_context(|| {
        format!(
            "Component {component_id} model {} does not declare simulation.spice metadata for current probing.",
            component.model
        )
    })?;
    let device_prefix = match spice.model_type {
        crate::library::SpiceModelType::Diode => "D",
        crate::library::SpiceModelType::BjtNpn | crate::library::SpiceModelType::BjtPnp => "Q",
        crate::library::SpiceModelType::MosfetN | crate::library::SpiceModelType::MosfetP => "M",
        crate::library::SpiceModelType::Subckt => {
            anyhow::bail!(
                "Component {component_id} uses a subcircuit model; add an explicit current-sense element or file-backed probe for branch current."
            );
        }
    };
    Ok(format!(
        "I({})",
        generated_current_sense_name(device_prefix, component_id)
    ))
}

fn primitive_current_probe_expression(
    component_id: &str,
    primitive: &crate::board_ir::SpicePrimitive,
) -> Result<String> {
    let prefix = match primitive {
        crate::board_ir::SpicePrimitive::DcVoltageSource
        | crate::board_ir::SpicePrimitive::PulseVoltageSource => "V",
        crate::board_ir::SpicePrimitive::DcCurrentSource
        | crate::board_ir::SpicePrimitive::PulseCurrentSource => "I",
        crate::board_ir::SpicePrimitive::Resistor => "R",
        crate::board_ir::SpicePrimitive::Capacitor => "C",
        crate::board_ir::SpicePrimitive::Inductor => "L",
    };
    let expression = match primitive {
        crate::board_ir::SpicePrimitive::Resistor
        | crate::board_ir::SpicePrimitive::Capacitor
        | crate::board_ir::SpicePrimitive::Inductor => {
            format!("I({})", generated_current_sense_name(prefix, component_id))
        }
        crate::board_ir::SpicePrimitive::DcVoltageSource
        | crate::board_ir::SpicePrimitive::PulseVoltageSource
        | crate::board_ir::SpicePrimitive::DcCurrentSource
        | crate::board_ir::SpicePrimitive::PulseCurrentSource => {
            format!("I({})", spice_element_name(prefix, component_id))
        }
    };
    Ok(expression)
}

pub(super) fn power_probe_expression(
    project: &crate::board_ir::BoardProject,
    project_path: &Path,
    analog: &crate::board_ir::AnalogScenario,
    component_id: &str,
) -> Result<String> {
    let (positive_pin, negative_pin) = component_voltage_pins(project, project_path, component_id)?;
    let positive_node = node_for_component_pin(analog, component_id, positive_pin)?;
    let negative_node = node_for_component_pin(analog, component_id, negative_pin)?;
    let current = current_probe_expression(project, project_path, component_id)?;
    Ok(format!("V({positive_node},{negative_node})*{current}"))
}

fn component_voltage_pins(
    project: &crate::board_ir::BoardProject,
    project_path: &Path,
    component_id: &str,
) -> Result<(&'static str, &'static str)> {
    let component = project
        .board
        .components
        .get(component_id)
        .with_context(|| format!("Probe component {component_id} was not found."))?;
    if let Some(spice) = &component.spice {
        return Ok(match spice.primitive {
            crate::board_ir::SpicePrimitive::Resistor
            | crate::board_ir::SpicePrimitive::Capacitor
            | crate::board_ir::SpicePrimitive::Inductor => ("A", "B"),
            crate::board_ir::SpicePrimitive::DcVoltageSource
            | crate::board_ir::SpicePrimitive::PulseVoltageSource
            | crate::board_ir::SpicePrimitive::DcCurrentSource
            | crate::board_ir::SpicePrimitive::PulseCurrentSource => ("P", "N"),
        });
    }

    let (library, _findings) = crate::library::load_library(project_path, project);
    let model = library.get(&component.model).with_context(|| {
        format!(
            "Component {component_id} references model {}, but that model was not found in the active libraries.",
            component.model
        )
    })?;
    let spice = model.simulation.spice.as_ref().with_context(|| {
        format!(
            "Component {component_id} model {} does not declare simulation.spice metadata for power probing.",
            component.model
        )
    })?;
    match spice.model_type {
        crate::library::SpiceModelType::Diode => Ok(("A", "K")),
        crate::library::SpiceModelType::BjtNpn | crate::library::SpiceModelType::BjtPnp => {
            Ok(("C", "E"))
        }
        crate::library::SpiceModelType::MosfetN | crate::library::SpiceModelType::MosfetP => {
            Ok(("D", "S"))
        }
        crate::library::SpiceModelType::Subckt => {
            anyhow::bail!(
                "Component {component_id} uses a subcircuit model; add an explicit file-backed power probe for subcircuit internals."
            );
        }
    }
}

fn node_for_component_pin(
    analog: &crate::board_ir::AnalogScenario,
    component_id: &str,
    pin_id: &str,
) -> Result<String> {
    analog
        .pin_bindings
        .iter()
        .find(|binding| binding.endpoint.component == component_id && binding.endpoint.pin == pin_id)
        .map(|binding| binding.node.clone())
        .with_context(|| {
            format!(
                "Scenario has no pin binding for component {component_id}.{pin_id}; power probing requires both branch voltage pins."
            )
        })
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
