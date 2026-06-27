use anyhow::Result;

use super::analog::AnalogAssertionDraft;

pub(super) fn validate_assertion_scenario_type(
    draft: &AnalogAssertionDraft,
    scenario_type: &str,
) -> Result<()> {
    let ac_aggregation = is_ac_assertion_aggregation_name(&draft.aggregation);
    let dc_aggregation = is_dc_assertion_aggregation_name(&draft.aggregation);
    let noise_aggregation = is_noise_assertion_aggregation_name(&draft.aggregation);
    match scenario_type {
        "analog_ac" if !ac_aggregation => {
            anyhow::bail!("analog_ac run setups require AC/Bode observation checks.");
        }
        "analog_dc" if !dc_aggregation => {
            anyhow::bail!("analog_dc run setups require DC operating-point observation checks.");
        }
        "analog_noise" if !noise_aggregation => {
            anyhow::bail!("analog_noise run setups require noise observation checks.");
        }
        "analog_transient" if ac_aggregation || dc_aggregation || noise_aggregation => {
            anyhow::bail!(
                "Frequency, DC, and noise observation checks require matching run setup types."
            );
        }
        "analog_ac" if dc_aggregation || noise_aggregation => {
            anyhow::bail!("DC operating-point observation checks require an analog_dc run setup.");
        }
        "analog_dc" if ac_aggregation || noise_aggregation => {
            anyhow::bail!("Noise observation checks require an analog_noise run setup.");
        }
        _ => Ok(()),
    }
}

pub(super) fn is_ac_assertion_aggregation_name(aggregation: &str) -> bool {
    matches!(
        aggregation,
        "gain_db_at_frequency"
            | "phase_deg_at_frequency"
            | "rising_gain_crossing_frequency"
            | "falling_gain_crossing_frequency"
            | "phase_margin_deg"
            | "gain_margin_db"
    )
}

pub(super) fn is_dc_assertion_aggregation_name(aggregation: &str) -> bool {
    aggregation == "operating_point"
}

pub(super) fn is_noise_assertion_aggregation_name(aggregation: &str) -> bool {
    matches!(
        aggregation,
        "output_noise_density_at_frequency"
            | "input_noise_density_at_frequency"
            | "integrated_output_noise"
            | "integrated_input_noise"
    )
}
