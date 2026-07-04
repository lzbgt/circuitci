use super::CircuitCiApp;
use super::simulation_run_setup_controls::{
    initialize_noise_source_default, noise_source_combo, pac_mode_combo, periodic_mode_combo,
};
use super::sketch::ProjectSnapshot;
use eframe::egui;

pub(super) fn render_planned_run_setup_controls(
    app: &mut CircuitCiApp,
    ui: &mut egui::Ui,
    snapshot: &ProjectSnapshot,
) -> bool {
    match app.analog_run_setup_kind.as_str() {
        "pss" => {
            render_pss_controls(app, ui, snapshot);
            true
        }
        "phase_noise" => {
            render_phase_noise_controls(app, ui, snapshot);
            true
        }
        "pac" => {
            render_periodic_ac_controls(app, ui, snapshot);
            true
        }
        _ => false,
    }
}

fn render_pss_controls(app: &mut CircuitCiApp, ui: &mut egui::Ui, snapshot: &ProjectSnapshot) {
    initialize_noise_source_default(snapshot, &mut app.analog_pss_drive_source);
    ui.label("Mode");
    periodic_mode_combo(ui, "analog_pss_mode", &mut app.analog_pss_mode);
    ui.end_row();

    ui.label("Frequency guess");
    ui.add(
        egui::DragValue::new(&mut app.analog_pss_frequency_guess_hz)
            .speed(100.0)
            .range(1.0e-9..=1.0e15)
            .suffix(" Hz"),
    );
    ui.end_row();

    ui.label("Stabilization");
    ui.add(
        egui::DragValue::new(&mut app.analog_pss_stabilization_time_us)
            .speed(1.0)
            .range(0.001..=1_000_000.0)
            .suffix(" us"),
    );
    ui.end_row();

    ui.label("Periods");
    ui.add(
        egui::DragValue::new(&mut app.analog_pss_periods)
            .speed(1.0)
            .range(1..=4096),
    );
    ui.end_row();

    if app.analog_pss_mode == "driven" {
        ui.label("Drive source");
        noise_source_combo(
            ui,
            "analog_pss_drive_source",
            &mut app.analog_pss_drive_source,
            snapshot,
        );
        ui.end_row();
    }
}

fn render_phase_noise_controls(
    app: &mut CircuitCiApp,
    ui: &mut egui::Ui,
    snapshot: &ProjectSnapshot,
) {
    initialize_noise_source_default(snapshot, &mut app.analog_phase_noise_drive_source);
    ui.label("Mode");
    periodic_mode_combo(
        ui,
        "analog_phase_noise_mode",
        &mut app.analog_phase_noise_mode,
    );
    ui.end_row();

    ui.label("Carrier");
    ui.add(
        egui::DragValue::new(&mut app.analog_phase_noise_carrier_frequency_hz)
            .speed(100.0)
            .range(1.0e-9..=1.0e15)
            .suffix(" Hz"),
    );
    ui.end_row();

    ui.label("Offset start");
    ui.add(
        egui::DragValue::new(&mut app.analog_phase_noise_offset_start_hz)
            .speed(10.0)
            .range(1.0e-9..=1.0e15)
            .suffix(" Hz"),
    );
    ui.end_row();

    ui.label("Offset stop");
    ui.add(
        egui::DragValue::new(&mut app.analog_phase_noise_offset_stop_hz)
            .speed(100.0)
            .range(1.0e-9..=1.0e15)
            .suffix(" Hz"),
    );
    ui.end_row();

    ui.label("Points/decade");
    ui.add(
        egui::DragValue::new(&mut app.analog_points_per_decade)
            .speed(1.0)
            .range(1..=1000),
    );
    ui.end_row();

    if app.analog_phase_noise_mode == "driven" {
        ui.label("Drive source");
        noise_source_combo(
            ui,
            "analog_phase_noise_drive_source",
            &mut app.analog_phase_noise_drive_source,
            snapshot,
        );
        ui.end_row();
    }
}

fn render_periodic_ac_controls(
    app: &mut CircuitCiApp,
    ui: &mut egui::Ui,
    snapshot: &ProjectSnapshot,
) {
    initialize_noise_source_default(snapshot, &mut app.analog_pac_input_source);
    initialize_noise_source_default(snapshot, &mut app.analog_pac_drive_source);
    ui.label("Mode");
    pac_mode_combo(ui, &mut app.analog_pac_mode);
    ui.end_row();

    ui.label("Carrier");
    ui.add(
        egui::DragValue::new(&mut app.analog_pac_carrier_frequency_hz)
            .speed(100.0)
            .range(1.0e-9..=1.0e15)
            .suffix(" Hz"),
    );
    ui.end_row();

    ui.label("Start frequency");
    ui.add(
        egui::DragValue::new(&mut app.analog_start_frequency_hz)
            .speed(10.0)
            .range(1.0e-9..=1.0e15)
            .suffix(" Hz"),
    );
    ui.end_row();

    ui.label("Stop frequency");
    ui.add(
        egui::DragValue::new(&mut app.analog_stop_frequency_hz)
            .speed(100.0)
            .range(1.0e-9..=1.0e15)
            .suffix(" Hz"),
    );
    ui.end_row();

    ui.label("Points/decade");
    ui.add(
        egui::DragValue::new(&mut app.analog_points_per_decade)
            .speed(1.0)
            .range(1..=1000),
    );
    ui.end_row();

    ui.label("Input source");
    noise_source_combo(
        ui,
        "analog_pac_input_source",
        &mut app.analog_pac_input_source,
        snapshot,
    );
    ui.end_row();

    ui.label("Drive source");
    noise_source_combo(
        ui,
        "analog_pac_drive_source",
        &mut app.analog_pac_drive_source,
        snapshot,
    );
    ui.end_row();

    ui.label("Sidebands");
    ui.add(
        egui::DragValue::new(&mut app.analog_pac_sidebands)
            .speed(1.0)
            .range(0..=1024),
    );
    ui.end_row();
}
