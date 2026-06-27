use super::analog::{AnalogAssertionUiStatus, AnalogScenarioChoice};
use super::analog_generated::AnalogGeneratedScenario;
use super::analog_models::AnalogModelFileScenario;
use super::analog_stimulus::{AnalogStimulusChoice, AnalogStimulusKind};
use super::sketch::ProjectSnapshot;
use eframe::egui;

pub(super) fn initialize_analog_net_defaults(
    snapshot: &ProjectSnapshot,
    ground_net: &mut String,
    probe_net: &mut String,
) {
    if ground_net.is_empty()
        && let Some(net) = snapshot.nets_detail.iter().find(|net| net.kind == "ground")
    {
        *ground_net = net.id.clone();
    }
    if probe_net.is_empty()
        && let Some(net) = snapshot
            .nets_detail
            .iter()
            .find(|net| net.kind != "ground")
            .or_else(|| snapshot.nets_detail.first())
    {
        *probe_net = net.id.clone();
    }
}

pub(super) fn net_combo(
    ui: &mut egui::Ui,
    id: &str,
    selected: &mut String,
    snapshot: &ProjectSnapshot,
) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(if selected.is_empty() {
            "select net"
        } else {
            selected.as_str()
        })
        .show_ui(ui, |ui| {
            for net in &snapshot.nets_detail {
                ui.selectable_value(selected, net.id.clone(), &net.id);
            }
        });
}

pub(super) fn initialize_analog_assertion_defaults(
    choices: &[AnalogScenarioChoice],
    scenario_name: &mut String,
    probe_name: &mut String,
    end_us: &mut f64,
) {
    let scenario_missing = !choices
        .iter()
        .any(|scenario| scenario.name == *scenario_name);
    if (scenario_name.is_empty() || scenario_missing)
        && let Some(scenario) = choices.first()
    {
        *scenario_name = scenario.name.clone();
        *end_us = scenario.stop_time_us;
    }
    let selected_scenario = choices
        .iter()
        .find(|scenario| scenario.name == *scenario_name)
        .or_else(|| choices.first());
    if let Some(scenario) = selected_scenario {
        let probe_missing = !scenario
            .probes
            .iter()
            .any(|probe| probe.name == *probe_name);
        if (probe_name.is_empty() || probe_missing)
            && let Some(probe) = scenario.probes.first()
        {
            *probe_name = probe.name.clone();
        }
        if *end_us <= 0.0 || *end_us > scenario.stop_time_us {
            *end_us = scenario.stop_time_us;
        }
    }
}

pub(super) fn initialize_model_file_scenario_default(
    scenarios: &[AnalogModelFileScenario],
    scenario_name: &mut String,
) {
    let scenario_missing = !scenarios
        .iter()
        .any(|scenario| scenario.name == *scenario_name);
    if (scenario_name.is_empty() || scenario_missing)
        && let Some(scenario) = scenarios.first()
    {
        *scenario_name = scenario.name.clone();
    }
}

pub(super) fn initialize_generated_component_defaults(
    scenarios: &[AnalogGeneratedScenario],
    scenario_name: &mut String,
    component_id: &mut String,
) {
    let scenario_missing = selected_generated_scenario(scenarios, scenario_name).is_none();
    if (scenario_name.is_empty() || scenario_missing)
        && let Some(scenario) = scenarios.first()
    {
        *scenario_name = scenario.name.clone();
    }
    let Some(scenario) = selected_generated_scenario(scenarios, scenario_name) else {
        return;
    };
    let component_missing = !scenario
        .board_components
        .iter()
        .any(|component| component.id == *component_id);
    if (component_id.is_empty() || component_missing)
        && let Some(component) = scenario
            .board_components
            .iter()
            .find(|component| !component.included)
            .or_else(|| scenario.board_components.first())
    {
        *component_id = component.id.clone();
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn initialize_generated_settings_defaults(
    scenarios: &[AnalogGeneratedScenario],
    scenario_name: &mut String,
    ground_net: &mut String,
    stop_time_us: &mut f64,
    max_step_us: &mut f64,
    node_net: &mut String,
    node_name: &mut String,
) {
    let scenario_missing = selected_generated_scenario(scenarios, scenario_name).is_none();
    if (scenario_name.is_empty() || scenario_missing)
        && let Some(scenario) = scenarios.first()
    {
        *scenario_name = scenario.name.clone();
        load_generated_settings_values(
            scenario,
            ground_net,
            stop_time_us,
            max_step_us,
            node_net,
            node_name,
        );
        return;
    }
    let Some(scenario) = selected_generated_scenario(scenarios, scenario_name) else {
        return;
    };
    if ground_net.is_empty()
        || !scenario
            .board_nets
            .iter()
            .any(|net| net.id == *ground_net && net.kind == "ground")
    {
        *ground_net = scenario.ground_net.clone();
    }
    if *stop_time_us <= 0.0 || *max_step_us <= 0.0 || *max_step_us > *stop_time_us {
        *stop_time_us = scenario.stop_time_us;
        *max_step_us = scenario.max_step_us;
    }
    let node_net_missing = !scenario.board_nets.iter().any(|net| net.id == *node_net);
    if node_net.is_empty() || node_net_missing {
        load_generated_node_binding_values(scenario, node_net, node_name);
    }
}

pub(super) fn load_generated_settings_values(
    scenario: &AnalogGeneratedScenario,
    ground_net: &mut String,
    stop_time_us: &mut f64,
    max_step_us: &mut f64,
    node_net: &mut String,
    node_name: &mut String,
) {
    *ground_net = scenario.ground_net.clone();
    *stop_time_us = scenario.stop_time_us;
    *max_step_us = scenario.max_step_us;
    load_generated_node_binding_values(scenario, node_net, node_name);
}

pub(super) fn load_generated_node_binding_values(
    scenario: &AnalogGeneratedScenario,
    node_net: &mut String,
    node_name: &mut String,
) {
    if let Some(binding) = scenario
        .node_bindings
        .iter()
        .find(|binding| binding.net == scenario.ground_net)
        .or_else(|| scenario.node_bindings.first())
    {
        *node_net = binding.net.clone();
        *node_name = binding.node.clone();
    } else if let Some(net) = scenario.board_nets.first() {
        *node_net = net.id.clone();
        *node_name = if net.id == scenario.ground_net {
            "0".to_string()
        } else {
            net.id.clone()
        };
    }
}

pub(super) fn selected_generated_scenario<'a>(
    scenarios: &'a [AnalogGeneratedScenario],
    scenario_name: &str,
) -> Option<&'a AnalogGeneratedScenario> {
    scenarios
        .iter()
        .find(|scenario| scenario.name == scenario_name)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn initialize_analog_stimulus_defaults(
    choices: &[AnalogStimulusChoice],
    scenario_name: &mut String,
    component_id: &mut String,
    dc_value: &mut f64,
    initial_value: &mut f64,
    pulsed_value: &mut f64,
    delay_us: &mut f64,
    rise_us: &mut f64,
    fall_us: &mut f64,
    width_us: &mut f64,
    period_us: &mut f64,
) {
    let selected_missing =
        selected_analog_stimulus_choice(choices, scenario_name, component_id).is_none();
    if (scenario_name.is_empty() || component_id.is_empty() || selected_missing)
        && let Some(choice) = choices.first()
    {
        *scenario_name = choice.scenario_name.clone();
        *component_id = choice.component_id.clone();
        load_analog_stimulus_choice_values(
            choice,
            dc_value,
            initial_value,
            pulsed_value,
            delay_us,
            rise_us,
            fall_us,
            width_us,
            period_us,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn load_selected_analog_stimulus_values(
    choices: &[AnalogStimulusChoice],
    scenario_name: &str,
    component_id: &str,
    dc_value: &mut f64,
    initial_value: &mut f64,
    pulsed_value: &mut f64,
    delay_us: &mut f64,
    rise_us: &mut f64,
    fall_us: &mut f64,
    width_us: &mut f64,
    period_us: &mut f64,
) {
    if let Some(choice) = selected_analog_stimulus_choice(choices, scenario_name, component_id) {
        load_analog_stimulus_choice_values(
            choice,
            dc_value,
            initial_value,
            pulsed_value,
            delay_us,
            rise_us,
            fall_us,
            width_us,
            period_us,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn load_analog_stimulus_choice_values(
    choice: &AnalogStimulusChoice,
    dc_value: &mut f64,
    initial_value: &mut f64,
    pulsed_value: &mut f64,
    delay_us: &mut f64,
    rise_us: &mut f64,
    fall_us: &mut f64,
    width_us: &mut f64,
    period_us: &mut f64,
) {
    *dc_value = choice.dc_value;
    *initial_value = choice.pulse.initial;
    *pulsed_value = choice.pulse.pulsed;
    *delay_us = choice.pulse.delay_us;
    *rise_us = choice.pulse.rise_us;
    *fall_us = choice.pulse.fall_us;
    *width_us = choice.pulse.width_us;
    *period_us = choice.pulse.period_us;
}

pub(super) fn selected_analog_stimulus_choice<'a>(
    choices: &'a [AnalogStimulusChoice],
    scenario_name: &str,
    component_id: &str,
) -> Option<&'a AnalogStimulusChoice> {
    choices
        .iter()
        .find(|choice| choice.scenario_name == scenario_name && choice.component_id == component_id)
}

pub(super) fn analog_scenario_combo(
    ui: &mut egui::Ui,
    id: &str,
    selected: &mut String,
    choices: &[AnalogScenarioChoice],
) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(if selected.is_empty() {
            "select run setup"
        } else {
            selected.as_str()
        })
        .show_ui(ui, |ui| {
            for scenario in choices {
                ui.selectable_value(selected, scenario.name.clone(), &scenario.name);
            }
        });
}

pub(super) fn analog_model_scenario_combo(
    ui: &mut egui::Ui,
    id: &str,
    selected: &mut String,
    scenarios: &[AnalogModelFileScenario],
) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(if selected.is_empty() {
            "select run setup"
        } else {
            selected.as_str()
        })
        .show_ui(ui, |ui| {
            for scenario in scenarios {
                ui.selectable_value(selected, scenario.name.clone(), &scenario.name);
            }
        });
}

pub(super) fn generated_scenario_combo(
    ui: &mut egui::Ui,
    selected: &mut String,
    scenarios: &[AnalogGeneratedScenario],
) {
    egui::ComboBox::from_id_salt("analog_generated_scenario")
        .selected_text(if selected.is_empty() {
            "select run setup"
        } else {
            selected.as_str()
        })
        .show_ui(ui, |ui| {
            for scenario in scenarios {
                ui.selectable_value(selected, scenario.name.clone(), &scenario.name);
            }
        });
}

pub(super) fn generated_component_combo(
    ui: &mut egui::Ui,
    scenario: &AnalogGeneratedScenario,
    selected: &mut String,
) {
    egui::ComboBox::from_id_salt("analog_generated_component")
        .selected_text(if selected.is_empty() {
            "select component".to_string()
        } else {
            selected.clone()
        })
        .show_ui(ui, |ui| {
            for component in &scenario.board_components {
                ui.selectable_value(selected, component.id.clone(), component.label());
            }
        });
}

pub(super) fn generated_net_combo(
    ui: &mut egui::Ui,
    id: &str,
    scenario: &AnalogGeneratedScenario,
    selected: &mut String,
) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(if selected.is_empty() {
            "select net".to_string()
        } else {
            selected.clone()
        })
        .show_ui(ui, |ui| {
            for net in &scenario.board_nets {
                ui.selectable_value(
                    selected,
                    net.id.clone(),
                    format!("{} ({})", net.id, net.kind),
                );
            }
        });
}

pub(super) fn analog_stimulus_combo(
    ui: &mut egui::Ui,
    scenario_name: &mut String,
    component_id: &mut String,
    choices: &[AnalogStimulusChoice],
) {
    let label = selected_analog_stimulus_choice(choices, scenario_name, component_id)
        .map(AnalogStimulusChoice::label)
        .unwrap_or_else(|| "select source".to_string());
    egui::ComboBox::from_id_salt("analog_stimulus_source")
        .selected_text(label)
        .show_ui(ui, |ui| {
            for choice in choices {
                if ui
                    .selectable_label(
                        choice.scenario_name == *scenario_name
                            && choice.component_id == *component_id,
                        choice.label(),
                    )
                    .clicked()
                {
                    *scenario_name = choice.scenario_name.clone();
                    *component_id = choice.component_id.clone();
                }
            }
        });
}

pub(super) fn stimulus_value_speed(kind: AnalogStimulusKind) -> f64 {
    match kind {
        AnalogStimulusKind::DcVoltage | AnalogStimulusKind::PulseVoltage => 0.1,
        AnalogStimulusKind::DcCurrent | AnalogStimulusKind::PulseCurrent => 0.001,
    }
}

pub(super) fn analog_probe_combo(
    ui: &mut egui::Ui,
    id: &str,
    selected: &mut String,
    scenario: Option<&AnalogScenarioChoice>,
) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(if selected.is_empty() {
            "select probe"
        } else {
            selected.as_str()
        })
        .show_ui(ui, |ui| {
            if let Some(scenario) = scenario {
                for probe in &scenario.probes {
                    ui.selectable_value(
                        selected,
                        probe.name.clone(),
                        format!("{} ({})", probe.name, probe.quantity),
                    );
                }
            }
        });
}

pub(super) fn assertion_status_color(status: AnalogAssertionUiStatus) -> egui::Color32 {
    match status {
        AnalogAssertionUiStatus::Unknown => egui::Color32::from_rgb(230, 190, 90),
        AnalogAssertionUiStatus::Pass => egui::Color32::from_rgb(86, 190, 112),
        AnalogAssertionUiStatus::Fail => egui::Color32::from_rgb(232, 83, 83),
    }
}

pub(super) fn string_combo(ui: &mut egui::Ui, id: &str, selected: &mut String, values: &[&str]) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(selected.as_str())
        .show_ui(ui, |ui| {
            for value in values {
                ui.selectable_value(selected, (*value).to_string(), *value);
            }
        });
}
