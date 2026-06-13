use crate::board_ir::{
    LayoutCopperFeature, LayoutCopperRegion, LayoutCopperSegment, LayoutDrill, LayoutPoint,
    LayoutSegment,
};

pub(super) fn validate_drill_geometry(
    drill: &LayoutDrill,
    drill_index: usize,
) -> Result<(), String> {
    if !finite_point(&drill.at) {
        return Err(format!(
            "board.layout.drills[{drill_index}].at must contain finite coordinates."
        ));
    }
    if !drill.drill_mm.is_finite() || drill.drill_mm <= 0.0 {
        return Err(format!(
            "board.layout.drills[{drill_index}].drill_mm must be finite and positive."
        ));
    }
    Ok(())
}

pub(super) fn validate_copper_feature_geometry(
    feature: &LayoutCopperFeature,
    feature_index: usize,
) -> Result<(), String> {
    if !finite_point(&feature.at) {
        return Err(format!(
            "board.layout.copper.features[{feature_index}].at must contain finite coordinates."
        ));
    }
    if !feature.size.x_mm.is_finite()
        || !feature.size.y_mm.is_finite()
        || feature.size.x_mm <= 0.0
        || feature.size.y_mm <= 0.0
    {
        return Err(format!(
            "board.layout.copper.features[{feature_index}].size must contain finite positive dimensions."
        ));
    }
    Ok(())
}

pub(super) fn validate_copper_segment_geometry(
    segment: &LayoutCopperSegment,
    segment_index: usize,
) -> Result<(), String> {
    if !finite_point(&segment.start) || !finite_point(&segment.end) {
        return Err(format!(
            "board.layout.copper.segments[{segment_index}] start/end must contain finite coordinates."
        ));
    }
    if point_distance_mm(&segment.start, &segment.end) <= f64::EPSILON {
        return Err(format!(
            "board.layout.copper.segments[{segment_index}] must have non-zero length."
        ));
    }
    if !segment.width_mm.is_finite() || segment.width_mm <= 0.0 {
        return Err(format!(
            "board.layout.copper.segments[{segment_index}].width_mm must be finite and positive."
        ));
    }
    Ok(())
}

pub(super) fn validate_copper_region_geometry(
    region: &LayoutCopperRegion,
    region_index: usize,
) -> Result<(), String> {
    if region.points.len() < 3 {
        return Err(format!(
            "board.layout.copper.regions[{region_index}].points must contain at least three points."
        ));
    }
    if region.points.iter().any(|point| !finite_point(point)) {
        return Err(format!(
            "board.layout.copper.regions[{region_index}].points must contain finite coordinates."
        ));
    }
    if polygon_signed_area_mm2(&region.points).abs() <= f64::EPSILON {
        return Err(format!(
            "board.layout.copper.regions[{region_index}].points must form a non-degenerate polygon."
        ));
    }
    Ok(())
}

pub(super) fn usable_outline_segment(segment: &LayoutSegment) -> bool {
    finite_point(&segment.start)
        && finite_point(&segment.end)
        && point_distance_mm(&segment.start, &segment.end) > f64::EPSILON
}

fn finite_point(point: &LayoutPoint) -> bool {
    point.x_mm.is_finite() && point.y_mm.is_finite()
}

#[derive(Debug, Clone, Copy)]
pub(super) struct DrillEdgeClearance<'a> {
    pub(super) edge: &'a LayoutSegment,
    pub(super) center_distance_mm: f64,
    pub(super) clearance_mm: f64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct CopperFeatureEdgeClearance<'a> {
    pub(super) edge: &'a LayoutSegment,
    pub(super) clearance_mm: f64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct CopperSegmentEdgeClearance<'a> {
    pub(super) edge: &'a LayoutSegment,
    pub(super) centerline_distance_mm: f64,
    pub(super) clearance_mm: f64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct CopperRegionEdgeClearance<'a> {
    pub(super) edge: &'a LayoutSegment,
    pub(super) clearance_mm: f64,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum CopperObjectRef<'a> {
    Feature {
        feature: &'a LayoutCopperFeature,
        index: usize,
    },
    Segment {
        segment: &'a LayoutCopperSegment,
        index: usize,
    },
    Region {
        region: &'a LayoutCopperRegion,
        index: usize,
    },
}

impl CopperObjectRef<'_> {
    pub(super) fn layer(&self) -> &str {
        match self {
            Self::Feature { feature, .. } => &feature.layer,
            Self::Segment { segment, .. } => &segment.layer,
            Self::Region { region, .. } => &region.layer,
        }
    }

    pub(super) fn kind(&self) -> &'static str {
        match self {
            Self::Feature { .. } => "feature",
            Self::Segment { .. } => "segment",
            Self::Region { .. } => "region",
        }
    }

    pub(super) fn net(&self) -> Option<&str> {
        match self {
            Self::Feature { feature, .. } => feature.net.as_deref(),
            Self::Segment { segment, .. } => segment.net.as_deref(),
            Self::Region { region, .. } => region.net.as_deref(),
        }
    }

    pub(super) fn island_id(&self) -> Option<&str> {
        match self {
            Self::Feature { feature, .. } => feature.island_id.as_deref(),
            Self::Segment { segment, .. } => segment.island_id.as_deref(),
            Self::Region { region, .. } => region.island_id.as_deref(),
        }
    }
}

pub(super) fn nearest_drill_edge_clearance<'a>(
    drill: &LayoutDrill,
    board_edges: &'a [&LayoutSegment],
) -> Option<DrillEdgeClearance<'a>> {
    let radius_mm = drill.drill_mm / 2.0;
    board_edges
        .iter()
        .filter_map(|edge| {
            let center_distance_mm =
                point_to_segment_distance_mm(&drill.at, &edge.start, &edge.end);
            center_distance_mm
                .is_finite()
                .then_some(DrillEdgeClearance {
                    edge,
                    center_distance_mm,
                    clearance_mm: center_distance_mm - radius_mm,
                })
        })
        .min_by(|first, second| first.clearance_mm.total_cmp(&second.clearance_mm))
}

pub(super) fn nearest_copper_feature_edge_clearance<'a>(
    feature: &LayoutCopperFeature,
    board_edges: &'a [&LayoutSegment],
) -> Option<CopperFeatureEdgeClearance<'a>> {
    board_edges
        .iter()
        .filter_map(|edge| {
            let clearance_mm = copper_feature_to_segment_clearance_mm(feature, edge)?;
            clearance_mm
                .is_finite()
                .then_some(CopperFeatureEdgeClearance { edge, clearance_mm })
        })
        .min_by(|first, second| first.clearance_mm.total_cmp(&second.clearance_mm))
}

pub(super) fn nearest_copper_segment_edge_clearance<'a>(
    segment: &LayoutCopperSegment,
    board_edges: &'a [&LayoutSegment],
) -> Option<CopperSegmentEdgeClearance<'a>> {
    let radius_mm = segment.width_mm / 2.0;
    board_edges
        .iter()
        .filter_map(|edge| {
            let centerline_distance_mm = segment_to_segment_distance_mm(
                &segment.start,
                &segment.end,
                &edge.start,
                &edge.end,
            );
            centerline_distance_mm
                .is_finite()
                .then_some(CopperSegmentEdgeClearance {
                    edge,
                    centerline_distance_mm,
                    clearance_mm: centerline_distance_mm - radius_mm,
                })
        })
        .min_by(|first, second| first.clearance_mm.total_cmp(&second.clearance_mm))
}

pub(super) fn nearest_copper_region_edge_clearance<'a>(
    region: &LayoutCopperRegion,
    board_edges: &'a [&LayoutSegment],
) -> Option<CopperRegionEdgeClearance<'a>> {
    board_edges
        .iter()
        .filter_map(|edge| {
            let clearance_mm =
                polygon_to_segment_clearance_mm(&region.points, &edge.start, &edge.end);
            clearance_mm
                .is_finite()
                .then_some(CopperRegionEdgeClearance { edge, clearance_mm })
        })
        .min_by(|first, second| first.clearance_mm.total_cmp(&second.clearance_mm))
}

pub(super) fn annular_ring_for_feature(
    drill: &LayoutDrill,
    feature: &LayoutCopperFeature,
) -> Option<f64> {
    let drill_radius_mm = drill.drill_mm / 2.0;
    let dx = drill.at.x_mm - feature.at.x_mm;
    let dy = drill.at.y_mm - feature.at.y_mm;
    let copper_boundary_distance_mm = match feature.shape.as_str() {
        "circle" => feature.size.x_mm.min(feature.size.y_mm) / 2.0 - dx.hypot(dy),
        "rect" => {
            let half_x = feature.size.x_mm / 2.0;
            let half_y = feature.size.y_mm / 2.0;
            (half_x - dx.abs()).min(half_y - dy.abs())
        }
        "oval" => oval_boundary_distance_mm(dx, dy, feature.size.x_mm, feature.size.y_mm),
        _ => return None,
    };
    Some(copper_boundary_distance_mm - drill_radius_mm)
}

pub(super) fn copper_object_spacing_mm(
    first: CopperObjectRef<'_>,
    second: CopperObjectRef<'_>,
) -> Option<f64> {
    match (first, second) {
        (
            CopperObjectRef::Feature { feature: first, .. },
            CopperObjectRef::Feature {
                feature: second, ..
            },
        ) => copper_feature_to_feature_clearance_mm(first, second),
        (CopperObjectRef::Feature { feature, .. }, CopperObjectRef::Segment { segment, .. })
        | (CopperObjectRef::Segment { segment, .. }, CopperObjectRef::Feature { feature, .. }) => {
            copper_feature_to_copper_segment_clearance_mm(feature, segment)
        }
        (
            CopperObjectRef::Segment { segment: first, .. },
            CopperObjectRef::Segment {
                segment: second, ..
            },
        ) => Some(
            segment_to_segment_distance_mm(&first.start, &first.end, &second.start, &second.end)
                - first.width_mm / 2.0
                - second.width_mm / 2.0,
        ),
        (CopperObjectRef::Feature { feature, .. }, CopperObjectRef::Region { region, .. })
        | (CopperObjectRef::Region { region, .. }, CopperObjectRef::Feature { feature, .. }) => {
            copper_feature_to_region_clearance_mm(feature, region)
        }
        (CopperObjectRef::Segment { segment, .. }, CopperObjectRef::Region { region, .. })
        | (CopperObjectRef::Region { region, .. }, CopperObjectRef::Segment { segment, .. }) => {
            Some(copper_segment_to_region_clearance_mm(segment, region))
        }
        (
            CopperObjectRef::Region { region: first, .. },
            CopperObjectRef::Region { region: second, .. },
        ) => Some(polygon_to_polygon_clearance_mm(
            &first.points,
            &second.points,
        )),
    }
    .filter(|value| value.is_finite())
}

fn copper_feature_to_feature_clearance_mm(
    first: &LayoutCopperFeature,
    second: &LayoutCopperFeature,
) -> Option<f64> {
    match (first.shape.as_str(), second.shape.as_str()) {
        ("circle", "circle") => {
            let first_radius = first.size.x_mm.min(first.size.y_mm) / 2.0;
            let second_radius = second.size.x_mm.min(second.size.y_mm) / 2.0;
            Some(point_distance_mm(&first.at, &second.at) - first_radius - second_radius)
        }
        ("circle", "rect") => Some(
            point_to_rect_distance_mm(
                &first.at,
                second.at.x_mm - second.size.x_mm / 2.0,
                second.at.x_mm + second.size.x_mm / 2.0,
                second.at.y_mm - second.size.y_mm / 2.0,
                second.at.y_mm + second.size.y_mm / 2.0,
            ) - first.size.x_mm.min(first.size.y_mm) / 2.0,
        ),
        ("rect", "circle") => copper_feature_to_feature_clearance_mm(second, first),
        ("rect", "rect") => {
            let dx = (first.at.x_mm - second.at.x_mm).abs()
                - first.size.x_mm / 2.0
                - second.size.x_mm / 2.0;
            let dy = (first.at.y_mm - second.at.y_mm).abs()
                - first.size.y_mm / 2.0
                - second.size.y_mm / 2.0;
            Some(dx.max(0.0).hypot(dy.max(0.0)))
        }
        ("circle" | "rect" | "oval", "circle" | "rect" | "oval") => {
            Some(sampled_feature_to_feature_distance_mm(first, second))
        }
        _ => None,
    }
}

fn copper_feature_to_copper_segment_clearance_mm(
    feature: &LayoutCopperFeature,
    segment: &LayoutCopperSegment,
) -> Option<f64> {
    let centerline = LayoutSegment {
        start: segment.start.clone(),
        end: segment.end.clone(),
        layer: None,
        source_primitive: None,
        source_primitive_index: None,
        sample_index: None,
        sample_count: None,
        contour_index: None,
        boundary_role: None,
    };
    copper_feature_to_segment_clearance_mm(feature, &centerline)
        .map(|clearance| clearance - segment.width_mm / 2.0)
}

fn copper_feature_to_region_clearance_mm(
    feature: &LayoutCopperFeature,
    region: &LayoutCopperRegion,
) -> Option<f64> {
    let feature_points = feature_boundary_points(feature);
    if feature_points.is_empty() {
        return None;
    }
    Some(polygon_to_polygon_clearance_mm(
        &feature_points,
        &region.points,
    ))
}

fn copper_segment_to_region_clearance_mm(
    segment: &LayoutCopperSegment,
    region: &LayoutCopperRegion,
) -> f64 {
    polygon_to_segment_clearance_mm(&region.points, &segment.start, &segment.end)
        - segment.width_mm / 2.0
}

fn copper_feature_to_segment_clearance_mm(
    feature: &LayoutCopperFeature,
    edge: &LayoutSegment,
) -> Option<f64> {
    let center = &feature.at;
    match feature.shape.as_str() {
        "circle" => {
            let radius = feature.size.x_mm.min(feature.size.y_mm) / 2.0;
            Some(point_to_segment_distance_mm(center, &edge.start, &edge.end) - radius)
        }
        "rect" => Some(segment_to_axis_aligned_rect_distance_mm(
            &edge.start,
            &edge.end,
            center,
            feature.size.x_mm / 2.0,
            feature.size.y_mm / 2.0,
        )),
        "oval" => Some(segment_to_axis_aligned_oval_distance_mm(
            &edge.start,
            &edge.end,
            center,
            feature.size.x_mm,
            feature.size.y_mm,
        )),
        _ => None,
    }
}

fn sampled_feature_to_feature_distance_mm(
    first: &LayoutCopperFeature,
    second: &LayoutCopperFeature,
) -> f64 {
    let first_points = feature_boundary_points(first);
    let second_points = feature_boundary_points(second);
    if first_points
        .iter()
        .any(|point| point_inside_copper_feature(point, second))
        || second_points
            .iter()
            .any(|point| point_inside_copper_feature(point, first))
    {
        return 0.0;
    }
    let mut min_distance = f64::INFINITY;
    for first_edge in closed_edges(&first_points) {
        for second_edge in closed_edges(&second_points) {
            min_distance = min_distance.min(segment_to_segment_distance_mm(
                first_edge.0,
                first_edge.1,
                second_edge.0,
                second_edge.1,
            ));
        }
    }
    min_distance
}

fn polygon_to_polygon_clearance_mm(first: &[LayoutPoint], second: &[LayoutPoint]) -> f64 {
    if first
        .iter()
        .any(|point| point_inside_polygon(point, second))
        || second
            .iter()
            .any(|point| point_inside_polygon(point, first))
    {
        return 0.0;
    }
    let mut min_distance = f64::INFINITY;
    for first_edge in closed_edges(first) {
        for second_edge in closed_edges(second) {
            min_distance = min_distance.min(segment_to_segment_distance_mm(
                first_edge.0,
                first_edge.1,
                second_edge.0,
                second_edge.1,
            ));
        }
    }
    min_distance
}

fn polygon_to_segment_clearance_mm(
    polygon: &[LayoutPoint],
    start: &LayoutPoint,
    end: &LayoutPoint,
) -> f64 {
    if point_inside_polygon(start, polygon) || point_inside_polygon(end, polygon) {
        return 0.0;
    }
    closed_edges(polygon)
        .map(|edge| segment_to_segment_distance_mm(edge.0, edge.1, start, end))
        .fold(f64::INFINITY, f64::min)
}

pub(super) fn feature_boundary_points(feature: &LayoutCopperFeature) -> Vec<LayoutPoint> {
    match feature.shape.as_str() {
        "circle" => (0..32)
            .map(|index| {
                let theta = std::f64::consts::TAU * index as f64 / 32.0;
                let radius = feature.size.x_mm.min(feature.size.y_mm) / 2.0;
                LayoutPoint {
                    x_mm: feature.at.x_mm + radius * theta.cos(),
                    y_mm: feature.at.y_mm + radius * theta.sin(),
                }
            })
            .collect(),
        "rect" => {
            let half_x = feature.size.x_mm / 2.0;
            let half_y = feature.size.y_mm / 2.0;
            vec![
                LayoutPoint {
                    x_mm: feature.at.x_mm - half_x,
                    y_mm: feature.at.y_mm - half_y,
                },
                LayoutPoint {
                    x_mm: feature.at.x_mm + half_x,
                    y_mm: feature.at.y_mm - half_y,
                },
                LayoutPoint {
                    x_mm: feature.at.x_mm + half_x,
                    y_mm: feature.at.y_mm + half_y,
                },
                LayoutPoint {
                    x_mm: feature.at.x_mm - half_x,
                    y_mm: feature.at.y_mm + half_y,
                },
            ]
        }
        "oval" => oval_boundary_points(&feature.at, feature.size.x_mm, feature.size.y_mm),
        _ => Vec::new(),
    }
}

fn oval_boundary_points(center: &LayoutPoint, width_mm: f64, height_mm: f64) -> Vec<LayoutPoint> {
    let mut points = Vec::with_capacity(34);
    if width_mm >= height_mm {
        let radius = height_mm / 2.0;
        let half_segment = (width_mm - height_mm) / 2.0;
        for index in 0..=16 {
            let theta = -std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * index as f64 / 16.0;
            points.push(LayoutPoint {
                x_mm: center.x_mm + half_segment + radius * theta.cos(),
                y_mm: center.y_mm + radius * theta.sin(),
            });
        }
        for index in 0..=16 {
            let theta = std::f64::consts::FRAC_PI_2 + std::f64::consts::PI * index as f64 / 16.0;
            points.push(LayoutPoint {
                x_mm: center.x_mm - half_segment + radius * theta.cos(),
                y_mm: center.y_mm + radius * theta.sin(),
            });
        }
    } else {
        let radius = width_mm / 2.0;
        let half_segment = (height_mm - width_mm) / 2.0;
        for index in 0..=16 {
            let theta = std::f64::consts::PI * index as f64 / 16.0;
            points.push(LayoutPoint {
                x_mm: center.x_mm + radius * theta.cos(),
                y_mm: center.y_mm + half_segment + radius * theta.sin(),
            });
        }
        for index in 0..=16 {
            let theta = std::f64::consts::PI + std::f64::consts::PI * index as f64 / 16.0;
            points.push(LayoutPoint {
                x_mm: center.x_mm + radius * theta.cos(),
                y_mm: center.y_mm - half_segment + radius * theta.sin(),
            });
        }
    }
    points
}

fn closed_edges(points: &[LayoutPoint]) -> impl Iterator<Item = (&LayoutPoint, &LayoutPoint)> {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
}

pub(super) fn point_inside_copper_feature(
    point: &LayoutPoint,
    feature: &LayoutCopperFeature,
) -> bool {
    let dx = point.x_mm - feature.at.x_mm;
    let dy = point.y_mm - feature.at.y_mm;
    match feature.shape.as_str() {
        "circle" => dx.hypot(dy) <= feature.size.x_mm.min(feature.size.y_mm) / 2.0 + f64::EPSILON,
        "rect" => point_inside_rect(
            point,
            feature.at.x_mm - feature.size.x_mm / 2.0,
            feature.at.x_mm + feature.size.x_mm / 2.0,
            feature.at.y_mm - feature.size.y_mm / 2.0,
            feature.at.y_mm + feature.size.y_mm / 2.0,
        ),
        "oval" => {
            oval_boundary_distance_mm(dx, dy, feature.size.x_mm, feature.size.y_mm) >= -f64::EPSILON
        }
        _ => false,
    }
}

pub(super) fn point_inside_polygon(point: &LayoutPoint, polygon: &[LayoutPoint]) -> bool {
    if polygon
        .iter()
        .any(|vertex| point_distance_mm(point, vertex) <= f64::EPSILON)
        || closed_edges(polygon).any(|edge| point_on_segment(point, edge.0, edge.1))
    {
        return true;
    }
    let mut inside = false;
    let mut previous = polygon.last().expect("polygon has at least one point");
    for current in polygon {
        let crosses_y = (current.y_mm > point.y_mm) != (previous.y_mm > point.y_mm);
        if crosses_y {
            let intersection_x = (previous.x_mm - current.x_mm) * (point.y_mm - current.y_mm)
                / (previous.y_mm - current.y_mm)
                + current.x_mm;
            if point.x_mm < intersection_x {
                inside = !inside;
            }
        }
        previous = current;
    }
    inside
}

fn polygon_signed_area_mm2(points: &[LayoutPoint]) -> f64 {
    closed_edges(points)
        .map(|edge| edge.0.x_mm * edge.1.y_mm - edge.1.x_mm * edge.0.y_mm)
        .sum::<f64>()
        / 2.0
}

fn oval_boundary_distance_mm(dx: f64, dy: f64, width_mm: f64, height_mm: f64) -> f64 {
    if width_mm >= height_mm {
        let radius = height_mm / 2.0;
        let segment_half = (width_mm - height_mm) / 2.0;
        if dx.abs() <= segment_half {
            radius - dy.abs()
        } else {
            radius - (dx.abs() - segment_half).hypot(dy)
        }
    } else {
        let radius = width_mm / 2.0;
        let segment_half = (height_mm - width_mm) / 2.0;
        if dy.abs() <= segment_half {
            radius - dx.abs()
        } else {
            radius - dx.hypot(dy.abs() - segment_half)
        }
    }
}

fn segment_to_axis_aligned_oval_distance_mm(
    start: &LayoutPoint,
    end: &LayoutPoint,
    center: &LayoutPoint,
    width_mm: f64,
    height_mm: f64,
) -> f64 {
    if width_mm >= height_mm {
        let radius = height_mm / 2.0;
        let half_segment = (width_mm - height_mm) / 2.0;
        let capsule_start = LayoutPoint {
            x_mm: center.x_mm - half_segment,
            y_mm: center.y_mm,
        };
        let capsule_end = LayoutPoint {
            x_mm: center.x_mm + half_segment,
            y_mm: center.y_mm,
        };
        (segment_to_segment_distance_mm(start, end, &capsule_start, &capsule_end) - radius).max(0.0)
    } else {
        let radius = width_mm / 2.0;
        let half_segment = (height_mm - width_mm) / 2.0;
        let capsule_start = LayoutPoint {
            x_mm: center.x_mm,
            y_mm: center.y_mm - half_segment,
        };
        let capsule_end = LayoutPoint {
            x_mm: center.x_mm,
            y_mm: center.y_mm + half_segment,
        };
        (segment_to_segment_distance_mm(start, end, &capsule_start, &capsule_end) - radius).max(0.0)
    }
}

fn segment_to_axis_aligned_rect_distance_mm(
    start: &LayoutPoint,
    end: &LayoutPoint,
    center: &LayoutPoint,
    half_x_mm: f64,
    half_y_mm: f64,
) -> f64 {
    let min_x = center.x_mm - half_x_mm;
    let max_x = center.x_mm + half_x_mm;
    let min_y = center.y_mm - half_y_mm;
    let max_y = center.y_mm + half_y_mm;
    if point_inside_rect(start, min_x, max_x, min_y, max_y)
        || point_inside_rect(end, min_x, max_x, min_y, max_y)
    {
        return 0.0;
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
    let rect_edges = [
        (&corners[0], &corners[1]),
        (&corners[1], &corners[2]),
        (&corners[2], &corners[3]),
        (&corners[3], &corners[0]),
    ];
    if rect_edges
        .iter()
        .any(|(first, second)| segments_intersect(start, end, first, second))
    {
        return 0.0;
    }
    let endpoint_distance = point_to_rect_distance_mm(start, min_x, max_x, min_y, max_y)
        .min(point_to_rect_distance_mm(end, min_x, max_x, min_y, max_y));
    corners
        .iter()
        .map(|corner| point_to_segment_distance_mm(corner, start, end))
        .fold(endpoint_distance, f64::min)
}

fn point_inside_rect(point: &LayoutPoint, min_x: f64, max_x: f64, min_y: f64, max_y: f64) -> bool {
    point.x_mm >= min_x && point.x_mm <= max_x && point.y_mm >= min_y && point.y_mm <= max_y
}

fn point_to_rect_distance_mm(
    point: &LayoutPoint,
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
) -> f64 {
    let dx = if point.x_mm < min_x {
        min_x - point.x_mm
    } else if point.x_mm > max_x {
        point.x_mm - max_x
    } else {
        0.0
    };
    let dy = if point.y_mm < min_y {
        min_y - point.y_mm
    } else if point.y_mm > max_y {
        point.y_mm - max_y
    } else {
        0.0
    };
    dx.hypot(dy)
}

pub(super) fn point_to_segment_distance_mm(
    point: &LayoutPoint,
    start: &LayoutPoint,
    end: &LayoutPoint,
) -> f64 {
    let dx = end.x_mm - start.x_mm;
    let dy = end.y_mm - start.y_mm;
    let length_squared = dx * dx + dy * dy;
    if length_squared <= f64::EPSILON {
        return point_distance_mm(point, start);
    }
    let t = (((point.x_mm - start.x_mm) * dx + (point.y_mm - start.y_mm) * dy) / length_squared)
        .clamp(0.0, 1.0);
    let projection = LayoutPoint {
        x_mm: start.x_mm + t * dx,
        y_mm: start.y_mm + t * dy,
    };
    point_distance_mm(point, &projection)
}

fn segment_to_segment_distance_mm(
    first_start: &LayoutPoint,
    first_end: &LayoutPoint,
    second_start: &LayoutPoint,
    second_end: &LayoutPoint,
) -> f64 {
    if segments_intersect(first_start, first_end, second_start, second_end) {
        return 0.0;
    }
    point_to_segment_distance_mm(first_start, second_start, second_end)
        .min(point_to_segment_distance_mm(
            first_end,
            second_start,
            second_end,
        ))
        .min(point_to_segment_distance_mm(
            second_start,
            first_start,
            first_end,
        ))
        .min(point_to_segment_distance_mm(
            second_end,
            first_start,
            first_end,
        ))
}

fn segments_intersect(
    first_start: &LayoutPoint,
    first_end: &LayoutPoint,
    second_start: &LayoutPoint,
    second_end: &LayoutPoint,
) -> bool {
    let o1 = orientation(first_start, first_end, second_start);
    let o2 = orientation(first_start, first_end, second_end);
    let o3 = orientation(second_start, second_end, first_start);
    let o4 = orientation(second_start, second_end, first_end);
    if o1.abs() <= f64::EPSILON && point_on_segment(second_start, first_start, first_end) {
        return true;
    }
    if o2.abs() <= f64::EPSILON && point_on_segment(second_end, first_start, first_end) {
        return true;
    }
    if o3.abs() <= f64::EPSILON && point_on_segment(first_start, second_start, second_end) {
        return true;
    }
    if o4.abs() <= f64::EPSILON && point_on_segment(first_end, second_start, second_end) {
        return true;
    }
    (o1 > 0.0) != (o2 > 0.0) && (o3 > 0.0) != (o4 > 0.0)
}

fn orientation(first: &LayoutPoint, second: &LayoutPoint, third: &LayoutPoint) -> f64 {
    (second.x_mm - first.x_mm) * (third.y_mm - first.y_mm)
        - (second.y_mm - first.y_mm) * (third.x_mm - first.x_mm)
}

fn point_on_segment(point: &LayoutPoint, start: &LayoutPoint, end: &LayoutPoint) -> bool {
    orientation(start, end, point).abs() <= f64::EPSILON
        && point.x_mm >= start.x_mm.min(end.x_mm) - f64::EPSILON
        && point.x_mm <= start.x_mm.max(end.x_mm) + f64::EPSILON
        && point.y_mm >= start.y_mm.min(end.y_mm) - f64::EPSILON
        && point.y_mm <= start.y_mm.max(end.y_mm) + f64::EPSILON
}

pub(super) fn point_distance_mm(first: &LayoutPoint, second: &LayoutPoint) -> f64 {
    (second.x_mm - first.x_mm).hypot(second.y_mm - first.y_mm)
}
