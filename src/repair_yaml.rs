use crate::board_ir::{BoardProject, NetKind, load_project};
use crate::library::{PortKind, load_library};
use crate::repair_yaml_bundle_install::load_bundle_install_package_metadata;
use crate::reports::{Finding, ValidationReport};
use crate::suite::validate_and_write_project_report;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::{Mapping, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoardYamlRepairFindingKind {
    InvalidPowerDomain,
    NetNotFound,
    PowerDomainNotFound,
    PinNotDeclared,
    RequiredPinFloating,
    AnalogModelPackageMetadata,
    BundleInstallPackageMetadata,
}

#[derive(Debug, Clone)]
pub struct BoardYamlRepairOptions {
    pub project: PathBuf,
    pub profile: String,
    pub output: PathBuf,
    pub finding: BoardYamlRepairFindingKind,
    pub dry_run: bool,
    pub apply_report: Option<PathBuf>,
    pub proposal_ids: Vec<String>,
    pub bundle_install_report: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BoardYamlRepairReport {
    pub schema_version: String,
    pub project: String,
    pub profile: String,
    pub finding: String,
    pub mode: String,
    pub result: String,
    pub messages: Vec<String>,
    #[serde(default)]
    pub reason_codes: Vec<String>,
    pub summary: BoardYamlRepairSummary,
    pub original_project: String,
    pub repaired_project: Option<String>,
    pub original_report: String,
    pub repaired_report: Option<String>,
    pub proposals: Vec<BoardYamlRepairProposal>,
    pub proof: BoardYamlRepairProof,
    pub reproduction: BoardYamlRepairReproduction,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BoardYamlRepairSummary {
    pub proposed: usize,
    pub selected: usize,
    pub applied: usize,
    pub blocked: usize,
    pub skipped: usize,
    pub original_matching_findings: usize,
    pub repaired_matching_findings: usize,
    pub original_matching_criticals: usize,
    pub repaired_matching_criticals: usize,
    pub new_criticals: usize,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct BoardYamlRepairProposal {
    pub id: String,
    pub finding_id: String,
    pub status: String,
    #[serde(default)]
    pub reason_code: Option<String>,
    pub description: String,
    pub yaml_path: String,
    pub affected_pins: Vec<String>,
    pub edits: Vec<BoardYamlRepairEdit>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct BoardYamlRepairEdit {
    pub op: String,
    pub path: String,
    pub from: serde_json::Value,
    pub to: serde_json::Value,
    pub reason: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BoardYamlRepairProof {
    pub original_finding_removed: Option<bool>,
    pub no_new_criticals: Option<bool>,
    pub original_matching_findings: Vec<BoardYamlFindingEvidence>,
    pub repaired_matching_findings: Vec<BoardYamlFindingEvidence>,
    pub new_critical_findings: Vec<BoardYamlFindingEvidence>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct BoardYamlFindingEvidence {
    pub id: String,
    pub severity: String,
    pub scenario: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BoardYamlRepairReproduction {
    pub command: String,
}

pub fn run_board_yaml_repair(options: BoardYamlRepairOptions) -> Result<BoardYamlRepairReport> {
    let project_yaml_text = std::fs::read_to_string(&options.project).with_context(|| {
        format!(
            "Failed to read Board IR YAML {}.",
            options.project.display()
        )
    })?;
    let project_yaml: Value = serde_yaml_ng::from_str(&project_yaml_text).with_context(|| {
        format!(
            "Failed to parse Board IR YAML {}.",
            options.project.display()
        )
    })?;
    let project = load_project(&options.project)?;
    let (library, library_findings) = load_library(&options.project, &project);
    if library_findings
        .iter()
        .any(|finding| finding.id == "MODEL_LOAD_FAILED")
    {
        bail!("Component library did not load cleanly; refusing to generate YAML repairs.");
    }

    let original_output = options.output.join("original");
    let repaired_output = options.output.join("repaired");
    let original_command = format!(
        "circuitci validate {} --profile {} --output {}",
        options.project.display(),
        options.profile,
        original_output.display()
    );
    let original_report = validate_and_write_project_report(
        &options.project,
        &options.profile,
        &original_output,
        original_command,
    )?;

    let mut proposals = match options.finding {
        BoardYamlRepairFindingKind::InvalidPowerDomain => {
            invalid_power_domain_proposals(&project, &library)?
        }
        BoardYamlRepairFindingKind::NetNotFound => net_not_found_proposals(&project, &library)?,
        BoardYamlRepairFindingKind::PowerDomainNotFound => {
            power_domain_not_found_proposals(&project, &library)?
        }
        BoardYamlRepairFindingKind::PinNotDeclared => {
            pin_not_declared_proposals(&project, &library)?
        }
        BoardYamlRepairFindingKind::RequiredPinFloating => {
            required_pin_floating_proposals(&project, &library)?
        }
        BoardYamlRepairFindingKind::AnalogModelPackageMetadata => {
            analog_model_package_metadata_proposals(&options.project, &project, &library)?
        }
        BoardYamlRepairFindingKind::BundleInstallPackageMetadata => {
            bundle_install_package_metadata_proposals(&options.project, &project, &options)?
        }
    };
    let finding_id = options.finding.finding_id();
    let original_matching = matching_findings(&original_report, finding_id);
    let original_matching_criticals = matching_critical_findings(&original_report, finding_id);

    if options.dry_run {
        let blocked = proposals
            .iter()
            .filter(|proposal| proposal.status == "blocked")
            .count();
        let skipped = proposals
            .iter()
            .filter(|proposal| proposal.status == "skipped")
            .count();
        let repair_messages = repair_messages(
            finding_id,
            RepairMessageContext {
                original_matching: &original_matching,
                repaired_matching: &[],
                proposed: proposals.len(),
                blocked,
                skipped,
                new_criticals: &[],
                dry_run: true,
                selective_apply: false,
                requires_matching_validation_finding: options
                    .finding
                    .requires_matching_validation_finding(),
            },
        );
        let report = BoardYamlRepairReport {
            schema_version: "0.4.0".to_string(),
            project: project.project.name.clone(),
            profile: options.profile.clone(),
            finding: finding_id.to_string(),
            mode: "dry_run".to_string(),
            result: "dry_run".to_string(),
            messages: repair_messages.messages,
            reason_codes: repair_messages.reason_codes,
            summary: BoardYamlRepairSummary {
                proposed: proposals.len(),
                selected: 0,
                applied: 0,
                blocked,
                skipped,
                original_matching_findings: original_matching.len(),
                repaired_matching_findings: 0,
                original_matching_criticals: original_matching_criticals.len(),
                repaired_matching_criticals: 0,
                new_criticals: 0,
            },
            original_project: options.project.display().to_string(),
            repaired_project: None,
            original_report: original_output.join("report.json").display().to_string(),
            repaired_report: None,
            proposals,
            proof: BoardYamlRepairProof {
                original_finding_removed: None,
                no_new_criticals: None,
                original_matching_findings: original_matching,
                repaired_matching_findings: Vec::new(),
                new_critical_findings: Vec::new(),
            },
            reproduction: BoardYamlRepairReproduction {
                command: repair_reproduction_command(&options),
            },
        };
        write_repair_reports(&report, &options.output)?;
        return Ok(report);
    }

    let mode = if let Some(report_path) = &options.apply_report {
        let dry_run_report = load_apply_report(report_path)?;
        validate_apply_report(
            &dry_run_report,
            &options,
            &project,
            &original_matching,
            &proposals,
        )?;
        proposals = select_apply_report_proposals(dry_run_report.proposals, &options.proposal_ids)?;
        "apply_report"
    } else {
        "apply"
    };

    let selected = proposals
        .iter()
        .filter(|proposal| proposal.status == "proposed")
        .count();
    let mut repaired_yaml = project_yaml;
    let applied = apply_proposals(&mut repaired_yaml, &mut proposals)?;
    absolutize_relative_libraries(
        &mut repaired_yaml,
        options.project.parent().unwrap_or_else(|| Path::new(".")),
    )?;
    absolutize_relative_analog_model_paths(
        &mut repaired_yaml,
        options.project.parent().unwrap_or_else(|| Path::new(".")),
    )?;
    std::fs::create_dir_all(&repaired_output).with_context(|| {
        format!(
            "Failed to create repair output directory {}.",
            repaired_output.display()
        )
    })?;
    let repaired_project = repaired_output.join("project.yaml");
    let mut yaml_text =
        serde_yaml_ng::to_string(&repaired_yaml).context("Failed to serialize repaired YAML.")?;
    yaml_text.insert_str(
        0,
        "# Generated by CircuitCI repair-yaml; original project is unchanged.\n",
    );
    std::fs::write(&repaired_project, yaml_text).with_context(|| {
        format!(
            "Failed to write repaired project {}.",
            repaired_project.display()
        )
    })?;

    let repaired_command = format!(
        "circuitci validate {} --profile {} --output {}",
        repaired_project.display(),
        options.profile,
        repaired_output.display()
    );
    let repaired_report = validate_and_write_project_report(
        &repaired_project,
        &options.profile,
        &repaired_output,
        repaired_command,
    )?;

    let repaired_matching = matching_findings(&repaired_report, finding_id);
    let repaired_matching_criticals = matching_critical_findings(&repaired_report, finding_id);
    let new_criticals = new_critical_findings(&original_report, &repaired_report);
    let original_finding_removed = if options.finding.requires_matching_validation_finding() {
        !original_matching.is_empty() && repaired_matching.is_empty()
    } else {
        applied == selected && selected > 0
    };
    let no_new_criticals = new_criticals.is_empty();
    let blocked = proposals
        .iter()
        .filter(|proposal| proposal.status == "blocked")
        .count();
    let skipped = proposals
        .iter()
        .filter(|proposal| proposal.status == "skipped")
        .count();
    let repair_messages = repair_messages(
        finding_id,
        RepairMessageContext {
            original_matching: &original_matching,
            repaired_matching: &repaired_matching,
            proposed: proposals.len(),
            blocked,
            skipped,
            new_criticals: &new_criticals,
            dry_run: false,
            selective_apply: !options.proposal_ids.is_empty(),
            requires_matching_validation_finding: options
                .finding
                .requires_matching_validation_finding(),
        },
    );
    let result =
        if original_finding_removed && no_new_criticals && selected > 0 && applied == selected {
            "pass"
        } else {
            "fail"
        };
    let report = BoardYamlRepairReport {
        schema_version: "0.4.0".to_string(),
        project: project.project.name.clone(),
        profile: options.profile.clone(),
        finding: finding_id.to_string(),
        mode: mode.to_string(),
        result: result.to_string(),
        messages: repair_messages.messages,
        reason_codes: repair_messages.reason_codes,
        summary: BoardYamlRepairSummary {
            proposed: proposals.len(),
            selected,
            applied,
            blocked,
            skipped,
            original_matching_findings: original_matching.len(),
            repaired_matching_findings: repaired_matching.len(),
            original_matching_criticals: original_matching_criticals.len(),
            repaired_matching_criticals: repaired_matching_criticals.len(),
            new_criticals: new_criticals.len(),
        },
        original_project: options.project.display().to_string(),
        repaired_project: Some(repaired_project.display().to_string()),
        original_report: original_output.join("report.json").display().to_string(),
        repaired_report: Some(repaired_output.join("report.json").display().to_string()),
        proposals,
        proof: BoardYamlRepairProof {
            original_finding_removed: Some(original_finding_removed),
            no_new_criticals: Some(no_new_criticals),
            original_matching_findings: original_matching,
            repaired_matching_findings: repaired_matching,
            new_critical_findings: new_criticals,
        },
        reproduction: BoardYamlRepairReproduction {
            command: repair_reproduction_command(&options),
        },
    };
    write_repair_reports(&report, &options.output)?;
    Ok(report)
}

fn load_apply_report(report_path: &Path) -> Result<BoardYamlRepairReport> {
    let report_text = std::fs::read_to_string(report_path)
        .with_context(|| format!("Failed to read repair report {}.", report_path.display()))?;
    let mut report: BoardYamlRepairReport = serde_json::from_str(&report_text)
        .with_context(|| format!("Failed to parse repair report {}.", report_path.display()))?;
    for proposal in &mut report.proposals {
        if proposal.reason_code.is_none()
            && proposal.status == "blocked"
            && proposal.finding_id == BoardYamlRepairFindingKind::NetNotFound.finding_id()
            && proposal.edits.is_empty()
        {
            proposal.reason_code = Some("conflicting_inferred_net_kinds".to_string());
        }
    }
    Ok(report)
}

fn validate_apply_report(
    report: &BoardYamlRepairReport,
    options: &BoardYamlRepairOptions,
    project: &BoardProject,
    original_matching: &[BoardYamlFindingEvidence],
    current_proposals: &[BoardYamlRepairProposal],
) -> Result<()> {
    if report.mode != "dry_run" || report.result != "dry_run" {
        bail!("--apply-report requires a dry-run repair_report.json.");
    }
    let finding_id = options.finding.finding_id();
    if report.finding != finding_id {
        bail!(
            "Dry-run report finding {} does not match requested finding {finding_id}.",
            report.finding
        );
    }
    if report.profile != options.profile {
        bail!(
            "Dry-run report profile {} does not match requested profile {}.",
            report.profile,
            options.profile
        );
    }
    if report.project != project.project.name {
        bail!(
            "Dry-run report project {} does not match current project {}.",
            report.project,
            project.project.name
        );
    }
    let current_project = canonicalize_existing_path(&options.project)?;
    let reported_project = canonicalize_existing_path(Path::new(&report.original_project))?;
    if current_project != reported_project {
        bail!(
            "Dry-run report original project {} does not match requested project {}.",
            report.original_project,
            options.project.display()
        );
    }
    let report_matching: BTreeSet<_> = report
        .proof
        .original_matching_findings
        .iter()
        .cloned()
        .collect();
    let current_matching: BTreeSet<_> = original_matching.iter().cloned().collect();
    if report_matching != current_matching {
        bail!(
            "Dry-run report original matching findings no longer match current validation output."
        );
    }
    if report.proposals.iter().any(|proposal| {
        proposal.finding_id != finding_id
            || (proposal.status != "proposed" && proposal.status != "blocked")
    }) {
        bail!("Dry-run report contains proposals that are not applicable for report-driven apply.");
    }
    if !report
        .proposals
        .iter()
        .any(|proposal| proposal.status == "proposed")
    {
        bail!("Dry-run report contains no proposed edits to apply.");
    }
    if report.proposals != current_proposals {
        bail!(
            "Dry-run report proposals no longer match the current project; regenerate the dry-run report."
        );
    }
    Ok(())
}

fn select_apply_report_proposals(
    mut proposals: Vec<BoardYamlRepairProposal>,
    proposal_ids: &[String],
) -> Result<Vec<BoardYamlRepairProposal>> {
    if proposal_ids.is_empty() {
        return Ok(proposals);
    }

    let requested: BTreeSet<_> = proposal_ids.iter().map(String::as_str).collect();
    if requested.len() != proposal_ids.len() {
        bail!("--proposal-id values must be unique.");
    }
    let known: BTreeSet<_> = proposals
        .iter()
        .map(|proposal| proposal.id.as_str())
        .collect();
    for proposal_id in &requested {
        if !known.contains(proposal_id) {
            bail!("Dry-run report does not contain requested proposal id {proposal_id}.");
        }
    }

    for proposal in &mut proposals {
        if requested.contains(proposal.id.as_str()) {
            if proposal.status != "proposed" {
                bail!(
                    "Requested proposal id {} has status {} and cannot be applied.",
                    proposal.id,
                    proposal.status
                );
            }
        } else if proposal.status == "proposed" {
            proposal.status = "skipped".to_string();
            proposal.reason_code = Some("not_selected".to_string());
        }
    }

    Ok(proposals)
}

fn canonicalize_existing_path(path: &Path) -> Result<PathBuf> {
    std::fs::canonicalize(path)
        .with_context(|| format!("Failed to canonicalize path {}.", path.display()))
}

fn invalid_power_domain_proposals(
    project: &BoardProject,
    library: &crate::library::ComponentLibrary,
) -> Result<Vec<BoardYamlRepairProposal>> {
    let mut by_net: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
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
                .map(|net| net.kind.as_yaml())
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

fn net_not_found_proposals(
    project: &BoardProject,
    library: &crate::library::ComponentLibrary,
) -> Result<Vec<BoardYamlRepairProposal>> {
    let mut by_net: std::collections::BTreeMap<String, MissingNetProposalDraft> =
        std::collections::BTreeMap::new();
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

fn power_domain_not_found_proposals(
    project: &BoardProject,
    library: &crate::library::ComponentLibrary,
) -> Result<Vec<BoardYamlRepairProposal>> {
    let mut by_net: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
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

fn pin_not_declared_proposals(
    project: &BoardProject,
    library: &crate::library::ComponentLibrary,
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

fn required_pin_floating_proposals(
    project: &BoardProject,
    library: &crate::library::ComponentLibrary,
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
            if net.kind.as_yaml() != expected_kind {
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

fn analog_model_package_metadata_proposals(
    project_path: &Path,
    project: &BoardProject,
    library: &crate::library::ComponentLibrary,
) -> Result<Vec<BoardYamlRepairProposal>> {
    let mut proposals = Vec::new();
    for (scenario_index, scenario) in project.scenarios.iter().enumerate() {
        let Some(analog) = scenario.analog.as_ref() else {
            continue;
        };
        let Some(generated) = analog.generated.as_ref() else {
            continue;
        };
        let package_models =
            package_models_for_generated_components(project_path, project, library, generated)?;
        if package_models.is_empty() {
            continue;
        }
        for (model_file_index, model_file) in analog.model_files.iter().enumerate() {
            let Some(model_file_path) =
                canonicalize_project_relative_path(project_path, &model_file.path)?
            else {
                continue;
            };
            let Some(expected) = package_models
                .iter()
                .find(|metadata| metadata.canonical_model_path == model_file_path)
            else {
                continue;
            };
            let yaml_path =
                format!("/scenarios/{scenario_index}/analog/model_files/{model_file_index}");
            let mut edits = Vec::new();
            let mut conflicts = Vec::new();
            collect_package_metadata_edit(
                &mut edits,
                &mut conflicts,
                &yaml_path,
                "model_package_name",
                model_file.model_package_name.as_deref(),
                expected.model_package_name.as_deref(),
            );
            collect_package_metadata_edit(
                &mut edits,
                &mut conflicts,
                &yaml_path,
                "model_package_version",
                model_file.model_package_version.as_deref(),
                expected.model_package_version.as_deref(),
            );
            collect_package_metadata_edit(
                &mut edits,
                &mut conflicts,
                &yaml_path,
                "model_package_artifact_id",
                model_file.model_package_artifact_id.as_deref(),
                expected.model_package_artifact_id.as_deref(),
            );
            collect_package_metadata_edit(
                &mut edits,
                &mut conflicts,
                &yaml_path,
                "model_package_lock_path",
                model_file.model_package_lock_path.as_deref(),
                expected.model_package_lock_path.as_deref(),
            );
            collect_package_metadata_edit(
                &mut edits,
                &mut conflicts,
                &yaml_path,
                "model_package_lock_sha256",
                model_file.model_package_lock_sha256.as_deref(),
                expected.model_package_lock_sha256.as_deref(),
            );
            collect_package_metadata_edit(
                &mut edits,
                &mut conflicts,
                &yaml_path,
                "model_package_registry_path",
                model_file.model_package_registry_path.as_deref(),
                expected.model_package_registry_path.as_deref(),
            );
            collect_package_metadata_edit(
                &mut edits,
                &mut conflicts,
                &yaml_path,
                "model_package_registry_sha256",
                model_file.model_package_registry_sha256.as_deref(),
                expected.model_package_registry_sha256.as_deref(),
            );
            collect_package_metadata_edit(
                &mut edits,
                &mut conflicts,
                &yaml_path,
                "model_package_registry_entry",
                model_file.model_package_registry_entry.as_deref(),
                expected.model_package_registry_entry.as_deref(),
            );
            if conflicts.is_empty() && edits.is_empty() {
                continue;
            }
            let proposal_id = format!("analog_model_package_metadata_{}", proposals.len() + 1);
            if conflicts.is_empty() {
                proposals.push(BoardYamlRepairProposal {
                    id: proposal_id,
                    finding_id: BoardYamlRepairFindingKind::AnalogModelPackageMetadata
                        .finding_id()
                        .to_string(),
                    status: "proposed".to_string(),
                    reason_code: None,
                    description: format!(
                        "Add package-lock and registry metadata to generated analog model file {} in scenario {}.",
                        model_file.path, scenario.name
                    ),
                    yaml_path,
                    affected_pins: expected.components.clone(),
                    edits,
                });
            } else {
                proposals.push(BoardYamlRepairProposal {
                    id: proposal_id,
                    finding_id: BoardYamlRepairFindingKind::AnalogModelPackageMetadata
                        .finding_id()
                        .to_string(),
                    status: "blocked".to_string(),
                    reason_code: Some("package_metadata_conflict".to_string()),
                    description: format!(
                        "Not changing generated analog model file {} in scenario {} because existing package fields conflict: {}.",
                        model_file.path,
                        scenario.name,
                        conflicts.join(", ")
                    ),
                    yaml_path,
                    affected_pins: expected.components.clone(),
                    edits: Vec::new(),
                });
            }
        }
    }
    Ok(proposals)
}

fn bundle_install_package_metadata_proposals(
    project_path: &Path,
    project: &BoardProject,
    options: &BoardYamlRepairOptions,
) -> Result<Vec<BoardYamlRepairProposal>> {
    let Some(report_path) = &options.bundle_install_report else {
        bail!(
            "--bundle-install-report is required with --finding bundle-install-package-metadata."
        );
    };
    let expected = load_bundle_install_package_metadata(report_path)?;
    let mut proposals = Vec::new();
    for (scenario_index, scenario) in project.scenarios.iter().enumerate() {
        let Some(analog) = scenario.analog.as_ref() else {
            continue;
        };
        for (model_file_index, model_file) in analog.model_files.iter().enumerate() {
            let artifact_id_matches = model_file
                .model_package_artifact_id
                .as_deref()
                .is_some_and(|id| id == expected.model_package_artifact_id);
            let model_path_matches =
                canonicalize_project_relative_path(project_path, &model_file.path)?
                    .is_some_and(|path| expected.runtime_artifact_paths.contains(&path));
            if !artifact_id_matches && !model_path_matches {
                continue;
            }
            let yaml_path =
                format!("/scenarios/{scenario_index}/analog/model_files/{model_file_index}");
            let mut edits = Vec::new();
            let mut conflicts = Vec::new();
            let reason = "Bundle install report scenario_import pins can qualify this analog model file with reusable package metadata.";
            collect_metadata_edit(
                &mut edits,
                &mut conflicts,
                &yaml_path,
                "model_package_name",
                model_file.model_package_name.as_deref(),
                expected.model_package_name.as_deref(),
                reason,
            );
            collect_metadata_edit(
                &mut edits,
                &mut conflicts,
                &yaml_path,
                "model_package_version",
                model_file.model_package_version.as_deref(),
                expected.model_package_version.as_deref(),
                reason,
            );
            collect_metadata_edit(
                &mut edits,
                &mut conflicts,
                &yaml_path,
                "model_package_artifact_id",
                model_file.model_package_artifact_id.as_deref(),
                Some(expected.model_package_artifact_id.as_str()),
                reason,
            );
            collect_metadata_edit(
                &mut edits,
                &mut conflicts,
                &yaml_path,
                "model_package_lock_path",
                model_file.model_package_lock_path.as_deref(),
                Some(expected.model_package_lock_path.as_str()),
                reason,
            );
            collect_metadata_edit(
                &mut edits,
                &mut conflicts,
                &yaml_path,
                "model_package_lock_sha256",
                model_file.model_package_lock_sha256.as_deref(),
                Some(expected.model_package_lock_sha256.as_str()),
                reason,
            );
            collect_metadata_edit(
                &mut edits,
                &mut conflicts,
                &yaml_path,
                "model_package_registry_path",
                model_file.model_package_registry_path.as_deref(),
                Some(expected.model_package_registry_path.as_str()),
                reason,
            );
            collect_metadata_edit(
                &mut edits,
                &mut conflicts,
                &yaml_path,
                "model_package_registry_sha256",
                model_file.model_package_registry_sha256.as_deref(),
                Some(expected.model_package_registry_sha256.as_str()),
                reason,
            );
            collect_metadata_edit(
                &mut edits,
                &mut conflicts,
                &yaml_path,
                "model_package_registry_entry",
                model_file.model_package_registry_entry.as_deref(),
                Some(expected.model_package_registry_entry.as_str()),
                reason,
            );
            if conflicts.is_empty() && edits.is_empty() {
                continue;
            }
            let proposal_id = format!("bundle_install_package_metadata_{}", proposals.len() + 1);
            if conflicts.is_empty() {
                proposals.push(BoardYamlRepairProposal {
                    id: proposal_id,
                    finding_id: BoardYamlRepairFindingKind::BundleInstallPackageMetadata
                        .finding_id()
                        .to_string(),
                    status: "proposed".to_string(),
                    reason_code: None,
                    description: format!(
                        "Add package registry pins from bundle install report {} to analog model file {} in scenario {}.",
                        report_path.display(),
                        model_file.path,
                        scenario.name
                    ),
                    yaml_path,
                    affected_pins: vec![scenario.name.clone()],
                    edits,
                });
            } else {
                proposals.push(BoardYamlRepairProposal {
                    id: proposal_id,
                    finding_id: BoardYamlRepairFindingKind::BundleInstallPackageMetadata
                        .finding_id()
                        .to_string(),
                    status: "blocked".to_string(),
                    reason_code: Some("bundle_install_package_metadata_conflict".to_string()),
                    description: format!(
                        "Not changing analog model file {} in scenario {} because existing package fields conflict with bundle install report {}: {}.",
                        model_file.path,
                        scenario.name,
                        report_path.display(),
                        conflicts.join(", ")
                    ),
                    yaml_path,
                    affected_pins: vec![scenario.name.clone()],
                    edits: Vec::new(),
                });
            }
        }
    }
    Ok(proposals)
}

#[derive(Debug)]
struct AnalogPackageModelMetadata {
    canonical_model_path: PathBuf,
    components: Vec<String>,
    model_package_name: Option<String>,
    model_package_version: Option<String>,
    model_package_artifact_id: Option<String>,
    model_package_lock_path: Option<String>,
    model_package_lock_sha256: Option<String>,
    model_package_registry_path: Option<String>,
    model_package_registry_sha256: Option<String>,
    model_package_registry_entry: Option<String>,
}

fn package_models_for_generated_components(
    project_path: &Path,
    project: &BoardProject,
    library: &crate::library::ComponentLibrary,
    generated: &crate::board_ir::AnalogGeneratedNetlist,
) -> Result<Vec<AnalogPackageModelMetadata>> {
    let mut models = Vec::new();
    let mut seen = BTreeSet::new();
    for component_id in &generated.components {
        let Some(component) = project.board.components.get(component_id) else {
            continue;
        };
        let Some(model) = library.get(&component.model) else {
            continue;
        };
        let Some(spice) = model.simulation.spice.as_ref() else {
            continue;
        };
        if spice.model_package_name.is_none()
            && spice.model_package_lock_path.is_none()
            && spice.model_package_registry_path.is_none()
        {
            continue;
        }
        let Some(canonical_model_path) =
            canonicalize_project_relative_path(project_path, &spice.model_path)?
        else {
            continue;
        };
        if !seen.insert(canonical_model_path.clone()) {
            continue;
        }
        models.push(AnalogPackageModelMetadata {
            canonical_model_path,
            components: vec![component_id.clone()],
            model_package_name: spice.model_package_name.clone(),
            model_package_version: spice.model_package_version.clone(),
            model_package_artifact_id: spice.model_package_artifact_id.clone(),
            model_package_lock_path: project_relative_existing_path(
                project_path,
                spice.model_package_lock_path.as_deref(),
            )?,
            model_package_lock_sha256: spice.model_package_lock_sha256.clone(),
            model_package_registry_path: project_relative_existing_path(
                project_path,
                spice.model_package_registry_path.as_deref(),
            )?,
            model_package_registry_sha256: spice.model_package_registry_sha256.clone(),
            model_package_registry_entry: spice.model_package_registry_entry.clone(),
        });
    }
    Ok(models)
}

fn collect_package_metadata_edit(
    edits: &mut Vec<BoardYamlRepairEdit>,
    conflicts: &mut Vec<String>,
    yaml_path: &str,
    field: &str,
    current: Option<&str>,
    expected: Option<&str>,
) {
    collect_metadata_edit(
        edits,
        conflicts,
        yaml_path,
        field,
        current,
        expected,
        &format!(
            "Generated analog model file can import package metadata from component-library simulation.spice.{field}."
        ),
    );
}

fn collect_metadata_edit(
    edits: &mut Vec<BoardYamlRepairEdit>,
    conflicts: &mut Vec<String>,
    yaml_path: &str,
    field: &str,
    current: Option<&str>,
    expected: Option<&str>,
    reason: &str,
) {
    let Some(expected) = expected else {
        return;
    };
    match current {
        Some(current) if current == expected => {}
        Some(current) => conflicts.push(format!("{field}={current} expected {expected}")),
        None => edits.push(BoardYamlRepairEdit {
            op: "add".to_string(),
            path: format!("{yaml_path}/{field}"),
            from: serde_json::Value::Null,
            to: serde_json::Value::String(expected.to_string()),
            reason: reason.to_string(),
        }),
    }
}

fn project_relative_existing_path(
    project_path: &Path,
    path: Option<&str>,
) -> Result<Option<String>> {
    let Some(path) = path else {
        return Ok(None);
    };
    let Some(canonical_path) = canonicalize_project_relative_path(project_path, path)? else {
        return Ok(None);
    };
    let project_dir = project_path.parent().unwrap_or_else(|| Path::new("."));
    Ok(Some(
        relative_path(&canonicalize_existing_path(project_dir)?, &canonical_path)
            .unwrap_or(canonical_path)
            .to_string_lossy()
            .replace('\\', "/"),
    ))
}

fn canonicalize_project_relative_path(project_path: &Path, path: &str) -> Result<Option<PathBuf>> {
    let path = Path::new(path);
    if path.is_absolute() {
        return Ok(Some(canonicalize_existing_path(path)?));
    }
    let project_dir = project_path.parent().unwrap_or_else(|| Path::new("."));
    let project_dir = canonicalize_existing_path(project_dir)?;
    for base in project_dir.ancestors() {
        let candidate = base.join(path);
        if candidate.exists() {
            return Ok(Some(canonicalize_existing_path(&candidate)?));
        }
    }
    Ok(None)
}

fn relative_path(from_dir: &Path, to: &Path) -> Option<PathBuf> {
    let from_components = from_dir.components().collect::<Vec<_>>();
    let to_components = to.components().collect::<Vec<_>>();
    let common = from_components
        .iter()
        .zip(&to_components)
        .take_while(|(left, right)| left == right)
        .count();
    if common == 0 {
        return None;
    }

    let mut path = PathBuf::new();
    for component in &from_components[common..] {
        match component {
            std::path::Component::Normal(_) => path.push(".."),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => path.push(".."),
            std::path::Component::Prefix(_) | std::path::Component::RootDir => return None,
        }
    }
    for component in &to_components[common..] {
        match component {
            std::path::Component::Normal(value) => path.push(value),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => path.push(".."),
            std::path::Component::Prefix(_) | std::path::Component::RootDir => return None,
        }
    }
    Some(if path.as_os_str().is_empty() {
        PathBuf::from(".")
    } else {
        path
    })
}

fn required_pin_candidate_net<'a>(
    component: &'a crate::board_ir::ComponentSpec,
    pin_name: &str,
    port: &crate::library::Port,
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

fn apply_proposals(
    project_yaml: &mut Value,
    proposals: &mut [BoardYamlRepairProposal],
) -> Result<usize> {
    let mut applied = 0;
    for proposal in proposals {
        if proposal.status != "proposed" {
            continue;
        }
        if let Some(net_name) = proposal
            .yaml_path
            .strip_prefix("/board/nets/")
            .and_then(|rest| rest.strip_suffix("/kind"))
        {
            replace_net_kind(project_yaml, net_name, "power")?;
        } else if let Some(net_name) = proposal.yaml_path.strip_prefix("/board/nets/") {
            let Some(kind) = proposal
                .edits
                .first()
                .and_then(|edit| edit.to.get("kind"))
                .and_then(|kind| kind.as_str())
            else {
                proposal.status = "skipped".to_string();
                proposal.reason_code = Some("unapplicable_yaml_path".to_string());
                continue;
            };
            add_net(project_yaml, net_name, kind)?;
        } else if let Some((component_id, pin_name)) = component_pin_path(&proposal.yaml_path) {
            match proposal.edits.first().map(|edit| edit.op.as_str()) {
                Some("add") => {
                    let Some(net_name) = proposal.edits.first().and_then(|edit| edit.to.as_str())
                    else {
                        proposal.status = "skipped".to_string();
                        proposal.reason_code = Some("unapplicable_yaml_path".to_string());
                        continue;
                    };
                    add_component_pin(project_yaml, component_id, pin_name, net_name)?;
                }
                Some("remove") => remove_component_pin(project_yaml, component_id, pin_name)?,
                _ => {
                    proposal.status = "skipped".to_string();
                    proposal.reason_code = Some("unapplicable_yaml_path".to_string());
                    continue;
                }
            }
        } else if proposal.yaml_path.starts_with("/scenarios/")
            && proposal.yaml_path.contains("/analog/model_files/")
        {
            let mut applied_metadata_edits = 0;
            for edit in &proposal.edits {
                let Some((scenario_index, model_file_index, field)) =
                    analog_model_file_metadata_path(&edit.path)
                else {
                    continue;
                };
                let Some(value) = edit.to.as_str() else {
                    continue;
                };
                add_analog_model_file_metadata(
                    project_yaml,
                    scenario_index,
                    model_file_index,
                    field,
                    value,
                )?;
                applied_metadata_edits += 1;
            }
            if applied_metadata_edits != proposal.edits.len() {
                proposal.status = "skipped".to_string();
                proposal.reason_code = Some("unapplicable_yaml_path".to_string());
                continue;
            }
        } else {
            proposal.status = "skipped".to_string();
            proposal.reason_code = Some("unapplicable_yaml_path".to_string());
            continue;
        }
        proposal.status = "applied".to_string();
        applied += 1;
    }
    Ok(applied)
}

fn component_pin_path(path: &str) -> Option<(&str, &str)> {
    let rest = path.strip_prefix("/board/components/")?;
    let (component_id, pin_path) = rest.split_once("/pins/")?;
    if component_id.is_empty() || pin_path.is_empty() || pin_path.contains('/') {
        return None;
    }
    Some((component_id, pin_path))
}

fn analog_model_file_metadata_path(path: &str) -> Option<(usize, usize, &str)> {
    let rest = path.strip_prefix("/scenarios/")?;
    let (scenario_index, rest) = rest.split_once("/analog/model_files/")?;
    let (model_file_index, field) = rest.split_once('/')?;
    if field.is_empty() || field.contains('/') {
        return None;
    }
    Some((
        scenario_index.parse().ok()?,
        model_file_index.parse().ok()?,
        field,
    ))
}

fn add_analog_model_file_metadata(
    project_yaml: &mut Value,
    scenario_index: usize,
    model_file_index: usize,
    field: &str,
    value: &str,
) -> Result<()> {
    let root = project_yaml
        .as_mapping_mut()
        .context("Board IR project must be a YAML object.")?;
    let scenarios = root
        .get_mut(Value::String("scenarios".to_string()))
        .context("Board IR scenarios field is missing.")?
        .as_sequence_mut()
        .context("Board IR scenarios field must be a list.")?;
    let scenario = scenarios
        .get_mut(scenario_index)
        .with_context(|| format!("Board IR scenario index {scenario_index} is missing."))?
        .as_mapping_mut()
        .context("Board IR scenario must be an object.")?;
    let analog = get_mapping_field_mut(scenario, "analog")?;
    let model_files = analog
        .get_mut(Value::String("model_files".to_string()))
        .context("Board IR analog.model_files field is missing.")?
        .as_sequence_mut()
        .context("Board IR analog.model_files field must be a list.")?;
    let model_file = model_files
        .get_mut(model_file_index)
        .with_context(|| format!("Board IR model file index {model_file_index} is missing."))?
        .as_mapping_mut()
        .context("Board IR analog.model_files entries must be objects.")?;
    let key = Value::String(field.to_string());
    if model_file.contains_key(&key) {
        bail!("Board IR analog model file metadata field {field} already exists.");
    }
    model_file.insert(key, Value::String(value.to_string()));
    Ok(())
}

fn replace_net_kind(project_yaml: &mut Value, net_name: &str, kind: &str) -> Result<()> {
    let root = project_yaml
        .as_mapping_mut()
        .context("Board IR project must be a YAML object.")?;
    let board = get_mapping_field_mut(root, "board")?;
    let nets = get_mapping_field_mut(board, "nets")?;
    let net = nets
        .get_mut(Value::String(net_name.to_string()))
        .with_context(|| format!("Board IR net {net_name} is missing."))?
        .as_mapping_mut()
        .with_context(|| format!("Board IR net {net_name} must be an object."))?;
    net.insert(
        Value::String("kind".to_string()),
        Value::String(kind.to_string()),
    );
    Ok(())
}

fn remove_component_pin(
    project_yaml: &mut Value,
    component_id: &str,
    pin_name: &str,
) -> Result<()> {
    let root = project_yaml
        .as_mapping_mut()
        .context("Board IR project must be a YAML object.")?;
    let board = get_mapping_field_mut(root, "board")?;
    let components = get_mapping_field_mut(board, "components")?;
    let component = components
        .get_mut(Value::String(component_id.to_string()))
        .with_context(|| format!("Board IR component {component_id} is missing."))?
        .as_mapping_mut()
        .with_context(|| format!("Board IR component {component_id} must be an object."))?;
    let pins = get_mapping_field_mut(component, "pins")?;
    let removed = pins.remove(Value::String(pin_name.to_string()));
    if removed.is_none() {
        bail!("Board IR pin binding {component_id}.{pin_name} is missing.");
    }
    Ok(())
}

fn add_component_pin(
    project_yaml: &mut Value,
    component_id: &str,
    pin_name: &str,
    net_name: &str,
) -> Result<()> {
    let root = project_yaml
        .as_mapping_mut()
        .context("Board IR project must be a YAML object.")?;
    let board = get_mapping_field_mut(root, "board")?;
    let components = get_mapping_field_mut(board, "components")?;
    let component = components
        .get_mut(Value::String(component_id.to_string()))
        .with_context(|| format!("Board IR component {component_id} is missing."))?
        .as_mapping_mut()
        .with_context(|| format!("Board IR component {component_id} must be an object."))?;
    let pins = ensure_mapping_field_mut(component, "pins")?;
    let pin_key = Value::String(pin_name.to_string());
    if pins.contains_key(&pin_key) {
        bail!("Board IR pin binding {component_id}.{pin_name} already exists.");
    }
    pins.insert(pin_key, Value::String(net_name.to_string()));
    Ok(())
}

fn add_net(project_yaml: &mut Value, net_name: &str, kind: &str) -> Result<()> {
    let root = project_yaml
        .as_mapping_mut()
        .context("Board IR project must be a YAML object.")?;
    let board = ensure_mapping_field_mut(root, "board")?;
    let nets = ensure_mapping_field_mut(board, "nets")?;
    let net_key = Value::String(net_name.to_string());
    if nets.contains_key(&net_key) {
        bail!("Board IR net {net_name} already exists.");
    }
    let mut net = Mapping::new();
    net.insert(
        Value::String("kind".to_string()),
        Value::String(kind.to_string()),
    );
    nets.insert(net_key, Value::Mapping(net));
    Ok(())
}

fn get_mapping_field_mut<'a>(mapping: &'a mut Mapping, key: &str) -> Result<&'a mut Mapping> {
    mapping
        .get_mut(Value::String(key.to_string()))
        .with_context(|| format!("Board IR field {key} is missing."))?
        .as_mapping_mut()
        .with_context(|| format!("Board IR field {key} must be an object."))
}

fn ensure_mapping_field_mut<'a>(mapping: &'a mut Mapping, key: &str) -> Result<&'a mut Mapping> {
    let key_value = Value::String(key.to_string());
    if !mapping.contains_key(&key_value) {
        mapping.insert(key_value.clone(), Value::Mapping(Mapping::new()));
    }
    mapping
        .get_mut(&key_value)
        .expect("field was inserted when absent")
        .as_mapping_mut()
        .with_context(|| format!("Board IR field {key} must be an object."))
}

fn absolutize_relative_libraries(project_yaml: &mut Value, project_dir: &Path) -> Result<()> {
    let mapping = project_yaml
        .as_mapping_mut()
        .context("Board IR project must be a YAML object.")?;
    let Some(libraries) = mapping.get_mut(Value::String("libraries".to_string())) else {
        return Ok(());
    };
    let libraries = libraries
        .as_sequence_mut()
        .context("Board IR field libraries must be a list.")?;
    for library in libraries {
        let Some(path_text) = library.as_str() else {
            bail!("Board IR libraries entries must be strings.");
        };
        let path = Path::new(path_text);
        if path.is_absolute() {
            continue;
        }
        let resolved = normalize_path(&project_dir.join(path));
        let absolute = std::fs::canonicalize(&resolved).unwrap_or(resolved);
        *library = Value::String(absolute.to_string_lossy().to_string());
    }
    Ok(())
}

fn absolutize_relative_analog_model_paths(
    project_yaml: &mut Value,
    project_dir: &Path,
) -> Result<()> {
    let Some(scenarios) = project_yaml
        .as_mapping_mut()
        .and_then(|mapping| mapping.get_mut(Value::String("scenarios".to_string())))
        .and_then(Value::as_sequence_mut)
    else {
        return Ok(());
    };
    for scenario in scenarios {
        let Some(model_files) = scenario
            .as_mapping_mut()
            .and_then(|mapping| mapping.get_mut(Value::String("analog".to_string())))
            .and_then(Value::as_mapping_mut)
            .and_then(|analog| analog.get_mut(Value::String("model_files".to_string())))
            .and_then(Value::as_sequence_mut)
        else {
            continue;
        };
        for model_file in model_files {
            let Some(model_file) = model_file.as_mapping_mut() else {
                continue;
            };
            for field in [
                "path",
                "source_path",
                "conformance_artifact",
                "model_package_lock_path",
                "model_package_registry_path",
            ] {
                absolutize_optional_path_field(model_file, field, project_dir)?;
            }
        }
    }
    Ok(())
}

fn absolutize_optional_path_field(
    mapping: &mut Mapping,
    field: &str,
    project_dir: &Path,
) -> Result<()> {
    let key = Value::String(field.to_string());
    let Some(value) = mapping.get_mut(&key) else {
        return Ok(());
    };
    let Some(path_text) = value.as_str() else {
        bail!("Board IR analog model file field {field} must be a string.");
    };
    let path = Path::new(path_text);
    if path.is_absolute() {
        return Ok(());
    }
    let resolved = normalize_path(&project_dir.join(path));
    let absolute = std::fs::canonicalize(&resolved).unwrap_or(resolved);
    *value = Value::String(absolute.to_string_lossy().to_string());
    Ok(())
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn matching_findings(report: &ValidationReport, finding_id: &str) -> Vec<BoardYamlFindingEvidence> {
    report
        .failures
        .iter()
        .chain(report.warnings.iter())
        .chain(report.infos.iter())
        .filter(|finding| finding.id == finding_id)
        .map(finding_evidence)
        .collect()
}

fn matching_critical_findings(
    report: &ValidationReport,
    finding_id: &str,
) -> Vec<BoardYamlFindingEvidence> {
    report
        .failures
        .iter()
        .filter(|finding| finding.id == finding_id)
        .map(finding_evidence)
        .collect()
}

fn new_critical_findings(
    original: &ValidationReport,
    repaired: &ValidationReport,
) -> Vec<BoardYamlFindingEvidence> {
    let original_keys: BTreeSet<_> = original.failures.iter().map(finding_key).collect();
    repaired
        .failures
        .iter()
        .filter(|finding| !original_keys.contains(&finding_key(finding)))
        .map(finding_evidence)
        .collect()
}

fn finding_key(finding: &Finding) -> String {
    format!("{}|{}|{}", finding.id, finding.scenario, finding.message)
}

fn finding_evidence(finding: &Finding) -> BoardYamlFindingEvidence {
    BoardYamlFindingEvidence {
        id: finding.id.clone(),
        severity: match finding.severity {
            crate::reports::Severity::Critical => "critical",
            crate::reports::Severity::Warning => "warning",
            crate::reports::Severity::Info => "info",
        }
        .to_string(),
        scenario: finding.scenario.clone(),
        message: finding.message.clone(),
    }
}

struct RepairMessageContext<'a> {
    original_matching: &'a [BoardYamlFindingEvidence],
    repaired_matching: &'a [BoardYamlFindingEvidence],
    proposed: usize,
    blocked: usize,
    skipped: usize,
    new_criticals: &'a [BoardYamlFindingEvidence],
    dry_run: bool,
    selective_apply: bool,
    requires_matching_validation_finding: bool,
}

struct RepairMessages {
    messages: Vec<String>,
    reason_codes: Vec<String>,
}

fn repair_messages(finding_id: &str, context: RepairMessageContext<'_>) -> RepairMessages {
    let mut messages = Vec::new();
    let mut reason_codes = Vec::new();
    if context.dry_run {
        messages.push(
            "Dry run skipped repaired project writing and repaired validation; proposal statuses were not applied."
                .to_string(),
        );
        reason_codes.push("dry_run_not_validated".to_string());
    }
    if context.requires_matching_validation_finding && context.original_matching.is_empty() {
        messages.push(format!(
            "Original validation report did not contain {finding_id}; no matching finding was available to repair."
        ));
        reason_codes.push("target_finding_absent".to_string());
    }
    if context.proposed == 0 {
        messages.push(format!(
            "No supported {finding_id} repair proposal was generated."
        ));
        reason_codes.push("no_supported_proposal".to_string());
    }
    if context.blocked > 0 {
        messages.push(format!(
            "{} repair proposal(s) were blocked as ambiguous or unsafe.",
            context.blocked
        ));
        reason_codes.push("proposal_blocked".to_string());
    }
    if context.skipped > 0 {
        if context.selective_apply {
            messages.push(format!(
                "{} repair proposal(s) were skipped because they were not selected by --proposal-id.",
                context.skipped
            ));
            reason_codes.push("proposal_skipped_not_selected".to_string());
        } else {
            messages.push(format!(
                "{} repair proposal(s) were skipped because their YAML edit path was not applicable.",
                context.skipped
            ));
            reason_codes.push("proposal_skipped_unapplicable_path".to_string());
        }
    }
    if context.requires_matching_validation_finding
        && !context.dry_run
        && !context.original_matching.is_empty()
        && !context.repaired_matching.is_empty()
    {
        if context.selective_apply {
            messages.push(format!(
                "Target finding {finding_id} remained after validating the selective repaired copy; non-selected findings may still require repair."
            ));
            reason_codes.push("target_finding_remains_selective_apply".to_string());
        } else {
            messages.push(format!(
                "Target finding {finding_id} remained after validating the repaired copy."
            ));
            reason_codes.push("target_finding_remains".to_string());
        }
    }
    if !context.new_criticals.is_empty() {
        messages.push(format!(
            "Repaired copy introduced {} new critical finding(s).",
            context.new_criticals.len()
        ));
        reason_codes.push("new_criticals_introduced".to_string());
    }
    RepairMessages {
        messages,
        reason_codes,
    }
}

fn repair_reproduction_command(options: &BoardYamlRepairOptions) -> String {
    let mut command = format!(
        "circuitci repair-yaml {} --profile {} --output {} --finding {}",
        options.project.display(),
        options.profile,
        options.output.display(),
        options.finding.as_cli_value()
    );
    if options.dry_run {
        command.push_str(" --dry-run");
    }
    if let Some(apply_report) = &options.apply_report {
        command.push_str(&format!(" --apply-report {}", apply_report.display()));
    }
    for proposal_id in &options.proposal_ids {
        command.push_str(&format!(" --proposal-id {proposal_id}"));
    }
    if let Some(report) = &options.bundle_install_report {
        command.push_str(&format!(" --bundle-install-report {}", report.display()));
    }
    command
}

fn write_repair_reports(report: &BoardYamlRepairReport, output: &Path) -> Result<()> {
    std::fs::create_dir_all(output)?;
    std::fs::write(
        output.join("repair_report.json"),
        serde_json::to_string_pretty(report)?,
    )?;
    std::fs::write(
        output.join("repair_report.md"),
        markdown_repair_report(report),
    )?;
    Ok(())
}

fn markdown_repair_report(report: &BoardYamlRepairReport) -> String {
    let mut text = String::new();
    text.push_str(&format!("# CircuitCI YAML Repair: {}\n\n", report.project));
    text.push_str(&format!(
        "- Result: `{}`\n- Mode: `{}`\n- Finding: `{}`\n- Proposed: {}\n- Selected: {}\n- Applied: {}\n- Blocked: {}\n- Skipped: {}\n- Original matching findings: {}\n- Repaired matching findings: {}\n- Original matching criticals: {}\n- Repaired matching criticals: {}\n- New criticals: {}\n\n",
        report.result,
        report.mode,
        report.finding,
        report.summary.proposed,
        report.summary.selected,
        report.summary.applied,
        report.summary.blocked,
        report.summary.skipped,
        report.summary.original_matching_findings,
        report.summary.repaired_matching_findings,
        report.summary.original_matching_criticals,
        report.summary.repaired_matching_criticals,
        report.summary.new_criticals
    ));
    text.push_str("## Messages\n\n");
    if report.messages.is_empty() {
        text.push_str("None.\n\n");
    } else {
        for (index, message) in report.messages.iter().enumerate() {
            if let Some(code) = report.reason_codes.get(index) {
                text.push_str(&format!("- `{code}`: {message}\n"));
            } else {
                text.push_str(&format!("- {message}\n"));
            }
        }
        text.push('\n');
    }
    text.push_str("## Proposals\n\n");
    if report.proposals.is_empty() {
        text.push_str("None.\n\n");
    } else {
        for proposal in &report.proposals {
            text.push_str(&format!(
                "- `{}` `{}` `{}`: {}\n",
                proposal.id,
                proposal.status,
                proposal.reason_code.as_deref().unwrap_or("none"),
                proposal.description
            ));
            for edit in &proposal.edits {
                text.push_str(&format!(
                    "  - `{}` `{}`: `{}` -> `{}`\n",
                    edit.op, edit.path, edit.from, edit.to
                ));
            }
        }
        text.push('\n');
    }
    text.push_str("## Proof\n\n");
    text.push_str(&format!(
        "- Original finding removed: `{}`\n- No new criticals: `{}`\n- Original report: `{}`\n- Repaired report: `{}`\n\n",
        optional_bool_markdown(report.proof.original_finding_removed),
        optional_bool_markdown(report.proof.no_new_criticals),
        report.original_report,
        report.repaired_report.as_deref().unwrap_or("not generated")
    ));
    text.push_str("## Reproduction\n\n");
    text.push_str(&format!("```bash\n{}\n```\n", report.reproduction.command));
    text
}

fn optional_bool_markdown(value: Option<bool>) -> &'static str {
    match value {
        Some(true) => "true",
        Some(false) => "false",
        None => "not evaluated",
    }
}

impl BoardYamlRepairFindingKind {
    pub fn finding_id(self) -> &'static str {
        match self {
            Self::InvalidPowerDomain => "INVALID_POWER_DOMAIN",
            Self::NetNotFound => "NET_NOT_FOUND",
            Self::PowerDomainNotFound => "POWER_DOMAIN_NOT_FOUND",
            Self::PinNotDeclared => "PIN_NOT_DECLARED",
            Self::RequiredPinFloating => "REQUIRED_PIN_FLOATING",
            Self::AnalogModelPackageMetadata => "ANALOG_MODEL_PACKAGE_METADATA",
            Self::BundleInstallPackageMetadata => "BUNDLE_INSTALL_PACKAGE_METADATA",
        }
    }

    pub fn as_cli_value(self) -> &'static str {
        match self {
            Self::InvalidPowerDomain => "invalid-power-domain",
            Self::NetNotFound => "net-not-found",
            Self::PowerDomainNotFound => "power-domain-not-found",
            Self::PinNotDeclared => "pin-not-declared",
            Self::RequiredPinFloating => "required-pin-floating",
            Self::AnalogModelPackageMetadata => "analog-model-package-metadata",
            Self::BundleInstallPackageMetadata => "bundle-install-package-metadata",
        }
    }

    fn requires_matching_validation_finding(self) -> bool {
        !matches!(
            self,
            Self::AnalogModelPackageMetadata | Self::BundleInstallPackageMetadata
        )
    }
}

impl NetKind {
    fn as_yaml(&self) -> &'static str {
        match self {
            Self::Power => "power",
            Self::Ground => "ground",
            Self::DigitalOrAnalog => "digital_or_analog",
        }
    }
}
