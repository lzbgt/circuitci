use crate::board_ir::{
    ComponentPlacement, LayoutFootprintArc, LayoutFootprintCircle, LayoutFootprintPolygon,
    LayoutFootprintRectangle, LayoutFootprintSegment, LayoutPoint, LayoutSegment,
};
use crate::library::BoundBoard;

pub(super) struct UsbBoardEdgeDistanceEvidence<'a> {
    pub(super) distance_mm: f64,
    pub(super) edge: &'a LayoutSegment,
    pub(super) connector_reference: UsbBoardEdgeConnectorReference<'a>,
}

pub(super) struct UsbBodyOverhangEvidence<'a> {
    pub(super) body_overhang_mm: f64,
    pub(super) edge: &'a LayoutSegment,
    pub(super) connector_reference: UsbBoardEdgeConnectorReference<'a>,
    pub(super) edge_angle_deg: f64,
    pub(super) outward_normal_deg: f64,
}

#[derive(Clone, Copy)]
pub(super) enum UsbBoardEdgeConnectorReference<'a> {
    PlacementCenter,
    FootprintSegment { layer: &'a str, kind: &'a str },
    FootprintRectangle { layer: &'a str, kind: &'a str },
    FootprintPolygon { layer: &'a str, kind: &'a str },
    FootprintCircle { layer: &'a str, kind: &'a str },
    FootprintArc { layer: &'a str, kind: &'a str },
}

impl UsbBoardEdgeConnectorReference<'_> {
    pub(super) fn label(&self) -> &'static str {
        match self {
            UsbBoardEdgeConnectorReference::PlacementCenter => "placement_center",
            UsbBoardEdgeConnectorReference::FootprintSegment { .. } => "footprint_segment",
            UsbBoardEdgeConnectorReference::FootprintRectangle { .. } => "footprint_rectangle",
            UsbBoardEdgeConnectorReference::FootprintPolygon { .. } => "footprint_polygon",
            UsbBoardEdgeConnectorReference::FootprintCircle { .. } => "footprint_circle",
            UsbBoardEdgeConnectorReference::FootprintArc { .. } => "footprint_arc",
        }
    }

    pub(super) fn footprint_layer(&self) -> Option<&str> {
        match self {
            UsbBoardEdgeConnectorReference::PlacementCenter => None,
            UsbBoardEdgeConnectorReference::FootprintSegment { layer, .. }
            | UsbBoardEdgeConnectorReference::FootprintRectangle { layer, .. }
            | UsbBoardEdgeConnectorReference::FootprintPolygon { layer, .. }
            | UsbBoardEdgeConnectorReference::FootprintCircle { layer, .. }
            | UsbBoardEdgeConnectorReference::FootprintArc { layer, .. } => Some(layer),
        }
    }

    pub(super) fn footprint_kind(&self) -> Option<&str> {
        match self {
            UsbBoardEdgeConnectorReference::PlacementCenter => None,
            UsbBoardEdgeConnectorReference::FootprintSegment { kind, .. }
            | UsbBoardEdgeConnectorReference::FootprintRectangle { kind, .. }
            | UsbBoardEdgeConnectorReference::FootprintPolygon { kind, .. }
            | UsbBoardEdgeConnectorReference::FootprintCircle { kind, .. }
            | UsbBoardEdgeConnectorReference::FootprintArc { kind, .. } => Some(kind),
        }
    }
}

pub(super) fn placement_is_finite(placement: &ComponentPlacement) -> bool {
    placement.x_mm.is_finite() && placement.y_mm.is_finite()
}

pub(super) fn placement_distance_mm(a: &ComponentPlacement, b: &ComponentPlacement) -> f64 {
    let dx = a.x_mm - b.x_mm;
    let dy = a.y_mm - b.y_mm;
    dx.hypot(dy)
}

pub(super) fn nearest_board_edge<'a>(
    bound: &'a BoundBoard<'_>,
    component_id: &'a str,
    placement: &ComponentPlacement,
) -> Option<UsbBoardEdgeDistanceEvidence<'a>> {
    bound
        .project
        .board
        .layout
        .outline
        .segments
        .iter()
        .filter(|segment| outline_segment_is_entry_candidate(segment))
        .map(|edge| {
            let (distance_mm, connector_reference) =
                connector_to_edge_distance(bound, component_id, placement, edge);
            UsbBoardEdgeDistanceEvidence {
                distance_mm,
                edge,
                connector_reference,
            }
        })
        .min_by(|left, right| left.distance_mm.total_cmp(&right.distance_mm))
}

pub(super) fn nearest_body_overhang_edge<'a>(
    bound: &'a BoundBoard<'_>,
    component_id: &'a str,
) -> Option<UsbBodyOverhangEvidence<'a>> {
    let centroid = outline_centroid(&bound.project.board.layout.outline.segments)?;
    bound
        .project
        .board
        .layout
        .outline
        .segments
        .iter()
        .filter(|segment| outline_segment_is_entry_candidate(segment))
        .filter_map(|edge| {
            let distance_mm = connector_body_to_edge_distance(bound, component_id, edge)?;
            let evidence = body_overhang_for_edge(bound, component_id, edge, &centroid)?;
            Some((distance_mm, evidence))
        })
        .min_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(_, evidence)| evidence)
}

fn body_overhang_for_edge<'a>(
    bound: &'a BoundBoard<'_>,
    component_id: &'a str,
    edge: &'a LayoutSegment,
    centroid: &LayoutPoint,
) -> Option<UsbBodyOverhangEvidence<'a>> {
    let edge_angle_deg = segment_angle_deg(edge);
    let outward_normal_deg = outward_normal_deg(edge, centroid, edge_angle_deg);
    connector_body_overhang(bound, component_id, edge, outward_normal_deg).map(
        |(body_overhang_mm, connector_reference)| UsbBodyOverhangEvidence {
            body_overhang_mm,
            edge,
            connector_reference,
            edge_angle_deg,
            outward_normal_deg,
        },
    )
}

fn connector_to_edge_distance<'a>(
    bound: &'a BoundBoard<'_>,
    component_id: &'a str,
    placement: &ComponentPlacement,
    edge: &LayoutSegment,
) -> (f64, UsbBoardEdgeConnectorReference<'a>) {
    let mut best_distance = placement_to_segment_distance_mm(placement, edge);
    let mut best_reference = UsbBoardEdgeConnectorReference::PlacementCenter;
    let Some(footprint) = bound.project.board.layout.footprints.get(component_id) else {
        return (best_distance, best_reference);
    };

    for segment in &footprint.segments {
        if !mechanical_footprint_kind(&segment.kind) {
            continue;
        }
        let Some(distance_mm) = footprint_segment_to_edge_distance_mm(segment, edge) else {
            continue;
        };
        if distance_mm < best_distance {
            best_distance = distance_mm;
            best_reference = UsbBoardEdgeConnectorReference::FootprintSegment {
                layer: &segment.layer,
                kind: &segment.kind,
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
        if distance_mm < best_distance {
            best_distance = distance_mm;
            best_reference = UsbBoardEdgeConnectorReference::FootprintRectangle {
                layer: &rectangle.layer,
                kind: &rectangle.kind,
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
        if distance_mm <= best_distance {
            best_distance = distance_mm;
            best_reference = UsbBoardEdgeConnectorReference::FootprintPolygon {
                layer: &polygon.layer,
                kind: &polygon.kind,
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
        if distance_mm <= best_distance {
            best_distance = distance_mm;
            best_reference = UsbBoardEdgeConnectorReference::FootprintCircle {
                layer: &circle.layer,
                kind: &circle.kind,
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
        if distance_mm <= best_distance {
            best_distance = distance_mm;
            best_reference = UsbBoardEdgeConnectorReference::FootprintArc {
                layer: &arc.layer,
                kind: &arc.kind,
            };
        }
    }

    (best_distance, best_reference)
}

fn connector_body_to_edge_distance(
    bound: &BoundBoard<'_>,
    component_id: &str,
    edge: &LayoutSegment,
) -> Option<f64> {
    let footprint = bound.project.board.layout.footprints.get(component_id)?;
    footprint
        .segments
        .iter()
        .filter(|segment| mechanical_footprint_kind(&segment.kind))
        .filter_map(|segment| footprint_segment_to_edge_distance_mm(segment, edge))
        .chain(
            footprint
                .rectangles
                .iter()
                .filter(|rectangle| mechanical_footprint_kind(&rectangle.kind))
                .filter_map(|rectangle| footprint_rectangle_to_edge_distance_mm(rectangle, edge)),
        )
        .chain(
            footprint
                .polygons
                .iter()
                .filter(|polygon| mechanical_footprint_kind(&polygon.kind))
                .filter_map(|polygon| footprint_polygon_to_edge_distance_mm(polygon, edge)),
        )
        .chain(
            footprint
                .circles
                .iter()
                .filter(|circle| mechanical_footprint_kind(&circle.kind))
                .filter_map(|circle| {
                    let points = footprint_circle_points(circle)?;
                    closed_polyline_to_edge_distance_mm(&points, edge)
                }),
        )
        .chain(
            footprint
                .arcs
                .iter()
                .filter(|arc| mechanical_footprint_kind(&arc.kind))
                .filter_map(|arc| {
                    let points = footprint_arc_points(arc)?;
                    open_polyline_to_edge_distance_mm(&points, edge)
                }),
        )
        .min_by(|left, right| left.total_cmp(right))
}

fn connector_body_overhang<'a>(
    bound: &'a BoundBoard<'_>,
    component_id: &'a str,
    edge: &LayoutSegment,
    outward_normal_deg: f64,
) -> Option<(f64, UsbBoardEdgeConnectorReference<'a>)> {
    let footprint = bound.project.board.layout.footprints.get(component_id)?;
    let mut best: Option<(f64, UsbBoardEdgeConnectorReference<'a>)> = None;

    for segment in &footprint.segments {
        if !mechanical_footprint_kind(&segment.kind)
            || !point_is_finite(&segment.start)
            || !point_is_finite(&segment.end)
            || segment_length_mm(&segment.start, &segment.end) <= f64::EPSILON
        {
            continue;
        }
        let overhang_mm =
            body_overhang_from_points([&segment.start, &segment.end], edge, outward_normal_deg);
        update_body_overhang_candidate(
            &mut best,
            overhang_mm,
            UsbBoardEdgeConnectorReference::FootprintSegment {
                layer: &segment.layer,
                kind: &segment.kind,
            },
        );
    }

    for rectangle in &footprint.rectangles {
        if !mechanical_footprint_kind(&rectangle.kind) {
            continue;
        }
        let Some(corners) = rectangle_corners(rectangle) else {
            continue;
        };
        let overhang_mm = body_overhang_from_points(corners.iter(), edge, outward_normal_deg);
        update_body_overhang_candidate(
            &mut best,
            overhang_mm,
            UsbBoardEdgeConnectorReference::FootprintRectangle {
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
        let overhang_mm =
            body_overhang_from_points(polygon.points.iter(), edge, outward_normal_deg);
        update_body_overhang_candidate(
            &mut best,
            overhang_mm,
            UsbBoardEdgeConnectorReference::FootprintPolygon {
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
        let overhang_mm = body_overhang_from_points(points.iter(), edge, outward_normal_deg);
        update_body_overhang_candidate(
            &mut best,
            overhang_mm,
            UsbBoardEdgeConnectorReference::FootprintCircle {
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
        let overhang_mm = body_overhang_from_points(points.iter(), edge, outward_normal_deg);
        update_body_overhang_candidate(
            &mut best,
            overhang_mm,
            UsbBoardEdgeConnectorReference::FootprintArc {
                layer: &arc.layer,
                kind: &arc.kind,
            },
        );
    }

    best
}

fn update_body_overhang_candidate<'a>(
    best: &mut Option<(f64, UsbBoardEdgeConnectorReference<'a>)>,
    overhang_mm: f64,
    reference: UsbBoardEdgeConnectorReference<'a>,
) {
    if !overhang_mm.is_finite() {
        return;
    }
    match best {
        Some((best_overhang_mm, _)) if overhang_mm <= *best_overhang_mm => {}
        _ => *best = Some((overhang_mm, reference)),
    }
}

pub(super) fn mechanical_footprint_kind(kind: &str) -> bool {
    matches!(kind, "fabrication" | "courtyard")
}

fn outline_segment_is_usable(segment: &LayoutSegment) -> bool {
    point_is_finite(&segment.start)
        && point_is_finite(&segment.end)
        && (segment.end.x_mm - segment.start.x_mm).hypot(segment.end.y_mm - segment.start.y_mm)
            > f64::EPSILON
}

fn outline_segment_is_entry_candidate(segment: &LayoutSegment) -> bool {
    outline_segment_is_usable(segment) && segment.boundary_role.as_deref() != Some("cutout")
}

pub(super) fn point_is_finite(point: &LayoutPoint) -> bool {
    point.x_mm.is_finite() && point.y_mm.is_finite()
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
        || segment_length_mm(&segment.start, &segment.end) <= f64::EPSILON
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

pub(super) fn rectangle_corners(rectangle: &LayoutFootprintRectangle) -> Option<[LayoutPoint; 4]> {
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

pub(super) fn footprint_circle_points(circle: &LayoutFootprintCircle) -> Option<Vec<LayoutPoint>> {
    if !point_is_finite(&circle.center) || !point_is_finite(&circle.end) {
        return None;
    }
    let radius = segment_length_mm(&circle.center, &circle.end);
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

pub(super) fn footprint_arc_points(arc: &LayoutFootprintArc) -> Option<Vec<LayoutPoint>> {
    if !point_is_finite(&arc.start) || !point_is_finite(&arc.mid) || !point_is_finite(&arc.end) {
        return None;
    }
    let center = arc_center(&arc.start, &arc.mid, &arc.end)?;
    let radius = segment_length_mm(&center, &arc.start);
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

pub(super) fn segment_length_mm(start: &LayoutPoint, end: &LayoutPoint) -> f64 {
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

pub(super) fn segment_to_segment_distance_mm(
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

pub(super) fn point_to_segment_distance_mm(
    px: f64,
    py: f64,
    ax: f64,
    ay: f64,
    bx: f64,
    by: f64,
) -> f64 {
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

pub(super) fn angular_error_deg(actual_deg: f64, expected_deg: f64) -> f64 {
    let delta = (normalize_rotation_deg(actual_deg) - normalize_rotation_deg(expected_deg)).abs();
    delta.min(360.0 - delta)
}
