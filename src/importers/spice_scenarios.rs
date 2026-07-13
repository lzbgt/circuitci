use super::*;
use anyhow::{Context, Result, bail};

pub(super) fn transient_scenario_for_yaml(
    options: &SpiceImportOptions,
    parts: &AnalogScenarioParts,
    stop_time_us: f64,
    max_step_us: f64,
) -> ScenarioYaml {
    ScenarioYaml {
        name: "imported_spice_transient".to_string(),
        scenario_type: "analog_transient".to_string(),
        checks: vec!["SPICE_TRANSIENT_ANALYSIS".to_string()],
        analog: AnalogYaml {
            backend: options.backend.clone(),
            netlist_source: "file".to_string(),
            netlist: parts.netlist.clone(),
            model_files: parts.model_files.clone(),
            node_bindings: parts.node_bindings.clone(),
            pin_bindings: parts.pin_bindings.clone(),
            analysis: AnalysisYaml {
                stop_time_us: Some(stop_time_us),
                max_step_us: Some(max_step_us),
                ..analysis_yaml("tran")
            },
            stimuli: parts.stimuli.clone(),
            probes: parts.probes.clone(),
            assertions: Vec::new(),
        },
    }
}

pub(super) fn operating_point_scenario_for_yaml(
    options: &SpiceImportOptions,
    parts: &AnalogScenarioParts,
) -> ScenarioYaml {
    ScenarioYaml {
        name: "imported_spice_operating_point".to_string(),
        scenario_type: "analog_dc".to_string(),
        checks: vec!["SPICE_DC_ANALYSIS".to_string()],
        analog: AnalogYaml {
            backend: options.backend.clone(),
            netlist_source: "file".to_string(),
            netlist: parts.netlist.clone(),
            model_files: parts.model_files.clone(),
            node_bindings: parts.node_bindings.clone(),
            pin_bindings: parts.pin_bindings.clone(),
            analysis: analysis_yaml("op"),
            stimuli: parts.stimuli.clone(),
            probes: parts.probes.clone(),
            assertions: Vec::new(),
        },
    }
}

pub(super) fn dc_sweep_scenario_for_yaml(
    options: &SpiceImportOptions,
    parts: &AnalogScenarioParts,
    dc: &DcSweepSpec,
) -> ScenarioYaml {
    ScenarioYaml {
        name: "imported_spice_dc_sweep".to_string(),
        scenario_type: "analog_dc_sweep".to_string(),
        checks: vec!["SPICE_DC_SWEEP_ANALYSIS".to_string()],
        analog: AnalogYaml {
            backend: options.backend.clone(),
            netlist_source: "file".to_string(),
            netlist: parts.netlist.clone(),
            model_files: parts.model_files.clone(),
            node_bindings: parts.node_bindings.clone(),
            pin_bindings: parts.pin_bindings.clone(),
            analysis: AnalysisYaml {
                dc_sweep_source: Some(dc.source.clone()),
                dc_sweep_start: Some(dc.start),
                dc_sweep_stop: Some(dc.stop),
                dc_sweep_step: Some(dc.step),
                ..analysis_yaml("dc_sweep")
            },
            stimuli: parts.stimuli.clone(),
            probes: parts.probes.clone(),
            assertions: Vec::new(),
        },
    }
}

pub(super) fn ac_scenario_for_yaml(
    options: &SpiceImportOptions,
    parts: &AnalogScenarioParts,
    ac: &AcSpec,
) -> ScenarioYaml {
    ScenarioYaml {
        name: "imported_spice_ac".to_string(),
        scenario_type: "analog_ac".to_string(),
        checks: vec!["SPICE_AC_ANALYSIS".to_string()],
        analog: AnalogYaml {
            backend: options.backend.clone(),
            netlist_source: "file".to_string(),
            netlist: parts.netlist.clone(),
            model_files: parts.model_files.clone(),
            node_bindings: parts.node_bindings.clone(),
            pin_bindings: parts.pin_bindings.clone(),
            analysis: AnalysisYaml {
                start_frequency_hz: Some(ac.start_frequency_hz),
                stop_frequency_hz: Some(ac.stop_frequency_hz),
                points_per_decade: Some(ac.points_per_decade),
                ..analysis_yaml("ac")
            },
            stimuli: parts.stimuli.clone(),
            probes: parts.probes.clone(),
            assertions: Vec::new(),
        },
    }
}

pub(super) fn noise_scenario_for_yaml(
    options: &SpiceImportOptions,
    parts: &AnalogScenarioParts,
    noise: &NoiseSpec,
) -> ScenarioYaml {
    ScenarioYaml {
        name: "imported_spice_noise".to_string(),
        scenario_type: "analog_noise".to_string(),
        checks: vec!["SPICE_NOISE_ANALYSIS".to_string()],
        analog: AnalogYaml {
            backend: options.backend.clone(),
            netlist_source: "file".to_string(),
            netlist: parts.netlist.clone(),
            model_files: parts.model_files.clone(),
            node_bindings: parts.node_bindings.clone(),
            pin_bindings: parts.pin_bindings.clone(),
            analysis: AnalysisYaml {
                start_frequency_hz: Some(noise.start_frequency_hz),
                stop_frequency_hz: Some(noise.stop_frequency_hz),
                points_per_decade: Some(noise.points_per_decade),
                noise_output_node: Some(noise.output_node.clone()),
                noise_reference_node: noise.reference_node.clone(),
                noise_input_source: Some(noise.input_source.clone()),
                ..analysis_yaml("noise")
            },
            stimuli: parts.stimuli.clone(),
            probes: parts.probes.clone(),
            assertions: Vec::new(),
        },
    }
}

pub(super) fn transfer_function_scenario_for_yaml(
    options: &SpiceImportOptions,
    parts: &AnalogScenarioParts,
    transfer_function: &TransferFunctionSpec,
) -> ScenarioYaml {
    ScenarioYaml {
        name: "imported_spice_transfer_function".to_string(),
        scenario_type: "analog_transfer_function".to_string(),
        checks: vec!["SPICE_TRANSFER_FUNCTION_ANALYSIS".to_string()],
        analog: AnalogYaml {
            backend: options.backend.clone(),
            netlist_source: "file".to_string(),
            netlist: parts.netlist.clone(),
            model_files: parts.model_files.clone(),
            node_bindings: parts.node_bindings.clone(),
            pin_bindings: parts.pin_bindings.clone(),
            analysis: AnalysisYaml {
                transfer_output_expression: Some(transfer_function.output_expression.clone()),
                transfer_input_source: Some(transfer_function.input_source.clone()),
                ..analysis_yaml("tf")
            },
            stimuli: parts.stimuli.clone(),
            probes: parts.probes.clone(),
            assertions: Vec::new(),
        },
    }
}

pub(super) fn pole_zero_scenario_for_yaml(
    options: &SpiceImportOptions,
    parts: &AnalogScenarioParts,
    pole_zero: &PoleZeroSpec,
) -> ScenarioYaml {
    ScenarioYaml {
        name: "imported_spice_pole_zero".to_string(),
        scenario_type: "analog_pole_zero".to_string(),
        checks: vec!["SPICE_POLE_ZERO_ANALYSIS".to_string()],
        analog: AnalogYaml {
            backend: options.backend.clone(),
            netlist_source: "file".to_string(),
            netlist: parts.netlist.clone(),
            model_files: parts.model_files.clone(),
            node_bindings: parts.node_bindings.clone(),
            pin_bindings: parts.pin_bindings.clone(),
            analysis: AnalysisYaml {
                pole_zero_output_node: Some(pole_zero.output_node.clone()),
                pole_zero_reference_node: Some(pole_zero.reference_node.clone()),
                pole_zero_input_source: Some(pole_zero.input_source.clone()),
                pole_zero_mode: Some(pole_zero.mode.clone()),
                ..analysis_yaml("pz")
            },
            stimuli: parts.stimuli.clone(),
            probes: parts.probes.clone(),
            assertions: Vec::new(),
        },
    }
}

pub(super) fn sensitivity_scenario_for_yaml(
    options: &SpiceImportOptions,
    parts: &AnalogScenarioParts,
    sensitivity: &SensitivitySpec,
) -> ScenarioYaml {
    let (start_frequency_hz, stop_frequency_hz, points_per_decade) =
        sensitivity.ac.as_ref().map_or((None, None, None), |ac| {
            (
                Some(ac.start_frequency_hz),
                Some(ac.stop_frequency_hz),
                Some(ac.points_per_decade),
            )
        });
    ScenarioYaml {
        name: "imported_spice_sensitivity".to_string(),
        scenario_type: "analog_sensitivity".to_string(),
        checks: vec!["SPICE_SENSITIVITY_ANALYSIS".to_string()],
        analog: AnalogYaml {
            backend: options.backend.clone(),
            netlist_source: "file".to_string(),
            netlist: parts.netlist.clone(),
            model_files: parts.model_files.clone(),
            node_bindings: parts.node_bindings.clone(),
            pin_bindings: parts.pin_bindings.clone(),
            analysis: AnalysisYaml {
                start_frequency_hz,
                stop_frequency_hz,
                points_per_decade,
                sensitivity_output_expression: Some(sensitivity.output_expression.clone()),
                sensitivity_mode: Some(sensitivity.mode.clone()),
                sensitivity_filters: sensitivity.filters.clone(),
                ..analysis_yaml("sens")
            },
            stimuli: parts.stimuli.clone(),
            probes: parts.probes.clone(),
            assertions: Vec::new(),
        },
    }
}

pub(super) fn distortion_scenario_for_yaml(
    options: &SpiceImportOptions,
    parts: &AnalogScenarioParts,
    distortion: &DistortionSpec,
) -> ScenarioYaml {
    ScenarioYaml {
        name: "imported_spice_distortion".to_string(),
        scenario_type: "analog_distortion".to_string(),
        checks: vec!["SPICE_DISTORTION_ANALYSIS".to_string()],
        analog: AnalogYaml {
            backend: options.backend.clone(),
            netlist_source: "file".to_string(),
            netlist: parts.netlist.clone(),
            model_files: parts.model_files.clone(),
            node_bindings: parts.node_bindings.clone(),
            pin_bindings: parts.pin_bindings.clone(),
            analysis: AnalysisYaml {
                distortion_mode: Some(distortion.mode.clone()),
                distortion_start_frequency_hz: Some(distortion.start_frequency_hz),
                distortion_stop_frequency_hz: Some(distortion.stop_frequency_hz),
                distortion_points_per_decade: Some(distortion.points_per_decade),
                distortion_output_expression: Some(distortion.output_expression.clone()),
                distortion_f1_sources: distortion.f1_sources.clone(),
                distortion_f2_sources: distortion.f2_sources.clone(),
                distortion_f2_over_f1: distortion.f2_over_f1,
                ..analysis_yaml("disto")
            },
            stimuli: parts.stimuli.clone(),
            probes: parts.probes.clone(),
            assertions: Vec::new(),
        },
    }
}

pub(super) fn fourier_scenario_for_yaml(
    options: &SpiceImportOptions,
    parts: &AnalogScenarioParts,
    fourier: &FourierSpec,
    index: usize,
    multiple_outputs: bool,
    stop_time_us: f64,
    max_step_us: f64,
) -> ScenarioYaml {
    let name = if multiple_outputs {
        format!("imported_spice_fourier_{}", index + 1)
    } else {
        "imported_spice_fourier".to_string()
    };
    ScenarioYaml {
        name,
        scenario_type: "analog_fourier".to_string(),
        checks: vec!["SPICE_FOURIER_ANALYSIS".to_string()],
        analog: AnalogYaml {
            backend: options.backend.clone(),
            netlist_source: "file".to_string(),
            netlist: parts.netlist.clone(),
            model_files: parts.model_files.clone(),
            node_bindings: parts.node_bindings.clone(),
            pin_bindings: parts.pin_bindings.clone(),
            analysis: AnalysisYaml {
                stop_time_us: Some(stop_time_us),
                max_step_us: Some(max_step_us),
                fourier_fundamental_frequency_hz: Some(fourier.fundamental_frequency_hz),
                fourier_output_expression: Some(fourier.output_expression.clone()),
                ..analysis_yaml("fourier")
            },
            stimuli: parts.stimuli.clone(),
            probes: parts.probes.clone(),
            assertions: Vec::new(),
        },
    }
}

pub(super) fn measure_scenario_for_yaml(
    options: &SpiceImportOptions,
    deck: &ParsedDeck,
    parts: AnalogScenarioParts,
    stop_time_us: f64,
    max_step_us: f64,
) -> Result<ScenarioYaml> {
    let mode = common_measure_mode(&deck.measures)?;
    let (
        tran_stop_time_us,
        tran_max_step_us,
        start_frequency_hz,
        stop_frequency_hz,
        points_per_decade,
    ) = if mode == "ac" {
        let ac = deck
            .ac
            .as_ref()
            .context("SPICE .meas ac import requires a valid .ac dec sweep.")?;
        (
            None,
            None,
            Some(ac.start_frequency_hz),
            Some(ac.stop_frequency_hz),
            Some(ac.points_per_decade),
        )
    } else {
        (Some(stop_time_us), Some(max_step_us), None, None, None)
    };
    Ok(ScenarioYaml {
        name: format!("{}_measure", mode),
        scenario_type: "analog_measure".to_string(),
        checks: vec!["SPICE_MEASURE_ANALYSIS".to_string()],
        analog: AnalogYaml {
            backend: options.backend.clone(),
            netlist_source: "file".to_string(),
            netlist: parts.netlist,
            model_files: parts.model_files,
            node_bindings: parts.node_bindings,
            pin_bindings: parts.pin_bindings,
            analysis: AnalysisYaml {
                stop_time_us: tran_stop_time_us,
                max_step_us: tran_max_step_us,
                start_frequency_hz,
                stop_frequency_hz,
                points_per_decade,
                measure_mode: Some(mode.to_string()),
                measure_statements: deck
                    .measures
                    .iter()
                    .map(|measure| MeasureStatementYaml {
                        name: measure.name.clone(),
                        statement: measure.statement.clone(),
                    })
                    .collect(),
                ..analysis_yaml("measure")
            },
            stimuli: parts.stimuli,
            probes: parts.probes,
            assertions: Vec::new(),
        },
    })
}

fn analysis_yaml(analysis_type: &str) -> AnalysisYaml {
    AnalysisYaml {
        analysis_type: analysis_type.to_string(),
        stop_time_us: None,
        max_step_us: None,
        start_frequency_hz: None,
        stop_frequency_hz: None,
        points_per_decade: None,
        noise_output_node: None,
        noise_reference_node: None,
        noise_input_source: None,
        transfer_output_expression: None,
        transfer_input_source: None,
        transfer_function_assertions: Vec::new(),
        pole_zero_output_node: None,
        pole_zero_reference_node: None,
        pole_zero_input_source: None,
        pole_zero_mode: None,
        pole_zero_assertions: Vec::new(),
        sensitivity_output_expression: None,
        sensitivity_mode: None,
        sensitivity_filters: Vec::new(),
        sensitivity_assertions: Vec::new(),
        distortion_mode: None,
        distortion_start_frequency_hz: None,
        distortion_stop_frequency_hz: None,
        distortion_points_per_decade: None,
        distortion_output_expression: None,
        distortion_f1_sources: Vec::new(),
        distortion_f2_sources: Vec::new(),
        distortion_f2_over_f1: None,
        distortion_assertions: Vec::new(),
        fourier_fundamental_frequency_hz: None,
        fourier_output_expression: None,
        fourier_harmonics: None,
        fourier_assertions: Vec::new(),
        dc_sweep_source: None,
        dc_sweep_start: None,
        dc_sweep_stop: None,
        dc_sweep_step: None,
        dc_sweep_assertions: Vec::new(),
        measure_mode: None,
        measure_statements: Vec::new(),
    }
}

fn common_measure_mode(measures: &[MeasureStatementSpec]) -> Result<&str> {
    let first = measures
        .first()
        .context("measure scenario requires at least one measure statement")?;
    if let Some(other) = measures.iter().find(|measure| measure.mode != first.mode) {
        bail!(
            "SPICE .meas statements mix modes {} and {}; import-spice emits one measure scenario per deck.",
            first.mode,
            other.mode
        );
    }
    Ok(&first.mode)
}
