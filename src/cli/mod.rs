use crate::reports::write_suite_reports;
use crate::suite::{run_suite, validate_and_write_project_report};
use anyhow::{Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
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
    SuggestScenarios {
        project: PathBuf,
        #[arg(long, short = 'o', default_value = "out/scenario_suggestions.yaml")]
        output: PathBuf,
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
}

#[derive(Debug, Clone, ValueEnum)]
enum ImportBackend {
    Auto,
    Ngspice,
    Xyce,
    EmbeddedNgspice,
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
        Some(Command::SuggestScenarios { project, output }) => {
            run_suggest_scenarios(project, output)
        }
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
        None => {
            Args::parse_from(["circuitci", "--help"]);
            Ok(())
        }
    }
}

fn run_suggest_scenarios(project_path: PathBuf, output: PathBuf) -> Result<()> {
    let project = crate::board_ir::load_project(&project_path)?;
    let (library, library_findings) = crate::library::load_library(&project_path, &project);
    let bound = crate::library::bind_project(&project, library, library_findings);
    let report = crate::scenario_suggestions::suggest_scenarios(&bound);
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

fn sanitized_project_name(path: &std::path::Path, fallback: &str) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or(fallback)
        .replace(|character: char| !character.is_ascii_alphanumeric(), "_")
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
        "CircuitCI imported {} KiCad PCB placements, {} pads, {} route segments, {} vias, {} copper zones, and {} routing constraints {} + {} -> {}",
        summary.placements,
        summary.pads,
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
