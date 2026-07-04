use serde::Deserialize;

use super::AnalogRelation;

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogMeasureAssertion {
    pub name: String,
    pub measurement: String,
    pub relation: AnalogRelation,
    pub threshold: f64,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub suggested_fixes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogDistortionAssertion {
    pub name: String,
    pub component: String,
    pub relation: AnalogRelation,
    pub threshold: f64,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub suggested_fixes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogTransferFunctionAssertion {
    pub name: String,
    pub metric: AnalogTransferFunctionMetric,
    pub relation: AnalogRelation,
    pub threshold: f64,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub suggested_fixes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalogTransferFunctionMetric {
    TransferFunctionGain,
    InputResistanceOhm,
    OutputResistanceOhm,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogSParameterAssertion {
    pub name: String,
    pub parameter: String,
    pub metric: AnalogSParameterMetric,
    pub aggregation: AnalogSParameterAggregation,
    pub relation: AnalogRelation,
    pub threshold: f64,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub suggested_fixes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalogSParameterMetric {
    MagnitudeDb,
    MagnitudeLinear,
    ReturnLossDb,
    InsertionLossDb,
    Vswr,
    GroupDelayS,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalogSParameterAggregation {
    Min,
    Max,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogSParameterNetworkAssertion {
    pub name: String,
    pub metric: AnalogSParameterNetworkMetric,
    pub relation: AnalogRelation,
    pub threshold: f64,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub suggested_fixes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalogSParameterNetworkMetric {
    ReciprocityErrorLinear,
    PassivityMaxSingularValue,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogFourierAssertion {
    pub name: String,
    #[serde(default)]
    pub harmonic: Option<u32>,
    pub metric: AnalogFourierMetric,
    pub relation: AnalogRelation,
    pub threshold: f64,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub suggested_fixes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogPoleZeroAssertion {
    pub name: String,
    pub root_kind: AnalogPoleZeroRootKind,
    #[serde(default)]
    pub root_index: Option<u32>,
    pub metric: AnalogPoleZeroMetric,
    pub relation: AnalogRelation,
    pub threshold: f64,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub suggested_fixes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogSensitivityAssertion {
    pub name: String,
    pub parameter: String,
    #[serde(default)]
    pub frequency_hz: Option<f64>,
    pub metric: AnalogSensitivityMetric,
    pub relation: AnalogRelation,
    pub threshold: f64,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub suggested_fixes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalogSensitivityMetric {
    SensitivityReal,
    SensitivityImaginary,
    SensitivityMagnitude,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum AnalogPoleZeroRootKind {
    Pole,
    Zero,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalogPoleZeroMetric {
    RealRadPerS,
    ImaginaryRadPerS,
    FrequencyHz,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalogFourierMetric {
    Magnitude,
    NormalizedMagnitude,
    PhaseDeg,
    NormalizedPhaseDeg,
    ThdPercent,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogDcSweepAssertion {
    pub name: String,
    pub probe: String,
    pub aggregation: AnalogDcSweepAggregation,
    pub relation: AnalogRelation,
    pub threshold: f64,
    #[serde(default)]
    pub at_sweep_value: Option<f64>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub suggested_fixes: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnalogDcSweepAggregation {
    Min,
    Max,
    Mean,
    Sample,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnalogSParameterPort {
    pub name: String,
    pub positive_node: String,
    pub negative_node: String,
    pub reference_impedance_ohm: f64,
}
