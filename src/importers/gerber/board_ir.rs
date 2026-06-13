use super::{
    GerberApertureShape, GerberBoundaryRole, GerberCopper, GerberCopperImportSummary,
    GerberOutline, GerberOutlineImportSummary, GerberPoint, GerberSolderMaskImportSummary,
    GerberSolderPasteImportSummary,
};
use anyhow::{Context, Result};
use serde::Serialize;
use serde_yaml_ng::{Mapping, Value};

#[derive(Debug, Serialize)]
struct OutlineYaml {
    segments: Vec<OutlineSegmentYaml>,
}

#[derive(Debug, Serialize)]
struct OutlineSegmentYaml {
    start: GerberPoint,
    end: GerberPoint,
    layer: String,
    source_primitive: String,
    source_primitive_index: usize,
    sample_index: usize,
    sample_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    contour_index: Option<usize>,
    boundary_role: GerberBoundaryRole,
}

#[derive(Debug, Serialize)]
struct CopperYaml {
    features: Vec<CopperFeatureYaml>,
    segments: Vec<CopperSegmentYaml>,
    regions: Vec<CopperRegionYaml>,
}

#[derive(Debug, Serialize)]
struct CopperFeatureYaml {
    at: GerberPoint,
    layer: String,
    polarity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    net: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    island_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner_kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    component: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    via_index: Option<usize>,
    source_primitive: String,
    source_primitive_index: usize,
    aperture: String,
    shape: GerberApertureShape,
    size: CopperFeatureSizeYaml,
}

#[derive(Debug, Serialize)]
struct CopperFeatureSizeYaml {
    x_mm: f64,
    y_mm: f64,
}

#[derive(Debug, Serialize)]
struct CopperSegmentYaml {
    start: GerberPoint,
    end: GerberPoint,
    layer: String,
    polarity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    net: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    island_id: Option<String>,
    source_primitive: String,
    source_primitive_index: usize,
    aperture: String,
    width_mm: f64,
}

#[derive(Debug, Serialize)]
struct CopperRegionYaml {
    points: Vec<GerberPoint>,
    layer: String,
    polarity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    net: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    island_id: Option<String>,
    source_primitive: String,
    source_primitive_index: usize,
}

pub(super) fn merge_outline_into_project(
    project_yaml: &mut Value,
    outline: &GerberOutline,
) -> Result<()> {
    let board = mapping_field_mut(project_yaml, "board")?;
    let layout = mapping_field_in_mapping_mut(board, "layout")?;
    layout.insert(
        Value::String("outline".to_string()),
        serde_yaml_ng::to_value(OutlineYaml {
            segments: outline
                .segments
                .iter()
                .map(|segment| OutlineSegmentYaml {
                    start: segment.start,
                    end: segment.end,
                    layer: outline.layer.clone(),
                    source_primitive: "gerber_linear".to_string(),
                    source_primitive_index: segment.source_primitive_index,
                    sample_index: 0,
                    sample_count: 1,
                    contour_index: segment.contour_index,
                    boundary_role: segment.boundary_role,
                })
                .collect(),
        })
        .context("Failed to serialize Gerber outline evidence into Board IR YAML.")?,
    );
    Ok(())
}

pub(super) fn merge_copper_into_project(
    project_yaml: &mut Value,
    copper: &GerberCopper,
) -> Result<()> {
    merge_layer_artwork_into_project(project_yaml, "copper", "copper", copper)
}

pub(super) fn merge_solder_mask_into_project(
    project_yaml: &mut Value,
    mask: &GerberCopper,
) -> Result<()> {
    merge_layer_artwork_into_project(project_yaml, "solder_mask", "solder mask", mask)
}

pub(super) fn merge_solder_paste_into_project(
    project_yaml: &mut Value,
    paste: &GerberCopper,
) -> Result<()> {
    merge_layer_artwork_into_project(project_yaml, "solder_paste", "solder paste", paste)
}

fn merge_layer_artwork_into_project(
    project_yaml: &mut Value,
    layout_key_name: &str,
    context_name: &str,
    copper: &GerberCopper,
) -> Result<()> {
    let board = mapping_field_mut(project_yaml, "board")?;
    let layout = mapping_field_in_mapping_mut(board, "layout")?;
    let copper_key = Value::String(layout_key_name.to_string());
    if !layout.contains_key(&copper_key) {
        layout.insert(
            copper_key.clone(),
            serde_yaml_ng::to_value(CopperYaml {
                features: Vec::new(),
                segments: Vec::new(),
                regions: Vec::new(),
            })
            .with_context(|| {
                format!("Failed to initialize Board IR {context_name} evidence YAML.")
            })?,
        );
    }
    let copper_yaml = layout
        .get_mut(&copper_key)
        .expect("copper field was inserted when absent")
        .as_mapping_mut()
        .with_context(|| {
            format!("Board IR field board.layout.{layout_key_name} must be an object.")
        })?;
    let features_key = Value::String("features".to_string());
    if !copper_yaml.contains_key(&features_key) {
        copper_yaml.insert(features_key.clone(), Value::Sequence(Vec::new()));
    }
    let features = copper_yaml
        .get_mut(&features_key)
        .expect("features field was inserted when absent")
        .as_sequence_mut()
        .with_context(|| {
            format!("Board IR field board.layout.{layout_key_name}.features must be a list.")
        })?;
    for feature in &copper.features {
        features.push(
            serde_yaml_ng::to_value(CopperFeatureYaml {
                at: feature.at,
                layer: copper.layer.clone(),
                polarity: "dark".to_string(),
                net: feature.net.clone(),
                island_id: feature.island_id.clone(),
                owner_kind: feature.owner_kind.clone(),
                component: feature.component.clone(),
                pin: feature.pin.clone(),
                via_index: feature.via_index,
                source_primitive: "gerber_flash".to_string(),
                source_primitive_index: feature.source_primitive_index,
                aperture: format!("D{}", feature.aperture_code),
                shape: feature.aperture.shape,
                size: CopperFeatureSizeYaml {
                    x_mm: feature.aperture.x_mm,
                    y_mm: feature.aperture.y_mm,
                },
            })
            .with_context(|| {
                format!("Failed to serialize Gerber {context_name} evidence into Board IR YAML.")
            })?,
        );
    }
    let segments_key = Value::String("segments".to_string());
    if !copper_yaml.contains_key(&segments_key) {
        copper_yaml.insert(segments_key.clone(), Value::Sequence(Vec::new()));
    }
    let segments = copper_yaml
        .get_mut(&segments_key)
        .expect("segments field was inserted when absent")
        .as_sequence_mut()
        .with_context(|| {
            format!("Board IR field board.layout.{layout_key_name}.segments must be a list.")
        })?;
    for segment in &copper.segments {
        segments.push(
            serde_yaml_ng::to_value(CopperSegmentYaml {
                start: segment.start,
                end: segment.end,
                layer: copper.layer.clone(),
                polarity: "dark".to_string(),
                net: segment.net.clone(),
                island_id: segment.island_id.clone(),
                source_primitive: segment.source_primitive.to_string(),
                source_primitive_index: segment.source_primitive_index,
                aperture: format!("D{}", segment.aperture_code),
                width_mm: segment.aperture.x_mm,
            })
            .with_context(|| {
                format!(
                    "Failed to serialize Gerber {context_name} segment evidence into Board IR YAML."
                )
            })?,
        );
    }
    let regions_key = Value::String("regions".to_string());
    if !copper_yaml.contains_key(&regions_key) {
        copper_yaml.insert(regions_key.clone(), Value::Sequence(Vec::new()));
    }
    let regions = copper_yaml
        .get_mut(&regions_key)
        .expect("regions field was inserted when absent")
        .as_sequence_mut()
        .with_context(|| {
            format!("Board IR field board.layout.{layout_key_name}.regions must be a list.")
        })?;
    for region in &copper.regions {
        regions.push(
            serde_yaml_ng::to_value(CopperRegionYaml {
                points: region.points.clone(),
                layer: copper.layer.clone(),
                polarity: "dark".to_string(),
                net: region.net.clone(),
                island_id: region.island_id.clone(),
                source_primitive: "gerber_region".to_string(),
                source_primitive_index: region.source_primitive_index,
            })
            .with_context(|| {
                format!(
                    "Failed to serialize Gerber {context_name} region evidence into Board IR YAML."
                )
            })?,
        );
    }
    Ok(())
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

fn mapping_field_in_mapping_mut<'a>(
    mapping: &'a mut Mapping,
    key: &str,
) -> Result<&'a mut Mapping> {
    mapping
        .get_mut(Value::String(key.to_string()))
        .with_context(|| format!("Board IR project is missing board.{key}."))?
        .as_mapping_mut()
        .with_context(|| format!("Board IR field board.{key} must be an object."))
}

pub(super) fn summary_for_outline(outline: &GerberOutline) -> GerberOutlineImportSummary {
    GerberOutlineImportSummary {
        outline_segments: outline.segments.len(),
        external_segments: outline
            .segments
            .iter()
            .filter(|segment| segment.boundary_role == GerberBoundaryRole::External)
            .count(),
        cutout_segments: outline
            .segments
            .iter()
            .filter(|segment| segment.boundary_role == GerberBoundaryRole::Cutout)
            .count(),
        unknown_segments: outline
            .segments
            .iter()
            .filter(|segment| segment.boundary_role == GerberBoundaryRole::Unknown)
            .count(),
    }
}

pub(super) fn summary_for_copper(copper: &GerberCopper) -> GerberCopperImportSummary {
    GerberCopperImportSummary {
        flash_features: copper.features.len(),
        trace_segments: copper.segments.len(),
        regions: copper.regions.len(),
        net_associated_features: copper
            .features
            .iter()
            .filter(|feature| feature.net.is_some())
            .count(),
        net_associated_segments: copper
            .segments
            .iter()
            .filter(|segment| segment.net.is_some())
            .count(),
        net_associated_regions: copper
            .regions
            .iter()
            .filter(|region| region.net.is_some())
            .count(),
        island_associated_features: copper
            .features
            .iter()
            .filter(|feature| feature.island_id.is_some())
            .count(),
        island_associated_segments: copper
            .segments
            .iter()
            .filter(|segment| segment.island_id.is_some())
            .count(),
        island_associated_regions: copper
            .regions
            .iter()
            .filter(|region| region.island_id.is_some())
            .count(),
        apertures: copper.aperture_count,
        ignored_draws: copper.ignored_draws,
        skipped_clear_flashes: copper.skipped_clear_flashes,
        skipped_clear_regions: copper.skipped_clear_regions,
    }
}

pub(super) fn summary_for_solder_mask(mask: &GerberCopper) -> GerberSolderMaskImportSummary {
    GerberSolderMaskImportSummary {
        openings: mask.features.len(),
        draw_openings: mask.segments.len(),
        region_openings: mask.regions.len(),
        apertures: mask.aperture_count,
        ignored_draws: mask.ignored_draws,
        skipped_clear_flashes: mask.skipped_clear_flashes,
        skipped_clear_regions: mask.skipped_clear_regions,
    }
}

pub(super) fn summary_for_solder_paste(paste: &GerberCopper) -> GerberSolderPasteImportSummary {
    GerberSolderPasteImportSummary {
        openings: paste.features.len(),
        draw_openings: paste.segments.len(),
        region_openings: paste.regions.len(),
        apertures: paste.aperture_count,
        ignored_draws: paste.ignored_draws,
        skipped_clear_flashes: paste.skipped_clear_flashes,
        skipped_clear_regions: paste.skipped_clear_regions,
    }
}
