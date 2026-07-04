use anyhow::Result;
use std::path::Path;

use super::{
    GeneratedAnalogScenarioKind, append_generated_analog_scenario_with_project_path,
    validate_frequency_range, validated_id,
};

#[derive(Debug, Clone)]
pub(in crate::gui) struct AnalogPssScenarioDraft {
    pub(in crate::gui) name: String,
    pub(in crate::gui) ground_net: String,
    pub(in crate::gui) probe_net: String,
    pub(in crate::gui) probe_name: String,
    pub(in crate::gui) mode: String,
    pub(in crate::gui) frequency_guess_hz: f64,
    pub(in crate::gui) stabilization_time_us: f64,
    pub(in crate::gui) periods: u32,
    pub(in crate::gui) drive_source: String,
}

#[derive(Debug, Clone)]
pub(in crate::gui) struct AnalogPhaseNoiseScenarioDraft {
    pub(in crate::gui) name: String,
    pub(in crate::gui) ground_net: String,
    pub(in crate::gui) probe_net: String,
    pub(in crate::gui) probe_name: String,
    pub(in crate::gui) mode: String,
    pub(in crate::gui) carrier_frequency_hz: f64,
    pub(in crate::gui) offset_start_hz: f64,
    pub(in crate::gui) offset_stop_hz: f64,
    pub(in crate::gui) points_per_decade: u32,
    pub(in crate::gui) drive_source: String,
}

#[derive(Debug, Clone)]
pub(in crate::gui) struct AnalogPeriodicAcScenarioDraft {
    pub(in crate::gui) name: String,
    pub(in crate::gui) ground_net: String,
    pub(in crate::gui) probe_net: String,
    pub(in crate::gui) probe_name: String,
    pub(in crate::gui) mode: String,
    pub(in crate::gui) carrier_frequency_hz: f64,
    pub(in crate::gui) start_frequency_hz: f64,
    pub(in crate::gui) stop_frequency_hz: f64,
    pub(in crate::gui) points_per_decade: u32,
    pub(in crate::gui) input_source: String,
    pub(in crate::gui) sidebands: u32,
    pub(in crate::gui) drive_source: String,
}

pub(in crate::gui) fn append_analog_pss_scenario_with_project_path(
    text: &str,
    project_path: &Path,
    draft: &AnalogPssScenarioDraft,
) -> Result<String> {
    validate_pss_draft(draft)?;
    append_generated_analog_scenario_with_project_path(
        text,
        project_path,
        &draft.name,
        &draft.ground_net,
        &draft.probe_net,
        &draft.probe_name,
        GeneratedAnalogScenarioKind::Pss {
            mode: draft.mode.trim().to_string(),
            frequency_guess_hz: draft.frequency_guess_hz,
            stabilization_time_us: draft.stabilization_time_us,
            periods: draft.periods,
            drive_source: optional_drive_source(&draft.drive_source),
        },
    )
}

pub(in crate::gui) fn append_analog_phase_noise_scenario_with_project_path(
    text: &str,
    project_path: &Path,
    draft: &AnalogPhaseNoiseScenarioDraft,
) -> Result<String> {
    validate_phase_noise_draft(draft)?;
    append_generated_analog_scenario_with_project_path(
        text,
        project_path,
        &draft.name,
        &draft.ground_net,
        &draft.probe_net,
        &draft.probe_name,
        GeneratedAnalogScenarioKind::PhaseNoise {
            mode: draft.mode.trim().to_string(),
            carrier_frequency_hz: draft.carrier_frequency_hz,
            offset_start_hz: draft.offset_start_hz,
            offset_stop_hz: draft.offset_stop_hz,
            points_per_decade: draft.points_per_decade,
            drive_source: optional_drive_source(&draft.drive_source),
        },
    )
}

pub(in crate::gui) fn append_analog_periodic_ac_scenario_with_project_path(
    text: &str,
    project_path: &Path,
    draft: &AnalogPeriodicAcScenarioDraft,
) -> Result<String> {
    validate_periodic_ac_draft(draft)?;
    append_generated_analog_scenario_with_project_path(
        text,
        project_path,
        &draft.name,
        &draft.ground_net,
        &draft.probe_net,
        &draft.probe_name,
        GeneratedAnalogScenarioKind::PeriodicAc {
            mode: draft.mode.trim().to_string(),
            carrier_frequency_hz: draft.carrier_frequency_hz,
            start_frequency_hz: draft.start_frequency_hz,
            stop_frequency_hz: draft.stop_frequency_hz,
            points_per_decade: draft.points_per_decade,
            input_source: draft.input_source.trim().to_string(),
            sidebands: draft.sidebands,
            drive_source: optional_drive_source(&draft.drive_source),
        },
    )
}

fn validate_pss_draft(draft: &AnalogPssScenarioDraft) -> Result<()> {
    validated_id(&draft.name, "scenario name")?;
    validated_id(&draft.probe_name, "probe name")?;
    validate_periodic_output_nets(&draft.ground_net, &draft.probe_net)?;
    match draft.mode.trim() {
        "driven" => {
            validated_id(&draft.drive_source, "PSS drive source")?;
        }
        "autonomous" => {
            if !draft.drive_source.trim().is_empty() {
                validated_id(&draft.drive_source, "PSS drive source")?;
            }
        }
        _ => anyhow::bail!("PSS mode must be driven or autonomous."),
    }
    if !draft.frequency_guess_hz.is_finite()
        || draft.frequency_guess_hz <= 0.0
        || !draft.stabilization_time_us.is_finite()
        || draft.stabilization_time_us <= 0.0
        || !(1..=4096).contains(&draft.periods)
    {
        anyhow::bail!(
            "PSS frequency guess and stabilization time must be finite positive values, and periods must be in 1..=4096."
        );
    }
    Ok(())
}

fn validate_phase_noise_draft(draft: &AnalogPhaseNoiseScenarioDraft) -> Result<()> {
    validated_id(&draft.name, "scenario name")?;
    validated_id(&draft.probe_name, "probe name")?;
    validate_periodic_output_nets(&draft.ground_net, &draft.probe_net)?;
    match draft.mode.trim() {
        "driven" => {
            validated_id(&draft.drive_source, "phase-noise drive source")?;
        }
        "autonomous" => {
            if !draft.drive_source.trim().is_empty() {
                validated_id(&draft.drive_source, "phase-noise drive source")?;
            }
        }
        _ => anyhow::bail!("Phase-noise mode must be driven or autonomous."),
    }
    if !draft.carrier_frequency_hz.is_finite() || draft.carrier_frequency_hz <= 0.0 {
        anyhow::bail!("Phase-noise carrier frequency must be finite and positive.");
    }
    validate_frequency_range(
        draft.offset_start_hz,
        draft.offset_stop_hz,
        draft.points_per_decade,
        "Phase-noise offset",
    )
}

fn validate_periodic_ac_draft(draft: &AnalogPeriodicAcScenarioDraft) -> Result<()> {
    validated_id(&draft.name, "scenario name")?;
    validated_id(&draft.probe_name, "probe name")?;
    validated_id(&draft.input_source, "periodic AC input source")?;
    if !draft.drive_source.trim().is_empty() {
        validated_id(&draft.drive_source, "periodic AC drive source")?;
    }
    validate_periodic_output_nets(&draft.ground_net, &draft.probe_net)?;
    if !matches!(draft.mode.trim(), "pac" | "pxf") {
        anyhow::bail!("Periodic AC mode must be pac or pxf.");
    }
    if !draft.carrier_frequency_hz.is_finite() || draft.carrier_frequency_hz <= 0.0 {
        anyhow::bail!("Periodic AC carrier frequency must be finite and positive.");
    }
    validate_frequency_range(
        draft.start_frequency_hz,
        draft.stop_frequency_hz,
        draft.points_per_decade,
        "Periodic AC",
    )?;
    if draft.sidebands > 1024 {
        anyhow::bail!("Periodic AC sidebands must be in 0..=1024.");
    }
    Ok(())
}

fn validate_periodic_output_nets(ground_net: &str, probe_net: &str) -> Result<()> {
    if ground_net.trim().is_empty() {
        anyhow::bail!("Ground net must not be blank.");
    }
    if probe_net.trim().is_empty() {
        anyhow::bail!("Probe net must not be blank.");
    }
    Ok(())
}

fn optional_drive_source(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}
