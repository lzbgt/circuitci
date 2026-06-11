use crate::board_ir::{AnalogNetlistSource, ComponentSpec, Scenario};
use crate::library::{BoundBoard, ComponentModel, SpiceModelType};
use crate::reports::Finding;
use serde_json::json;
use std::collections::BTreeMap;

use super::SPICE_OPERATING_LIMIT;
use super::analog_runner::NgspiceRun;
use super::spice_netlist::current_sense_name;

pub(super) struct OperatingLimitProbe {
    pub(super) component_id: String,
    pub(super) rating: String,
    pub(super) expression: String,
    pub(super) rating_value: f64,
    pub(super) limit: f64,
    pub(super) unit: &'static str,
    pub(super) quantity: &'static str,
}

pub(super) struct OperatingLimitProbes {
    pub(super) probes: Vec<OperatingLimitProbe>,
    pub(super) metadata_findings: Vec<Finding>,
}

pub(super) fn operating_limit_probes(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
) -> OperatingLimitProbes {
    let Some(analog) = &scenario.analog else {
        return OperatingLimitProbes {
            probes: Vec::new(),
            metadata_findings: Vec::new(),
        };
    };
    if analog.netlist_source != AnalogNetlistSource::GeneratedFromBoard {
        return OperatingLimitProbes {
            probes: Vec::new(),
            metadata_findings: Vec::new(),
        };
    }
    let Some(generated) = &analog.generated else {
        return OperatingLimitProbes {
            probes: Vec::new(),
            metadata_findings: Vec::new(),
        };
    };
    let node_by_net: BTreeMap<&str, &str> = analog
        .node_bindings
        .iter()
        .map(|binding| (binding.net.as_str(), binding.node.as_str()))
        .collect();
    let mut probes = Vec::new();
    let mut metadata_findings = Vec::new();
    for component_id in &generated.components {
        let Some(component) = bound.project.board.components.get(component_id) else {
            continue;
        };
        let Some(model) = bound.library.get(&component.model) else {
            continue;
        };
        let Some(spice) = &model.simulation.spice else {
            continue;
        };
        match spice.model_type {
            SpiceModelType::MosfetN | SpiceModelType::MosfetP => {
                push_mosfet_operating_probes(
                    component_id,
                    component,
                    model,
                    &node_by_net,
                    &mut probes,
                    &mut metadata_findings,
                    &scenario.name,
                );
            }
            SpiceModelType::BjtNpn | SpiceModelType::BjtPnp => {
                push_bjt_operating_probes(
                    component_id,
                    component,
                    model,
                    &node_by_net,
                    &mut probes,
                    &mut metadata_findings,
                    &scenario.name,
                );
            }
            SpiceModelType::Diode => {
                push_diode_operating_probes(
                    component_id,
                    component,
                    model,
                    &node_by_net,
                    &mut probes,
                    &mut metadata_findings,
                    &scenario.name,
                );
            }
            SpiceModelType::Subckt => {}
        }
    }
    OperatingLimitProbes {
        probes,
        metadata_findings,
    }
}

fn push_mosfet_operating_probes(
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
    node_by_net: &BTreeMap<&str, &str>,
    probes: &mut Vec<OperatingLimitProbe>,
    metadata_findings: &mut Vec<Finding>,
    scenario_name: &str,
) {
    let (Some(drain), Some(gate), Some(source)) = (
        spice_node_for_pin(component, node_by_net, "D"),
        spice_node_for_pin(component, node_by_net, "G"),
        spice_node_for_pin(component, node_by_net, "S"),
    ) else {
        return;
    };
    let current_sense = current_sense_name("M", component_id);
    let vds = voltage_expression(drain, source);
    let vgs = voltage_expression(gate, source);
    if let Some((rating, rating_value, limit)) = rating_abs(model, &["VDSS"], "V") {
        probes.push(OperatingLimitProbe {
            component_id: component_id.to_string(),
            rating,
            expression: format!("abs({vds})"),
            rating_value,
            limit,
            unit: "V",
            quantity: "voltage",
        });
    } else {
        metadata_findings.push(missing_operating_rating_finding(
            component_id,
            model,
            scenario_name,
            "voltage",
            "V",
            &["VDSS"],
        ));
    }
    if let Some((rating, rating_value, limit)) =
        rating_abs(model, &["VGSS", "VGSS_continuous"], "V")
    {
        probes.push(OperatingLimitProbe {
            component_id: component_id.to_string(),
            rating,
            expression: format!("abs({vgs})"),
            rating_value,
            limit,
            unit: "V",
            quantity: "voltage",
        });
    } else {
        metadata_findings.push(missing_operating_rating_finding(
            component_id,
            model,
            scenario_name,
            "voltage",
            "V",
            &["VGSS", "VGSS_continuous"],
        ));
    }
    if let Some((rating, rating_value, limit)) = rating_abs(model, &["ID_continuous", "ID"], "A") {
        probes.push(OperatingLimitProbe {
            component_id: component_id.to_string(),
            rating,
            expression: format!("abs(I({current_sense}))"),
            rating_value,
            limit,
            unit: "A",
            quantity: "current",
        });
    } else {
        metadata_findings.push(missing_operating_rating_finding(
            component_id,
            model,
            scenario_name,
            "current",
            "A",
            &["ID_continuous", "ID"],
        ));
    }
    if let Some((rating, rating_value, limit)) = rating_abs(model, &["PD"], "W") {
        probes.push(OperatingLimitProbe {
            component_id: component_id.to_string(),
            rating,
            expression: format!("abs({vds}*I({current_sense}))"),
            rating_value,
            limit,
            unit: "W",
            quantity: "power",
        });
    } else {
        metadata_findings.push(missing_operating_rating_finding(
            component_id,
            model,
            scenario_name,
            "power",
            "W",
            &["PD"],
        ));
    }
}

fn push_bjt_operating_probes(
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
    node_by_net: &BTreeMap<&str, &str>,
    probes: &mut Vec<OperatingLimitProbe>,
    metadata_findings: &mut Vec<Finding>,
    scenario_name: &str,
) {
    let (Some(collector), Some(base), Some(emitter)) = (
        spice_node_for_pin(component, node_by_net, "C"),
        spice_node_for_pin(component, node_by_net, "B"),
        spice_node_for_pin(component, node_by_net, "E"),
    ) else {
        return;
    };
    let current_sense = current_sense_name("Q", component_id);
    let vce = voltage_expression(collector, emitter);
    let vcb = voltage_expression(collector, base);
    let veb = voltage_expression(emitter, base);
    if let Some((rating, rating_value, limit)) = rating_abs(model, &["VCEO"], "V") {
        probes.push(OperatingLimitProbe {
            component_id: component_id.to_string(),
            rating,
            expression: format!("abs({vce})"),
            rating_value,
            limit,
            unit: "V",
            quantity: "voltage",
        });
    } else {
        metadata_findings.push(missing_operating_rating_finding(
            component_id,
            model,
            scenario_name,
            "voltage",
            "V",
            &["VCEO"],
        ));
    }
    if let Some((rating, rating_value, limit)) = rating_abs(model, &["VCBO"], "V") {
        probes.push(OperatingLimitProbe {
            component_id: component_id.to_string(),
            rating,
            expression: format!("abs({vcb})"),
            rating_value,
            limit,
            unit: "V",
            quantity: "voltage",
        });
    } else {
        metadata_findings.push(missing_operating_rating_finding(
            component_id,
            model,
            scenario_name,
            "voltage",
            "V",
            &["VCBO"],
        ));
    }
    if let Some((rating, rating_value, limit)) = rating_abs(model, &["VEBO"], "V") {
        probes.push(OperatingLimitProbe {
            component_id: component_id.to_string(),
            rating,
            expression: format!("abs({veb})"),
            rating_value,
            limit,
            unit: "V",
            quantity: "voltage",
        });
    } else {
        metadata_findings.push(missing_operating_rating_finding(
            component_id,
            model,
            scenario_name,
            "voltage",
            "V",
            &["VEBO"],
        ));
    }
    if let Some((rating, rating_value, limit)) = rating_abs(model, &["IC"], "A") {
        probes.push(OperatingLimitProbe {
            component_id: component_id.to_string(),
            rating,
            expression: format!("abs(I({current_sense}))"),
            rating_value,
            limit,
            unit: "A",
            quantity: "current",
        });
    } else {
        metadata_findings.push(missing_operating_rating_finding(
            component_id,
            model,
            scenario_name,
            "current",
            "A",
            &["IC"],
        ));
    }
    if let Some((rating, rating_value, limit)) = rating_abs(model, &["PD"], "W") {
        probes.push(OperatingLimitProbe {
            component_id: component_id.to_string(),
            rating,
            expression: format!("abs({vce}*I({current_sense}))"),
            rating_value,
            limit,
            unit: "W",
            quantity: "power",
        });
    } else {
        metadata_findings.push(missing_operating_rating_finding(
            component_id,
            model,
            scenario_name,
            "power",
            "W",
            &["PD"],
        ));
    }
}

fn push_diode_operating_probes(
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
    node_by_net: &BTreeMap<&str, &str>,
    probes: &mut Vec<OperatingLimitProbe>,
    metadata_findings: &mut Vec<Finding>,
    scenario_name: &str,
) {
    let (Some(anode), Some(cathode)) = (
        spice_node_for_pin(component, node_by_net, "A"),
        spice_node_for_pin(component, node_by_net, "K"),
    ) else {
        return;
    };
    let current_sense = current_sense_name("D", component_id);
    let forward_voltage = voltage_expression(anode, cathode);
    let reverse_voltage = voltage_expression(cathode, anode);
    if let Some((rating, rating_value, limit)) = rating_abs(model, &["VRRM", "VR"], "V") {
        probes.push(OperatingLimitProbe {
            component_id: component_id.to_string(),
            rating,
            expression: format!("max(0,{reverse_voltage})"),
            rating_value,
            limit,
            unit: "V",
            quantity: "voltage",
        });
    } else {
        metadata_findings.push(missing_operating_rating_finding(
            component_id,
            model,
            scenario_name,
            "voltage",
            "V",
            &["VRRM", "VR"],
        ));
    }
    if let Some((rating, rating_value, limit)) = rating_abs(model, &["IF", "IF_AV"], "A") {
        probes.push(OperatingLimitProbe {
            component_id: component_id.to_string(),
            rating,
            expression: format!("max(0,I({current_sense}))"),
            rating_value,
            limit,
            unit: "A",
            quantity: "current",
        });
    } else {
        metadata_findings.push(missing_operating_rating_finding(
            component_id,
            model,
            scenario_name,
            "current",
            "A",
            &["IF", "IF_AV"],
        ));
    }
    if let Some((rating, rating_value, limit)) = rating_abs(model, &["PD", "Ptot"], "W") {
        probes.push(OperatingLimitProbe {
            component_id: component_id.to_string(),
            rating,
            expression: format!("max(0,{forward_voltage}*I({current_sense}))"),
            rating_value,
            limit,
            unit: "W",
            quantity: "power",
        });
    } else {
        metadata_findings.push(missing_operating_rating_finding(
            component_id,
            model,
            scenario_name,
            "power",
            "W",
            &["PD", "Ptot"],
        ));
    }
}

fn missing_operating_rating_finding(
    component_id: &str,
    model: &ComponentModel,
    scenario_name: &str,
    quantity: &'static str,
    unit: &'static str,
    keys: &[&str],
) -> Finding {
    let keys_text = keys.join(" or ");
    let mut finding = Finding::critical(
        SPICE_OPERATING_LIMIT,
        scenario_name,
        format!(
            "Component {component_id} model {} is missing datasheet absolute maximum rating {keys_text} ({unit}) required for generated {quantity} operating-limit checks.",
            model.component_id
        ),
    );
    finding
        .measured
        .insert("component".to_string(), json!(component_id));
    finding
        .measured
        .insert("model".to_string(), json!(model.component_id));
    finding
        .measured
        .insert("quantity".to_string(), json!(quantity));
    finding
        .measured
        .insert("missing_rating".to_string(), json!(keys));
    finding.measured.insert("unit".to_string(), json!(unit));
    finding
        .limit
        .insert("absolute_maximum_rating_required".to_string(), json!(true));
    finding.suggested_fixes.push(
        "Add datasheet-backed absolute maximum rating metadata for this generated semiconductor model before treating the simulation as physical evidence.".to_string(),
    );
    finding
}

fn voltage_expression(positive: &str, negative: &str) -> String {
    if positive == "0" {
        format!("-V({negative})")
    } else if negative == "0" {
        format!("V({positive})")
    } else {
        format!("V({positive},{negative})")
    }
}

fn spice_node_for_pin<'a>(
    component: &ComponentSpec,
    node_by_net: &BTreeMap<&'a str, &'a str>,
    pin: &str,
) -> Option<&'a str> {
    let net = component.pins.get(pin)?;
    node_by_net.get(net.as_str()).copied()
}

fn rating_abs(model: &ComponentModel, keys: &[&str], unit: &str) -> Option<(String, f64, f64)> {
    let ratings = &model.datasheet.as_ref()?.absolute_maximum_ratings;
    for key in keys {
        let Some(rating) = ratings.get(*key) else {
            continue;
        };
        if rating.unit.eq_ignore_ascii_case(unit) && rating.value.is_finite() {
            let limit = rating.value.abs();
            if limit > 0.0 {
                return Some(((*key).to_string(), rating.value, limit));
            }
        }
    }
    None
}

pub(super) fn evaluate_operating_limits(
    scenario: &Scenario,
    run: &NgspiceRun,
    operating_probes: &[OperatingLimitProbe],
    findings: &mut Vec<Finding>,
) {
    for (probe_offset, probe) in operating_probes.iter().enumerate() {
        let probe_index = run.user_probe_count + probe_offset;
        let Some(values) = run.series.values_by_probe.get(probe_index) else {
            continue;
        };
        let Some((max_index, max_abs)) =
            values
                .iter()
                .copied()
                .enumerate()
                .max_by(|(_, left), (_, right)| {
                    left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal)
                })
        else {
            continue;
        };
        if max_abs <= probe.limit {
            continue;
        }
        let time_of_max_us = run
            .series
            .time_s
            .get(max_index)
            .copied()
            .map(|seconds| seconds * 1_000_000.0);
        let mut finding = Finding::critical(
            SPICE_OPERATING_LIMIT,
            &scenario.name,
            format!(
                "Component {} exceeded datasheet {}: maximum simulated {} was {:.6} {}, limit is {:.6} {}.",
                probe.component_id,
                probe.rating,
                probe.quantity,
                max_abs,
                probe.unit,
                probe.limit,
                probe.unit
            ),
        );
        finding
            .measured
            .insert("component".to_string(), json!(probe.component_id));
        finding
            .measured
            .insert("rating".to_string(), json!(probe.rating));
        finding
            .measured
            .insert("quantity".to_string(), json!(probe.quantity));
        finding
            .measured
            .insert("expression".to_string(), json!(probe.expression));
        finding
            .measured
            .insert("max_abs".to_string(), json!(max_abs));
        if let Some(time_us) = time_of_max_us {
            finding
                .measured
                .insert("time_of_max_us".to_string(), json!(time_us));
        }
        finding
            .measured
            .insert("unit".to_string(), json!(probe.unit));
        finding
            .limit
            .insert("rating".to_string(), json!(probe.rating));
        finding
            .limit
            .insert("rating_value".to_string(), json!(probe.rating_value));
        finding
            .limit
            .insert("max_abs".to_string(), json!(probe.limit));
        finding.limit.insert("unit".to_string(), json!(probe.unit));
        finding.suggested_fixes.push(
            "Reduce device stress, choose a higher-rated part, or update the model metadata only if the datasheet value is wrong.".to_string(),
        );
        findings.push(finding);
    }
}

#[cfg(test)]
mod tests {
    use super::current_sense_name;

    #[test]
    fn operating_limit_current_probe_uses_generated_netlist_sense_name() {
        assert_eq!(current_sense_name("M", "M1"), "VCCI_M1");
        assert_eq!(current_sense_name("Q", "Q-2"), "VCCI_Q_2");
        assert_eq!(current_sense_name("D", "D13"), "VCCI_D13");
        assert_eq!(current_sense_name("D", "D-2"), "VCCI_D_2");
    }
}
