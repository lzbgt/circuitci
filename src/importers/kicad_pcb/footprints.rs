use super::{
    FootprintAt, PcbPoint, coordinate_points, footprint_at, footprint_reference,
    non_empty_child_string, transform_footprint_point,
};
use crate::importers::kicad_sch::sexp::{Sexp, child_list, list_children, numeric_at, string_at};
use anyhow::{Context, Result, bail};
use serde::Serialize;
use serde_yaml_ng::Value;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub(super) struct PcbFootprint {
    properties: Vec<PcbFootprintProperty>,
    segments: Vec<PcbFootprintSegment>,
    rectangles: Vec<PcbFootprintRectangle>,
    polygons: Vec<PcbFootprintPolygon>,
    circles: Vec<PcbFootprintCircle>,
    arcs: Vec<PcbFootprintArc>,
    semantics: Option<PcbFootprintSemantics>,
    entry_direction: Option<PcbEntryDirection>,
    entry_clearance: Option<PcbEntryClearance>,
    entry_aperture: Option<PcbEntryAperture>,
}

#[derive(Debug, Clone)]
struct PcbFootprintProperty {
    name: String,
    value: String,
    source: &'static str,
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

#[derive(Debug, Clone, Default)]
struct PcbFootprintSemantics {
    body_bounds: Option<PcbSemanticBounds>,
    courtyard_bounds: Option<PcbSemanticBounds>,
    pin_1: Option<PcbPinMarker>,
}

#[derive(Debug, Clone, Copy)]
struct PcbSemanticBounds {
    min: PcbPoint,
    max: PcbPoint,
}

#[derive(Debug, Clone, Copy)]
struct PcbPinMarker {
    at: PcbPoint,
}

#[derive(Debug, Clone, Default)]
struct PcbEntryDirection {
    offset_deg: f64,
}

#[derive(Debug, Clone, Default)]
struct PcbEntryClearance {
    depth_mm: Option<f64>,
    width_mm: Option<f64>,
}

#[derive(Debug, Clone, Default)]
struct PcbEntryAperture {
    front_offset_mm: Option<f64>,
    lateral_offset_mm: Option<f64>,
    width_mm: Option<f64>,
}

#[derive(Debug, Serialize)]
struct FootprintYaml {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    properties: Vec<FootprintPropertyYaml>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    semantics: Option<FootprintSemanticsYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    entry_direction: Option<EntryDirectionYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    entry_clearance: Option<EntryClearanceYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    entry_aperture: Option<EntryApertureYaml>,
}

#[derive(Debug, Serialize)]
struct FootprintPropertyYaml {
    name: String,
    value: String,
    source: &'static str,
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

#[derive(Debug, Serialize)]
struct FootprintSemanticsYaml {
    #[serde(skip_serializing_if = "Option::is_none")]
    body_bounds: Option<FootprintBoundsYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    courtyard_bounds: Option<FootprintBoundsYaml>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pin_1: Option<PinMarkerYaml>,
}

#[derive(Debug, Serialize)]
struct FootprintBoundsYaml {
    min: PcbPoint,
    max: PcbPoint,
    source: String,
}

#[derive(Debug, Serialize)]
struct PinMarkerYaml {
    at: PcbPoint,
    source: String,
}

#[derive(Debug, Serialize)]
struct EntryDirectionYaml {
    offset_deg: f64,
    source: String,
}

#[derive(Debug, Serialize)]
struct EntryClearanceYaml {
    #[serde(skip_serializing_if = "Option::is_none")]
    depth_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    width_mm: Option<f64>,
    source: String,
}

#[derive(Debug, Serialize)]
struct EntryApertureYaml {
    #[serde(skip_serializing_if = "Option::is_none")]
    front_offset_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lateral_offset_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    width_mm: Option<f64>,
    source: String,
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
        let mut evidence = PcbFootprint {
            properties: parse_footprint_properties(footprint, &reference, path)?,
            entry_direction: parse_entry_direction(footprint, &reference, path)?,
            entry_clearance: parse_entry_clearance(footprint, &reference, path)?,
            entry_aperture: parse_entry_aperture(footprint, &reference, path)?,
            ..Default::default()
        };
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
        let pin_1 = parse_pin_1_marker(footprint, footprint_at, &reference, path)?;
        evidence.semantics = footprint_semantics(&evidence, pin_1);
        if !evidence.properties.is_empty()
            || footprint_graphic_count(&evidence) > 0
            || evidence.semantics.is_some()
            || evidence.entry_direction.is_some()
            || evidence.entry_clearance.is_some()
            || evidence.entry_aperture.is_some()
        {
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

pub(super) fn footprint_has_entry_aperture(footprint: &PcbFootprint) -> bool {
    footprint.entry_aperture.is_some()
}

pub(super) fn footprint_has_entry_direction(footprint: &PcbFootprint) -> bool {
    footprint.entry_direction.is_some()
}

pub(super) fn footprint_has_entry_clearance(footprint: &PcbFootprint) -> bool {
    footprint.entry_clearance.is_some()
}

pub(super) fn footprint_yaml_value(footprint: &PcbFootprint) -> Result<Value> {
    serde_yaml_ng::to_value(FootprintYaml {
        properties: footprint
            .properties
            .iter()
            .map(|property| FootprintPropertyYaml {
                name: property.name.clone(),
                value: property.value.clone(),
                source: property.source,
            })
            .collect(),
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
        semantics: footprint
            .semantics
            .as_ref()
            .map(|semantics| FootprintSemanticsYaml {
                body_bounds: semantics.body_bounds.map(|bounds| FootprintBoundsYaml {
                    min: bounds.min,
                    max: bounds.max,
                    source: "kicad_footprint_graphics".to_string(),
                }),
                courtyard_bounds: semantics
                    .courtyard_bounds
                    .map(|bounds| FootprintBoundsYaml {
                        min: bounds.min,
                        max: bounds.max,
                        source: "kicad_footprint_graphics".to_string(),
                    }),
                pin_1: semantics.pin_1.map(|pin_1| PinMarkerYaml {
                    at: pin_1.at,
                    source: "kicad_pad_1".to_string(),
                }),
            }),
        entry_direction: footprint.entry_direction.as_ref().map(|entry_direction| {
            EntryDirectionYaml {
                offset_deg: entry_direction.offset_deg,
                source: "kicad_footprint_property".to_string(),
            }
        }),
        entry_clearance: footprint.entry_clearance.as_ref().map(|entry_clearance| {
            EntryClearanceYaml {
                depth_mm: entry_clearance.depth_mm,
                width_mm: entry_clearance.width_mm,
                source: "kicad_footprint_property".to_string(),
            }
        }),
        entry_aperture: footprint
            .entry_aperture
            .as_ref()
            .map(|entry_aperture| EntryApertureYaml {
                front_offset_mm: entry_aperture.front_offset_mm,
                lateral_offset_mm: entry_aperture.lateral_offset_mm,
                width_mm: entry_aperture.width_mm,
                source: "kicad_footprint_property".to_string(),
            }),
    })
    .context("Failed to serialize KiCad PCB footprint drawing evidence into Board IR YAML.")
}

fn footprint_semantics(
    footprint: &PcbFootprint,
    pin_1: Option<PcbPinMarker>,
) -> Option<PcbFootprintSemantics> {
    let semantics = PcbFootprintSemantics {
        body_bounds: bounds_for_kind(footprint, "fabrication"),
        courtyard_bounds: bounds_for_kind(footprint, "courtyard"),
        pin_1,
    };
    (semantics.body_bounds.is_some()
        || semantics.courtyard_bounds.is_some()
        || semantics.pin_1.is_some())
    .then_some(semantics)
}

fn bounds_for_kind(footprint: &PcbFootprint, kind: &str) -> Option<PcbSemanticBounds> {
    let mut bounds = BoundsBuilder::default();
    for segment in footprint
        .segments
        .iter()
        .filter(|segment| segment.kind == kind)
    {
        bounds.include_point(segment.start);
        bounds.include_point(segment.end);
    }
    for rectangle in footprint
        .rectangles
        .iter()
        .filter(|rectangle| rectangle.kind == kind)
    {
        bounds.include_point(rectangle.start);
        bounds.include_point(rectangle.end);
    }
    for polygon in footprint
        .polygons
        .iter()
        .filter(|polygon| polygon.kind == kind)
    {
        for point in &polygon.points {
            bounds.include_point(*point);
        }
    }
    for circle in footprint
        .circles
        .iter()
        .filter(|circle| circle.kind == kind)
    {
        bounds.include_circle(circle.center, circle.end);
    }
    for arc in footprint.arcs.iter().filter(|arc| arc.kind == kind) {
        bounds.include_point(arc.start);
        bounds.include_point(arc.mid);
        bounds.include_point(arc.end);
    }
    bounds.finish()
}

#[derive(Debug, Default)]
struct BoundsBuilder {
    min_x: Option<f64>,
    min_y: Option<f64>,
    max_x: Option<f64>,
    max_y: Option<f64>,
}

impl BoundsBuilder {
    fn include_point(&mut self, point: PcbPoint) {
        self.min_x = Some(self.min_x.map_or(point.x_mm, |min| min.min(point.x_mm)));
        self.min_y = Some(self.min_y.map_or(point.y_mm, |min| min.min(point.y_mm)));
        self.max_x = Some(self.max_x.map_or(point.x_mm, |max| max.max(point.x_mm)));
        self.max_y = Some(self.max_y.map_or(point.y_mm, |max| max.max(point.y_mm)));
    }

    fn include_circle(&mut self, center: PcbPoint, end: PcbPoint) {
        let radius_mm = point_distance_mm(center, end);
        self.include_point(PcbPoint {
            x_mm: center.x_mm - radius_mm,
            y_mm: center.y_mm - radius_mm,
        });
        self.include_point(PcbPoint {
            x_mm: center.x_mm + radius_mm,
            y_mm: center.y_mm + radius_mm,
        });
    }

    fn finish(self) -> Option<PcbSemanticBounds> {
        Some(PcbSemanticBounds {
            min: PcbPoint {
                x_mm: self.min_x?,
                y_mm: self.min_y?,
            },
            max: PcbPoint {
                x_mm: self.max_x?,
                y_mm: self.max_y?,
            },
        })
    }
}

fn parse_pin_1_marker(
    footprint: &[Sexp],
    footprint_at: FootprintAt,
    reference: &str,
    path: &Path,
) -> Result<Option<PcbPinMarker>> {
    let mut pin_1 = None;
    for pad in list_children(footprint, "pad") {
        let Some(pad_name) = string_at(pad, 1).map(str::trim) else {
            continue;
        };
        if pad_name != "1" {
            continue;
        }
        let local_at = child_list(pad, "at").with_context(|| {
            format!(
                "KiCad PCB footprint {reference} pad 1 in {} is missing (at x y).",
                path.display()
            )
        })?;
        let local_x_mm = numeric_at(local_at, 1).with_context(|| {
            format!(
                "KiCad PCB footprint {reference} pad 1 in {} has invalid x coordinate.",
                path.display()
            )
        })?;
        let local_y_mm = numeric_at(local_at, 2).with_context(|| {
            format!(
                "KiCad PCB footprint {reference} pad 1 in {} has invalid y coordinate.",
                path.display()
            )
        })?;
        let marker = PcbPinMarker {
            at: transform_footprint_point(footprint_at, local_x_mm, local_y_mm),
        };
        if pin_1.replace(marker).is_some() {
            bail!(
                "KiCad PCB footprint {reference} in {} has duplicate pad 1 markers.",
                path.display()
            );
        }
    }
    Ok(pin_1)
}

fn parse_entry_direction(
    footprint: &[Sexp],
    reference: &str,
    path: &Path,
) -> Result<Option<PcbEntryDirection>> {
    let mut offset_deg = None;
    for property in list_children(footprint, "property") {
        let Some(name) = string_at(property, 1) else {
            continue;
        };
        if name != "CircuitCI_EntryDirectionOffsetDeg" {
            continue;
        }
        let value = string_at(property, 2)
            .with_context(|| {
                format!(
                    "KiCad PCB footprint {reference} entry-direction property {name} in {} is missing a value.",
                    path.display()
                )
            })?
            .trim()
            .parse::<f64>()
            .with_context(|| {
                format!(
                    "KiCad PCB footprint {reference} entry-direction property {name} in {} is not a number.",
                    path.display()
                )
            })?;
        if !value.is_finite() {
            bail!(
                "KiCad PCB footprint {reference} entry-direction property {name} in {} must be finite.",
                path.display()
            );
        }
        if offset_deg.replace(value).is_some() {
            bail!(
                "KiCad PCB footprint {reference} has duplicate entry-direction property {name} in {}.",
                path.display()
            );
        }
    }
    Ok(offset_deg.map(|offset_deg| PcbEntryDirection { offset_deg }))
}

fn parse_entry_clearance(
    footprint: &[Sexp],
    reference: &str,
    path: &Path,
) -> Result<Option<PcbEntryClearance>> {
    let mut depth_mm = None;
    let mut width_mm = None;
    for property in list_children(footprint, "property") {
        let Some(name) = string_at(property, 1) else {
            continue;
        };
        let target = match name {
            "CircuitCI_EntryClearanceDepthMM" => &mut depth_mm,
            "CircuitCI_EntryClearanceWidthMM" => &mut width_mm,
            _ => continue,
        };
        let value = string_at(property, 2)
            .with_context(|| {
                format!(
                    "KiCad PCB footprint {reference} entry-clearance property {name} in {} is missing a value.",
                    path.display()
                )
            })?
            .trim()
            .parse::<f64>()
            .with_context(|| {
                format!(
                    "KiCad PCB footprint {reference} entry-clearance property {name} in {} is not a number.",
                    path.display()
                )
            })?;
        if !value.is_finite() {
            bail!(
                "KiCad PCB footprint {reference} entry-clearance property {name} in {} must be finite.",
                path.display()
            );
        }
        if value <= 0.0 {
            bail!(
                "KiCad PCB footprint {reference} entry-clearance property {name} in {} must be greater than zero.",
                path.display()
            );
        }
        if target.replace(value).is_some() {
            bail!(
                "KiCad PCB footprint {reference} has duplicate entry-clearance property {name} in {}.",
                path.display()
            );
        }
    }
    if depth_mm.is_none() && width_mm.is_none() {
        Ok(None)
    } else {
        Ok(Some(PcbEntryClearance { depth_mm, width_mm }))
    }
}

fn parse_entry_aperture(
    footprint: &[Sexp],
    reference: &str,
    path: &Path,
) -> Result<Option<PcbEntryAperture>> {
    let mut aperture = PcbEntryAperture::default();
    let mut saw_aperture_property = false;
    for property in list_children(footprint, "property") {
        let Some(name) = string_at(property, 1) else {
            continue;
        };
        let field = match name {
            "CircuitCI_EntryApertureFrontOffsetMM" => EntryApertureField::FrontOffset,
            "CircuitCI_EntryApertureLateralOffsetMM" => EntryApertureField::LateralOffset,
            "CircuitCI_EntryApertureWidthMM" => EntryApertureField::Width,
            _ => continue,
        };
        saw_aperture_property = true;
        let value = string_at(property, 2)
            .with_context(|| {
                format!(
                    "KiCad PCB footprint {reference} aperture property {name} in {} is missing a value.",
                    path.display()
                )
            })?
            .trim()
            .parse::<f64>()
            .with_context(|| {
                format!(
                    "KiCad PCB footprint {reference} aperture property {name} in {} is not a number.",
                    path.display()
                )
            })?;
        if !value.is_finite() {
            bail!(
                "KiCad PCB footprint {reference} aperture property {name} in {} must be finite.",
                path.display()
            );
        }
        match field {
            EntryApertureField::FrontOffset => set_unique_aperture_value(
                &mut aperture.front_offset_mm,
                value,
                reference,
                name,
                path,
            )?,
            EntryApertureField::LateralOffset => set_unique_aperture_value(
                &mut aperture.lateral_offset_mm,
                value,
                reference,
                name,
                path,
            )?,
            EntryApertureField::Width => {
                if value <= 0.0 {
                    bail!(
                        "KiCad PCB footprint {reference} aperture property {name} in {} must be greater than zero.",
                        path.display()
                    );
                }
                set_unique_aperture_value(&mut aperture.width_mm, value, reference, name, path)?;
            }
        }
    }
    Ok(saw_aperture_property.then_some(aperture))
}

fn parse_footprint_properties(
    footprint: &[Sexp],
    reference: &str,
    path: &Path,
) -> Result<Vec<PcbFootprintProperty>> {
    let mut properties = Vec::new();
    if let Some(identifier) = string_at(footprint, 1)
        .map(str::trim)
        .filter(|identifier| !identifier.is_empty())
    {
        properties.push(PcbFootprintProperty {
            name: "Footprint".to_string(),
            value: identifier.to_string(),
            source: "kicad_footprint_identifier",
        });
    }
    for property in list_children(footprint, "property") {
        let name = string_at(property, 1)
            .with_context(|| {
                format!(
                    "KiCad PCB footprint {reference} property in {} is missing a name.",
                    path.display()
                )
            })?
            .trim();
        if name.is_empty() {
            bail!(
                "KiCad PCB footprint {reference} property in {} has an empty name.",
                path.display()
            );
        }
        let value = string_at(property, 2).with_context(|| {
            format!(
                "KiCad PCB footprint {reference} property {name} in {} is missing a value.",
                path.display()
            )
        })?;
        properties.push(PcbFootprintProperty {
            name: name.to_string(),
            value: value.to_string(),
            source: "kicad_footprint_property",
        });
    }
    Ok(properties)
}

#[derive(Clone, Copy)]
enum EntryApertureField {
    FrontOffset,
    LateralOffset,
    Width,
}

fn set_unique_aperture_value(
    target: &mut Option<f64>,
    value: f64,
    reference: &str,
    name: &str,
    path: &Path,
) -> Result<()> {
    if target.replace(value).is_some() {
        bail!(
            "KiCad PCB footprint {reference} has duplicate aperture property {name} in {}.",
            path.display()
        );
    }
    Ok(())
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
