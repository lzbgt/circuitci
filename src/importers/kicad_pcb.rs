use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_yaml_ng::{Mapping, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

mod footprints;
mod outline;
mod parse;

use parse::{
    ParsedPcb, PcbNetRule, PcbPad, PcbPadSize, PcbPlacementSide, PcbPoint, PcbRoute, PcbZone,
    parse_kicad_pcb,
};

use footprints::{
    footprint_graphic_count, footprint_has_entry_aperture, footprint_has_entry_clearance,
    footprint_has_entry_direction, footprint_yaml_value,
};
use outline::outline_yaml_value;

#[derive(Debug)]
pub struct KicadPcbPlacementImportOptions {
    pub input: PathBuf,
    pub project: PathBuf,
    pub output: PathBuf,
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
    fabrication: Option<PadFabricationYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    rotation_deg: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    drill_mm: Option<f64>,
}

#[derive(Debug, Serialize)]
struct PadFabricationYaml {
    #[serde(skip_serializing_if = "Option::is_none")]
    solder_mask_margin_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    solder_paste_margin_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    solder_paste_margin_ratio: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    clearance_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    zone_connect: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thermal_bridge_width_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thermal_gap_mm: Option<f64>,
    source: String,
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
    island_id: String,
    polygon: Vec<PcbPoint>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    filled_polygons: Vec<Vec<PcbPoint>>,
}

pub fn import_kicad_pcb_placements(
    options: &KicadPcbPlacementImportOptions,
) -> Result<KicadPcbImportSummary> {
    import_kicad_pcb_placements_with_progress(options, |_, _| {})
}

pub fn import_kicad_pcb_placements_with_progress<F>(
    options: &KicadPcbPlacementImportOptions,
    mut on_progress: F,
) -> Result<KicadPcbImportSummary>
where
    F: FnMut(&'static str, String),
{
    import_kicad_pcb_placements_with_progress_and_cancel(options, &mut on_progress, || false)
}

pub fn import_kicad_pcb_placements_with_progress_and_cancel<F, C>(
    options: &KicadPcbPlacementImportOptions,
    mut on_progress: F,
    should_cancel: C,
) -> Result<KicadPcbImportSummary>
where
    F: FnMut(&'static str, String),
    C: Fn() -> bool,
{
    on_progress(
        "Parsing KiCad PCB",
        format!("Reading {}.", options.input.display()),
    );
    let parsed_pcb = parse_kicad_pcb(&options.input)?;
    ensure_not_canceled(&should_cancel)?;
    on_progress(
        "Loading Board IR",
        format!("Reading {}.", options.project.display()),
    );
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
    ensure_not_canceled(&should_cancel)?;
    on_progress(
        "Merging PCB evidence",
        format!(
            "{} placement(s), {} pad set(s), {} route(s), {} zone group(s).",
            parsed_pcb.placements.len(),
            parsed_pcb.pads.len(),
            parsed_pcb.routes.len(),
            parsed_pcb.zones.len()
        ),
    );
    let summary = merge_pcb_into_project(&mut project_yaml, &parsed_pcb)?;
    ensure_not_canceled(&should_cancel)?;
    if summary.placements == 0 {
        bail!(
            "KiCad PCB {} has no footprint references matching Board IR project components in {}.",
            options.input.display(),
            options.project.display()
        );
    }
    if let Some(parent) = options.output.parent() {
        on_progress(
            "Preparing output",
            format!("Creating output directory {}.", parent.display()),
        );
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
    ensure_not_canceled(&should_cancel)?;
    on_progress(
        "Serializing Board IR",
        format!("Writing {}.", options.output.display()),
    );
    let mut yaml = serde_yaml_ng::to_string(&project_yaml)?;
    yaml.insert_str(
        0,
        "# Generated by CircuitCI by adding KiCad PCB layout evidence to Board IR.\n",
    );
    fs::write(&options.output, yaml)
        .with_context(|| format!("Failed to write {}", options.output.display()))?;
    Ok(summary)
}

fn ensure_not_canceled(should_cancel: &impl Fn() -> bool) -> Result<()> {
    if should_cancel() {
        return Err(crate::cancellation::canceled(
            "KiCad PCB import canceled before completion.",
        ));
    }
    Ok(())
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
        let mut footprint_value = footprint_yaml_value(footprint)?;
        if !footprint_has_entry_direction(footprint) {
            preserve_existing_entry_direction(footprint_yaml, reference, &mut footprint_value)?;
        }
        if !footprint_has_entry_clearance(footprint) {
            preserve_existing_entry_clearance(footprint_yaml, reference, &mut footprint_value)?;
        }
        if !footprint_has_entry_aperture(footprint) {
            preserve_existing_entry_aperture(footprint_yaml, reference, &mut footprint_value)?;
        }
        footprint_yaml.insert(Value::String(reference.clone()), footprint_value);
        summary.footprint_graphics += footprint_graphic_count(footprint);
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
    if !parsed_pcb.outline.is_empty() {
        layout.insert(
            Value::String("outline".to_string()),
            outline_yaml_value(&parsed_pcb.outline)?,
        );
        summary.outline_segments = parsed_pcb.outline.len();
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

fn preserve_existing_entry_direction(
    footprint_yaml: &Mapping,
    reference: &str,
    footprint_value: &mut Value,
) -> Result<()> {
    preserve_existing_footprint_field(
        footprint_yaml,
        reference,
        footprint_value,
        "entry_direction",
    )
}

fn preserve_existing_entry_aperture(
    footprint_yaml: &Mapping,
    reference: &str,
    footprint_value: &mut Value,
) -> Result<()> {
    preserve_existing_footprint_field(footprint_yaml, reference, footprint_value, "entry_aperture")
}

fn preserve_existing_entry_clearance(
    footprint_yaml: &Mapping,
    reference: &str,
    footprint_value: &mut Value,
) -> Result<()> {
    preserve_existing_footprint_field(
        footprint_yaml,
        reference,
        footprint_value,
        "entry_clearance",
    )
}

fn preserve_existing_footprint_field(
    footprint_yaml: &Mapping,
    reference: &str,
    footprint_value: &mut Value,
    field: &str,
) -> Result<()> {
    let Some(existing_value) = footprint_yaml
        .get(Value::String(reference.to_string()))
        .and_then(Value::as_mapping)
        .and_then(|footprint| footprint.get(Value::String(field.to_string())))
        .cloned()
    else {
        return Ok(());
    };
    let footprint_mapping = footprint_value
        .as_mapping_mut()
        .context("Serialized KiCad PCB footprint evidence must be a YAML object.")?;
    footprint_mapping
        .entry(Value::String(field.to_string()))
        .or_insert(existing_value);
    Ok(())
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
        fabrication: pad.fabrication.map(|fabrication| PadFabricationYaml {
            solder_mask_margin_mm: fabrication.solder_mask_margin_mm,
            solder_paste_margin_mm: fabrication.solder_paste_margin_mm,
            solder_paste_margin_ratio: fabrication.solder_paste_margin_ratio,
            clearance_mm: fabrication.clearance_mm,
            zone_connect: fabrication.zone_connect,
            thermal_bridge_width_mm: fabrication.thermal_bridge_width_mm,
            thermal_gap_mm: fabrication.thermal_gap_mm,
            source: "kicad_pad_property".to_string(),
        }),
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
                island_id: zone.island_id.clone(),
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

#[cfg(test)]
mod tests {
    use super::{
        KicadPcbPlacementImportOptions, import_kicad_pcb_placements_with_progress,
        import_kicad_pcb_placements_with_progress_and_cancel,
    };

    #[test]
    fn import_kicad_pcb_with_progress_emits_phases() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("wheel_imported_pcb.project.yaml");
        let mut stages = Vec::new();

        let summary = import_kicad_pcb_placements_with_progress(
            &KicadPcbPlacementImportOptions {
                input: "demos/smart_robot/kicad/wheel_actuator/wheel_actuator.kicad_pcb".into(),
                project: "demos/smart_robot/circuitci/wheel_actuator/project.yaml".into(),
                output,
            },
            |stage, _detail| stages.push(stage.to_string()),
        )
        .unwrap();

        assert!(summary.placements > 0);
        for expected in [
            "Parsing KiCad PCB",
            "Loading Board IR",
            "Merging PCB evidence",
            "Preparing output",
            "Serializing Board IR",
        ] {
            assert!(stages.iter().any(|stage| stage == expected), "{expected}");
        }
    }

    #[test]
    fn import_kicad_pcb_cancellation_stops_before_write() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("canceled_pcb.project.yaml");

        let error = import_kicad_pcb_placements_with_progress_and_cancel(
            &KicadPcbPlacementImportOptions {
                input: "demos/smart_robot/kicad/wheel_actuator/wheel_actuator.kicad_pcb".into(),
                project: "demos/smart_robot/circuitci/wheel_actuator/project.yaml".into(),
                output: output.clone(),
            },
            |_, _| {},
            || true,
        )
        .unwrap_err();

        assert!(error.to_string().contains("canceled"));
        assert!(!output.exists());
    }
}
