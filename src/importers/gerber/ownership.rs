use crate::board_ir::{BoardLayout, CopperZone, LayoutPad, LayoutPoint, NetRoute};

use super::{
    GerberCopper, GerberCopperFeature, GerberCopperRegion, GerberCopperSegment, GerberPoint,
    POINT_EPSILON_MM, point_distance_mm, point_inside_polygon, polygon_signed_area_mm2,
};

#[derive(Debug, Clone)]
struct CopperNetOwner {
    net: String,
}

#[derive(Debug, Clone)]
struct CopperOwnerPad {
    at: GerberPoint,
    layers: Vec<String>,
    net: String,
    shape: Option<String>,
    size_x_mm: Option<f64>,
    size_y_mm: Option<f64>,
}

#[derive(Debug, Clone)]
struct CopperOwnerRouteSegment {
    start: GerberPoint,
    end: GerberPoint,
    layer: String,
    width_mm: f64,
    net: String,
}

#[derive(Debug, Clone)]
struct CopperOwnerZone {
    layer: String,
    net: String,
    polygons: Vec<Vec<GerberPoint>>,
}

#[derive(Debug, Clone, Default)]
struct CopperOwnershipIndex {
    pads: Vec<CopperOwnerPad>,
    route_segments: Vec<CopperOwnerRouteSegment>,
    zones: Vec<CopperOwnerZone>,
}

pub(super) fn associate_copper_nets(copper: &mut GerberCopper, layout: &BoardLayout) {
    let ownership = CopperOwnershipIndex::from_layout(layout);
    for feature in &mut copper.features {
        feature.net = ownership
            .owner_for_feature(feature, &copper.layer)
            .map(|owner| owner.net);
    }
    for segment in &mut copper.segments {
        segment.net = ownership
            .owner_for_segment(segment, &copper.layer)
            .map(|owner| owner.net);
    }
    for region in &mut copper.regions {
        region.net = ownership
            .owner_for_region(region, &copper.layer)
            .map(|owner| owner.net);
    }
}

impl CopperOwnershipIndex {
    fn from_layout(layout: &BoardLayout) -> Self {
        let pads = layout
            .pads
            .values()
            .flat_map(|component_pads| component_pads.values())
            .filter_map(owner_pad_from_layout_pad)
            .collect();
        let route_segments = layout
            .routes
            .iter()
            .flat_map(|(net, route)| owner_route_segments_from_route(net, route))
            .collect();
        let zones = layout
            .zones
            .iter()
            .flat_map(|(net, zones)| owner_zones_from_zones(net, zones))
            .collect();
        Self {
            pads,
            route_segments,
            zones,
        }
    }

    fn owner_for_feature(
        &self,
        feature: &GerberCopperFeature,
        layer: &str,
    ) -> Option<CopperNetOwner> {
        let mut candidates = Vec::new();
        for pad in self.pads.iter().filter(|pad| pad_on_layer(pad, layer)) {
            if point_inside_feature_pad(feature.at, pad) {
                candidates.push(pad.net.clone());
            }
        }
        for route in self
            .route_segments
            .iter()
            .filter(|route| route.layer == layer)
        {
            let feature_radius = feature.aperture.x_mm.min(feature.aperture.y_mm) / 2.0;
            if point_to_segment_distance_mm(feature.at, route.start, route.end)
                <= feature_radius + route.width_mm / 2.0 + POINT_EPSILON_MM
            {
                candidates.push(route.net.clone());
            }
        }
        for zone in self.zones.iter().filter(|zone| zone.layer == layer) {
            if zone
                .polygons
                .iter()
                .any(|polygon| point_inside_polygon(feature.at, polygon))
            {
                candidates.push(zone.net.clone());
            }
        }
        unique_owner(candidates)
    }

    fn owner_for_segment(
        &self,
        segment: &GerberCopperSegment,
        layer: &str,
    ) -> Option<CopperNetOwner> {
        let mut candidates = Vec::new();
        for route in self
            .route_segments
            .iter()
            .filter(|route| route.layer == layer)
        {
            if segment_to_segment_distance_mm(segment.start, segment.end, route.start, route.end)
                <= segment.aperture.x_mm / 2.0 + route.width_mm / 2.0 + POINT_EPSILON_MM
            {
                candidates.push(route.net.clone());
            }
        }
        for pad in self.pads.iter().filter(|pad| pad_on_layer(pad, layer)) {
            if point_to_segment_distance_mm(pad.at, segment.start, segment.end)
                <= segment.aperture.x_mm / 2.0 + POINT_EPSILON_MM
            {
                candidates.push(pad.net.clone());
            }
        }
        for zone in self.zones.iter().filter(|zone| zone.layer == layer) {
            let midpoint = GerberPoint {
                x_mm: (segment.start.x_mm + segment.end.x_mm) / 2.0,
                y_mm: (segment.start.y_mm + segment.end.y_mm) / 2.0,
            };
            if zone
                .polygons
                .iter()
                .any(|polygon| point_inside_polygon(midpoint, polygon))
            {
                candidates.push(zone.net.clone());
            }
        }
        unique_owner(candidates)
    }

    fn owner_for_region(&self, region: &GerberCopperRegion, layer: &str) -> Option<CopperNetOwner> {
        let representative = polygon_representative_point(&region.points);
        let mut candidates = Vec::new();
        for zone in self.zones.iter().filter(|zone| zone.layer == layer) {
            if zone
                .polygons
                .iter()
                .any(|polygon| point_inside_polygon(representative, polygon))
            {
                candidates.push(zone.net.clone());
            }
        }
        for pad in self.pads.iter().filter(|pad| pad_on_layer(pad, layer)) {
            if point_inside_polygon(pad.at, &region.points) {
                candidates.push(pad.net.clone());
            }
        }
        unique_owner(candidates)
    }
}

fn owner_pad_from_layout_pad(pad: &LayoutPad) -> Option<CopperOwnerPad> {
    if !finite_gerber_point(point_from_layout(&pad.at)) || pad.net.trim().is_empty() {
        return None;
    }
    Some(CopperOwnerPad {
        at: point_from_layout(&pad.at),
        layers: pad.layers.clone(),
        net: pad.net.clone(),
        shape: pad.shape.clone(),
        size_x_mm: pad.size.as_ref().map(|size| size.x_mm),
        size_y_mm: pad.size.as_ref().map(|size| size.y_mm),
    })
}

fn owner_route_segments_from_route<'a>(
    net: &'a str,
    route: &'a NetRoute,
) -> impl Iterator<Item = CopperOwnerRouteSegment> + 'a {
    route.segments.iter().filter_map(move |segment| {
        if net.trim().is_empty()
            || !finite_gerber_point(point_from_layout(&segment.start))
            || !finite_gerber_point(point_from_layout(&segment.end))
            || !segment.width_mm.is_finite()
            || segment.width_mm <= 0.0
        {
            return None;
        }
        Some(CopperOwnerRouteSegment {
            start: point_from_layout(&segment.start),
            end: point_from_layout(&segment.end),
            layer: segment.layer.clone(),
            width_mm: segment.width_mm,
            net: net.to_string(),
        })
    })
}

fn owner_zones_from_zones(net: &str, zones: &[CopperZone]) -> Vec<CopperOwnerZone> {
    let mut owner_zones = Vec::new();
    for zone in zones {
        if net.trim().is_empty() {
            continue;
        }
        let polygons = if zone.filled_polygons.is_empty() {
            vec![points_from_layout(&zone.polygon)]
        } else {
            zone.filled_polygons
                .iter()
                .map(|polygon| points_from_layout(polygon))
                .collect()
        };
        let polygons = polygons
            .into_iter()
            .filter(|polygon| {
                polygon.len() >= 3 && polygon_signed_area_mm2(polygon).abs() > f64::EPSILON
            })
            .collect::<Vec<_>>();
        if !polygons.is_empty() {
            owner_zones.push(CopperOwnerZone {
                layer: zone.layer.clone(),
                net: net.to_string(),
                polygons,
            });
        }
    }
    owner_zones
}

fn unique_owner(candidates: Vec<String>) -> Option<CopperNetOwner> {
    let mut unique = candidates
        .into_iter()
        .filter(|candidate| !candidate.trim().is_empty())
        .collect::<Vec<_>>();
    unique.sort();
    unique.dedup();
    (unique.len() == 1).then(|| CopperNetOwner {
        net: unique.pop().expect("unique contains one element"),
    })
}

fn pad_on_layer(pad: &CopperOwnerPad, layer: &str) -> bool {
    pad.layers.is_empty() || pad.layers.iter().any(|candidate| candidate == layer)
}

fn point_inside_feature_pad(point: GerberPoint, pad: &CopperOwnerPad) -> bool {
    let Some(size_x_mm) = pad.size_x_mm else {
        return point_distance_mm(point, pad.at) <= 0.05;
    };
    let Some(size_y_mm) = pad.size_y_mm else {
        return point_distance_mm(point, pad.at) <= 0.05;
    };
    if !size_x_mm.is_finite() || !size_y_mm.is_finite() || size_x_mm <= 0.0 || size_y_mm <= 0.0 {
        return false;
    }
    let dx = (point.x_mm - pad.at.x_mm).abs();
    let dy = (point.y_mm - pad.at.y_mm).abs();
    match pad.shape.as_deref() {
        Some("circle") => dx.hypot(dy) <= size_x_mm.min(size_y_mm) / 2.0 + POINT_EPSILON_MM,
        Some("oval") => point_inside_axis_aligned_oval(dx, dy, size_x_mm, size_y_mm),
        Some("rect") | None => {
            dx <= size_x_mm / 2.0 + POINT_EPSILON_MM && dy <= size_y_mm / 2.0 + POINT_EPSILON_MM
        }
        Some(_) => point_distance_mm(point, pad.at) <= 0.05,
    }
}

fn point_inside_axis_aligned_oval(dx: f64, dy: f64, width_mm: f64, height_mm: f64) -> bool {
    if width_mm >= height_mm {
        let radius = height_mm / 2.0;
        let segment_half = (width_mm - height_mm) / 2.0;
        if dx <= segment_half {
            dy <= radius + POINT_EPSILON_MM
        } else {
            (dx - segment_half).hypot(dy) <= radius + POINT_EPSILON_MM
        }
    } else {
        let radius = width_mm / 2.0;
        let segment_half = (height_mm - width_mm) / 2.0;
        if dy <= segment_half {
            dx <= radius + POINT_EPSILON_MM
        } else {
            dx.hypot(dy - segment_half) <= radius + POINT_EPSILON_MM
        }
    }
}

fn point_from_layout(point: &LayoutPoint) -> GerberPoint {
    GerberPoint {
        x_mm: point.x_mm,
        y_mm: point.y_mm,
    }
}

fn points_from_layout(points: &[LayoutPoint]) -> Vec<GerberPoint> {
    points.iter().map(point_from_layout).collect()
}

fn finite_gerber_point(point: GerberPoint) -> bool {
    point.x_mm.is_finite() && point.y_mm.is_finite()
}

fn polygon_representative_point(points: &[GerberPoint]) -> GerberPoint {
    let count = points.len() as f64;
    GerberPoint {
        x_mm: points.iter().map(|point| point.x_mm).sum::<f64>() / count,
        y_mm: points.iter().map(|point| point.y_mm).sum::<f64>() / count,
    }
}

fn point_to_segment_distance_mm(point: GerberPoint, start: GerberPoint, end: GerberPoint) -> f64 {
    let dx = end.x_mm - start.x_mm;
    let dy = end.y_mm - start.y_mm;
    let length_squared = dx * dx + dy * dy;
    if length_squared <= f64::EPSILON {
        return point_distance_mm(point, start);
    }
    let t = (((point.x_mm - start.x_mm) * dx + (point.y_mm - start.y_mm) * dy) / length_squared)
        .clamp(0.0, 1.0);
    let projection = GerberPoint {
        x_mm: start.x_mm + t * dx,
        y_mm: start.y_mm + t * dy,
    };
    point_distance_mm(point, projection)
}

fn segment_to_segment_distance_mm(
    first_start: GerberPoint,
    first_end: GerberPoint,
    second_start: GerberPoint,
    second_end: GerberPoint,
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
    first_start: GerberPoint,
    first_end: GerberPoint,
    second_start: GerberPoint,
    second_end: GerberPoint,
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

fn orientation(first: GerberPoint, second: GerberPoint, third: GerberPoint) -> f64 {
    (second.x_mm - first.x_mm) * (third.y_mm - first.y_mm)
        - (second.y_mm - first.y_mm) * (third.x_mm - first.x_mm)
}

fn point_on_segment(point: GerberPoint, start: GerberPoint, end: GerberPoint) -> bool {
    orientation(start, end, point).abs() <= f64::EPSILON
        && point.x_mm >= start.x_mm.min(end.x_mm) - f64::EPSILON
        && point.x_mm <= start.x_mm.max(end.x_mm) + f64::EPSILON
        && point.y_mm >= start.y_mm.min(end.y_mm) - f64::EPSILON
        && point.y_mm <= start.y_mm.max(end.y_mm) + f64::EPSILON
}
