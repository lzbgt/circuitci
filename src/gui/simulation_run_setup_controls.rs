use super::analog_generated::AnalogGeneratedScenario;
use super::sketch::ProjectSnapshot;
use super::sketch_spice::{SketchComponentSpice, SketchSpiceKind};
use eframe::egui;

pub(super) fn initialize_noise_source_default(snapshot: &ProjectSnapshot, selected: &mut String) {
    if !selected.is_empty()
        && snapshot
            .components_detail
            .iter()
            .any(|component| component.id == *selected)
    {
        return;
    }
    if let Some(component) = snapshot
        .components_detail
        .iter()
        .find(|component| component.spice.as_ref().is_some_and(is_noise_source_spice))
        .or_else(|| snapshot.components_detail.first())
    {
        *selected = component.id.clone();
    }
}

pub(super) fn noise_source_combo(
    ui: &mut egui::Ui,
    id: &str,
    selected: &mut String,
    snapshot: &ProjectSnapshot,
) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(if selected.is_empty() {
            "select source"
        } else {
            selected.as_str()
        })
        .show_ui(ui, |ui| {
            for component in snapshot
                .components_detail
                .iter()
                .filter(|component| component.spice.as_ref().is_some_and(is_noise_source_spice))
                .chain(snapshot.components_detail.iter().filter(|component| {
                    component
                        .spice
                        .as_ref()
                        .is_none_or(|spice| !is_noise_source_spice(spice))
                }))
            {
                ui.selectable_value(selected, component.id.clone(), &component.id);
            }
        });
}

pub(super) fn pole_zero_mode_combo(ui: &mut egui::Ui, selected: &mut String) {
    if !matches!(selected.as_str(), "poles" | "zeros" | "poles_and_zeros") {
        *selected = "poles_and_zeros".to_string();
    }
    let selected_label = match selected.as_str() {
        "poles" => "Poles",
        "zeros" => "Zeros",
        "poles_and_zeros" => "Poles and zeros",
        _ => "Poles and zeros",
    };
    egui::ComboBox::from_id_salt("analog_pole_zero_mode")
        .selected_text(selected_label)
        .show_ui(ui, |ui| {
            ui.selectable_value(selected, "poles".to_string(), "Poles");
            ui.selectable_value(selected, "zeros".to_string(), "Zeros");
            ui.selectable_value(selected, "poles_and_zeros".to_string(), "Poles and zeros");
        });
}

pub(super) fn sensitivity_mode_combo(ui: &mut egui::Ui, selected: &mut String) {
    if !matches!(selected.as_str(), "dc" | "ac") {
        *selected = "dc".to_string();
    }
    let selected_label = match selected.as_str() {
        "ac" => "AC",
        _ => "DC",
    };
    egui::ComboBox::from_id_salt("analog_sensitivity_mode")
        .selected_text(selected_label)
        .show_ui(ui, |ui| {
            ui.selectable_value(selected, "dc".to_string(), "DC");
            ui.selectable_value(selected, "ac".to_string(), "AC");
        });
}

pub(super) fn distortion_mode_combo(ui: &mut egui::Ui, selected: &mut String) {
    if !matches!(selected.as_str(), "harmonic" | "intermodulation") {
        *selected = "harmonic".to_string();
    }
    let selected_label = match selected.as_str() {
        "intermodulation" => "Intermodulation",
        _ => "Harmonic",
    };
    egui::ComboBox::from_id_salt("analog_distortion_mode")
        .selected_text(selected_label)
        .show_ui(ui, |ui| {
            ui.selectable_value(selected, "harmonic".to_string(), "Harmonic");
            ui.selectable_value(selected, "intermodulation".to_string(), "Intermodulation");
        });
}

pub(super) fn initialize_sensitivity_filters_default(
    snapshot: &ProjectSnapshot,
    selected: &mut String,
) {
    if !selected.trim().is_empty() {
        return;
    }
    let filters = snapshot
        .components_detail
        .iter()
        .filter(|component| {
            component.spice.as_ref().is_some_and(|spice| {
                matches!(
                    spice.kind,
                    SketchSpiceKind::Resistor
                        | SketchSpiceKind::Capacitor
                        | SketchSpiceKind::Inductor
                )
            })
        })
        .map(|component| component.id.as_str())
        .collect::<Vec<_>>();
    if !filters.is_empty() {
        *selected = filters.join(", ");
    }
}

pub(super) fn sensitivity_filters_from_text(text: &str) -> Vec<String> {
    text.split(',')
        .map(str::trim)
        .filter(|filter| !filter.is_empty())
        .map(ToString::to_string)
        .collect()
}

pub(super) fn generated_noise_source_combo(
    ui: &mut egui::Ui,
    scenario: &AnalogGeneratedScenario,
    selected: &mut String,
) {
    if selected.is_empty()
        && let Some(input_source) = &scenario.noise_input_source
    {
        *selected = input_source.clone();
    }
    egui::ComboBox::from_id_salt("analog_generated_noise_input_source")
        .selected_text(if selected.is_empty() {
            "select source"
        } else {
            selected.as_str()
        })
        .show_ui(ui, |ui| {
            for component in &scenario.board_components {
                ui.selectable_value(selected, component.id.clone(), &component.id);
            }
        });
}

fn is_noise_source_spice(spice: &SketchComponentSpice) -> bool {
    matches!(
        spice.kind,
        SketchSpiceKind::DcVoltageSource
            | SketchSpiceKind::PulseVoltageSource
            | SketchSpiceKind::DcCurrentSource
            | SketchSpiceKind::PulseCurrentSource
    )
}
