use crate::board_ir::{ComponentSpec, NetKind, Scenario, SpicePrimitive};
use crate::library::{BoundBoard, ClockSource, ComponentModel};
use crate::reports::Finding;
use serde_json::json;

use super::CLOCK_SOURCE_VALID;

pub(super) fn validate_clock_sources(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    for (component_id, component) in &bound.project.board.components {
        if let Some(target) = &scenario.target
            && target.component != *component_id
        {
            continue;
        }
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        for clock in &model.clock_sources {
            validate_clock_source(
                component_id,
                component,
                model,
                clock,
                bound,
                scenario,
                findings,
            );
        }
    }
}

fn validate_clock_source(
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
    clock: &ClockSource,
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    if !validate_clock_metadata(component_id, model, clock, scenario, findings) {
        return;
    }
    let Some(input_net) = component.pins.get(&clock.input_pin).map(String::as_str) else {
        clock_pin_finding(
            component_id,
            &clock.name,
            &clock.input_pin,
            scenario,
            findings,
        );
        return;
    };
    let Some(output_net) = component.pins.get(&clock.output_pin).map(String::as_str) else {
        clock_pin_finding(
            component_id,
            &clock.name,
            &clock.output_pin,
            scenario,
            findings,
        );
        return;
    };
    if input_net == output_net {
        let mut finding = Finding::critical(
            CLOCK_SOURCE_VALID,
            &scenario.name,
            format!(
                "Clock source {component_id}.{} input and output pins share net {input_net}.",
                clock.name
            ),
        );
        finding.component = Some(component_id.to_string());
        finding.net = Some(input_net.to_string());
        finding
            .limit
            .insert("distinct_clock_nets".to_string(), json!(true));
        finding.suggested_fixes = vec![
            "Wire the crystal across distinct oscillator input/output pins.".to_string(),
            "Correct the component model clock_sources pin names if they do not match the MCU package.".to_string(),
        ];
        findings.push(finding);
        return;
    }

    let Some(crystal) = find_crystal(bound, input_net, output_net) else {
        let mut finding = Finding::critical(
            CLOCK_SOURCE_VALID,
            &scenario.name,
            format!(
                "Clock source {component_id}.{} has no crystal component between nets {input_net} and {output_net}.",
                clock.name
            ),
        );
        finding.component = Some(component_id.to_string());
        finding
            .measured
            .insert("clock_input_net".to_string(), json!(input_net));
        finding
            .measured
            .insert("clock_output_net".to_string(), json!(output_net));
        finding.limit.insert(
            "required_crystal_between_clock_nets".to_string(),
            json!(true),
        );
        finding.suggested_fixes = vec![
            "Add or map a crystal/resonator component between the MCU oscillator pins.".to_string(),
            "Use a component model with crystal metadata so load capacitance can be checked."
                .to_string(),
        ];
        findings.push(finding);
        return;
    };
    let Some(crystal_model) = bound.library.get(&crystal.component.model) else {
        return;
    };
    let Some(crystal_spec) = crystal_model.crystal.as_ref() else {
        return;
    };
    if !crystal_spec.frequency_hz.is_finite()
        || crystal_spec.frequency_hz <= 0.0
        || !crystal_spec.load_capacitance_f.is_finite()
        || crystal_spec.load_capacitance_f <= 0.0
        || crystal_spec
            .load_capacitance_tolerance_f
            .is_some_and(|tol| !tol.is_finite() || tol < 0.0)
    {
        let mut finding = Finding::critical(
            CLOCK_SOURCE_VALID,
            &scenario.name,
            format!(
                "Crystal model {} has invalid crystal frequency or load capacitance metadata.",
                crystal.component.model
            ),
        );
        finding.component = Some(crystal.id.clone());
        finding
            .limit
            .insert("crystal_metadata_valid".to_string(), json!(true));
        finding.suggested_fixes = vec![
            "Correct crystal.frequency_Hz, crystal.load_capacitance_F, and tolerance metadata before using it for clock validation.".to_string(),
            "Use analog_transient or lab measurement for oscillator startup margin beyond this static screen.".to_string(),
        ];
        findings.push(finding);
        return;
    }

    let input_cap_f = load_capacitance_to_ground(bound, input_net);
    let output_cap_f = load_capacitance_to_ground(bound, output_net);
    let (Some(input_cap_f), Some(output_cap_f)) = (input_cap_f, output_cap_f) else {
        let mut finding = Finding::critical(
            CLOCK_SOURCE_VALID,
            &scenario.name,
            format!(
                "Clock source {component_id}.{} requires load capacitors from both oscillator nets to ground.",
                clock.name
            ),
        );
        finding.component = Some(component_id.to_string());
        finding
            .measured
            .insert("clock_input_net".to_string(), json!(input_net));
        finding
            .measured
            .insert("clock_output_net".to_string(), json!(output_net));
        finding
            .measured
            .insert("input_load_capacitance_F".to_string(), json!(input_cap_f));
        finding
            .measured
            .insert("output_load_capacitance_F".to_string(), json!(output_cap_f));
        finding.limit.insert(
            "required_load_capacitors_to_ground".to_string(),
            json!(true),
        );
        finding.suggested_fixes = vec![
            "Add explicit load capacitors from each oscillator pin to ground or map their schematic values into Board IR.".to_string(),
            "Use the crystal datasheet load capacitance and board stray capacitance to size the capacitors.".to_string(),
        ];
        findings.push(finding);
        return;
    };

    let stray_f = clock.stray_capacitance_f.unwrap_or(0.0);
    if !stray_f.is_finite() || stray_f < 0.0 {
        let mut finding = Finding::critical(
            CLOCK_SOURCE_VALID,
            &scenario.name,
            format!(
                "Clock source {component_id}.{} has invalid stray_capacitance_F metadata.",
                clock.name
            ),
        );
        finding.component = Some(component_id.to_string());
        finding
            .limit
            .insert("stray_capacitance_non_negative".to_string(), json!(true));
        finding.suggested_fixes = vec![
            "Use finite non-negative stray_capacitance_F or omit it when unknown.".to_string(),
            "Use board-layout extraction or measurement for oscillator sign-off.".to_string(),
        ];
        findings.push(finding);
        return;
    }

    let effective_load_f = (input_cap_f * output_cap_f) / (input_cap_f + output_cap_f) + stray_f;
    let tolerance_f = crystal_spec
        .load_capacitance_tolerance_f
        .unwrap_or(crystal_spec.load_capacitance_f * 0.2);
    let min_load_f = crystal_spec.load_capacitance_f - tolerance_f;
    let max_load_f = crystal_spec.load_capacitance_f + tolerance_f;
    if effective_load_f < min_load_f || effective_load_f > max_load_f {
        let mut finding = Finding::critical(
            CLOCK_SOURCE_VALID,
            &scenario.name,
            format!(
                "Clock source {component_id}.{} effective load capacitance {:.3e} F is outside crystal requirement {:.3e} F ± {:.3e} F.",
                clock.name, effective_load_f, crystal_spec.load_capacitance_f, tolerance_f
            ),
        );
        finding.component = Some(component_id.to_string());
        finding.net = Some(input_net.to_string());
        finding
            .measured
            .insert("crystal_component".to_string(), json!(crystal.id));
        finding
            .measured
            .insert("frequency_Hz".to_string(), json!(crystal_spec.frequency_hz));
        finding
            .measured
            .insert("input_load_capacitance_F".to_string(), json!(input_cap_f));
        finding
            .measured
            .insert("output_load_capacitance_F".to_string(), json!(output_cap_f));
        finding
            .measured
            .insert("stray_capacitance_F".to_string(), json!(stray_f));
        finding.measured.insert(
            "effective_load_capacitance_F".to_string(),
            json!(effective_load_f),
        );
        finding.limit.insert(
            "crystal_load_capacitance_min_F".to_string(),
            json!(min_load_f),
        );
        finding.limit.insert(
            "crystal_load_capacitance_max_F".to_string(),
            json!(max_load_f),
        );
        finding.suggested_fixes = vec![
            "Resize the two crystal load capacitors using CL = C1*C2/(C1+C2) + Cstray.".to_string(),
            "Select a crystal whose specified load capacitance matches the board support network.".to_string(),
            "Use oscillator startup simulation or lab measurement for final gain-margin and startup-time sign-off.".to_string(),
        ];
        findings.push(finding);
    }
}

struct CrystalInstance<'a> {
    id: String,
    component: &'a ComponentSpec,
}

fn find_crystal<'a>(
    bound: &'a BoundBoard<'_>,
    input_net: &str,
    output_net: &str,
) -> Option<CrystalInstance<'a>> {
    bound
        .project
        .board
        .components
        .iter()
        .find_map(|(component_id, component)| {
            let model = bound.library.get(&component.model)?;
            model.crystal.as_ref()?;
            let nets = component
                .pins
                .values()
                .map(String::as_str)
                .collect::<Vec<_>>();
            if nets.len() == 2
                && ((nets[0] == input_net && nets[1] == output_net)
                    || (nets[0] == output_net && nets[1] == input_net))
            {
                Some(CrystalInstance {
                    id: component_id.clone(),
                    component,
                })
            } else {
                None
            }
        })
}

fn load_capacitance_to_ground(bound: &BoundBoard<'_>, net_name: &str) -> Option<f64> {
    let mut total_f = 0.0;
    for component in bound.project.board.components.values() {
        let Some(spice) = &component.spice else {
            continue;
        };
        if spice.primitive != SpicePrimitive::Capacitor {
            continue;
        }
        let Some(value_f) = spice.value_f else {
            continue;
        };
        if !value_f.is_finite() || value_f <= 0.0 {
            continue;
        }
        let nets = component
            .pins
            .values()
            .map(String::as_str)
            .collect::<Vec<_>>();
        if nets.len() != 2 {
            continue;
        }
        let connects_target = nets.contains(&net_name);
        let connects_ground = nets.iter().any(|net| is_ground_net(bound, net));
        if connects_target && connects_ground {
            total_f += value_f;
        }
    }
    (total_f > 0.0).then_some(total_f)
}

fn is_ground_net(bound: &BoundBoard<'_>, net_name: &str) -> bool {
    bound
        .project
        .board
        .nets
        .get(net_name)
        .is_some_and(|net| net.kind == NetKind::Ground)
}

fn validate_clock_metadata(
    component_id: &str,
    model: &ComponentModel,
    clock: &ClockSource,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) -> bool {
    let mut valid = true;
    if clock.input_pin == clock.output_pin {
        clock_metadata_finding(
            component_id,
            &clock.name,
            "input_pin",
            "clock source input_pin and output_pin must be distinct.",
            scenario,
            findings,
        );
        valid = false;
    }
    for (role, pin) in [
        ("input_pin", clock.input_pin.as_str()),
        ("output_pin", clock.output_pin.as_str()),
    ] {
        if !model.ports.contains_key(pin) {
            clock_metadata_finding(
                component_id,
                &clock.name,
                role,
                &format!("clock source {role} {pin} is not declared in model ports."),
                scenario,
                findings,
            );
            valid = false;
        }
    }
    valid
}

fn clock_metadata_finding(
    component_id: &str,
    clock_name: &str,
    field: &str,
    message: &str,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(CLOCK_SOURCE_VALID, &scenario.name, message.to_string());
    finding.component = Some(component_id.to_string());
    finding
        .measured
        .insert("clock_source".to_string(), json!(clock_name));
    finding
        .limit
        .insert("clock_source_field".to_string(), json!(field));
    finding.suggested_fixes = vec![
        "Correct the component model clock_sources metadata before using it for clock validation."
            .to_string(),
        "Use analog_transient or lab measurement for oscillator behavior beyond this static support-network screen.".to_string(),
    ];
    findings.push(finding);
}

fn clock_pin_finding(
    component_id: &str,
    clock_name: &str,
    pin: &str,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let mut finding = Finding::critical(
        CLOCK_SOURCE_VALID,
        &scenario.name,
        format!("Clock source {component_id}.{clock_name} pin {pin} is not connected."),
    );
    finding.component = Some(component_id.to_string());
    finding
        .limit
        .insert("required_clock_pin".to_string(), json!(pin));
    finding.suggested_fixes = vec![
        "Connect every declared external clock source pin to explicit nets.".to_string(),
        "Correct the component model clock_sources pin names if they do not match the board symbol.".to_string(),
    ];
    findings.push(finding);
}
