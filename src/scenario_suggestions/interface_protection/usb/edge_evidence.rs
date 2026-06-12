use super::super::super::{
    SuggestedBoardEdge, SuggestedFootprint, SuggestedFootprintPolygon, SuggestedFootprintRectangle,
    SuggestedFootprintSegment, SuggestedPoint,
};
use super::super::component_placement;
use crate::board_ir::{
    ComponentPlacement, LayoutFootprint, LayoutFootprintPolygon, LayoutFootprintRectangle,
    LayoutFootprintSegment, LayoutPoint, LayoutSegment,
};
use crate::library::BoundBoard;

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
    (!segments.is_empty() || !rectangles.is_empty() || !polygons.is_empty()).then_some(
        SuggestedFootprint {
            segments,
            rectangles,
            polygons,
        },
    )
}

pub(super) fn nearest_board_edge_evidence(
    bound: &BoundBoard<'_>,
    component_id: &str,
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
        .filter(|segment| outline_segment_length_mm(segment) > f64::EPSILON)
        .map(|segment| {
            let distance =
                connector_to_board_edge_distance(bound, component_id, placement, segment);
            (segment, distance)
        })
        .min_by(|left, right| left.1.distance_mm.total_cmp(&right.1.distance_mm))?;
    let edge_angle_deg = segment_angle_deg(edge.0);
    let outward_normal_deg = outward_normal_deg(edge.0, &centroid, edge_angle_deg);
    Some(SuggestedBoardEdge {
        start: suggested_point(&edge.0.start),
        end: suggested_point(&edge.0.end),
        layer: edge.0.layer.clone(),
        distance_to_connector_mm: edge.1.distance_mm,
        connector_edge_reference: edge.1.reference.label().to_string(),
        footprint_graphic_layer: edge.1.reference.footprint_layer().map(str::to_string),
        footprint_graphic_kind: edge.1.reference.footprint_kind().map(str::to_string),
        edge_angle_deg,
        outward_normal_deg,
        connector_rotation_error_deg: angular_error_deg(rotation_deg, outward_normal_deg),
    })
}

struct BoardEdgeConnectorDistance<'a> {
    distance_mm: f64,
    reference: BoardEdgeConnectorReference<'a>,
}

#[derive(Clone, Copy)]
enum BoardEdgeConnectorReference<'a> {
    PlacementCenter,
    FootprintSegment { layer: &'a str, kind: &'a str },
    FootprintRectangle { layer: &'a str, kind: &'a str },
    FootprintPolygon { layer: &'a str, kind: &'a str },
}

impl BoardEdgeConnectorReference<'_> {
    fn label(&self) -> &'static str {
        match self {
            BoardEdgeConnectorReference::PlacementCenter => "placement_center",
            BoardEdgeConnectorReference::FootprintSegment { .. } => "footprint_segment",
            BoardEdgeConnectorReference::FootprintRectangle { .. } => "footprint_rectangle",
            BoardEdgeConnectorReference::FootprintPolygon { .. } => "footprint_polygon",
        }
    }

    fn footprint_layer(&self) -> Option<&str> {
        match self {
            BoardEdgeConnectorReference::PlacementCenter => None,
            BoardEdgeConnectorReference::FootprintSegment { layer, .. }
            | BoardEdgeConnectorReference::FootprintRectangle { layer, .. }
            | BoardEdgeConnectorReference::FootprintPolygon { layer, .. } => Some(layer),
        }
    }

    fn footprint_kind(&self) -> Option<&str> {
        match self {
            BoardEdgeConnectorReference::PlacementCenter => None,
            BoardEdgeConnectorReference::FootprintSegment { kind, .. }
            | BoardEdgeConnectorReference::FootprintRectangle { kind, .. }
            | BoardEdgeConnectorReference::FootprintPolygon { kind, .. } => Some(kind),
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
    };
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
            best = BoardEdgeConnectorDistance {
                distance_mm,
                reference: BoardEdgeConnectorReference::FootprintRectangle {
                    layer: &rectangle.layer,
                    kind: &rectangle.kind,
                },
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
            };
        }
    }
    best
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
        if outline_segment_length_mm(segment) <= f64::EPSILON {
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
    let corners = [
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
    ];
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

fn footprint_polygon_to_edge_distance_mm(
    polygon: &LayoutFootprintPolygon,
    edge: &LayoutSegment,
) -> Option<f64> {
    if polygon.points.len() < 3 || polygon.points.iter().any(|point| !point_is_finite(point)) {
        return None;
    }
    Some(
        (0..polygon.points.len())
            .map(|index| {
                let next_index = (index + 1) % polygon.points.len();
                segment_to_segment_distance_mm(
                    &polygon.points[index],
                    &polygon.points[next_index],
                    &edge.start,
                    &edge.end,
                )
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
