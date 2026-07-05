use super::super::kicad_sch::sexp::{
    Sexp, as_list, child_list, list_children, numeric_at, parse_sexp_document, string_at, tag,
};
use super::footprints::{PcbFootprint, parse_footprints};
use super::outline::{PcbOutline, parse_outline};
use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(super) struct PcbPlacement {
    pub(super) x_mm: f64,
    pub(super) y_mm: f64,
    pub(super) side: Option<PcbPlacementSide>,
    pub(super) rotation_deg: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct FootprintAt {
    pub(super) x_mm: f64,
    pub(super) y_mm: f64,
    pub(super) rotation_deg: f64,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum PcbPlacementSide {
    Top,
    Bottom,
}

#[derive(Debug, Clone, Default)]
pub(super) struct PcbRoute {
    pub(super) segments: Vec<PcbRouteSegment>,
    pub(super) vias: Vec<PcbRouteVia>,
}

#[derive(Debug, Clone)]
pub(super) struct PcbPad {
    pub(super) at: PcbPoint,
    pub(super) net_name: String,
    pub(super) layers: Vec<String>,
    pub(super) kind: Option<String>,
    pub(super) shape: Option<String>,
    pub(super) size: PcbPadSize,
    pub(super) fabrication: Option<PcbPadFabrication>,
    pub(super) rotation_deg: Option<f64>,
    pub(super) drill_mm: Option<f64>,
}

#[derive(Debug, Clone)]
pub(super) struct PcbRouteSegment {
    pub(super) start: PcbPoint,
    pub(super) end: PcbPoint,
    pub(super) width_mm: f64,
    pub(super) layer: String,
}

#[derive(Debug, Clone)]
pub(super) struct PcbRouteVia {
    pub(super) at: PcbPoint,
    pub(super) size_mm: f64,
    pub(super) drill_mm: f64,
    pub(super) layers: Vec<String>,
}

#[derive(Debug, Clone)]
pub(super) struct PcbZone {
    pub(super) layer: String,
    pub(super) polygon: Vec<PcbPoint>,
    pub(super) island_id: String,
    pub(super) filled_polygons: Vec<Vec<PcbPoint>>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct PcbNetRule {
    pub(super) net_class: Option<String>,
    pub(super) track_width_mm: Option<f64>,
    pub(super) diff_pair_width_mm: Option<f64>,
    pub(super) diff_pair_gap_mm: Option<f64>,
    pub(super) length_max_mm: Option<f64>,
    pub(super) skew_max_mm: Option<f64>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub(super) struct PcbPoint {
    pub(super) x_mm: f64,
    pub(super) y_mm: f64,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub(super) struct PcbPadSize {
    pub(super) x_mm: f64,
    pub(super) y_mm: f64,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub(super) struct PcbPadFabrication {
    pub(super) solder_mask_margin_mm: Option<f64>,
    pub(super) solder_paste_margin_mm: Option<f64>,
    pub(super) solder_paste_margin_ratio: Option<f64>,
    pub(super) clearance_mm: Option<f64>,
    pub(super) zone_connect: Option<u8>,
    pub(super) thermal_bridge_width_mm: Option<f64>,
    pub(super) thermal_gap_mm: Option<f64>,
}

#[derive(Debug, Clone)]
pub(super) struct ParsedPcb {
    pub(super) placements: BTreeMap<String, PcbPlacement>,
    pub(super) footprints: BTreeMap<String, PcbFootprint>,
    pub(super) pads: BTreeMap<String, BTreeMap<String, PcbPad>>,
    pub(super) outline: PcbOutline,
    pub(super) routes: BTreeMap<String, PcbRoute>,
    pub(super) zones: BTreeMap<String, Vec<PcbZone>>,
    pub(super) net_rules: BTreeMap<String, PcbNetRule>,
}

pub(super) fn parse_kicad_pcb(path: &PathBuf) -> Result<ParsedPcb> {
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
            let fabrication = pad_fabrication(pad).with_context(|| {
                format!(
                    "KiCad PCB footprint {reference} pad {pad_name} has invalid fabrication overrides."
                )
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
                    fabrication,
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
            let zone_index = zones.get(&net_name).map_or(0, Vec::len);
            zones.entry(net_name.clone()).or_default().push(PcbZone {
                island_id: zone_island_id(&net_name, &layer, zone_index),
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

pub(super) fn coordinate_points(
    pts: &[Sexp],
    item_kind: &str,
    path: &Path,
) -> Result<Vec<PcbPoint>> {
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

pub(super) fn route_point(item: &[Sexp], field: &str, path: &Path) -> Result<PcbPoint> {
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

pub(super) fn footprint_at(footprint: &[Sexp], reference: &str) -> Result<FootprintAt> {
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

pub(super) fn transform_footprint_point(
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

fn pad_fabrication(pad: &[Sexp]) -> Result<Option<PcbPadFabrication>> {
    let fabrication = PcbPadFabrication {
        solder_mask_margin_mm: optional_child_number(pad, "solder_mask_margin")?,
        solder_paste_margin_mm: optional_child_number(pad, "solder_paste_margin")?,
        solder_paste_margin_ratio: optional_child_number(pad, "solder_paste_margin_ratio")?,
        clearance_mm: optional_nonnegative_child_number(pad, "clearance")?,
        zone_connect: optional_child_u8(pad, "zone_connect")?,
        thermal_bridge_width_mm: optional_positive_child_number(pad, "thermal_bridge_width")?,
        thermal_gap_mm: optional_positive_child_number(pad, "thermal_gap")?,
    };
    Ok((fabrication.solder_mask_margin_mm.is_some()
        || fabrication.solder_paste_margin_mm.is_some()
        || fabrication.solder_paste_margin_ratio.is_some()
        || fabrication.clearance_mm.is_some()
        || fabrication.zone_connect.is_some()
        || fabrication.thermal_bridge_width_mm.is_some()
        || fabrication.thermal_gap_mm.is_some())
    .then_some(fabrication))
}

fn optional_child_number(item: &[Sexp], field: &str) -> Result<Option<f64>> {
    let Some(child) = child_list(item, field) else {
        return Ok(None);
    };
    let value = numeric_at(child, 1).with_context(|| format!("invalid {field} value"))?;
    if !value.is_finite() {
        bail!("{field} value must be finite");
    }
    Ok(Some(value))
}

fn optional_nonnegative_child_number(item: &[Sexp], field: &str) -> Result<Option<f64>> {
    let Some(value) = optional_child_number(item, field)? else {
        return Ok(None);
    };
    if value < 0.0 {
        bail!("{field} value must be non-negative");
    }
    Ok(Some(value))
}

fn optional_positive_child_number(item: &[Sexp], field: &str) -> Result<Option<f64>> {
    let Some(value) = optional_child_number(item, field)? else {
        return Ok(None);
    };
    if value <= 0.0 {
        bail!("{field} value must be positive");
    }
    Ok(Some(value))
}

fn optional_child_u8(item: &[Sexp], field: &str) -> Result<Option<u8>> {
    let Some(value) = optional_child_number(item, field)? else {
        return Ok(None);
    };
    if value.fract().abs() > f64::EPSILON || value < 0.0 || value > u8::MAX as f64 {
        bail!("{field} value must be an unsigned integer");
    }
    Ok(Some(value as u8))
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

pub(super) fn non_empty_child_string(item: &[Sexp], field: &str, path: &Path) -> Result<String> {
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

pub(super) fn footprint_reference(footprint: &[Sexp]) -> Option<String> {
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

fn footprint_side(footprint: &[Sexp]) -> Option<PcbPlacementSide> {
    let layer = child_list(footprint, "layer").and_then(|layer| string_at(layer, 1))?;
    if layer.starts_with("F.") {
        Some(PcbPlacementSide::Top)
    } else if layer.starts_with("B.") {
        Some(PcbPlacementSide::Bottom)
    } else {
        None
    }
}

fn zone_island_id(net_name: &str, layer: &str, zone_index: usize) -> String {
    format!(
        "{}_{}_zone_{}",
        sanitize_island_id_part(layer),
        sanitize_island_id_part(net_name),
        zone_index
    )
}

fn sanitize_island_id_part(value: &str) -> String {
    let mut sanitized = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }
    let sanitized = sanitized.trim_matches('_');
    if sanitized.is_empty() {
        "unnamed".to_string()
    } else {
        sanitized.to_string()
    }
}
