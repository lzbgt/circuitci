use super::CircuitCiApp;
use super::analog::{
    AnalogAcScenarioDraft, AnalogDcScenarioDraft, AnalogDcSweepScenarioDraft,
    AnalogDistortionScenarioDraft, AnalogFourierScenarioDraft, AnalogHarmonicBalanceScenarioDraft,
    AnalogMeasureScenarioDraft, AnalogNoiseScenarioDraft, AnalogPoleZeroScenarioDraft,
    AnalogSParameterScenarioDraft, AnalogScenarioDraft, AnalogSensitivityScenarioDraft,
    AnalogTransferFunctionScenarioDraft, append_analog_ac_scenario_with_project_path,
    append_analog_dc_scenario_with_project_path, append_analog_dc_sweep_scenario_with_project_path,
    append_analog_distortion_scenario_with_project_path,
    append_analog_fourier_scenario_with_project_path,
    append_analog_harmonic_balance_scenario_with_project_path,
    append_analog_measure_scenario_with_project_path,
    append_analog_noise_scenario_with_project_path,
    append_analog_pole_zero_scenario_with_project_path,
    append_analog_sensitivity_scenario_with_project_path,
    append_analog_sparameter_scenario_with_project_path,
    append_analog_transfer_function_scenario_with_project_path,
    append_analog_transient_scenario_with_project_path,
};
use super::simulation_run_setup_controls::sensitivity_filters_from_text;
use std::path::Path;

impl CircuitCiApp {
    pub(super) fn apply_add_analog_scenario(&mut self) {
        if self.analog_run_setup_kind == "ac" {
            let draft = AnalogAcScenarioDraft {
                name: self.analog_scenario_name.clone(),
                ground_net: self.analog_ground_net.clone(),
                probe_net: self.analog_probe_net.clone(),
                probe_name: self.analog_probe_name.clone(),
                start_frequency_hz: self.analog_start_frequency_hz,
                stop_frequency_hz: self.analog_stop_frequency_hz,
                points_per_decade: self.analog_points_per_decade,
            };
            match append_analog_ac_scenario_with_project_path(
                &self.project_yaml,
                Path::new(&self.project_path),
                &draft,
            ) {
                Ok(updated) => self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "AC/Bode run setup {} added.",
                        self.analog_scenario_name.trim()
                    ),
                ),
                Err(error) => self.record_error(error),
            }
        } else if self.analog_run_setup_kind == "dc" {
            let draft = AnalogDcScenarioDraft {
                name: self.analog_scenario_name.clone(),
                ground_net: self.analog_ground_net.clone(),
                probe_net: self.analog_probe_net.clone(),
                probe_name: self.analog_probe_name.clone(),
            };
            match append_analog_dc_scenario_with_project_path(
                &self.project_yaml,
                Path::new(&self.project_path),
                &draft,
            ) {
                Ok(updated) => self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "DC operating-point run setup {} added.",
                        self.analog_scenario_name.trim()
                    ),
                ),
                Err(error) => self.record_error(error),
            }
        } else if self.analog_run_setup_kind == "dc_sweep" {
            let draft = AnalogDcSweepScenarioDraft {
                name: self.analog_scenario_name.clone(),
                ground_net: self.analog_ground_net.clone(),
                probe_net: self.analog_probe_net.clone(),
                probe_name: self.analog_probe_name.clone(),
                source: self.analog_dc_sweep_source.clone(),
                start: self.analog_dc_sweep_start,
                stop: self.analog_dc_sweep_stop,
                step: self.analog_dc_sweep_step,
            };
            match append_analog_dc_sweep_scenario_with_project_path(
                &self.project_yaml,
                Path::new(&self.project_path),
                &draft,
            ) {
                Ok(updated) => self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "DC sweep run setup {} added.",
                        self.analog_scenario_name.trim()
                    ),
                ),
                Err(error) => self.record_error(error),
            }
        } else if self.analog_run_setup_kind == "tf" {
            let draft = AnalogTransferFunctionScenarioDraft {
                name: self.analog_scenario_name.clone(),
                ground_net: self.analog_ground_net.clone(),
                probe_net: self.analog_probe_net.clone(),
                probe_name: self.analog_probe_name.clone(),
                input_source: self.analog_transfer_function_input_source.clone(),
            };
            match append_analog_transfer_function_scenario_with_project_path(
                &self.project_yaml,
                Path::new(&self.project_path),
                &draft,
            ) {
                Ok(updated) => self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Transfer-function run setup {} added.",
                        self.analog_scenario_name.trim()
                    ),
                ),
                Err(error) => self.record_error(error),
            }
        } else if self.analog_run_setup_kind == "pz" {
            let draft = AnalogPoleZeroScenarioDraft {
                name: self.analog_scenario_name.clone(),
                ground_net: self.analog_ground_net.clone(),
                probe_net: self.analog_probe_net.clone(),
                probe_name: self.analog_probe_name.clone(),
                input_source: self.analog_pole_zero_input_source.clone(),
                mode: self.analog_pole_zero_mode.clone(),
            };
            match append_analog_pole_zero_scenario_with_project_path(
                &self.project_yaml,
                Path::new(&self.project_path),
                &draft,
            ) {
                Ok(updated) => self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Pole-zero run setup {} added.",
                        self.analog_scenario_name.trim()
                    ),
                ),
                Err(error) => self.record_error(error),
            }
        } else if self.analog_run_setup_kind == "sens" {
            let draft = AnalogSensitivityScenarioDraft {
                name: self.analog_scenario_name.clone(),
                ground_net: self.analog_ground_net.clone(),
                probe_net: self.analog_probe_net.clone(),
                probe_name: self.analog_probe_name.clone(),
                mode: self.analog_sensitivity_mode.clone(),
                start_frequency_hz: self.analog_start_frequency_hz,
                stop_frequency_hz: self.analog_stop_frequency_hz,
                points_per_decade: self.analog_points_per_decade,
                filters: sensitivity_filters_from_text(&self.analog_sensitivity_filters),
            };
            match append_analog_sensitivity_scenario_with_project_path(
                &self.project_yaml,
                Path::new(&self.project_path),
                &draft,
            ) {
                Ok(updated) => self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Sensitivity run setup {} added.",
                        self.analog_scenario_name.trim()
                    ),
                ),
                Err(error) => self.record_error(error),
            }
        } else if self.analog_run_setup_kind == "disto" {
            let f2_sources = if self.analog_distortion_mode == "intermodulation" {
                vec![self.analog_distortion_f2_source.clone()]
            } else {
                Vec::new()
            };
            let f2_over_f1 = if self.analog_distortion_mode == "intermodulation" {
                Some(self.analog_distortion_f2_over_f1)
            } else {
                None
            };
            let draft = AnalogDistortionScenarioDraft {
                name: self.analog_scenario_name.clone(),
                ground_net: self.analog_ground_net.clone(),
                probe_net: self.analog_probe_net.clone(),
                probe_name: self.analog_probe_name.clone(),
                mode: self.analog_distortion_mode.clone(),
                start_frequency_hz: self.analog_start_frequency_hz,
                stop_frequency_hz: self.analog_stop_frequency_hz,
                points_per_decade: self.analog_points_per_decade,
                f1_sources: vec![self.analog_distortion_f1_source.clone()],
                f2_sources,
                f2_over_f1,
            };
            match append_analog_distortion_scenario_with_project_path(
                &self.project_yaml,
                Path::new(&self.project_path),
                &draft,
            ) {
                Ok(updated) => self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Distortion run setup {} added.",
                        self.analog_scenario_name.trim()
                    ),
                ),
                Err(error) => self.record_error(error),
            }
        } else if self.analog_run_setup_kind == "measure" {
            let draft = AnalogMeasureScenarioDraft {
                name: self.analog_scenario_name.clone(),
                ground_net: self.analog_ground_net.clone(),
                probe_net: self.analog_probe_net.clone(),
                probe_name: self.analog_probe_name.clone(),
                mode: self.analog_measure_mode.clone(),
                template_name: self.analog_measure_template_name.clone(),
                operation: self.analog_measure_operation.clone(),
                from: self.analog_measure_from,
                to: self.analog_measure_to,
                stop_time_us: self.analog_stop_time_us,
                max_step_us: self.analog_max_step_us,
                start_frequency_hz: self.analog_start_frequency_hz,
                stop_frequency_hz: self.analog_stop_frequency_hz,
                points_per_decade: self.analog_points_per_decade,
            };
            match append_analog_measure_scenario_with_project_path(
                &self.project_yaml,
                Path::new(&self.project_path),
                &draft,
            ) {
                Ok(updated) => self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Measure run setup {} added.",
                        self.analog_scenario_name.trim()
                    ),
                ),
                Err(error) => self.record_error(error),
            }
        } else if self.analog_run_setup_kind == "sparam" {
            let draft = AnalogSParameterScenarioDraft {
                name: self.analog_scenario_name.clone(),
                ground_net: self.analog_ground_net.clone(),
                port1_net: self.analog_probe_net.clone(),
                port2_net: self.analog_sparameter_port2_net.clone(),
                probe_name: self.analog_probe_name.clone(),
                start_frequency_hz: self.analog_start_frequency_hz,
                stop_frequency_hz: self.analog_stop_frequency_hz,
                points_per_decade: self.analog_points_per_decade,
                reference_impedance_ohm: self.analog_sparameter_reference_impedance_ohm,
            };
            match append_analog_sparameter_scenario_with_project_path(
                &self.project_yaml,
                Path::new(&self.project_path),
                &draft,
            ) {
                Ok(updated) => self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "S-parameter run setup {} added.",
                        self.analog_scenario_name.trim()
                    ),
                ),
                Err(error) => self.record_error(error),
            }
        } else if self.analog_run_setup_kind == "noise" {
            let input_probe = format!("{}_input", self.analog_probe_name.trim());
            let draft = AnalogNoiseScenarioDraft {
                name: self.analog_scenario_name.clone(),
                ground_net: self.analog_ground_net.clone(),
                probe_net: self.analog_probe_net.clone(),
                output_probe_name: self.analog_probe_name.clone(),
                input_probe_name: input_probe,
                input_source: self.analog_noise_input_source.clone(),
                start_frequency_hz: self.analog_start_frequency_hz,
                stop_frequency_hz: self.analog_stop_frequency_hz,
                points_per_decade: self.analog_points_per_decade,
            };
            match append_analog_noise_scenario_with_project_path(
                &self.project_yaml,
                Path::new(&self.project_path),
                &draft,
            ) {
                Ok(updated) => self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Noise run setup {} added.",
                        self.analog_scenario_name.trim()
                    ),
                ),
                Err(error) => self.record_error(error),
            }
        } else if self.analog_run_setup_kind == "fourier" {
            let draft = AnalogFourierScenarioDraft {
                name: self.analog_scenario_name.clone(),
                ground_net: self.analog_ground_net.clone(),
                probe_net: self.analog_probe_net.clone(),
                probe_name: self.analog_probe_name.clone(),
                stop_time_us: self.analog_stop_time_us,
                max_step_us: self.analog_max_step_us,
                fundamental_frequency_hz: self.analog_fourier_fundamental_frequency_hz,
                harmonics: self.analog_fourier_harmonics,
            };
            match append_analog_fourier_scenario_with_project_path(
                &self.project_yaml,
                Path::new(&self.project_path),
                &draft,
            ) {
                Ok(updated) => self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Fourier run setup {} added.",
                        self.analog_scenario_name.trim()
                    ),
                ),
                Err(error) => self.record_error(error),
            }
        } else if self.analog_run_setup_kind == "hb" {
            let draft = AnalogHarmonicBalanceScenarioDraft {
                name: self.analog_scenario_name.clone(),
                ground_net: self.analog_ground_net.clone(),
                probe_net: self.analog_probe_net.clone(),
                probe_name: self.analog_probe_name.clone(),
                fundamental_frequency_hz: self.analog_hb_fundamental_frequency_hz,
                harmonics: self.analog_hb_harmonics,
                drive_source: self.analog_hb_drive_source.clone(),
            };
            match append_analog_harmonic_balance_scenario_with_project_path(
                &self.project_yaml,
                Path::new(&self.project_path),
                &draft,
            ) {
                Ok(updated) => self.apply_edited_project_yaml(
                    updated,
                    &format!(
                        "Harmonic-balance run setup {} added.",
                        self.analog_scenario_name.trim()
                    ),
                ),
                Err(error) => self.record_error(error),
            }
        } else {
            let draft = AnalogScenarioDraft {
                name: self.analog_scenario_name.clone(),
                ground_net: self.analog_ground_net.clone(),
                probe_net: self.analog_probe_net.clone(),
                probe_name: self.analog_probe_name.clone(),
                stop_time_us: self.analog_stop_time_us,
                max_step_us: self.analog_max_step_us,
            };
            match append_analog_transient_scenario_with_project_path(
                &self.project_yaml,
                Path::new(&self.project_path),
                &draft,
            ) {
                Ok(updated) => self.apply_edited_project_yaml(
                    updated,
                    &format!("Run setup {} added.", self.analog_scenario_name.trim()),
                ),
                Err(error) => self.record_error(error),
            }
        }
    }
}
