use super::{
    FootprintAt, PcbPoint, coordinate_points, footprint_at, footprint_reference,
    non_empty_child_string, transform_footprint_point,
};
use crate::importers::kicad_sch::sexp::{Sexp, child_list, list_children, numeric_at};
use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_yaml_ng::Value;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub(super) struct PcbFootprint {
    segments: Vec<PcbFootprintSegment>,
    rectangles: Vec<PcbFootprintRectangle>,
    polygons: Vec<PcbFootprintPolygon>,
    circles: Vec<PcbFootprintCircle>,
    arcs: Vec<PcbFootprintArc>,
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

#[derive(Debug, Clone)]
struct PcbFootprintPolygon {
    points: Vec<PcbPoint>,
    layer: String,
    kind: String,
}

#[derive(Debug, Clone)]
struct PcbFootprintCircle {
    center: PcbPoint,
    end: PcbPoint,
    layer: String,
    kind: String,
}

#[derive(Debug, Clone)]
struct PcbFootprintArc {
    start: PcbPoint,
    mid: PcbPoint,
    end: PcbPoint,
    layer: String,
    kind: String,
}

#[derive(Debug, Serialize)]
struct FootprintYaml {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    segments: Vec<FootprintSegmentYaml>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    rectangles: Vec<FootprintRectangleYaml>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    polygons: Vec<FootprintPolygonYaml>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    circles: Vec<FootprintCircleYaml>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    arcs: Vec<FootprintArcYaml>,
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

#[derive(Debug, Serialize)]
struct FootprintPolygonYaml {
    points: Vec<PcbPoint>,
    layer: String,
    kind: String,
}

#[derive(Debug, Serialize)]
struct FootprintCircleYaml {
    center: PcbPoint,
    end: PcbPoint,
    layer: String,
    kind: String,
}

#[derive(Debug, Serialize)]
struct FootprintArcYaml {
    start: PcbPoint,
    mid: PcbPoint,
    end: PcbPoint,
    layer: String,
    kind: String,
}

pub(super) fn parse_footprints(
    root_list: &[Sexp],
    path: &Path,
) -> Result<BTreeMap<String, PcbFootprint>> {
    let mut footprints = BTreeMap::new();
    for footprint in list_children(root_list, "footprint") {
        let reference = footprint_reference(footprint)
            .with_context(|| "KiCad PCB footprint is missing Reference property or fp_text.")?;
        let footprint_at = footprint_at(footprint, &reference)?;
        let mut evidence = PcbFootprint::default();
        for line in list_children(footprint, "fp_line") {
            let start = transformed_child_point(line, "start", footprint_at, path)?;
            let end = transformed_child_point(line, "end", footprint_at, path)?;
            if point_distance_mm(start, end) <= f64::EPSILON {
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
        for polygon in list_children(footprint, "fp_poly") {
            let pts = child_list(polygon, "pts").with_context(|| {
                format!(
                    "KiCad PCB footprint {reference} fp_poly in {} is missing pts list.",
                    path.display()
                )
            })?;
            let points =
                transformed_coordinate_points(pts, "footprint fp_poly", footprint_at, path)?;
            let layer = non_empty_child_string(polygon, "layer", path)?;
            evidence.polygons.push(PcbFootprintPolygon {
                points,
                kind: footprint_graphic_kind(&layer).to_string(),
                layer,
            });
        }
        for circle in list_children(footprint, "fp_circle") {
            let center = transformed_child_point(circle, "center", footprint_at, path)?;
            let end = transformed_child_point(circle, "end", footprint_at, path)?;
            if point_distance_mm(center, end) <= f64::EPSILON {
                bail!(
                    "KiCad PCB footprint {reference} fp_circle in {} has zero radius.",
                    path.display()
                );
            }
            let layer = non_empty_child_string(circle, "layer", path)?;
            evidence.circles.push(PcbFootprintCircle {
                center,
                end,
                kind: footprint_graphic_kind(&layer).to_string(),
                layer,
            });
        }
        for arc in list_children(footprint, "fp_arc") {
            let start = transformed_child_point(arc, "start", footprint_at, path)?;
            let mid = transformed_child_point(arc, "mid", footprint_at, path)?;
            let end = transformed_child_point(arc, "end", footprint_at, path)?;
            if arc_center(start, mid, end).is_none() {
                bail!(
                    "KiCad PCB footprint {reference} fp_arc in {} is degenerate.",
                    path.display()
                );
            }
            let layer = non_empty_child_string(arc, "layer", path)?;
            evidence.arcs.push(PcbFootprintArc {
                start,
                mid,
                end,
                kind: footprint_graphic_kind(&layer).to_string(),
                layer,
            });
        }
        if footprint_graphic_count(&evidence) > 0 {
            footprints.insert(reference, evidence);
        }
    }
    Ok(footprints)
}

pub(super) fn footprint_graphic_count(footprint: &PcbFootprint) -> usize {
    footprint.segments.len()
        + footprint.rectangles.len()
        + footprint.polygons.len()
        + footprint.circles.len()
        + footprint.arcs.len()
}

pub(super) fn footprint_yaml_value(footprint: &PcbFootprint) -> Result<Value> {
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
        polygons: footprint
            .polygons
            .iter()
            .map(|polygon| FootprintPolygonYaml {
                points: polygon.points.clone(),
                layer: polygon.layer.clone(),
                kind: polygon.kind.clone(),
            })
            .collect(),
        circles: footprint
            .circles
            .iter()
            .map(|circle| FootprintCircleYaml {
                center: circle.center,
                end: circle.end,
                layer: circle.layer.clone(),
                kind: circle.kind.clone(),
            })
            .collect(),
        arcs: footprint
            .arcs
            .iter()
            .map(|arc| FootprintArcYaml {
                start: arc.start,
                mid: arc.mid,
                end: arc.end,
                layer: arc.layer.clone(),
                kind: arc.kind.clone(),
            })
            .collect(),
    })
    .context("Failed to serialize KiCad PCB footprint drawing evidence into Board IR YAML.")
}

fn transformed_coordinate_points(
    pts: &[Sexp],
    item_kind: &str,
    footprint_at: FootprintAt,
    path: &Path,
) -> Result<Vec<PcbPoint>> {
    coordinate_points(pts, item_kind, path).map(|points| {
        points
            .into_iter()
            .map(|point| transform_footprint_point(footprint_at, point.x_mm, point.y_mm))
            .collect()
    })
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

fn point_distance_mm(a: PcbPoint, b: PcbPoint) -> f64 {
    (a.x_mm - b.x_mm).hypot(a.y_mm - b.y_mm)
}

fn arc_center(start: PcbPoint, mid: PcbPoint, end: PcbPoint) -> Option<PcbPoint> {
    let d = 2.0
        * (start.x_mm * (mid.y_mm - end.y_mm)
            + mid.x_mm * (end.y_mm - start.y_mm)
            + end.x_mm * (start.y_mm - mid.y_mm));
    if d.abs() <= f64::EPSILON {
        return None;
    }
    let start_sq = start.x_mm * start.x_mm + start.y_mm * start.y_mm;
    let mid_sq = mid.x_mm * mid.x_mm + mid.y_mm * mid.y_mm;
    let end_sq = end.x_mm * end.x_mm + end.y_mm * end.y_mm;
    Some(PcbPoint {
        x_mm: (start_sq * (mid.y_mm - end.y_mm)
            + mid_sq * (end.y_mm - start.y_mm)
            + end_sq * (start.y_mm - mid.y_mm))
            / d,
        y_mm: (start_sq * (end.x_mm - mid.x_mm)
            + mid_sq * (start.x_mm - end.x_mm)
            + end_sq * (mid.x_mm - start.x_mm))
            / d,
    })
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
