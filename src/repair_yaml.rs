use crate::board_ir::{BoardProject, NetKind, load_project};
use crate::library::{PortKind, load_library};
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
    PinNotDeclared,
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
        BoardYamlRepairFindingKind::PinNotDeclared => {
            pin_not_declared_proposals(&project, &library)?
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
        let messages = repair_messages(
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
            },
        );
        let report = BoardYamlRepairReport {
            schema_version: "0.3.0".to_string(),
            project: project.project.name.clone(),
            profile: options.profile.clone(),
            finding: finding_id.to_string(),
            mode: "dry_run".to_string(),
            result: "dry_run".to_string(),
            messages,
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
    let original_finding_removed = !original_matching.is_empty() && repaired_matching.is_empty();
    let no_new_criticals = new_criticals.is_empty();
    let blocked = proposals
        .iter()
        .filter(|proposal| proposal.status == "blocked")
        .count();
    let skipped = proposals
        .iter()
        .filter(|proposal| proposal.status == "skipped")
        .count();
    let messages = repair_messages(
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
        },
    );
    let result =
        if original_finding_removed && no_new_criticals && selected > 0 && applied == selected {
            "pass"
        } else {
            "fail"
        };
    let report = BoardYamlRepairReport {
        schema_version: "0.3.0".to_string(),
        project: project.project.name.clone(),
        profile: options.profile.clone(),
        finding: finding_id.to_string(),
        mode: mode.to_string(),
        result: result.to_string(),
        messages,
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
    serde_json::from_str(&report_text)
        .with_context(|| format!("Failed to parse repair report {}.", report_path.display()))
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
                continue;
            };
            add_net(project_yaml, net_name, kind)?;
        } else if let Some((component_id, pin_name)) = component_pin_path(&proposal.yaml_path) {
            remove_component_pin(project_yaml, component_id, pin_name)?;
        } else {
            proposal.status = "skipped".to_string();
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
}

fn repair_messages(finding_id: &str, context: RepairMessageContext<'_>) -> Vec<String> {
    let mut messages = Vec::new();
    if context.dry_run {
        messages.push(
            "Dry run skipped repaired project writing and repaired validation; proposal statuses were not applied."
                .to_string(),
        );
    }
    if context.original_matching.is_empty() {
        messages.push(format!(
            "Original validation report did not contain {finding_id}; no matching finding was available to repair."
        ));
    }
    if context.proposed == 0 {
        messages.push(format!(
            "No supported {finding_id} repair proposal was generated."
        ));
    }
    if context.blocked > 0 {
        messages.push(format!(
            "{} repair proposal(s) were blocked as ambiguous or unsafe.",
            context.blocked
        ));
    }
    if context.skipped > 0 {
        if context.selective_apply {
            messages.push(format!(
                "{} repair proposal(s) were skipped because they were not selected by --proposal-id.",
                context.skipped
            ));
        } else {
            messages.push(format!(
                "{} repair proposal(s) were skipped because their YAML edit path was not applicable.",
                context.skipped
            ));
        }
    }
    if !context.dry_run
        && !context.original_matching.is_empty()
        && !context.repaired_matching.is_empty()
    {
        if context.selective_apply {
            messages.push(format!(
                "Target finding {finding_id} remained after validating the selective repaired copy; non-selected findings may still require repair."
            ));
        } else {
            messages.push(format!(
                "Target finding {finding_id} remained after validating the repaired copy."
            ));
        }
    }
    if !context.new_criticals.is_empty() {
        messages.push(format!(
            "Repaired copy introduced {} new critical finding(s).",
            context.new_criticals.len()
        ));
    }
    messages
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
        for message in &report.messages {
            text.push_str(&format!("- {message}\n"));
        }
        text.push('\n');
    }
    text.push_str("## Proposals\n\n");
    if report.proposals.is_empty() {
        text.push_str("None.\n\n");
    } else {
        for proposal in &report.proposals {
            text.push_str(&format!(
                "- `{}` `{}`: {}\n",
                proposal.id, proposal.status, proposal.description
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
            Self::PinNotDeclared => "PIN_NOT_DECLARED",
        }
    }

    pub fn as_cli_value(self) -> &'static str {
        match self {
            Self::InvalidPowerDomain => "invalid-power-domain",
            Self::NetNotFound => "net-not-found",
            Self::PinNotDeclared => "pin-not-declared",
        }
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
