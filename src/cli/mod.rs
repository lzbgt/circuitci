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
    SuggestScenarios {
        project: PathBuf,
        #[arg(long, short = 'o', default_value = "out/scenario_suggestions.yaml")]
        output: PathBuf,
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
        source: Option<String>,
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
        #[arg(long, default_value = "generic.schematic.imported_component")]
        default_model: String,
    },
    InspectEasyedaPro {
        eprj2: PathBuf,
        #[arg(long, short = 'o')]
        output: PathBuf,
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
        Some(Command::SetManufacturingMetadata {
            project,
            output,
            stencil_thickness_mm,
            min_drill_edge_clearance_mm,
            min_slot_edge_clearance_mm,
            min_paste_area_ratio,
            max_paste_area_ratio,
            min_solder_paste_spacing_mm,
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
                source,
            },
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
            default_model,
        }) => run_import_jlc_assembly(bom, placement, output, name, default_model),
        Some(Command::InspectEasyedaPro { eprj2, output }) => {
            run_inspect_easyeda_pro(eprj2, output)
        }
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

#[derive(Debug, Default)]
struct ManufacturingMetadataArgs {
    stencil_thickness_mm: Option<f64>,
    min_drill_edge_clearance_mm: Option<f64>,
    min_slot_edge_clearance_mm: Option<f64>,
    min_paste_area_ratio: Option<f64>,
    max_paste_area_ratio: Option<f64>,
    min_solder_paste_spacing_mm: Option<f64>,
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
    default_model: String,
) -> Result<()> {
    let name = name.unwrap_or_else(|| sanitized_project_name(&bom, "imported_jlc_assembly"));
    let summary = crate::importers::jlc::import_jlc_assembly(
        &crate::importers::jlc::JlcAssemblyImportOptions {
            bom: bom.clone(),
            placement: placement.clone(),
            output: output.clone(),
            name,
            default_model,
        },
    )?;
    println!(
        "CircuitCI imported JLC/EasyEDA assembly: {} components, {} BOM rows, {} placements, {} BOM-matched components, {} placement-matched components {} + {} -> {}",
        summary.components,
        summary.bom_rows,
        summary.placements,
        summary.components_with_bom,
        summary.components_with_placement,
        bom.display(),
        placement.display(),
        output.display()
    );
    Ok(())
}

fn run_inspect_easyeda_pro(eprj2: PathBuf, output: PathBuf) -> Result<()> {
    let summary = crate::importers::easyeda_pro::inspect_easyeda_pro_project(
        &crate::importers::easyeda_pro::EasyedaProInspectOptions {
            eprj2: eprj2.clone(),
            output: output.clone(),
        },
    )?;
    println!(
        "CircuitCI inspected EasyEDA Pro project: {} projects, {} branches, {} structures, latest ticket {}, {} boards, {} schematics, {} sheets, {} PCBs, {} encoded history payloads {} -> {}",
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
        summary.encoded_history_payloads,
        eprj2.display(),
        output.display()
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
