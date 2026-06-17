use crate::board_ir::Scenario;
use crate::library::{BoundBoard, ComponentModel, PortKind};
use crate::reports::Finding;
use serde_json::json;

use super::{
    LOAD_CABLE_CURRENT_VALID, LOAD_CABLE_THERMAL_DERATING_VALID, LOAD_CABLE_VOLTAGE_DROP_VALID,
    LOAD_CONNECTOR_CURRENT_VALID,
};

const VALIDATION_INPUT_MISSING: &str = "VALIDATION_INPUT_MISSING";

pub(super) fn validate_load_connector_current(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(target) = &scenario.target else {
        missing_input(
            scenario,
            "target.component",
            "Set scenario.target.component to the load behind the connector.",
            findings,
        );
        return;
    };
    let Some(power_pin) = target.power_pin.as_deref() else {
        missing_input(
            scenario,
            "target.power_pin",
            "Set scenario.target.power_pin to the load supply pin being connector-rated.",
            findings,
        );
        return;
    };
    let Some(load_component) = bound.project.board.components.get(&target.component) else {
        missing_input(
            scenario,
            "target.component",
            "Set scenario.target.component to an existing load component.",
            findings,
        );
        return;
    };
    let Some(load_model) = bound.library.get(&load_component.model) else {
        missing_input(
            scenario,
            "target.component.model",
            "Bind the load component to a component model with power-pin current metadata.",
            findings,
        );
        return;
    };
    let Some(load_port) = load_model.ports.get(power_pin) else {
        missing_input(
            scenario,
            "target.power_pin",
            "Set scenario.target.power_pin to a pin declared by the target component model.",
            findings,
        );
        return;
    };
    if load_port.kind != PortKind::ElectricalPower {
        missing_input(
            scenario,
            "target.power_pin",
            "Set scenario.target.power_pin to an electrical_power port.",
            findings,
        );
        return;
    }
    let Some(load_current_a) = load_port.electrical.max_supply_current_a else {
        missing_input(
            scenario,
            "target.power_pin.max_supply_current_A",
            "Add max_supply_current_A to the target load power pin.",
            findings,
        );
        return;
    };
    if !load_current_a.is_finite() || load_current_a <= 0.0 {
        missing_input(
            scenario,
            "target.power_pin.max_supply_current_A",
            "Set max_supply_current_A to a finite value greater than zero.",
            findings,
        );
        return;
    }

    let connector_evidence = connector_evidence(bound, scenario, findings);
    let Some(connector_current_a) = parameter_or_connector_rating(
        scenario,
        "connector_current_rating_A",
        connector_evidence
            .as_ref()
            .and_then(|(_, model)| model.connector.as_ref()?.current_rating_a),
        findings,
    ) else {
        return;
    };
    let Some(min_margin) =
        optional_margin(scenario, "min_connector_current_margin_ratio", findings)
    else {
        return;
    };

    let load_net = load_component.pins.get(power_pin).cloned();
    let load_voltage_v = load_net
        .as_deref()
        .and_then(|net| bound.project.board.nets.get(net))
        .and_then(|net| net.nominal_voltage);
    let Some(connector_voltage_rating_v) = parameter_or_connector_voltage(
        scenario,
        connector_evidence
            .as_ref()
            .and_then(|(_, model)| model.connector.as_ref()?.voltage_rating_v),
        findings,
    ) else {
        return;
    };
    if let (Some(voltage_v), Some(limit_v)) = (load_voltage_v, connector_voltage_rating_v)
        && voltage_v > limit_v
    {
        let mut finding = Finding::critical(
            LOAD_CONNECTOR_CURRENT_VALID,
            &scenario.name,
            "Load rail nominal voltage exceeds the declared connector voltage rating.",
        );
        finding.component = Some(target.component.clone());
        finding
            .measured
            .insert("load_voltage_V".to_string(), json!(voltage_v));
        finding
            .limit
            .insert("connector_voltage_rating_V".to_string(), json!(limit_v));
        finding.suggested_fixes = vec![
            "Select a connector with a higher voltage rating.".to_string(),
            "Lower the load rail voltage or split the connector by voltage domain.".to_string(),
        ];
        findings.push(finding);
    }

    let required_connector_current_a = load_current_a * min_margin;
    if required_connector_current_a > connector_current_a {
        let mut finding = Finding::critical(
            LOAD_CONNECTOR_CURRENT_VALID,
            &scenario.name,
            format!(
                "Load current {:.6} A with {:.3}x margin exceeds {:.6} A connector rating.",
                load_current_a, min_margin, connector_current_a
            ),
        );
        finding.component = Some(target.component.clone());
        if let Some((component_id, _)) = &connector_evidence {
            finding
                .measured
                .insert("connector_component".to_string(), json!(component_id));
        }
        if let Some(net) = load_net {
            finding.measured.insert("load_net".to_string(), json!(net));
        }
        finding
            .measured
            .insert("load_current_A".to_string(), json!(load_current_a));
        finding.limit.insert(
            "required_connector_current_A".to_string(),
            json!(required_connector_current_a),
        );
        finding.limit.insert(
            "connector_current_rating_A".to_string(),
            json!(connector_current_a),
        );
        finding.limit.insert(
            "min_connector_current_margin_ratio".to_string(),
            json!(min_margin),
        );
        finding.suggested_fixes = vec![
            "Select a higher-current connector and matching wire gauge.".to_string(),
            "Lower the load current budget or split the load across multiple connectors."
                .to_string(),
            "Validate connector temperature rise for the selected cable and duty cycle."
                .to_string(),
        ];
        findings.push(finding);
    }
}

pub(super) fn validate_load_cable_current(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(target) = &scenario.target else {
        missing_input(
            scenario,
            "target.component",
            "Set scenario.target.component to the load behind the cable assembly.",
            findings,
        );
        return;
    };
    let Some(power_pin) = target.power_pin.as_deref() else {
        missing_input(
            scenario,
            "target.power_pin",
            "Set scenario.target.power_pin to the load supply pin being cable-rated.",
            findings,
        );
        return;
    };
    let Some(load_component) = bound.project.board.components.get(&target.component) else {
        missing_input(
            scenario,
            "target.component",
            "Set scenario.target.component to an existing load component.",
            findings,
        );
        return;
    };
    let Some(load_model) = bound.library.get(&load_component.model) else {
        missing_input(
            scenario,
            "target.component.model",
            "Bind the load component to a component model with power-pin current metadata.",
            findings,
        );
        return;
    };
    let Some(load_port) = load_model.ports.get(power_pin) else {
        missing_input(
            scenario,
            "target.power_pin",
            "Set scenario.target.power_pin to a pin declared by the target component model.",
            findings,
        );
        return;
    };
    if load_port.kind != PortKind::ElectricalPower {
        missing_input(
            scenario,
            "target.power_pin",
            "Set scenario.target.power_pin to an electrical_power port.",
            findings,
        );
        return;
    }
    let Some(load_current_a) = load_port.electrical.max_supply_current_a else {
        missing_input(
            scenario,
            "target.power_pin.max_supply_current_A",
            "Add max_supply_current_A to the target load power pin.",
            findings,
        );
        return;
    };
    if !load_current_a.is_finite() || load_current_a <= 0.0 {
        missing_input(
            scenario,
            "target.power_pin.max_supply_current_A",
            "Set max_supply_current_A to a finite value greater than zero.",
            findings,
        );
        return;
    }

    let cable_evidence = cable_evidence(bound, scenario, findings);
    let Some(cable_current_a) = parameter_or_cable_rating(
        scenario,
        cable_evidence
            .as_ref()
            .and_then(|(_, model)| model.cable_assembly.as_ref()?.current_rating_a),
        findings,
    ) else {
        return;
    };
    let Some(min_margin) = optional_margin(scenario, "min_cable_current_margin_ratio", findings)
    else {
        return;
    };

    let load_net = load_component.pins.get(power_pin).cloned();
    let load_voltage_v = load_net
        .as_deref()
        .and_then(|net| bound.project.board.nets.get(net))
        .and_then(|net| net.nominal_voltage);
    let Some(cable_voltage_rating_v) = parameter_or_cable_voltage(
        scenario,
        cable_evidence
            .as_ref()
            .and_then(|(_, model)| model.cable_assembly.as_ref()?.voltage_rating_v),
        findings,
    ) else {
        return;
    };
    if let (Some(voltage_v), Some(limit_v)) = (load_voltage_v, cable_voltage_rating_v)
        && voltage_v > limit_v
    {
        let mut finding = Finding::critical(
            LOAD_CABLE_CURRENT_VALID,
            &scenario.name,
            "Load rail nominal voltage exceeds the declared cable assembly voltage rating.",
        );
        finding.component = Some(target.component.clone());
        finding
            .measured
            .insert("load_voltage_V".to_string(), json!(voltage_v));
        finding
            .limit
            .insert("cable_voltage_rating_V".to_string(), json!(limit_v));
        finding.suggested_fixes = vec![
            "Select a cable assembly with a higher voltage rating.".to_string(),
            "Lower the load rail voltage or split the cable by voltage domain.".to_string(),
        ];
        findings.push(finding);
    }

    let required_cable_current_a = load_current_a * min_margin;
    if required_cable_current_a > cable_current_a {
        let mut finding = Finding::critical(
            LOAD_CABLE_CURRENT_VALID,
            &scenario.name,
            format!(
                "Load current {:.6} A with {:.3}x margin exceeds {:.6} A cable assembly rating.",
                load_current_a, min_margin, cable_current_a
            ),
        );
        finding.component = Some(target.component.clone());
        if let Some((component_id, _)) = &cable_evidence {
            finding
                .measured
                .insert("cable_component".to_string(), json!(component_id));
        }
        if let Some(net) = load_net {
            finding.measured.insert("load_net".to_string(), json!(net));
        }
        finding
            .measured
            .insert("load_current_A".to_string(), json!(load_current_a));
        finding.limit.insert(
            "required_cable_current_A".to_string(),
            json!(required_cable_current_a),
        );
        finding
            .limit
            .insert("cable_current_rating_A".to_string(), json!(cable_current_a));
        finding.limit.insert(
            "min_cable_current_margin_ratio".to_string(),
            json!(min_margin),
        );
        finding.suggested_fixes = vec![
            "Select a higher-current cable assembly with source-backed wire and crimp ratings."
                .to_string(),
            "Lower the load current budget or split the load across multiple cable contacts."
                .to_string(),
            "Validate cable temperature rise for the selected duty cycle and harness routing."
                .to_string(),
        ];
        findings.push(finding);
    }
}

pub(super) fn validate_load_cable_thermal_derating(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(target) = &scenario.target else {
        missing_input(
            scenario,
            "target.component",
            "Set scenario.target.component to the load behind the cable assembly.",
            findings,
        );
        return;
    };
    let Some(power_pin) = target.power_pin.as_deref() else {
        missing_input(
            scenario,
            "target.power_pin",
            "Set scenario.target.power_pin to the load supply pin being cable-derated.",
            findings,
        );
        return;
    };
    let Some(load_component) = bound.project.board.components.get(&target.component) else {
        missing_input(
            scenario,
            "target.component",
            "Set scenario.target.component to an existing load component.",
            findings,
        );
        return;
    };
    let Some(load_model) = bound.library.get(&load_component.model) else {
        missing_input(
            scenario,
            "target.component.model",
            "Bind the load component to a component model with power-pin current metadata.",
            findings,
        );
        return;
    };
    let Some(load_port) = load_model.ports.get(power_pin) else {
        missing_input(
            scenario,
            "target.power_pin",
            "Set scenario.target.power_pin to a pin declared by the target component model.",
            findings,
        );
        return;
    };
    if load_port.kind != PortKind::ElectricalPower {
        missing_input(
            scenario,
            "target.power_pin",
            "Set scenario.target.power_pin to an electrical_power port.",
            findings,
        );
        return;
    }
    let Some(load_current_a) = load_port.electrical.max_supply_current_a else {
        missing_input(
            scenario,
            "target.power_pin.max_supply_current_A",
            "Add max_supply_current_A to the target load power pin.",
            findings,
        );
        return;
    };
    if !load_current_a.is_finite() || load_current_a <= 0.0 {
        missing_input(
            scenario,
            "target.power_pin.max_supply_current_A",
            "Set max_supply_current_A to a finite value greater than zero.",
            findings,
        );
        return;
    }

    let cable_evidence = cable_evidence(bound, scenario, findings);
    let Some(test_current_a) = parameter_or_cable_thermal_value(
        scenario,
        "cable_temperature_rise_test_current_A",
        "temperature_rise_test_current_A",
        cable_evidence.as_ref().and_then(|(_, model)| {
            model
                .cable_assembly
                .as_ref()?
                .temperature_rise_test_current_a
        }),
        findings,
    ) else {
        return;
    };
    let Some(test_rise_c) = parameter_or_cable_thermal_value(
        scenario,
        "cable_temperature_rise_at_test_current_C",
        "temperature_rise_at_test_current_C",
        cable_evidence.as_ref().and_then(|(_, model)| {
            model
                .cable_assembly
                .as_ref()?
                .temperature_rise_at_test_current_c
        }),
        findings,
    ) else {
        return;
    };
    let Some(max_rise_c) = parameter_or_cable_thermal_value(
        scenario,
        "max_cable_temperature_rise_C",
        "max_temperature_rise_C",
        cable_evidence
            .as_ref()
            .and_then(|(_, model)| model.cable_assembly.as_ref()?.max_temperature_rise_c),
        findings,
    ) else {
        return;
    };
    let Some(min_margin) = optional_margin(scenario, "thermal_current_margin_ratio", findings)
    else {
        return;
    };

    let thermal_current_a = load_current_a * min_margin;
    let estimated_rise_c = test_rise_c * (thermal_current_a / test_current_a).powi(2);
    if estimated_rise_c > max_rise_c {
        let mut finding = Finding::critical(
            LOAD_CABLE_THERMAL_DERATING_VALID,
            &scenario.name,
            format!(
                "Estimated cable temperature rise {:.6} C at {:.6} A exceeds {:.6} C limit.",
                estimated_rise_c, thermal_current_a, max_rise_c
            ),
        );
        finding.component = Some(target.component.clone());
        if let Some((component_id, _)) = &cable_evidence {
            finding
                .measured
                .insert("cable_component".to_string(), json!(component_id));
        }
        if let Some(net) = load_component.pins.get(power_pin) {
            finding.measured.insert("load_net".to_string(), json!(net));
        }
        finding
            .measured
            .insert("load_current_A".to_string(), json!(load_current_a));
        finding
            .measured
            .insert("thermal_current_A".to_string(), json!(thermal_current_a));
        finding.measured.insert(
            "temperature_rise_test_current_A".to_string(),
            json!(test_current_a),
        );
        finding.measured.insert(
            "temperature_rise_at_test_current_C".to_string(),
            json!(test_rise_c),
        );
        finding.measured.insert(
            "estimated_temperature_rise_C".to_string(),
            json!(estimated_rise_c),
        );
        finding.limit.insert(
            "max_cable_temperature_rise_C".to_string(),
            json!(max_rise_c),
        );
        finding.limit.insert(
            "thermal_current_margin_ratio".to_string(),
            json!(min_margin),
        );
        finding.suggested_fixes = vec![
            "Select a cable assembly with lower thermal rise at the required current."
                .to_string(),
            "Reduce the load current, use more conductors, or split the load across another harness."
                .to_string(),
            "Add measured harness temperature-rise evidence for the final routing and duty cycle."
                .to_string(),
        ];
        findings.push(finding);
    }
}

pub(super) fn validate_load_cable_voltage_drop(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(target) = &scenario.target else {
        missing_input(
            scenario,
            "target.component",
            "Set scenario.target.component to the load behind the cable assembly.",
            findings,
        );
        return;
    };
    let Some(power_pin) = target.power_pin.as_deref() else {
        missing_input(
            scenario,
            "target.power_pin",
            "Set scenario.target.power_pin to the load supply pin being cable-drop-rated.",
            findings,
        );
        return;
    };
    let Some(load_component) = bound.project.board.components.get(&target.component) else {
        missing_input(
            scenario,
            "target.component",
            "Set scenario.target.component to an existing load component.",
            findings,
        );
        return;
    };
    let Some(load_model) = bound.library.get(&load_component.model) else {
        missing_input(
            scenario,
            "target.component.model",
            "Bind the load component to a component model with power-pin current metadata.",
            findings,
        );
        return;
    };
    let Some(load_port) = load_model.ports.get(power_pin) else {
        missing_input(
            scenario,
            "target.power_pin",
            "Set scenario.target.power_pin to a pin declared by the target component model.",
            findings,
        );
        return;
    };
    if load_port.kind != PortKind::ElectricalPower {
        missing_input(
            scenario,
            "target.power_pin",
            "Set scenario.target.power_pin to an electrical_power port.",
            findings,
        );
        return;
    }
    let Some(load_current_a) = load_port.electrical.max_supply_current_a else {
        missing_input(
            scenario,
            "target.power_pin.max_supply_current_A",
            "Add max_supply_current_A to the target load power pin.",
            findings,
        );
        return;
    };
    if !load_current_a.is_finite() || load_current_a <= 0.0 {
        missing_input(
            scenario,
            "target.power_pin.max_supply_current_A",
            "Set max_supply_current_A to a finite value greater than zero.",
            findings,
        );
        return;
    }

    let cable_evidence = cable_evidence(bound, scenario, findings);
    let Some(loop_resistance_ohm) = parameter_or_cable_drop_value(
        scenario,
        "cable_loop_resistance_ohm",
        "loop_resistance_ohm",
        cable_evidence
            .as_ref()
            .and_then(|(_, model)| model.cable_assembly.as_ref()?.loop_resistance_ohm),
        findings,
    ) else {
        return;
    };
    let Some(max_drop_v) = parameter_or_cable_drop_value(
        scenario,
        "max_cable_voltage_drop_V",
        "max_voltage_drop_V",
        cable_evidence
            .as_ref()
            .and_then(|(_, model)| model.cable_assembly.as_ref()?.max_voltage_drop_v),
        findings,
    ) else {
        return;
    };
    let Some(min_margin) = optional_margin(scenario, "drop_current_margin_ratio", findings) else {
        return;
    };
    let max_power_loss_w = optional_parameter_or_cable_drop_value(
        scenario,
        "max_cable_power_loss_W",
        cable_evidence
            .as_ref()
            .and_then(|(_, model)| model.cable_assembly.as_ref()?.max_power_loss_w),
        findings,
    );
    let Some(max_power_loss_w) = max_power_loss_w else {
        return;
    };

    let drop_current_a = load_current_a * min_margin;
    let voltage_drop_v = drop_current_a * loop_resistance_ohm;
    let power_loss_w = drop_current_a.powi(2) * loop_resistance_ohm;
    let voltage_failed = voltage_drop_v > max_drop_v;
    let power_failed = max_power_loss_w.is_some_and(|limit_w| power_loss_w > limit_w);
    if voltage_failed || power_failed {
        let mut finding = Finding::critical(
            LOAD_CABLE_VOLTAGE_DROP_VALID,
            &scenario.name,
            format!(
                "Estimated cable drop {:.6} V and loss {:.6} W at {:.6} A exceed declared harness limits.",
                voltage_drop_v, power_loss_w, drop_current_a
            ),
        );
        finding.component = Some(target.component.clone());
        if let Some((component_id, _)) = &cable_evidence {
            finding
                .measured
                .insert("cable_component".to_string(), json!(component_id));
        }
        if let Some(net) = load_component.pins.get(power_pin) {
            finding.measured.insert("load_net".to_string(), json!(net));
        }
        finding
            .measured
            .insert("load_current_A".to_string(), json!(load_current_a));
        finding
            .measured
            .insert("drop_current_A".to_string(), json!(drop_current_a));
        finding.measured.insert(
            "cable_loop_resistance_ohm".to_string(),
            json!(loop_resistance_ohm),
        );
        finding.measured.insert(
            "estimated_voltage_drop_V".to_string(),
            json!(voltage_drop_v),
        );
        finding
            .measured
            .insert("estimated_power_loss_W".to_string(), json!(power_loss_w));
        finding
            .limit
            .insert("max_cable_voltage_drop_V".to_string(), json!(max_drop_v));
        if let Some(limit_w) = max_power_loss_w {
            finding
                .limit
                .insert("max_cable_power_loss_W".to_string(), json!(limit_w));
        }
        finding
            .limit
            .insert("drop_current_margin_ratio".to_string(), json!(min_margin));
        finding.suggested_fixes = vec![
            "Select a lower-resistance cable assembly or use more conductors in parallel."
                .to_string(),
            "Relax the voltage-drop budget only if the downstream load minimum voltage still passes."
                .to_string(),
            "Add measured end-to-end harness resistance or voltage-drop evidence for the final cable length."
                .to_string(),
        ];
        findings.push(finding);
    }
}

fn connector_evidence<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> Option<(String, &'a ComponentModel)> {
    let raw = scenario.parameters.get("connector_component")?;
    let Some(component_id) = raw.as_str() else {
        missing_input(
            scenario,
            "connector_component",
            "Set load_budget parameters.connector_component to a component id string.",
            findings,
        );
        return None;
    };
    let Some(component) = bound.project.board.components.get(component_id) else {
        missing_input(
            scenario,
            "connector_component",
            "Set parameters.connector_component to an existing connector component.",
            findings,
        );
        return None;
    };
    let Some(model) = bound.library.get(&component.model) else {
        missing_input(
            scenario,
            "connector_component.model",
            "Bind the connector component to a component model with connector metadata.",
            findings,
        );
        return None;
    };
    Some((component_id.to_string(), model))
}

fn cable_evidence<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> Option<(String, &'a ComponentModel)> {
    let raw = scenario.parameters.get("cable_component")?;
    let Some(component_id) = raw.as_str() else {
        missing_input(
            scenario,
            "cable_component",
            "Set load_budget parameters.cable_component to a component id string.",
            findings,
        );
        return None;
    };
    let Some(component) = bound.project.board.components.get(component_id) else {
        missing_input(
            scenario,
            "cable_component",
            "Set parameters.cable_component to an existing cable assembly component.",
            findings,
        );
        return None;
    };
    let Some(model) = bound.library.get(&component.model) else {
        missing_input(
            scenario,
            "cable_component.model",
            "Bind the cable assembly component to a model with cable_assembly metadata.",
            findings,
        );
        return None;
    };
    Some((component_id.to_string(), model))
}

fn parameter_or_connector_rating(
    scenario: &Scenario,
    name: &str,
    connector_value: Option<f64>,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    if let Some(raw) = scenario.parameters.get(name) {
        return positive_parameter(scenario, name, raw, findings);
    }
    let Some(value) = connector_value else {
        missing_input(
            scenario,
            name,
            "Add load_budget parameters.connector_current_rating_A, or bind parameters.connector_component to a model with connector.current_rating_A.",
            findings,
        );
        return None;
    };
    if value.is_finite() && value > 0.0 {
        Some(value)
    } else {
        missing_input(
            scenario,
            "connector.current_rating_A",
            "Set connector.current_rating_A to a finite value greater than zero.",
            findings,
        );
        None
    }
}

fn parameter_or_connector_voltage(
    scenario: &Scenario,
    connector_value: Option<f64>,
    findings: &mut Vec<Finding>,
) -> Option<Option<f64>> {
    if let Some(raw) = scenario.parameters.get("connector_voltage_rating_V") {
        return positive_parameter(scenario, "connector_voltage_rating_V", raw, findings).map(Some);
    }
    Some(connector_value.filter(|value| value.is_finite() && *value > 0.0))
}

fn parameter_or_cable_rating(
    scenario: &Scenario,
    cable_value: Option<f64>,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    if let Some(raw) = scenario.parameters.get("cable_current_rating_A") {
        return positive_parameter(scenario, "cable_current_rating_A", raw, findings);
    }
    let Some(value) = cable_value else {
        missing_input(
            scenario,
            "cable_current_rating_A",
            "Add load_budget parameters.cable_current_rating_A, or bind parameters.cable_component to a model with cable_assembly.current_rating_A.",
            findings,
        );
        return None;
    };
    if value.is_finite() && value > 0.0 {
        Some(value)
    } else {
        missing_input(
            scenario,
            "cable_assembly.current_rating_A",
            "Set cable_assembly.current_rating_A to a finite value greater than zero.",
            findings,
        );
        None
    }
}

fn parameter_or_cable_voltage(
    scenario: &Scenario,
    cable_value: Option<f64>,
    findings: &mut Vec<Finding>,
) -> Option<Option<f64>> {
    if let Some(raw) = scenario.parameters.get("cable_voltage_rating_V") {
        return positive_parameter(scenario, "cable_voltage_rating_V", raw, findings).map(Some);
    }
    Some(cable_value.filter(|value| value.is_finite() && *value > 0.0))
}

fn parameter_or_cable_thermal_value(
    scenario: &Scenario,
    parameter_name: &str,
    model_field: &str,
    cable_value: Option<f64>,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    if let Some(raw) = scenario.parameters.get(parameter_name) {
        return positive_parameter(scenario, parameter_name, raw, findings);
    }
    let Some(value) = cable_value else {
        missing_input(
            scenario,
            parameter_name,
            &format!(
                "Add load_budget parameters.{parameter_name}, or bind parameters.cable_component to a model with cable_assembly.{model_field}."
            ),
            findings,
        );
        return None;
    };
    if value.is_finite() && value > 0.0 {
        Some(value)
    } else {
        missing_input(
            scenario,
            &format!("cable_assembly.{model_field}"),
            &format!("Set cable_assembly.{model_field} to a finite value greater than zero."),
            findings,
        );
        None
    }
}

fn parameter_or_cable_drop_value(
    scenario: &Scenario,
    parameter_name: &str,
    model_field: &str,
    cable_value: Option<f64>,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    if let Some(raw) = scenario.parameters.get(parameter_name) {
        return positive_parameter(scenario, parameter_name, raw, findings);
    }
    let Some(value) = cable_value else {
        missing_input(
            scenario,
            parameter_name,
            &format!(
                "Add load_budget parameters.{parameter_name}, or bind parameters.cable_component to a model with cable_assembly.{model_field}."
            ),
            findings,
        );
        return None;
    };
    if value.is_finite() && value > 0.0 {
        Some(value)
    } else {
        missing_input(
            scenario,
            &format!("cable_assembly.{model_field}"),
            &format!("Set cable_assembly.{model_field} to a finite value greater than zero."),
            findings,
        );
        None
    }
}

fn optional_parameter_or_cable_drop_value(
    scenario: &Scenario,
    name: &str,
    cable_value: Option<f64>,
    findings: &mut Vec<Finding>,
) -> Option<Option<f64>> {
    if let Some(raw) = scenario.parameters.get(name) {
        return positive_parameter(scenario, name, raw, findings).map(Some);
    }
    Some(cable_value.filter(|value| value.is_finite() && *value > 0.0))
}

pub(super) fn optional_margin(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    let Some(raw) = scenario.parameters.get(name) else {
        return Some(1.0);
    };
    let Some(value) = raw.as_f64() else {
        missing_input(
            scenario,
            name,
            &format!("Set load_budget parameters.{name} to a number."),
            findings,
        );
        return None;
    };
    if value.is_finite() && value >= 1.0 {
        Some(value)
    } else {
        missing_input(
            scenario,
            name,
            &format!("Set {name} to a finite value at least 1.0."),
            findings,
        );
        None
    }
}

pub(super) fn positive_parameter(
    scenario: &Scenario,
    name: &str,
    raw: &serde_yaml_ng::Value,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    let Some(value) = raw.as_f64() else {
        missing_input(
            scenario,
            name,
            &format!("Set load_budget parameters.{name} to a number."),
            findings,
        );
        return None;
    };
    if value.is_finite() && value > 0.0 {
        Some(value)
    } else {
        missing_input(
            scenario,
            name,
            &format!("Set load_budget parameters.{name} to a finite value greater than zero."),
            findings,
        );
        None
    }
}

pub(super) fn missing_input(
    scenario: &Scenario,
    input: &str,
    fix: &str,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        VALIDATION_INPUT_MISSING,
        &scenario.name,
        format!("Load budget validation requires {input}."),
    );
    finding
        .limit
        .insert("required_input".to_string(), json!(input));
    finding.suggested_fixes = vec![fix.to_string()];
    findings.push(finding);
}
