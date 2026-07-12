use crate::analog_model_resolver::{
    declared_model_file_path_for_source_dir, effective_model_files,
    inferred_model_file_for_model_path,
};
use crate::board_ir::{
    AnalogModelFile, AnalogScenario, AnalogSweepComponentField, ComponentSpec, SpicePrimitive,
    SpicePulseSpec,
};
use crate::library::{
    BoundBoard, ComponentModel, SpiceInstanceParameter, SpiceModel, SpiceModelType,
};
use crate::validation::analog_util::component_value_parameter_name;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

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
    let model_files = effective_model_files(bound, analog)?;
    for model_file in &model_files {
        if model_file.artifact_format.as_deref() == Some("osdi_shared_object") {
            continue;
        }
        let path =
            declared_model_file_path_for_source_dir(&bound.project.source_dir, &model_file.path)
                .map_err(|error| {
                    format!("Failed to resolve model file {}: {error}", model_file.path)
                })?;
        text.push_str(".include \"");
        text.push_str(&path.to_string_lossy());
        text.push_str("\"\n");
    }
    text.push('\n');
    for (name, value) in generated_component_value_parameters(bound, analog)? {
        text.push_str(".param ");
        text.push_str(&name);
        text.push('=');
        text.push_str(&value.to_string());
        text.push('\n');
    }
    if !generated.components.is_empty() {
        text.push('\n');
    }

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
        let line = generate_component_line(
            bound,
            analog,
            &model_files,
            &node_by_net,
            component_id,
            component,
            model,
        )?;
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
    model_files: &[AnalogModelFile],
    node_by_net: &BTreeMap<String, String>,
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
) -> Result<String, String> {
    if let Some(spice) = &component.spice {
        return match spice.primitive {
            SpicePrimitive::Resistor => passive_two_pin_line(
                analog,
                component_id,
                component,
                node_by_net,
                "R",
                component_value_expression(
                    analog,
                    component_id,
                    AnalogSweepComponentField::ValueOhm,
                    positive(spice.value_ohm, component_id, "spice.value_ohm")?,
                ),
                None,
            ),
            SpicePrimitive::Capacitor => {
                let initial_condition = if let Some(initial_v) = spice.initial_v {
                    if !initial_v.is_finite() {
                        return Err(format!(
                            "Component {component_id} spice.initial_v must be finite."
                        ));
                    }
                    Some(format!("IC={initial_v}"))
                } else {
                    None
                };
                passive_two_pin_line(
                    analog,
                    component_id,
                    component,
                    node_by_net,
                    "C",
                    component_value_expression(
                        analog,
                        component_id,
                        AnalogSweepComponentField::ValueF,
                        positive(spice.value_f, component_id, "spice.value_f")?,
                    ),
                    initial_condition.as_deref(),
                )
            }
            SpicePrimitive::Inductor => passive_two_pin_line(
                analog,
                component_id,
                component,
                node_by_net,
                "L",
                component_value_expression(
                    analog,
                    component_id,
                    AnalogSweepComponentField::ValueH,
                    positive(spice.value_h, component_id, "spice.value_h")?,
                ),
                None,
            ),
            SpicePrimitive::DcVoltageSource => Ok(format!(
                "{} {} {} DC {}{}",
                element_name("V", component_id),
                pin_node(component_id, component, node_by_net, "P")?,
                pin_node(component_id, component, node_by_net, "N")?,
                component_value_expression(
                    analog,
                    component_id,
                    AnalogSweepComponentField::DcV,
                    finite(spice.dc_v, component_id, "spice.dc_v")?,
                ),
                ac_source_suffix(analog),
            )),
            SpicePrimitive::PulseVoltageSource => {
                let pulse = spice.pulse.as_ref().ok_or_else(|| {
                    format!("Component {component_id} pulse_voltage_source requires spice.pulse.")
                })?;
                voltage_pulse_line(
                    component_id,
                    component,
                    node_by_net,
                    pulse,
                    ac_source_suffix(analog),
                )
            }
            SpicePrimitive::DcCurrentSource => Ok(format!(
                "{} {} {} DC {}{}",
                element_name("I", component_id),
                pin_node(component_id, component, node_by_net, "P")?,
                pin_node(component_id, component, node_by_net, "N")?,
                component_value_expression(
                    analog,
                    component_id,
                    AnalogSweepComponentField::DcA,
                    finite(spice.dc_a, component_id, "spice.dc_a")?,
                ),
                ac_source_suffix(analog),
            )),
            SpicePrimitive::PulseCurrentSource => {
                let pulse = spice.current_pulse.as_ref().ok_or_else(|| {
                    format!(
                        "Component {component_id} pulse_current_source requires spice.current_pulse."
                    )
                })?;
                current_pulse_line(
                    component_id,
                    component,
                    node_by_net,
                    pulse,
                    ac_source_suffix(analog),
                )
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
    require_declared_model_file(bound, model_files, component_id, &spice_model.model_path)?;
    match spice_model.model_type {
        SpiceModelType::Diode => {
            let anode = pin_node(component_id, component, node_by_net, "A")?;
            let sensed_anode = sense_node(component_id, "a");
            Ok(format!(
                "{} {} {} 0\n{} {} {} {}",
                current_sense_name("D", component_id),
                anode,
                sensed_anode,
                element_name("D", component_id),
                sensed_anode,
                pin_node(component_id, component, node_by_net, "K")?,
                spice_model.model_name
            ))
        }
        SpiceModelType::BjtNpn | SpiceModelType::BjtPnp => {
            let collector = pin_node(component_id, component, node_by_net, "C")?;
            let sensed_collector = sense_node(component_id, "c");
            Ok(format!(
                "{} {} {} 0\n{} {} {} {} {}",
                current_sense_name("Q", component_id),
                collector,
                sensed_collector,
                element_name("Q", component_id),
                sensed_collector,
                pin_node(component_id, component, node_by_net, "B")?,
                pin_node(component_id, component, node_by_net, "E")?,
                spice_model.model_name
            ))
        }
        SpiceModelType::MosfetN | SpiceModelType::MosfetP => {
            let drain = pin_node(component_id, component, node_by_net, "D")?;
            let sensed_drain = sense_node(component_id, "d");
            let source = pin_node(component_id, component, node_by_net, "S")?;
            let body =
                mosfet_body_node(component_id, component, node_by_net, spice_model, &source)?;
            Ok(format!(
                "{} {} {} 0\n{} {} {} {} {} {}",
                current_sense_name("M", component_id),
                drain,
                sensed_drain,
                element_name("M", component_id),
                sensed_drain,
                pin_node(component_id, component, node_by_net, "G")?,
                source,
                body,
                spice_model.model_name
            ))
        }
        SpiceModelType::Subckt => subckt_line(component_id, component, node_by_net, spice_model),
    }
}

fn passive_two_pin_line(
    analog: &AnalogScenario,
    component_id: &str,
    component: &ComponentSpec,
    node_by_net: &BTreeMap<String, String>,
    prefix: &str,
    value: String,
    suffix: Option<&str>,
) -> Result<String, String> {
    let node_a = pin_node(component_id, component, node_by_net, "A")?;
    let node_b = pin_node(component_id, component, node_by_net, "B")?;
    if passive_current_sense_requested(analog, prefix, component_id) {
        let node_a_for_component = sense_node(component_id, "a");
        let mut line = format!(
            "{} {} {} 0\n{} {} {} {}",
            current_sense_name(prefix, component_id),
            node_a,
            node_a_for_component,
            element_name(prefix, component_id),
            node_a_for_component,
            node_b,
            value
        );
        if let Some(suffix) = suffix {
            line.push(' ');
            line.push_str(suffix);
        }
        Ok(line)
    } else {
        let mut line = format!(
            "{} {} {} {}",
            element_name(prefix, component_id),
            node_a,
            node_b,
            value
        );
        if let Some(suffix) = suffix {
            line.push(' ');
            line.push_str(suffix);
        }
        Ok(line)
    }
}

fn generated_component_value_parameters(
    bound: &BoundBoard<'_>,
    analog: &AnalogScenario,
) -> Result<BTreeMap<String, f64>, String> {
    let Some(generated) = analog.generated.as_ref() else {
        return Ok(BTreeMap::new());
    };
    let mut parameters = BTreeMap::new();
    for component_id in &generated.components {
        let Some(component) = bound.project.board.components.get(component_id) else {
            continue;
        };
        let Some(spice) = &component.spice else {
            continue;
        };
        for field in swept_component_fields(analog, component_id) {
            let nominal = match (&spice.primitive, field) {
                (SpicePrimitive::Resistor, AnalogSweepComponentField::ValueOhm) => {
                    positive(spice.value_ohm, component_id, "spice.value_ohm")?
                }
                (SpicePrimitive::Capacitor, AnalogSweepComponentField::ValueF) => {
                    positive(spice.value_f, component_id, "spice.value_f")?
                }
                (SpicePrimitive::Inductor, AnalogSweepComponentField::ValueH) => {
                    positive(spice.value_h, component_id, "spice.value_h")?
                }
                (SpicePrimitive::DcVoltageSource, AnalogSweepComponentField::DcV) => {
                    finite(spice.dc_v, component_id, "spice.dc_v")?
                }
                (SpicePrimitive::DcCurrentSource, AnalogSweepComponentField::DcA) => {
                    finite(spice.dc_a, component_id, "spice.dc_a")?
                }
                _ => continue,
            };
            parameters.insert(
                component_value_parameter_name(component_id, field.as_str()),
                nominal,
            );
        }
    }
    Ok(parameters)
}

fn component_value_expression(
    analog: &AnalogScenario,
    component_id: &str,
    field: AnalogSweepComponentField,
    nominal: f64,
) -> String {
    if component_value_swept(analog, component_id, field) {
        format!(
            "{{{}}}",
            component_value_parameter_name(component_id, field.as_str())
        )
    } else {
        nominal.to_string()
    }
}

fn component_value_swept(
    analog: &AnalogScenario,
    component_id: &str,
    field: AnalogSweepComponentField,
) -> bool {
    analog.sweeps.iter().any(|sweep| {
        sweep.component_values.iter().any(|component_value| {
            component_value.component == component_id && component_value.field == field
        }) || sweep.monte_carlo.as_ref().is_some_and(|monte_carlo| {
            monte_carlo.component_values.iter().any(|component_value| {
                component_value.component == component_id && component_value.field == field
            })
        })
    })
}

fn swept_component_fields(
    analog: &AnalogScenario,
    component_id: &str,
) -> Vec<AnalogSweepComponentField> {
    let mut fields = Vec::new();
    for sweep in &analog.sweeps {
        for component_value in &sweep.component_values {
            if component_value.component == component_id && !fields.contains(&component_value.field)
            {
                fields.push(component_value.field);
            }
        }
        if let Some(monte_carlo) = &sweep.monte_carlo {
            for component_value in &monte_carlo.component_values {
                if component_value.component == component_id
                    && !fields.contains(&component_value.field)
                {
                    fields.push(component_value.field);
                }
            }
        }
    }
    fields
}

fn passive_current_sense_requested(
    analog: &AnalogScenario,
    prefix: &str,
    component_id: &str,
) -> bool {
    let branch = current_sense_name(prefix, component_id).to_ascii_lowercase();
    analog
        .probes
        .iter()
        .any(|probe| expression_references_branch_current(&probe.expression, &branch))
}

fn expression_references_branch_current(expression: &str, branch_lowercase: &str) -> bool {
    let normalized: String = expression
        .chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect();
    let current = format!("i({branch_lowercase})");
    normalized.contains(&current)
}

fn mosfet_body_node(
    component_id: &str,
    component: &ComponentSpec,
    node_by_net: &BTreeMap<String, String>,
    spice_model: &SpiceModel,
    source: &str,
) -> Result<String, String> {
    if let Some(body) = optional_pin_node(component_id, component, node_by_net, "B")? {
        return Ok(body);
    }
    match spice_model.body_pin_policy.as_deref() {
        Some("tie_to_source_when_absent") => Ok(source.to_string()),
        _ => Err(format!(
            "Generated SPICE MOSFET component {component_id} has no B body pin; model {} must declare simulation.spice.body_pin_policy=tie_to_source_when_absent or the board must bind pin B.",
            spice_model.model_name
        )),
    }
}

fn subckt_line(
    component_id: &str,
    component: &ComponentSpec,
    node_by_net: &BTreeMap<String, String>,
    spice_model: &SpiceModel,
) -> Result<String, String> {
    if spice_model.pin_order.is_empty() {
        return Err(format!(
            "Generated SPICE subckt component {component_id} model {} requires simulation.spice.pin_order.",
            spice_model.model_name
        ));
    }
    let mut line = element_name("X", component_id);
    for pin in &spice_model.pin_order {
        validate_spice_token("SPICE subckt pin", pin)?;
        line.push(' ');
        line.push_str(&pin_node(component_id, component, node_by_net, pin)?);
    }
    line.push(' ');
    line.push_str(&spice_model.model_name);
    for parameter in &spice_model.instance_parameters {
        line.push(' ');
        line.push_str(&instance_parameter_assignment(
            component_id,
            component,
            parameter,
        )?);
    }
    Ok(line)
}

fn instance_parameter_assignment(
    component_id: &str,
    component: &ComponentSpec,
    parameter: &SpiceInstanceParameter,
) -> Result<String, String> {
    validate_spice_token("SPICE instance parameter", &parameter.spice_name)?;
    let value = if let Some(value) = component.parameters.get(&parameter.component_parameter) {
        value.as_f64().ok_or_else(|| {
            format!(
                "Generated SPICE subckt component {component_id} parameter {} must be numeric for SPICE instance parameter {}.",
                parameter.component_parameter, parameter.spice_name
            )
        })?
    } else if let Some(default_value) = parameter.default_value {
        default_value
    } else {
        return Err(format!(
            "Generated SPICE subckt component {component_id} requires component parameter {} for SPICE instance parameter {}.",
            parameter.component_parameter, parameter.spice_name
        ));
    };
    if !value.is_finite() {
        return Err(format!(
            "Generated SPICE subckt component {component_id} parameter {} must be finite for SPICE instance parameter {}.",
            parameter.component_parameter, parameter.spice_name
        ));
    }
    Ok(format!("{}={value}", parameter.spice_name))
}

fn voltage_pulse_line(
    component_id: &str,
    component: &ComponentSpec,
    node_by_net: &BTreeMap<String, String>,
    pulse: &SpicePulseSpec,
    ac_suffix: &str,
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
        "{} {} {} PULSE({} {} {}u {}u {}u {}u {}u){}",
        element_name("V", component_id),
        pin_node(component_id, component, node_by_net, "P")?,
        pin_node(component_id, component, node_by_net, "N")?,
        pulse.initial_v,
        pulse.pulsed_v,
        pulse.delay_us,
        pulse.rise_us,
        pulse.fall_us,
        pulse.width_us,
        pulse.period_us,
        ac_suffix
    ))
}

fn current_pulse_line(
    component_id: &str,
    component: &ComponentSpec,
    node_by_net: &BTreeMap<String, String>,
    pulse: &crate::board_ir::SpiceCurrentPulseSpec,
    ac_suffix: &str,
) -> Result<String, String> {
    let fields = [
        ("initial_a", pulse.initial_a),
        ("pulsed_a", pulse.pulsed_a),
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
                "Component {component_id} spice.current_pulse.{field} must be finite and in range."
            ));
        }
    }
    Ok(format!(
        "{} {} {} PULSE({} {} {}u {}u {}u {}u {}u){}",
        element_name("I", component_id),
        pin_node(component_id, component, node_by_net, "P")?,
        pin_node(component_id, component, node_by_net, "N")?,
        pulse.initial_a,
        pulse.pulsed_a,
        pulse.delay_us,
        pulse.rise_us,
        pulse.fall_us,
        pulse.width_us,
        pulse.period_us,
        ac_suffix
    ))
}

fn ac_source_suffix(analog: &AnalogScenario) -> &'static str {
    if analog.analysis.analysis_type == "ac" {
        " AC 1"
    } else {
        ""
    }
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

fn optional_pin_node(
    component_id: &str,
    component: &ComponentSpec,
    node_by_net: &BTreeMap<String, String>,
    pin: &str,
) -> Result<Option<String>, String> {
    let Some(net) = component.pins.get(pin) else {
        return Ok(None);
    };
    let node = node_by_net.get(net).ok_or_else(|| {
        format!(
            "Generated SPICE component {component_id}.{pin} is on net {net}, but that net has no analog node binding."
        )
    })?;
    validate_spice_token("SPICE node", node)?;
    Ok(Some(node.clone()))
}

fn require_declared_model_file(
    bound: &BoundBoard<'_>,
    model_files: &[AnalogModelFile],
    component_id: &str,
    model_path: &str,
) -> Result<(), String> {
    let expected = inferred_model_file_for_model_path(&bound.project.source_dir, model_path)
        .map_err(|error| {
            format!("Failed to resolve model path {model_path} for {component_id}: {error}")
        })?
        .canonical_path;
    for model_file in model_files {
        let declared =
            declared_model_file_path_for_source_dir(&bound.project.source_dir, &model_file.path)
                .map_err(|error| {
                    format!(
                        "Failed to resolve declared model file {} for {component_id}: {error}",
                        model_file.path
                    )
                })?;
        if declared == expected {
            if model_file.sha256.is_none() {
                return Err(format!(
                    "Generated SPICE component {component_id} requires model file {model_path}, but the matching analog.model_files entry has no SHA-256 pin."
                ));
            }
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
    let suffix = element_suffix(component_id);
    if suffix.starts_with(prefix) {
        suffix
    } else {
        format!("{prefix}{suffix}")
    }
}

pub(super) fn current_sense_name(device_prefix: &str, component_id: &str) -> String {
    format!("VCCI_{}", element_name(device_prefix, component_id))
}

fn sense_node(component_id: &str, terminal: &str) -> String {
    format!(
        "cci_{}_{}",
        element_suffix(component_id).to_ascii_lowercase(),
        terminal
    )
}

fn element_suffix(component_id: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::{generate_board_netlist, subckt_line};
    use crate::board_ir::{ComponentSpec, load_project};
    use crate::library::{
        SpiceInstanceParameter, SpiceModel, SpiceModelType, bind_project, load_library,
    };
    use std::collections::BTreeMap;
    use std::path::Path;

    #[test]
    fn generated_diode_inserts_anode_current_sense_before_diode() {
        let project_path = Path::new("examples/good_diode_switching/project.yaml");
        let project = load_project(project_path).unwrap();
        let (library, findings) = load_library(project_path, &project);
        assert!(findings.is_empty());
        let bound = bind_project(&project, library, findings);
        let analog = project.scenarios[0].analog.as_ref().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let deck = dir.path().join("generated.cir");

        generate_board_netlist(&bound, analog, &deck).unwrap();

        let text = std::fs::read_to_string(deck).unwrap();
        let sense = text.find("VCCI_D1 in cci_d1_a 0").unwrap();
        let diode = text.find("D1 cci_d1_a out ONSEMI_1N4148WS").unwrap();
        assert!(sense < diode);
    }

    #[test]
    fn generated_subckt_maps_component_parameters_to_instance_parameters() {
        let component: ComponentSpec = serde_yaml_ng::from_str(
            "model: vendor.test.parameterized_subckt
parameters:
  programmed_charge_current_A: 2.0
pins:
  IN: input
  OUT: output
  GND: gnd
",
        )
        .unwrap();
        let node_by_net = BTreeMap::from([
            ("input".to_string(), "vin".to_string()),
            ("output".to_string(), "vout".to_string()),
            ("gnd".to_string(), "0".to_string()),
        ]);
        let spice_model = SpiceModel {
            model_name: "CIRCUITCI_PARAMETERIZED_SUBCKT".to_string(),
            model_type: SpiceModelType::Subckt,
            model_path: "models/spice/generic/analog_behavioral.lib".to_string(),
            provenance: "datasheet_limited_generic_behavioral".to_string(),
            model_package_name: None,
            model_package_version: None,
            model_package_artifact_id: None,
            model_package_lock_path: None,
            model_package_lock_sha256: None,
            model_package_registry_path: None,
            model_package_registry_sha256: None,
            model_package_registry_entry: None,
            body_pin_policy: None,
            pin_order: vec!["IN".to_string(), "OUT".to_string(), "GND".to_string()],
            instance_parameters: vec![
                SpiceInstanceParameter {
                    spice_name: "ICHG_A".to_string(),
                    component_parameter: "programmed_charge_current_A".to_string(),
                    default_value: None,
                },
                SpiceInstanceParameter {
                    spice_name: "VSYS_V".to_string(),
                    component_parameter: "observation_system_voltage_V".to_string(),
                    default_value: Some(12.0),
                },
            ],
            valid_operating_notes: Vec::new(),
        };

        let line = subckt_line("UCHG", &component, &node_by_net, &spice_model).unwrap();

        assert_eq!(
            line,
            "XUCHG vin vout 0 CIRCUITCI_PARAMETERIZED_SUBCKT ICHG_A=2 VSYS_V=12"
        );
    }

    #[test]
    fn generated_passive_inserts_current_sense_only_when_probe_requests_it() {
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(
            "project:
  name: passive_current_sense_test
  version: 0.1.0
board:
  components:
    R1:
      model: generic.analog.resistor
      spice:
        primitive: resistor
        value_ohm: 1000.0
      pins:
        A: in
        B: out
  nets:
    in:
      kind: power
      nominal_voltage: 5
      powered: true
    out:
      kind: digital_or_analog
    gnd:
      kind: ground
scenarios:
  - name: with_current_probe
    type: analog_transient
    checks: [SPICE_TRANSIENT_ANALYSIS]
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [R1]
      model_files: []
      node_bindings:
        - { net: in, node: in }
        - { net: out, node: out }
        - { net: gnd, node: '0' }
      pin_bindings:
        - { endpoint: { component: R1, pin: A }, node: in }
        - { endpoint: { component: R1, pin: B }, node: out }
      analysis: { type: tran, stop_time_us: 10, max_step_us: 1 }
      stimuli: []
      probes:
        - { name: r1_current, expression: I(VCCI_R1), quantity: current }
      assertions: []
",
        )
        .unwrap();
        let (library, findings) = load_library(Path::new("project.yaml"), &project);
        let bound = bind_project(&project, library, findings);
        let analog = project.scenarios[0].analog.as_ref().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let deck = dir.path().join("generated.cir");

        generate_board_netlist(&bound, analog, &deck).unwrap();

        let text = std::fs::read_to_string(deck).unwrap();
        let sense = text.find("VCCI_R1 in cci_r1_a 0").unwrap();
        let resistor = text.find("R1 cci_r1_a out 1000").unwrap();
        assert!(sense < resistor);
    }

    #[test]
    fn generated_ac_sources_emit_unity_small_signal_drive() {
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(
            "project:
  name: generated_ac_source_drive_test
  version: 0.1.0
board:
  components:
    V1:
      model: generic.analog.dc_voltage_source
      spice:
        primitive: dc_voltage_source
        dc_v: 1.0
      pins:
        P: input
        N: gnd
    R1:
      model: generic.analog.resistor
      spice:
        primitive: resistor
        value_ohm: 1000.0
      pins:
        A: input
        B: filtered
    C1:
      model: generic.analog.capacitor
      spice:
        primitive: capacitor
        value_f: 0.0000001
      pins:
        A: filtered
        B: gnd
  nets:
    input: {kind: digital_or_analog}
    filtered: {kind: digital_or_analog}
    gnd: {kind: ground}
scenarios:
  - name: bode
    type: analog_ac
    checks: [SPICE_AC_ANALYSIS]
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [V1, R1, C1]
      model_files: []
      node_bindings:
        - { net: input, node: input }
        - { net: filtered, node: filtered }
        - { net: gnd, node: '0' }
      pin_bindings:
        - { endpoint: { component: V1, pin: P }, node: input }
        - { endpoint: { component: V1, pin: N }, node: '0' }
        - { endpoint: { component: R1, pin: A }, node: input }
        - { endpoint: { component: R1, pin: B }, node: filtered }
        - { endpoint: { component: C1, pin: A }, node: filtered }
        - { endpoint: { component: C1, pin: B }, node: '0' }
      analysis: { type: ac, start_frequency_hz: 10, stop_frequency_hz: 100000, points_per_decade: 20 }
      stimuli: []
      probes:
        - { name: filtered, expression: V(filtered) }
      assertions: []
",
        )
        .unwrap();
        let (library, findings) = load_library(Path::new("project.yaml"), &project);
        let bound = bind_project(&project, library, findings);
        let analog = project.scenarios[0].analog.as_ref().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let deck = dir.path().join("generated.cir");

        generate_board_netlist(&bound, analog, &deck).unwrap();

        let text = std::fs::read_to_string(deck).unwrap();
        assert!(text.contains("V1 input 0 DC 1 AC 1"), "{text}");
        assert!(text.contains("R1 input filtered 1000"), "{text}");
        assert!(text.contains("C1 filtered 0 0.0000001"), "{text}");
    }

    #[test]
    fn generated_passive_omits_current_sense_when_probe_does_not_request_it() {
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(
            "project:
  name: passive_without_current_sense_test
  version: 0.1.0
board:
  components:
    R1:
      model: generic.analog.resistor
      spice:
        primitive: resistor
        value_ohm: 1000.0
      pins:
        A: in
        B: out
  nets:
    in:
      kind: power
      nominal_voltage: 5
      powered: true
    out:
      kind: digital_or_analog
    gnd:
      kind: ground
scenarios:
  - name: without_current_probe
    type: analog_transient
    checks: [SPICE_TRANSIENT_ANALYSIS]
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [R1]
      model_files: []
      node_bindings:
        - { net: in, node: in }
        - { net: out, node: out }
        - { net: gnd, node: '0' }
      pin_bindings:
        - { endpoint: { component: R1, pin: A }, node: in }
        - { endpoint: { component: R1, pin: B }, node: out }
      analysis: { type: tran, stop_time_us: 10, max_step_us: 1 }
      stimuli: []
      probes:
        - { name: out_voltage, expression: V(out), quantity: voltage }
      assertions: []
",
        )
        .unwrap();
        let (library, findings) = load_library(Path::new("project.yaml"), &project);
        let bound = bind_project(&project, library, findings);
        let analog = project.scenarios[0].analog.as_ref().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let deck = dir.path().join("generated.cir");

        generate_board_netlist(&bound, analog, &deck).unwrap();

        let text = std::fs::read_to_string(deck).unwrap();
        assert!(!text.contains("VCCI_R1"));
        assert!(text.contains("R1 in out 1000"));
    }

    #[test]
    fn generated_component_value_sweep_parameterizes_supported_primitives() {
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(
            "project:
  name: component_value_sweep_test
  version: 0.1.0
board:
  components:
    RLOAD:
      model: generic.analog.resistor
      spice:
        primitive: resistor
        value_ohm: 1000.0
      pins:
        A: out
        B: gnd
    ILOAD:
      model: generic.analog.dc_current_source
      spice:
        primitive: dc_current_source
        dc_a: 0.01
      pins:
        P: out
        N: gnd
  nets:
    out:
      kind: digital_or_analog
    gnd:
      kind: ground
scenarios:
  - name: load_sweep
    type: analog_transient
    checks: [SPICE_TRANSIENT_ANALYSIS]
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [RLOAD, ILOAD]
      model_files: []
      node_bindings:
        - { net: out, node: out }
        - { net: gnd, node: '0' }
      pin_bindings:
        - { endpoint: { component: RLOAD, pin: A }, node: out }
        - { endpoint: { component: RLOAD, pin: B }, node: '0' }
        - { endpoint: { component: ILOAD, pin: P }, node: out }
        - { endpoint: { component: ILOAD, pin: N }, node: '0' }
      analysis: { type: tran, stop_time_us: 10, max_step_us: 1 }
      stimuli: []
      sweeps:
        - name: load_corner
          component_values:
            - { component: RLOAD, field: value_ohm, values: [900, 1000, 1100] }
            - { component: ILOAD, field: dc_a, values: [0.005, 0.01, 0.02] }
      probes:
        - { name: out_voltage, expression: V(out), quantity: voltage }
      assertions: []
",
        )
        .unwrap();
        let (library, findings) = load_library(Path::new("project.yaml"), &project);
        let bound = bind_project(&project, library, findings);
        let analog = project.scenarios[0].analog.as_ref().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let deck = dir.path().join("generated.cir");

        generate_board_netlist(&bound, analog, &deck).unwrap();

        let text = std::fs::read_to_string(deck).unwrap();
        assert!(text.contains(".param CCI_RLOAD_VALUE_OHM=1000"));
        assert!(text.contains(".param CCI_ILOAD_DC_A=0.01"));
        assert!(text.contains("RLOAD out 0 {CCI_RLOAD_VALUE_OHM}"));
        assert!(text.contains("ILOAD out 0 DC {CCI_ILOAD_DC_A}"));
    }

    #[test]
    fn generated_passive_inserts_current_sense_for_power_probe_expression() {
        let project: crate::board_ir::BoardProject = serde_yaml_ng::from_str(
            "project:
  name: passive_power_sense_test
  version: 0.1.0
board:
  components:
    R1:
      model: generic.analog.resistor
      spice:
        primitive: resistor
        value_ohm: 1000.0
      pins:
        A: in
        B: out
  nets:
    in:
      kind: power
      nominal_voltage: 5
      powered: true
    out:
      kind: digital_or_analog
    gnd:
      kind: ground
scenarios:
  - name: with_power_probe
    type: analog_transient
    checks: [SPICE_TRANSIENT_ANALYSIS]
    analog:
      backend: auto
      netlist_source: generated_from_board
      generated:
        ground_net: gnd
        components: [R1]
      model_files: []
      node_bindings:
        - { net: in, node: in }
        - { net: out, node: out }
        - { net: gnd, node: '0' }
      pin_bindings:
        - { endpoint: { component: R1, pin: A }, node: in }
        - { endpoint: { component: R1, pin: B }, node: out }
      analysis: { type: tran, stop_time_us: 10, max_step_us: 1 }
      stimuli: []
      probes:
        - { name: r1_power, expression: 'V(in,out)*I(VCCI_R1)', quantity: power }
      assertions: []
",
        )
        .unwrap();
        let (library, findings) = load_library(Path::new("project.yaml"), &project);
        let bound = bind_project(&project, library, findings);
        let analog = project.scenarios[0].analog.as_ref().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let deck = dir.path().join("generated.cir");

        generate_board_netlist(&bound, analog, &deck).unwrap();

        let text = std::fs::read_to_string(deck).unwrap();
        assert!(text.contains("VCCI_R1 in cci_r1_a 0"));
        assert!(text.contains("R1 cci_r1_a out 1000"));
    }
}
