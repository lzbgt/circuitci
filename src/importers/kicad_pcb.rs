use super::kicad_sch::sexp::{
    Sexp, as_list, child_list, list_children, numeric_at, parse_sexp_document, string_at, tag,
};
use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_yaml_ng::{Mapping, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug)]
pub struct KicadPcbPlacementImportOptions {
    pub input: PathBuf,
    pub project: PathBuf,
    pub output: PathBuf,
}

#[derive(Debug, Clone)]
struct PcbPlacement {
    x_mm: f64,
    y_mm: f64,
    side: Option<PcbPlacementSide>,
    rotation_deg: Option<f64>,
}

#[derive(Debug, Clone, Default)]
struct PcbFootprint {
    segments: Vec<PcbFootprintSegment>,
    rectangles: Vec<PcbFootprintRectangle>,
}

#[derive(Debug, Clone)]
struct PcbFootprintSegment {
    start: PcbPoint,
    end: PcbPoint,
    layer: String,
    kind: String,
}

#[derive(Debug, Clone)]
struct PcbFootprintRectangle {
    start: PcbPoint,
    end: PcbPoint,
    layer: String,
    kind: String,
}

#[derive(Debug, Clone, Copy)]
struct FootprintAt {
    x_mm: f64,
    y_mm: f64,
    rotation_deg: f64,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum PcbPlacementSide {
    Top,
    Bottom,
}

#[derive(Debug, Serialize)]
struct PlacementYaml {
    x_mm: f64,
    y_mm: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    side: Option<PcbPlacementSide>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rotation_deg: Option<f64>,
}

#[derive(Debug, Serialize)]
struct FootprintYaml {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    segments: Vec<FootprintSegmentYaml>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    rectangles: Vec<FootprintRectangleYaml>,
}

#[derive(Debug, Serialize)]
struct FootprintSegmentYaml {
    start: PcbPoint,
    end: PcbPoint,
    layer: String,
    kind: String,
}

#[derive(Debug, Serialize)]
struct FootprintRectangleYaml {
    start: PcbPoint,
    end: PcbPoint,
    layer: String,
    kind: String,
}

#[derive(Debug, Clone, Default)]
pub struct KicadPcbImportSummary {
    pub placements: usize,
    pub footprint_graphics: usize,
    pub pads: usize,
    pub outline_segments: usize,
    pub route_segments: usize,
    pub route_vias: usize,
    pub zones: usize,
    pub routing_constraints: usize,
}

#[derive(Debug, Clone, Default)]
struct PcbRoute {
    segments: Vec<PcbRouteSegment>,
    vias: Vec<PcbRouteVia>,
}

#[derive(Debug, Clone)]
struct PcbPad {
    at: PcbPoint,
    net_name: String,
    layers: Vec<String>,
    kind: Option<String>,
    shape: Option<String>,
    size: PcbPadSize,
    rotation_deg: Option<f64>,
    drill_mm: Option<f64>,
}

#[derive(Debug, Clone)]
struct PcbRouteSegment {
    start: PcbPoint,
    end: PcbPoint,
    width_mm: f64,
    layer: String,
}

#[derive(Debug, Clone)]
struct PcbRouteVia {
    at: PcbPoint,
    size_mm: f64,
    drill_mm: f64,
    layers: Vec<String>,
}

#[derive(Debug, Clone)]
struct PcbZone {
    layer: String,
    polygon: Vec<PcbPoint>,
    filled_polygons: Vec<Vec<PcbPoint>>,
}

#[derive(Debug, Clone)]
struct PcbOutline {
    segments: Vec<PcbOutlineSegment>,
}

#[derive(Debug, Clone)]
struct PcbOutlineSegment {
    start: PcbPoint,
    end: PcbPoint,
    layer: String,
}

#[derive(Debug, Clone, Default)]
struct PcbNetRule {
    net_class: Option<String>,
    track_width_mm: Option<f64>,
    diff_pair_width_mm: Option<f64>,
    diff_pair_gap_mm: Option<f64>,
    length_max_mm: Option<f64>,
    skew_max_mm: Option<f64>,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct PcbPoint {
    x_mm: f64,
    y_mm: f64,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct PcbPadSize {
    x_mm: f64,
    y_mm: f64,
}

#[derive(Debug, Serialize)]
struct RouteYaml<'a> {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    segments: Vec<RouteSegmentYaml>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    vias: Vec<RouteViaYaml<'a>>,
}

#[derive(Debug, Serialize)]
struct PadYaml<'a> {
    at: PcbPoint,
    net: &'a str,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    layers: Vec<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kind: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shape: Option<&'a str>,
    size: PcbPadSize,
    #[serde(skip_serializing_if = "Option::is_none")]
    rotation_deg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    drill_mm: Option<f64>,
}

#[derive(Debug, Serialize)]
struct RouteSegmentYaml {
    start: PcbPoint,
    end: PcbPoint,
    width_mm: f64,
    layer: String,
}

#[derive(Debug, Serialize)]
struct RouteViaYaml<'a> {
    at: PcbPoint,
    size_mm: f64,
    drill_mm: f64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    layers: Vec<&'a str>,
}

#[derive(Debug, Serialize)]
struct NetRuleYaml<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    net_class: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    track_width_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff_pair_width_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diff_pair_gap_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    length_max_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    skew_max_mm: Option<f64>,
}

#[derive(Debug, Serialize)]
struct ZoneYaml {
    layer: String,
    polygon: Vec<PcbPoint>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    filled_polygons: Vec<Vec<PcbPoint>>,
}

#[derive(Debug, Serialize)]
struct OutlineYaml {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    segments: Vec<OutlineSegmentYaml>,
}

#[derive(Debug, Serialize)]
struct OutlineSegmentYaml {
    start: PcbPoint,
    end: PcbPoint,
    layer: String,
}

pub fn import_kicad_pcb_placements(
    options: &KicadPcbPlacementImportOptions,
) -> Result<KicadPcbImportSummary> {
    let parsed_pcb = parse_kicad_pcb(&options.input)?;
    let text = fs::read_to_string(&options.project).with_context(|| {
        format!(
            "Failed to read Board IR project {}",
            options.project.display()
        )
    })?;
    let mut project_yaml: Value = serde_yaml_ng::from_str(&text).with_context(|| {
        format!(
            "Failed to parse Board IR project YAML {}",
            options.project.display()
        )
    })?;
    let summary = merge_pcb_into_project(&mut project_yaml, &parsed_pcb)?;
    if summary.placements == 0 {
        bail!(
            "KiCad PCB {} has no footprint references matching Board IR project components in {}.",
            options.input.display(),
            options.project.display()
        );
    }
    if let Some(parent) = options.output.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "Failed to create import output directory {}",
                parent.display()
            )
        })?;
    }
    absolutize_relative_libraries(
        &mut project_yaml,
        options.project.parent().unwrap_or_else(|| Path::new(".")),
    )?;
    let mut yaml = serde_yaml_ng::to_string(&project_yaml)?;
    yaml.insert_str(
        0,
        "# Generated by CircuitCI by adding KiCad PCB layout evidence to Board IR.\n",
    );
    fs::write(&options.output, yaml)
        .with_context(|| format!("Failed to write {}", options.output.display()))?;
    Ok(summary)
}

#[derive(Debug, Clone)]
struct ParsedPcb {
    placements: BTreeMap<String, PcbPlacement>,
    footprints: BTreeMap<String, PcbFootprint>,
    pads: BTreeMap<String, BTreeMap<String, PcbPad>>,
    outline: PcbOutline,
    routes: BTreeMap<String, PcbRoute>,
    zones: BTreeMap<String, Vec<PcbZone>>,
    net_rules: BTreeMap<String, PcbNetRule>,
}

fn parse_kicad_pcb(path: &PathBuf) -> Result<ParsedPcb> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("Failed to read KiCad PCB {}", path.display()))?;
    let root = parse_sexp_document(&text)?;
    let root_list = as_list(&root).context("KiCad PCB must be an S-expression list.")?;
    if tag(root_list) != Some("kicad_pcb") {
        bail!("KiCad PCB {} root token is not kicad_pcb.", path.display());
    }
    let placements = parse_placements(root_list, path)?;
    let footprints = parse_footprints(root_list, path)?;
    let pads = parse_pads(root_list, path)?;
    let outline = parse_outline(root_list, path)?;
    let routes = parse_routes(root_list, path)?;
    let zones = parse_zones(root_list, path)?;
    let net_rules = parse_net_rules(root_list, path)?;
    Ok(ParsedPcb {
        placements,
        footprints,
        pads,
        outline,
        routes,
        zones,
        net_rules,
    })
}

fn parse_pads(
    root_list: &[Sexp],
    path: &Path,
) -> Result<BTreeMap<String, BTreeMap<String, PcbPad>>> {
    let net_names = parse_net_names(root_list)?;
    let mut pads = BTreeMap::<String, BTreeMap<String, PcbPad>>::new();
    for footprint in list_children(root_list, "footprint") {
        let reference = footprint_reference(footprint)
            .with_context(|| "KiCad PCB footprint is missing Reference property or fp_text.")?;
        let footprint_at = footprint_at(footprint, &reference)?;
        for pad in list_children(footprint, "pad") {
            let pad_name = string_at(pad, 1)
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .with_context(|| {
                    format!("KiCad PCB footprint {reference} has a pad with missing pad name.")
                })?
                .to_string();
            let Some(net_name) = pad_net_name(pad, &net_names, path)? else {
                continue;
            };
            let local_at = child_list(pad, "at").with_context(|| {
                format!("KiCad PCB footprint {reference} pad {pad_name} is missing (at x y).")
            })?;
            let local_x_mm = numeric_at(local_at, 1).with_context(|| {
                format!("KiCad PCB footprint {reference} pad {pad_name} has invalid x coordinate.")
            })?;
            let local_y_mm = numeric_at(local_at, 2).with_context(|| {
                format!("KiCad PCB footprint {reference} pad {pad_name} has invalid y coordinate.")
            })?;
            let at = transform_footprint_point(footprint_at, local_x_mm, local_y_mm);
            let layers = pad_layers(pad);
            let kind = string_at(pad, 2)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            let shape = string_at(pad, 3)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            let size = pad_size(pad).with_context(|| {
                format!("KiCad PCB footprint {reference} pad {pad_name} has invalid pad size.")
            })?;
            let rotation_deg = pad_rotation_deg(footprint_at, local_at);
            let drill_mm = pad_drill_mm(pad).with_context(|| {
                format!("KiCad PCB footprint {reference} pad {pad_name} has invalid drill size.")
            })?;
            let component_pads = pads.entry(reference.clone()).or_default();
            if component_pads.contains_key(&pad_name) {
                bail!("KiCad PCB footprint {reference} contains duplicate pad {pad_name}.");
            }
            component_pads.insert(
                pad_name,
                PcbPad {
                    at,
                    net_name,
                    layers,
                    kind,
                    shape,
                    size,
                    rotation_deg,
                    drill_mm,
                },
            );
        }
    }
    Ok(pads)
}

fn parse_placements(root_list: &[Sexp], path: &Path) -> Result<BTreeMap<String, PcbPlacement>> {
    let mut placements = BTreeMap::new();
    for footprint in list_children(root_list, "footprint") {
        let reference = footprint_reference(footprint)
            .with_context(|| "KiCad PCB footprint is missing Reference property or fp_text.")?;
        if placements.contains_key(&reference) {
            bail!("KiCad PCB contains duplicate footprint reference {reference}.");
        }
        let at = footprint_at(footprint, &reference)?;
        let side = footprint_side(footprint);
        placements.insert(
            reference,
            PcbPlacement {
                x_mm: at.x_mm,
                y_mm: at.y_mm,
                side,
                rotation_deg: Some(at.rotation_deg),
            },
        );
    }
    if placements.is_empty() {
        bail!("KiCad PCB {} contains no footprints.", path.display());
    }
    Ok(placements)
}

fn parse_footprints(root_list: &[Sexp], path: &Path) -> Result<BTreeMap<String, PcbFootprint>> {
    let mut footprints = BTreeMap::new();
    for footprint in list_children(root_list, "footprint") {
        let reference = footprint_reference(footprint)
            .with_context(|| "KiCad PCB footprint is missing Reference property or fp_text.")?;
        let footprint_at = footprint_at(footprint, &reference)?;
        let mut evidence = PcbFootprint::default();
        for line in list_children(footprint, "fp_line") {
            let start = transformed_child_point(line, "start", footprint_at, path)?;
            let end = transformed_child_point(line, "end", footprint_at, path)?;
            if (end.x_mm - start.x_mm).hypot(end.y_mm - start.y_mm) <= f64::EPSILON {
                bail!(
                    "KiCad PCB footprint {reference} fp_line in {} has zero length.",
                    path.display()
                );
            }
            let layer = non_empty_child_string(line, "layer", path)?;
            evidence.segments.push(PcbFootprintSegment {
                start,
                end,
                kind: footprint_graphic_kind(&layer).to_string(),
                layer,
            });
        }
        for rect in list_children(footprint, "fp_rect") {
            let start = transformed_child_point(rect, "start", footprint_at, path)?;
            let end = transformed_child_point(rect, "end", footprint_at, path)?;
            if (end.x_mm - start.x_mm).abs() <= f64::EPSILON
                || (end.y_mm - start.y_mm).abs() <= f64::EPSILON
            {
                bail!(
                    "KiCad PCB footprint {reference} fp_rect in {} has zero area.",
                    path.display()
                );
            }
            let layer = non_empty_child_string(rect, "layer", path)?;
            evidence.rectangles.push(PcbFootprintRectangle {
                start,
                end,
                kind: footprint_graphic_kind(&layer).to_string(),
                layer,
            });
        }
        if !evidence.segments.is_empty() || !evidence.rectangles.is_empty() {
            footprints.insert(reference, evidence);
        }
    }
    Ok(footprints)
}

fn parse_routes(root_list: &[Sexp], path: &Path) -> Result<BTreeMap<String, PcbRoute>> {
    let net_names = parse_net_names(root_list)?;
    let mut routes = BTreeMap::<String, PcbRoute>::new();
    for segment in list_children(root_list, "segment") {
        let net_name = route_net_name(segment, &net_names, "segment", path)?;
        let start = route_point(segment, "start", path)?;
        let end = route_point(segment, "end", path)?;
        let width_mm = positive_child_number(segment, "width", path)?;
        let layer = non_empty_child_string(segment, "layer", path)?;
        routes
            .entry(net_name)
            .or_default()
            .segments
            .push(PcbRouteSegment {
                start,
                end,
                width_mm,
                layer,
            });
    }
    for via in list_children(root_list, "via") {
        let net_name = route_net_name(via, &net_names, "via", path)?;
        let at = route_point(via, "at", path)?;
        let size_mm = positive_child_number(via, "size", path)?;
        let drill_mm = positive_child_number(via, "drill", path)?;
        let layers = child_list(via, "layers")
            .map(|layers| {
                layers
                    .iter()
                    .skip(1)
                    .filter_map(|item| match item {
                        Sexp::Atom(value) | Sexp::Str(value) if !value.trim().is_empty() => {
                            Some(value.trim().to_string())
                        }
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();
        routes.entry(net_name).or_default().vias.push(PcbRouteVia {
            at,
            size_mm,
            drill_mm,
            layers,
        });
    }
    Ok(routes)
}

fn parse_outline(root_list: &[Sexp], path: &Path) -> Result<PcbOutline> {
    let mut segments = Vec::new();
    for line in list_children(root_list, "gr_line") {
        let layer = non_empty_child_string(line, "layer", path)?;
        if layer != "Edge.Cuts" {
            continue;
        }
        let start = route_point(line, "start", path)?;
        let end = route_point(line, "end", path)?;
        let length_mm = (end.x_mm - start.x_mm).hypot(end.y_mm - start.y_mm);
        if length_mm <= f64::EPSILON {
            bail!(
                "KiCad PCB Edge.Cuts gr_line in {} has zero length.",
                path.display()
            );
        }
        segments.push(PcbOutlineSegment { start, end, layer });
    }
    Ok(PcbOutline { segments })
}

fn parse_zones(root_list: &[Sexp], path: &Path) -> Result<BTreeMap<String, Vec<PcbZone>>> {
    let net_names = parse_net_names(root_list)?;
    let mut zones = BTreeMap::<String, Vec<PcbZone>>::new();
    for zone in list_children(root_list, "zone") {
        let Some(net_name) = zone_net_name(zone, &net_names, path)? else {
            continue;
        };
        let layers = zone_layers(zone, path)?;
        let polygon = zone_polygon(zone, path)?;
        let filled_polygons_by_layer = zone_filled_polygons_by_layer(zone, path)?;
        for layer in layers {
            let filled_polygons = filled_polygons_by_layer
                .get(&layer)
                .cloned()
                .unwrap_or_default();
            zones.entry(net_name.clone()).or_default().push(PcbZone {
                layer,
                polygon: polygon.clone(),
                filled_polygons,
            });
        }
    }
    Ok(zones)
}

fn parse_net_rules(root_list: &[Sexp], path: &Path) -> Result<BTreeMap<String, PcbNetRule>> {
    let mut class_rules = BTreeMap::<String, PcbNetRule>::new();
    let mut net_classes = BTreeMap::<String, Vec<String>>::new();
    for net_class in all_lists_by_tag(root_list, "net_class") {
        let Some(class_name) = string_at(net_class, 1).map(str::trim) else {
            continue;
        };
        if class_name.is_empty() {
            continue;
        }
        let mut rule = PcbNetRule {
            net_class: Some(class_name.to_string()),
            track_width_mm: first_positive_child_length_mm(
                net_class,
                &["trace_width", "track_width"],
                path,
            )?,
            diff_pair_width_mm: first_positive_child_length_mm(
                net_class,
                &["diff_pair_width"],
                path,
            )?,
            diff_pair_gap_mm: first_positive_child_length_mm(net_class, &["diff_pair_gap"], path)?,
            length_max_mm: None,
            skew_max_mm: None,
        };
        let nets = list_children(net_class, "add_net")
            .filter_map(|item| string_at(item, 1).map(str::trim))
            .filter(|net| !net.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        if nets.is_empty() {
            continue;
        }
        for net in &nets {
            net_classes
                .entry(class_name.to_string())
                .or_default()
                .push(net.clone());
        }
        class_rules.insert(class_name.to_string(), rule.clone());
        rule.net_class = None;
    }

    let mut net_rules = BTreeMap::<String, PcbNetRule>::new();
    for (class_name, nets) in &net_classes {
        let Some(class_rule) = class_rules.get(class_name) else {
            continue;
        };
        for net in nets {
            merge_net_rule(net_rules.entry(net.clone()).or_default(), class_rule);
        }
    }

    for custom_rule in all_lists_by_tag(root_list, "rule") {
        let Some(condition) =
            child_list(custom_rule, "condition").and_then(|condition| string_at(condition, 1))
        else {
            continue;
        };
        let mut rule_update = PcbNetRule::default();
        for constraint in list_children(custom_rule, "constraint") {
            match string_at(constraint, 1) {
                Some("length") => {
                    rule_update.length_max_mm =
                        positive_constraint_bound_mm(constraint, "max", path)?;
                }
                Some("skew") => {
                    rule_update.skew_max_mm =
                        nonnegative_constraint_bound_mm(constraint, "max", path)?;
                }
                _ => {}
            }
        }
        if rule_update.length_max_mm.is_none() && rule_update.skew_max_mm.is_none() {
            continue;
        }
        for class_name in condition_net_classes(condition) {
            if let Some(nets) = net_classes.get(&class_name) {
                for net in nets {
                    merge_net_rule(net_rules.entry(net.clone()).or_default(), &rule_update);
                }
            }
        }
        for net in condition_net_names(condition) {
            merge_net_rule(net_rules.entry(net).or_default(), &rule_update);
        }
    }

    Ok(net_rules)
}

fn all_lists_by_tag<'a>(list: &'a [Sexp], wanted: &'a str) -> Vec<&'a [Sexp]> {
    let mut matches = Vec::new();
    collect_lists_by_tag(list, wanted, &mut matches);
    matches
}

fn collect_lists_by_tag<'a>(list: &'a [Sexp], wanted: &str, matches: &mut Vec<&'a [Sexp]>) {
    if tag(list) == Some(wanted) {
        matches.push(list);
    }
    for item in list.iter().skip(1) {
        if let Some(child) = as_list(item) {
            collect_lists_by_tag(child, wanted, matches);
        }
    }
}

fn first_positive_child_length_mm(
    list: &[Sexp],
    names: &[&str],
    path: &Path,
) -> Result<Option<f64>> {
    for name in names {
        if let Some(child) = child_list(list, name) {
            let value = length_at_mm(child, 1).with_context(|| {
                format!(
                    "KiCad PCB {} entry in {} has invalid {} value.",
                    tag(list).unwrap_or("constraint"),
                    path.display(),
                    name
                )
            })?;
            if value <= 0.0 {
                bail!(
                    "KiCad PCB {} entry in {} has non-positive {} {}.",
                    tag(list).unwrap_or("constraint"),
                    path.display(),
                    name,
                    value
                );
            }
            return Ok(Some(value));
        }
    }
    Ok(None)
}

fn positive_constraint_bound_mm(
    constraint: &[Sexp],
    name: &str,
    path: &Path,
) -> Result<Option<f64>> {
    let Some(value) = constraint_bound_mm(constraint, name, path)? else {
        return Ok(None);
    };
    if value <= 0.0 {
        bail!(
            "KiCad PCB custom rule in {} has non-positive {} bound {}.",
            path.display(),
            name,
            value
        );
    }
    Ok(Some(value))
}

fn nonnegative_constraint_bound_mm(
    constraint: &[Sexp],
    name: &str,
    path: &Path,
) -> Result<Option<f64>> {
    let Some(value) = constraint_bound_mm(constraint, name, path)? else {
        return Ok(None);
    };
    if value < 0.0 {
        bail!(
            "KiCad PCB custom rule in {} has negative {} bound {}.",
            path.display(),
            name,
            value
        );
    }
    Ok(Some(value))
}

fn constraint_bound_mm(constraint: &[Sexp], name: &str, path: &Path) -> Result<Option<f64>> {
    let Some(bound) = child_list(constraint, name) else {
        return Ok(None);
    };
    length_at_mm(bound, 1)
        .with_context(|| {
            format!(
                "KiCad PCB custom rule in {} has invalid {name} bound.",
                path.display()
            )
        })
        .map(Some)
}

fn length_at_mm(list: &[Sexp], index: usize) -> Option<f64> {
    let value = string_at(list, index)?.trim();
    let (number, scale) = if let Some(number) = value.strip_suffix("mm") {
        (number, 1.0)
    } else if let Some(number) = value.strip_suffix("mil") {
        (number, 0.0254)
    } else if let Some(number) = value.strip_suffix("in") {
        (number, 25.4)
    } else {
        (value, 1.0)
    };
    let parsed = number.trim().parse::<f64>().ok()? * scale;
    parsed.is_finite().then_some(parsed)
}

fn condition_net_classes(condition: &str) -> Vec<String> {
    quoted_condition_values(condition, "hasNetclass")
        .into_iter()
        .chain(quoted_equality_values(condition, "NetClass"))
        .collect()
}

fn condition_net_names(condition: &str) -> Vec<String> {
    quoted_equality_values(condition, "NetName")
}

fn quoted_condition_values(condition: &str, function_name: &str) -> Vec<String> {
    let mut values = Vec::new();
    let needle = format!("{function_name}(");
    let mut rest = condition;
    while let Some(start) = rest.find(&needle) {
        rest = &rest[start + needle.len()..];
        if let Some(value) = leading_quoted_value(rest) {
            values.push(value);
        }
    }
    values
}

fn quoted_equality_values(condition: &str, property: &str) -> Vec<String> {
    let mut values = Vec::new();
    for operator in ["==", "="] {
        let mut rest = condition;
        while let Some(property_start) = rest.find(property) {
            rest = &rest[property_start + property.len()..];
            let trimmed = rest.trim_start();
            let Some(after_operator) = trimmed.strip_prefix(operator) else {
                continue;
            };
            if let Some(value) = leading_quoted_value(after_operator.trim_start()) {
                values.push(value);
            }
            rest = after_operator;
        }
    }
    values
}

fn leading_quoted_value(input: &str) -> Option<String> {
    let quote = input
        .chars()
        .find(|character| *character == '\'' || *character == '"')?;
    let start = input.find(quote)? + quote.len_utf8();
    let tail = &input[start..];
    let end = tail.find(quote)?;
    let value = tail[..end].trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn merge_net_rule(target: &mut PcbNetRule, update: &PcbNetRule) {
    if target.net_class.is_none() {
        target.net_class = update.net_class.clone();
    }
    target.track_width_mm = target.track_width_mm.or(update.track_width_mm);
    target.diff_pair_width_mm = target.diff_pair_width_mm.or(update.diff_pair_width_mm);
    target.diff_pair_gap_mm = target.diff_pair_gap_mm.or(update.diff_pair_gap_mm);
    target.length_max_mm = min_optional(target.length_max_mm, update.length_max_mm);
    target.skew_max_mm = min_optional(target.skew_max_mm, update.skew_max_mm);
}

fn min_optional(current: Option<f64>, update: Option<f64>) -> Option<f64> {
    match (current, update) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn parse_net_names(root_list: &[Sexp]) -> Result<BTreeMap<String, String>> {
    let mut net_names = BTreeMap::new();
    for net in list_children(root_list, "net") {
        let net_id = string_at(net, 1).context("KiCad PCB net entry is missing net id.")?;
        let net_name = string_at(net, 2).context("KiCad PCB net entry is missing net name.")?;
        if net_name.trim().is_empty() {
            bail!("KiCad PCB net {net_id} has empty net name.");
        }
        net_names.insert(net_id.to_string(), net_name.trim().to_string());
    }
    Ok(net_names)
}

fn route_net_name(
    item: &[Sexp],
    net_names: &BTreeMap<String, String>,
    item_kind: &str,
    path: &Path,
) -> Result<String> {
    let net = child_list(item, "net").with_context(|| {
        format!(
            "KiCad PCB {} in {} is missing net id.",
            item_kind,
            path.display()
        )
    })?;
    let net_id = string_at(net, 1).with_context(|| {
        format!(
            "KiCad PCB {} in {} has invalid net id.",
            item_kind,
            path.display()
        )
    })?;
    net_names.get(net_id).cloned().with_context(|| {
        format!(
            "KiCad PCB {} in {} references unknown net id {}.",
            item_kind,
            path.display(),
            net_id
        )
    })
}

fn zone_net_name(
    zone: &[Sexp],
    net_names: &BTreeMap<String, String>,
    path: &Path,
) -> Result<Option<String>> {
    if let Some(net) = child_list(zone, "net") {
        let net_id = string_at(net, 1)
            .with_context(|| format!("KiCad PCB zone in {} has invalid net id.", path.display()))?;
        return net_names.get(net_id).cloned().map(Some).with_context(|| {
            format!(
                "KiCad PCB zone in {} references unknown net id {}.",
                path.display(),
                net_id
            )
        });
    }
    let Some(net_name) = child_list(zone, "net_name").and_then(|net_name| string_at(net_name, 1))
    else {
        return Ok(None);
    };
    let net_name = net_name.trim();
    if net_name.is_empty() {
        return Ok(None);
    }
    Ok(Some(net_name.to_string()))
}

fn zone_layers(zone: &[Sexp], path: &Path) -> Result<Vec<String>> {
    if let Some(layer) = child_list(zone, "layer") {
        let value = string_at(layer, 1)
            .with_context(|| format!("KiCad PCB zone in {} has invalid layer.", path.display()))?;
        let value = value.trim();
        if value.is_empty() {
            bail!("KiCad PCB zone in {} has empty layer.", path.display());
        }
        return Ok(vec![value.to_string()]);
    }
    if let Some(layers) = child_list(zone, "layers") {
        let values = layers
            .iter()
            .skip(1)
            .filter_map(|item| match item {
                Sexp::Atom(value) | Sexp::Str(value) if !value.trim().is_empty() => {
                    Some(value.trim().to_string())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        if values.is_empty() {
            bail!("KiCad PCB zone in {} has empty layers.", path.display());
        }
        return Ok(values);
    }
    bail!(
        "KiCad PCB zone in {} is missing layer evidence.",
        path.display()
    )
}

fn zone_polygon(zone: &[Sexp], path: &Path) -> Result<Vec<PcbPoint>> {
    let polygon = child_list(zone, "polygon")
        .with_context(|| format!("KiCad PCB zone in {} is missing polygon.", path.display()))?;
    let pts = child_list(polygon, "pts").with_context(|| {
        format!(
            "KiCad PCB zone polygon in {} is missing pts list.",
            path.display()
        )
    })?;
    coordinate_points(pts, "zone polygon", path)
}

fn zone_filled_polygons_by_layer(
    zone: &[Sexp],
    path: &Path,
) -> Result<BTreeMap<String, Vec<Vec<PcbPoint>>>> {
    let mut filled_polygons = BTreeMap::<String, Vec<Vec<PcbPoint>>>::new();
    for filled_polygon in list_children(zone, "filled_polygon") {
        let layer = child_list(filled_polygon, "layer").with_context(|| {
            format!(
                "KiCad PCB zone filled_polygon in {} is missing layer.",
                path.display()
            )
        })?;
        let layer = string_at(layer, 1).with_context(|| {
            format!(
                "KiCad PCB zone filled_polygon in {} has invalid layer.",
                path.display()
            )
        })?;
        let layer = layer.trim();
        if layer.is_empty() {
            bail!(
                "KiCad PCB zone filled_polygon in {} has empty layer.",
                path.display()
            );
        }
        let pts = child_list(filled_polygon, "pts").with_context(|| {
            format!(
                "KiCad PCB zone filled_polygon in {} is missing pts list.",
                path.display()
            )
        })?;
        filled_polygons
            .entry(layer.to_string())
            .or_default()
            .push(coordinate_points(pts, "zone filled_polygon", path)?);
    }
    Ok(filled_polygons)
}

fn coordinate_points(pts: &[Sexp], item_kind: &str, path: &Path) -> Result<Vec<PcbPoint>> {
    let mut points = Vec::new();
    for xy in list_children(pts, "xy") {
        let x_mm = numeric_at(xy, 1).with_context(|| {
            format!(
                "KiCad PCB {item_kind} in {} has invalid x coordinate.",
                path.display(),
            )
        })?;
        let y_mm = numeric_at(xy, 2).with_context(|| {
            format!(
                "KiCad PCB {item_kind} in {} has invalid y coordinate.",
                path.display(),
            )
        })?;
        points.push(PcbPoint { x_mm, y_mm });
    }
    if points.len() < 3 {
        bail!(
            "KiCad PCB {item_kind} in {} has fewer than three points.",
            path.display(),
        );
    }
    Ok(points)
}

fn route_point(item: &[Sexp], field: &str, path: &Path) -> Result<PcbPoint> {
    let point = child_list(item, field).with_context(|| {
        format!(
            "KiCad PCB route item in {} is missing ({field} x y).",
            path.display()
        )
    })?;
    let x_mm = numeric_at(point, 1).with_context(|| {
        format!(
            "KiCad PCB route item in {} has invalid {field} x coordinate.",
            path.display()
        )
    })?;
    let y_mm = numeric_at(point, 2).with_context(|| {
        format!(
            "KiCad PCB route item in {} has invalid {field} y coordinate.",
            path.display()
        )
    })?;
    Ok(PcbPoint { x_mm, y_mm })
}

fn footprint_at(footprint: &[Sexp], reference: &str) -> Result<FootprintAt> {
    let at = child_list(footprint, "at")
        .with_context(|| format!("KiCad PCB footprint {reference} is missing (at x y)."))?;
    let x_mm = numeric_at(at, 1)
        .with_context(|| format!("KiCad PCB footprint {reference} has invalid x placement."))?;
    let y_mm = numeric_at(at, 2)
        .with_context(|| format!("KiCad PCB footprint {reference} has invalid y placement."))?;
    let rotation_deg = numeric_at(at, 3).unwrap_or(0.0);
    Ok(FootprintAt {
        x_mm,
        y_mm,
        rotation_deg,
    })
}

fn transform_footprint_point(
    footprint_at: FootprintAt,
    local_x_mm: f64,
    local_y_mm: f64,
) -> PcbPoint {
    let radians = footprint_at.rotation_deg.to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    PcbPoint {
        x_mm: footprint_at.x_mm + local_x_mm * cos - local_y_mm * sin,
        y_mm: footprint_at.y_mm + local_x_mm * sin + local_y_mm * cos,
    }
}

fn transformed_child_point(
    item: &[Sexp],
    field: &str,
    footprint_at: FootprintAt,
    path: &Path,
) -> Result<PcbPoint> {
    let point = child_list(item, field).with_context(|| {
        format!(
            "KiCad PCB footprint graphic item in {} is missing ({field} x y).",
            path.display()
        )
    })?;
    let x_mm = numeric_at(point, 1).with_context(|| {
        format!(
            "KiCad PCB footprint graphic item in {} has invalid {field} x coordinate.",
            path.display()
        )
    })?;
    let y_mm = numeric_at(point, 2).with_context(|| {
        format!(
            "KiCad PCB footprint graphic item in {} has invalid {field} y coordinate.",
            path.display()
        )
    })?;
    Ok(transform_footprint_point(footprint_at, x_mm, y_mm))
}

fn footprint_graphic_kind(layer: &str) -> &'static str {
    if layer.ends_with(".Fab") {
        "fabrication"
    } else if layer.ends_with(".CrtYd") {
        "courtyard"
    } else if layer.ends_with(".SilkS") {
        "silkscreen"
    } else {
        "other"
    }
}

fn pad_net_name(
    pad: &[Sexp],
    net_names: &BTreeMap<String, String>,
    path: &Path,
) -> Result<Option<String>> {
    let Some(net) = child_list(pad, "net") else {
        return Ok(None);
    };
    if let Some(name) = string_at(net, 2)
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        return Ok(Some(name.to_string()));
    }
    let net_id = string_at(net, 1)
        .with_context(|| format!("KiCad PCB pad in {} has invalid net id.", path.display()))?;
    if net_id == "0" {
        return Ok(None);
    }
    net_names.get(net_id).cloned().map(Some).with_context(|| {
        format!(
            "KiCad PCB pad in {} references unknown net id {net_id}.",
            path.display()
        )
    })
}

fn pad_layers(pad: &[Sexp]) -> Vec<String> {
    child_list(pad, "layers")
        .map(|layers| {
            layers
                .iter()
                .skip(1)
                .filter_map(|item| match item {
                    Sexp::Atom(value) | Sexp::Str(value) if !value.trim().is_empty() => {
                        Some(value.trim().to_string())
                    }
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

fn pad_size(pad: &[Sexp]) -> Result<PcbPadSize> {
    let size = child_list(pad, "size").context("missing (size x y)")?;
    let x_mm = numeric_at(size, 1).context("invalid x size")?;
    let y_mm = numeric_at(size, 2).context("invalid y size")?;
    if x_mm <= 0.0 || y_mm <= 0.0 {
        bail!("pad size must be positive");
    }
    Ok(PcbPadSize { x_mm, y_mm })
}

fn pad_rotation_deg(footprint_at: FootprintAt, local_at: &[Sexp]) -> Option<f64> {
    let rotation_deg = footprint_at.rotation_deg + numeric_at(local_at, 3).unwrap_or(0.0);
    (rotation_deg.abs() > 1.0e-9).then_some(rotation_deg)
}

fn pad_drill_mm(pad: &[Sexp]) -> Result<Option<f64>> {
    let Some(drill) = child_list(pad, "drill") else {
        return Ok(None);
    };
    let Some(value) = numeric_at(drill, 1) else {
        return Ok(None);
    };
    if value <= 0.0 {
        bail!("pad drill must be positive");
    }
    Ok(Some(value))
}

fn positive_child_number(item: &[Sexp], field: &str, path: &Path) -> Result<f64> {
    let child = child_list(item, field).with_context(|| {
        format!(
            "KiCad PCB route item in {} is missing ({field} value).",
            path.display()
        )
    })?;
    let value = numeric_at(child, 1).with_context(|| {
        format!(
            "KiCad PCB route item in {} has invalid {field}.",
            path.display()
        )
    })?;
    if value <= 0.0 {
        bail!(
            "KiCad PCB route item in {} has non-positive {field} {}.",
            path.display(),
            value
        );
    }
    Ok(value)
}

fn non_empty_child_string(item: &[Sexp], field: &str, path: &Path) -> Result<String> {
    let child = child_list(item, field).with_context(|| {
        format!(
            "KiCad PCB route item in {} is missing ({field} value).",
            path.display()
        )
    })?;
    let value = string_at(child, 1).with_context(|| {
        format!(
            "KiCad PCB route item in {} has invalid {field}.",
            path.display()
        )
    })?;
    let value = value.trim();
    if value.is_empty() {
        bail!(
            "KiCad PCB route item in {} has empty {field}.",
            path.display()
        );
    }
    Ok(value.to_string())
}

fn footprint_reference(footprint: &[super::kicad_sch::sexp::Sexp]) -> Option<String> {
    for property in list_children(footprint, "property") {
        if string_at(property, 1) == Some("Reference") {
            let reference = string_at(property, 2)?.trim();
            if !reference.is_empty() {
                return Some(reference.to_string());
            }
        }
    }
    for fp_text in list_children(footprint, "fp_text") {
        if string_at(fp_text, 1) == Some("reference") {
            let reference = string_at(fp_text, 2)?.trim();
            if !reference.is_empty() {
                return Some(reference.to_string());
            }
        }
    }
    None
}

fn footprint_side(footprint: &[super::kicad_sch::sexp::Sexp]) -> Option<PcbPlacementSide> {
    let layer = child_list(footprint, "layer").and_then(|layer| string_at(layer, 1))?;
    if layer.starts_with("F.") {
        Some(PcbPlacementSide::Top)
    } else if layer.starts_with("B.") {
        Some(PcbPlacementSide::Bottom)
    } else {
        None
    }
}

fn merge_pcb_into_project(
    project_yaml: &mut Value,
    parsed_pcb: &ParsedPcb,
) -> Result<KicadPcbImportSummary> {
    let board = mapping_field_mut(project_yaml, "board")?;
    let component_refs = mapping_field(board, "components")?
        .keys()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    if component_refs.is_empty() {
        bail!("Board IR project has no board.components entries.");
    }
    let board_nets = mapping_field(board, "nets")?
        .keys()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    if board_nets.is_empty() {
        bail!("Board IR project has no board.nets entries.");
    }
    let layout = ensure_mapping_field_mut(board, "layout")?;
    let placement_yaml = ensure_mapping_field_mut(layout, "placements")?;
    let mut summary = KicadPcbImportSummary::default();
    for (reference, placement) in &parsed_pcb.placements {
        if !component_refs.contains(reference) {
            continue;
        }
        placement_yaml.insert(
            Value::String(reference.clone()),
            serde_yaml_ng::to_value(PlacementYaml {
                x_mm: placement.x_mm,
                y_mm: placement.y_mm,
                side: placement.side,
                rotation_deg: placement.rotation_deg,
            })?,
        );
        summary.placements += 1;
    }
    let footprint_yaml = ensure_mapping_field_mut(layout, "footprints")?;
    for (reference, footprint) in &parsed_pcb.footprints {
        if !component_refs.contains(reference) {
            continue;
        }
        let footprint_value = footprint_yaml_value(footprint)?;
        footprint_yaml.insert(Value::String(reference.clone()), footprint_value);
        summary.footprint_graphics += footprint.segments.len() + footprint.rectangles.len();
    }
    let pad_yaml = ensure_mapping_field_mut(layout, "pads")?;
    for (reference, pads) in &parsed_pcb.pads {
        if !component_refs.contains(reference) {
            continue;
        }
        let mut component_pad_yaml = Mapping::new();
        for (pad_name, pad) in pads {
            let Some(board_net_name) = map_pcb_net_to_board_net(&pad.net_name, &board_nets)? else {
                continue;
            };
            component_pad_yaml.insert(
                Value::String(pad_name.clone()),
                pad_yaml_value(pad, &board_net_name)?,
            );
        }
        if component_pad_yaml.is_empty() {
            continue;
        }
        summary.pads += component_pad_yaml.len();
        pad_yaml.insert(
            Value::String(reference.clone()),
            Value::Mapping(component_pad_yaml),
        );
    }
    if !parsed_pcb.outline.segments.is_empty() {
        layout.insert(
            Value::String("outline".to_string()),
            outline_yaml_value(&parsed_pcb.outline)?,
        );
        summary.outline_segments = parsed_pcb.outline.segments.len();
    }
    let route_yaml = ensure_mapping_field_mut(layout, "routes")?;
    for (pcb_net_name, route) in &parsed_pcb.routes {
        let Some(board_net_name) = map_pcb_net_to_board_net(pcb_net_name, &board_nets)? else {
            continue;
        };
        let route_value = route_yaml_value(route)?;
        route_yaml.insert(Value::String(board_net_name), route_value);
        summary.route_segments += route.segments.len();
        summary.route_vias += route.vias.len();
    }
    let zone_yaml = ensure_mapping_field_mut(layout, "zones")?;
    for (pcb_net_name, zones) in &parsed_pcb.zones {
        let Some(board_net_name) = map_pcb_net_to_board_net(pcb_net_name, &board_nets)? else {
            continue;
        };
        zone_yaml.insert(Value::String(board_net_name), zone_yaml_value(zones)?);
        summary.zones += zones.len();
    }
    let constraints = ensure_mapping_field_mut(layout, "constraints")?;
    let net_rules_yaml = ensure_mapping_field_mut(constraints, "net_rules")?;
    for (pcb_net_name, rule) in &parsed_pcb.net_rules {
        let Some(board_net_name) = map_pcb_net_to_board_net(pcb_net_name, &board_nets)? else {
            continue;
        };
        net_rules_yaml.insert(Value::String(board_net_name), net_rule_yaml_value(rule)?);
        summary.routing_constraints += 1;
    }
    Ok(summary)
}

fn outline_yaml_value(outline: &PcbOutline) -> Result<Value> {
    serde_yaml_ng::to_value(OutlineYaml {
        segments: outline
            .segments
            .iter()
            .map(|segment| OutlineSegmentYaml {
                start: segment.start,
                end: segment.end,
                layer: segment.layer.clone(),
            })
            .collect(),
    })
    .context("Failed to serialize KiCad PCB board outline evidence into Board IR YAML.")
}

fn footprint_yaml_value(footprint: &PcbFootprint) -> Result<Value> {
    serde_yaml_ng::to_value(FootprintYaml {
        segments: footprint
            .segments
            .iter()
            .map(|segment| FootprintSegmentYaml {
                start: segment.start,
                end: segment.end,
                layer: segment.layer.clone(),
                kind: segment.kind.clone(),
            })
            .collect(),
        rectangles: footprint
            .rectangles
            .iter()
            .map(|rectangle| FootprintRectangleYaml {
                start: rectangle.start,
                end: rectangle.end,
                layer: rectangle.layer.clone(),
                kind: rectangle.kind.clone(),
            })
            .collect(),
    })
    .context("Failed to serialize KiCad PCB footprint drawing evidence into Board IR YAML.")
}

fn route_yaml_value(route: &PcbRoute) -> Result<Value> {
    serde_yaml_ng::to_value(RouteYaml {
        segments: route
            .segments
            .iter()
            .map(|segment| RouteSegmentYaml {
                start: segment.start,
                end: segment.end,
                width_mm: segment.width_mm,
                layer: segment.layer.clone(),
            })
            .collect(),
        vias: route
            .vias
            .iter()
            .map(|via| RouteViaYaml {
                at: via.at,
                size_mm: via.size_mm,
                drill_mm: via.drill_mm,
                layers: via.layers.iter().map(String::as_str).collect(),
            })
            .collect(),
    })
    .context("Failed to serialize KiCad PCB route geometry into Board IR YAML.")
}

fn pad_yaml_value(pad: &PcbPad, board_net_name: &str) -> Result<Value> {
    serde_yaml_ng::to_value(PadYaml {
        at: pad.at,
        net: board_net_name,
        layers: pad.layers.iter().map(String::as_str).collect(),
        kind: pad.kind.as_deref(),
        shape: pad.shape.as_deref(),
        size: pad.size,
        rotation_deg: pad.rotation_deg,
        drill_mm: pad.drill_mm,
    })
    .context("Failed to serialize KiCad PCB pad evidence into Board IR YAML.")
}

fn net_rule_yaml_value(rule: &PcbNetRule) -> Result<Value> {
    serde_yaml_ng::to_value(NetRuleYaml {
        net_class: rule.net_class.as_deref(),
        track_width_mm: rule.track_width_mm,
        diff_pair_width_mm: rule.diff_pair_width_mm,
        diff_pair_gap_mm: rule.diff_pair_gap_mm,
        length_max_mm: rule.length_max_mm,
        skew_max_mm: rule.skew_max_mm,
    })
    .context("Failed to serialize KiCad PCB route constraints into Board IR YAML.")
}

fn zone_yaml_value(zones: &[PcbZone]) -> Result<Value> {
    serde_yaml_ng::to_value(
        zones
            .iter()
            .map(|zone| ZoneYaml {
                layer: zone.layer.clone(),
                polygon: zone.polygon.clone(),
                filled_polygons: zone.filled_polygons.clone(),
            })
            .collect::<Vec<_>>(),
    )
    .context("Failed to serialize KiCad PCB copper zones into Board IR YAML.")
}

fn map_pcb_net_to_board_net(
    pcb_net_name: &str,
    board_nets: &BTreeSet<String>,
) -> Result<Option<String>> {
    if board_nets.contains(pcb_net_name) {
        return Ok(Some(pcb_net_name.to_string()));
    }
    let lowercase = pcb_net_name.to_ascii_lowercase();
    if board_nets.contains(&lowercase) {
        return Ok(Some(lowercase));
    }
    if is_ground_net_name(pcb_net_name) {
        for candidate in ["gnd", "net_gnd"] {
            if board_nets.contains(candidate) {
                return Ok(Some(candidate.to_string()));
            }
        }
    }
    let sanitized = sanitize_identifier(pcb_net_name);
    let prefixed = format!("net_{sanitized}");
    if board_nets.contains(&prefixed) {
        return Ok(Some(prefixed));
    }
    if board_nets.contains(&sanitized) {
        return Ok(Some(sanitized));
    }
    let matches = board_nets
        .iter()
        .filter(|candidate| board_net_matches_pcb_net(candidate, &sanitized))
        .cloned()
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Ok(None),
        [single] => Ok(Some(single.clone())),
        ambiguous => bail!(
            "KiCad PCB net {} maps ambiguously to Board IR nets: {}.",
            pcb_net_name,
            ambiguous.join(", ")
        ),
    }
}

fn board_net_matches_pcb_net(board_net_name: &str, sanitized_pcb_name: &str) -> bool {
    sanitize_identifier(board_net_name) == sanitized_pcb_name
        || board_net_name
            .strip_prefix("net_")
            .is_some_and(|suffix| sanitize_identifier(suffix) == sanitized_pcb_name)
}

fn is_ground_net_name(name: &str) -> bool {
    matches!(
        sanitize_identifier(name).as_str(),
        "gnd" | "ground" | "vss" | "0"
    )
}

fn sanitize_identifier(input: &str) -> String {
    let mut output = String::new();
    let mut last_was_underscore = false;
    for character in input.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
            last_was_underscore = false;
        } else if !last_was_underscore {
            output.push('_');
            last_was_underscore = true;
        }
    }
    let output = output.trim_matches('_').to_string();
    if output.is_empty() {
        "net".to_string()
    } else {
        output
    }
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
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn mapping_field_mut<'a>(value: &'a mut Value, key: &str) -> Result<&'a mut Mapping> {
    let mapping = value
        .as_mapping_mut()
        .with_context(|| format!("Expected YAML object while reading {key}."))?;
    mapping
        .get_mut(Value::String(key.to_string()))
        .with_context(|| format!("Board IR project is missing {key}."))?
        .as_mapping_mut()
        .with_context(|| format!("Board IR field {key} must be an object."))
}

fn mapping_field<'a>(mapping: &'a Mapping, key: &str) -> Result<&'a Mapping> {
    mapping
        .get(Value::String(key.to_string()))
        .with_context(|| format!("Board IR project is missing board.{key}."))?
        .as_mapping()
        .with_context(|| format!("Board IR field board.{key} must be an object."))
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
