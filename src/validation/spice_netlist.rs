use crate::board_ir::{AnalogScenario, ComponentSpec, SpicePrimitive, SpicePulseSpec};
use crate::library::{BoundBoard, ComponentModel, SpiceModelType};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub(super) fn generate_board_netlist(
    bound: &BoundBoard<'_>,
    analog: &AnalogScenario,
    path: &Path,
) -> Result<(), String> {
    let generated = analog.generated.as_ref().ok_or_else(|| {
        "analog.netlist_source generated_from_board requires analog.generated.".to_string()
    })?;
    if generated.components.is_empty() {
        return Err("analog.generated.components must contain at least one component.".to_string());
    }

    let node_by_net = node_bindings(analog)?;
    let ground_node = node_by_net.get(&generated.ground_net).ok_or_else(|| {
        format!(
            "analog.generated.ground_net {} has no node binding.",
            generated.ground_net
        )
    })?;
    if ground_node != "0" {
        return Err(format!(
            "analog.generated.ground_net {} must bind to SPICE node 0, but it binds to {}.",
            generated.ground_net, ground_node
        ));
    }

    let mut text = String::new();
    text.push_str("* Generated from CircuitCI Board IR. Do not edit by hand.\n");
    text.push_str("* Source project: ");
    text.push_str(&bound.project.project.name);
    text.push('\n');
    for model_file in &analog.model_files {
        let path =
            absolute_path(&bound.project.source_dir.join(&model_file.path)).map_err(|error| {
                format!("Failed to resolve model file {}: {error}", model_file.path)
            })?;
        text.push_str(".include \"");
        text.push_str(&path.to_string_lossy());
        text.push_str("\"\n");
    }
    text.push('\n');

    for component_id in &generated.components {
        let component = bound
            .project
            .board
            .components
            .get(component_id)
            .ok_or_else(|| {
                format!("Generated SPICE component {component_id} is not on the board.")
            })?;
        let model = bound.library.get(&component.model).ok_or_else(|| {
            format!(
                "Generated SPICE component {component_id} references unresolved model {}.",
                component.model
            )
        })?;
        let line =
            generate_component_line(bound, analog, &node_by_net, component_id, component, model)?;
        text.push_str(&line);
        text.push('\n');
    }

    fs::write(path, text).map_err(|error| {
        format!(
            "Failed to write generated SPICE netlist {}: {error}",
            path.display()
        )
    })
}

fn node_bindings(analog: &AnalogScenario) -> Result<BTreeMap<String, String>, String> {
    let mut node_by_net = BTreeMap::new();
    for binding in &analog.node_bindings {
        validate_spice_token("SPICE node", &binding.node)?;
        if let Some(previous) = node_by_net.insert(binding.net.clone(), binding.node.clone())
            && previous != binding.node
        {
            return Err(format!(
                "Board net {} has conflicting SPICE node bindings {} and {}.",
                binding.net, previous, binding.node
            ));
        }
    }
    Ok(node_by_net)
}

fn generate_component_line(
    bound: &BoundBoard<'_>,
    analog: &AnalogScenario,
    node_by_net: &BTreeMap<String, String>,
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
) -> Result<String, String> {
    if let Some(spice) = &component.spice {
        return match spice.primitive {
            SpicePrimitive::Resistor => Ok(format!(
                "{} {} {} {}",
                element_name("R", component_id),
                pin_node(component_id, component, node_by_net, "A")?,
                pin_node(component_id, component, node_by_net, "B")?,
                positive(spice.value_ohm, component_id, "spice.value_ohm")?
            )),
            SpicePrimitive::Capacitor => Ok(format!(
                "{} {} {} {}",
                element_name("C", component_id),
                pin_node(component_id, component, node_by_net, "A")?,
                pin_node(component_id, component, node_by_net, "B")?,
                positive(spice.value_f, component_id, "spice.value_f")?
            )),
            SpicePrimitive::DcVoltageSource => Ok(format!(
                "{} {} {} DC {}",
                element_name("V", component_id),
                pin_node(component_id, component, node_by_net, "P")?,
                pin_node(component_id, component, node_by_net, "N")?,
                finite(spice.dc_v, component_id, "spice.dc_v")?
            )),
            SpicePrimitive::PulseVoltageSource => {
                let pulse = spice.pulse.as_ref().ok_or_else(|| {
                    format!("Component {component_id} pulse_voltage_source requires spice.pulse.")
                })?;
                pulse_line(component_id, component, node_by_net, pulse)
            }
        };
    }

    let spice_model = model.simulation.spice.as_ref().ok_or_else(|| {
        format!(
            "Generated SPICE component {component_id} model {} lacks simulation.spice metadata.",
            model.component_id
        )
    })?;
    validate_spice_token("SPICE model name", &spice_model.model_name)?;
    require_declared_model_file(bound, analog, component_id, &spice_model.model_path)?;
    match spice_model.model_type {
        SpiceModelType::Diode => Ok(format!(
            "{} {} {} {}",
            element_name("D", component_id),
            pin_node(component_id, component, node_by_net, "A")?,
            pin_node(component_id, component, node_by_net, "K")?,
            spice_model.model_name
        )),
        SpiceModelType::BjtNpn | SpiceModelType::BjtPnp => Ok(format!(
            "{} {} {} {} {}",
            element_name("Q", component_id),
            pin_node(component_id, component, node_by_net, "C")?,
            pin_node(component_id, component, node_by_net, "B")?,
            pin_node(component_id, component, node_by_net, "E")?,
            spice_model.model_name
        )),
        SpiceModelType::MosfetN | SpiceModelType::MosfetP | SpiceModelType::Subckt => Err(format!(
            "Generated SPICE component {component_id} uses unsupported first-slice model type {:?}.",
            spice_model.model_type
        )),
    }
}

fn pulse_line(
    component_id: &str,
    component: &ComponentSpec,
    node_by_net: &BTreeMap<String, String>,
    pulse: &SpicePulseSpec,
) -> Result<String, String> {
    let fields = [
        ("initial_v", pulse.initial_v),
        ("pulsed_v", pulse.pulsed_v),
        ("delay_us", pulse.delay_us),
        ("rise_us", pulse.rise_us),
        ("fall_us", pulse.fall_us),
        ("width_us", pulse.width_us),
        ("period_us", pulse.period_us),
    ];
    for (field, value) in fields {
        if !value.is_finite()
            || (field.ends_with("_us") && value < 0.0)
            || matches!(field, "width_us" | "period_us") && value <= 0.0
        {
            return Err(format!(
                "Component {component_id} spice.pulse.{field} must be finite and in range."
            ));
        }
    }
    Ok(format!(
        "{} {} {} PULSE({} {} {}u {}u {}u {}u {}u)",
        element_name("V", component_id),
        pin_node(component_id, component, node_by_net, "P")?,
        pin_node(component_id, component, node_by_net, "N")?,
        pulse.initial_v,
        pulse.pulsed_v,
        pulse.delay_us,
        pulse.rise_us,
        pulse.fall_us,
        pulse.width_us,
        pulse.period_us
    ))
}

fn pin_node(
    component_id: &str,
    component: &ComponentSpec,
    node_by_net: &BTreeMap<String, String>,
    pin: &str,
) -> Result<String, String> {
    let net = component.pins.get(pin).ok_or_else(|| {
        format!("Generated SPICE component {component_id} is missing required pin {pin}.")
    })?;
    let node = node_by_net.get(net).ok_or_else(|| {
        format!(
            "Generated SPICE component {component_id}.{pin} is on net {net}, but that net has no analog node binding."
        )
    })?;
    validate_spice_token("SPICE node", node)?;
    Ok(node.clone())
}

fn require_declared_model_file(
    bound: &BoundBoard<'_>,
    analog: &AnalogScenario,
    component_id: &str,
    model_path: &str,
) -> Result<(), String> {
    let expected = absolute_path(Path::new(model_path)).map_err(|error| {
        format!("Failed to resolve model path {model_path} for {component_id}: {error}")
    })?;
    for model_file in &analog.model_files {
        let declared =
            absolute_path(&bound.project.source_dir.join(&model_file.path)).map_err(|error| {
                format!(
                    "Failed to resolve declared model file {} for {component_id}: {error}",
                    model_file.path
                )
            })?;
        if declared == expected {
            return Ok(());
        }
    }
    Err(format!(
        "Generated SPICE component {component_id} requires model file {model_path}, but analog.model_files does not declare it."
    ))
}

fn finite(value: Option<f64>, component_id: &str, field: &str) -> Result<f64, String> {
    value
        .filter(|value| value.is_finite())
        .ok_or_else(|| format!("Component {component_id} requires finite {field}."))
}

fn positive(value: Option<f64>, component_id: &str, field: &str) -> Result<f64, String> {
    value
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or_else(|| format!("Component {component_id} requires positive {field}."))
}

fn element_name(prefix: &str, component_id: &str) -> String {
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
    if suffix.starts_with(prefix) {
        suffix
    } else {
        format!("{prefix}{suffix}")
    }
}

fn validate_spice_token(label: &str, token: &str) -> Result<(), String> {
    if token.is_empty()
        || token
            .chars()
            .any(|character| character.is_whitespace() || matches!(character, '"' | '\''))
    {
        return Err(format!(
            "{label} {token:?} is not a supported generated SPICE token."
        ));
    }
    Ok(())
}

fn absolute_path(path: &Path) -> std::io::Result<PathBuf> {
    if path.is_absolute() {
        return Ok(normalize_path(path));
    }
    Ok(normalize_path(&env::current_dir()?.join(path)))
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(part) => normalized.push(part),
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }
    }
    normalized
}
