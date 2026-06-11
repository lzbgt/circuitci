use crate::board_ir::{AnalogNetlistSource, AnalogOperatingConditions, ComponentSpec, Scenario};
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
    pub(super) derating: Option<DeratingEvidence>,
    pub(super) pulse: Option<PulseLimit>,
}

pub(super) struct DeratingEvidence {
    pub(super) ambient_temperature_c: f64,
    pub(super) derate_above_c: f64,
    pub(super) derating_per_c: f64,
}

pub(super) struct PulseLimit {
    pub(super) rating: String,
    pub(super) rating_value: f64,
    pub(super) limit: f64,
    pub(super) pulse_width_us: f64,
    pub(super) duty_cycle_max: f64,
}

pub(super) struct OperatingLimitProbes {
    pub(super) probes: Vec<OperatingLimitProbe>,
    pub(super) metadata_findings: Vec<Finding>,
}

struct OperatingLimitContext<'a> {
    scenario_name: &'a str,
    operating_conditions: &'a AnalogOperatingConditions,
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
    let context = OperatingLimitContext {
        scenario_name: &scenario.name,
        operating_conditions: &analog.operating_conditions,
    };
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
                    &context,
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
                    &context,
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
                    &context,
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
    context: &OperatingLimitContext<'_>,
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
    if let Some(limit) = rating_limit(model, &["VDSS"], "V", context.operating_conditions, false) {
        push_probe(
            probes,
            component_id,
            limit,
            format!("abs({vds})"),
            "V",
            "voltage",
            None,
        );
    } else {
        metadata_findings.push(missing_operating_rating_finding(
            component_id,
            model,
            context.scenario_name,
            "voltage",
            "V",
            &["VDSS"],
        ));
    }
    if let Some(limit) = rating_limit(
        model,
        &["VGSS", "VGSS_continuous"],
        "V",
        context.operating_conditions,
        false,
    ) {
        push_probe(
            probes,
            component_id,
            limit,
            format!("abs({vgs})"),
            "V",
            "voltage",
            None,
        );
    } else {
        metadata_findings.push(missing_operating_rating_finding(
            component_id,
            model,
            context.scenario_name,
            "voltage",
            "V",
            &["VGSS", "VGSS_continuous"],
        ));
    }
    if let Some(limit) = rating_limit(
        model,
        &["ID_continuous", "ID"],
        "A",
        context.operating_conditions,
        false,
    ) {
        let pulse = if context.operating_conditions.allow_pulse_ratings {
            match pulse_limit(model, &["ID_pulsed"], "A") {
                Ok(pulse) => pulse,
                Err(keys) => {
                    metadata_findings.push(incomplete_pulse_rating_finding(
                        component_id,
                        model,
                        context.scenario_name,
                        &keys,
                    ));
                    None
                }
            }
        } else {
            None
        };
        push_probe(
            probes,
            component_id,
            limit,
            format!("abs(I({current_sense}))"),
            "A",
            "current",
            pulse,
        );
    } else {
        metadata_findings.push(missing_operating_rating_finding(
            component_id,
            model,
            context.scenario_name,
            "current",
            "A",
            &["ID_continuous", "ID"],
        ));
    }
    match rating_limit(model, &["PD"], "W", context.operating_conditions, true) {
        Some(limit) => {
            push_probe(
                probes,
                component_id,
                limit,
                format!("abs({vds}*I({current_sense}))"),
                "W",
                "power",
                None,
            );
        }
        None if has_rating(model, &["PD"], "W")
            && context.operating_conditions.ambient_temperature_c.is_some() =>
        {
            metadata_findings.push(missing_derating_finding(
                component_id,
                model,
                context.scenario_name,
                "PD",
            ));
        }
        None => {
            metadata_findings.push(missing_operating_rating_finding(
                component_id,
                model,
                context.scenario_name,
                "power",
                "W",
                &["PD"],
            ));
        }
    }
}

fn push_bjt_operating_probes(
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
    node_by_net: &BTreeMap<&str, &str>,
    probes: &mut Vec<OperatingLimitProbe>,
    metadata_findings: &mut Vec<Finding>,
    context: &OperatingLimitContext<'_>,
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
    if let Some(limit) = rating_limit(model, &["VCEO"], "V", context.operating_conditions, false) {
        push_probe(
            probes,
            component_id,
            limit,
            format!("abs({vce})"),
            "V",
            "voltage",
            None,
        );
    } else {
        metadata_findings.push(missing_operating_rating_finding(
            component_id,
            model,
            context.scenario_name,
            "voltage",
            "V",
            &["VCEO"],
        ));
    }
    if let Some(limit) = rating_limit(model, &["VCBO"], "V", context.operating_conditions, false) {
        push_probe(
            probes,
            component_id,
            limit,
            format!("abs({vcb})"),
            "V",
            "voltage",
            None,
        );
    } else {
        metadata_findings.push(missing_operating_rating_finding(
            component_id,
            model,
            context.scenario_name,
            "voltage",
            "V",
            &["VCBO"],
        ));
    }
    if let Some(limit) = rating_limit(model, &["VEBO"], "V", context.operating_conditions, false) {
        push_probe(
            probes,
            component_id,
            limit,
            format!("abs({veb})"),
            "V",
            "voltage",
            None,
        );
    } else {
        metadata_findings.push(missing_operating_rating_finding(
            component_id,
            model,
            context.scenario_name,
            "voltage",
            "V",
            &["VEBO"],
        ));
    }
    if let Some(limit) = rating_limit(model, &["IC"], "A", context.operating_conditions, false) {
        push_probe(
            probes,
            component_id,
            limit,
            format!("abs(I({current_sense}))"),
            "A",
            "current",
            None,
        );
    } else {
        metadata_findings.push(missing_operating_rating_finding(
            component_id,
            model,
            context.scenario_name,
            "current",
            "A",
            &["IC"],
        ));
    }
    match rating_limit(model, &["PD"], "W", context.operating_conditions, true) {
        Some(limit) => {
            push_probe(
                probes,
                component_id,
                limit,
                format!("abs({vce}*I({current_sense}))"),
                "W",
                "power",
                None,
            );
        }
        None if has_rating(model, &["PD"], "W")
            && context.operating_conditions.ambient_temperature_c.is_some() =>
        {
            metadata_findings.push(missing_derating_finding(
                component_id,
                model,
                context.scenario_name,
                "PD",
            ));
        }
        None => {
            metadata_findings.push(missing_operating_rating_finding(
                component_id,
                model,
                context.scenario_name,
                "power",
                "W",
                &["PD"],
            ));
        }
    }
}

fn push_diode_operating_probes(
    component_id: &str,
    component: &ComponentSpec,
    model: &ComponentModel,
    node_by_net: &BTreeMap<&str, &str>,
    probes: &mut Vec<OperatingLimitProbe>,
    metadata_findings: &mut Vec<Finding>,
    context: &OperatingLimitContext<'_>,
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
    if let Some(limit) = rating_limit(
        model,
        &["VRRM", "VR"],
        "V",
        context.operating_conditions,
        false,
    ) {
        push_probe(
            probes,
            component_id,
            limit,
            format!("max(0,{reverse_voltage})"),
            "V",
            "voltage",
            None,
        );
    } else {
        metadata_findings.push(missing_operating_rating_finding(
            component_id,
            model,
            context.scenario_name,
            "voltage",
            "V",
            &["VRRM", "VR"],
        ));
    }
    if let Some(limit) = rating_limit(
        model,
        &["IF", "IF_AV"],
        "A",
        context.operating_conditions,
        false,
    ) {
        push_probe(
            probes,
            component_id,
            limit,
            format!("max(0,I({current_sense}))"),
            "A",
            "current",
            None,
        );
    } else {
        metadata_findings.push(missing_operating_rating_finding(
            component_id,
            model,
            context.scenario_name,
            "current",
            "A",
            &["IF", "IF_AV"],
        ));
    }
    match rating_limit(
        model,
        &["PD", "Ptot"],
        "W",
        context.operating_conditions,
        true,
    ) {
        Some(limit) => {
            push_probe(
                probes,
                component_id,
                limit,
                format!("max(0,{forward_voltage}*I({current_sense}))"),
                "W",
                "power",
                None,
            );
        }
        None if has_rating(model, &["PD", "Ptot"], "W")
            && context.operating_conditions.ambient_temperature_c.is_some() =>
        {
            metadata_findings.push(missing_derating_finding(
                component_id,
                model,
                context.scenario_name,
                "PD or Ptot",
            ));
        }
        None => {
            metadata_findings.push(missing_operating_rating_finding(
                component_id,
                model,
                context.scenario_name,
                "power",
                "W",
                &["PD", "Ptot"],
            ));
        }
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

fn missing_derating_finding(
    component_id: &str,
    model: &ComponentModel,
    scenario_name: &str,
    rating: &str,
) -> Finding {
    let mut finding = Finding::critical(
        SPICE_OPERATING_LIMIT,
        scenario_name,
        format!(
            "Component {component_id} model {} has {rating} but lacks linear temperature derating metadata required by scenario ambient temperature.",
            model.component_id
        ),
    );
    finding
        .measured
        .insert("component".to_string(), json!(component_id));
    finding
        .measured
        .insert("model".to_string(), json!(model.component_id));
    finding.measured.insert("rating".to_string(), json!(rating));
    finding
        .limit
        .insert("temperature_derating_required".to_string(), json!(true));
    finding.suggested_fixes.push(
        "Add datasheet-backed derate_above_c and derating_per_c metadata, or remove ambient-temperature derating from this scenario.".to_string(),
    );
    finding
}

fn incomplete_pulse_rating_finding(
    component_id: &str,
    model: &ComponentModel,
    scenario_name: &str,
    keys: &[String],
) -> Finding {
    let mut finding = Finding::critical(
        SPICE_OPERATING_LIMIT,
        scenario_name,
        format!(
            "Component {component_id} model {} enables pulse current checks but lacks qualified pulse rating metadata for {}.",
            model.component_id,
            keys.join(" or ")
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
        .insert("missing_pulse_rating".to_string(), json!(keys));
    finding
        .limit
        .insert("pulse_width_and_duty_required".to_string(), json!(true));
    finding.suggested_fixes.push(
        "Add datasheet-backed pulse_width_us and duty_cycle_max metadata for the pulse current rating, or disable pulse-rating allowance for this scenario.".to_string(),
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

struct EffectiveRating {
    rating: String,
    rating_value: f64,
    limit: f64,
    derating: Option<DeratingEvidence>,
}

fn push_probe(
    probes: &mut Vec<OperatingLimitProbe>,
    component_id: &str,
    limit: EffectiveRating,
    expression: String,
    unit: &'static str,
    quantity: &'static str,
    pulse: Option<PulseLimit>,
) {
    probes.push(OperatingLimitProbe {
        component_id: component_id.to_string(),
        rating: limit.rating,
        expression,
        rating_value: limit.rating_value,
        limit: limit.limit,
        unit,
        quantity,
        derating: limit.derating,
        pulse,
    });
}

fn rating_limit(
    model: &ComponentModel,
    keys: &[&str],
    unit: &str,
    operating_conditions: &AnalogOperatingConditions,
    derate_when_ambient_known: bool,
) -> Option<EffectiveRating> {
    let ratings = &model.datasheet.as_ref()?.absolute_maximum_ratings;
    for key in keys {
        let Some(rating) = ratings.get(*key) else {
            continue;
        };
        if !rating.unit.eq_ignore_ascii_case(unit) || !rating.value.is_finite() {
            continue;
        }
        let limit = rating.value.abs();
        if limit <= 0.0 {
            continue;
        }
        if derate_when_ambient_known
            && let Some(ambient_temperature_c) = operating_conditions.ambient_temperature_c
        {
            let (Some(derate_above_c), Some(derating_per_c)) =
                (rating.derate_above_c, rating.derating_per_c)
            else {
                return None;
            };
            if !derate_above_c.is_finite() || !derating_per_c.is_finite() || derating_per_c <= 0.0 {
                return None;
            }
            let temperature_delta = (ambient_temperature_c - derate_above_c).max(0.0);
            let derated_limit = (limit - temperature_delta * derating_per_c).max(0.0);
            if derated_limit <= 0.0 {
                return None;
            }
            return Some(EffectiveRating {
                rating: (*key).to_string(),
                rating_value: rating.value,
                limit: derated_limit,
                derating: Some(DeratingEvidence {
                    ambient_temperature_c,
                    derate_above_c,
                    derating_per_c,
                }),
            });
        }
        return Some(EffectiveRating {
            rating: (*key).to_string(),
            rating_value: rating.value,
            limit,
            derating: None,
        });
    }
    None
}

fn pulse_limit(
    model: &ComponentModel,
    keys: &[&str],
    unit: &str,
) -> Result<Option<PulseLimit>, Vec<String>> {
    let key_list = || keys.iter().map(|key| (*key).to_string()).collect();
    let Some(ratings) = model
        .datasheet
        .as_ref()
        .map(|datasheet| &datasheet.absolute_maximum_ratings)
    else {
        return Err(key_list());
    };
    for key in keys {
        let Some(rating) = ratings.get(*key) else {
            continue;
        };
        if !rating.unit.eq_ignore_ascii_case(unit) || !rating.value.is_finite() {
            continue;
        }
        let (Some(pulse_width_us), Some(duty_cycle_max)) =
            (rating.pulse_width_us, rating.duty_cycle_max)
        else {
            return Err(key_list());
        };
        let limit = rating.value.abs();
        if limit <= 0.0
            || !pulse_width_us.is_finite()
            || pulse_width_us <= 0.0
            || !duty_cycle_max.is_finite()
            || duty_cycle_max <= 0.0
            || duty_cycle_max > 1.0
        {
            return Err(key_list());
        }
        return Ok(Some(PulseLimit {
            rating: (*key).to_string(),
            rating_value: rating.value,
            limit,
            pulse_width_us,
            duty_cycle_max,
        }));
    }
    Err(key_list())
}

fn has_rating(model: &ComponentModel, keys: &[&str], unit: &str) -> bool {
    model.datasheet.as_ref().is_some_and(|datasheet| {
        keys.iter().any(|key| {
            datasheet
                .absolute_maximum_ratings
                .get(*key)
                .is_some_and(|rating| {
                    rating.unit.eq_ignore_ascii_case(unit) && rating.value.is_finite()
                })
        })
    })
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
        let pulse_evidence = probe.pulse.as_ref().map(|pulse| {
            let duration_above_continuous_us =
                duration_above_limit_us(&run.series.time_s, values, probe.limit);
            let total_duration_us = transient_duration_us(&run.series.time_s);
            let duty_cycle = if total_duration_us > 0.0 {
                duration_above_continuous_us / total_duration_us
            } else {
                1.0
            };
            (pulse, duration_above_continuous_us, duty_cycle)
        });
        if let Some((pulse, duration_above_continuous_us, duty_cycle)) = pulse_evidence
            && max_abs <= pulse.limit
            && duration_above_continuous_us <= pulse.pulse_width_us
            && duty_cycle <= pulse.duty_cycle_max
        {
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
        finding
            .limit
            .insert("effective_limit".to_string(), json!(probe.limit));
        finding.limit.insert("unit".to_string(), json!(probe.unit));
        if let Some(derating) = &probe.derating {
            finding.measured.insert(
                "scenario_temperature_c".to_string(),
                json!(derating.ambient_temperature_c),
            );
            finding
                .limit
                .insert("derate_above_c".to_string(), json!(derating.derate_above_c));
            finding
                .limit
                .insert("derating_per_c".to_string(), json!(derating.derating_per_c));
        }
        if let Some((pulse, duration_above_continuous_us, duty_cycle)) = pulse_evidence {
            finding.measured.insert(
                "pulse_duration_us".to_string(),
                json!(duration_above_continuous_us),
            );
            finding
                .measured
                .insert("pulse_duty_cycle".to_string(), json!(duty_cycle));
            finding
                .limit
                .insert("pulse_rating".to_string(), json!(pulse.rating));
            finding
                .limit
                .insert("pulse_rating_value".to_string(), json!(pulse.rating_value));
            finding
                .limit
                .insert("pulse_max_abs".to_string(), json!(pulse.limit));
            finding
                .limit
                .insert("pulse_width_us".to_string(), json!(pulse.pulse_width_us));
            finding.limit.insert(
                "pulse_duty_cycle_max".to_string(),
                json!(pulse.duty_cycle_max),
            );
        }
        finding.suggested_fixes.push(
            "Reduce device stress, choose a higher-rated part, or update the model metadata only if the datasheet value is wrong.".to_string(),
        );
        findings.push(finding);
    }
}

fn transient_duration_us(time_s: &[f64]) -> f64 {
    let (Some(start), Some(end)) = (time_s.first(), time_s.last()) else {
        return 0.0;
    };
    ((end - start) * 1_000_000.0).max(0.0)
}

fn duration_above_limit_us(time_s: &[f64], values: &[f64], limit: f64) -> f64 {
    if time_s.len() < 2 || values.len() < 2 {
        return 0.0;
    }
    time_s
        .windows(2)
        .zip(values.windows(2))
        .filter_map(|(time_pair, value_pair)| {
            let interval_s = time_pair[1] - time_pair[0];
            if interval_s <= 0.0 {
                return None;
            }
            if value_pair[0] > limit || value_pair[1] > limit {
                Some(interval_s * 1_000_000.0)
            } else {
                None
            }
        })
        .sum()
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
