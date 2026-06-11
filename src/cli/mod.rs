use crate::reports::write_suite_reports;
use crate::suite::{run_suite, validate_and_write_project_report};
use anyhow::{Result, bail};
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
    ValidateSuite {
        manifest: PathBuf,
        #[arg(long, short = 'o', default_value = "out/suite")]
        output: PathBuf,
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
        Some(Command::ValidateSuite { manifest, output }) => run_validate_suite(manifest, output),
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
    let command = format!(
        "circuitci validate {} --profile {} --output {}",
        project_path.display(),
        profile,
        output.display()
    );
    let report = validate_and_write_project_report(&project_path, &profile, &output, command)?;
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
