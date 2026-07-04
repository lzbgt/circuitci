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
            Self::Transient
                | Self::Ac
                | Self::Dc
                | Self::DcSweep
                | Self::Noise
                | Self::SParameter
                | Self::Sensitivity
                | Self::Fourier
                | Self::HarmonicBalance
        )
    }

    pub(super) fn prefers_auto_xyce(self) -> bool {
        matches!(self, Self::HarmonicBalance)
    }
}
