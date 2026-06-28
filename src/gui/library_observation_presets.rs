use super::analog::{
    AnalogAssertionDraft, AnalogProbeDraft, AnalogScenarioDraft, append_analog_assertion,
    append_analog_transient_scenario_with_project_path, append_analog_voltage_probe,
};
use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Clone)]
pub(super) struct ObservationPresetResult {
    pub(super) project_yaml: String,
    pub(super) scenario_name: String,
    pub(super) probe_count: usize,
}

pub(super) fn create_component_observation_preset(
    text: &str,
    project_path: &Path,
    component_id: &str,
) -> Result<ObservationPresetResult> {
    let component_id = component_id.trim();
    if component_id.is_empty() {
        anyhow::bail!("Select a component before creating an observation preset.");
    }
    let project: crate::board_ir::BoardProject =
        serde_yaml_ng::from_str(text).context("Project YAML is not valid Board IR.")?;
    let component = project
        .board
        .components
        .get(component_id)
        .with_context(|| format!("Component {component_id} was not found."))?;
    let (library, findings) = crate::library::load_library(project_path, &project);
    let hard_failures: Vec<_> = findings
        .iter()
        .filter(|finding| finding.id == "LIBRARY_NOT_FOUND" || finding.id == "MODEL_LOAD_FAILED")
        .map(|finding| finding.message.clone())
        .collect();
    if !hard_failures.is_empty() {
        anyhow::bail!("{}", hard_failures.join("; "));
    }
    let model = library
        .get(&component.model)
        .with_context(|| format!("Model {} was not found.", component.model))?;
    if model.simulation.spice.is_none() {
        anyhow::bail!(
            "Component {component_id} model {} has no generated-SPICE simulation face.",
            component.model
        );
    }
    let ground_net = observation_ground_net(&project, component, model)?;
    let probe_specs = observation_voltage_probe_specs(component_id, component, model, &ground_net);
    let first = probe_specs.first().with_context(|| {
        format!("Component {component_id} has no non-ground pin nets to probe.")
    })?;
    let scenario_name = unique_observation_scenario_name(&project, component_id);
    let mut updated = append_analog_transient_scenario_with_project_path(
        text,
        project_path,
        &AnalogScenarioDraft {
            name: scenario_name.clone(),
            ground_net,
            probe_net: first.net.clone(),
            probe_name: first.probe_name.clone(),
            stop_time_us: 100.0,
            max_step_us: 0.2,
        },
    )?;
    for probe in probe_specs.iter().skip(1) {
        updated = append_analog_voltage_probe(
            &updated,
            &AnalogProbeDraft {
                scenario_name: scenario_name.clone(),
                net_id: probe.net.clone(),
                probe_name: probe.probe_name.clone(),
            },
        )?;
    }
    for assertion in observation_default_assertions(
        &project,
        component,
        model,
        &probe_specs,
        &scenario_name,
        100.0,
    ) {
        updated = append_analog_assertion(&updated, &assertion)?;
    }
    Ok(ObservationPresetResult {
        project_yaml: updated,
        scenario_name,
        probe_count: probe_specs.len(),
    })
}

#[derive(Debug, Clone)]
struct ObservationProbeSpec {
    probe_name: String,
    net: String,
}

struct ComparatorPulseCheck<'a> {
    scenario_name: &'a str,
    output_probe_name: &'a str,
    name_stem: &'a str,
    pulse: &'a crate::board_ir::SpicePulseSpec,
    reference_v: f64,
    high_level_v: f64,
    margin_v: f64,
    pulse_above_reference_drives_output_high: bool,
    stop_time_us: f64,
}

fn observation_ground_net(
    project: &crate::board_ir::BoardProject,
    component: &crate::board_ir::ComponentSpec,
    model: &crate::library::ComponentModel,
) -> Result<String> {
    for (pin_id, port) in &model.ports {
        if port.kind == crate::library::PortKind::ElectricalGround
            && let Some(net) = component.pins.get(pin_id)
        {
            return Ok(net.clone());
        }
    }
    project
        .board
        .nets
        .iter()
        .find(|(_, net)| net.kind == crate::board_ir::NetKind::Ground)
        .map(|(net_id, _)| net_id.clone())
        .context("Observation presets require a ground net or an electrical-ground component pin.")
}

fn observation_voltage_probe_specs(
    component_id: &str,
    component: &crate::board_ir::ComponentSpec,
    model: &crate::library::ComponentModel,
    ground_net: &str,
) -> Vec<ObservationProbeSpec> {
    let mut probes = Vec::new();
    let mut seen_nets = BTreeSet::new();
    let mut seen_probe_names = BTreeSet::new();
    let pin_order = model
        .simulation
        .spice
        .as_ref()
        .map(|spice| spice.pin_order.as_slice())
        .unwrap_or(&[]);
    let mut pins = pin_order
        .iter()
        .filter(|pin| component.pins.contains_key(pin.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    for pin in component.pins.keys() {
        if !pins.iter().any(|ordered| ordered == pin) {
            pins.push(pin.clone());
        }
    }
    for pin in pins {
        let Some(net) = component.pins.get(&pin) else {
            continue;
        };
        if net == ground_net {
            continue;
        }
        if let Some(port) = model.ports.get(&pin)
            && port.kind == crate::library::PortKind::ElectricalGround
        {
            continue;
        }
        if !seen_nets.insert(net.clone()) {
            continue;
        }
        let base_probe_name = sanitize_observation_id(&format!("v_{component_id}_{pin}"));
        probes.push(ObservationProbeSpec {
            probe_name: unique_observation_probe_name(&base_probe_name, &mut seen_probe_names),
            net: net.clone(),
        });
    }
    probes
}

fn observation_default_assertions(
    project: &crate::board_ir::BoardProject,
    component: &crate::board_ir::ComponentSpec,
    model: &crate::library::ComponentModel,
    probes: &[ObservationProbeSpec],
    scenario_name: &str,
    stop_time_us: f64,
) -> Vec<AnalogAssertionDraft> {
    let mut assertions = Vec::new();
    if let Some(power) = &model.power_conversion
        && let Some(output_probe) = probe_for_component_pin(probes, component, &power.output_pin)
        && let Some(output_port) = model.ports.get(&power.output_pin)
    {
        if let Some(min_v) = output_port.electrical.operating_voltage_min_v {
            assertions.push(default_voltage_assertion(
                scenario_name,
                &format!("{}_min_voltage", output_probe.probe_name),
                &output_probe.probe_name,
                "min",
                "above",
                min_v,
                (0.0, stop_time_us),
            ));
        }
        if let Some(max_v) = output_port.electrical.operating_voltage_max_v {
            assertions.push(default_voltage_assertion(
                scenario_name,
                &format!("{}_max_voltage", output_probe.probe_name),
                &output_probe.probe_name,
                "max",
                "below",
                max_v,
                (0.0, stop_time_us),
            ));
        }
    }
    if let Some(reset) = &model.reset_supervisor
        && let Some(output_probe) =
            probe_for_component_pin(probes, component, &reset.reset_output_pin)
        && let Some(monitored_net) = component.pins.get(&reset.monitored_pin)
        && let Some(pulse) = pulse_source_for_net(project, monitored_net)
    {
        let low_time = (pulse.delay_us * 0.5).clamp(0.0, stop_time_us);
        let high_time =
            (pulse.delay_us + pulse.rise_us + stop_time_us * 0.1).clamp(0.0, stop_time_us);
        let low_threshold = (pulse.pulsed_v.abs() * 0.1).max(0.1);
        let high_threshold = pulse.pulsed_v * 0.9;
        match reset.active {
            crate::library::ResetSupervisorActive::Low => {
                assertions.push(default_sample_assertion(
                    scenario_name,
                    &format!("{}_asserted_low", output_probe.probe_name),
                    &output_probe.probe_name,
                    "below",
                    low_threshold,
                    low_time,
                ));
                assertions.push(default_sample_assertion(
                    scenario_name,
                    &format!("{}_released_high", output_probe.probe_name),
                    &output_probe.probe_name,
                    "above",
                    high_threshold,
                    high_time,
                ));
            }
            crate::library::ResetSupervisorActive::High => {
                assertions.push(default_sample_assertion(
                    scenario_name,
                    &format!("{}_asserted_high", output_probe.probe_name),
                    &output_probe.probe_name,
                    "above",
                    high_threshold,
                    low_time,
                ));
                assertions.push(default_sample_assertion(
                    scenario_name,
                    &format!("{}_released_low", output_probe.probe_name),
                    &output_probe.probe_name,
                    "below",
                    low_threshold,
                    high_time,
                ));
            }
        }
    }
    if let Some(power_switch) = &model.power_switch
        && let Some(output_probe) =
            probe_for_component_pin(probes, component, &power_switch.output_pin)
        && let Some(output_net) = component.pins.get(&power_switch.output_pin)
        && let Some(nominal_v) = nominal_voltage_for_net(project, output_net)
        && nominal_v > 0.0
    {
        assertions.push(default_voltage_assertion(
            scenario_name,
            &format!("{}_enabled_min_voltage", output_probe.probe_name),
            &output_probe.probe_name,
            "min",
            "above",
            nominal_v * 0.98,
            (0.0, stop_time_us),
        ));
        assertions.push(default_voltage_assertion(
            scenario_name,
            &format!("{}_enabled_max_voltage", output_probe.probe_name),
            &output_probe.probe_name,
            "max",
            "below",
            nominal_v * 1.02,
            (0.0, stop_time_us),
        ));
    }
    if let Some(power_mux) = &model.power_mux
        && let Some(output_probe) =
            probe_for_component_pin(probes, component, &power_mux.output_pin)
        && let Some(output_net) = component.pins.get(&power_mux.output_pin)
        && let Some(nominal_v) = nominal_voltage_for_net(project, output_net)
        && nominal_v > 0.0
    {
        assertions.push(default_voltage_assertion(
            scenario_name,
            &format!("{}_selected_source_min_voltage", output_probe.probe_name),
            &output_probe.probe_name,
            "min",
            "above",
            nominal_v * 0.98,
            (0.0, stop_time_us),
        ));
        assertions.push(default_voltage_assertion(
            scenario_name,
            &format!("{}_selected_source_max_voltage", output_probe.probe_name),
            &output_probe.probe_name,
            "max",
            "below",
            nominal_v * 1.02,
            (0.0, stop_time_us),
        ));
    }
    if let Some(charger) = &model.battery_charger
        && let Some(battery_probe) =
            probe_for_component_pin(probes, component, &charger.battery_pin)
        && let Some(battery_port) = model.ports.get(&charger.battery_pin)
    {
        let max_v = battery_port
            .electrical
            .operating_voltage_max_v
            .or(charger.regulation_voltage_v.map(|voltage| voltage * 1.01));
        if let Some(max_v) = max_v {
            assertions.push(default_voltage_assertion(
                scenario_name,
                &format!("{}_regulation_ceiling", battery_probe.probe_name),
                &battery_probe.probe_name,
                "max",
                "below",
                max_v,
                (0.0, stop_time_us),
            ));
        }
    }
    if model.battery_charger.is_some()
        && let Some(output_probe) = probe_for_component_pin(probes, component, "OUT")
        && let Some(output_port) = model.ports.get("OUT")
        && let Some(max_v) = output_port.electrical.operating_voltage_max_v
    {
        assertions.push(default_voltage_assertion(
            scenario_name,
            &format!("{}_power_path_ceiling", output_probe.probe_name),
            &output_probe.probe_name,
            "max",
            "below",
            max_v,
            (0.0, stop_time_us),
        ));
    }
    if supports_comms_output_observation(model) {
        add_comms_output_observation_assertions(
            component,
            model,
            probes,
            scenario_name,
            stop_time_us,
            &mut assertions,
        );
    }
    add_level_shifter_observation_assertions(
        project,
        component,
        model,
        probes,
        scenario_name,
        stop_time_us,
        &mut assertions,
    );
    add_protection_clamp_observation_assertions(
        component,
        model,
        probes,
        scenario_name,
        stop_time_us,
        &mut assertions,
    );
    if let Some(function) = &model.analog_function {
        match function.kind {
            crate::library::AnalogFunctionKind::OpAmp => {
                add_op_amp_observation_assertions(
                    project,
                    component,
                    function,
                    probes,
                    scenario_name,
                    stop_time_us,
                    &mut assertions,
                );
            }
            crate::library::AnalogFunctionKind::Comparator => {
                add_comparator_observation_assertions(
                    project,
                    component,
                    function,
                    probes,
                    scenario_name,
                    stop_time_us,
                    &mut assertions,
                );
            }
        }
    }
    assertions
}

fn supports_comms_output_observation(model: &crate::library::ComponentModel) -> bool {
    matches!(model.category.as_str(), "comms" | "rs485_transceiver")
}

fn add_comms_output_observation_assertions(
    component: &crate::board_ir::ComponentSpec,
    model: &crate::library::ComponentModel,
    probes: &[ObservationProbeSpec],
    scenario_name: &str,
    stop_time_us: f64,
    assertions: &mut Vec<AnalogAssertionDraft>,
) {
    for (pin, port) in &model.ports {
        if port.kind != crate::library::PortKind::DigitalElectricalOutput {
            continue;
        }
        let Some(output_probe) = probe_for_component_pin(probes, component, pin) else {
            continue;
        };
        let Some(high_threshold) = port.electrical.drive_high_voltage_v else {
            continue;
        };
        let state = component_parameter_f64(
            component,
            &format!("observation_{}_state", pin.to_ascii_lowercase()),
        )
        .unwrap_or(1.0);
        let (suffix, relation, threshold) = if state >= 0.5 {
            ("output_high", "above", high_threshold)
        } else {
            ("output_low", "below", 0.5)
        };
        assertions.push(default_voltage_assertion(
            scenario_name,
            &format!("{}_{}", output_probe.probe_name, suffix),
            &output_probe.probe_name,
            "mean",
            relation,
            threshold,
            (0.0, stop_time_us),
        ));
    }
}

fn add_op_amp_observation_assertions(
    project: &crate::board_ir::BoardProject,
    component: &crate::board_ir::ComponentSpec,
    function: &crate::library::AnalogFunction,
    probes: &[ObservationProbeSpec],
    scenario_name: &str,
    stop_time_us: f64,
    assertions: &mut Vec<AnalogAssertionDraft>,
) {
    let (Some(non_inverting_pin), Some(inverting_pin), Some(output_pin)) = (
        function.positive_input_pin.as_deref(),
        function.negative_input_pin.as_deref(),
        function.output_pin.as_deref(),
    ) else {
        return;
    };
    let Some(non_inverting_net) = component.pins.get(non_inverting_pin) else {
        return;
    };
    let Some(inverting_net) = component.pins.get(inverting_pin) else {
        return;
    };
    let Some(output_net) = component.pins.get(output_pin) else {
        return;
    };
    if inverting_net != output_net {
        return;
    }
    let Some(output_probe) = probe_for_net(probes, output_net) else {
        return;
    };
    let Some(input_pulse) = pulse_source_for_net(project, non_inverting_net) else {
        return;
    };
    let tolerance = function.default_output_tolerance_v.unwrap_or(0.05);
    add_tracks_pulse_assertions(
        scenario_name,
        &output_probe.probe_name,
        "tracks_input",
        input_pulse,
        tolerance,
        stop_time_us,
        assertions,
    );
}

fn add_comparator_observation_assertions(
    project: &crate::board_ir::BoardProject,
    component: &crate::board_ir::ComponentSpec,
    function: &crate::library::AnalogFunction,
    probes: &[ObservationProbeSpec],
    scenario_name: &str,
    stop_time_us: f64,
    assertions: &mut Vec<AnalogAssertionDraft>,
) {
    let (Some(non_inverting_pin), Some(inverting_pin), Some(output_pin), Some(supply_pin)) = (
        function.positive_input_pin.as_deref(),
        function.negative_input_pin.as_deref(),
        function.output_pin.as_deref(),
        function.positive_supply_pin.as_deref(),
    ) else {
        return;
    };
    let Some(non_inverting_net) = component.pins.get(non_inverting_pin) else {
        return;
    };
    let Some(inverting_net) = component.pins.get(inverting_pin) else {
        return;
    };
    let Some(output_net) = component.pins.get(output_pin) else {
        return;
    };
    let Some(supply_net) = component.pins.get(supply_pin) else {
        return;
    };
    let Some(output_probe) = probe_for_net(probes, output_net) else {
        return;
    };
    let Some(high_level) = dc_voltage_for_net(project, supply_net)
        .or_else(|| nominal_voltage_for_net(project, supply_net))
    else {
        return;
    };
    let margin = function.default_logic_margin_v.unwrap_or(0.5);
    if let Some(pulse) = pulse_source_for_net(project, non_inverting_net)
        && let Some(reference) = dc_voltage_for_net(project, inverting_net)
            .or_else(|| nominal_voltage_for_net(project, inverting_net))
    {
        add_comparator_pulse_assertions(
            &ComparatorPulseCheck {
                scenario_name,
                output_probe_name: &output_probe.probe_name,
                name_stem: "positive_input",
                pulse,
                reference_v: reference,
                high_level_v: high_level,
                margin_v: margin,
                pulse_above_reference_drives_output_high: true,
                stop_time_us,
            },
            assertions,
        );
    } else if let Some(pulse) = pulse_source_for_net(project, inverting_net)
        && let Some(reference) = dc_voltage_for_net(project, non_inverting_net)
            .or_else(|| nominal_voltage_for_net(project, non_inverting_net))
    {
        add_comparator_pulse_assertions(
            &ComparatorPulseCheck {
                scenario_name,
                output_probe_name: &output_probe.probe_name,
                name_stem: "negative_input",
                pulse,
                reference_v: reference,
                high_level_v: high_level,
                margin_v: margin,
                pulse_above_reference_drives_output_high: false,
                stop_time_us,
            },
            assertions,
        );
    }
}

fn add_level_shifter_observation_assertions(
    project: &crate::board_ir::BoardProject,
    component: &crate::board_ir::ComponentSpec,
    model: &crate::library::ComponentModel,
    probes: &[ObservationProbeSpec],
    scenario_name: &str,
    stop_time_us: f64,
    assertions: &mut Vec<AnalogAssertionDraft>,
) {
    for channel in &model.signal_conditioning.channels {
        if channel.kind != crate::library::SignalConditioningKind::LevelShifter {
            continue;
        }
        let Some(side_a_probe) = probe_for_component_pin(probes, component, &channel.side_a_pin)
        else {
            continue;
        };
        let Some(side_b_probe) = probe_for_component_pin(probes, component, &channel.side_b_pin)
        else {
            continue;
        };
        let Some(side_a_supply_pin) = channel.side_a_supply_pin.as_deref() else {
            continue;
        };
        let Some(side_b_supply_pin) = channel.side_b_supply_pin.as_deref() else {
            continue;
        };
        let Some(side_a_v) = voltage_for_component_pin(project, component, side_a_supply_pin)
        else {
            continue;
        };
        let Some(side_b_v) = voltage_for_component_pin(project, component, side_b_supply_pin)
        else {
            continue;
        };
        add_port_voltage_window_assertions(
            component,
            model,
            probes,
            scenario_name,
            side_a_supply_pin,
            stop_time_us,
            assertions,
        );
        add_port_voltage_window_assertions(
            component,
            model,
            probes,
            scenario_name,
            side_b_supply_pin,
            stop_time_us,
            assertions,
        );
        if let Some(enable_pin) = channel.enable_pin.as_deref()
            && let Some(enable_probe) = probe_for_component_pin(probes, component, enable_pin)
        {
            assertions.push(default_voltage_assertion(
                scenario_name,
                &format!("{}_enable_high", enable_probe.probe_name),
                &enable_probe.probe_name,
                "mean",
                "above",
                side_a_v * 0.7,
                (0.0, stop_time_us),
            ));
        }
        let state = component_parameter_f64(
            component,
            &format!("observation_{}_state", channel.name.to_ascii_lowercase()),
        )
        .unwrap_or(1.0);
        if state >= 0.5 {
            assertions.push(default_voltage_assertion(
                scenario_name,
                &format!("{}_input_high", side_a_probe.probe_name),
                &side_a_probe.probe_name,
                "mean",
                "above",
                side_a_v * 0.7,
                (0.0, stop_time_us),
            ));
            assertions.push(default_voltage_assertion(
                scenario_name,
                &format!("{}_translated_high", side_b_probe.probe_name),
                &side_b_probe.probe_name,
                "mean",
                "above",
                side_b_v * 0.7,
                (0.0, stop_time_us),
            ));
        } else {
            assertions.push(default_voltage_assertion(
                scenario_name,
                &format!("{}_input_low", side_a_probe.probe_name),
                &side_a_probe.probe_name,
                "mean",
                "below",
                side_a_v * 0.3,
                (0.0, stop_time_us),
            ));
            assertions.push(default_voltage_assertion(
                scenario_name,
                &format!("{}_translated_low", side_b_probe.probe_name),
                &side_b_probe.probe_name,
                "mean",
                "below",
                side_b_v * 0.3,
                (0.0, stop_time_us),
            ));
        }
    }
}

fn add_port_voltage_window_assertions(
    component: &crate::board_ir::ComponentSpec,
    model: &crate::library::ComponentModel,
    probes: &[ObservationProbeSpec],
    scenario_name: &str,
    pin: &str,
    stop_time_us: f64,
    assertions: &mut Vec<AnalogAssertionDraft>,
) {
    let Some(probe) = probe_for_component_pin(probes, component, pin) else {
        return;
    };
    let Some(port) = model.ports.get(pin) else {
        return;
    };
    if let Some(min_v) = port.electrical.operating_voltage_min_v {
        assertions.push(default_voltage_assertion(
            scenario_name,
            &format!("{}_min_voltage", probe.probe_name),
            &probe.probe_name,
            "mean",
            "above",
            min_v,
            (0.0, stop_time_us),
        ));
    }
    if let Some(max_v) = port.electrical.operating_voltage_max_v {
        assertions.push(default_voltage_assertion(
            scenario_name,
            &format!("{}_max_voltage", probe.probe_name),
            &probe.probe_name,
            "mean",
            "below",
            max_v,
            (0.0, stop_time_us),
        ));
    }
}

fn add_protection_clamp_observation_assertions(
    component: &crate::board_ir::ComponentSpec,
    model: &crate::library::ComponentModel,
    probes: &[ObservationProbeSpec],
    scenario_name: &str,
    stop_time_us: f64,
    assertions: &mut Vec<AnalogAssertionDraft>,
) {
    for clamp in &model.signal_conditioning.protection_clamps {
        let Some(max_v) = clamp.working_voltage_max_v else {
            continue;
        };
        let Some(probe) = probe_for_component_pin(probes, component, &clamp.protected_pin) else {
            continue;
        };
        assertions.push(default_voltage_assertion(
            scenario_name,
            &format!("{}_{}_standoff", probe.probe_name, clamp.name),
            &probe.probe_name,
            "max",
            "below",
            max_v,
            (0.0, stop_time_us),
        ));
    }
}

fn add_tracks_pulse_assertions(
    scenario_name: &str,
    output_probe_name: &str,
    name_stem: &str,
    pulse: &crate::board_ir::SpicePulseSpec,
    tolerance: f64,
    stop_time_us: f64,
    assertions: &mut Vec<AnalogAssertionDraft>,
) {
    let low_time = (pulse.delay_us * 0.5).clamp(0.0, stop_time_us);
    let high_time = pulse_high_sample_time(pulse, stop_time_us);
    assertions.push(default_sample_assertion(
        scenario_name,
        &format!("{output_probe_name}_{name_stem}_low_above"),
        output_probe_name,
        "above",
        pulse.initial_v - tolerance,
        low_time,
    ));
    assertions.push(default_sample_assertion(
        scenario_name,
        &format!("{output_probe_name}_{name_stem}_low_below"),
        output_probe_name,
        "below",
        pulse.initial_v + tolerance,
        low_time,
    ));
    assertions.push(default_sample_assertion(
        scenario_name,
        &format!("{output_probe_name}_{name_stem}_high_above"),
        output_probe_name,
        "above",
        pulse.pulsed_v - tolerance,
        high_time,
    ));
    assertions.push(default_sample_assertion(
        scenario_name,
        &format!("{output_probe_name}_{name_stem}_high_below"),
        output_probe_name,
        "below",
        pulse.pulsed_v + tolerance,
        high_time,
    ));
}

fn add_comparator_pulse_assertions(
    check: &ComparatorPulseCheck<'_>,
    assertions: &mut Vec<AnalogAssertionDraft>,
) {
    let low_time = (check.pulse.delay_us * 0.5).clamp(0.0, check.stop_time_us);
    let high_time = pulse_high_sample_time(check.pulse, check.stop_time_us);
    let low_relation = if (check.pulse.initial_v > check.reference_v)
        == check.pulse_above_reference_drives_output_high
    {
        "above"
    } else {
        "below"
    };
    let high_relation = if (check.pulse.pulsed_v > check.reference_v)
        == check.pulse_above_reference_drives_output_high
    {
        "above"
    } else {
        "below"
    };
    push_comparator_state_assertion(
        check,
        &format!("{}_low_state", check.name_stem),
        low_relation,
        low_time,
        assertions,
    );
    push_comparator_state_assertion(
        check,
        &format!("{}_high_state", check.name_stem),
        high_relation,
        high_time,
        assertions,
    );
}

fn push_comparator_state_assertion(
    check: &ComparatorPulseCheck<'_>,
    assertion_name: &str,
    input_relation_to_reference: &str,
    at_us: f64,
    assertions: &mut Vec<AnalogAssertionDraft>,
) {
    let (relation, threshold) = if input_relation_to_reference == "above" {
        ("above", check.high_level_v - check.margin_v)
    } else {
        ("below", check.margin_v)
    };
    assertions.push(default_sample_assertion(
        check.scenario_name,
        &format!("{}_{}", check.output_probe_name, assertion_name),
        check.output_probe_name,
        relation,
        threshold,
        at_us,
    ));
}

fn probe_for_component_pin<'a>(
    probes: &'a [ObservationProbeSpec],
    component: &crate::board_ir::ComponentSpec,
    pin: &str,
) -> Option<&'a ObservationProbeSpec> {
    let net = component.pins.get(pin)?;
    probe_for_net(probes, net)
}

fn probe_for_net<'a>(
    probes: &'a [ObservationProbeSpec],
    net: &str,
) -> Option<&'a ObservationProbeSpec> {
    probes.iter().find(|probe| probe.net == net)
}

fn pulse_source_for_net<'a>(
    project: &'a crate::board_ir::BoardProject,
    net_id: &str,
) -> Option<&'a crate::board_ir::SpicePulseSpec> {
    project.board.components.values().find_map(|component| {
        let spice = component.spice.as_ref()?;
        if spice.primitive != crate::board_ir::SpicePrimitive::PulseVoltageSource {
            return None;
        }
        if component.pins.get("P").is_some_and(|net| net == net_id) {
            spice.pulse.as_ref()
        } else {
            None
        }
    })
}

fn dc_voltage_for_net(project: &crate::board_ir::BoardProject, net_id: &str) -> Option<f64> {
    project.board.components.values().find_map(|component| {
        let spice = component.spice.as_ref()?;
        if spice.primitive != crate::board_ir::SpicePrimitive::DcVoltageSource {
            return None;
        }
        if component.pins.get("P").is_some_and(|net| net == net_id) {
            spice.dc_v
        } else {
            None
        }
    })
}

fn nominal_voltage_for_net(project: &crate::board_ir::BoardProject, net_id: &str) -> Option<f64> {
    project
        .board
        .nets
        .get(net_id)
        .and_then(|net| net.nominal_voltage)
}

fn voltage_for_component_pin(
    project: &crate::board_ir::BoardProject,
    component: &crate::board_ir::ComponentSpec,
    pin: &str,
) -> Option<f64> {
    let net = component.pins.get(pin)?;
    dc_voltage_for_net(project, net).or_else(|| nominal_voltage_for_net(project, net))
}

fn pulse_high_sample_time(pulse: &crate::board_ir::SpicePulseSpec, stop_time_us: f64) -> f64 {
    let high_time = pulse.delay_us + pulse.rise_us + pulse.width_us * 0.5;
    high_time.clamp(0.0, stop_time_us)
}

fn component_parameter_f64(
    component: &crate::board_ir::ComponentSpec,
    parameter: &str,
) -> Option<f64> {
    component
        .parameters
        .get(parameter)
        .and_then(serde_yaml_ng::Value::as_f64)
}

fn default_voltage_assertion(
    scenario_name: &str,
    assertion_name: &str,
    probe_name: &str,
    aggregation: &str,
    relation: &str,
    threshold: f64,
    window_us: (f64, f64),
) -> AnalogAssertionDraft {
    let (start_us, end_us) = window_us;
    AnalogAssertionDraft {
        scenario_name: scenario_name.to_string(),
        assertion_name: sanitize_observation_id(assertion_name),
        probe_name: probe_name.to_string(),
        reference_probe: String::new(),
        aggregation: aggregation.to_string(),
        relation: relation.to_string(),
        threshold,
        reference_threshold: 0.0,
        target: 0.0,
        tolerance: 0.0,
        at_us: 0.0,
        at_hz: 1000.0,
        start_us,
        end_us,
        time_limit_us: 0.0,
        frequency_limit_hz: 1000.0,
        duty_limit_percent: 0.0,
        count_limit: 0.0,
        overshoot_limit_percent: 0.0,
    }
}

fn default_sample_assertion(
    scenario_name: &str,
    assertion_name: &str,
    probe_name: &str,
    relation: &str,
    threshold: f64,
    at_us: f64,
) -> AnalogAssertionDraft {
    AnalogAssertionDraft {
        scenario_name: scenario_name.to_string(),
        assertion_name: sanitize_observation_id(assertion_name),
        probe_name: probe_name.to_string(),
        reference_probe: String::new(),
        aggregation: "sample".to_string(),
        relation: relation.to_string(),
        threshold,
        reference_threshold: 0.0,
        target: 0.0,
        tolerance: 0.0,
        at_us,
        at_hz: 1000.0,
        start_us: 0.0,
        end_us: 0.0,
        time_limit_us: 0.0,
        frequency_limit_hz: 1000.0,
        duty_limit_percent: 0.0,
        count_limit: 0.0,
        overshoot_limit_percent: 0.0,
    }
}

fn unique_observation_scenario_name(
    project: &crate::board_ir::BoardProject,
    component_id: &str,
) -> String {
    let base = sanitize_observation_id(&format!("{component_id}_observation"));
    let existing = project
        .scenarios
        .iter()
        .map(|scenario| scenario.name.as_str())
        .collect::<BTreeSet<_>>();
    if !existing.contains(base.as_str()) {
        return base;
    }
    for suffix in 2.. {
        let candidate = format!("{base}_{suffix}");
        if !existing.contains(candidate.as_str()) {
            return candidate;
        }
    }
    unreachable!("unbounded suffix search must return")
}

fn unique_observation_probe_name(base: &str, seen: &mut BTreeSet<String>) -> String {
    if seen.insert(base.to_string()) {
        return base.to_string();
    }
    for suffix in 2.. {
        let candidate = format!("{base}_{suffix}");
        if seen.insert(candidate.clone()) {
            return candidate;
        }
    }
    unreachable!("unbounded suffix search must return")
}

fn sanitize_observation_id(value: &str) -> String {
    let mut out = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            out.push(character.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    let out = out.trim_matches('_');
    if out.is_empty() {
        "observation".to_string()
    } else if out
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
    {
        format!("obs_{out}")
    } else {
        out.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::create_component_observation_preset;
    use std::path::Path;

    fn ap2112_project_yaml() -> &'static str {
        "project:
  name: ap2112_observation_preset_test
  version: 0.1.0
libraries:
  - libs/generic/analog
  - libs/vendor/diodes/regulators
board:
  components:
    VUSB:
      model: generic.analog.dc_voltage_source
      pins: { P: usb_5v, N: gnd }
      spice: { primitive: dc_voltage_source, dc_v: 5.0 }
    VEN:
      model: generic.analog.dc_voltage_source
      pins: { P: regulator_en, N: gnd }
      spice: { primitive: dc_voltage_source, dc_v: 5.0 }
    UREG:
      model: vendor.diodes.ap2112k_3v3
      pins: { VIN: usb_5v, EN: regulator_en, GND: gnd, VOUT: rail_3v3 }
    RLOAD:
      model: generic.analog.resistor
      pins: { A: rail_3v3, B: gnd }
      spice: { primitive: resistor, value_ohm: 3300.0 }
  nets:
    usb_5v: { kind: power, nominal_voltage: 5.0, powered: true }
    regulator_en: { kind: power }
    rail_3v3: { kind: power, nominal_voltage: 3.3, powered: true }
    gnd: { kind: ground }
"
    }

    fn tlv803_project_yaml() -> &'static str {
        "project:
  name: tlv803_observation_preset_test
  version: 0.1.0
libraries:
  - libs/generic/analog
  - libs/vendor/ti/reset_supervisors
board:
  components:
    VDD:
      model: generic.analog.pulse_voltage_source
      pins: { P: rail_3v3, N: gnd }
      spice:
        primitive: pulse_voltage_source
        pulse:
          initial_v: 0.0
          pulsed_v: 3.3
          delay_us: 10.0
          rise_us: 1.0
          fall_us: 1.0
          width_us: 80.0
          period_us: 160.0
    URESET:
      model: vendor.ti.tlv803ea29
      pins: { VDD: rail_3v3, GND: gnd, RESET: reset_n }
    RPU:
      model: generic.analog.resistor
      pins: { A: rail_3v3, B: reset_n }
      spice: { primitive: resistor, value_ohm: 10000.0 }
  nets:
    rail_3v3: { kind: power, nominal_voltage: 3.3, powered: true }
    reset_n: { kind: digital_or_analog }
    gnd: { kind: ground }
"
    }

    fn tps22918_project_yaml() -> &'static str {
        "project:
  name: tps22918_observation_preset_test
  version: 0.1.0
libraries:
  - libs/generic/analog
  - libs/vendor/ti/load_switches
board:
  components:
    VUSB:
      model: generic.analog.dc_voltage_source
      pins: { P: usb_5v, N: gnd }
      spice: { primitive: dc_voltage_source, dc_v: 5.0 }
    VON:
      model: generic.analog.dc_voltage_source
      pins: { P: switch_on, N: gnd }
      spice: { primitive: dc_voltage_source, dc_v: 5.0 }
    USW:
      model: vendor.ti.tps22918
      pins: { VIN: usb_5v, VOUT: switched_5v, GND: gnd, ON: switch_on }
    RLOAD:
      model: generic.analog.resistor
      pins: { A: switched_5v, B: gnd }
      spice: { primitive: resistor, value_ohm: 1000.0 }
  nets:
    usb_5v: { kind: power, nominal_voltage: 5.0, powered: true }
    switched_5v: { kind: power, nominal_voltage: 5.0, powered: true }
    switch_on: { kind: digital_or_analog }
    gnd: { kind: ground }
"
    }

    fn mcp73831_project_yaml() -> &'static str {
        "project:
  name: mcp73831_observation_preset_test
  version: 0.1.0
libraries:
  - libs/generic/analog
  - libs/vendor/microchip/chargers
board:
  components:
    VUSB:
      model: generic.analog.dc_voltage_source
      pins: { P: usb_5v, N: gnd }
      spice: { primitive: dc_voltage_source, dc_v: 5.0 }
    UCHG:
      model: vendor.microchip.mcp73831_4v2
      parameters:
        programmed_charge_current_A: 0.1
      power_domains:
        VDD: usb_5v
        VBAT: battery
      pins: { VDD: usb_5v, VBAT: battery, VSS: gnd, PROG: prog }
    RPROG:
      model: generic.analog.resistor
      pins: { A: prog, B: gnd }
      spice: { primitive: resistor, value_ohm: 10000.0 }
    CBAT:
      model: generic.analog.capacitor
      pins: { A: battery, B: gnd }
      spice: { primitive: capacitor, value_f: 0.000001 }
  nets:
    usb_5v: { kind: power, nominal_voltage: 5.0, powered: true }
    battery: { kind: power, nominal_voltage: 4.2, powered: true }
    prog: { kind: digital_or_analog }
    gnd: { kind: ground }
"
    }

    fn bq24075_project_yaml() -> &'static str {
        "project:
  name: bq24075_observation_preset_test
  version: 0.1.0
libraries:
  - libs/generic/analog
  - libs/vendor/ti/chargers
board:
  components:
    VIN:
      model: generic.analog.dc_voltage_source
      pins: { P: adapter_6v, N: gnd }
      spice: { primitive: dc_voltage_source, dc_v: 6.0 }
    UCHG:
      model: vendor.ti.bq24075
      parameters:
        programmed_charge_current_A: 0.45
      power_domains:
        IN: adapter_6v
        BAT: battery
        OUT: sys_out
      pins: { IN: adapter_6v, BAT: battery, OUT: sys_out, VSS: gnd, ISET: iset }
    RISET:
      model: generic.analog.resistor
      pins: { A: iset, B: gnd }
      spice: { primitive: resistor, value_ohm: 1977.7777777778 }
    RLOAD:
      model: generic.analog.resistor
      pins: { A: sys_out, B: gnd }
      spice: { primitive: resistor, value_ohm: 5500.0 }
  nets:
    adapter_6v: { kind: power, nominal_voltage: 6.0, powered: true }
    battery: { kind: power, nominal_voltage: 4.2, powered: true }
    sys_out: { kind: power, nominal_voltage: 5.5, powered: true }
    iset: { kind: digital_or_analog }
    gnd: { kind: ground }
"
    }

    fn opamp_buffer_project_yaml() -> &'static str {
        "project:
  name: opamp_observation_preset_test
  version: 0.1.0
libraries:
  - libs/generic/analog
board:
  components:
    VCC:
      model: generic.analog.dc_voltage_source
      pins: { P: vcc_5v, N: gnd }
      spice: { primitive: dc_voltage_source, dc_v: 5.0 }
    VIN:
      model: generic.analog.pulse_voltage_source
      pins: { P: input, N: gnd }
      spice:
        primitive: pulse_voltage_source
        pulse:
          initial_v: 0.5
          pulsed_v: 2.5
          delay_us: 10.0
          rise_us: 1.0
          fall_us: 1.0
          width_us: 40.0
          period_us: 80.0
    XU1:
      model: generic.analog.ideal_opamp
      pins: { INP: input, INN: output, VCC: vcc_5v, VEE: gnd, OUT: output }
    RLOAD:
      model: generic.analog.resistor
      pins: { A: output, B: gnd }
      spice: { primitive: resistor, value_ohm: 10000.0 }
  nets:
    vcc_5v: { kind: power, nominal_voltage: 5.0, powered: true }
    input: { kind: digital_or_analog }
    output: { kind: digital_or_analog }
    gnd: { kind: ground }
"
    }

    fn comparator_project_yaml() -> &'static str {
        "project:
  name: comparator_observation_preset_test
  version: 0.1.0
libraries:
  - libs/generic/analog
board:
  components:
    VCC:
      model: generic.analog.dc_voltage_source
      pins: { P: vcc_5v, N: gnd }
      spice: { primitive: dc_voltage_source, dc_v: 5.0 }
    VIN:
      model: generic.analog.pulse_voltage_source
      pins: { P: input, N: gnd }
      spice:
        primitive: pulse_voltage_source
        pulse:
          initial_v: 0.5
          pulsed_v: 2.5
          delay_us: 10.0
          rise_us: 1.0
          fall_us: 1.0
          width_us: 40.0
          period_us: 80.0
    VREF:
      model: generic.analog.dc_voltage_source
      pins: { P: reference, N: gnd }
      spice: { primitive: dc_voltage_source, dc_v: 1.2 }
    XU1:
      model: generic.analog.ideal_comparator
      pins: { INP: input, INN: reference, VCC: vcc_5v, VEE: gnd, OUT: output }
    RLOAD:
      model: generic.analog.resistor
      pins: { A: output, B: gnd }
      spice: { primitive: resistor, value_ohm: 10000.0 }
  nets:
    vcc_5v: { kind: power, nominal_voltage: 5.0, powered: true }
    input: { kind: digital_or_analog }
    reference: { kind: digital_or_analog, nominal_voltage: 1.2 }
    output: { kind: digital_or_analog }
    gnd: { kind: ground }
"
    }

    #[test]
    fn observation_preset_creates_generated_spice_setup_for_component_pins() {
        let result =
            create_component_observation_preset(ap2112_project_yaml(), Path::new("."), "UREG")
                .unwrap();
        let project: crate::board_ir::BoardProject =
            serde_yaml_ng::from_str(&result.project_yaml).unwrap();
        let scenario = project
            .scenarios
            .iter()
            .find(|scenario| scenario.name == "ureg_observation")
            .unwrap();
        let analog = scenario.analog.as_ref().unwrap();
        let generated = analog.generated.as_ref().unwrap();

        assert_eq!(result.probe_count, 3);
        assert_eq!(generated.ground_net, "gnd");
        assert!(
            generated
                .components
                .iter()
                .any(|component| component == "UREG")
        );
        assert_eq!(
            analog
                .probes
                .iter()
                .map(|probe| probe.name.as_str())
                .collect::<Vec<_>>(),
            vec!["v_ureg_vin", "v_ureg_en", "v_ureg_vout"]
        );
        assert_eq!(analog.model_files.len(), 1);
        assert_eq!(
            analog.model_files[0].path,
            "models/spice/generic/analog_behavioral.lib"
        );
        assert!(analog.model_files[0].sha256.is_some());
        assert_eq!(
            analog
                .assertions
                .iter()
                .map(|assertion| assertion.name.as_str())
                .collect::<Vec<_>>(),
            vec!["v_ureg_vout_min_voltage", "v_ureg_vout_max_voltage"]
        );
    }

    #[test]
    fn observation_preset_adds_reset_supervisor_threshold_checks_for_pulsed_rail() {
        let result =
            create_component_observation_preset(tlv803_project_yaml(), Path::new("."), "URESET")
                .unwrap();
        let project: crate::board_ir::BoardProject =
            serde_yaml_ng::from_str(&result.project_yaml).unwrap();
        let scenario = project
            .scenarios
            .iter()
            .find(|scenario| scenario.name == "ureset_observation")
            .unwrap();
        let analog = scenario.analog.as_ref().unwrap();

        assert_eq!(
            analog
                .probes
                .iter()
                .map(|probe| probe.name.as_str())
                .collect::<Vec<_>>(),
            vec!["v_ureset_vdd", "v_ureset_reset"]
        );
        assert_eq!(
            analog
                .assertions
                .iter()
                .map(|assertion| {
                    (
                        assertion.name.as_str(),
                        assertion.probe.as_str(),
                        assertion.relation.clone(),
                        assertion.at_us,
                        assertion.threshold_v,
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                (
                    "v_ureset_reset_asserted_low",
                    "v_ureset_reset",
                    crate::board_ir::AnalogRelation::Below,
                    Some(5.0),
                    Some(0.33)
                ),
                (
                    "v_ureset_reset_released_high",
                    "v_ureset_reset",
                    crate::board_ir::AnalogRelation::Above,
                    Some(21.0),
                    Some(2.9699999999999998)
                )
            ]
        );
    }

    #[test]
    fn observation_preset_adds_load_switch_output_voltage_checks() {
        let result =
            create_component_observation_preset(tps22918_project_yaml(), Path::new("."), "USW")
                .unwrap();
        let project: crate::board_ir::BoardProject =
            serde_yaml_ng::from_str(&result.project_yaml).unwrap();
        let scenario = project
            .scenarios
            .iter()
            .find(|scenario| scenario.name == "usw_observation")
            .unwrap();
        let analog = scenario.analog.as_ref().unwrap();

        assert_eq!(
            analog
                .probes
                .iter()
                .map(|probe| probe.name.as_str())
                .collect::<Vec<_>>(),
            vec!["v_usw_vin", "v_usw_vout", "v_usw_on"]
        );
        assert_eq!(
            analog
                .assertions
                .iter()
                .map(|assertion| {
                    (
                        assertion.name.as_str(),
                        assertion.probe.as_str(),
                        assertion.aggregation.clone(),
                        assertion.relation.clone(),
                        assertion.threshold_v,
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                (
                    "v_usw_vout_enabled_min_voltage",
                    "v_usw_vout",
                    crate::board_ir::AnalogAggregation::Min,
                    crate::board_ir::AnalogRelation::Above,
                    Some(4.9)
                ),
                (
                    "v_usw_vout_enabled_max_voltage",
                    "v_usw_vout",
                    crate::board_ir::AnalogAggregation::Max,
                    crate::board_ir::AnalogRelation::Below,
                    Some(5.1)
                )
            ]
        );
    }

    #[test]
    fn observation_preset_adds_battery_charger_voltage_ceiling_check() {
        let result =
            create_component_observation_preset(mcp73831_project_yaml(), Path::new("."), "UCHG")
                .unwrap();
        let project: crate::board_ir::BoardProject =
            serde_yaml_ng::from_str(&result.project_yaml).unwrap();
        let scenario = project
            .scenarios
            .iter()
            .find(|scenario| scenario.name == "uchg_observation")
            .unwrap();
        let analog = scenario.analog.as_ref().unwrap();

        assert_eq!(
            analog
                .probes
                .iter()
                .map(|probe| probe.name.as_str())
                .collect::<Vec<_>>(),
            vec!["v_uchg_vdd", "v_uchg_vbat", "v_uchg_prog"]
        );
        assert_eq!(
            analog
                .assertions
                .iter()
                .map(|assertion| {
                    (
                        assertion.name.as_str(),
                        assertion.probe.as_str(),
                        assertion.aggregation.clone(),
                        assertion.relation.clone(),
                        assertion.threshold_v,
                    )
                })
                .collect::<Vec<_>>(),
            vec![(
                "v_uchg_vbat_regulation_ceiling",
                "v_uchg_vbat",
                crate::board_ir::AnalogAggregation::Max,
                crate::board_ir::AnalogRelation::Below,
                Some(4.232)
            )]
        );
    }

    #[test]
    fn observation_preset_adds_power_path_charger_voltage_checks() {
        let result =
            create_component_observation_preset(bq24075_project_yaml(), Path::new("."), "UCHG")
                .unwrap();
        let project: crate::board_ir::BoardProject =
            serde_yaml_ng::from_str(&result.project_yaml).unwrap();
        let scenario = project
            .scenarios
            .iter()
            .find(|scenario| scenario.name == "uchg_observation")
            .unwrap();
        let analog = scenario.analog.as_ref().unwrap();

        assert_eq!(
            analog
                .probes
                .iter()
                .map(|probe| probe.name.as_str())
                .collect::<Vec<_>>(),
            vec!["v_uchg_in", "v_uchg_bat", "v_uchg_out", "v_uchg_iset"]
        );
        assert_eq!(
            analog
                .assertions
                .iter()
                .map(|assertion| {
                    (
                        assertion.name.as_str(),
                        assertion.probe.as_str(),
                        assertion.threshold_v,
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                ("v_uchg_bat_regulation_ceiling", "v_uchg_bat", Some(4.23)),
                ("v_uchg_out_power_path_ceiling", "v_uchg_out", Some(5.6))
            ]
        );
    }

    #[test]
    fn observation_preset_adds_op_amp_follower_tracking_checks_for_pulsed_input() {
        let result =
            create_component_observation_preset(opamp_buffer_project_yaml(), Path::new("."), "XU1")
                .unwrap();
        let project: crate::board_ir::BoardProject =
            serde_yaml_ng::from_str(&result.project_yaml).unwrap();
        let scenario = project
            .scenarios
            .iter()
            .find(|scenario| scenario.name == "xu1_observation")
            .unwrap();
        let analog = scenario.analog.as_ref().unwrap();

        assert_eq!(
            analog
                .probes
                .iter()
                .map(|probe| probe.name.as_str())
                .collect::<Vec<_>>(),
            vec!["v_xu1_inp", "v_xu1_inn", "v_xu1_vcc"]
        );
        assert_eq!(
            analog
                .assertions
                .iter()
                .map(|assertion| {
                    (
                        assertion.name.as_str(),
                        assertion.probe.as_str(),
                        assertion.relation.clone(),
                        assertion.at_us,
                        assertion.threshold_v,
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                (
                    "v_xu1_inn_tracks_input_low_above",
                    "v_xu1_inn",
                    crate::board_ir::AnalogRelation::Above,
                    Some(5.0),
                    Some(0.45)
                ),
                (
                    "v_xu1_inn_tracks_input_low_below",
                    "v_xu1_inn",
                    crate::board_ir::AnalogRelation::Below,
                    Some(5.0),
                    Some(0.55)
                ),
                (
                    "v_xu1_inn_tracks_input_high_above",
                    "v_xu1_inn",
                    crate::board_ir::AnalogRelation::Above,
                    Some(31.0),
                    Some(2.45)
                ),
                (
                    "v_xu1_inn_tracks_input_high_below",
                    "v_xu1_inn",
                    crate::board_ir::AnalogRelation::Below,
                    Some(31.0),
                    Some(2.55)
                )
            ]
        );
    }

    #[test]
    fn observation_preset_adds_comparator_output_state_checks_for_pulsed_input() {
        let result =
            create_component_observation_preset(comparator_project_yaml(), Path::new("."), "XU1")
                .unwrap();
        let project: crate::board_ir::BoardProject =
            serde_yaml_ng::from_str(&result.project_yaml).unwrap();
        let scenario = project
            .scenarios
            .iter()
            .find(|scenario| scenario.name == "xu1_observation")
            .unwrap();
        let analog = scenario.analog.as_ref().unwrap();

        assert_eq!(
            analog
                .probes
                .iter()
                .map(|probe| probe.name.as_str())
                .collect::<Vec<_>>(),
            vec!["v_xu1_inp", "v_xu1_inn", "v_xu1_vcc", "v_xu1_out"]
        );
        assert_eq!(
            analog
                .assertions
                .iter()
                .map(|assertion| {
                    (
                        assertion.name.as_str(),
                        assertion.probe.as_str(),
                        assertion.relation.clone(),
                        assertion.at_us,
                        assertion.threshold_v,
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                (
                    "v_xu1_out_positive_input_low_state",
                    "v_xu1_out",
                    crate::board_ir::AnalogRelation::Below,
                    Some(5.0),
                    Some(0.5)
                ),
                (
                    "v_xu1_out_positive_input_high_state",
                    "v_xu1_out",
                    crate::board_ir::AnalogRelation::Above,
                    Some(31.0),
                    Some(4.5)
                )
            ]
        );
    }
}
