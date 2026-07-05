use anyhow::Result;
use std::path::Path;

mod generated;
mod planned;
pub(in crate::gui) use generated::{
    GeneratedAnalogScenarioKind, append_generated_analog_scenario_with_project_path,
};
pub(super) use planned::{
    AnalogPeriodicAcScenarioDraft, AnalogPhaseNoiseScenarioDraft, AnalogPssScenarioDraft,
    append_analog_periodic_ac_scenario_with_project_path,
    append_analog_phase_noise_scenario_with_project_path,
    append_analog_pss_scenario_with_project_path,
};

#[derive(Debug, Clone)]
pub(super) struct AnalogScenarioDraft {
    pub(super) name: String,
    pub(super) ground_net: String,
    pub(super) probe_net: String,
    pub(super) probe_name: String,
    pub(super) stop_time_us: f64,
    pub(super) max_step_us: f64,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogAcScenarioDraft {
    pub(super) name: String,
    pub(super) ground_net: String,
    pub(super) probe_net: String,
    pub(super) probe_name: String,
    pub(super) start_frequency_hz: f64,
    pub(super) stop_frequency_hz: f64,
    pub(super) points_per_decade: u32,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogDcScenarioDraft {
    pub(super) name: String,
    pub(super) ground_net: String,
    pub(super) probe_net: String,
    pub(super) probe_name: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogDcSweepScenarioDraft {
    pub(super) name: String,
    pub(super) ground_net: String,
    pub(super) probe_net: String,
    pub(super) probe_name: String,
    pub(super) source: String,
    pub(super) start: f64,
    pub(super) stop: f64,
    pub(super) step: f64,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogTransferFunctionScenarioDraft {
    pub(super) name: String,
    pub(super) ground_net: String,
    pub(super) probe_net: String,
    pub(super) probe_name: String,
    pub(super) input_source: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogPoleZeroScenarioDraft {
    pub(super) name: String,
    pub(super) ground_net: String,
    pub(super) probe_net: String,
    pub(super) probe_name: String,
    pub(super) input_source: String,
    pub(super) mode: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogSensitivityScenarioDraft {
    pub(super) name: String,
    pub(super) ground_net: String,
    pub(super) probe_net: String,
    pub(super) probe_name: String,
    pub(super) mode: String,
    pub(super) start_frequency_hz: f64,
    pub(super) stop_frequency_hz: f64,
    pub(super) points_per_decade: u32,
    pub(super) filters: Vec<String>,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogDistortionScenarioDraft {
    pub(super) name: String,
    pub(super) ground_net: String,
    pub(super) probe_net: String,
    pub(super) probe_name: String,
    pub(super) mode: String,
    pub(super) start_frequency_hz: f64,
    pub(super) stop_frequency_hz: f64,
    pub(super) points_per_decade: u32,
    pub(super) f1_sources: Vec<String>,
    pub(super) f2_sources: Vec<String>,
    pub(super) f2_over_f1: Option<f64>,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogMeasureScenarioDraft {
    pub(super) name: String,
    pub(super) ground_net: String,
    pub(super) probe_net: String,
    pub(super) probe_name: String,
    pub(super) mode: String,
    pub(super) template_name: String,
    pub(super) operation: String,
    pub(super) from: f64,
    pub(super) to: f64,
    pub(super) stop_time_us: f64,
    pub(super) max_step_us: f64,
    pub(super) start_frequency_hz: f64,
    pub(super) stop_frequency_hz: f64,
    pub(super) points_per_decade: u32,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogSParameterScenarioDraft {
    pub(super) name: String,
    pub(super) ground_net: String,
    pub(super) port1_net: String,
    pub(super) port2_net: String,
    pub(super) probe_name: String,
    pub(super) start_frequency_hz: f64,
    pub(super) stop_frequency_hz: f64,
    pub(super) points_per_decade: u32,
    pub(super) reference_impedance_ohm: f64,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogNoiseScenarioDraft {
    pub(super) name: String,
    pub(super) ground_net: String,
    pub(super) probe_net: String,
    pub(super) output_probe_name: String,
    pub(super) input_probe_name: String,
    pub(super) input_source: String,
    pub(super) start_frequency_hz: f64,
    pub(super) stop_frequency_hz: f64,
    pub(super) points_per_decade: u32,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogHarmonicBalanceScenarioDraft {
    pub(super) name: String,
    pub(super) ground_net: String,
    pub(super) probe_net: String,
    pub(super) probe_name: String,
    pub(super) fundamental_frequency_hz: f64,
    pub(super) harmonics: u32,
    pub(super) drive_source: String,
}

#[derive(Debug, Clone)]
pub(super) struct AnalogFourierScenarioDraft {
    pub(super) name: String,
    pub(super) ground_net: String,
    pub(super) probe_net: String,
    pub(super) probe_name: String,
    pub(super) stop_time_us: f64,
    pub(super) max_step_us: f64,
    pub(super) fundamental_frequency_hz: f64,
    pub(super) harmonics: u32,
}

#[cfg(test)]
pub(super) fn append_analog_transient_scenario(
    text: &str,
    draft: &AnalogScenarioDraft,
) -> Result<String> {
    append_analog_transient_scenario_with_project_path(text, Path::new("project.yaml"), draft)
}

pub(super) fn append_analog_transient_scenario_with_project_path(
    text: &str,
    project_path: &Path,
    draft: &AnalogScenarioDraft,
) -> Result<String> {
    validate_transient_draft(draft)?;
    append_generated_analog_scenario_with_project_path(
        text,
        project_path,
        &draft.name,
        &draft.ground_net,
        &draft.probe_net,
        &draft.probe_name,
        GeneratedAnalogScenarioKind::Transient {
            stop_time_us: draft.stop_time_us,
            max_step_us: draft.max_step_us,
        },
    )
}

pub(super) fn append_analog_ac_scenario_with_project_path(
    text: &str,
    project_path: &Path,
    draft: &AnalogAcScenarioDraft,
) -> Result<String> {
    validate_ac_draft(draft)?;
    append_generated_analog_scenario_with_project_path(
        text,
        project_path,
        &draft.name,
        &draft.ground_net,
        &draft.probe_net,
        &draft.probe_name,
        GeneratedAnalogScenarioKind::Ac {
            start_frequency_hz: draft.start_frequency_hz,
            stop_frequency_hz: draft.stop_frequency_hz,
            points_per_decade: draft.points_per_decade,
        },
    )
}

pub(super) fn append_analog_dc_scenario_with_project_path(
    text: &str,
    project_path: &Path,
    draft: &AnalogDcScenarioDraft,
) -> Result<String> {
    validate_dc_draft(draft)?;
    append_generated_analog_scenario_with_project_path(
        text,
        project_path,
        &draft.name,
        &draft.ground_net,
        &draft.probe_net,
        &draft.probe_name,
        GeneratedAnalogScenarioKind::Dc,
    )
}

pub(super) fn append_analog_dc_sweep_scenario_with_project_path(
    text: &str,
    project_path: &Path,
    draft: &AnalogDcSweepScenarioDraft,
) -> Result<String> {
    validate_dc_sweep_draft(draft)?;
    append_generated_analog_scenario_with_project_path(
        text,
        project_path,
        &draft.name,
        &draft.ground_net,
        &draft.probe_net,
        &draft.probe_name,
        GeneratedAnalogScenarioKind::DcSweep {
            source: draft.source.trim().to_string(),
            start: draft.start,
            stop: draft.stop,
            step: draft.step,
        },
    )
}

pub(super) fn append_analog_transfer_function_scenario_with_project_path(
    text: &str,
    project_path: &Path,
    draft: &AnalogTransferFunctionScenarioDraft,
) -> Result<String> {
    validate_transfer_function_draft(draft)?;
    append_generated_analog_scenario_with_project_path(
        text,
        project_path,
        &draft.name,
        &draft.ground_net,
        &draft.probe_net,
        &draft.probe_name,
        GeneratedAnalogScenarioKind::TransferFunction {
            input_source: draft.input_source.trim().to_string(),
        },
    )
}

pub(super) fn append_analog_pole_zero_scenario_with_project_path(
    text: &str,
    project_path: &Path,
    draft: &AnalogPoleZeroScenarioDraft,
) -> Result<String> {
    validate_pole_zero_draft(draft)?;
    append_generated_analog_scenario_with_project_path(
        text,
        project_path,
        &draft.name,
        &draft.ground_net,
        &draft.probe_net,
        &draft.probe_name,
        GeneratedAnalogScenarioKind::PoleZero {
            input_source: draft.input_source.trim().to_string(),
            mode: draft.mode.trim().to_string(),
        },
    )
}

pub(super) fn append_analog_sensitivity_scenario_with_project_path(
    text: &str,
    project_path: &Path,
    draft: &AnalogSensitivityScenarioDraft,
) -> Result<String> {
    validate_sensitivity_draft(draft)?;
    append_generated_analog_scenario_with_project_path(
        text,
        project_path,
        &draft.name,
        &draft.ground_net,
        &draft.probe_net,
        &draft.probe_name,
        GeneratedAnalogScenarioKind::Sensitivity {
            mode: draft.mode.trim().to_string(),
            start_frequency_hz: draft.start_frequency_hz,
            stop_frequency_hz: draft.stop_frequency_hz,
            points_per_decade: draft.points_per_decade,
            filters: draft
                .filters
                .iter()
                .map(|filter| filter.trim().to_string())
                .filter(|filter| !filter.is_empty())
                .collect(),
        },
    )
}

pub(super) fn append_analog_distortion_scenario_with_project_path(
    text: &str,
    project_path: &Path,
    draft: &AnalogDistortionScenarioDraft,
) -> Result<String> {
    validate_distortion_draft(draft)?;
    append_generated_analog_scenario_with_project_path(
        text,
        project_path,
        &draft.name,
        &draft.ground_net,
        &draft.probe_net,
        &draft.probe_name,
        GeneratedAnalogScenarioKind::Distortion {
            mode: draft.mode.trim().to_string(),
            start_frequency_hz: draft.start_frequency_hz,
            stop_frequency_hz: draft.stop_frequency_hz,
            points_per_decade: draft.points_per_decade,
            f1_sources: trim_nonempty_values(&draft.f1_sources),
            f2_sources: trim_nonempty_values(&draft.f2_sources),
            f2_over_f1: draft.f2_over_f1,
        },
    )
}

pub(super) fn append_analog_measure_scenario_with_project_path(
    text: &str,
    project_path: &Path,
    draft: &AnalogMeasureScenarioDraft,
) -> Result<String> {
    validate_measure_draft(draft)?;
    append_generated_analog_scenario_with_project_path(
        text,
        project_path,
        &draft.name,
        &draft.ground_net,
        &draft.probe_net,
        &draft.probe_name,
        GeneratedAnalogScenarioKind::Measure {
            mode: draft.mode.trim().to_string(),
            template_name: draft.template_name.trim().to_string(),
            operation: draft.operation.trim().to_string(),
            from: draft.from,
            to: draft.to,
            stop_time_us: draft.stop_time_us,
            max_step_us: draft.max_step_us,
            start_frequency_hz: draft.start_frequency_hz,
            stop_frequency_hz: draft.stop_frequency_hz,
            points_per_decade: draft.points_per_decade,
        },
    )
}

pub(super) fn append_analog_sparameter_scenario_with_project_path(
    text: &str,
    project_path: &Path,
    draft: &AnalogSParameterScenarioDraft,
) -> Result<String> {
    validate_sparameter_draft(draft)?;
    append_generated_analog_scenario_with_project_path(
        text,
        project_path,
        &draft.name,
        &draft.ground_net,
        &draft.port1_net,
        &draft.probe_name,
        GeneratedAnalogScenarioKind::SParameter {
            port2_net: draft.port2_net.trim().to_string(),
            start_frequency_hz: draft.start_frequency_hz,
            stop_frequency_hz: draft.stop_frequency_hz,
            points_per_decade: draft.points_per_decade,
            reference_impedance_ohm: draft.reference_impedance_ohm,
        },
    )
}

pub(super) fn append_analog_noise_scenario_with_project_path(
    text: &str,
    project_path: &Path,
    draft: &AnalogNoiseScenarioDraft,
) -> Result<String> {
    validate_noise_draft(draft)?;
    append_generated_analog_scenario_with_project_path(
        text,
        project_path,
        &draft.name,
        &draft.ground_net,
        &draft.probe_net,
        &draft.output_probe_name,
        GeneratedAnalogScenarioKind::Noise {
            start_frequency_hz: draft.start_frequency_hz,
            stop_frequency_hz: draft.stop_frequency_hz,
            points_per_decade: draft.points_per_decade,
            input_source: draft.input_source.trim().to_string(),
            input_probe_name: draft.input_probe_name.trim().to_string(),
        },
    )
}

pub(super) fn append_analog_harmonic_balance_scenario_with_project_path(
    text: &str,
    project_path: &Path,
    draft: &AnalogHarmonicBalanceScenarioDraft,
) -> Result<String> {
    validate_harmonic_balance_draft(draft)?;
    append_generated_analog_scenario_with_project_path(
        text,
        project_path,
        &draft.name,
        &draft.ground_net,
        &draft.probe_net,
        &draft.probe_name,
        GeneratedAnalogScenarioKind::HarmonicBalance {
            fundamental_frequency_hz: draft.fundamental_frequency_hz,
            harmonics: draft.harmonics,
            drive_source: draft.drive_source.trim().to_string(),
        },
    )
}

pub(super) fn append_analog_fourier_scenario_with_project_path(
    text: &str,
    project_path: &Path,
    draft: &AnalogFourierScenarioDraft,
) -> Result<String> {
    validate_fourier_draft(draft)?;
    append_generated_analog_scenario_with_project_path(
        text,
        project_path,
        &draft.name,
        &draft.ground_net,
        &draft.probe_net,
        &draft.probe_name,
        GeneratedAnalogScenarioKind::Fourier {
            stop_time_us: draft.stop_time_us,
            max_step_us: draft.max_step_us,
            fundamental_frequency_hz: draft.fundamental_frequency_hz,
            harmonics: draft.harmonics,
        },
    )
}

fn validate_transient_draft(draft: &AnalogScenarioDraft) -> Result<()> {
    validated_id(&draft.name, "scenario name")?;
    validated_id(&draft.probe_name, "probe name")?;
    if draft.ground_net.trim().is_empty() {
        anyhow::bail!("Ground net must not be blank.");
    }
    if draft.probe_net.trim().is_empty() {
        anyhow::bail!("Probe net must not be blank.");
    }
    if !draft.stop_time_us.is_finite()
        || !draft.max_step_us.is_finite()
        || draft.stop_time_us <= 0.0
        || draft.max_step_us <= 0.0
        || draft.max_step_us > draft.stop_time_us
    {
        anyhow::bail!(
            "Stop time and max step must be finite positive values, with max step no larger than stop time."
        );
    }
    Ok(())
}

fn validate_ac_draft(draft: &AnalogAcScenarioDraft) -> Result<()> {
    validated_id(&draft.name, "scenario name")?;
    validated_id(&draft.probe_name, "probe name")?;
    if draft.ground_net.trim().is_empty() {
        anyhow::bail!("Ground net must not be blank.");
    }
    if draft.probe_net.trim().is_empty() {
        anyhow::bail!("Probe net must not be blank.");
    }
    if !draft.start_frequency_hz.is_finite()
        || !draft.stop_frequency_hz.is_finite()
        || draft.start_frequency_hz <= 0.0
        || draft.stop_frequency_hz <= draft.start_frequency_hz
        || draft.points_per_decade == 0
        || draft.points_per_decade > 1000
    {
        anyhow::bail!(
            "Analog AC/Bode start and stop frequencies must be finite and positive, stop must exceed start, and points per decade must be in 1..=1000."
        );
    }
    Ok(())
}

fn validate_dc_draft(draft: &AnalogDcScenarioDraft) -> Result<()> {
    validated_id(&draft.name, "scenario name")?;
    validated_id(&draft.probe_name, "probe name")?;
    if draft.ground_net.trim().is_empty() {
        anyhow::bail!("Ground net must not be blank.");
    }
    if draft.probe_net.trim().is_empty() {
        anyhow::bail!("Probe net must not be blank.");
    }
    Ok(())
}

fn validate_dc_sweep_draft(draft: &AnalogDcSweepScenarioDraft) -> Result<()> {
    validated_id(&draft.name, "scenario name")?;
    validated_id(&draft.probe_name, "probe name")?;
    validated_id(&draft.source, "DC sweep source")?;
    if draft.ground_net.trim().is_empty() {
        anyhow::bail!("Ground net must not be blank.");
    }
    if draft.probe_net.trim().is_empty() {
        anyhow::bail!("Probe net must not be blank.");
    }
    if !draft.start.is_finite()
        || !draft.stop.is_finite()
        || !draft.step.is_finite()
        || draft.start == draft.stop
        || draft.step <= 0.0
        || draft.step > (draft.stop - draft.start).abs()
    {
        anyhow::bail!(
            "DC sweep start and stop must be finite and distinct, and step must be finite positive and no larger than the sweep span."
        );
    }
    Ok(())
}

fn validate_transfer_function_draft(draft: &AnalogTransferFunctionScenarioDraft) -> Result<()> {
    validated_id(&draft.name, "scenario name")?;
    validated_id(&draft.probe_name, "probe name")?;
    validated_id(&draft.input_source, "transfer-function input source")?;
    if draft.ground_net.trim().is_empty() {
        anyhow::bail!("Ground net must not be blank.");
    }
    if draft.probe_net.trim().is_empty() {
        anyhow::bail!("Probe net must not be blank.");
    }
    Ok(())
}

fn validate_pole_zero_draft(draft: &AnalogPoleZeroScenarioDraft) -> Result<()> {
    validated_id(&draft.name, "scenario name")?;
    validated_id(&draft.probe_name, "probe name")?;
    validated_id(&draft.input_source, "pole-zero input source")?;
    if draft.ground_net.trim().is_empty() {
        anyhow::bail!("Ground net must not be blank.");
    }
    if draft.probe_net.trim().is_empty() {
        anyhow::bail!("Probe net must not be blank.");
    }
    match draft.mode.trim() {
        "poles" | "zeros" | "poles_and_zeros" => Ok(()),
        _ => anyhow::bail!("Pole-zero mode must be poles, zeros, or poles_and_zeros."),
    }
}

fn validate_sensitivity_draft(draft: &AnalogSensitivityScenarioDraft) -> Result<()> {
    validated_id(&draft.name, "scenario name")?;
    validated_id(&draft.probe_name, "probe name")?;
    if draft.ground_net.trim().is_empty() {
        anyhow::bail!("Ground net must not be blank.");
    }
    if draft.probe_net.trim().is_empty() {
        anyhow::bail!("Probe net must not be blank.");
    }
    match draft.mode.trim() {
        "dc" => {}
        "ac" => validate_frequency_range(
            draft.start_frequency_hz,
            draft.stop_frequency_hz,
            draft.points_per_decade,
            "Analog sensitivity",
        )?,
        _ => anyhow::bail!("Sensitivity mode must be dc or ac."),
    }
    if draft.filters.iter().all(|filter| filter.trim().is_empty()) {
        anyhow::bail!("Sensitivity filters must include at least one parameter.");
    }
    Ok(())
}

fn validate_distortion_draft(draft: &AnalogDistortionScenarioDraft) -> Result<()> {
    validated_id(&draft.name, "scenario name")?;
    validated_id(&draft.probe_name, "probe name")?;
    if draft.ground_net.trim().is_empty() {
        anyhow::bail!("Ground net must not be blank.");
    }
    if draft.probe_net.trim().is_empty() {
        anyhow::bail!("Probe net must not be blank.");
    }
    validate_frequency_range(
        draft.start_frequency_hz,
        draft.stop_frequency_hz,
        draft.points_per_decade,
        "Analog distortion",
    )?;
    for source in trim_nonempty_values(&draft.f1_sources) {
        validated_id(&source, "distortion F1 source")?;
    }
    for source in trim_nonempty_values(&draft.f2_sources) {
        validated_id(&source, "distortion F2 source")?;
    }
    if draft
        .f1_sources
        .iter()
        .all(|source| source.trim().is_empty())
    {
        anyhow::bail!("Distortion F1 sources must include at least one source.");
    }
    match draft.mode.trim() {
        "harmonic" => {
            if draft.f2_over_f1.is_some() {
                anyhow::bail!("Harmonic distortion mode must not set f2_over_f1.");
            }
        }
        "intermodulation" => {
            if draft
                .f2_sources
                .iter()
                .all(|source| source.trim().is_empty())
            {
                anyhow::bail!("Intermodulation distortion requires at least one F2 source.");
            }
            match draft.f2_over_f1 {
                Some(ratio) if ratio.is_finite() && ratio > 0.0 && ratio < 1.0 => {}
                _ => {
                    anyhow::bail!("Intermodulation distortion requires finite f2_over_f1 in 0..1.")
                }
            }
        }
        _ => anyhow::bail!("Distortion mode must be harmonic or intermodulation."),
    }
    Ok(())
}

fn validate_measure_draft(draft: &AnalogMeasureScenarioDraft) -> Result<()> {
    validated_id(&draft.name, "scenario name")?;
    validated_id(&draft.probe_name, "probe name")?;
    validated_id(&draft.template_name, "measure template name")?;
    if draft.ground_net.trim().is_empty() {
        anyhow::bail!("Ground net must not be blank.");
    }
    if draft.probe_net.trim().is_empty() {
        anyhow::bail!("Probe net must not be blank.");
    }
    if !matches!(draft.operation.trim(), "avg" | "max" | "min" | "rms") {
        anyhow::bail!("Measure operation must be avg, max, min, or rms.");
    }
    if !draft.from.is_finite()
        || !draft.to.is_finite()
        || draft.from < 0.0
        || draft.to <= draft.from
    {
        anyhow::bail!("Measure window start and stop must be finite, non-negative, and ordered.");
    }
    match draft.mode.trim() {
        "tran" => {
            if !draft.stop_time_us.is_finite()
                || !draft.max_step_us.is_finite()
                || draft.stop_time_us <= 0.0
                || draft.max_step_us <= 0.0
                || draft.max_step_us > draft.stop_time_us
                || draft.to > draft.stop_time_us
            {
                anyhow::bail!(
                    "Transient measure stop time and max step must be finite positive values, max step must not exceed stop time, and the measure window must fit inside stop time."
                );
            }
        }
        "ac" => {
            validate_frequency_range(
                draft.start_frequency_hz,
                draft.stop_frequency_hz,
                draft.points_per_decade,
                "Analog measure AC",
            )?;
            if draft.from < draft.start_frequency_hz || draft.to > draft.stop_frequency_hz {
                anyhow::bail!("AC measure window must fit inside the AC sweep range.");
            }
        }
        _ => anyhow::bail!("Measure mode must be tran or ac."),
    }
    Ok(())
}

fn validate_sparameter_draft(draft: &AnalogSParameterScenarioDraft) -> Result<()> {
    validated_id(&draft.name, "scenario name")?;
    validated_id(&draft.probe_name, "probe name")?;
    if draft.ground_net.trim().is_empty() {
        anyhow::bail!("Ground net must not be blank.");
    }
    if draft.port1_net.trim().is_empty() {
        anyhow::bail!("S-parameter port 1 net must not be blank.");
    }
    if draft.port2_net.trim().is_empty() {
        anyhow::bail!("S-parameter port 2 net must not be blank.");
    }
    if draft.port1_net.trim() == draft.port2_net.trim() {
        anyhow::bail!("S-parameter ports must use distinct nets.");
    }
    validate_frequency_range(
        draft.start_frequency_hz,
        draft.stop_frequency_hz,
        draft.points_per_decade,
        "Analog S-parameter",
    )?;
    if !draft.reference_impedance_ohm.is_finite() || draft.reference_impedance_ohm <= 0.0 {
        anyhow::bail!("S-parameter reference impedance must be finite and positive.");
    }
    Ok(())
}

fn validate_noise_draft(draft: &AnalogNoiseScenarioDraft) -> Result<()> {
    validated_id(&draft.name, "scenario name")?;
    validated_id(&draft.output_probe_name, "output noise probe name")?;
    validated_id(&draft.input_probe_name, "input noise probe name")?;
    validated_id(&draft.input_source, "noise input source")?;
    if draft.ground_net.trim().is_empty() {
        anyhow::bail!("Ground net must not be blank.");
    }
    if draft.probe_net.trim().is_empty() {
        anyhow::bail!("Probe net must not be blank.");
    }
    validate_frequency_range(
        draft.start_frequency_hz,
        draft.stop_frequency_hz,
        draft.points_per_decade,
        "Analog noise",
    )
}

fn validate_harmonic_balance_draft(draft: &AnalogHarmonicBalanceScenarioDraft) -> Result<()> {
    validated_id(&draft.name, "scenario name")?;
    validated_id(&draft.probe_name, "probe name")?;
    validated_id(&draft.drive_source, "harmonic-balance drive source")?;
    if draft.ground_net.trim().is_empty() {
        anyhow::bail!("Ground net must not be blank.");
    }
    if draft.probe_net.trim().is_empty() {
        anyhow::bail!("Probe net must not be blank.");
    }
    if !draft.fundamental_frequency_hz.is_finite()
        || draft.fundamental_frequency_hz <= 0.0
        || !(1..=1024).contains(&draft.harmonics)
    {
        anyhow::bail!(
            "Harmonic-balance fundamental frequency must be finite and positive, and harmonics must be in 1..=1024."
        );
    }
    Ok(())
}

fn validate_fourier_draft(draft: &AnalogFourierScenarioDraft) -> Result<()> {
    validated_id(&draft.name, "scenario name")?;
    validated_id(&draft.probe_name, "probe name")?;
    if draft.ground_net.trim().is_empty() {
        anyhow::bail!("Ground net must not be blank.");
    }
    if draft.probe_net.trim().is_empty() {
        anyhow::bail!("Probe net must not be blank.");
    }
    if !draft.stop_time_us.is_finite()
        || !draft.max_step_us.is_finite()
        || draft.stop_time_us <= 0.0
        || draft.max_step_us <= 0.0
        || draft.max_step_us > draft.stop_time_us
        || !draft.fundamental_frequency_hz.is_finite()
        || draft.fundamental_frequency_hz <= 0.0
        || !(1..=1024).contains(&draft.harmonics)
    {
        anyhow::bail!(
            "Fourier stop time, max step, and fundamental frequency must be finite positive values, max step must not exceed stop time, and harmonics must be in 1..=1024."
        );
    }
    let period_us = 1.0e6 / draft.fundamental_frequency_hz;
    if draft.stop_time_us < period_us {
        anyhow::bail!("Fourier stop time must cover at least one fundamental period.");
    }
    Ok(())
}

fn validate_frequency_range(
    start_frequency_hz: f64,
    stop_frequency_hz: f64,
    points_per_decade: u32,
    label: &str,
) -> Result<()> {
    if !start_frequency_hz.is_finite()
        || !stop_frequency_hz.is_finite()
        || start_frequency_hz <= 0.0
        || stop_frequency_hz <= start_frequency_hz
        || points_per_decade == 0
        || points_per_decade > 1000
    {
        anyhow::bail!(
            "{label} start and stop frequencies must be finite and positive, stop must exceed start, and points per decade must be in 1..=1000."
        );
    }
    Ok(())
}

fn validated_id<'a>(value: &'a str, label: &str) -> Result<&'a str> {
    let value = value.trim();
    if value.is_empty() {
        anyhow::bail!("{label} must not be blank.");
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.'))
    {
        anyhow::bail!("{label} {value} contains unsupported characters.");
    }
    Ok(value)
}

fn trim_nonempty_values(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .collect()
}

#[cfg(test)]
mod core_run_setup_tests;

#[cfg(test)]
mod planned_run_setup_tests;

#[cfg(test)]
mod sparameter_run_setup_tests;
