mod board_ir;
mod ownership;

use crate::board_ir::BoardProject;
use anyhow::{Context, Result, bail};
use board_ir::{
    merge_copper_into_project, merge_outline_into_project, merge_solder_mask_into_project,
    merge_solder_paste_into_project, summary_for_copper, summary_for_outline,
    summary_for_solder_mask, summary_for_solder_paste,
};
use ownership::associate_copper_nets;
use serde::Serialize;
use serde_yaml_ng::Value;
use std::fs;
use std::path::{Path, PathBuf};

const POINT_EPSILON_MM: f64 = 1.0e-6;
const GERBER_ARC_MAX_STEP_DEG: f64 = 15.0;
const GERBER_ARC_MAX_CHORD_MM: f64 = 0.25;
const GERBER_ARC_MAX_SEGMENTS: usize = 256;

#[derive(Debug, Clone)]
pub struct GerberOutlineImportOptions {
    pub gerber: PathBuf,
    pub project: PathBuf,
    pub output: PathBuf,
}

#[derive(Debug, Clone)]
pub struct GerberCopperImportOptions {
    pub gerber: PathBuf,
    pub project: PathBuf,
    pub output: PathBuf,
}

#[derive(Debug, Clone)]
pub struct GerberSolderMaskImportOptions {
    pub gerber: PathBuf,
    pub project: PathBuf,
    pub output: PathBuf,
}

#[derive(Debug, Clone)]
pub struct GerberSolderPasteImportOptions {
    pub gerber: PathBuf,
    pub project: PathBuf,
    pub output: PathBuf,
}

#[derive(Debug, Clone, Default)]
pub struct GerberOutlineImportSummary {
    pub outline_segments: usize,
    pub external_segments: usize,
    pub cutout_segments: usize,
    pub unknown_segments: usize,
}

#[derive(Debug, Clone, Default)]
pub struct GerberCopperImportSummary {
    pub flash_features: usize,
    pub trace_segments: usize,
    pub regions: usize,
    pub net_associated_features: usize,
    pub net_associated_segments: usize,
    pub net_associated_regions: usize,
    pub island_associated_features: usize,
    pub island_associated_segments: usize,
    pub island_associated_regions: usize,
    pub apertures: usize,
    pub ignored_draws: usize,
    pub skipped_clear_flashes: usize,
    pub skipped_clear_regions: usize,
}

#[derive(Debug, Clone, Default)]
pub struct GerberSolderMaskImportSummary {
    pub openings: usize,
    pub draw_openings: usize,
    pub region_openings: usize,
    pub apertures: usize,
    pub ignored_draws: usize,
    pub skipped_clear_flashes: usize,
    pub skipped_clear_regions: usize,
}

#[derive(Debug, Clone, Default)]
pub struct GerberSolderPasteImportSummary {
    pub openings: usize,
    pub draw_openings: usize,
    pub region_openings: usize,
    pub apertures: usize,
    pub ignored_draws: usize,
    pub skipped_clear_flashes: usize,
    pub skipped_clear_regions: usize,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct GerberPoint {
    x_mm: f64,
    y_mm: f64,
}

#[derive(Debug, Clone)]
struct GerberOutline {
    layer: String,
    segments: Vec<GerberOutlineSegment>,
}

#[derive(Debug, Clone)]
struct GerberCopper {
    layer: String,
    features: Vec<GerberCopperFeature>,
    segments: Vec<GerberCopperSegment>,
    regions: Vec<GerberCopperRegion>,
    aperture_count: usize,
    ignored_draws: usize,
    skipped_clear_flashes: usize,
    skipped_clear_regions: usize,
}

#[derive(Debug, Clone)]
struct GerberCopperFeature {
    at: GerberPoint,
    aperture_code: u32,
    aperture: GerberAperture,
    source_primitive_index: usize,
    net: Option<String>,
    island_id: Option<String>,
    owner_kind: Option<String>,
    component: Option<String>,
    pin: Option<String>,
    via_index: Option<usize>,
}

#[derive(Debug, Clone)]
struct GerberCopperSegment {
    start: GerberPoint,
    end: GerberPoint,
    aperture_code: u32,
    aperture: GerberAperture,
    source_primitive: &'static str,
    source_primitive_index: usize,
    net: Option<String>,
    island_id: Option<String>,
}

#[derive(Debug, Clone)]
struct GerberCopperRegion {
    points: Vec<GerberPoint>,
    source_primitive_index: usize,
    net: Option<String>,
    island_id: Option<String>,
}

#[derive(Debug, Clone)]
struct GerberAperture {
    shape: GerberApertureShape,
    x_mm: f64,
    y_mm: f64,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum GerberApertureShape {
    Circle,
    Rect,
    Oval,
}

#[derive(Debug, Clone)]
struct GerberOutlineSegment {
    start: GerberPoint,
    end: GerberPoint,
    source_primitive_index: usize,
    contour_index: Option<usize>,
    boundary_role: GerberBoundaryRole,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum GerberBoundaryRole {
    External,
    Cutout,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
struct CoordinateFormat {
    x_decimals: u32,
    y_decimals: u32,
}

#[derive(Debug, Clone, Copy)]
struct GerberState {
    format: Option<CoordinateFormat>,
    units_mm: bool,
    absolute: bool,
    line_mode: bool,
    current: Option<GerberPoint>,
    modal_operation: Option<GerberOperation>,
    aperture_code: Option<u32>,
    dark_polarity: bool,
}

impl Default for GerberState {
    fn default() -> Self {
        Self {
            format: None,
            units_mm: false,
            absolute: true,
            line_mode: true,
            current: None,
            modal_operation: None,
            aperture_code: None,
            dark_polarity: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GerberOperation {
    Draw,
    Move,
    Flash,
}

#[derive(Debug, Clone, Copy)]
enum GerberArcDirection {
    Clockwise,
    CounterClockwise,
}

pub fn import_gerber_outline(
    options: &GerberOutlineImportOptions,
) -> Result<GerberOutlineImportSummary> {
    let text = fs::read_to_string(&options.gerber)
        .with_context(|| format!("Failed to read Gerber outline {}", options.gerber.display()))?;
    let outline = parse_gerber_outline(&text, &options.gerber)?;
    let project_text = fs::read_to_string(&options.project).with_context(|| {
        format!(
            "Failed to read Board IR project {}",
            options.project.display()
        )
    })?;
    let mut project_yaml: Value = serde_yaml_ng::from_str(&project_text).with_context(|| {
        format!(
            "Failed to parse Board IR project YAML {}",
            options.project.display()
        )
    })?;
    merge_outline_into_project(&mut project_yaml, &outline)?;
    absolutize_relative_libraries(
        &mut project_yaml,
        options.project.parent().unwrap_or_else(|| Path::new(".")),
    )?;
    if let Some(parent) = options.output.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create import output directory {}",
                parent.display()
            )
        })?;
    }
    let mut yaml = serde_yaml_ng::to_string(&project_yaml)?;
    yaml.insert_str(
        0,
        "# Generated by CircuitCI by adding Gerber board-outline evidence to Board IR.\n",
    );
    fs::write(&options.output, yaml)
        .with_context(|| format!("Failed to write {}", options.output.display()))?;
    Ok(summary_for_outline(&outline))
}

pub fn import_gerber_copper(
    options: &GerberCopperImportOptions,
) -> Result<GerberCopperImportSummary> {
    let text = fs::read_to_string(&options.gerber)
        .with_context(|| format!("Failed to read Gerber copper {}", options.gerber.display()))?;
    let mut copper = parse_gerber_copper(&text, &options.gerber)?;
    let project_text = fs::read_to_string(&options.project).with_context(|| {
        format!(
            "Failed to read Board IR project {}",
            options.project.display()
        )
    })?;
    let mut project_yaml: Value = serde_yaml_ng::from_str(&project_text).with_context(|| {
        format!(
            "Failed to parse Board IR project YAML {}",
            options.project.display()
        )
    })?;
    let project: BoardProject = serde_yaml_ng::from_str(&project_text).with_context(|| {
        format!(
            "Failed to parse Board IR project YAML {} for copper ownership association",
            options.project.display()
        )
    })?;
    associate_copper_nets(&mut copper, &project.board.layout);
    merge_copper_into_project(&mut project_yaml, &copper)?;
    absolutize_relative_libraries(
        &mut project_yaml,
        options.project.parent().unwrap_or_else(|| Path::new(".")),
    )?;
    if let Some(parent) = options.output.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create import output directory {}",
                parent.display()
            )
        })?;
    }
    let mut yaml = serde_yaml_ng::to_string(&project_yaml)?;
    yaml.insert_str(
        0,
        "# Generated by CircuitCI by adding Gerber copper flash evidence to Board IR.\n",
    );
    fs::write(&options.output, yaml)
        .with_context(|| format!("Failed to write {}", options.output.display()))?;
    Ok(summary_for_copper(&copper))
}

pub fn import_gerber_solder_mask(
    options: &GerberSolderMaskImportOptions,
) -> Result<GerberSolderMaskImportSummary> {
    let text = fs::read_to_string(&options.gerber).with_context(|| {
        format!(
            "Failed to read Gerber solder mask {}",
            options.gerber.display()
        )
    })?;
    let mask = parse_gerber_copper(&text, &options.gerber)?;
    let mut project_yaml: Value =
        serde_yaml_ng::from_str(&fs::read_to_string(&options.project).with_context(|| {
            format!(
                "Failed to read Board IR project YAML {}",
                options.project.display()
            )
        })?)
        .with_context(|| {
            format!(
                "Failed to parse Board IR project YAML {} for solder-mask import",
                options.project.display()
            )
        })?;
    merge_solder_mask_into_project(&mut project_yaml, &mask)?;
    if let Some(parent) = options.output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory {}", parent.display()))?;
    }
    let mut output = String::new();
    output
        .push_str("# Generated by CircuitCI by adding Gerber solder-mask evidence to Board IR.\n");
    output.push_str(&serde_yaml_ng::to_string(&project_yaml)?);
    fs::write(&options.output, output)
        .with_context(|| format!("Failed to write {}", options.output.display()))?;
    Ok(summary_for_solder_mask(&mask))
}

pub fn import_gerber_solder_paste(
    options: &GerberSolderPasteImportOptions,
) -> Result<GerberSolderPasteImportSummary> {
    let text = fs::read_to_string(&options.gerber).with_context(|| {
        format!(
            "Failed to read Gerber solder paste {}",
            options.gerber.display()
        )
    })?;
    let paste = parse_gerber_copper(&text, &options.gerber)?;
    let mut project_yaml: Value =
        serde_yaml_ng::from_str(&fs::read_to_string(&options.project).with_context(|| {
            format!(
                "Failed to read Board IR project YAML {}",
                options.project.display()
            )
        })?)
        .with_context(|| {
            format!(
                "Failed to parse Board IR project YAML {} for solder-paste import",
                options.project.display()
            )
        })?;
    merge_solder_paste_into_project(&mut project_yaml, &paste)?;
    if let Some(parent) = options.output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory {}", parent.display()))?;
    }
    let mut output = String::new();
    output
        .push_str("# Generated by CircuitCI by adding Gerber solder-paste evidence to Board IR.\n");
    output.push_str(&serde_yaml_ng::to_string(&project_yaml)?);
    fs::write(&options.output, output)
        .with_context(|| format!("Failed to write {}", options.output.display()))?;
    Ok(summary_for_solder_paste(&paste))
}

fn parse_gerber_outline(text: &str, path: &Path) -> Result<GerberOutline> {
    let mut state = GerberState::default();
    let mut layer = "gerber_outline".to_string();
    let mut segments = Vec::new();
    let mut source_primitive_index = 0;
    for raw_record in text.split('*') {
        let record = raw_record.replace('%', "");
        let record = record.trim();
        if record.is_empty() {
            continue;
        }
        if let Some(comment) = record.strip_prefix("G04") {
            if let Some(comment_layer) = comment.trim().strip_prefix("Layer:") {
                let comment_layer = comment_layer.trim();
                if !comment_layer.is_empty() {
                    layer = comment_layer.to_string();
                }
            }
            continue;
        }
        if let Some(format_record) = record.strip_prefix("FSLAX") {
            state.format = Some(parse_coordinate_format(format_record, path)?);
            continue;
        }
        if record == "MOMM" {
            state.units_mm = true;
            continue;
        }
        if record == "MOIN" {
            bail!(
                "Gerber outline {} uses inches; only millimeter outline imports are currently supported.",
                path.display()
            );
        }
        if record == "G90" {
            state.absolute = true;
            continue;
        }
        if record == "G91" {
            bail!(
                "Gerber outline {} uses incremental coordinates; only absolute coordinates are supported.",
                path.display()
            );
        }
        if record == "G01" || record.starts_with("G01X") || record.starts_with("G01Y") {
            state.line_mode = true;
        } else if record.starts_with("G02") || record.starts_with("G03") {
            bail!(
                "Gerber outline {} contains arc interpolation {}; import only supports linear outline draws.",
                path.display(),
                record
            );
        } else if matches!(record, "G75" | "G74") || record.starts_with("G54D") {
            continue;
        } else if record == "M02" {
            break;
        } else if is_aperture_parameter(record) || is_aperture_selection(record) {
            continue;
        }

        if !(record.contains('X') || record.contains('Y') || record.contains('D')) {
            continue;
        }
        if !state.line_mode {
            bail!(
                "Gerber outline {} has coordinate record before linear interpolation mode: {}.",
                path.display(),
                record
            );
        }
        let operation = parse_operation(record)?;
        if operation.is_some() {
            state.modal_operation = operation;
        }
        let has_coordinates = record.contains('X') || record.contains('Y');
        if !has_coordinates {
            continue;
        }
        let Some(format) = state.format else {
            bail!(
                "Gerber outline {} has coordinates before an FS coordinate format record.",
                path.display()
            );
        };
        if !state.units_mm {
            bail!(
                "Gerber outline {} has coordinates before MOMM millimeter units.",
                path.display()
            );
        }
        if !state.absolute {
            bail!(
                "Gerber outline {} is not in absolute coordinate mode.",
                path.display()
            );
        }
        let target = parse_target_point(record, format, state.current, path)?;
        match state.modal_operation {
            Some(GerberOperation::Move) => {
                state.current = Some(target);
            }
            Some(GerberOperation::Draw) => {
                let Some(start) = state.current else {
                    bail!(
                        "Gerber outline {} has draw command before a current position.",
                        path.display()
                    );
                };
                if point_distance_mm(start, target) <= POINT_EPSILON_MM {
                    bail!(
                        "Gerber outline {} has zero-length outline draw.",
                        path.display()
                    );
                }
                segments.push(GerberOutlineSegment {
                    start,
                    end: target,
                    source_primitive_index,
                    contour_index: None,
                    boundary_role: GerberBoundaryRole::Unknown,
                });
                source_primitive_index += 1;
                state.current = Some(target);
            }
            Some(GerberOperation::Flash) => {
                bail!(
                    "Gerber outline {} contains D03 flash geometry; outline flashes are not currently imported.",
                    path.display()
                );
            }
            None => {
                bail!(
                    "Gerber outline {} has coordinates without D01/D02 modal operation.",
                    path.display()
                );
            }
        }
    }
    if segments.is_empty() {
        bail!(
            "Gerber outline {} produced no linear outline segments.",
            path.display()
        );
    }
    classify_outline_contours(&mut segments);
    Ok(GerberOutline { layer, segments })
}

fn parse_gerber_copper(text: &str, path: &Path) -> Result<GerberCopper> {
    let mut state = GerberState::default();
    let mut layer = "gerber_copper".to_string();
    let mut apertures = std::collections::BTreeMap::<u32, GerberAperture>::new();
    let mut features = Vec::new();
    let mut segments = Vec::new();
    let mut regions = Vec::new();
    let mut region_points: Option<Vec<GerberPoint>> = None;
    let mut source_primitive_index = 0;
    let mut ignored_draws = 0;
    let mut skipped_clear_flashes = 0;
    let mut skipped_clear_regions = 0;
    for raw_record in text.split('*') {
        let record = raw_record.replace('%', "");
        let record = record.trim();
        if record.is_empty() {
            continue;
        }
        if let Some(comment) = record.strip_prefix("G04") {
            if let Some(comment_layer) = comment.trim().strip_prefix("Layer:") {
                let comment_layer = comment_layer.trim();
                if !comment_layer.is_empty() {
                    layer = comment_layer.to_string();
                }
            }
            continue;
        }
        if let Some(format_record) = record.strip_prefix("FSLAX") {
            state.format = Some(parse_coordinate_format(format_record, path)?);
            continue;
        }
        if record == "MOMM" {
            state.units_mm = true;
            continue;
        }
        if record == "MOIN" {
            bail!(
                "Gerber copper {} uses inches; only millimeter copper imports are currently supported.",
                path.display()
            );
        }
        if record == "G90" {
            state.absolute = true;
            continue;
        }
        if record == "G91" {
            bail!(
                "Gerber copper {} uses incremental coordinates; only absolute coordinates are supported.",
                path.display()
            );
        }
        if record == "LPD" {
            state.dark_polarity = true;
            continue;
        }
        if record == "LPC" {
            state.dark_polarity = false;
            continue;
        }
        if record == "G36" {
            if region_points.is_some() {
                bail!(
                    "Gerber copper {} starts nested G36 regions; nested regions are unsupported.",
                    path.display()
                );
            }
            region_points = Some(Vec::new());
            state.current = None;
            state.modal_operation = None;
            continue;
        }
        if record == "G37" {
            let Some(mut points) = region_points.take() else {
                bail!(
                    "Gerber copper {} ends a G37 region before G36.",
                    path.display()
                );
            };
            if points.len() < 3 {
                bail!(
                    "Gerber copper {} has a G36/G37 region with fewer than three points.",
                    path.display()
                );
            }
            if let (Some(first), Some(last)) = (points.first().copied(), points.last().copied())
                && points_close(first, last)
            {
                points.pop();
            }
            if points.len() < 3 || polygon_signed_area_mm2(&points).abs() <= f64::EPSILON {
                bail!(
                    "Gerber copper {} has a degenerate G36/G37 region.",
                    path.display()
                );
            }
            if state.dark_polarity {
                regions.push(GerberCopperRegion {
                    points,
                    source_primitive_index,
                    net: None,
                    island_id: None,
                });
                source_primitive_index += 1;
            } else {
                skipped_clear_regions += 1;
            }
            state.current = None;
            state.modal_operation = None;
            continue;
        }
        if let Some(aperture) = parse_aperture_definition(record, path)? {
            if apertures.insert(aperture.0, aperture.1).is_some() {
                bail!(
                    "Gerber copper {} defines aperture D{} more than once.",
                    path.display(),
                    aperture.0
                );
            }
            continue;
        }
        let arc_direction = if record.starts_with("G02") {
            Some(GerberArcDirection::Clockwise)
        } else if record.starts_with("G03") {
            Some(GerberArcDirection::CounterClockwise)
        } else {
            None
        };
        if record == "G01" || record.starts_with("G01X") || record.starts_with("G01Y") {
            state.line_mode = true;
        } else if let Some(selection) = record.strip_prefix("G54") {
            if let Some(code) = aperture_selection_code(selection) {
                if !apertures.contains_key(&code) {
                    bail!(
                        "Gerber copper {} selects undefined aperture D{}.",
                        path.display(),
                        code
                    );
                }
                state.aperture_code = Some(code);
                continue;
            }
            bail!(
                "Gerber copper {} has unsupported aperture selection record {}.",
                path.display(),
                record
            );
        } else if matches!(record, "G75" | "G74") {
            continue;
        } else if let Some(code) = aperture_selection_code(record) {
            if !apertures.contains_key(&code) {
                bail!(
                    "Gerber copper {} selects undefined aperture D{}.",
                    path.display(),
                    code
                );
            }
            state.aperture_code = Some(code);
            continue;
        } else if record == "M02" {
            break;
        }

        if !(record.contains('X') || record.contains('Y') || record.contains('D')) {
            continue;
        }
        if arc_direction.is_none() && !state.line_mode {
            bail!(
                "Gerber copper {} has coordinate record before linear interpolation mode: {}.",
                path.display(),
                record
            );
        }
        let operation = parse_operation(record)?;
        if operation.is_some() {
            state.modal_operation = operation;
        }
        let has_coordinates = record.contains('X') || record.contains('Y');
        if !has_coordinates {
            continue;
        }
        let Some(format) = state.format else {
            bail!(
                "Gerber copper {} has coordinates before an FS coordinate format record.",
                path.display()
            );
        };
        if !state.units_mm {
            bail!(
                "Gerber copper {} has coordinates before MOMM millimeter units.",
                path.display()
            );
        }
        if !state.absolute {
            bail!(
                "Gerber copper {} is not in absolute coordinate mode.",
                path.display()
            );
        }
        let target = parse_target_point(record, format, state.current, path)?;
        if let Some(points) = region_points.as_mut() {
            match state.modal_operation {
                Some(GerberOperation::Move) => {
                    if points.is_empty() {
                        points.push(target);
                    } else if !points_close(points[0], target) {
                        bail!(
                            "Gerber copper {} has multiple contours in one G36/G37 region; only single-contour regions are supported.",
                            path.display()
                        );
                    }
                    state.current = Some(target);
                    continue;
                }
                Some(GerberOperation::Draw) => {
                    if points.is_empty() {
                        let Some(start) = state.current else {
                            bail!(
                                "Gerber copper {} has region draw command before a region start point.",
                                path.display()
                            );
                        };
                        points.push(start);
                    }
                    if let Some(direction) = arc_direction {
                        let Some(start) = state.current else {
                            bail!(
                                "Gerber copper {} has region arc command before a current position.",
                                path.display()
                            );
                        };
                        for point in
                            sample_arc_points(start, target, record, format, path, direction)?
                        {
                            if point_distance_mm(
                                *points.last().expect("points is not empty"),
                                point,
                            ) > POINT_EPSILON_MM
                            {
                                points.push(point);
                            }
                        }
                    } else if point_distance_mm(
                        *points.last().expect("points is not empty"),
                        target,
                    ) > POINT_EPSILON_MM
                    {
                        points.push(target);
                    }
                    state.current = Some(target);
                    continue;
                }
                Some(GerberOperation::Flash) => {
                    bail!(
                        "Gerber copper {} has a D03 flash inside a G36/G37 region.",
                        path.display()
                    );
                }
                None => {
                    bail!(
                        "Gerber copper {} has region coordinates without D01/D02 modal operation.",
                        path.display()
                    );
                }
            }
        }
        match state.modal_operation {
            Some(GerberOperation::Move) => {
                state.current = Some(target);
            }
            Some(GerberOperation::Draw) => {
                let Some(start) = state.current else {
                    bail!(
                        "Gerber copper {} has draw command before a current position.",
                        path.display()
                    );
                };
                let Some(aperture_code) = state.aperture_code else {
                    bail!(
                        "Gerber copper {} draws before selecting an aperture.",
                        path.display()
                    );
                };
                let aperture = apertures.get(&aperture_code).cloned().with_context(|| {
                    format!(
                        "Gerber copper {} draws with undefined aperture D{}.",
                        path.display(),
                        aperture_code
                    )
                })?;
                if point_distance_mm(start, target) <= POINT_EPSILON_MM {
                    bail!(
                        "Gerber copper {} has zero-length linear draw.",
                        path.display()
                    );
                }
                if state.dark_polarity && aperture.shape == GerberApertureShape::Circle {
                    if let Some(direction) = arc_direction {
                        let mut segment_start = start;
                        for point in
                            sample_arc_points(start, target, record, format, path, direction)?
                        {
                            if point_distance_mm(segment_start, point) <= POINT_EPSILON_MM {
                                continue;
                            }
                            segments.push(GerberCopperSegment {
                                start: segment_start,
                                end: point,
                                aperture_code,
                                aperture: aperture.clone(),
                                source_primitive: "gerber_arc_draw",
                                source_primitive_index,
                                net: None,
                                island_id: None,
                            });
                            source_primitive_index += 1;
                            segment_start = point;
                        }
                    } else {
                        segments.push(GerberCopperSegment {
                            start,
                            end: target,
                            aperture_code,
                            aperture,
                            source_primitive: "gerber_linear_draw",
                            source_primitive_index,
                            net: None,
                            island_id: None,
                        });
                        source_primitive_index += 1;
                    }
                } else {
                    ignored_draws += 1;
                }
                state.current = Some(target);
            }
            Some(GerberOperation::Flash) => {
                let Some(aperture_code) = state.aperture_code else {
                    bail!(
                        "Gerber copper {} flashes before selecting an aperture.",
                        path.display()
                    );
                };
                let aperture = apertures.get(&aperture_code).cloned().with_context(|| {
                    format!(
                        "Gerber copper {} flashes undefined aperture D{}.",
                        path.display(),
                        aperture_code
                    )
                })?;
                if state.dark_polarity {
                    features.push(GerberCopperFeature {
                        at: target,
                        aperture_code,
                        aperture,
                        source_primitive_index,
                        net: None,
                        island_id: None,
                        owner_kind: None,
                        component: None,
                        pin: None,
                        via_index: None,
                    });
                    source_primitive_index += 1;
                } else {
                    skipped_clear_flashes += 1;
                }
                state.current = Some(target);
            }
            None => {
                bail!(
                    "Gerber copper {} has coordinates without D01/D02/D03 modal operation.",
                    path.display()
                );
            }
        }
    }
    if region_points.is_some() {
        bail!(
            "Gerber copper {} starts a G36 region without a matching G37.",
            path.display()
        );
    }
    if features.is_empty() && segments.is_empty() && regions.is_empty() {
        bail!(
            "Gerber copper {} produced no dark flash, circular-aperture linear draw, or region copper evidence.",
            path.display()
        );
    }
    Ok(GerberCopper {
        layer,
        features,
        segments,
        regions,
        aperture_count: apertures.len(),
        ignored_draws,
        skipped_clear_flashes,
        skipped_clear_regions,
    })
}

fn parse_coordinate_format(record: &str, path: &Path) -> Result<CoordinateFormat> {
    let Some((x_format, y_part)) = record.split_once('Y') else {
        bail!(
            "Gerber outline {} has unsupported FS coordinate format FSLAX{}.",
            path.display(),
            record
        );
    };
    if x_format.len() != 2 || y_part.len() < 2 {
        bail!(
            "Gerber outline {} has unsupported FS coordinate format FSLAX{}.",
            path.display(),
            record
        );
    }
    let y_format = &y_part[..2];
    let x_decimals = x_format[1..2].parse::<u32>().with_context(|| {
        format!(
            "Gerber outline {} has invalid X decimal count in FS record.",
            path.display()
        )
    })?;
    let y_decimals = y_format[1..2].parse::<u32>().with_context(|| {
        format!(
            "Gerber outline {} has invalid Y decimal count in FS record.",
            path.display()
        )
    })?;
    if x_decimals == 0 || y_decimals == 0 {
        bail!(
            "Gerber outline {} has unsupported zero-decimal coordinate format.",
            path.display()
        );
    }
    Ok(CoordinateFormat {
        x_decimals,
        y_decimals,
    })
}

fn parse_operation(record: &str) -> Result<Option<GerberOperation>> {
    let Some(index) = record.rfind('D') else {
        return Ok(None);
    };
    let code_text = record[index + 1..]
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect::<String>();
    if code_text.is_empty() {
        return Ok(None);
    }
    Ok(match code_text.as_str() {
        "01" | "1" => Some(GerberOperation::Draw),
        "02" | "2" => Some(GerberOperation::Move),
        "03" | "3" => Some(GerberOperation::Flash),
        _ => None,
    })
}

fn parse_target_point(
    record: &str,
    format: CoordinateFormat,
    current: Option<GerberPoint>,
    path: &Path,
) -> Result<GerberPoint> {
    let x_mm = match coordinate_field(record, 'X') {
        Some(value) => parse_coordinate_mm(value, format.x_decimals, path)?,
        None => current.map(|point| point.x_mm).with_context(|| {
            format!(
                "Gerber outline {} omits X before a current point.",
                path.display()
            )
        })?,
    };
    let y_mm = match coordinate_field(record, 'Y') {
        Some(value) => parse_coordinate_mm(value, format.y_decimals, path)?,
        None => current.map(|point| point.y_mm).with_context(|| {
            format!(
                "Gerber outline {} omits Y before a current point.",
                path.display()
            )
        })?,
    };
    if !(x_mm.is_finite() && y_mm.is_finite()) {
        bail!(
            "Gerber outline {} produced non-finite coordinates.",
            path.display()
        );
    }
    Ok(GerberPoint { x_mm, y_mm })
}

fn sample_arc_points(
    start: GerberPoint,
    target: GerberPoint,
    record: &str,
    format: CoordinateFormat,
    path: &Path,
    direction: GerberArcDirection,
) -> Result<Vec<GerberPoint>> {
    let i_mm =
        coordinate_offset_field(record, 'I', format.x_decimals, path)?.with_context(|| {
            format!(
                "Gerber copper {} arc record {} is missing I center offset.",
                path.display(),
                record
            )
        })?;
    let j_mm =
        coordinate_offset_field(record, 'J', format.y_decimals, path)?.with_context(|| {
            format!(
                "Gerber copper {} arc record {} is missing J center offset.",
                path.display(),
                record
            )
        })?;
    let center = GerberPoint {
        x_mm: start.x_mm + i_mm,
        y_mm: start.y_mm + j_mm,
    };
    let radius_mm = point_distance_mm(start, center);
    let target_radius_mm = point_distance_mm(target, center);
    if !(radius_mm.is_finite() && target_radius_mm.is_finite()) || radius_mm <= POINT_EPSILON_MM {
        bail!(
            "Gerber copper {} arc record {} has invalid radius.",
            path.display(),
            record
        );
    }
    if (target_radius_mm - radius_mm).abs() > 0.05 {
        bail!(
            "Gerber copper {} arc record {} has inconsistent start/end radius.",
            path.display(),
            record
        );
    }
    let start_angle = (start.y_mm - center.y_mm).atan2(start.x_mm - center.x_mm);
    let end_angle = (target.y_mm - center.y_mm).atan2(target.x_mm - center.x_mm);
    let sweep = match direction {
        GerberArcDirection::CounterClockwise => {
            let mut value = end_angle - start_angle;
            if value <= 0.0 {
                value += std::f64::consts::TAU;
            }
            value
        }
        GerberArcDirection::Clockwise => {
            let mut value = start_angle - end_angle;
            if value <= 0.0 {
                value += std::f64::consts::TAU;
            }
            value
        }
    };
    let angular_steps = (sweep / GERBER_ARC_MAX_STEP_DEG.to_radians()).ceil() as usize;
    let chord_steps = ((radius_mm * sweep) / GERBER_ARC_MAX_CHORD_MM).ceil() as usize;
    let steps = angular_steps
        .max(chord_steps)
        .clamp(1, GERBER_ARC_MAX_SEGMENTS);
    let mut points = Vec::with_capacity(steps);
    for step in 1..=steps {
        if step == steps {
            points.push(target);
            continue;
        }
        let fraction = step as f64 / steps as f64;
        let angle = match direction {
            GerberArcDirection::CounterClockwise => start_angle + sweep * fraction,
            GerberArcDirection::Clockwise => start_angle - sweep * fraction,
        };
        points.push(GerberPoint {
            x_mm: center.x_mm + radius_mm * angle.cos(),
            y_mm: center.y_mm + radius_mm * angle.sin(),
        });
    }
    Ok(points)
}

fn coordinate_field(record: &str, axis: char) -> Option<&str> {
    let start = record.find(axis)? + axis.len_utf8();
    let bytes = record.as_bytes();
    let mut end = start;
    if matches!(bytes.get(end), Some(b'-' | b'+')) {
        end += 1;
    }
    while matches!(bytes.get(end), Some(byte) if byte.is_ascii_digit()) {
        end += 1;
    }
    (end > start).then_some(&record[start..end])
}

fn coordinate_offset_field(
    record: &str,
    axis: char,
    decimals: u32,
    path: &Path,
) -> Result<Option<f64>> {
    coordinate_field(record, axis)
        .map(|value| parse_coordinate_mm(value, decimals, path))
        .transpose()
}

fn parse_coordinate_mm(value: &str, decimals: u32, path: &Path) -> Result<f64> {
    let raw = value.parse::<i64>().with_context(|| {
        format!(
            "Gerber outline {} has invalid coordinate value {}.",
            path.display(),
            value
        )
    })?;
    Ok(raw as f64 / 10_i64.pow(decimals) as f64)
}

fn is_aperture_parameter(record: &str) -> bool {
    record.starts_with("ADD")
}

fn is_aperture_selection(record: &str) -> bool {
    aperture_selection_code(record).is_some()
}

fn aperture_selection_code(record: &str) -> Option<u32> {
    record
        .strip_prefix('D')
        .and_then(|code| code.parse::<u32>().ok())
        .filter(|value| *value >= 10)
}

fn parse_aperture_definition(record: &str, path: &Path) -> Result<Option<(u32, GerberAperture)>> {
    let Some(definition) = record.strip_prefix("ADD") else {
        return Ok(None);
    };
    let code_end = definition
        .find(|character: char| !character.is_ascii_digit())
        .with_context(|| {
            format!(
                "Gerber copper {} has malformed aperture definition {}.",
                path.display(),
                record
            )
        })?;
    let code = definition[..code_end].parse::<u32>().with_context(|| {
        format!(
            "Gerber copper {} has invalid aperture code in {}.",
            path.display(),
            record
        )
    })?;
    if code < 10 {
        bail!(
            "Gerber copper {} defines reserved aperture D{}.",
            path.display(),
            code
        );
    }
    let body = &definition[code_end..];
    let (shape_code, parameters) = body.split_once(',').with_context(|| {
        format!(
            "Gerber copper {} has aperture definition without size parameters: {}.",
            path.display(),
            record
        )
    })?;
    let parameter_text = parameters.trim();
    if parameter_text.is_empty() {
        bail!(
            "Gerber copper {} has aperture definition without size parameters: {}.",
            path.display(),
            record
        );
    }
    let dimensions = parameter_text
        .split('X')
        .map(|value| {
            value.parse::<f64>().with_context(|| {
                format!(
                    "Gerber copper {} has invalid aperture size {} in {}.",
                    path.display(),
                    value,
                    record
                )
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let aperture = match shape_code {
        "C" => {
            if dimensions.len() != 1 {
                bail!(
                    "Gerber copper {} circle aperture must have one diameter: {}.",
                    path.display(),
                    record
                );
            }
            ensure_positive_aperture_dimensions(&dimensions, path, record)?;
            GerberAperture {
                shape: GerberApertureShape::Circle,
                x_mm: dimensions[0],
                y_mm: dimensions[0],
            }
        }
        "R" => {
            if dimensions.len() != 2 {
                bail!(
                    "Gerber copper {} rectangle aperture must have X and Y sizes: {}.",
                    path.display(),
                    record
                );
            }
            ensure_positive_aperture_dimensions(&dimensions, path, record)?;
            GerberAperture {
                shape: GerberApertureShape::Rect,
                x_mm: dimensions[0],
                y_mm: dimensions[1],
            }
        }
        "O" => {
            if dimensions.len() != 2 {
                bail!(
                    "Gerber copper {} oval aperture must have X and Y sizes: {}.",
                    path.display(),
                    record
                );
            }
            ensure_positive_aperture_dimensions(&dimensions, path, record)?;
            GerberAperture {
                shape: GerberApertureShape::Oval,
                x_mm: dimensions[0],
                y_mm: dimensions[1],
            }
        }
        "RoundRect" => parse_easyeda_round_rect_aperture(&dimensions, path, record)?,
        _ => {
            bail!(
                "Gerber copper {} uses unsupported aperture shape {} in {}; supported shapes are C, R, O, and EasyEDA RoundRect.",
                path.display(),
                shape_code,
                record
            );
        }
    };
    Ok(Some((code, aperture)))
}

fn ensure_positive_aperture_dimensions(
    dimensions: &[f64],
    path: &Path,
    record: &str,
) -> Result<()> {
    if dimensions
        .iter()
        .any(|value| !value.is_finite() || *value <= 0.0)
    {
        bail!(
            "Gerber copper {} has non-positive aperture size in {}.",
            path.display(),
            record
        );
    }
    Ok(())
}

fn parse_easyeda_round_rect_aperture(
    dimensions: &[f64],
    path: &Path,
    record: &str,
) -> Result<GerberAperture> {
    if dimensions.len() != 5 {
        bail!(
            "Gerber copper {} EasyEDA RoundRect aperture must have stroke diameter and four corner coordinates: {}.",
            path.display(),
            record
        );
    }
    if dimensions.iter().any(|value| !value.is_finite()) || dimensions[0] <= 0.0 {
        bail!(
            "Gerber copper {} has invalid EasyEDA RoundRect aperture dimensions in {}.",
            path.display(),
            record
        );
    }
    let stroke_mm = dimensions[0];
    let max_x_mm = dimensions[1].abs().max(dimensions[3].abs());
    let max_y_mm = dimensions[2].abs().max(dimensions[4].abs());
    let x_mm = 2.0 * max_x_mm + stroke_mm;
    let y_mm = 2.0 * max_y_mm + stroke_mm;
    if !(x_mm.is_finite() && y_mm.is_finite() && x_mm > 0.0 && y_mm > 0.0) {
        bail!(
            "Gerber copper {} produced invalid EasyEDA RoundRect aperture bounds in {}.",
            path.display(),
            record
        );
    }
    Ok(GerberAperture {
        shape: GerberApertureShape::Rect,
        x_mm,
        y_mm,
    })
}

struct OutlineContour {
    segment_indices: Vec<usize>,
    points: Vec<GerberPoint>,
    area_mm2: f64,
}

fn classify_outline_contours(segments: &mut [GerberOutlineSegment]) {
    let contours = outline_contours(segments);
    for (contour_index, contour) in contours.iter().enumerate() {
        let containing_contours = contours
            .iter()
            .enumerate()
            .filter(|(other_index, other)| {
                *other_index != contour_index
                    && other.area_mm2.abs() > contour.area_mm2.abs()
                    && point_inside_polygon(contour_representative_point(contour), &other.points)
            })
            .count();
        let boundary_role = if containing_contours % 2 == 0 {
            GerberBoundaryRole::External
        } else {
            GerberBoundaryRole::Cutout
        };
        for segment_index in &contour.segment_indices {
            segments[*segment_index].contour_index = Some(contour_index);
            segments[*segment_index].boundary_role = boundary_role;
        }
    }
}

fn outline_contours(segments: &[GerberOutlineSegment]) -> Vec<OutlineContour> {
    let mut contours = Vec::new();
    let mut used = vec![false; segments.len()];
    for first_index in 0..segments.len() {
        if used[first_index] {
            continue;
        }
        let start = segments[first_index].start;
        let mut current = segments[first_index].end;
        let mut segment_indices = vec![first_index];
        let mut points = vec![start, current];
        used[first_index] = true;
        loop {
            if points_close(current, start) {
                break;
            }
            let Some((next_index, next_point)) = segments
                .iter()
                .enumerate()
                .filter(|(index, _)| !used[*index])
                .find_map(|(index, segment)| {
                    if points_close(segment.start, current) {
                        Some((index, segment.end))
                    } else if points_close(segment.end, current) {
                        Some((index, segment.start))
                    } else {
                        None
                    }
                })
            else {
                break;
            };
            used[next_index] = true;
            segment_indices.push(next_index);
            current = next_point;
            points.push(current);
        }
        if segment_indices.len() >= 3 && points_close(current, start) {
            points.pop();
            let area_mm2 = polygon_signed_area_mm2(&points);
            if area_mm2.abs() > f64::EPSILON {
                contours.push(OutlineContour {
                    segment_indices,
                    points,
                    area_mm2,
                });
            }
        }
    }
    contours
}

fn contour_representative_point(contour: &OutlineContour) -> GerberPoint {
    let count = contour.points.len() as f64;
    GerberPoint {
        x_mm: contour.points.iter().map(|point| point.x_mm).sum::<f64>() / count,
        y_mm: contour.points.iter().map(|point| point.y_mm).sum::<f64>() / count,
    }
}

fn polygon_signed_area_mm2(points: &[GerberPoint]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(first, second)| first.x_mm * second.y_mm - second.x_mm * first.y_mm)
        .sum::<f64>()
        / 2.0
}

fn point_inside_polygon(point: GerberPoint, polygon: &[GerberPoint]) -> bool {
    let mut inside = false;
    for (first, second) in polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(polygon.len())
    {
        let crosses_y = (first.y_mm > point.y_mm) != (second.y_mm > point.y_mm);
        if crosses_y {
            let intersection_x = (second.x_mm - first.x_mm) * (point.y_mm - first.y_mm)
                / (second.y_mm - first.y_mm)
                + first.x_mm;
            if point.x_mm < intersection_x {
                inside = !inside;
            }
        }
    }
    inside
}

fn point_distance_mm(first: GerberPoint, second: GerberPoint) -> f64 {
    (second.x_mm - first.x_mm).hypot(second.y_mm - first.y_mm)
}

fn points_close(first: GerberPoint, second: GerberPoint) -> bool {
    point_distance_mm(first, second) <= POINT_EPSILON_MM
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
        let absolute = fs::canonicalize(&resolved).unwrap_or(resolved);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_linear_outline_and_cutout_roles() {
        let text = r#"
G04 Layer: BoardOutlineLayer*
%FSLAX45Y45*%
%MOMM*%
G75*
G54D10*
G01X0Y0D02*
G01X1000000Y0D01*
G01X1000000Y-1000000D01*
G01X0Y-1000000D01*
G01X0Y0D01*
G01X200000Y-200000D02*
G01X400000Y-200000D01*
G01X400000Y-400000D01*
G01X200000Y-400000D01*
G01X200000Y-200000D01*
M02*
"#;
        let outline = parse_gerber_outline(text, Path::new("fixture.gko")).unwrap();
        assert_eq!(outline.layer, "BoardOutlineLayer");
        assert_eq!(outline.segments.len(), 8);
        assert_eq!(summary_for_outline(&outline).external_segments, 4);
        assert_eq!(summary_for_outline(&outline).cutout_segments, 4);
        assert_eq!(outline.segments[0].end.x_mm, 10.0);
        assert_eq!(outline.segments[4].start.x_mm, 2.0);
    }

    #[test]
    fn rejects_inch_units() {
        let text = "%FSLAX45Y45*%\n%MOIN*%\nG01X0Y0D02*\n";
        let error = parse_gerber_outline(text, Path::new("bad.gko")).unwrap_err();
        assert!(error.to_string().contains("uses inches"));
    }
}
