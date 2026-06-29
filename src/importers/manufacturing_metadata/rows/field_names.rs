use super::ManufacturingField;

pub(super) fn normalize_field(field: &str) -> Option<ManufacturingField> {
    match normalize_name(field).as_str() {
        "stencilthicknessmm" | "stencilthickness" | "stencilfoilthickness" => {
            Some(ManufacturingField::StencilThicknessMm)
        }
        "mindrilledgeclearancemm"
        | "mindrilledgeclearance"
        | "holetoboardedgeclearance"
        | "minimumholetoboardedgeclearance" => Some(ManufacturingField::MinDrillEdgeClearanceMm),
        "minslotedgeclearancemm" | "minslotedgeclearance" | "slottoboardedgeclearance" => {
            Some(ManufacturingField::MinSlotEdgeClearanceMm)
        }
        "minpastearearatio" | "minimumsolderpastearearatio" => {
            Some(ManufacturingField::MinPasteAreaRatio)
        }
        "maxpastearearatio" | "maximumsolderpastearearatio" => {
            Some(ManufacturingField::MaxPasteAreaRatio)
        }
        "minsolderpastespacingmm" | "minsolderpastespacing" | "minpastespace" => {
            Some(ManufacturingField::MinSolderPasteSpacingMm)
        }
        "maxstitchviadistancemm"
        | "maxstitchviadistance"
        | "maximumstitchviadistance"
        | "stitchviadistance" => Some(ManufacturingField::MaxStitchViaDistanceMm),
        "controlledimpedancenet" | "controlledimpedance" | "impedancenet" => {
            Some(ManufacturingField::ControlledImpedanceNet)
        }
        "controlledimpedancepair"
        | "controlledimpedancedifferentialpair"
        | "differentialimpedancepair"
        | "differentialpairimpedance" => Some(ManufacturingField::ControlledImpedancePair),
        "controlledimpedancecoupon"
        | "impedancecoupon"
        | "fabricatorimpedancecoupon"
        | "couponimpedance" => Some(ManufacturingField::ControlledImpedanceCoupon),
        "controlledimpedancecouponsample"
        | "impedancecouponsample"
        | "fabricatorimpedancecouponsample"
        | "couponimpedancesample" => Some(ManufacturingField::ControlledImpedanceCouponSample),
        "controlledimpedancesolverresult"
        | "impedancesolverresult"
        | "controlledimpedancefieldsolverresult"
        | "fieldsolverimpedance" => Some(ManufacturingField::ControlledImpedanceSolverResult),
        "controlledimpedancesolversample"
        | "impedancesolversample"
        | "controlledimpedancefieldsolversample"
        | "fieldsolverimpedancesample" => Some(ManufacturingField::ControlledImpedanceSolverSample),
        "controlledimpedancesolvermaterialcorner"
        | "impedancesolvermaterialcorner"
        | "controlledimpedancefieldsolvermaterialcorner"
        | "fieldsolvermaterialcorner"
        | "solvermaterialcorner" => {
            Some(ManufacturingField::ControlledImpedanceSolverMaterialCorner)
        }
        "controlledimpedancesolverqualification"
        | "impedancesolverqualification"
        | "controlledimpedancefieldsolverqualification"
        | "fieldsolverqualification"
        | "solverqualification" => Some(ManufacturingField::ControlledImpedanceSolverQualification),
        "controlledimpedancesolvermateriallibrary"
        | "impedancesolvermateriallibrary"
        | "controlledimpedancefieldsolvermateriallibrary"
        | "fieldsolvermateriallibrary"
        | "solvermateriallibrary" => {
            Some(ManufacturingField::ControlledImpedanceSolverMaterialLibrary)
        }
        "controlledimpedancesolvermaterialacceptance"
        | "impedancesolvermaterialacceptance"
        | "controlledimpedancefieldsolvermaterialacceptance"
        | "fieldsolvermaterialacceptance"
        | "solvermaterialacceptance" => {
            Some(ManufacturingField::ControlledImpedanceSolverMaterialAcceptance)
        }
        "controlledimpedancesolvermaterialprocess"
        | "impedancesolvermaterialprocess"
        | "controlledimpedancefieldsolvermaterialprocess"
        | "fieldsolvermaterialprocess"
        | "solvermaterialprocess"
        | "solvermateriallotprocess"
        | "solvermaterialdrift" => {
            Some(ManufacturingField::ControlledImpedanceSolverMaterialProcess)
        }
        "controlledimpedancesolverruntimeallowlist"
        | "impedancesolverruntimeallowlist"
        | "controlledimpedancefieldsolverruntimeallowlist"
        | "fieldsolverruntimeallowlist"
        | "solverruntimeallowlist"
        | "solverruntimeoptions" => {
            Some(ManufacturingField::ControlledImpedanceSolverRuntimeAllowlist)
        }
        "controlledimpedancesolverentitlement"
        | "impedancesolverentitlement"
        | "controlledimpedancefieldsolverentitlement"
        | "fieldsolverentitlement"
        | "solverentitlement"
        | "solverlicense"
        | "solverlicensedfeatures"
        | "solverfeatureentitlement" => {
            Some(ManufacturingField::ControlledImpedanceSolverEntitlement)
        }
        "controlledimpedancesolverexecutionenvironment"
        | "impedancesolverexecutionenvironment"
        | "controlledimpedancefieldsolverexecutionenvironment"
        | "fieldsolverexecutionenvironment"
        | "solverexecutionenvironment"
        | "solverenvironment"
        | "solverenvironmentlock"
        | "solverreproducibility" => {
            Some(ManufacturingField::ControlledImpedanceSolverExecutionEnvironment)
        }
        "controlledimpedancesolverrunlog"
        | "impedancesolverrunlog"
        | "controlledimpedancefieldsolverrunlog"
        | "fieldsolverrunlog"
        | "solverrunlog"
        | "solverrandomseed"
        | "solvernumerictolerance"
        | "solverreproducibilityrun"
        | "solverresidualtrend"
        | "solvermonotonicresidual"
        | "solvermonotonicconvergence"
        | "solverprecisionpolicy"
        | "solvernumericalprecision"
        | "solverroundoffpolicy" => Some(ManufacturingField::ControlledImpedanceSolverRunLog),
        "controlledimpedancesolverrerun"
        | "impedancesolverrerun"
        | "controlledimpedancefieldsolverrerun"
        | "fieldsolverrerun"
        | "solverrerun"
        | "solverdeterministicrerun"
        | "solverreproducibilityrerun" => Some(ManufacturingField::ControlledImpedanceSolverRerun),
        "controlledimpedancesolverconvergencesample"
        | "impedancesolverconvergencesample"
        | "controlledimpedancefieldsolverconvergencesample"
        | "fieldsolverconvergencesample"
        | "solverconvergencesample"
        | "solverconvergencewindow"
        | "solverstoppingcriteria" => {
            Some(ManufacturingField::ControlledImpedanceSolverConvergenceSample)
        }
        "thermalcopper" | "thermalcopperpolicy" | "thermalpolicy" | "thermalcopperarea" => {
            Some(ManufacturingField::ThermalCopper)
        }
        "thermalmeasurement" | "thermalmeasuredtemperature" | "measuredtemperature" => {
            Some(ManufacturingField::ThermalMeasurement)
        }
        "thermalpackage" | "packagethermal" | "componentthermalpackage" => {
            Some(ManufacturingField::ThermalPackage)
        }
        "thermalenvironment" | "operatingthermalenvironment" | "reviewedthermalenvironment" => {
            Some(ManufacturingField::ThermalEnvironment)
        }
        "thermallimit" | "thermallimits" | "temperaturelimit" | "thermaltemperaturelimit" => {
            Some(ManufacturingField::ThermalLimit)
        }
        "stackuplayer" | "stackuplayermetadata" | "boardstackuplayer" => {
            Some(ManufacturingField::StackupLayer)
        }
        "rfantennakeepout" | "antennakeepout" | "rfkeepout" => {
            Some(ManufacturingField::RfAntennaKeepout)
        }
        "rfantennafeedpath" | "antennafeedpath" | "rffeedpath" => {
            Some(ManufacturingField::RfAntennaFeedPath)
        }
        "rfantennamatchingnetwork"
        | "antennamatchingnetwork"
        | "rfmatchingnetwork"
        | "rfantennamatchingtopology"
        | "antennamatchingtopology" => Some(ManufacturingField::RfAntennaMatchingNetwork),
        "rfantennameasurement"
        | "antennas11"
        | "antennareturnloss"
        | "rfmeasurement"
        | "rfantennareturnloss" => Some(ManufacturingField::RfAntennaMeasurement),
        "rfantennaperformancelimit"
        | "antennaperformancelimit"
        | "rfperformancelimit"
        | "rfantennareturnlosslimit"
        | "antennareturnlosslimit" => Some(ManufacturingField::RfAntennaPerformanceLimit),
        "rfantennameasurementcondition"
        | "antennameasurementcondition"
        | "rfmeasurementcondition"
        | "rfantennatestcondition"
        | "antennatestcondition" => Some(ManufacturingField::RfAntennaMeasurementCondition),
        "source" | "evidencesource" => Some(ManufacturingField::Source),
        _ => None,
    }
}

fn normalize_name(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}
