use crate::board_ir::load_project;
use crate::library::{bind_project, load_library};
use crate::reports::{ValidationReport, write_reports};
use crate::validation::validate;
use anyhow::Result;
use clap::{Parser, Subcommand};
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
        None => {
            Args::parse_from(["circuitci", "--help"]);
            Ok(())
        }
    }
}

fn run_validate(
    project_path: PathBuf,
    profile: String,
    output: PathBuf,
    json: Option<PathBuf>,
) -> Result<()> {
    let project = load_project(&project_path)?;
    let (library, library_findings) = load_library(&project_path, &project);
    let bound = bind_project(&project, library, library_findings);
    let (findings, limitations) = validate(&bound);
    let command = format!(
        "circuitci validate {} --profile {} --output {}",
        project_path.display(),
        profile,
        output.display()
    );
    let report = ValidationReport::from_parts(
        project.project.name.clone(),
        profile,
        findings,
        limitations,
        command,
    );
    write_reports(&report, &output)?;
    if let Some(json_path) = json {
        std::fs::create_dir_all(
            json_path
                .parent()
                .unwrap_or_else(|| std::path::Path::new(".")),
        )?;
        std::fs::copy(output.join("report.json"), json_path)?;
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
