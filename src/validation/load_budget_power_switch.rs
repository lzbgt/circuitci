use crate::board_ir::Scenario;
use crate::library::{BoundBoard, ComponentModel, PortKind};
use crate::reports::Finding;
use serde_json::json;

use super::{
    POWER_SWITCH_BUDGET_VALID, POWER_SWITCH_INRUSH_VALID, POWER_SWITCH_REVERSE_CURRENT_VALID,
    load_budget::{missing_input, optional_margin, positive_parameter},
};

pub(super) fn validate_power_switch_budget(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(target) = &scenario.target else {
        missing_input(
            scenario,
            "target.component",
            "Set scenario.target.component to the switched load behind the power switch.",
            findings,
        );
        return;
    };
    let Some(power_pin) = target.power_pin.as_deref() else {
        missing_input(
            scenario,
            "target.power_pin",
            "Set scenario.target.power_pin to the switched load supply pin.",
            findings,
        );
        return;
    };
    let Some(load_component) = bound.project.board.components.get(&target.component) else {
        missing_input(
            scenario,
            "target.component",
            "Set scenario.target.component to an existing switched load component.",
            findings,
        );
        return;
    };
    let Some(load_model) = bound.library.get(&load_component.model) else {
        missing_input(
            scenario,
            "target.component.model",
            "Bind the switched load to a component model with power-pin current metadata.",
            findings,
        );
        return;
    };
    let Some(load_port) = load_model.ports.get(power_pin) else {
        missing_input(
            scenario,
            "target.power_pin",
            "Set scenario.target.power_pin to a pin declared by the switched load model.",
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
            "Add max_supply_current_A to the switched load power pin.",
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

    let Some((switch_component_id, switch_model)) =
        power_switch_evidence(bound, scenario, findings)
    else {
        return;
    };
    let Some(power_switch) = switch_model.power_switch.as_ref() else {
        missing_input(
            scenario,
            "switch_component.power_switch",
            "Bind parameters.switch_component to a model with power_switch metadata.",
            findings,
        );
        return;
    };
    let Some(switch_component) = bound.project.board.components.get(&switch_component_id) else {
        missing_input(
            scenario,
            "switch_component",
            "Set parameters.switch_component to an existing power-switch component.",
            findings,
        );
        return;
    };
    let Some(input_net) = switch_component.pins.get(&power_switch.input_pin) else {
        missing_input(
            scenario,
            "switch_component.input_pin",
            "Connect the power switch input pin to the upstream rail.",
            findings,
        );
        return;
    };
    let Some(output_net) = switch_component.pins.get(&power_switch.output_pin) else {
        missing_input(
            scenario,
            "switch_component.output_pin",
            "Connect the power switch output pin to the switched load rail.",
            findings,
        );
        return;
    };
    let Some(load_net) = load_component.pins.get(power_pin) else {
        missing_input(
            scenario,
            "target.power_pin.net",
            "Connect the switched load power pin to the power switch output rail.",
            findings,
        );
        return;
    };
    if output_net != load_net {
        let mut finding = Finding::critical(
            POWER_SWITCH_BUDGET_VALID,
            &scenario.name,
            "Power switch output net does not feed the targeted switched load rail.",
        );
        finding.component = Some(switch_component_id.clone());
        finding
            .measured
            .insert("switch_output_net".to_string(), json!(output_net));
        finding
            .measured
            .insert("load_net".to_string(), json!(load_net));
        finding.suggested_fixes = vec![
            "Connect the selected switch output to the declared switched load rail.".to_string(),
            "Point the scenario at the load behind this switch if the topology is intentional."
                .to_string(),
        ];
        findings.push(finding);
    }

    check_switch_pin_voltage(
        SwitchPinVoltageCheck {
            bound,
            scenario,
            component_id: &switch_component_id,
            switch_model,
            pin: &power_switch.input_pin,
            net: input_net,
            role: "input",
        },
        findings,
    );
    check_switch_pin_voltage(
        SwitchPinVoltageCheck {
            bound,
            scenario,
            component_id: &switch_component_id,
            switch_model,
            pin: &power_switch.output_pin,
            net: output_net,
            role: "output",
        },
        findings,
    );

    let Some(max_output_current_a) = positive_model_value(
        scenario,
        power_switch.max_output_current_a,
        "power_switch.max_output_current_A",
        "Set power_switch.max_output_current_A to the selected switch continuous output current rating.",
        findings,
    ) else {
        return;
    };
    let Some(current_limit_a) = positive_model_value(
        scenario,
        power_switch.current_limit_a,
        "power_switch.current_limit_A",
        "Set power_switch.current_limit_A from the selected eFuse/load-switch current-limit setting or MOSFET protection design.",
        findings,
    ) else {
        return;
    };
    let Some(min_switch_current_margin) =
        optional_margin(scenario, "min_switch_current_margin_ratio", findings)
    else {
        return;
    };
    let Some(min_current_limit_margin) =
        optional_margin(scenario, "min_current_limit_margin_ratio", findings)
    else {
        return;
    };

    let required_switch_current_a = load_current_a * min_switch_current_margin;
    if required_switch_current_a > max_output_current_a {
        let mut finding = Finding::critical(
            POWER_SWITCH_BUDGET_VALID,
            &scenario.name,
            format!(
                "Switched load current {:.6} A with {:.3}x margin exceeds {:.6} A switch rating.",
                load_current_a, min_switch_current_margin, max_output_current_a
            ),
        );
        finding.component = Some(switch_component_id.clone());
        finding
            .measured
            .insert("load_component".to_string(), json!(target.component));
        finding
            .measured
            .insert("load_current_A".to_string(), json!(load_current_a));
        finding.limit.insert(
            "required_switch_current_A".to_string(),
            json!(required_switch_current_a),
        );
        finding.limit.insert(
            "switch_max_output_current_A".to_string(),
            json!(max_output_current_a),
        );
        finding.limit.insert(
            "min_switch_current_margin_ratio".to_string(),
            json!(min_switch_current_margin),
        );
        finding.suggested_fixes = vec![
            "Select a higher-current eFuse, load switch, or MOSFET path.".to_string(),
            "Reduce the switched load current budget or split the rail across multiple switches."
                .to_string(),
        ];
        findings.push(finding);
    }

    let required_current_limit_a = load_current_a * min_current_limit_margin;
    if required_current_limit_a > current_limit_a {
        let mut finding = Finding::critical(
            POWER_SWITCH_BUDGET_VALID,
            &scenario.name,
            format!(
                "Switched load current {:.6} A with {:.3}x current-limit margin exceeds {:.6} A switch limit.",
                load_current_a, min_current_limit_margin, current_limit_a
            ),
        );
        finding.component = Some(switch_component_id.clone());
        finding
            .measured
            .insert("load_component".to_string(), json!(target.component));
        finding
            .measured
            .insert("load_current_A".to_string(), json!(load_current_a));
        finding.limit.insert(
            "required_current_limit_A".to_string(),
            json!(required_current_limit_a),
        );
        finding
            .limit
            .insert("switch_current_limit_A".to_string(), json!(current_limit_a));
        finding.limit.insert(
            "min_current_limit_margin_ratio".to_string(),
            json!(min_current_limit_margin),
        );
        finding.suggested_fixes = vec![
            "Raise the selected current-limit setting if the switch and wiring can support it."
                .to_string(),
            "Select a switch with a higher current-limit range or reduce the load budget."
                .to_string(),
        ];
        findings.push(finding);
    }

    let Some(on_resistance_ohm) = positive_model_value(
        scenario,
        power_switch.on_resistance_ohm,
        "power_switch.on_resistance_ohm",
        "Set power_switch.on_resistance_ohm from the selected part at the relevant VIN, temperature, and gate-drive condition.",
        findings,
    ) else {
        return;
    };
    let Some(rja_c_per_w) = positive_model_value(
        scenario,
        power_switch.thermal_resistance_junction_to_ambient_c_per_w,
        "power_switch.thermal_resistance_junction_to_ambient_C_per_W",
        "Set power_switch.thermal_resistance_junction_to_ambient_C_per_W for the selected package and board assumptions.",
        findings,
    ) else {
        return;
    };
    let Some(max_junction_c) = positive_model_value(
        scenario,
        power_switch.max_junction_temperature_c,
        "power_switch.max_junction_temperature_C",
        "Set power_switch.max_junction_temperature_C from the selected switch datasheet.",
        findings,
    ) else {
        return;
    };
    let Some(ambient_c) = positive_parameter(
        scenario,
        "ambient_temperature_C",
        scenario
            .parameters
            .get("ambient_temperature_C")
            .unwrap_or(&serde_yaml_ng::Value::Null),
        findings,
    ) else {
        return;
    };
    let Some(thermal_current_margin) =
        optional_margin(scenario, "thermal_current_margin_ratio", findings)
    else {
        return;
    };
    let Some(max_temperature_margin_c) =
        optional_nonnegative_parameter(scenario, "max_junction_temperature_margin_C", findings)
    else {
        return;
    };

    let thermal_current_a = load_current_a * thermal_current_margin;
    let conduction_loss_w = thermal_current_a.powi(2) * on_resistance_ohm;
    let estimated_junction_c = ambient_c + conduction_loss_w * rja_c_per_w;
    let allowed_junction_c = max_junction_c - max_temperature_margin_c;
    if estimated_junction_c > allowed_junction_c {
        let mut finding = Finding::critical(
            POWER_SWITCH_BUDGET_VALID,
            &scenario.name,
            format!(
                "Estimated switch junction temperature {:.6} C exceeds {:.6} C budget.",
                estimated_junction_c, allowed_junction_c
            ),
        );
        finding.component = Some(switch_component_id);
        finding
            .measured
            .insert("load_component".to_string(), json!(target.component));
        finding
            .measured
            .insert("thermal_current_A".to_string(), json!(thermal_current_a));
        finding
            .measured
            .insert("on_resistance_ohm".to_string(), json!(on_resistance_ohm));
        finding
            .measured
            .insert("conduction_loss_W".to_string(), json!(conduction_loss_w));
        finding.measured.insert(
            "estimated_junction_temperature_C".to_string(),
            json!(estimated_junction_c),
        );
        finding
            .limit
            .insert("ambient_temperature_C".to_string(), json!(ambient_c));
        finding.limit.insert(
            "max_junction_temperature_C".to_string(),
            json!(max_junction_c),
        );
        finding.limit.insert(
            "max_junction_temperature_margin_C".to_string(),
            json!(max_temperature_margin_c),
        );
        finding.limit.insert(
            "thermal_resistance_junction_to_ambient_C_per_W".to_string(),
            json!(rja_c_per_w),
        );
        finding.suggested_fixes = vec![
            "Select a lower-resistance switch or package with better thermal performance."
                .to_string(),
            "Reduce switched rail current, improve copper area, or add measured thermal evidence."
                .to_string(),
        ];
        findings.push(finding);
    }
}

pub(super) fn validate_power_switch_reverse_current(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(target) = &scenario.target else {
        missing_input(
            scenario,
            "target.component",
            "Set scenario.target.component to the switched load behind the power switch.",
            findings,
        );
        return;
    };
    let Some(power_pin) = target.power_pin.as_deref() else {
        missing_input(
            scenario,
            "target.power_pin",
            "Set scenario.target.power_pin to the switched load supply pin.",
            findings,
        );
        return;
    };
    let Some(load_component) = bound.project.board.components.get(&target.component) else {
        missing_input(
            scenario,
            "target.component",
            "Set scenario.target.component to an existing switched load component.",
            findings,
        );
        return;
    };
    let Some(load_net) = load_component.pins.get(power_pin) else {
        missing_input(
            scenario,
            "target.power_pin.net",
            "Connect the switched load power pin to the power switch output rail.",
            findings,
        );
        return;
    };
    let Some((switch_component_id, switch_model)) =
        power_switch_evidence(bound, scenario, findings)
    else {
        return;
    };
    let Some(power_switch) = switch_model.power_switch.as_ref() else {
        missing_input(
            scenario,
            "switch_component.power_switch",
            "Bind parameters.switch_component to a model with power_switch metadata.",
            findings,
        );
        return;
    };
    let Some(switch_component) = bound.project.board.components.get(&switch_component_id) else {
        missing_input(
            scenario,
            "switch_component",
            "Set parameters.switch_component to an existing power-switch component.",
            findings,
        );
        return;
    };
    let Some(output_net) = switch_component.pins.get(&power_switch.output_pin) else {
        missing_input(
            scenario,
            "switch_component.output_pin",
            "Connect the power switch output pin to the switched load rail.",
            findings,
        );
        return;
    };
    if output_net != load_net {
        let mut finding = Finding::critical(
            POWER_SWITCH_REVERSE_CURRENT_VALID,
            &scenario.name,
            "Power switch output net does not feed the targeted switched load rail.",
        );
        finding.component = Some(switch_component_id.clone());
        finding
            .measured
            .insert("switch_output_net".to_string(), json!(output_net));
        finding
            .measured
            .insert("load_net".to_string(), json!(load_net));
        finding.suggested_fixes = vec![
            "Connect the selected switch output to the declared switched load rail.".to_string(),
            "Point the scenario at the load behind this switch if the topology is intentional."
                .to_string(),
        ];
        findings.push(finding);
    }
    let Some(reverse_current_required) =
        optional_bool_parameter(scenario, "reverse_current_blocking_required", findings)
    else {
        return;
    };
    if !reverse_current_required {
        return;
    }
    let Some(reverse_current_blocking) = power_switch.reverse_current_blocking else {
        missing_input(
            scenario,
            "power_switch.reverse_current_blocking",
            "Set power_switch.reverse_current_blocking from selected switch datasheet or measured reverse-current behavior.",
            findings,
        );
        return;
    };
    if !reverse_current_blocking {
        let mut finding = Finding::critical(
            POWER_SWITCH_REVERSE_CURRENT_VALID,
            &scenario.name,
            "Selected switch does not declare reverse-current blocking for the e-stop switched rail.",
        );
        finding.component = Some(switch_component_id);
        finding
            .measured
            .insert("reverse_current_blocking".to_string(), json!(false));
        finding
            .limit
            .insert("reverse_current_blocking_required".to_string(), json!(true));
        finding.suggested_fixes = vec![
            "Select an eFuse/load switch with reverse-current blocking or add a validated back-to-back MOSFET path."
                .to_string(),
            "If backfeed is intentionally allowed, remove the reverse-current blocking requirement and validate the upstream energy path."
                .to_string(),
        ];
        findings.push(finding);
    }
}

pub(super) fn validate_power_switch_inrush(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(target) = &scenario.target else {
        missing_input(
            scenario,
            "target.component",
            "Set scenario.target.component to the switched load behind the power switch.",
            findings,
        );
        return;
    };
    let Some(power_pin) = target.power_pin.as_deref() else {
        missing_input(
            scenario,
            "target.power_pin",
            "Set scenario.target.power_pin to the switched load supply pin.",
            findings,
        );
        return;
    };
    let Some(load_component) = bound.project.board.components.get(&target.component) else {
        missing_input(
            scenario,
            "target.component",
            "Set scenario.target.component to an existing switched load component.",
            findings,
        );
        return;
    };
    let Some(load_net) = load_component.pins.get(power_pin) else {
        missing_input(
            scenario,
            "target.power_pin.net",
            "Connect the switched load power pin to the power switch output rail.",
            findings,
        );
        return;
    };
    let load_voltage_v = bound
        .project
        .board
        .nets
        .get(load_net)
        .and_then(|net| net.nominal_voltage)
        .unwrap_or(0.0);
    if load_voltage_v <= 0.0 || !load_voltage_v.is_finite() {
        missing_input(
            scenario,
            "target.power_pin.net.nominal_voltage",
            "Set nominal_voltage on the switched load rail for inrush estimation.",
            findings,
        );
        return;
    }
    let Some((switch_component_id, switch_model)) =
        power_switch_evidence(bound, scenario, findings)
    else {
        return;
    };
    let Some(power_switch) = switch_model.power_switch.as_ref() else {
        missing_input(
            scenario,
            "switch_component.power_switch",
            "Bind parameters.switch_component to a model with power_switch metadata.",
            findings,
        );
        return;
    };
    let Some(max_inrush_current_a) = positive_model_value(
        scenario,
        power_switch.max_inrush_current_a,
        "power_switch.max_inrush_current_A",
        "Set power_switch.max_inrush_current_A from selected switch soft-start/current-limit evidence.",
        findings,
    ) else {
        return;
    };
    let Some(soft_start_time_us) = positive_model_value(
        scenario,
        power_switch.soft_start_time_us,
        "power_switch.soft_start_time_us",
        "Set power_switch.soft_start_time_us from selected switch slew-rate or soft-start evidence.",
        findings,
    ) else {
        return;
    };
    let Some(switched_capacitance_f) =
        parameter_value(scenario, "switched_capacitance_F", findings)
    else {
        return;
    };
    let Some(min_margin) = optional_margin(scenario, "min_inrush_current_margin_ratio", findings)
    else {
        return;
    };

    let estimated_inrush_current_a =
        switched_capacitance_f * load_voltage_v / (soft_start_time_us * 1e-6);
    let required_inrush_current_a = estimated_inrush_current_a * min_margin;
    if required_inrush_current_a > max_inrush_current_a {
        let mut finding = Finding::critical(
            POWER_SWITCH_INRUSH_VALID,
            &scenario.name,
            format!(
                "Estimated inrush current {:.6} A with {:.3}x margin exceeds {:.6} A switch limit.",
                estimated_inrush_current_a, min_margin, max_inrush_current_a
            ),
        );
        finding.component = Some(switch_component_id);
        finding
            .measured
            .insert("load_component".to_string(), json!(target.component));
        finding
            .measured
            .insert("load_net".to_string(), json!(load_net));
        finding
            .measured
            .insert("load_voltage_V".to_string(), json!(load_voltage_v));
        finding.measured.insert(
            "switched_capacitance_F".to_string(),
            json!(switched_capacitance_f),
        );
        finding
            .measured
            .insert("soft_start_time_us".to_string(), json!(soft_start_time_us));
        finding.measured.insert(
            "estimated_inrush_current_A".to_string(),
            json!(estimated_inrush_current_a),
        );
        finding.limit.insert(
            "required_inrush_current_A".to_string(),
            json!(required_inrush_current_a),
        );
        finding.limit.insert(
            "switch_max_inrush_current_A".to_string(),
            json!(max_inrush_current_a),
        );
        finding.limit.insert(
            "min_inrush_current_margin_ratio".to_string(),
            json!(min_margin),
        );
        finding.suggested_fixes = vec![
            "Select a switch with a higher soft-start/inrush current rating.".to_string(),
            "Increase soft-start time, reduce switched capacitance, or split the switched rail."
                .to_string(),
            "Validate turn-on waveform and upstream rail droop with measurement or transient simulation."
                .to_string(),
        ];
        findings.push(finding);
    }
}

fn power_switch_evidence<'a>(
    bound: &'a BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> Option<(String, &'a ComponentModel)> {
    let Some(raw) = scenario.parameters.get("switch_component") else {
        missing_input(
            scenario,
            "switch_component",
            "Set load_budget parameters.switch_component to the selected switch component.",
            findings,
        );
        return None;
    };
    let Some(component_id) = raw.as_str() else {
        missing_input(
            scenario,
            "switch_component",
            "Set load_budget parameters.switch_component to a component id string.",
            findings,
        );
        return None;
    };
    let Some(component) = bound.project.board.components.get(component_id) else {
        missing_input(
            scenario,
            "switch_component",
            "Set parameters.switch_component to an existing power switch component.",
            findings,
        );
        return None;
    };
    let Some(model) = bound.library.get(&component.model) else {
        missing_input(
            scenario,
            "switch_component.model",
            "Bind the power switch component to a model with power_switch metadata.",
            findings,
        );
        return None;
    };
    Some((component_id.to_string(), model))
}

struct SwitchPinVoltageCheck<'a> {
    bound: &'a BoundBoard<'a>,
    scenario: &'a Scenario,
    component_id: &'a str,
    switch_model: &'a ComponentModel,
    pin: &'a str,
    net: &'a str,
    role: &'a str,
}

fn check_switch_pin_voltage(check: SwitchPinVoltageCheck<'_>, findings: &mut Vec<Finding>) {
    let Some(net_voltage_v) = check
        .bound
        .project
        .board
        .nets
        .get(check.net)
        .and_then(|net| net.nominal_voltage)
    else {
        missing_input(
            check.scenario,
            &format!("switch_component.{}_net.nominal_voltage", check.role),
            &format!(
                "Set nominal_voltage on the power switch {} net.",
                check.role
            ),
            findings,
        );
        return;
    };
    let Some(port) = check.switch_model.ports.get(check.pin) else {
        missing_input(
            check.scenario,
            &format!("switch_component.{}_pin.port", check.role),
            &format!(
                "Declare the power switch {} pin in its component model ports.",
                check.role
            ),
            findings,
        );
        return;
    };
    let Some(max_voltage_v) = port.electrical.operating_voltage_max_v else {
        missing_input(
            check.scenario,
            &format!(
                "switch_component.{}_pin.operating_voltage_max_V",
                check.role
            ),
            &format!(
                "Set operating_voltage_max_V on the power switch {} power pin.",
                check.role
            ),
            findings,
        );
        return;
    };
    if net_voltage_v > max_voltage_v {
        let mut finding = Finding::critical(
            POWER_SWITCH_BUDGET_VALID,
            &check.scenario.name,
            format!(
                "Power switch {} net voltage {:.6} V exceeds {:.6} V pin rating.",
                check.role, net_voltage_v, max_voltage_v
            ),
        );
        finding.component = Some(check.component_id.to_string());
        finding
            .measured
            .insert(format!("{}_net", check.role), json!(check.net));
        finding.measured.insert(
            format!("{}_net_voltage_V", check.role),
            json!(net_voltage_v),
        );
        finding.limit.insert(
            format!("{}_pin_operating_voltage_max_V", check.role),
            json!(max_voltage_v),
        );
        finding.suggested_fixes = vec![
            "Select a switch with a higher input/output voltage rating.".to_string(),
            "Lower the switched rail voltage or use a different rail partition.".to_string(),
        ];
        findings.push(finding);
    }
}

fn positive_model_value(
    scenario: &Scenario,
    value: Option<f64>,
    field: &str,
    fix: &str,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    let Some(value) = value else {
        missing_input(scenario, field, fix, findings);
        return None;
    };
    if value.is_finite() && value > 0.0 {
        Some(value)
    } else {
        missing_input(
            scenario,
            field,
            &format!("Set {field} to a finite value greater than zero."),
            findings,
        );
        None
    }
}

fn optional_nonnegative_parameter(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    let Some(raw) = scenario.parameters.get(name) else {
        return Some(0.0);
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
    if value.is_finite() && value >= 0.0 {
        Some(value)
    } else {
        missing_input(
            scenario,
            name,
            &format!("Set {name} to a finite value at least 0.0."),
            findings,
        );
        None
    }
}

fn optional_bool_parameter(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<bool> {
    let Some(raw) = scenario.parameters.get(name) else {
        return Some(true);
    };
    let Some(value) = raw.as_bool() else {
        missing_input(
            scenario,
            name,
            &format!("Set load_budget parameters.{name} to a boolean."),
            findings,
        );
        return None;
    };
    Some(value)
}

fn parameter_value(scenario: &Scenario, name: &str, findings: &mut Vec<Finding>) -> Option<f64> {
    let Some(raw) = scenario.parameters.get(name) else {
        missing_input(
            scenario,
            name,
            &format!("Set load_budget parameters.{name} to a finite value greater than zero."),
            findings,
        );
        return None;
    };
    positive_parameter(scenario, name, raw, findings)
}
