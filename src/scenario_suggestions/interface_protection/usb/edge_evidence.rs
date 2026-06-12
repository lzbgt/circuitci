use super::super::super::{
    SuggestedBoardEdge, SuggestedComponentClearance, SuggestedFootprint, SuggestedFootprintArc,
    SuggestedFootprintCircle, SuggestedFootprintPolygon, SuggestedFootprintRectangle,
    SuggestedFootprintSegment, SuggestedPoint, SuggestedUsbEntryClearance,
    SuggestedUsbEntryObstruction,
};
use super::super::component_placement;
use crate::board_ir::{
    ComponentPlacement, LayoutFootprint, LayoutFootprintArc, LayoutFootprintCircle,
    LayoutFootprintPolygon, LayoutFootprintRectangle, LayoutFootprintSegment, LayoutPoint,
    LayoutSegment,
};
use crate::library::BoundBoard;
use crate::library::UsbConnector;

pub(super) fn suggested_footprint(
    bound: &BoundBoard<'_>,
    component_id: &str,
) -> Option<SuggestedFootprint> {
    let footprint = bound.project.board.layout.footprints.get(component_id)?;
    suggested_footprint_from_layout(footprint)
}

fn suggested_footprint_from_layout(footprint: &LayoutFootprint) -> Option<SuggestedFootprint> {
    let segments = footprint
        .segments
        .iter()
        .map(|segment| SuggestedFootprintSegment {
            start: suggested_point(&segment.start),
            end: suggested_point(&segment.end),
            layer: segment.layer.clone(),
            kind: segment.kind.clone(),
        })
        .collect::<Vec<_>>();
    let rectangles = footprint
        .rectangles
        .iter()
        .map(|rectangle| SuggestedFootprintRectangle {
            start: suggested_point(&rectangle.start),
            end: suggested_point(&rectangle.end),
            layer: rectangle.layer.clone(),
            kind: rectangle.kind.clone(),
        })
        .collect::<Vec<_>>();
    let polygons = footprint
        .polygons
        .iter()
        .map(|polygon| SuggestedFootprintPolygon {
            points: polygon.points.iter().map(suggested_point).collect(),
            layer: polygon.layer.clone(),
            kind: polygon.kind.clone(),
        })
        .collect::<Vec<_>>();
    let circles = footprint
        .circles
        .iter()
        .map(|circle| SuggestedFootprintCircle {
            center: suggested_point(&circle.center),
            end: suggested_point(&circle.end),
            layer: circle.layer.clone(),
            kind: circle.kind.clone(),
        })
        .collect::<Vec<_>>();
    let arcs = footprint
        .arcs
        .iter()
        .map(|arc| SuggestedFootprintArc {
            start: suggested_point(&arc.start),
            mid: suggested_point(&arc.mid),
            end: suggested_point(&arc.end),
            layer: arc.layer.clone(),
            kind: arc.kind.clone(),
        })
        .collect::<Vec<_>>();
    (!segments.is_empty()
        || !rectangles.is_empty()
        || !polygons.is_empty()
        || !circles.is_empty()
        || !arcs.is_empty())
    .then_some(SuggestedFootprint {
        segments,
        rectangles,
        polygons,
        circles,
        arcs,
    })
}

pub(super) fn nearest_board_edge_evidence(
    bound: &BoundBoard<'_>,
    component_id: &str,
    connector: &UsbConnector,
) -> Option<SuggestedBoardEdge> {
    let placement = component_placement(bound, component_id)?;
    let rotation_deg = placement.rotation_deg?;
    let centroid = outline_centroid(&bound.project.board.layout.outline.segments)?;
    let edge = bound
        .project
        .board
        .layout
        .outline
        .segments
        .iter()
        .filter(|segment| outline_segment_is_entry_candidate(segment))
        .map(|segment| {
            let distance =
                connector_to_board_edge_distance(bound, component_id, placement, segment);
            (segment, distance)
        })
        .min_by(|left, right| left.1.distance_mm.total_cmp(&right.1.distance_mm))?;
    let edge_angle_deg = segment_angle_deg(edge.0);
    let outward_normal_deg = outward_normal_deg(edge.0, &centroid, edge_angle_deg);
    let entry_direction_offset_deg = connector.entry_direction_offset_deg;
    let expected_connector_rotation_deg =
        normalize_rotation_deg(outward_normal_deg - entry_direction_offset_deg.unwrap_or(0.0));
    Some(SuggestedBoardEdge {
        start: suggested_point(&edge.0.start),
        end: suggested_point(&edge.0.end),
        layer: edge.0.layer.clone(),
        source_primitive: edge.0.source_primitive.clone(),
        source_primitive_index: edge.0.source_primitive_index,
        sample_index: edge.0.sample_index,
        sample_count: edge.0.sample_count,
        contour_index: edge.0.contour_index,
        boundary_role: edge.0.boundary_role.clone(),
        distance_to_connector_mm: edge.1.distance_mm,
        connector_edge_reference: edge.1.reference.label().to_string(),
        footprint_graphic_layer: edge.1.reference.footprint_layer().map(str::to_string),
        footprint_graphic_kind: edge.1.reference.footprint_kind().map(str::to_string),
        connector_body_overhang_mm: edge.1.body_overhang_mm,
        edge_angle_deg,
        outward_normal_deg,
        expected_connector_rotation_deg: Some(expected_connector_rotation_deg),
        connector_entry_direction_offset_deg: entry_direction_offset_deg,
        connector_rotation_error_deg: angular_error_deg(
            rotation_deg,
            expected_connector_rotation_deg,
        ),
    })
}

pub(super) fn nearest_component_clearance_evidence(
    bound: &BoundBoard<'_>,
    connector_id: &str,
) -> Option<SuggestedComponentClearance> {
    let connector_primitives = mechanical_clearance_primitives(bound, connector_id, false);
    if connector_primitives.is_empty() {
        return None;
    }
    let mut nearest: Option<ComponentClearanceCandidate<'_>> = None;
    for component_id in bound.project.board.components.keys() {
        if component_id == connector_id {
            continue;
        }
        let component_primitives = mechanical_clearance_primitives(bound, component_id, true);
        if component_primitives.is_empty() {
            continue;
        }
        let Some((clearance_mm, connector_reference, component_reference)) =
            nearest_clearance_between(&connector_primitives, &component_primitives)
        else {
            continue;
        };
        if nearest
            .as_ref()
            .is_none_or(|candidate| clearance_mm < candidate.clearance_mm)
        {
            nearest = Some(ComponentClearanceCandidate {
                component_id,
                clearance_mm,
                connector_reference,
                component_reference,
            });
        }
    }
    nearest.map(|candidate| SuggestedComponentClearance {
        component: candidate.component_id.to_string(),
        clearance_mm: candidate.clearance_mm,
        connector_clearance_reference: candidate.connector_reference.label().to_string(),
        connector_footprint_graphic_layer: candidate
            .connector_reference
            .footprint_layer()
            .map(str::to_string),
        connector_footprint_graphic_kind: candidate
            .connector_reference
            .footprint_kind()
            .map(str::to_string),
        component_clearance_reference: candidate.component_reference.label().to_string(),
        component_footprint_graphic_layer: candidate
            .component_reference
            .footprint_layer()
            .map(str::to_string),
        component_footprint_graphic_kind: candidate
            .component_reference
            .footprint_kind()
            .map(str::to_string),
    })
}

pub(super) fn entry_clearance_evidence(
    bound: &BoundBoard<'_>,
    connector_id: &str,
    connector: &UsbConnector,
    entry_direction_deg: f64,
    entry_direction_source: &str,
    entry_direction_offset_deg: Option<f64>,
) -> Option<SuggestedUsbEntryClearance> {
    if !entry_direction_deg.is_finite() {
        return None;
    }
    let placement = component_placement(bound, connector_id)?;
    let connector_primitives = mechanical_clearance_primitives(bound, connector_id, false);
    if connector_primitives.is_empty() {
        return None;
    }
    let connector_front_projection_mm = connector_primitives
        .iter()
        .flat_map(clearance_primitive_points)
        .map(|point| direction_projection(&point, entry_direction_deg))
        .max_by(|left, right| left.total_cmp(right))?;
    let aperture = entry_aperture_evidence(
        placement,
        connector,
        entry_direction_deg,
        connector_front_projection_mm,
    )?;
    let mut nearest: Option<EntryObstructionCandidate<'_>> = None;
    for component_id in bound.project.board.components.keys() {
        if component_id == connector_id {
            continue;
        }
        for primitive in mechanical_clearance_primitives(bound, component_id, true) {
            let Some((depth_mm, lateral_offset_mm, reference)) = entry_obstruction_candidate(
                &primitive,
                entry_direction_deg,
                aperture.front_projection_mm,
                aperture.center_lateral_projection_mm,
            ) else {
                continue;
            };
            let candidate = EntryObstructionCandidate {
                component_id,
                depth_mm,
                lateral_offset_mm,
                reference,
            };
            if nearest.as_ref().is_none_or(|current| {
                depth_mm < current.depth_mm
                    || ((depth_mm - current.depth_mm).abs() <= f64::EPSILON
                        && lateral_offset_mm.abs() < current.lateral_offset_mm.abs())
            }) {
                nearest = Some(candidate);
            }
        }
    }
    Some(SuggestedUsbEntryClearance {
        entry_direction_deg,
        entry_direction_source: entry_direction_source.to_string(),
        entry_direction_offset_deg,
        entry_aperture_source: aperture.source.to_string(),
        connector_front_projection_mm,
        entry_aperture_front_projection_mm: aperture.front_projection_mm,
        entry_aperture_center_lateral_projection_mm: aperture.center_lateral_projection_mm,
        entry_aperture_front_offset_mm: aperture.front_offset_mm,
        entry_aperture_lateral_offset_mm: aperture.lateral_offset_mm,
        entry_aperture_width_mm: aperture.aperture_width_mm,
        model_min_cable_entry_clearance_width_mm: aperture.aperture_width_mm,
        nearest_obstruction: nearest.map(|candidate| SuggestedUsbEntryObstruction {
            component: candidate.component_id.to_string(),
            obstruction_depth_mm: candidate.depth_mm,
            obstruction_lateral_offset_mm: candidate.lateral_offset_mm,
            obstruction_reference: candidate.reference.label().to_string(),
            obstruction_footprint_graphic_layer: candidate
                .reference
                .footprint_layer()
                .map(str::to_string),
            obstruction_footprint_graphic_kind: candidate
                .reference
                .footprint_kind()
                .map(str::to_string),
        }),
    })
}

struct EntryApertureEvidence {
    source: &'static str,
    front_projection_mm: f64,
    center_lateral_projection_mm: f64,
    front_offset_mm: Option<f64>,
    lateral_offset_mm: Option<f64>,
    aperture_width_mm: Option<f64>,
}

fn entry_aperture_evidence(
    placement: &ComponentPlacement,
    connector: &UsbConnector,
    entry_direction_deg: f64,
    connector_front_projection_mm: f64,
) -> Option<EntryApertureEvidence> {
    let front_offset_mm = finite_optional(connector.entry_aperture_front_offset_mm)?;
    let lateral_offset_mm = finite_optional(connector.entry_aperture_lateral_offset_mm)?;
    let aperture_width_mm = finite_optional(connector.entry_aperture_width_mm)?;
    if aperture_width_mm.is_some_and(|width_mm| width_mm <= 0.0) {
        return None;
    }
    let has_model_aperture =
        front_offset_mm.is_some() || lateral_offset_mm.is_some() || aperture_width_mm.is_some();
    let center_lateral_projection_mm = placement_lateral_projection(placement, entry_direction_deg)
        + lateral_offset_mm.unwrap_or(0.0);
    Some(EntryApertureEvidence {
        source: if has_model_aperture {
            "component_model_aperture"
        } else {
            "footprint_front"
        },
        front_projection_mm: connector_front_projection_mm + front_offset_mm.unwrap_or(0.0),
        center_lateral_projection_mm,
        front_offset_mm,
        lateral_offset_mm,
        aperture_width_mm,
    })
}

fn finite_optional(value: Option<f64>) -> Option<Option<f64>> {
    match value {
        Some(value) if value.is_finite() => Some(Some(value)),
        Some(_) => None,
        None => Some(None),
    }
}

struct BoardEdgeConnectorDistance<'a> {
    distance_mm: f64,
    reference: BoardEdgeConnectorReference<'a>,
    body_overhang_mm: Option<f64>,
}

struct ComponentClearanceCandidate<'a> {
    component_id: &'a str,
    clearance_mm: f64,
    connector_reference: ComponentClearanceReference<'a>,
    component_reference: ComponentClearanceReference<'a>,
}

struct EntryObstructionCandidate<'a> {
    component_id: &'a str,
    depth_mm: f64,
    lateral_offset_mm: f64,
    reference: ComponentClearanceReference<'a>,
}

#[derive(Clone, Copy)]
enum BoardEdgeConnectorReference<'a> {
    PlacementCenter,
    FootprintSegment { layer: &'a str, kind: &'a str },
    FootprintRectangle { layer: &'a str, kind: &'a str },
    FootprintPolygon { layer: &'a str, kind: &'a str },
    FootprintCircle { layer: &'a str, kind: &'a str },
    FootprintArc { layer: &'a str, kind: &'a str },
}

#[derive(Clone, Copy)]
enum ComponentClearanceReference<'a> {
    PlacementCenter,
    FootprintSegment { layer: &'a str, kind: &'a str },
    FootprintRectangle { layer: &'a str, kind: &'a str },
    FootprintPolygon { layer: &'a str, kind: &'a str },
    FootprintCircle { layer: &'a str, kind: &'a str },
    FootprintArc { layer: &'a str, kind: &'a str },
}

impl ComponentClearanceReference<'_> {
    fn label(&self) -> &'static str {
        match self {
            ComponentClearanceReference::PlacementCenter => "placement_center",
            ComponentClearanceReference::FootprintSegment { .. } => "footprint_segment",
            ComponentClearanceReference::FootprintRectangle { .. } => "footprint_rectangle",
            ComponentClearanceReference::FootprintPolygon { .. } => "footprint_polygon",
            ComponentClearanceReference::FootprintCircle { .. } => "footprint_circle",
            ComponentClearanceReference::FootprintArc { .. } => "footprint_arc",
        }
    }

    fn footprint_layer(&self) -> Option<&str> {
        match self {
            ComponentClearanceReference::PlacementCenter => None,
            ComponentClearanceReference::FootprintSegment { layer, .. }
            | ComponentClearanceReference::FootprintRectangle { layer, .. }
            | ComponentClearanceReference::FootprintPolygon { layer, .. }
            | ComponentClearanceReference::FootprintCircle { layer, .. }
            | ComponentClearanceReference::FootprintArc { layer, .. } => Some(layer),
        }
    }

    fn footprint_kind(&self) -> Option<&str> {
        match self {
            ComponentClearanceReference::PlacementCenter => None,
            ComponentClearanceReference::FootprintSegment { kind, .. }
            | ComponentClearanceReference::FootprintRectangle { kind, .. }
            | ComponentClearanceReference::FootprintPolygon { kind, .. }
            | ComponentClearanceReference::FootprintCircle { kind, .. }
            | ComponentClearanceReference::FootprintArc { kind, .. } => Some(kind),
        }
    }
}

impl BoardEdgeConnectorReference<'_> {
    fn label(&self) -> &'static str {
        match self {
            BoardEdgeConnectorReference::PlacementCenter => "placement_center",
            BoardEdgeConnectorReference::FootprintSegment { .. } => "footprint_segment",
            BoardEdgeConnectorReference::FootprintRectangle { .. } => "footprint_rectangle",
            BoardEdgeConnectorReference::FootprintPolygon { .. } => "footprint_polygon",
            BoardEdgeConnectorReference::FootprintCircle { .. } => "footprint_circle",
            BoardEdgeConnectorReference::FootprintArc { .. } => "footprint_arc",
        }
    }

    fn footprint_layer(&self) -> Option<&str> {
        match self {
            BoardEdgeConnectorReference::PlacementCenter => None,
            BoardEdgeConnectorReference::FootprintSegment { layer, .. }
            | BoardEdgeConnectorReference::FootprintRectangle { layer, .. }
            | BoardEdgeConnectorReference::FootprintPolygon { layer, .. }
            | BoardEdgeConnectorReference::FootprintCircle { layer, .. }
            | BoardEdgeConnectorReference::FootprintArc { layer, .. } => Some(layer),
        }
    }

    fn footprint_kind(&self) -> Option<&str> {
        match self {
            BoardEdgeConnectorReference::PlacementCenter => None,
            BoardEdgeConnectorReference::FootprintSegment { kind, .. }
            | BoardEdgeConnectorReference::FootprintRectangle { kind, .. }
            | BoardEdgeConnectorReference::FootprintPolygon { kind, .. }
            | BoardEdgeConnectorReference::FootprintCircle { kind, .. }
            | BoardEdgeConnectorReference::FootprintArc { kind, .. } => Some(kind),
        }
    }
}

fn connector_to_board_edge_distance<'a>(
    bound: &'a BoundBoard<'_>,
    component_id: &'a str,
    placement: &ComponentPlacement,
    edge: &LayoutSegment,
) -> BoardEdgeConnectorDistance<'a> {
    let mut best = BoardEdgeConnectorDistance {
        distance_mm: placement_to_segment_distance_mm(placement, edge),
        reference: BoardEdgeConnectorReference::PlacementCenter,
        body_overhang_mm: None,
    };
    let centroid = outline_centroid(&bound.project.board.layout.outline.segments);
    let outward_normal_deg = centroid.as_ref().map(|centroid| {
        let edge_angle_deg = segment_angle_deg(edge);
        outward_normal_deg(edge, centroid, edge_angle_deg)
    });
    let Some(footprint) = bound.project.board.layout.footprints.get(component_id) else {
        return best;
    };
    for segment in &footprint.segments {
        if !mechanical_footprint_kind(&segment.kind) {
            continue;
        }
        let Some(distance_mm) = footprint_segment_to_edge_distance_mm(segment, edge) else {
            continue;
        };
        if distance_mm < best.distance_mm {
            best = BoardEdgeConnectorDistance {
                distance_mm,
                reference: BoardEdgeConnectorReference::FootprintSegment {
                    layer: &segment.layer,
                    kind: &segment.kind,
                },
                body_overhang_mm: outward_normal_deg.map(|normal| {
                    body_overhang_from_points([&segment.start, &segment.end], edge, normal)
                }),
            };
        }
    }
    for rectangle in &footprint.rectangles {
        if !mechanical_footprint_kind(&rectangle.kind) {
            continue;
        }
        let Some(distance_mm) = footprint_rectangle_to_edge_distance_mm(rectangle, edge) else {
            continue;
        };
        if distance_mm < best.distance_mm {
            let body_overhang_mm = outward_normal_deg.and_then(|normal| {
                rectangle_corners(rectangle)
                    .map(|corners| body_overhang_from_points(corners.iter(), edge, normal))
            });
            best = BoardEdgeConnectorDistance {
                distance_mm,
                reference: BoardEdgeConnectorReference::FootprintRectangle {
                    layer: &rectangle.layer,
                    kind: &rectangle.kind,
                },
                body_overhang_mm,
            };
        }
    }
    for polygon in &footprint.polygons {
        if !mechanical_footprint_kind(&polygon.kind) {
            continue;
        }
        let Some(distance_mm) = footprint_polygon_to_edge_distance_mm(polygon, edge) else {
            continue;
        };
        if distance_mm <= best.distance_mm {
            best = BoardEdgeConnectorDistance {
                distance_mm,
                reference: BoardEdgeConnectorReference::FootprintPolygon {
                    layer: &polygon.layer,
                    kind: &polygon.kind,
                },
                body_overhang_mm: outward_normal_deg
                    .map(|normal| body_overhang_from_points(polygon.points.iter(), edge, normal)),
            };
        }
    }
    for circle in &footprint.circles {
        if !mechanical_footprint_kind(&circle.kind) {
            continue;
        }
        let Some(points) = footprint_circle_points(circle) else {
            continue;
        };
        let Some(distance_mm) = closed_polyline_to_edge_distance_mm(&points, edge) else {
            continue;
        };
        if distance_mm <= best.distance_mm {
            best = BoardEdgeConnectorDistance {
                distance_mm,
                reference: BoardEdgeConnectorReference::FootprintCircle {
                    layer: &circle.layer,
                    kind: &circle.kind,
                },
                body_overhang_mm: outward_normal_deg
                    .map(|normal| body_overhang_from_points(points.iter(), edge, normal)),
            };
        }
    }
    for arc in &footprint.arcs {
        if !mechanical_footprint_kind(&arc.kind) {
            continue;
        }
        let Some(points) = footprint_arc_points(arc) else {
            continue;
        };
        let Some(distance_mm) = open_polyline_to_edge_distance_mm(&points, edge) else {
            continue;
        };
        if distance_mm <= best.distance_mm {
            best = BoardEdgeConnectorDistance {
                distance_mm,
                reference: BoardEdgeConnectorReference::FootprintArc {
                    layer: &arc.layer,
                    kind: &arc.kind,
                },
                body_overhang_mm: outward_normal_deg
                    .map(|normal| body_overhang_from_points(points.iter(), edge, normal)),
            };
        }
    }
    best
}

#[derive(Clone)]
enum ClearancePrimitive<'a> {
    Point {
        point: LayoutPoint,
        reference: ComponentClearanceReference<'a>,
    },
    Segment {
        start: LayoutPoint,
        end: LayoutPoint,
        reference: ComponentClearanceReference<'a>,
    },
}

impl<'a> ClearancePrimitive<'a> {
    fn reference(&self) -> ComponentClearanceReference<'a> {
        match self {
            ClearancePrimitive::Point { reference, .. }
            | ClearancePrimitive::Segment { reference, .. } => *reference,
        }
    }
}

fn mechanical_clearance_primitives<'a>(
    bound: &'a BoundBoard<'_>,
    component_id: &'a str,
    include_placement_fallback: bool,
) -> Vec<ClearancePrimitive<'a>> {
    let mut primitives = Vec::new();
    if let Some(footprint) = bound.project.board.layout.footprints.get(component_id) {
        for segment in &footprint.segments {
            if !mechanical_footprint_kind(&segment.kind)
                || !point_is_finite(&segment.start)
                || !point_is_finite(&segment.end)
                || point_distance_mm(&segment.start, &segment.end) <= f64::EPSILON
            {
                continue;
            }
            primitives.push(ClearancePrimitive::Segment {
                start: segment.start.clone(),
                end: segment.end.clone(),
                reference: ComponentClearanceReference::FootprintSegment {
                    layer: &segment.layer,
                    kind: &segment.kind,
                },
            });
        }
        for rectangle in &footprint.rectangles {
            if !mechanical_footprint_kind(&rectangle.kind) {
                continue;
            }
            let Some(corners) = rectangle_corners(rectangle) else {
                continue;
            };
            push_closed_clearance_polyline(
                &mut primitives,
                &corners,
                ComponentClearanceReference::FootprintRectangle {
                    layer: &rectangle.layer,
                    kind: &rectangle.kind,
                },
            );
        }
        for polygon in &footprint.polygons {
            if !mechanical_footprint_kind(&polygon.kind)
                || polygon.points.len() < 3
                || polygon.points.iter().any(|point| !point_is_finite(point))
            {
                continue;
            }
            push_closed_clearance_polyline(
                &mut primitives,
                &polygon.points,
                ComponentClearanceReference::FootprintPolygon {
                    layer: &polygon.layer,
                    kind: &polygon.kind,
                },
            );
        }
        for circle in &footprint.circles {
            if !mechanical_footprint_kind(&circle.kind) {
                continue;
            }
            let Some(points) = footprint_circle_points(circle) else {
                continue;
            };
            push_closed_clearance_polyline(
                &mut primitives,
                &points,
                ComponentClearanceReference::FootprintCircle {
                    layer: &circle.layer,
                    kind: &circle.kind,
                },
            );
        }
        for arc in &footprint.arcs {
            if !mechanical_footprint_kind(&arc.kind) {
                continue;
            }
            let Some(points) = footprint_arc_points(arc) else {
                continue;
            };
            push_open_clearance_polyline(
                &mut primitives,
                &points,
                ComponentClearanceReference::FootprintArc {
                    layer: &arc.layer,
                    kind: &arc.kind,
                },
            );
        }
    }
    if primitives.is_empty()
        && include_placement_fallback
        && let Some(placement) = component_placement(bound, component_id)
    {
        primitives.push(ClearancePrimitive::Point {
            point: LayoutPoint {
                x_mm: placement.x_mm,
                y_mm: placement.y_mm,
            },
            reference: ComponentClearanceReference::PlacementCenter,
        });
    }
    primitives
}

fn push_closed_clearance_polyline<'a>(
    primitives: &mut Vec<ClearancePrimitive<'a>>,
    points: &[LayoutPoint],
    reference: ComponentClearanceReference<'a>,
) {
    if points.len() < 2 {
        return;
    }
    for index in 0..points.len() {
        let next_index = (index + 1) % points.len();
        primitives.push(ClearancePrimitive::Segment {
            start: points[index].clone(),
            end: points[next_index].clone(),
            reference,
        });
    }
}

fn push_open_clearance_polyline<'a>(
    primitives: &mut Vec<ClearancePrimitive<'a>>,
    points: &[LayoutPoint],
    reference: ComponentClearanceReference<'a>,
) {
    if points.len() < 2 {
        return;
    }
    for window in points.windows(2) {
        primitives.push(ClearancePrimitive::Segment {
            start: window[0].clone(),
            end: window[1].clone(),
            reference,
        });
    }
}

fn nearest_clearance_between<'a>(
    connector: &[ClearancePrimitive<'a>],
    other: &[ClearancePrimitive<'a>],
) -> Option<(
    f64,
    ComponentClearanceReference<'a>,
    ComponentClearanceReference<'a>,
)> {
    let mut nearest: Option<(
        f64,
        ComponentClearanceReference<'a>,
        ComponentClearanceReference<'a>,
    )> = None;
    for connector_primitive in connector {
        for other_primitive in other {
            let clearance_mm = clearance_primitive_distance(connector_primitive, other_primitive);
            if nearest
                .as_ref()
                .is_none_or(|(nearest_clearance, _, _)| clearance_mm < *nearest_clearance)
            {
                nearest = Some((
                    clearance_mm,
                    connector_primitive.reference(),
                    other_primitive.reference(),
                ));
            }
        }
    }
    nearest
}

fn clearance_primitive_distance(
    first: &ClearancePrimitive<'_>,
    second: &ClearancePrimitive<'_>,
) -> f64 {
    match (first, second) {
        (
            ClearancePrimitive::Point { point: a, .. },
            ClearancePrimitive::Point { point: b, .. },
        ) => point_distance_mm(a, b),
        (
            ClearancePrimitive::Point { point, .. },
            ClearancePrimitive::Segment { start, end, .. },
        )
        | (
            ClearancePrimitive::Segment { start, end, .. },
            ClearancePrimitive::Point { point, .. },
        ) => point_to_segment_distance_mm(
            point.x_mm, point.y_mm, start.x_mm, start.y_mm, end.x_mm, end.y_mm,
        ),
        (
            ClearancePrimitive::Segment {
                start: a_start,
                end: a_end,
                ..
            },
            ClearancePrimitive::Segment {
                start: b_start,
                end: b_end,
                ..
            },
        ) => segment_to_segment_distance_mm(a_start, a_end, b_start, b_end),
    }
}

fn clearance_primitive_points(primitive: &ClearancePrimitive<'_>) -> Vec<LayoutPoint> {
    match primitive {
        ClearancePrimitive::Point { point, .. } => vec![point.clone()],
        ClearancePrimitive::Segment { start, end, .. } => vec![start.clone(), end.clone()],
    }
}

fn entry_obstruction_candidate<'a>(
    primitive: &ClearancePrimitive<'a>,
    entry_direction_deg: f64,
    connector_front_projection_mm: f64,
    center_lateral_projection_mm: f64,
) -> Option<(f64, f64, ComponentClearanceReference<'a>)> {
    match primitive {
        ClearancePrimitive::Point { point, reference } => {
            let (forward, lateral) = entry_projection(point, entry_direction_deg);
            (forward >= connector_front_projection_mm - f64::EPSILON).then_some((
                (forward - connector_front_projection_mm).max(0.0),
                lateral - center_lateral_projection_mm,
                *reference,
            ))
        }
        ClearancePrimitive::Segment {
            start,
            end,
            reference,
        } => {
            let (start_forward, start_lateral) = entry_projection(start, entry_direction_deg);
            let (end_forward, end_lateral) = entry_projection(end, entry_direction_deg);
            let max_forward = start_forward.max(end_forward);
            if max_forward < connector_front_projection_mm - f64::EPSILON {
                return None;
            }
            if (start_forward - end_forward).abs() <= f64::EPSILON {
                let lateral = if start_lateral.abs() <= end_lateral.abs() {
                    start_lateral
                } else {
                    end_lateral
                };
                return Some((
                    (start_forward - connector_front_projection_mm).max(0.0),
                    lateral - center_lateral_projection_mm,
                    *reference,
                ));
            }
            if (start_forward <= connector_front_projection_mm
                && end_forward >= connector_front_projection_mm)
                || (end_forward <= connector_front_projection_mm
                    && start_forward >= connector_front_projection_mm)
            {
                let t = ((connector_front_projection_mm - start_forward)
                    / (end_forward - start_forward))
                    .clamp(0.0, 1.0);
                let lateral = start_lateral + (end_lateral - start_lateral) * t;
                return Some((0.0, lateral - center_lateral_projection_mm, *reference));
            }
            let (forward, lateral) = if start_forward < end_forward {
                (start_forward, start_lateral)
            } else {
                (end_forward, end_lateral)
            };
            Some((
                (forward - connector_front_projection_mm).max(0.0),
                lateral - center_lateral_projection_mm,
                *reference,
            ))
        }
    }
}

fn entry_projection(point: &LayoutPoint, entry_direction_deg: f64) -> (f64, f64) {
    let radians = entry_direction_deg.to_radians();
    let normal_x = -radians.sin();
    let normal_y = radians.cos();
    (
        point.x_mm * radians.cos() + point.y_mm * radians.sin(),
        point.x_mm * normal_x + point.y_mm * normal_y,
    )
}

fn direction_projection(point: &LayoutPoint, entry_direction_deg: f64) -> f64 {
    let radians = entry_direction_deg.to_radians();
    point.x_mm * radians.cos() + point.y_mm * radians.sin()
}

fn placement_lateral_projection(placement: &ComponentPlacement, entry_direction_deg: f64) -> f64 {
    let radians = entry_direction_deg.to_radians();
    let normal_x = -radians.sin();
    let normal_y = radians.cos();
    placement.x_mm * normal_x + placement.y_mm * normal_y
}

fn mechanical_footprint_kind(kind: &str) -> bool {
    matches!(kind, "fabrication" | "courtyard")
}

fn suggested_point(point: &LayoutPoint) -> SuggestedPoint {
    SuggestedPoint {
        x_mm: point.x_mm,
        y_mm: point.y_mm,
    }
}

fn outline_centroid(segments: &[LayoutSegment]) -> Option<LayoutPoint> {
    let mut count = 0.0;
    let mut x_sum = 0.0;
    let mut y_sum = 0.0;
    for segment in segments {
        if !outline_segment_is_entry_candidate(segment) {
            continue;
        }
        x_sum += segment.start.x_mm + segment.end.x_mm;
        y_sum += segment.start.y_mm + segment.end.y_mm;
        count += 2.0;
    }
    (count > 0.0).then_some(LayoutPoint {
        x_mm: x_sum / count,
        y_mm: y_sum / count,
    })
}

fn placement_to_segment_distance_mm(
    placement: &ComponentPlacement,
    segment: &LayoutSegment,
) -> f64 {
    point_to_segment_distance_mm(
        placement.x_mm,
        placement.y_mm,
        segment.start.x_mm,
        segment.start.y_mm,
        segment.end.x_mm,
        segment.end.y_mm,
    )
}

fn footprint_segment_to_edge_distance_mm(
    segment: &LayoutFootprintSegment,
    edge: &LayoutSegment,
) -> Option<f64> {
    if !point_is_finite(&segment.start)
        || !point_is_finite(&segment.end)
        || point_distance_mm(&segment.start, &segment.end) <= f64::EPSILON
    {
        return None;
    }
    Some(segment_to_segment_distance_mm(
        &segment.start,
        &segment.end,
        &edge.start,
        &edge.end,
    ))
}

fn footprint_rectangle_to_edge_distance_mm(
    rectangle: &LayoutFootprintRectangle,
    edge: &LayoutSegment,
) -> Option<f64> {
    let corners = rectangle_corners(rectangle)?;
    Some(
        (0..corners.len())
            .map(|index| {
                let next_index = (index + 1) % corners.len();
                segment_to_segment_distance_mm(
                    &corners[index],
                    &corners[next_index],
                    &edge.start,
                    &edge.end,
                )
            })
            .fold(f64::INFINITY, f64::min),
    )
}

fn rectangle_corners(rectangle: &LayoutFootprintRectangle) -> Option<[LayoutPoint; 4]> {
    if !point_is_finite(&rectangle.start) || !point_is_finite(&rectangle.end) {
        return None;
    }
    let min_x = rectangle.start.x_mm.min(rectangle.end.x_mm);
    let max_x = rectangle.start.x_mm.max(rectangle.end.x_mm);
    let min_y = rectangle.start.y_mm.min(rectangle.end.y_mm);
    let max_y = rectangle.start.y_mm.max(rectangle.end.y_mm);
    if (max_x - min_x).abs() <= f64::EPSILON || (max_y - min_y).abs() <= f64::EPSILON {
        return None;
    }
    Some([
        LayoutPoint {
            x_mm: min_x,
            y_mm: min_y,
        },
        LayoutPoint {
            x_mm: max_x,
            y_mm: min_y,
        },
        LayoutPoint {
            x_mm: max_x,
            y_mm: max_y,
        },
        LayoutPoint {
            x_mm: min_x,
            y_mm: max_y,
        },
    ])
}

fn footprint_polygon_to_edge_distance_mm(
    polygon: &LayoutFootprintPolygon,
    edge: &LayoutSegment,
) -> Option<f64> {
    if polygon.points.len() < 3 || polygon.points.iter().any(|point| !point_is_finite(point)) {
        return None;
    }
    closed_polyline_to_edge_distance_mm(&polygon.points, edge)
}

fn footprint_circle_points(circle: &LayoutFootprintCircle) -> Option<Vec<LayoutPoint>> {
    if !point_is_finite(&circle.center) || !point_is_finite(&circle.end) {
        return None;
    }
    let radius = point_distance_mm(&circle.center, &circle.end);
    if radius <= f64::EPSILON {
        return None;
    }
    let segments = 32;
    Some(
        (0..segments)
            .map(|index| {
                let angle = 2.0 * std::f64::consts::PI * (index as f64) / (segments as f64);
                LayoutPoint {
                    x_mm: circle.center.x_mm + radius * angle.cos(),
                    y_mm: circle.center.y_mm + radius * angle.sin(),
                }
            })
            .collect(),
    )
}

fn footprint_arc_points(arc: &LayoutFootprintArc) -> Option<Vec<LayoutPoint>> {
    if !point_is_finite(&arc.start) || !point_is_finite(&arc.mid) || !point_is_finite(&arc.end) {
        return None;
    }
    let center = arc_center(&arc.start, &arc.mid, &arc.end)?;
    let radius = point_distance_mm(&center, &arc.start);
    if radius <= f64::EPSILON {
        return None;
    }
    let start_angle = (arc.start.y_mm - center.y_mm).atan2(arc.start.x_mm - center.x_mm);
    let mid_angle = (arc.mid.y_mm - center.y_mm).atan2(arc.mid.x_mm - center.x_mm);
    let end_angle = (arc.end.y_mm - center.y_mm).atan2(arc.end.x_mm - center.x_mm);
    let ccw = angle_on_ccw_arc(start_angle, mid_angle, end_angle);
    let delta = if ccw {
        (end_angle - start_angle).rem_euclid(2.0 * std::f64::consts::PI)
    } else {
        -((start_angle - end_angle).rem_euclid(2.0 * std::f64::consts::PI))
    };
    let segments = 16;
    Some(
        (0..=segments)
            .map(|index| {
                let angle = start_angle + delta * (index as f64) / (segments as f64);
                LayoutPoint {
                    x_mm: center.x_mm + radius * angle.cos(),
                    y_mm: center.y_mm + radius * angle.sin(),
                }
            })
            .collect(),
    )
}

fn arc_center(start: &LayoutPoint, mid: &LayoutPoint, end: &LayoutPoint) -> Option<LayoutPoint> {
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
    Some(LayoutPoint {
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
    let total = (end_angle - start_angle).rem_euclid(2.0 * std::f64::consts::PI);
    let partial = (test_angle - start_angle).rem_euclid(2.0 * std::f64::consts::PI);
    partial <= total
}

fn closed_polyline_to_edge_distance_mm(
    points: &[LayoutPoint],
    edge: &LayoutSegment,
) -> Option<f64> {
    if points.len() < 2 || points.iter().any(|point| !point_is_finite(point)) {
        return None;
    }
    Some(
        (0..points.len())
            .map(|index| {
                let next_index = (index + 1) % points.len();
                segment_to_segment_distance_mm(
                    &points[index],
                    &points[next_index],
                    &edge.start,
                    &edge.end,
                )
            })
            .fold(f64::INFINITY, f64::min),
    )
}

fn open_polyline_to_edge_distance_mm(points: &[LayoutPoint], edge: &LayoutSegment) -> Option<f64> {
    if points.len() < 2 || points.iter().any(|point| !point_is_finite(point)) {
        return None;
    }
    Some(
        points
            .windows(2)
            .map(|window| {
                segment_to_segment_distance_mm(&window[0], &window[1], &edge.start, &edge.end)
            })
            .fold(f64::INFINITY, f64::min),
    )
}

fn segment_to_segment_distance_mm(
    a_start: &LayoutPoint,
    a_end: &LayoutPoint,
    b_start: &LayoutPoint,
    b_end: &LayoutPoint,
) -> f64 {
    if segments_intersect(a_start, a_end, b_start, b_end) {
        return 0.0;
    }
    [
        point_to_segment_distance_mm(
            a_start.x_mm,
            a_start.y_mm,
            b_start.x_mm,
            b_start.y_mm,
            b_end.x_mm,
            b_end.y_mm,
        ),
        point_to_segment_distance_mm(
            a_end.x_mm,
            a_end.y_mm,
            b_start.x_mm,
            b_start.y_mm,
            b_end.x_mm,
            b_end.y_mm,
        ),
        point_to_segment_distance_mm(
            b_start.x_mm,
            b_start.y_mm,
            a_start.x_mm,
            a_start.y_mm,
            a_end.x_mm,
            a_end.y_mm,
        ),
        point_to_segment_distance_mm(
            b_end.x_mm,
            b_end.y_mm,
            a_start.x_mm,
            a_start.y_mm,
            a_end.x_mm,
            a_end.y_mm,
        ),
    ]
    .into_iter()
    .fold(f64::INFINITY, f64::min)
}

fn segments_intersect(
    a_start: &LayoutPoint,
    a_end: &LayoutPoint,
    b_start: &LayoutPoint,
    b_end: &LayoutPoint,
) -> bool {
    let d1 = cross_product(a_start, a_end, b_start);
    let d2 = cross_product(a_start, a_end, b_end);
    let d3 = cross_product(b_start, b_end, a_start);
    let d4 = cross_product(b_start, b_end, a_end);
    if ((d1 > f64::EPSILON && d2 < -f64::EPSILON) || (d1 < -f64::EPSILON && d2 > f64::EPSILON))
        && ((d3 > f64::EPSILON && d4 < -f64::EPSILON) || (d3 < -f64::EPSILON && d4 > f64::EPSILON))
    {
        return true;
    }
    (d1.abs() <= f64::EPSILON && point_on_segment(b_start, a_start, a_end))
        || (d2.abs() <= f64::EPSILON && point_on_segment(b_end, a_start, a_end))
        || (d3.abs() <= f64::EPSILON && point_on_segment(a_start, b_start, b_end))
        || (d4.abs() <= f64::EPSILON && point_on_segment(a_end, b_start, b_end))
}

fn cross_product(origin: &LayoutPoint, end: &LayoutPoint, point: &LayoutPoint) -> f64 {
    (end.x_mm - origin.x_mm) * (point.y_mm - origin.y_mm)
        - (end.y_mm - origin.y_mm) * (point.x_mm - origin.x_mm)
}

fn point_on_segment(point: &LayoutPoint, start: &LayoutPoint, end: &LayoutPoint) -> bool {
    point.x_mm >= start.x_mm.min(end.x_mm) - f64::EPSILON
        && point.x_mm <= start.x_mm.max(end.x_mm) + f64::EPSILON
        && point.y_mm >= start.y_mm.min(end.y_mm) - f64::EPSILON
        && point.y_mm <= start.y_mm.max(end.y_mm) + f64::EPSILON
}

fn point_distance_mm(start: &LayoutPoint, end: &LayoutPoint) -> f64 {
    (end.x_mm - start.x_mm).hypot(end.y_mm - start.y_mm)
}

fn body_overhang_from_points<'a>(
    points: impl IntoIterator<Item = &'a LayoutPoint>,
    edge: &LayoutSegment,
    outward_normal_deg: f64,
) -> f64 {
    points
        .into_iter()
        .map(|point| point_body_overhang_mm(point, edge, outward_normal_deg))
        .fold(0.0, f64::max)
}

fn point_body_overhang_mm(
    point: &LayoutPoint,
    edge: &LayoutSegment,
    outward_normal_deg: f64,
) -> f64 {
    let radians = outward_normal_deg.to_radians();
    let normal_x = radians.cos();
    let normal_y = radians.sin();
    let dx = point.x_mm - edge.start.x_mm;
    let dy = point.y_mm - edge.start.y_mm;
    (dx * normal_x + dy * normal_y).max(0.0)
}

fn point_is_finite(point: &LayoutPoint) -> bool {
    point.x_mm.is_finite() && point.y_mm.is_finite()
}

fn point_to_segment_distance_mm(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let dx = bx - ax;
    let dy = by - ay;
    let length_sq = dx * dx + dy * dy;
    if length_sq <= f64::EPSILON {
        return (px - ax).hypot(py - ay);
    }
    let t = (((px - ax) * dx + (py - ay) * dy) / length_sq).clamp(0.0, 1.0);
    let nearest_x = ax + t * dx;
    let nearest_y = ay + t * dy;
    (px - nearest_x).hypot(py - nearest_y)
}

fn outline_segment_length_mm(segment: &LayoutSegment) -> f64 {
    (segment.end.x_mm - segment.start.x_mm).hypot(segment.end.y_mm - segment.start.y_mm)
}

fn outline_segment_is_entry_candidate(segment: &LayoutSegment) -> bool {
    outline_segment_length_mm(segment) > f64::EPSILON
        && segment.boundary_role.as_deref() != Some("cutout")
}

fn segment_angle_deg(segment: &LayoutSegment) -> f64 {
    normalize_rotation_deg(
        (segment.end.y_mm - segment.start.y_mm)
            .atan2(segment.end.x_mm - segment.start.x_mm)
            .to_degrees(),
    )
}

fn outward_normal_deg(segment: &LayoutSegment, centroid: &LayoutPoint, edge_angle_deg: f64) -> f64 {
    let midpoint_x = (segment.start.x_mm + segment.end.x_mm) / 2.0;
    let midpoint_y = (segment.start.y_mm + segment.end.y_mm) / 2.0;
    let away_x = midpoint_x - centroid.x_mm;
    let away_y = midpoint_y - centroid.y_mm;
    let left_normal_deg = normalize_rotation_deg(edge_angle_deg + 90.0);
    let right_normal_deg = normalize_rotation_deg(edge_angle_deg - 90.0);
    let left_dot = angle_dot(left_normal_deg, away_x, away_y);
    let right_dot = angle_dot(right_normal_deg, away_x, away_y);
    if left_dot >= right_dot {
        left_normal_deg
    } else {
        right_normal_deg
    }
}

fn angle_dot(angle_deg: f64, x: f64, y: f64) -> f64 {
    let radians = angle_deg.to_radians();
    radians.cos() * x + radians.sin() * y
}

fn normalize_rotation_deg(rotation_deg: f64) -> f64 {
    rotation_deg.rem_euclid(360.0)
}

fn angular_error_deg(actual_deg: f64, expected_deg: f64) -> f64 {
    let delta = (normalize_rotation_deg(actual_deg) - normalize_rotation_deg(expected_deg)).abs();
    delta.min(360.0 - delta)
}
