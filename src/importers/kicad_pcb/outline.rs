use super::{PcbPoint, non_empty_child_string, route_point};
use crate::importers::kicad_sch::sexp::{Sexp, list_children};
use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_yaml_ng::Value;
use std::f64::consts::TAU;
use std::path::Path;

const OUTLINE_CIRCLE_SAMPLE_SEGMENTS: usize = 32;
const OUTLINE_ARC_MAX_SEGMENT_ANGLE_RAD: f64 = std::f64::consts::PI / 16.0;
const OUTLINE_ARC_MAX_SAMPLE_SEGMENTS: usize = 64;

#[derive(Debug, Clone)]
pub(super) struct PcbOutline {
    segments: Vec<PcbOutlineSegment>,
}

impl PcbOutline {
    pub(super) fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    pub(super) fn len(&self) -> usize {
        self.segments.len()
    }
}

#[derive(Debug, Clone)]
struct PcbOutlineSegment {
    start: PcbPoint,
    end: PcbPoint,
    layer: String,
    source_primitive: PcbOutlinePrimitive,
    source_primitive_index: usize,
    sample_index: usize,
    sample_count: usize,
    contour_index: Option<usize>,
    boundary_role: PcbOutlineBoundaryRole,
}

#[derive(Debug, Clone, Copy, Serialize)]
enum PcbOutlinePrimitive {
    #[serde(rename = "gr_line")]
    Line,
    #[serde(rename = "gr_rect")]
    Rect,
    #[serde(rename = "gr_circle")]
    Circle,
    #[serde(rename = "gr_arc")]
    Arc,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum PcbOutlineBoundaryRole {
    External,
    Cutout,
    Unknown,
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
    source_primitive: PcbOutlinePrimitive,
    source_primitive_index: usize,
    sample_index: usize,
    sample_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    contour_index: Option<usize>,
    boundary_role: PcbOutlineBoundaryRole,
}

pub(super) fn parse_outline(root_list: &[Sexp], path: &Path) -> Result<PcbOutline> {
    let mut segments = Vec::new();
    let mut source_primitive_index = 0;
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
        segments.push(PcbOutlineSegment {
            start,
            end,
            layer,
            source_primitive: PcbOutlinePrimitive::Line,
            source_primitive_index,
            sample_index: 0,
            sample_count: 1,
            contour_index: None,
            boundary_role: PcbOutlineBoundaryRole::Unknown,
        });
        source_primitive_index += 1;
    }
    for rect in list_children(root_list, "gr_rect") {
        let layer = non_empty_child_string(rect, "layer", path)?;
        if layer != "Edge.Cuts" {
            continue;
        }
        let start = route_point(rect, "start", path)?;
        let end = route_point(rect, "end", path)?;
        if (end.x_mm - start.x_mm).abs() <= f64::EPSILON
            || (end.y_mm - start.y_mm).abs() <= f64::EPSILON
        {
            bail!(
                "KiCad PCB Edge.Cuts gr_rect in {} has zero width or height.",
                path.display()
            );
        }
        segments.extend(sample_outline_rect(
            start,
            end,
            layer,
            source_primitive_index,
        ));
        source_primitive_index += 1;
    }
    for circle in list_children(root_list, "gr_circle") {
        let layer = non_empty_child_string(circle, "layer", path)?;
        if layer != "Edge.Cuts" {
            continue;
        }
        let center = route_point(circle, "center", path)?;
        let end = route_point(circle, "end", path)?;
        let radius_mm = point_distance_mm(center, end);
        if radius_mm <= f64::EPSILON {
            bail!(
                "KiCad PCB Edge.Cuts gr_circle in {} has zero radius.",
                path.display()
            );
        }
        segments.extend(sample_outline_circle(
            center,
            end,
            layer,
            source_primitive_index,
        ));
        source_primitive_index += 1;
    }
    for arc in list_children(root_list, "gr_arc") {
        let layer = non_empty_child_string(arc, "layer", path)?;
        if layer != "Edge.Cuts" {
            continue;
        }
        let start = route_point(arc, "start", path)?;
        let mid = route_point(arc, "mid", path)?;
        let end = route_point(arc, "end", path)?;
        let Some(sampled_segments) =
            sample_outline_arc(start, mid, end, layer, source_primitive_index)
        else {
            bail!(
                "KiCad PCB Edge.Cuts gr_arc in {} is degenerate.",
                path.display()
            );
        };
        segments.extend(sampled_segments);
        source_primitive_index += 1;
    }
    classify_outline_contours(&mut segments);
    Ok(PcbOutline { segments })
}

fn sample_outline_rect(
    start: PcbPoint,
    end: PcbPoint,
    layer: String,
    source_primitive_index: usize,
) -> Vec<PcbOutlineSegment> {
    let corners = [
        start,
        PcbPoint {
            x_mm: end.x_mm,
            y_mm: start.y_mm,
        },
        end,
        PcbPoint {
            x_mm: start.x_mm,
            y_mm: end.y_mm,
        },
    ];
    corners
        .iter()
        .copied()
        .zip(corners.iter().copied().cycle().skip(1))
        .take(corners.len())
        .enumerate()
        .map(|(index, (segment_start, segment_end))| PcbOutlineSegment {
            start: segment_start,
            end: segment_end,
            layer: layer.clone(),
            source_primitive: PcbOutlinePrimitive::Rect,
            source_primitive_index,
            sample_index: index,
            sample_count: corners.len(),
            contour_index: None,
            boundary_role: PcbOutlineBoundaryRole::Unknown,
        })
        .collect()
}

fn sample_outline_circle(
    center: PcbPoint,
    end: PcbPoint,
    layer: String,
    source_primitive_index: usize,
) -> Vec<PcbOutlineSegment> {
    let radius_mm = point_distance_mm(center, end);
    let initial_angle = (end.y_mm - center.y_mm).atan2(end.x_mm - center.x_mm);
    (0..OUTLINE_CIRCLE_SAMPLE_SEGMENTS)
        .map(|index| {
            let start_angle =
                initial_angle + TAU * index as f64 / OUTLINE_CIRCLE_SAMPLE_SEGMENTS as f64;
            let end_angle =
                initial_angle + TAU * (index + 1) as f64 / OUTLINE_CIRCLE_SAMPLE_SEGMENTS as f64;
            PcbOutlineSegment {
                start: point_on_circle(center, radius_mm, start_angle),
                end: point_on_circle(center, radius_mm, end_angle),
                layer: layer.clone(),
                source_primitive: PcbOutlinePrimitive::Circle,
                source_primitive_index,
                sample_index: index,
                sample_count: OUTLINE_CIRCLE_SAMPLE_SEGMENTS,
                contour_index: None,
                boundary_role: PcbOutlineBoundaryRole::Unknown,
            }
        })
        .collect()
}

fn sample_outline_arc(
    start: PcbPoint,
    mid: PcbPoint,
    end: PcbPoint,
    layer: String,
    source_primitive_index: usize,
) -> Option<Vec<PcbOutlineSegment>> {
    let center = arc_center(start, mid, end)?;
    let radius_mm = point_distance_mm(center, start);
    if radius_mm <= f64::EPSILON {
        return None;
    }
    let start_angle = (start.y_mm - center.y_mm).atan2(start.x_mm - center.x_mm);
    let mid_angle = (mid.y_mm - center.y_mm).atan2(mid.x_mm - center.x_mm);
    let end_angle = (end.y_mm - center.y_mm).atan2(end.x_mm - center.x_mm);
    let ccw = angle_on_ccw_arc(start_angle, mid_angle, end_angle);
    let sweep = if ccw {
        positive_angle_delta(start_angle, end_angle)
    } else {
        -positive_angle_delta(end_angle, start_angle)
    };
    let segment_count = ((sweep.abs() / OUTLINE_ARC_MAX_SEGMENT_ANGLE_RAD).ceil() as usize)
        .clamp(1, OUTLINE_ARC_MAX_SAMPLE_SEGMENTS);
    Some(
        (0..segment_count)
            .map(|index| {
                let start_t = index as f64 / segment_count as f64;
                let end_t = (index + 1) as f64 / segment_count as f64;
                PcbOutlineSegment {
                    start: point_on_circle(center, radius_mm, start_angle + sweep * start_t),
                    end: point_on_circle(center, radius_mm, start_angle + sweep * end_t),
                    layer: layer.clone(),
                    source_primitive: PcbOutlinePrimitive::Arc,
                    source_primitive_index,
                    sample_index: index,
                    sample_count: segment_count,
                    contour_index: None,
                    boundary_role: PcbOutlineBoundaryRole::Unknown,
                }
            })
            .collect(),
    )
}

struct OutlineContour {
    segment_indices: Vec<usize>,
    points: Vec<PcbPoint>,
    area_mm2: f64,
}

fn classify_outline_contours(segments: &mut [PcbOutlineSegment]) {
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
            PcbOutlineBoundaryRole::External
        } else {
            PcbOutlineBoundaryRole::Cutout
        };
        for segment_index in &contour.segment_indices {
            segments[*segment_index].contour_index = Some(contour_index);
            segments[*segment_index].boundary_role = boundary_role;
        }
    }
}

fn outline_contours(segments: &[PcbOutlineSegment]) -> Vec<OutlineContour> {
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

fn contour_representative_point(contour: &OutlineContour) -> PcbPoint {
    let count = contour.points.len() as f64;
    PcbPoint {
        x_mm: contour.points.iter().map(|point| point.x_mm).sum::<f64>() / count,
        y_mm: contour.points.iter().map(|point| point.y_mm).sum::<f64>() / count,
    }
}

fn polygon_signed_area_mm2(points: &[PcbPoint]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(a, b)| a.x_mm * b.y_mm - b.x_mm * a.y_mm)
        .sum::<f64>()
        / 2.0
}

fn point_inside_polygon(point: PcbPoint, polygon: &[PcbPoint]) -> bool {
    let mut inside = false;
    for (a, b) in polygon
        .iter()
        .zip(polygon.iter().cycle().skip(1))
        .take(polygon.len())
    {
        let crosses_y = (a.y_mm > point.y_mm) != (b.y_mm > point.y_mm);
        if crosses_y {
            let x_at_y = (b.x_mm - a.x_mm) * (point.y_mm - a.y_mm) / (b.y_mm - a.y_mm) + a.x_mm;
            if point.x_mm < x_at_y {
                inside = !inside;
            }
        }
    }
    inside
}

fn points_close(a: PcbPoint, b: PcbPoint) -> bool {
    (a.x_mm - b.x_mm).abs() <= 1.0e-9 && (a.y_mm - b.y_mm).abs() <= 1.0e-9
}

fn point_on_circle(center: PcbPoint, radius_mm: f64, angle_rad: f64) -> PcbPoint {
    PcbPoint {
        x_mm: center.x_mm + radius_mm * angle_rad.cos(),
        y_mm: center.y_mm + radius_mm * angle_rad.sin(),
    }
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

fn angle_on_ccw_arc(start_angle: f64, test_angle: f64, end_angle: f64) -> bool {
    positive_angle_delta(start_angle, test_angle) <= positive_angle_delta(start_angle, end_angle)
}

fn positive_angle_delta(from: f64, to: f64) -> f64 {
    (to - from).rem_euclid(TAU)
}

fn point_distance_mm(a: PcbPoint, b: PcbPoint) -> f64 {
    (a.x_mm - b.x_mm).hypot(a.y_mm - b.y_mm)
}

pub(super) fn outline_yaml_value(outline: &PcbOutline) -> Result<Value> {
    serde_yaml_ng::to_value(OutlineYaml {
        segments: outline
            .segments
            .iter()
            .map(|segment| OutlineSegmentYaml {
                start: segment.start,
                end: segment.end,
                layer: segment.layer.clone(),
                source_primitive: segment.source_primitive,
                source_primitive_index: segment.source_primitive_index,
                sample_index: segment.sample_index,
                sample_count: segment.sample_count,
                contour_index: segment.contour_index,
                boundary_role: segment.boundary_role,
            })
            .collect(),
    })
    .context("Failed to serialize KiCad PCB board outline evidence into Board IR YAML.")
}
