#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AnalogRuntimeFeature {
    Transient,
    Ac,
    Dc,
    DcSweep,
    Noise,
    SParameter,
    TransferFunction,
    PoleZero,
    Sensitivity,
    Distortion,
    Fourier,
    HarmonicBalance,
    PeriodicSteadyState,
    PhaseNoise,
    PeriodicAc,
    Measure,
}

impl AnalogRuntimeFeature {
    pub(super) fn supports_embedded_ngspice(self) -> bool {
        matches!(self, Self::Transient)
    }

    pub(super) fn supports_auto_xyce(self) -> bool {
        matches!(
            self,
            Self::Ac | Self::Dc | Self::DcSweep | Self::Noise | Self::SParameter
        )
    }
}
