use crate::repair_yaml::{BoardYamlRepairFindingKind, BoardYamlRepairOptions};
use crate::reports::write_suite_reports;
use crate::suite::{run_suite, validate_and_write_project_report};
use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use serde_yaml_ng::{Mapping, Value};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "circuitci",
    version,
    about = "Agent-native embedded board validation runtime"
)]
pub struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init,
    Validate {
        project: PathBuf,
        #[arg(long, default_value = "iot_basic_v0")]
        profile: String,
        #[arg(long, short = 'o', default_value = "out")]
        output: PathBuf,
        #[arg(long)]
        json: Option<PathBuf>,
        #[arg(long)]
        no_open_ui: bool,
    },
    ValidateSuite {
        manifest: PathBuf,
        #[arg(long, short = 'o', default_value = "out/suite")]
        output: PathBuf,
    },
    VerifyModelPackage {
        lock: PathBuf,
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long = "registry-entry")]
        registry_entry: Option<String>,
        #[arg(
            long,
            short = 'o',
            default_value = "out/model_package_verification.json"
        )]
        output: PathBuf,
    },
    VerifyModelPackageBundle {
        bundle: PathBuf,
        #[arg(
            long,
            short = 'o',
            default_value = "out/model_package_bundle_verification.json"
        )]
        output: PathBuf,
    },
    InstallModelPackageBundle {
        bundle: PathBuf,
        #[arg(long = "install-dir")]
        install_dir: PathBuf,
        #[arg(long = "registry-output")]
        registry_output: Option<PathBuf>,
        #[arg(long = "registry-entry")]
        registry_entry: Option<String>,
        #[arg(long = "registry-artifact-id")]
        registry_artifact_id: Option<String>,
        #[arg(
            long,
            short = 'o',
            default_value = "out/model_package_bundle_install.json"
        )]
        output: PathBuf,
    },
    ImportModelPackageBundle {
        bundle: PathBuf,
        #[arg(long)]
        project: PathBuf,
        #[arg(long, default_value = "iot_basic_v0")]
        profile: String,
        #[arg(long = "install-dir")]
        install_dir: PathBuf,
        #[arg(long = "registry-output")]
        registry_output: Option<PathBuf>,
        #[arg(long = "registry-entry")]
        registry_entry: Option<String>,
        #[arg(long = "registry-artifact-id")]
        registry_artifact_id: Option<String>,
        #[arg(long, short = 'o', default_value = "out/model_package_bundle_import")]
        output: PathBuf,
    },
    ExportModelPackage {
        #[arg(long = "package-name")]
        package_name: String,
        #[arg(long = "package-version")]
        package_version: String,
        #[arg(long = "package-artifact")]
        package_artifacts: Vec<String>,
        #[arg(long = "artifact-id")]
        artifact_id: Option<String>,
        #[arg(long)]
        artifact: Option<PathBuf>,
        #[arg(long = "artifact-format")]
        artifact_format: Option<String>,
        #[arg(long)]
        compiler: Option<String>,
        #[arg(long, short = 'o')]
        output: PathBuf,
        #[arg(long = "registry-output")]
        registry_output: Option<PathBuf>,
        #[arg(long = "registry-entry")]
        registry_entry: Option<String>,
        #[arg(long = "registry-artifact-id")]
        registry_artifact_id: Option<String>,
    },
    ExportModelConformanceReport {
        #[arg(long)]
        report: PathBuf,
        #[arg(long = "package-name")]
        package_name: String,
        #[arg(long = "package-version")]
        package_version: String,
        #[arg(long = "artifact-id")]
        artifact_id: String,
        #[arg(long = "runtime-artifact")]
        runtime_artifact: PathBuf,
        #[arg(long = "check-name")]
        check_name: String,
        #[arg(long)]
        analysis: String,
        #[arg(long)]
        solver: Option<String>,
        #[arg(long, short = 'o')]
        output: PathBuf,
    },
    ExportModelPackageBundle {
        lock: PathBuf,
        #[arg(long)]
        registry: Option<PathBuf>,
        #[arg(long = "registry-entry")]
        registry_entry: Option<String>,
        #[arg(long, short = 'o')]
        output: PathBuf,
    },
    MergeModelPackageRegistry {
        #[arg(long)]
        base: Option<PathBuf>,
        #[arg(long = "input")]
        inputs: Vec<PathBuf>,
        #[arg(long, short = 'o')]
        output: PathBuf,
    },
    RepairYaml {
        project: PathBuf,
        #[arg(long, default_value = "iot_basic_v0")]
        profile: String,
        #[arg(long, short = 'o', default_value = "out/repair")]
        output: PathBuf,
        #[arg(long, value_enum, default_value_t = RepairYamlFinding::InvalidPowerDomain)]
        finding: RepairYamlFinding,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        apply_report: Option<PathBuf>,
        #[arg(long = "proposal-id")]
        proposal_ids: Vec<String>,
        #[arg(long = "bundle-install-report")]
        bundle_install_report: Option<PathBuf>,
    },
    SuggestScenarios {
        project: PathBuf,
        #[arg(long)]
        profile: Option<String>,
        #[arg(long, short = 'o', default_value = "out/scenario_suggestions.yaml")]
        output: PathBuf,
    },
    #[cfg(feature = "gui")]
    ExportSketchSvg {
        project: PathBuf,
        #[arg(long, short = 'o')]
        output: PathBuf,
        #[arg(long, default_value_t = 1280)]
        width: u32,
        #[arg(long, default_value_t = 820)]
        height: u32,
    },
    SetManufacturingMetadata {
        project: PathBuf,
        #[arg(long, short = 'o')]
        output: PathBuf,
        #[arg(long)]
        stencil_thickness_mm: Option<f64>,
        #[arg(long)]
        min_drill_edge_clearance_mm: Option<f64>,
        #[arg(long)]
        min_slot_edge_clearance_mm: Option<f64>,
        #[arg(long)]
        min_paste_area_ratio: Option<f64>,
        #[arg(long)]
        max_paste_area_ratio: Option<f64>,
        #[arg(long)]
        min_solder_paste_spacing_mm: Option<f64>,
        #[arg(long)]
        max_stitch_via_distance_mm: Option<f64>,
        #[arg(long)]
        source: Option<String>,
    },
    ImportManufacturingMetadata {
        #[arg(long)]
        project: PathBuf,
        #[arg(long)]
        metadata: PathBuf,
        #[arg(long, short = 'o')]
        output: PathBuf,
        #[arg(long)]
        manifest: Option<PathBuf>,
        #[arg(long)]
        source: Option<String>,
        #[arg(long)]
        allow_unknown_fields: bool,
    },
    ImportSpice {
        deck: PathBuf,
        #[arg(long, short = 'o')]
        output: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, value_enum, default_value_t = ImportBackend::Auto)]
        backend: ImportBackend,
        #[arg(long, default_value_t = 1000.0)]
        stop_time_us: f64,
        #[arg(long, default_value_t = 1.0)]
        max_step_us: f64,
    },
    ImportKicadNetlist {
        netlist: PathBuf,
        #[arg(long, short = 'o')]
        output: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, default_value = "generic.schematic.imported_component")]
        default_model: String,
        #[arg(long)]
        mapping: Option<PathBuf>,
    },
    ImportKicadSchematic {
        schematic: PathBuf,
        #[arg(long, short = 'o')]
        output: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, default_value = "generic.schematic.imported_component")]
        default_model: String,
        #[arg(long)]
        mapping: Option<PathBuf>,
    },
    ImportKicadPcb {
        pcb: PathBuf,
        #[arg(long)]
        project: PathBuf,
        #[arg(long, short = 'o')]
        output: PathBuf,
    },
    ImportJlcAssembly {
        #[arg(long)]
        bom: PathBuf,
        #[arg(long)]
        placement: PathBuf,
        #[arg(long, short = 'o')]
        output: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        manifest: Option<PathBuf>,
        #[arg(long, default_value = "generic.schematic.imported_component")]
        default_model: String,
    },
    InspectEasyedaPro {
        eprj2: PathBuf,
        #[arg(long, short = 'o')]
        output: PathBuf,
        #[arg(long)]
        manifest: Option<PathBuf>,
    },
    ImportEasyedaFlyingProbe {
        json: PathBuf,
        #[arg(long)]
        project: PathBuf,
        #[arg(long, short = 'o')]
        output: PathBuf,
        #[arg(long, default_value = "generic.schematic.imported_component")]
        default_model: String,
    },
    ImportGerberOutline {
        gerber: PathBuf,
        #[arg(long)]
        project: PathBuf,
        #[arg(long, short = 'o')]
        output: PathBuf,
    },
    ImportGerberCopper {
        gerber: PathBuf,
        #[arg(long)]
        project: PathBuf,
        #[arg(long, short = 'o')]
        output: PathBuf,
    },
    ImportGerberSolderMask {
        gerber: PathBuf,
        #[arg(long)]
        project: PathBuf,
        #[arg(long, short = 'o')]
        output: PathBuf,
    },
    ImportGerberSolderPaste {
        gerber: PathBuf,
        #[arg(long)]
        project: PathBuf,
        #[arg(long, short = 'o')]
        output: PathBuf,
    },
    ImportExcellonDrill {
        drill: PathBuf,
        #[arg(long)]
        project: PathBuf,
        #[arg(long, short = 'o')]
        output: PathBuf,
    },
}

#[derive(Debug, Clone, ValueEnum)]
enum ImportBackend {
    Auto,
    Ngspice,
    Xyce,
    EmbeddedNgspice,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum RepairYamlFinding {
    InvalidPowerDomain,
    NetNotFound,
    PinNotDeclared,
    RequiredPinFloating,
    AnalogModelPackageMetadata,
    BundleInstallPackageMetadata,
}

impl RepairYamlFinding {
    fn as_repair_kind(self) -> BoardYamlRepairFindingKind {
        match self {
            Self::InvalidPowerDomain => BoardYamlRepairFindingKind::InvalidPowerDomain,
            Self::NetNotFound => BoardYamlRepairFindingKind::NetNotFound,
            Self::PinNotDeclared => BoardYamlRepairFindingKind::PinNotDeclared,
            Self::RequiredPinFloating => BoardYamlRepairFindingKind::RequiredPinFloating,
            Self::AnalogModelPackageMetadata => {
                BoardYamlRepairFindingKind::AnalogModelPackageMetadata
            }
            Self::BundleInstallPackageMetadata => {
                BoardYamlRepairFindingKind::BundleInstallPackageMetadata
            }
        }
    }
}

impl ImportBackend {
    fn as_board_ir(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Ngspice => "ngspice",
            Self::Xyce => "xyce",
            Self::EmbeddedNgspice => "embedded_ngspice",
        }
    }
}

pub fn run() -> Result<()> {
    let args = Args::parse();
    match args.command {
        Some(Command::Init) => {
            println!("CircuitCI project initialization is not implemented yet.");
            Ok(())
        }
        Some(Command::Validate {
            project,
            profile,
            output,
            json,
            no_open_ui: _,
        }) => run_validate(project, profile, output, json),
        Some(Command::ValidateSuite { manifest, output }) => run_validate_suite(manifest, output),
        Some(Command::VerifyModelPackage {
            lock,
            registry,
            registry_entry,
            output,
        }) => run_verify_model_package(lock, registry, registry_entry, output),
        Some(Command::VerifyModelPackageBundle { bundle, output }) => {
            run_verify_model_package_bundle(bundle, output)
        }
        Some(Command::InstallModelPackageBundle {
            bundle,
            install_dir,
            registry_output,
            registry_entry,
            registry_artifact_id,
            output,
        }) => run_install_model_package_bundle(
            bundle,
            install_dir,
            registry_output,
            registry_entry,
            registry_artifact_id,
            output,
        ),
        Some(Command::ImportModelPackageBundle {
            bundle,
            project,
            profile,
            install_dir,
            registry_output,
            registry_entry,
            registry_artifact_id,
            output,
        }) => run_import_model_package_bundle(
            crate::model_package_bundle::ModelPackageBundleImportOptions {
                bundle,
                project,
                profile,
                install_dir,
                registry_output,
                registry_entry,
                registry_artifact_id,
                output,
            },
        ),
        Some(Command::ExportModelPackage {
            package_name,
            package_version,
            package_artifacts,
            artifact_id,
            artifact,
            artifact_format,
            compiler,
            output,
            registry_output,
            registry_entry,
            registry_artifact_id,
        }) => run_export_model_package(CliModelPackageExportArgs {
            package_name,
            package_version,
            package_artifacts,
            artifact_id,
            artifact,
            artifact_format,
            compiler,
            output,
            registry_output,
            registry_entry,
            registry_artifact_id,
        }),
        Some(Command::ExportModelConformanceReport {
            report,
            package_name,
            package_version,
            artifact_id,
            runtime_artifact,
            check_name,
            analysis,
            solver,
            output,
        }) => run_export_model_conformance_report(
            report,
            package_name,
            package_version,
            artifact_id,
            runtime_artifact,
            check_name,
            analysis,
            solver,
            output,
        ),
        Some(Command::ExportModelPackageBundle {
            lock,
            registry,
            registry_entry,
            output,
        }) => run_export_model_package_bundle(lock, registry, registry_entry, output),
        Some(Command::MergeModelPackageRegistry {
            base,
            inputs,
            output,
        }) => run_merge_model_package_registry(base, inputs, output),
        Some(Command::RepairYaml {
            project,
            profile,
            output,
            finding,
            dry_run,
            apply_report,
            proposal_ids,
            bundle_install_report,
        }) => run_repair_yaml(CliRepairYamlArgs {
            project,
            profile,
            output,
            finding,
            dry_run,
            apply_report,
            proposal_ids,
            bundle_install_report,
        }),
        Some(Command::SuggestScenarios {
            project,
            profile,
            output,
        }) => run_suggest_scenarios(project, profile, output),
        #[cfg(feature = "gui")]
        Some(Command::ExportSketchSvg {
            project,
            output,
            width,
            height,
        }) => run_export_sketch_svg(project, output, width, height),
        Some(Command::SetManufacturingMetadata {
            project,
            output,
            stencil_thickness_mm,
            min_drill_edge_clearance_mm,
            min_slot_edge_clearance_mm,
            min_paste_area_ratio,
            max_paste_area_ratio,
            min_solder_paste_spacing_mm,
            max_stitch_via_distance_mm,
            source,
        }) => run_set_manufacturing_metadata(
            project,
            output,
            ManufacturingMetadataArgs {
                stencil_thickness_mm,
                min_drill_edge_clearance_mm,
                min_slot_edge_clearance_mm,
                min_paste_area_ratio,
                max_paste_area_ratio,
                min_solder_paste_spacing_mm,
                max_stitch_via_distance_mm,
                source,
            },
        ),
        Some(Command::ImportManufacturingMetadata {
            project,
            metadata,
            output,
            manifest,
            source,
            allow_unknown_fields,
        }) => run_import_manufacturing_metadata(
            project,
            metadata,
            output,
            manifest,
            source,
            allow_unknown_fields,
        ),
        Some(Command::ImportSpice {
            deck,
            output,
            name,
            backend,
            stop_time_us,
            max_step_us,
        }) => run_import_spice(deck, output, name, backend, stop_time_us, max_step_us),
        Some(Command::ImportKicadNetlist {
            netlist,
            output,
            name,
            default_model,
            mapping,
        }) => run_import_kicad_netlist(netlist, output, name, default_model, mapping),
        Some(Command::ImportKicadSchematic {
            schematic,
            output,
            name,
            default_model,
            mapping,
        }) => run_import_kicad_schematic(schematic, output, name, default_model, mapping),
        Some(Command::ImportKicadPcb {
            pcb,
            project,
            output,
        }) => run_import_kicad_pcb(pcb, project, output),
        Some(Command::ImportJlcAssembly {
            bom,
            placement,
            output,
            name,
            manifest,
            default_model,
        }) => run_import_jlc_assembly(bom, placement, output, name, manifest, default_model),
        Some(Command::InspectEasyedaPro {
            eprj2,
            output,
            manifest,
        }) => run_inspect_easyeda_pro(eprj2, output, manifest),
        Some(Command::ImportEasyedaFlyingProbe {
            json,
            project,
            output,
            default_model,
        }) => run_import_easyeda_flying_probe(json, project, output, default_model),
        Some(Command::ImportGerberOutline {
            gerber,
            project,
            output,
        }) => run_import_gerber_outline(gerber, project, output),
        Some(Command::ImportGerberCopper {
            gerber,
            project,
            output,
        }) => run_import_gerber_copper(gerber, project, output),
        Some(Command::ImportGerberSolderMask {
            gerber,
            project,
            output,
        }) => run_import_gerber_solder_mask(gerber, project, output),
        Some(Command::ImportGerberSolderPaste {
            gerber,
            project,
            output,
        }) => run_import_gerber_solder_paste(gerber, project, output),
        Some(Command::ImportExcellonDrill {
            drill,
            project,
            output,
        }) => run_import_excellon_drill(drill, project, output),
        None => {
            Args::parse_from(["circuitci", "--help"]);
            Ok(())
        }
    }
}

fn run_verify_model_package(
    lock: PathBuf,
    registry: Option<PathBuf>,
    registry_entry: Option<String>,
    output: PathBuf,
) -> Result<()> {
    let report = crate::model_package::verify_model_package(
        &crate::model_package::ModelPackageVerifyOptions {
            lock: lock.clone(),
            registry,
            registry_entry,
            output: output.clone(),
        },
    )?;
    crate::model_package::write_model_package_verification_report(&report, &output)?;
    println!(
        "CircuitCI model package verification {}: artifacts={} findings={} -> {}",
        report.result,
        report.artifacts.len(),
        report.findings.len(),
        output.display()
    );
    if report.result != "pass" {
        bail!(
            "Model package verification failed for {}; see {}",
            lock.display(),
            output.display()
        );
    }
    Ok(())
}

fn run_verify_model_package_bundle(bundle: PathBuf, output: PathBuf) -> Result<()> {
    let report = crate::model_package_bundle::verify_model_package_bundle(
        &crate::model_package_bundle::ModelPackageBundleVerifyOptions {
            bundle: bundle.clone(),
            output: output.clone(),
        },
    )?;
    crate::model_package_bundle::write_model_package_bundle_verification_report(&report, &output)?;
    println!(
        "CircuitCI model package bundle verification {}: artifacts={} conformance_checks={} findings={} -> {}",
        report.result,
        report.artifacts.len(),
        report.conformance_checks.len(),
        report.findings.len(),
        output.display()
    );
    if report.result != "pass" {
        bail!(
            "Model package bundle verification failed for {}; see {}",
            bundle.display(),
            output.display()
        );
    }
    Ok(())
}

fn run_install_model_package_bundle(
    bundle: PathBuf,
    install_dir: PathBuf,
    registry_output: Option<PathBuf>,
    registry_entry: Option<String>,
    registry_artifact_id: Option<String>,
    output: PathBuf,
) -> Result<()> {
    let report = crate::model_package_bundle::install_model_package_bundle(
        &crate::model_package_bundle::ModelPackageBundleInstallOptions {
            bundle: bundle.clone(),
            install_dir,
            registry_output,
            registry_entry,
            registry_artifact_id,
            output: output.clone(),
        },
    )?;
    crate::model_package_bundle::write_model_package_bundle_install_report(&report, &output)?;
    println!(
        "CircuitCI installed model package bundle {}: artifacts={} conformance_checks={} findings={} -> {}",
        report.install_dir,
        report.artifacts.len(),
        report.conformance_checks.len(),
        report.findings.len(),
        output.display()
    );
    if let Some(import) = &report.scenario_import {
        println!(
            "CircuitCI model package registry pin path={} sha256={} entry={} artifact={}",
            import.model_package_registry_path,
            import.model_package_registry_sha256,
            import.model_package_registry_entry,
            import.model_package_artifact_id
        );
    }
    if report.result != "pass" {
        bail!(
            "Model package bundle install failed for {}; see {}",
            bundle.display(),
            output.display()
        );
    }
    Ok(())
}

fn run_import_model_package_bundle(
    options: crate::model_package_bundle::ModelPackageBundleImportOptions,
) -> Result<()> {
    let bundle = options.bundle.clone();
    let output = options.output.clone();
    let report = crate::model_package_bundle::import_model_package_bundle(&options)?;
    crate::model_package_bundle::write_model_package_bundle_import_report(&report, &output)?;
    println!(
        "CircuitCI imported model package bundle {}: result={} artifacts={} conformance_checks={} repair_applied={} repair_blocked={} -> {}",
        report.bundle_path,
        report.result,
        report.summary.bundle_artifacts,
        report.summary.conformance_checks,
        report.summary.repair_applied,
        report.summary.repair_blocked,
        output.join("model_package_bundle_import.json").display()
    );
    if report.result != "pass" {
        bail!(
            "Model package bundle import failed for {}; see {}",
            bundle.display(),
            output.join("model_package_bundle_import.json").display()
        );
    }
    Ok(())
}

fn run_merge_model_package_registry(
    base: Option<PathBuf>,
    inputs: Vec<PathBuf>,
    output: PathBuf,
) -> Result<()> {
    let summary = crate::model_package::merge_model_package_registries(
        &crate::model_package::ModelPackageRegistryMergeOptions {
            base,
            inputs,
            output,
        },
    )?;
    println!(
        "CircuitCI merged model package registry {} sha256={} entries={} input_registries={} deduplicated_entries={}",
        summary.registry_path,
        summary.registry_sha256,
        summary.entries,
        summary.input_registries,
        summary.deduplicated_entries
    );
    Ok(())
}

#[derive(Debug)]
struct CliModelPackageExportArgs {
    package_name: String,
    package_version: String,
    package_artifacts: Vec<String>,
    artifact_id: Option<String>,
    artifact: Option<PathBuf>,
    artifact_format: Option<String>,
    compiler: Option<String>,
    output: PathBuf,
    registry_output: Option<PathBuf>,
    registry_entry: Option<String>,
    registry_artifact_id: Option<String>,
}

fn run_export_model_package(args: CliModelPackageExportArgs) -> Result<()> {
    let artifacts = export_model_package_artifacts(&args)?;
    let summary = crate::model_package::export_model_package_lock(
        &crate::model_package::ModelPackageExportOptions {
            package_name: args.package_name,
            package_version: args.package_version,
            artifacts,
            output: args.output,
            registry_output: args.registry_output,
            registry_entry: args.registry_entry,
            registry_artifact_id: args.registry_artifact_id,
        },
    )?;
    println!(
        "CircuitCI exported model package lock {} sha256={} artifact={} artifact_sha256={}",
        summary.lock_path, summary.lock_sha256, summary.artifact_path, summary.artifact_sha256
    );
    if let (Some(registry_path), Some(registry_sha256)) = (
        summary.registry_path.as_deref(),
        summary.registry_sha256.as_deref(),
    ) {
        println!(
            "CircuitCI exported model package registry {registry_path} sha256={registry_sha256}"
        );
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_export_model_conformance_report(
    report: PathBuf,
    package_name: String,
    package_version: String,
    artifact_id: String,
    runtime_artifact: PathBuf,
    check_name: String,
    analysis: String,
    solver: Option<String>,
    output: PathBuf,
) -> Result<()> {
    let summary = crate::model_package::export_model_conformance_report(
        &crate::model_package::ModelConformanceReportExportOptions {
            validation_report: report,
            package_name,
            package_version,
            artifact_id,
            runtime_artifact,
            check_name,
            analysis,
            solver,
            output,
        },
    )?;
    println!(
        "CircuitCI exported model conformance report {} sha256={} result={} artifact={} artifact_sha256={}",
        summary.output,
        summary.sha256,
        summary.result,
        summary.artifact_id,
        summary.runtime_artifact_sha256
    );
    Ok(())
}

fn run_export_model_package_bundle(
    lock: PathBuf,
    registry: Option<PathBuf>,
    registry_entry: Option<String>,
    output: PathBuf,
) -> Result<()> {
    let summary = crate::model_package::export_model_package_bundle(
        &crate::model_package::ModelPackageBundleExportOptions {
            lock,
            registry,
            registry_entry,
            output,
        },
    )?;
    println!(
        "CircuitCI exported model package bundle {} manifest={} manifest_sha256={} artifacts={} conformance_checks={}",
        summary.output,
        summary.manifest_path,
        summary.manifest_sha256,
        summary.artifact_count,
        summary.conformance_check_count
    );
    Ok(())
}

fn export_model_package_artifacts(
    args: &CliModelPackageExportArgs,
) -> Result<Vec<crate::model_package::ModelPackageExportArtifactInput>> {
    if !args.package_artifacts.is_empty() {
        if args.artifact_id.is_some() || args.artifact.is_some() || args.artifact_format.is_some() {
            bail!(
                "--package-artifact cannot be combined with --artifact-id, --artifact, or --artifact-format."
            );
        }
        if args.compiler.is_some() {
            bail!(
                "--compiler cannot be combined with --package-artifact; include compiler= in each artifact spec."
            );
        }
        return args
            .package_artifacts
            .iter()
            .map(|spec| parse_model_package_artifact_spec(spec))
            .collect();
    }
    Ok(vec![
        crate::model_package::ModelPackageExportArtifactInput {
            id: args
                .artifact_id
                .clone()
                .context("--artifact-id is required when --package-artifact is not supplied.")?,
            artifact: args
                .artifact
                .clone()
                .context("--artifact is required when --package-artifact is not supplied.")?,
            artifact_format: args.artifact_format.clone().context(
                "--artifact-format is required when --package-artifact is not supplied.",
            )?,
            compiler: args.compiler.clone(),
        },
    ])
}

fn parse_model_package_artifact_spec(
    spec: &str,
) -> Result<crate::model_package::ModelPackageExportArtifactInput> {
    let mut id = None;
    let mut artifact = None;
    let mut artifact_format = None;
    let mut compiler = None;
    let mut seen = std::collections::BTreeSet::new();
    for piece in spec.split(',') {
        let (raw_key, raw_value) = piece.split_once('=').with_context(|| {
            format!("Invalid --package-artifact segment {piece:?}; expected key=value.")
        })?;
        let key = raw_key.trim();
        let value = raw_value.trim();
        if key.is_empty() || value.is_empty() {
            bail!(
                "Invalid --package-artifact segment {piece:?}; keys and values must be non-empty."
            );
        }
        let canonical_key = match key {
            "id" => "id",
            "path" | "artifact" => "path",
            "artifact_format" | "format" => "artifact_format",
            "compiler" => "compiler",
            _ => bail!("Unknown --package-artifact key {key:?}."),
        };
        if !seen.insert(canonical_key) {
            bail!("Duplicate --package-artifact key {canonical_key:?}.");
        }
        match canonical_key {
            "id" => id = Some(value.to_string()),
            "path" => artifact = Some(PathBuf::from(value)),
            "artifact_format" => artifact_format = Some(value.to_string()),
            "compiler" => compiler = Some(value.to_string()),
            _ => unreachable!(),
        }
    }
    Ok(crate::model_package::ModelPackageExportArtifactInput {
        id: id.context("--package-artifact requires id=<artifact-id>.")?,
        artifact: artifact.context("--package-artifact requires path=<artifact-path>.")?,
        artifact_format: artifact_format
            .context("--package-artifact requires artifact_format=<format>.")?,
        compiler,
    })
}

fn run_repair_yaml(args: CliRepairYamlArgs) -> Result<()> {
    if args.dry_run && args.apply_report.is_some() {
        anyhow::bail!("--dry-run and --apply-report cannot be used together.");
    }
    if !args.proposal_ids.is_empty() && args.apply_report.is_none() {
        anyhow::bail!("--proposal-id can only be used with --apply-report.");
    }
    if args.bundle_install_report.is_some()
        && args.finding.as_repair_kind() != BoardYamlRepairFindingKind::BundleInstallPackageMetadata
    {
        anyhow::bail!(
            "--bundle-install-report can only be used with --finding bundle-install-package-metadata."
        );
    }
    let report = crate::repair_yaml::run_board_yaml_repair(BoardYamlRepairOptions {
        project: args.project,
        profile: args.profile,
        output: args.output.clone(),
        finding: args.finding.as_repair_kind(),
        dry_run: args.dry_run,
        apply_report: args.apply_report,
        proposal_ids: args.proposal_ids,
        bundle_install_report: args.bundle_install_report,
    })?;
    println!(
        "CircuitCI YAML repair {}: {} mode={} (proposed={}, selected={}, applied={}, blocked={}, skipped={}, original_matching_criticals={}, repaired_matching_criticals={}, new_criticals={}) -> {}",
        report.finding,
        report.result,
        report.mode,
        report.summary.proposed,
        report.summary.selected,
        report.summary.applied,
        report.summary.blocked,
        report.summary.skipped,
        report.summary.original_matching_criticals,
        report.summary.repaired_matching_criticals,
        report.summary.new_criticals,
        args.output.join("repair_report.json").display()
    );
    Ok(())
}

struct CliRepairYamlArgs {
    project: PathBuf,
    profile: String,
    output: PathBuf,
    finding: RepairYamlFinding,
    dry_run: bool,
    apply_report: Option<PathBuf>,
    proposal_ids: Vec<String>,
    bundle_install_report: Option<PathBuf>,
}

fn run_suggest_scenarios(
    project_path: PathBuf,
    profile: Option<String>,
    output: PathBuf,
) -> Result<()> {
    let project = crate::board_ir::load_project(&project_path)?;
    let (library, library_findings) = crate::library::load_library(&project_path, &project);
    let bound = crate::library::bind_project(&project, library, library_findings);
    let report =
        crate::scenario_suggestions::suggest_scenarios_for_profile(&bound, profile.as_deref());
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let yaml = serde_yaml_ng::to_string(&report)?;
    std::fs::write(&output, yaml)?;
    println!(
        "CircuitCI suggested {} scenarios for {} -> {}",
        report.suggestions.len(),
        report.project,
        output.display()
    );
    Ok(())
}

#[cfg(feature = "gui")]
fn run_export_sketch_svg(project: PathBuf, output: PathBuf, width: u32, height: u32) -> Result<()> {
    let summary = crate::gui::export_sketch_svg(&project, &output, width, height)?;
    println!(
        "CircuitCI exported Sketch SVG for {} (components={}, nets={}, wires={}, pins={}) -> {}",
        project.display(),
        summary.components,
        summary.nets,
        summary.wires,
        summary.pin_anchors,
        output.display()
    );
    Ok(())
}

#[derive(Debug, Default)]
struct ManufacturingMetadataArgs {
    stencil_thickness_mm: Option<f64>,
    min_drill_edge_clearance_mm: Option<f64>,
    min_slot_edge_clearance_mm: Option<f64>,
    min_paste_area_ratio: Option<f64>,
    max_paste_area_ratio: Option<f64>,
    min_solder_paste_spacing_mm: Option<f64>,
    max_stitch_via_distance_mm: Option<f64>,
    source: Option<String>,
}

fn run_set_manufacturing_metadata(
    project: PathBuf,
    output: PathBuf,
    metadata: ManufacturingMetadataArgs,
) -> Result<()> {
    validate_manufacturing_metadata(&metadata)?;
    let text = std::fs::read_to_string(&project)
        .with_context(|| format!("Failed to read Board IR project {}", project.display()))?;
    let mut project_yaml: Value = serde_yaml_ng::from_str(&text).with_context(|| {
        format!(
            "Failed to parse Board IR project YAML {}",
            project.display()
        )
    })?;
    let updates = apply_manufacturing_metadata(&mut project_yaml, &metadata)?;
    if updates == 0 {
        bail!("At least one manufacturing metadata value must be supplied.");
    }
    absolutize_relative_libraries(
        &mut project_yaml,
        project
            .parent()
            .unwrap_or_else(|| std::path::Path::new(".")),
    )?;
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create manufacturing metadata output directory {}",
                parent.display()
            )
        })?;
    }
    let mut yaml = serde_yaml_ng::to_string(&project_yaml)?;
    yaml.insert_str(
        0,
        "# Generated by CircuitCI by applying explicit board manufacturing metadata.\n",
    );
    std::fs::write(&output, yaml).with_context(|| {
        format!(
            "Failed to write manufacturing metadata project {}",
            output.display()
        )
    })?;
    println!(
        "CircuitCI applied {} board manufacturing metadata fields {} -> {}",
        updates,
        project.display(),
        output.display()
    );
    Ok(())
}

fn validate_manufacturing_metadata(metadata: &ManufacturingMetadataArgs) -> Result<()> {
    validate_positive("stencil_thickness_mm", metadata.stencil_thickness_mm)?;
    validate_non_negative(
        "min_drill_edge_clearance_mm",
        metadata.min_drill_edge_clearance_mm,
    )?;
    validate_non_negative(
        "min_slot_edge_clearance_mm",
        metadata.min_slot_edge_clearance_mm,
    )?;
    validate_non_negative("min_paste_area_ratio", metadata.min_paste_area_ratio)?;
    validate_non_negative("max_paste_area_ratio", metadata.max_paste_area_ratio)?;
    validate_non_negative(
        "min_solder_paste_spacing_mm",
        metadata.min_solder_paste_spacing_mm,
    )?;
    validate_non_negative(
        "max_stitch_via_distance_mm",
        metadata.max_stitch_via_distance_mm,
    )?;
    if let (Some(min), Some(max)) = (metadata.min_paste_area_ratio, metadata.max_paste_area_ratio)
        && max < min
    {
        bail!("max_paste_area_ratio must be greater than or equal to min_paste_area_ratio.");
    }
    if metadata
        .source
        .as_deref()
        .is_some_and(|source| source.trim().is_empty())
    {
        bail!("source must not be empty when supplied.");
    }
    Ok(())
}

fn validate_positive(name: &str, value: Option<f64>) -> Result<()> {
    let Some(value) = value else {
        return Ok(());
    };
    if !value.is_finite() || value <= 0.0 {
        bail!("{name} must be finite and greater than zero.");
    }
    Ok(())
}

fn validate_non_negative(name: &str, value: Option<f64>) -> Result<()> {
    let Some(value) = value else {
        return Ok(());
    };
    if !value.is_finite() || value < 0.0 {
        bail!("{name} must be finite and greater than or equal to zero.");
    }
    Ok(())
}

fn apply_manufacturing_metadata(
    project_yaml: &mut Value,
    metadata: &ManufacturingMetadataArgs,
) -> Result<usize> {
    let root = project_yaml
        .as_mapping_mut()
        .context("Board IR project must be a YAML object.")?;
    let board = ensure_mapping_field_mut(root, "board")?;
    let manufacturing = ensure_mapping_field_mut(board, "manufacturing")?;
    let mut updates = 0;
    updates += insert_optional_number(
        manufacturing,
        "stencil_thickness_mm",
        metadata.stencil_thickness_mm,
    )?;
    updates += insert_optional_number(
        manufacturing,
        "min_drill_edge_clearance_mm",
        metadata.min_drill_edge_clearance_mm,
    )?;
    updates += insert_optional_number(
        manufacturing,
        "min_slot_edge_clearance_mm",
        metadata.min_slot_edge_clearance_mm,
    )?;
    updates += insert_optional_number(
        manufacturing,
        "min_paste_area_ratio",
        metadata.min_paste_area_ratio,
    )?;
    updates += insert_optional_number(
        manufacturing,
        "max_paste_area_ratio",
        metadata.max_paste_area_ratio,
    )?;
    updates += insert_optional_number(
        manufacturing,
        "min_solder_paste_spacing_mm",
        metadata.min_solder_paste_spacing_mm,
    )?;
    updates += insert_optional_number(
        manufacturing,
        "max_stitch_via_distance_mm",
        metadata.max_stitch_via_distance_mm,
    )?;
    if let Some(source) = metadata.source.as_deref() {
        manufacturing.insert(
            Value::String("source".to_string()),
            Value::String(source.trim().to_string()),
        );
        updates += 1;
    }
    Ok(updates)
}

fn insert_optional_number(mapping: &mut Mapping, name: &str, value: Option<f64>) -> Result<usize> {
    let Some(value) = value else {
        return Ok(0);
    };
    mapping.insert(
        Value::String(name.to_string()),
        serde_yaml_ng::to_value(value)
            .with_context(|| format!("Failed to encode manufacturing metadata {name}."))?,
    );
    Ok(1)
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

fn absolutize_relative_libraries(
    project_yaml: &mut Value,
    project_dir: &std::path::Path,
) -> Result<()> {
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
        let path = std::path::Path::new(path_text);
        if path.is_absolute() {
            continue;
        }
        let resolved = normalize_path(&project_dir.join(path));
        let absolute = std::fs::canonicalize(&resolved).unwrap_or(resolved);
        *library = Value::String(absolute.to_string_lossy().to_string());
    }
    Ok(())
}

fn normalize_path(path: &std::path::Path) -> PathBuf {
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

fn sanitized_project_name(path: &std::path::Path, fallback: &str) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(fallback)
        .replace(|character: char| !character.is_ascii_alphanumeric(), "_")
}

fn run_import_manufacturing_metadata(
    project: PathBuf,
    metadata: PathBuf,
    output: PathBuf,
    manifest: Option<PathBuf>,
    source: Option<String>,
    allow_unknown_fields: bool,
) -> Result<()> {
    let manifest = manifest.unwrap_or_else(|| output.with_extension("manufacturing.json"));
    let summary = crate::importers::manufacturing_metadata::import_manufacturing_metadata(
        &crate::importers::manufacturing_metadata::ManufacturingMetadataImportOptions {
            project: project.clone(),
            metadata: metadata.clone(),
            output: output.clone(),
            manifest: manifest.clone(),
            source,
            allow_unknown_fields,
        },
    )?;
    println!(
        "CircuitCI imported manufacturing metadata: {} applied fields, {} skipped rows from {} rows {} + {} -> {}, manifest {}",
        summary.applied_fields,
        summary.skipped_rows,
        summary.rows,
        project.display(),
        metadata.display(),
        output.display(),
        manifest.display()
    );
    Ok(())
}

fn run_import_spice(
    deck: PathBuf,
    output: PathBuf,
    name: Option<String>,
    backend: ImportBackend,
    stop_time_us: f64,
    max_step_us: f64,
) -> Result<()> {
    let name = name.unwrap_or_else(|| sanitized_project_name(&deck, "imported_spice_project"));
    crate::importers::spice::import_spice(&crate::importers::spice::SpiceImportOptions {
        input: deck.clone(),
        output: output.clone(),
        name,
        backend: backend.as_board_ir().to_string(),
        stop_time_us,
        max_step_us,
    })?;
    println!(
        "CircuitCI imported SPICE deck {} -> {}",
        deck.display(),
        output.display()
    );
    Ok(())
}

fn run_import_kicad_netlist(
    netlist: PathBuf,
    output: PathBuf,
    name: Option<String>,
    default_model: String,
    mapping: Option<PathBuf>,
) -> Result<()> {
    let name = name.unwrap_or_else(|| sanitized_project_name(&netlist, "imported_kicad_project"));
    crate::importers::kicad::import_kicad_netlist(&crate::importers::kicad::KicadImportOptions {
        input: netlist.clone(),
        output: output.clone(),
        name,
        default_model,
        mapping,
    })?;
    println!(
        "CircuitCI imported KiCad XML netlist {} -> {}",
        netlist.display(),
        output.display()
    );
    Ok(())
}

fn run_import_kicad_schematic(
    schematic: PathBuf,
    output: PathBuf,
    name: Option<String>,
    default_model: String,
    mapping: Option<PathBuf>,
) -> Result<()> {
    let name = name.unwrap_or_else(|| sanitized_project_name(&schematic, "imported_kicad_project"));
    crate::importers::kicad_sch::import_kicad_schematic(
        &crate::importers::kicad::KicadImportOptions {
            input: schematic.clone(),
            output: output.clone(),
            name,
            default_model,
            mapping,
        },
    )?;
    println!(
        "CircuitCI imported KiCad schematic {} -> {}",
        schematic.display(),
        output.display()
    );
    Ok(())
}

fn run_import_kicad_pcb(pcb: PathBuf, project: PathBuf, output: PathBuf) -> Result<()> {
    let summary = crate::importers::kicad_pcb::import_kicad_pcb_placements(
        &crate::importers::kicad_pcb::KicadPcbPlacementImportOptions {
            input: pcb.clone(),
            project: project.clone(),
            output: output.clone(),
        },
    )?;
    println!(
        "CircuitCI imported {} KiCad PCB placements, {} footprint graphics, {} pads, {} board outline segments, {} route segments, {} vias, {} copper zones, and {} routing constraints {} + {} -> {}",
        summary.placements,
        summary.footprint_graphics,
        summary.pads,
        summary.outline_segments,
        summary.route_segments,
        summary.route_vias,
        summary.zones,
        summary.routing_constraints,
        pcb.display(),
        project.display(),
        output.display()
    );
    Ok(())
}

fn run_import_jlc_assembly(
    bom: PathBuf,
    placement: PathBuf,
    output: PathBuf,
    name: Option<String>,
    manifest: Option<PathBuf>,
    default_model: String,
) -> Result<()> {
    let name = name.unwrap_or_else(|| sanitized_project_name(&bom, "imported_jlc_assembly"));
    let manifest = manifest.unwrap_or_else(|| output.with_extension("json"));
    let summary = crate::importers::jlc::import_jlc_assembly(
        &crate::importers::jlc::JlcAssemblyImportOptions {
            bom: bom.clone(),
            placement: placement.clone(),
            output: output.clone(),
            manifest: manifest.clone(),
            name,
            default_model,
        },
    )?;
    println!(
        "CircuitCI imported JLC/EasyEDA assembly: {} components, {} BOM rows, {} placements, {} BOM-matched components, {} placement-matched components {} + {} -> {}, manifest {}",
        summary.components,
        summary.bom_rows,
        summary.placements,
        summary.components_with_bom,
        summary.components_with_placement,
        bom.display(),
        placement.display(),
        output.display(),
        manifest.display()
    );
    Ok(())
}

fn run_inspect_easyeda_pro(
    eprj2: PathBuf,
    output: PathBuf,
    manifest: Option<PathBuf>,
) -> Result<()> {
    let manifest = manifest.unwrap_or_else(|| output.with_extension("json"));
    let summary = crate::importers::easyeda_pro::inspect_easyeda_pro_project(
        &crate::importers::easyeda_pro::EasyedaProInspectOptions {
            eprj2: eprj2.clone(),
            output: output.clone(),
            manifest: manifest.clone(),
        },
    )?;
    println!(
        "CircuitCI inspected EasyEDA Pro project: {} projects, {} branches, {} structures, latest ticket {}, {} boards, {} schematics, {} sheets, {} PCBs, {} structure objects, {} encoded history payloads {} -> {}, manifest {}",
        summary.projects,
        summary.branches,
        summary.project_structures,
        summary
            .latest_ticket
            .map(|ticket| ticket.to_string())
            .unwrap_or_else(|| "none".to_string()),
        summary.boards,
        summary.schematics,
        summary.sheets,
        summary.pcbs,
        summary.structure_objects,
        summary.encoded_history_payloads,
        eprj2.display(),
        output.display(),
        manifest.display()
    );
    Ok(())
}

fn run_import_easyeda_flying_probe(
    json: PathBuf,
    project: PathBuf,
    output: PathBuf,
    default_model: String,
) -> Result<()> {
    let summary = crate::importers::easyeda_flying_probe::import_easyeda_flying_probe(
        &crate::importers::easyeda_flying_probe::EasyedaFlyingProbeImportOptions {
            input: json.clone(),
            project: project.clone(),
            output: output.clone(),
            default_model,
        },
    )?;
    println!(
        "CircuitCI imported EasyEDA/JLC flying-probe pads: {} pin rows, {} connected pin rows, {} pads imported, {} duplicate pin rows, {} multipart pin rows, {} unconnected pins skipped, {} components created, {} nets imported {} + {} -> {}",
        summary.pin_rows,
        summary.connected_pin_rows,
        summary.pads_imported,
        summary.duplicate_pin_rows,
        summary.multipart_pin_rows,
        summary.skipped_unconnected_pins,
        summary.components_created,
        summary.nets_imported,
        json.display(),
        project.display(),
        output.display()
    );
    Ok(())
}

fn run_import_gerber_outline(gerber: PathBuf, project: PathBuf, output: PathBuf) -> Result<()> {
    let summary = crate::importers::gerber::import_gerber_outline(
        &crate::importers::gerber::GerberOutlineImportOptions {
            gerber: gerber.clone(),
            project: project.clone(),
            output: output.clone(),
        },
    )?;
    println!(
        "CircuitCI imported Gerber outline: {} segments ({} external, {} cutout, {} unknown) {} + {} -> {}",
        summary.outline_segments,
        summary.external_segments,
        summary.cutout_segments,
        summary.unknown_segments,
        gerber.display(),
        project.display(),
        output.display()
    );
    Ok(())
}

fn run_import_gerber_copper(gerber: PathBuf, project: PathBuf, output: PathBuf) -> Result<()> {
    let summary = crate::importers::gerber::import_gerber_copper(
        &crate::importers::gerber::GerberCopperImportOptions {
            gerber: gerber.clone(),
            project: project.clone(),
            output: output.clone(),
        },
    )?;
    println!(
        "CircuitCI imported Gerber copper: {} flash features, {} trace segments, {} regions, {} net-associated features, {} net-associated segments, {} net-associated regions, {} island-associated features, {} island-associated segments, {} island-associated regions, {} apertures, {} ignored draw records, {} skipped clear flashes, {} skipped clear regions {} + {} -> {}",
        summary.flash_features,
        summary.trace_segments,
        summary.regions,
        summary.net_associated_features,
        summary.net_associated_segments,
        summary.net_associated_regions,
        summary.island_associated_features,
        summary.island_associated_segments,
        summary.island_associated_regions,
        summary.apertures,
        summary.ignored_draws,
        summary.skipped_clear_flashes,
        summary.skipped_clear_regions,
        gerber.display(),
        project.display(),
        output.display()
    );
    Ok(())
}

fn run_import_gerber_solder_mask(gerber: PathBuf, project: PathBuf, output: PathBuf) -> Result<()> {
    let summary = crate::importers::gerber::import_gerber_solder_mask(
        &crate::importers::gerber::GerberSolderMaskImportOptions {
            gerber: gerber.clone(),
            project: project.clone(),
            output: output.clone(),
        },
    )?;
    println!(
        "CircuitCI imported Gerber solder mask: {} flash openings, {} draw openings, {} region openings, {} owner-associated flash openings, {} owner-associated draw openings, {} owner-associated region openings, {} apertures, {} ignored draw records, {} skipped clear flashes, {} skipped clear regions {} + {} -> {}",
        summary.openings,
        summary.draw_openings,
        summary.region_openings,
        summary.owner_associated_openings,
        summary.owner_associated_draw_openings,
        summary.owner_associated_region_openings,
        summary.apertures,
        summary.ignored_draws,
        summary.skipped_clear_flashes,
        summary.skipped_clear_regions,
        gerber.display(),
        project.display(),
        output.display()
    );
    Ok(())
}

fn run_import_gerber_solder_paste(
    gerber: PathBuf,
    project: PathBuf,
    output: PathBuf,
) -> Result<()> {
    let summary = crate::importers::gerber::import_gerber_solder_paste(
        &crate::importers::gerber::GerberSolderPasteImportOptions {
            gerber: gerber.clone(),
            project: project.clone(),
            output: output.clone(),
        },
    )?;
    println!(
        "CircuitCI imported Gerber solder paste: {} flash openings, {} draw openings, {} region openings, {} owner-associated flash openings, {} owner-associated draw openings, {} owner-associated region openings, {} apertures, {} ignored draw records, {} skipped clear flashes, {} skipped clear regions {} + {} -> {}",
        summary.openings,
        summary.draw_openings,
        summary.region_openings,
        summary.owner_associated_openings,
        summary.owner_associated_draw_openings,
        summary.owner_associated_region_openings,
        summary.apertures,
        summary.ignored_draws,
        summary.skipped_clear_flashes,
        summary.skipped_clear_regions,
        gerber.display(),
        project.display(),
        output.display()
    );
    Ok(())
}

fn run_import_excellon_drill(drill: PathBuf, project: PathBuf, output: PathBuf) -> Result<()> {
    let summary = crate::importers::drill::import_excellon_drill(
        &crate::importers::drill::ExcellonDrillImportOptions {
            drill: drill.clone(),
            project: project.clone(),
            output: output.clone(),
        },
    )?;
    println!(
        "CircuitCI imported Excellon/NC drill evidence: {} hits, {} routed slots, {} tools ({} plated, {} non-plated, {} unknown plating, {} pad-associated, {} via-associated) {} + {} -> {}",
        summary.drill_hits,
        summary.slots,
        summary.tools,
        summary.plated_hits,
        summary.non_plated_hits,
        summary.unknown_plating_hits,
        summary.pad_associated_hits,
        summary.via_associated_hits,
        drill.display(),
        project.display(),
        output.display()
    );
    Ok(())
}

fn run_validate(
    project_path: PathBuf,
    profile: String,
    output: PathBuf,
    json: Option<PathBuf>,
) -> Result<()> {
    let command = format!(
        "circuitci validate {} --profile {} --output {}",
        project_path.display(),
        profile,
        output.display()
    );
    let report = validate_and_write_project_report(&project_path, &profile, &output, command)?;
    if let Some(json_path) = json {
        let source_json = output.join("report.json");
        std::fs::create_dir_all(
            json_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new(".")),
        )?;
        let same_path = match (source_json.canonicalize(), json_path.canonicalize()) {
            (Ok(source), Ok(destination)) => source == destination,
            _ => source_json == json_path,
        };
        if !same_path {
            std::fs::copy(source_json, json_path)?;
        }
    }
    println!(
        "CircuitCI {}: {} (critical={}, warning={}, info={})",
        report.project,
        report.result,
        report.summary.critical,
        report.summary.warning,
        report.summary.info
    );
    Ok(())
}

fn run_validate_suite(manifest: PathBuf, output: PathBuf) -> Result<()> {
    let command = format!(
        "circuitci validate-suite {} --output {}",
        manifest.display(),
        output.display()
    );
    let report = run_suite(
        &manifest,
        &output,
        command,
        |project_path, profile, case_output| {
            let case_command = format!(
                "circuitci validate {} --profile {} --output {}",
                project_path.display(),
                profile,
                case_output.display()
            );
            validate_and_write_project_report(project_path, profile, case_output, case_command)
        },
    )?;
    write_suite_reports(&report, &output)?;
    println!(
        "CircuitCI suite {}: {} (cases={}, passed={}, failed={})",
        report.suite,
        report.result,
        report.summary.cases,
        report.summary.passed,
        report.summary.failed
    );
    if report.result == "fail" {
        bail!("Suite {} failed expectations.", report.suite);
    }
    Ok(())
}
