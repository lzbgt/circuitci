use super::{BoardYamlRepairEdit, BoardYamlRepairFindingKind, BoardYamlRepairProposal};
use crate::board_ir::{BoardProject, NetKind};
use crate::library::{ComponentLibrary, Port, PortKind};
use anyhow::{Context, Result};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn invalid_power_domain_proposals(
    project: &BoardProject,
    library: &ComponentLibrary,
) -> Result<Vec<BoardYamlRepairProposal>> {
    let mut by_net: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (component_id, component) in &project.board.components {
        let Some(model) = library.get(&component.model) else {
            continue;
        };
        for (pin_name, port) in &model.ports {
            if port.kind != PortKind::ElectricalPower {
                continue;
            }
            let rail = component
                .power_domains
                .get(pin_name)
                .or_else(|| component.pins.get(pin_name))
                .or(component.power_domain.as_ref());
            let Some(net_name) = rail else {
                continue;
            };
            let Some(net) = project.board.nets.get(net_name) else {
                continue;
            };
            if net.kind != NetKind::Power {
                by_net
                    .entry(net_name.clone())
                    .or_default()
                    .push(format!("{component_id}.{pin_name}"));
            }
        }
    }

    by_net
        .into_iter()
        .enumerate()
        .map(|(index, (net, affected_pins))| {
            let net_kind = project
                .board
                .nets
                .get(&net)
                .map(|net| net_kind_as_yaml(&net.kind))
                .context("net disappeared while building repair proposal")?;
            let yaml_path = format!("/board/nets/{net}/kind");
            Ok(BoardYamlRepairProposal {
                id: format!("invalid_power_domain_{}", index + 1),
                finding_id: BoardYamlRepairFindingKind::InvalidPowerDomain
                    .finding_id()
                    .to_string(),
                status: "proposed".to_string(),
                reason_code: None,
                description: format!(
                    "Declare net {net} as power because it feeds model power pin(s) {}.",
                    affected_pins.join(", ")
                ),
                yaml_path: yaml_path.clone(),
                affected_pins,
                edits: vec![BoardYamlRepairEdit {
                    op: "replace".to_string(),
                    path: yaml_path,
                    from: serde_json::Value::String(net_kind.to_string()),
                    to: serde_json::Value::String("power".to_string()),
                    reason: "Model power pins must resolve to Board IR nets with kind: power."
                        .to_string(),
                }],
            })
        })
        .collect()
}

pub(super) fn model_not_found_proposals(
    project: &BoardProject,
    library: &ComponentLibrary,
) -> Result<Vec<BoardYamlRepairProposal>> {
    let mut proposals = Vec::new();
    for (component_id, component) in &project.board.components {
        if library.get(&component.model).is_some() {
            continue;
        }
        let yaml_path = format!("/board/components/{component_id}/model");
        let affected_pins = component
            .pins
            .keys()
            .map(|pin| format!("{component_id}.{pin}"))
            .collect::<Vec<_>>();
        let trimmed_model = component.model.trim();
        if trimmed_model.is_empty() {
            proposals.push(blocked_model_not_found_proposal(
                proposals.len() + 1,
                &component.model,
                &yaml_path,
                affected_pins,
                "empty_model_id",
                "Not repairing unresolved model because the component model id is empty after trimming.",
            ));
            continue;
        }

        let exact_trimmed_match = library
            .get(trimmed_model)
            .map(|model| model.component_id.as_str());
        let case_folded_matches = library
            .iter()
            .filter_map(|(model_id, _)| {
                model_id
                    .eq_ignore_ascii_case(trimmed_model)
                    .then_some(model_id)
            })
            .collect::<Vec<_>>();
        let candidate = exact_trimmed_match.or_else(|| {
            if case_folded_matches.len() == 1 {
                Some(case_folded_matches[0])
            } else {
                None
            }
        });
        let Some(model_id) = candidate else {
            let (reason_code, description) = if case_folded_matches.len() > 1 {
                (
                    "ambiguous_model_id_canonicalization",
                    format!(
                        "Not repairing unresolved model {} because case-insensitive matching found multiple loaded model ids: {}.",
                        component.model,
                        case_folded_matches.join(", ")
                    ),
                )
            } else {
                (
                    "unresolved_model_id",
                    format!(
                        "Not repairing unresolved model {} because no loaded model id matches after trimming and case-folding.",
                        component.model
                    ),
                )
            };
            proposals.push(blocked_model_not_found_proposal(
                proposals.len() + 1,
                &component.model,
                &yaml_path,
                affected_pins,
                reason_code,
                &description,
            ));
            continue;
        };
        if model_id == component.model {
            continue;
        }

        proposals.push(BoardYamlRepairProposal {
            id: format!("model_not_found_{}", proposals.len() + 1),
            finding_id: BoardYamlRepairFindingKind::ModelNotFound
                .finding_id()
                .to_string(),
            status: "proposed".to_string(),
            reason_code: None,
            description: format!(
                "Replace unresolved component model {} with canonical loaded model id {model_id}.",
                component.model
            ),
            yaml_path: yaml_path.clone(),
            affected_pins,
            edits: vec![BoardYamlRepairEdit {
                op: "replace".to_string(),
                path: yaml_path,
                from: serde_json::Value::String(component.model.clone()),
                to: serde_json::Value::String(model_id.to_string()),
                reason:
                    "MODEL_NOT_FOUND can be repaired only when the unresolved id has one canonical loaded model-id match after trimming and case-folding."
                        .to_string(),
            }],
        });
    }
    Ok(proposals)
}

pub(super) fn net_not_found_proposals(
    project: &BoardProject,
    library: &ComponentLibrary,
) -> Result<Vec<BoardYamlRepairProposal>> {
    let mut by_net: BTreeMap<String, MissingNetProposalDraft> = BTreeMap::new();
    for (component_id, component) in &project.board.components {
        let Some(model) = library.get(&component.model) else {
            continue;
        };
        for (pin_name, net_name) in &component.pins {
            if project.board.nets.contains_key(net_name) {
                continue;
            }
            let Some(port) = model.ports.get(pin_name) else {
                continue;
            };
            let kind = inferred_net_kind(&port.kind);
            let draft = by_net.entry(net_name.clone()).or_default();
            draft
                .affected_pins
                .push(format!("{component_id}.{pin_name}"));
            draft.inferred_kinds.insert(kind);
        }
    }

    let mut proposals = Vec::new();
    for (net, draft) in by_net {
        let yaml_path = format!("/board/nets/{net}");
        let kinds: Vec<&'static str> = draft.inferred_kinds.into_iter().collect();
        let Some(kind) = kinds.first() else {
            continue;
        };
        if kinds.len() > 1 {
            proposals.push(BoardYamlRepairProposal {
                id: format!("net_not_found_{}", proposals.len() + 1),
                finding_id: BoardYamlRepairFindingKind::NetNotFound
                    .finding_id()
                    .to_string(),
                status: "blocked".to_string(),
                reason_code: Some("conflicting_inferred_net_kinds".to_string()),
                description: format!(
                    "Not repairing missing net {net} because declared model pins infer conflicting net kinds: {}.",
                    kinds.join(", ")
                ),
                yaml_path,
                affected_pins: draft.affected_pins,
                edits: Vec::new(),
            });
            continue;
        }
        proposals.push(BoardYamlRepairProposal {
            id: format!("net_not_found_{}", proposals.len() + 1),
            finding_id: BoardYamlRepairFindingKind::NetNotFound
                .finding_id()
                .to_string(),
            status: "proposed".to_string(),
            reason_code: None,
            description: format!(
                "Add missing net {net} as {kind} because it is referenced by declared model pin(s) {}.",
                draft.affected_pins.join(", ")
            ),
            yaml_path: yaml_path.clone(),
            affected_pins: draft.affected_pins,
            edits: vec![BoardYamlRepairEdit {
                op: "add".to_string(),
                path: yaml_path,
                from: serde_json::Value::Null,
                to: serde_json::json!({ "kind": kind }),
                reason:
                    "Missing nets referenced by declared model pins can be added with kind inferred from the model port kind."
                        .to_string(),
            }],
        });
    }
    Ok(proposals)
}

pub(super) fn power_domain_not_found_proposals(
    project: &BoardProject,
    library: &ComponentLibrary,
) -> Result<Vec<BoardYamlRepairProposal>> {
    let mut by_net: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (component_id, component) in &project.board.components {
        let Some(model) = library.get(&component.model) else {
            continue;
        };
        for (pin_name, port) in &model.ports {
            if port.kind != PortKind::ElectricalPower {
                continue;
            }
            let Some((net_name, source)) = required_pin_candidate_net(component, pin_name, port)
            else {
                continue;
            };
            if project.board.nets.contains_key(net_name) {
                continue;
            }
            by_net
                .entry(net_name.clone())
                .or_default()
                .push(format!("{component_id}.{pin_name} via {source}"));
        }
    }

    Ok(by_net
        .into_iter()
        .enumerate()
        .map(|(index, (net, affected_pins))| {
            let yaml_path = format!("/board/nets/{net}");
            BoardYamlRepairProposal {
                id: format!("power_domain_not_found_{}", index + 1),
                finding_id: BoardYamlRepairFindingKind::PowerDomainNotFound
                    .finding_id()
                    .to_string(),
                status: "proposed".to_string(),
                reason_code: None,
                description: format!(
                    "Add missing power net {net} because explicit component power-domain metadata names it for model power pin(s) {}.",
                    affected_pins.join(", ")
                ),
                yaml_path: yaml_path.clone(),
                affected_pins,
                edits: vec![BoardYamlRepairEdit {
                    op: "add".to_string(),
                    path: yaml_path,
                    from: serde_json::Value::Null,
                    to: serde_json::json!({ "kind": "power" }),
                    reason:
                        "Missing power-domain nets can be added only when explicit component power-domain metadata names the net for a model power pin."
                            .to_string(),
                }],
            }
        })
        .collect())
}

pub(super) fn pin_not_declared_proposals(
    project: &BoardProject,
    library: &ComponentLibrary,
) -> Result<Vec<BoardYamlRepairProposal>> {
    let mut proposals = Vec::new();
    for (component_id, component) in &project.board.components {
        let Some(model) = library.get(&component.model) else {
            continue;
        };
        for (pin_name, net_name) in &component.pins {
            if model.ports.contains_key(pin_name) {
                continue;
            }
            let yaml_path = format!("/board/components/{component_id}/pins/{pin_name}");
            proposals.push(BoardYamlRepairProposal {
                id: format!("pin_not_declared_{}", proposals.len() + 1),
                finding_id: BoardYamlRepairFindingKind::PinNotDeclared
                    .finding_id()
                    .to_string(),
                status: "proposed".to_string(),
                reason_code: None,
                description: format!(
                    "Remove stray pin binding {component_id}.{pin_name} because model {} does not declare it.",
                    model.component_id
                ),
                yaml_path: yaml_path.clone(),
                affected_pins: vec![format!("{component_id}.{pin_name}")],
                edits: vec![BoardYamlRepairEdit {
                    op: "remove".to_string(),
                    path: yaml_path,
                    from: serde_json::Value::String(net_name.clone()),
                    to: serde_json::Value::Null,
                    reason: "Undeclared component pins are outside the resolved model contract and produce PIN_NOT_DECLARED warnings."
                        .to_string(),
                }],
            });
        }
    }
    Ok(proposals)
}

pub(super) fn required_pin_floating_proposals(
    project: &BoardProject,
    library: &ComponentLibrary,
) -> Result<Vec<BoardYamlRepairProposal>> {
    let mut proposals = Vec::new();
    for (component_id, component) in &project.board.components {
        let Some(model) = library.get(&component.model) else {
            continue;
        };
        for (pin_name, port) in &model.ports {
            if !port.required || component.pins.contains_key(pin_name) {
                continue;
            }
            let Some((net_name, source)) = required_pin_candidate_net(component, pin_name, port)
            else {
                continue;
            };
            let Some(net) = project.board.nets.get(net_name) else {
                continue;
            };
            let expected_kind = inferred_net_kind(&port.kind);
            if net_kind_as_yaml(&net.kind) != expected_kind {
                continue;
            }
            let yaml_path = format!("/board/components/{component_id}/pins/{pin_name}");
            proposals.push(BoardYamlRepairProposal {
                id: format!("required_pin_floating_{}", proposals.len() + 1),
                finding_id: BoardYamlRepairFindingKind::RequiredPinFloating
                    .finding_id()
                    .to_string(),
                status: "proposed".to_string(),
                reason_code: None,
                description: format!(
                    "Connect required pin {component_id}.{pin_name} to existing {expected_kind} net {net_name} from {source}."
                ),
                yaml_path: yaml_path.clone(),
                affected_pins: vec![format!("{component_id}.{pin_name}")],
                edits: vec![BoardYamlRepairEdit {
                    op: "add".to_string(),
                    path: yaml_path,
                    from: serde_json::Value::Null,
                    to: serde_json::Value::String(net_name.clone()),
                    reason:
                        "A required model pin can be connected when the component already declares a compatible net for that pin through power-domain metadata."
                            .to_string(),
                }],
            });
        }
    }
    Ok(proposals)
}

fn required_pin_candidate_net<'a>(
    component: &'a crate::board_ir::ComponentSpec,
    pin_name: &str,
    port: &Port,
) -> Option<(&'a String, &'static str)> {
    component
        .power_domains
        .get(pin_name)
        .map(|net| (net, "component power_domains"))
        .or_else(|| {
            if port.kind == PortKind::ElectricalPower {
                component
                    .power_domain
                    .as_ref()
                    .map(|net| (net, "component power_domain"))
            } else {
                None
            }
        })
}

fn blocked_model_not_found_proposal(
    index: usize,
    model_id: &str,
    yaml_path: &str,
    affected_pins: Vec<String>,
    reason_code: &str,
    description: &str,
) -> BoardYamlRepairProposal {
    BoardYamlRepairProposal {
        id: format!("model_not_found_{index}"),
        finding_id: BoardYamlRepairFindingKind::ModelNotFound
            .finding_id()
            .to_string(),
        status: "blocked".to_string(),
        reason_code: Some(reason_code.to_string()),
        description: description.to_string(),
        yaml_path: yaml_path.to_string(),
        affected_pins,
        edits: vec![BoardYamlRepairEdit {
            op: "replace".to_string(),
            path: yaml_path.to_string(),
            from: serde_json::Value::String(model_id.to_string()),
            to: serde_json::Value::Null,
            reason:
                "Unresolved models are not repaired unless the existing model string proves exactly one loaded canonical model id."
                    .to_string(),
        }],
    }
}

#[derive(Debug, Default)]
struct MissingNetProposalDraft {
    affected_pins: Vec<String>,
    inferred_kinds: BTreeSet<&'static str>,
}

fn inferred_net_kind(port_kind: &PortKind) -> &'static str {
    match port_kind {
        PortKind::ElectricalPower => "power",
        PortKind::ElectricalGround => "ground",
        PortKind::DigitalElectricalInput
        | PortKind::DigitalElectricalOutput
        | PortKind::DigitalElectricalIo
        | PortKind::Passive => "digital_or_analog",
    }
}

fn net_kind_as_yaml(kind: &NetKind) -> &'static str {
    match kind {
        NetKind::Power => "power",
        NetKind::Ground => "ground",
        NetKind::DigitalOrAnalog => "digital_or_analog",
    }
}
