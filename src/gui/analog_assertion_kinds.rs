use anyhow::Result;

use super::analog::AnalogAssertionDraft;

pub(super) fn validate_assertion_scenario_type(
    draft: &AnalogAssertionDraft,
    scenario_type: &str,
) -> Result<()> {
    let ac_aggregation = is_ac_assertion_aggregation_name(&draft.aggregation);
    match scenario_type {
        "analog_ac" if !ac_aggregation => {
            anyhow::bail!("analog_ac run setups require AC/Bode observation checks.");
        }
        "analog_transient" if ac_aggregation => {
            anyhow::bail!("AC/Bode observation checks require an analog_ac run setup.");
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
    )
}
