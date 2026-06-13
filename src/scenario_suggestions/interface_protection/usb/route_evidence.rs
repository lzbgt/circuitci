use super::super::super::{
    SuggestedProtectionClamp, SuggestedUsbFilledZoneClearanceSegment,
    SuggestedUsbGroundZoneContact, SuggestedUsbRoutePad, SuggestedUsbRoutePadSize,
    SuggestedUsbUnreferencedSegment,
};
use crate::board_ir::{
    ComponentSpec, CopperZone, LayoutPad, LayoutPoint, NetKind, NetRoute, RouteSegment, RouteVia,
};
use crate::library::{BoundBoard, UsbConnector};
use std::collections::BTreeMap;

pub(super) fn return_path_unreferenced_segments(
    route: &NetRoute,
    ground_zones: &[GroundZoneEvidence<'_>],
    geometry: GroundReferenceGeometry,
) -> (f64, Vec<SuggestedUsbUnreferencedSegment>) {
    let mut unreferenced_length_mm = 0.0;
    let mut unreferenced_segments = Vec::new();
    for (segment_index, segment) in route.segments.iter().enumerate() {
        let midpoint_x_mm = (segment.start.x_mm + segment.end.x_mm) / 2.0;
        let midpoint_y_mm = (segment.start.y_mm + segment.end.y_mm) / 2.0;
        let referenced = ground_zones.iter().any(|zone| {
            zone.zone.layer == segment.layer
                && point_inside_ground_reference(midpoint_x_mm, midpoint_y_mm, zone, geometry)
        });
        if referenced {
            continue;
        }
        let segment_length_mm = segment_length_mm(segment);
        unreferenced_length_mm += segment_length_mm;
        unreferenced_segments.push(SuggestedUsbUnreferencedSegment {
            segment_index,
            segment_length_mm,
            midpoint_x_mm,
            midpoint_y_mm,
            layer: segment.layer.clone(),
        });
    }
    (unreferenced_length_mm, unreferenced_segments)
}

pub(super) fn return_path_filled_zone_clearance_segments(
    route: &NetRoute,
    ground_zones: &[GroundZoneEvidence<'_>],
) -> Vec<SuggestedUsbFilledZoneClearanceSegment> {
    route
        .segments
        .iter()
        .enumerate()
        .map(|(segment_index, segment)| {
            let midpoint_x_mm = (segment.start.x_mm + segment.end.x_mm) / 2.0;
            let midpoint_y_mm = (segment.start.y_mm + segment.end.y_mm) / 2.0;
            let filled_zone_edge_clearance_mm = ground_zones
                .iter()
                .filter(|zone| zone.zone.layer == segment.layer)
                .filter_map(|zone| {
                    point_clearance_to_any_filled_polygon_edge(
                        midpoint_x_mm,
                        midpoint_y_mm,
                        zone.zone,
                    )
                })
                .max_by(|left, right| left.total_cmp(right));
            SuggestedUsbFilledZoneClearanceSegment {
                segment_index,
                segment_length_mm: segment_length_mm(segment),
                midpoint_x_mm,
                midpoint_y_mm,
                layer: segment.layer.clone(),
                filled_zone_edge_clearance_mm,
            }
        })
        .collect()
}

#[derive(Debug, Clone, Copy)]
pub(super) enum GroundReferenceGeometry {
    Outline,
    FilledPolygon,
}

fn point_inside_ground_reference(
    point_x_mm: f64,
    point_y_mm: f64,
    zone: &GroundZoneEvidence<'_>,
    geometry: GroundReferenceGeometry,
) -> bool {
    match geometry {
        GroundReferenceGeometry::Outline => {
            point_inside_zone_outline(point_x_mm, point_y_mm, zone.zone)
        }
        GroundReferenceGeometry::FilledPolygon => {
            point_inside_any_filled_polygon(point_x_mm, point_y_mm, zone.zone)
        }
    }
}

pub(super) fn ground_zones_have_filled_polygons(ground_zones: &[GroundZoneEvidence<'_>]) -> bool {
    ground_zones.iter().any(|zone| {
        zone.zone
            .filled_polygons
            .iter()
            .any(|polygon| polygon_is_usable(polygon))
    })
}

#[derive(Debug, Clone, Copy)]
pub(super) struct GroundZoneEvidence<'a> {
    net_name: &'a str,
    zone: &'a CopperZone,
}

pub(super) fn ground_zone_outlines<'a>(bound: &'a BoundBoard<'_>) -> Vec<GroundZoneEvidence<'a>> {
    let mut zones = Vec::new();
    for (net_name, zone_list) in &bound.project.board.layout.zones {
        let Some(net) = bound.project.board.nets.get(net_name) else {
            continue;
        };
        if net.kind != NetKind::Ground {
            continue;
        }
        zones.extend(
            zone_list
                .iter()
                .filter(|zone| zone_outline_is_usable(zone))
                .map(|zone| GroundZoneEvidence { net_name, zone }),
        );
    }
    zones
}

pub(super) fn route_ground_zone_contacts(
    bound: &BoundBoard<'_>,
    route: &NetRoute,
    ground_zones: &[GroundZoneEvidence<'_>],
    geometry: GroundReferenceGeometry,
) -> Vec<SuggestedUsbGroundZoneContact> {
    let mut contacts = BTreeMap::<String, SuggestedUsbGroundZoneContact>::new();
    for segment in &route.segments {
        let midpoint_x_mm = (segment.start.x_mm + segment.end.x_mm) / 2.0;
        let midpoint_y_mm = (segment.start.y_mm + segment.end.y_mm) / 2.0;
        for zone in ground_zones.iter().filter(|zone| {
            zone.zone.layer == segment.layer
                && point_inside_ground_reference(midpoint_x_mm, midpoint_y_mm, zone, geometry)
        }) {
            for contact in ground_zone_contacts(bound, zone, geometry, midpoint_x_mm, midpoint_y_mm)
            {
                contacts.entry(contact_key(&contact)).or_insert(contact);
            }
        }
    }
    contacts.into_values().collect()
}

pub(super) fn usb_vbus_route_pad_contact_evidence_exists(
    bound: &BoundBoard<'_>,
    connector_id: &str,
    component: &ComponentSpec,
    connector: &UsbConnector,
    vbus_clamp: &SuggestedProtectionClamp,
) -> bool {
    route_pad_exists(
        bound,
        connector_id,
        &connector.vbus_pin,
        component.pins.get(&connector.vbus_pin).map(String::as_str),
    ) && route_pad_exists(
        bound,
        &vbus_clamp.component,
        &vbus_clamp.protected_pin,
        Some(vbus_clamp.protected_net.as_str()),
    )
}

fn route_pad_exists(
    bound: &BoundBoard<'_>,
    component_id: &str,
    pin: &str,
    expected_net: Option<&str>,
) -> bool {
    let Some(expected_net) = expected_net else {
        return false;
    };
    route_pad_for_pin(bound, component_id, pin, expected_net).is_some()
}

pub(super) fn suggested_usb_route_pad(
    bound: &BoundBoard<'_>,
    component_id: &str,
    pin: &str,
    expected_net: &str,
) -> Option<SuggestedUsbRoutePad> {
    let (pad_name, pad) = route_pad_for_pin(bound, component_id, pin, expected_net)?;
    Some(SuggestedUsbRoutePad {
        component: component_id.to_string(),
        pin: pad_name.to_string(),
        net: pad.net.clone(),
        x_mm: pad.at.x_mm,
        y_mm: pad.at.y_mm,
        layers: pad.layers.clone(),
        kind: pad.kind.clone(),
        shape: pad.shape.clone(),
        size: pad.size.as_ref().map(|size| SuggestedUsbRoutePadSize {
            x_mm: size.x_mm,
            y_mm: size.y_mm,
        }),
        rotation_deg: pad.rotation_deg,
        drill_mm: pad.drill_mm,
    })
}

fn route_pad_for_pin<'a>(
    bound: &'a BoundBoard<'_>,
    component_id: &str,
    pin: &str,
    expected_net: &str,
) -> Option<(&'a str, &'a LayoutPad)> {
    let pads = bound.project.board.layout.pads.get(component_id)?;
    if let Some((pad_name, pad)) = pads.get_key_value(pin)
        && pad.net == expected_net
        && pad.at.x_mm.is_finite()
        && pad.at.y_mm.is_finite()
    {
        return Some((pad_name.as_str(), pad));
    }
    let mut matching_pads = pads.iter().filter(|(_, pad)| {
        pad.net == expected_net && pad.at.x_mm.is_finite() && pad.at.y_mm.is_finite()
    });
    let (pad_name, pad) = matching_pads.next()?;
    matching_pads
        .next()
        .is_none()
        .then_some((pad_name.as_str(), pad))
}

pub(super) fn pad_to_route_distance_mm(
    route: &NetRoute,
    pad: &SuggestedUsbRoutePad,
) -> Option<f64> {
    nearest_pad_projection(route, pad).map(|projection| projection.distance_to_pad_mm)
}

pub(super) fn route_distance_between_pads_mm(
    route: &NetRoute,
    from: &SuggestedUsbRoutePad,
    to: &SuggestedUsbRoutePad,
) -> Option<f64> {
    let from_projection = nearest_pad_projection(route, from)?;
    let to_projection = nearest_pad_projection(route, to)?;
    shortest_route_distance_mm(route, &from_projection, &to_projection)
}

#[derive(Debug, Clone, Copy)]
struct PadProjection {
    segment_index: usize,
    t: f64,
    x_mm: f64,
    y_mm: f64,
    distance_to_pad_mm: f64,
}

fn nearest_pad_projection(route: &NetRoute, pad: &SuggestedUsbRoutePad) -> Option<PadProjection> {
    if pad_has_supported_extent(pad) {
        return route
            .segments
            .iter()
            .enumerate()
            .filter(|(_, segment)| pad_layers_include(&pad.layers, segment.layer.as_str()))
            .filter(|(_, segment)| segment_touches_pad(segment, pad))
            .filter_map(|(segment_index, segment)| {
                project_point_to_segment(
                    pad.x_mm,
                    pad.y_mm,
                    segment.start.x_mm,
                    segment.start.y_mm,
                    segment.end.x_mm,
                    segment.end.y_mm,
                )
                .map(|projection| {
                    let center_distance_mm =
                        (pad.x_mm - projection.x_mm).hypot(pad.y_mm - projection.y_mm);
                    (
                        center_distance_mm,
                        PadProjection {
                            segment_index,
                            t: projection.t,
                            x_mm: projection.x_mm,
                            y_mm: projection.y_mm,
                            distance_to_pad_mm: 0.0,
                        },
                    )
                })
            })
            .min_by(|left, right| left.0.total_cmp(&right.0))
            .map(|(_, projection)| projection);
    }

    route
        .segments
        .iter()
        .enumerate()
        .filter(|(_, segment)| pad_layers_include(&pad.layers, segment.layer.as_str()))
        .filter_map(|(segment_index, segment)| {
            project_point_to_segment(
                pad.x_mm,
                pad.y_mm,
                segment.start.x_mm,
                segment.start.y_mm,
                segment.end.x_mm,
                segment.end.y_mm,
            )
            .map(|projection| PadProjection {
                segment_index,
                t: projection.t,
                x_mm: projection.x_mm,
                y_mm: projection.y_mm,
                distance_to_pad_mm: (pad.x_mm - projection.x_mm).hypot(pad.y_mm - projection.y_mm),
            })
        })
        .min_by(|left, right| left.distance_to_pad_mm.total_cmp(&right.distance_to_pad_mm))
}

fn pad_has_supported_extent(pad: &SuggestedUsbRoutePad) -> bool {
    let Some(size) = &pad.size else {
        return false;
    };
    if size.x_mm <= 0.0 || size.y_mm <= 0.0 || !size.x_mm.is_finite() || !size.y_mm.is_finite() {
        return false;
    }
    pad.shape.as_deref().is_some_and(|shape| {
        matches!(
            shape.trim().to_ascii_lowercase().as_str(),
            "rect" | "circle" | "oval"
        )
    })
}

fn segment_touches_pad(segment: &RouteSegment, pad: &SuggestedUsbRoutePad) -> bool {
    let Some(size) = &pad.size else {
        return false;
    };
    let Some(shape) = pad.shape.as_deref().map(str::to_ascii_lowercase) else {
        return false;
    };
    let route_half_width_mm = segment.width_mm / 2.0;
    let start = point_to_pad_local(segment.start.x_mm, segment.start.y_mm, pad);
    let end = point_to_pad_local(segment.end.x_mm, segment.end.y_mm, pad);
    match shape.trim() {
        "rect" => segment_intersects_axis_aligned_rect(
            start,
            end,
            size.x_mm / 2.0 + route_half_width_mm,
            size.y_mm / 2.0 + route_half_width_mm,
        ),
        "circle" => {
            let radius_mm = size.x_mm.min(size.y_mm) / 2.0 + route_half_width_mm;
            point_to_segment_distance_mm(0.0, 0.0, start.0, start.1, end.0, end.1)
                .is_some_and(|distance_mm| distance_mm <= radius_mm)
        }
        "oval" => segment_touches_oval_pad(start, end, size.x_mm, size.y_mm, route_half_width_mm),
        _ => false,
    }
}

fn point_to_pad_local(x_mm: f64, y_mm: f64, pad: &SuggestedUsbRoutePad) -> (f64, f64) {
    let dx = x_mm - pad.x_mm;
    let dy = y_mm - pad.y_mm;
    let radians = -pad.rotation_deg.unwrap_or(0.0).to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    (dx * cos - dy * sin, dx * sin + dy * cos)
}

fn segment_touches_oval_pad(
    start: (f64, f64),
    end: (f64, f64),
    size_x_mm: f64,
    size_y_mm: f64,
    route_half_width_mm: f64,
) -> bool {
    if (size_x_mm - size_y_mm).abs() <= f64::EPSILON {
        let radius_mm = size_x_mm.min(size_y_mm) / 2.0 + route_half_width_mm;
        return point_to_segment_distance_mm(0.0, 0.0, start.0, start.1, end.0, end.1)
            .is_some_and(|distance_mm| distance_mm <= radius_mm);
    }
    if size_x_mm > size_y_mm {
        let radius_mm = size_y_mm / 2.0 + route_half_width_mm;
        let half_straight_mm = (size_x_mm - size_y_mm) / 2.0;
        segment_to_segment_distance_mm(
            start,
            end,
            (-half_straight_mm, 0.0),
            (half_straight_mm, 0.0),
        )
        .is_some_and(|distance_mm| distance_mm <= radius_mm)
    } else {
        let radius_mm = size_x_mm / 2.0 + route_half_width_mm;
        let half_straight_mm = (size_y_mm - size_x_mm) / 2.0;
        segment_to_segment_distance_mm(
            start,
            end,
            (0.0, -half_straight_mm),
            (0.0, half_straight_mm),
        )
        .is_some_and(|distance_mm| distance_mm <= radius_mm)
    }
}

fn segment_intersects_axis_aligned_rect(
    start: (f64, f64),
    end: (f64, f64),
    half_width_mm: f64,
    half_height_mm: f64,
) -> bool {
    if half_width_mm <= 0.0 || half_height_mm <= 0.0 {
        return false;
    }
    if point_inside_axis_aligned_rect(start, half_width_mm, half_height_mm)
        || point_inside_axis_aligned_rect(end, half_width_mm, half_height_mm)
    {
        return true;
    }
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let mut t_min = 0.0;
    let mut t_max = 1.0;
    clip_segment_to_slab(-dx, start.0 + half_width_mm, &mut t_min, &mut t_max)
        && clip_segment_to_slab(dx, half_width_mm - start.0, &mut t_min, &mut t_max)
        && clip_segment_to_slab(-dy, start.1 + half_height_mm, &mut t_min, &mut t_max)
        && clip_segment_to_slab(dy, half_height_mm - start.1, &mut t_min, &mut t_max)
}

fn point_inside_axis_aligned_rect(
    point: (f64, f64),
    half_width_mm: f64,
    half_height_mm: f64,
) -> bool {
    point.0.abs() <= half_width_mm + f64::EPSILON && point.1.abs() <= half_height_mm + f64::EPSILON
}

fn clip_segment_to_slab(p: f64, q: f64, t_min: &mut f64, t_max: &mut f64) -> bool {
    if p.abs() <= f64::EPSILON {
        return q >= -f64::EPSILON;
    }
    let r = q / p;
    if p < 0.0 {
        if r > *t_max {
            return false;
        }
        if r > *t_min {
            *t_min = r;
        }
    } else {
        if r < *t_min {
            return false;
        }
        if r < *t_max {
            *t_max = r;
        }
    }
    true
}

fn segment_to_segment_distance_mm(
    first_start: (f64, f64),
    first_end: (f64, f64),
    second_start: (f64, f64),
    second_end: (f64, f64),
) -> Option<f64> {
    if segments_intersect(first_start, first_end, second_start, second_end) {
        return Some(0.0);
    }
    Some(
        [
            point_to_segment_distance_mm(
                first_start.0,
                first_start.1,
                second_start.0,
                second_start.1,
                second_end.0,
                second_end.1,
            )?,
            point_to_segment_distance_mm(
                first_end.0,
                first_end.1,
                second_start.0,
                second_start.1,
                second_end.0,
                second_end.1,
            )?,
            point_to_segment_distance_mm(
                second_start.0,
                second_start.1,
                first_start.0,
                first_start.1,
                first_end.0,
                first_end.1,
            )?,
            point_to_segment_distance_mm(
                second_end.0,
                second_end.1,
                first_start.0,
                first_start.1,
                first_end.0,
                first_end.1,
            )?,
        ]
        .into_iter()
        .fold(f64::INFINITY, f64::min),
    )
}

fn segments_intersect(
    first_start: (f64, f64),
    first_end: (f64, f64),
    second_start: (f64, f64),
    second_end: (f64, f64),
) -> bool {
    let first_min_x = first_start.0.min(first_end.0);
    let first_max_x = first_start.0.max(first_end.0);
    let first_min_y = first_start.1.min(first_end.1);
    let first_max_y = first_start.1.max(first_end.1);
    let second_min_x = second_start.0.min(second_end.0);
    let second_max_x = second_start.0.max(second_end.0);
    let second_min_y = second_start.1.min(second_end.1);
    let second_max_y = second_start.1.max(second_end.1);
    if first_max_x + f64::EPSILON < second_min_x
        || second_max_x + f64::EPSILON < first_min_x
        || first_max_y + f64::EPSILON < second_min_y
        || second_max_y + f64::EPSILON < first_min_y
    {
        return false;
    }
    let first_second_start = orientation(first_start, first_end, second_start);
    let first_second_end = orientation(first_start, first_end, second_end);
    let second_first_start = orientation(second_start, second_end, first_start);
    let second_first_end = orientation(second_start, second_end, first_end);
    if first_second_start.abs() <= f64::EPSILON
        && point_on_segment(
            second_start.0,
            second_start.1,
            first_start.0,
            first_start.1,
            first_end.0,
            first_end.1,
        )
    {
        return true;
    }
    if first_second_end.abs() <= f64::EPSILON
        && point_on_segment(
            second_end.0,
            second_end.1,
            first_start.0,
            first_start.1,
            first_end.0,
            first_end.1,
        )
    {
        return true;
    }
    if second_first_start.abs() <= f64::EPSILON
        && point_on_segment(
            first_start.0,
            first_start.1,
            second_start.0,
            second_start.1,
            second_end.0,
            second_end.1,
        )
    {
        return true;
    }
    if second_first_end.abs() <= f64::EPSILON
        && point_on_segment(
            first_end.0,
            first_end.1,
            second_start.0,
            second_start.1,
            second_end.0,
            second_end.1,
        )
    {
        return true;
    }
    (first_second_start > 0.0) != (first_second_end > 0.0)
        && (second_first_start > 0.0) != (second_first_end > 0.0)
}

fn orientation(a: (f64, f64), b: (f64, f64), c: (f64, f64)) -> f64 {
    (b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)
}

#[derive(Debug, Clone, Copy)]
struct SegmentProjection {
    t: f64,
    x_mm: f64,
    y_mm: f64,
}

fn project_point_to_segment(
    point_x_mm: f64,
    point_y_mm: f64,
    start_x_mm: f64,
    start_y_mm: f64,
    end_x_mm: f64,
    end_y_mm: f64,
) -> Option<SegmentProjection> {
    let dx = end_x_mm - start_x_mm;
    let dy = end_y_mm - start_y_mm;
    let length_squared = dx.mul_add(dx, dy * dy);
    if length_squared <= f64::EPSILON {
        return None;
    }
    let raw_t = ((point_x_mm - start_x_mm) * dx + (point_y_mm - start_y_mm) * dy) / length_squared;
    let t = raw_t.clamp(0.0, 1.0);
    Some(SegmentProjection {
        t,
        x_mm: start_x_mm + t * dx,
        y_mm: start_y_mm + t * dy,
    })
}

fn shortest_route_distance_mm(
    route: &NetRoute,
    from: &PadProjection,
    to: &PadProjection,
) -> Option<f64> {
    let mut graph = RouteGraph::default();
    for (segment_index, segment) in route.segments.iter().enumerate() {
        add_segment_to_graph(&mut graph, segment_index, segment, from, to);
    }
    let from_node = graph.find_node(from.x_mm, from.y_mm)?;
    let to_node = graph.find_node(to.x_mm, to.y_mm)?;
    graph.shortest_distance(from_node, to_node)
}

#[derive(Default)]
struct RouteGraph {
    nodes: Vec<(f64, f64)>,
    edges: Vec<Vec<(usize, f64)>>,
}

impl RouteGraph {
    fn node_for(&mut self, x_mm: f64, y_mm: f64) -> usize {
        if let Some(index) = self.find_node(x_mm, y_mm) {
            return index;
        }
        let index = self.nodes.len();
        self.nodes.push((x_mm, y_mm));
        self.edges.push(Vec::new());
        index
    }

    fn find_node(&self, x_mm: f64, y_mm: f64) -> Option<usize> {
        self.nodes.iter().position(|(candidate_x, candidate_y)| {
            points_equal(*candidate_x, *candidate_y, x_mm, y_mm)
        })
    }

    fn connect(&mut self, a: usize, b: usize, distance_mm: f64) {
        if a == b || distance_mm <= f64::EPSILON {
            return;
        }
        self.edges[a].push((b, distance_mm));
        self.edges[b].push((a, distance_mm));
    }

    fn shortest_distance(&self, start: usize, end: usize) -> Option<f64> {
        let mut distances = vec![f64::INFINITY; self.nodes.len()];
        let mut visited = vec![false; self.nodes.len()];
        distances[start] = 0.0;
        loop {
            let Some(current) = distances
                .iter()
                .enumerate()
                .filter(|(index, _)| !visited[*index])
                .min_by(|(_, left), (_, right)| left.total_cmp(right))
                .map(|(index, _)| index)
            else {
                break;
            };
            if current == end {
                return Some(distances[current]);
            }
            visited[current] = true;
            for (next, edge_distance) in &self.edges[current] {
                let candidate = distances[current] + edge_distance;
                if candidate < distances[*next] {
                    distances[*next] = candidate;
                }
            }
        }
        distances[end].is_finite().then_some(distances[end])
    }
}

fn add_segment_to_graph(
    graph: &mut RouteGraph,
    segment_index: usize,
    segment: &RouteSegment,
    from: &PadProjection,
    to: &PadProjection,
) {
    let mut points = vec![
        (0.0, segment.start.x_mm, segment.start.y_mm),
        (1.0, segment.end.x_mm, segment.end.y_mm),
    ];
    if from.segment_index == segment_index {
        points.push((from.t, from.x_mm, from.y_mm));
    }
    if to.segment_index == segment_index {
        points.push((to.t, to.x_mm, to.y_mm));
    }
    points.sort_by(|left, right| left.0.total_cmp(&right.0));
    points.dedup_by(|left, right| points_equal(left.1, left.2, right.1, right.2));
    for window in points.windows(2) {
        let first = graph.node_for(window[0].1, window[0].2);
        let second = graph.node_for(window[1].1, window[1].2);
        graph.connect(
            first,
            second,
            (window[0].1 - window[1].1).hypot(window[0].2 - window[1].2),
        );
    }
}

fn points_equal(a_x_mm: f64, a_y_mm: f64, b_x_mm: f64, b_y_mm: f64) -> bool {
    const EPSILON_MM: f64 = 1.0e-9;
    (a_x_mm - b_x_mm).abs() <= EPSILON_MM && (a_y_mm - b_y_mm).abs() <= EPSILON_MM
}

fn ground_zone_contacts(
    bound: &BoundBoard<'_>,
    zone: &GroundZoneEvidence<'_>,
    geometry: GroundReferenceGeometry,
    covered_x_mm: f64,
    covered_y_mm: f64,
) -> Vec<SuggestedUsbGroundZoneContact> {
    let mut contacts = Vec::new();
    for (component_id, component_pads) in &bound.project.board.layout.pads {
        for (pad_name, pad) in component_pads {
            if pad.net != zone.net_name
                || !pad_layers_include(&pad.layers, zone.zone.layer.as_str())
                || !pad.at.x_mm.is_finite()
                || !pad.at.y_mm.is_finite()
                || !pad_proves_ground_reference(covered_x_mm, covered_y_mm, pad, zone, geometry)
            {
                continue;
            }
            contacts.push(pad_contact(zone, component_id, pad_name, pad));
        }
    }
    if let Some(route) = bound.project.board.layout.routes.get(zone.net_name) {
        for (via_index, via) in route.vias.iter().enumerate() {
            if !via_layers_include(&via.layers, zone.zone.layer.as_str())
                || !via.at.x_mm.is_finite()
                || !via.at.y_mm.is_finite()
                || !contact_proves_ground_reference(
                    covered_x_mm,
                    covered_y_mm,
                    via.at.x_mm,
                    via.at.y_mm,
                    zone,
                    geometry,
                )
            {
                continue;
            }
            contacts.push(via_contact(zone, via_index, via));
        }
    }
    contacts
}

fn pad_proves_ground_reference(
    covered_x_mm: f64,
    covered_y_mm: f64,
    pad: &LayoutPad,
    zone: &GroundZoneEvidence<'_>,
    geometry: GroundReferenceGeometry,
) -> bool {
    match geometry {
        GroundReferenceGeometry::Outline => layout_pad_overlaps_polygon(pad, &zone.zone.polygon),
        GroundReferenceGeometry::FilledPolygon => zone.zone.filled_polygons.iter().any(|polygon| {
            polygon_is_usable(polygon)
                && point_inside_polygon(covered_x_mm, covered_y_mm, polygon)
                && layout_pad_overlaps_polygon(pad, polygon)
        }),
    }
}

fn contact_proves_ground_reference(
    covered_x_mm: f64,
    covered_y_mm: f64,
    contact_x_mm: f64,
    contact_y_mm: f64,
    zone: &GroundZoneEvidence<'_>,
    geometry: GroundReferenceGeometry,
) -> bool {
    match geometry {
        GroundReferenceGeometry::Outline => {
            point_inside_zone_outline(contact_x_mm, contact_y_mm, zone.zone)
        }
        GroundReferenceGeometry::FilledPolygon => point_inside_same_filled_polygon(
            covered_x_mm,
            covered_y_mm,
            contact_x_mm,
            contact_y_mm,
            zone.zone,
        ),
    }
}

fn layout_pad_overlaps_polygon(pad: &LayoutPad, polygon: &[LayoutPoint]) -> bool {
    if !polygon_is_usable(polygon) || !pad.at.x_mm.is_finite() || !pad.at.y_mm.is_finite() {
        return false;
    }
    if point_inside_polygon(pad.at.x_mm, pad.at.y_mm, polygon) {
        return true;
    }
    if !layout_pad_has_supported_extent(pad) {
        return false;
    }
    if polygon
        .iter()
        .any(|point| layout_pad_contains_point(pad, point.x_mm, point.y_mm))
    {
        return true;
    }
    let mut previous = polygon.last().expect("polygon has points");
    for current in polygon {
        if layout_segment_touches_pad(
            (previous.x_mm, previous.y_mm),
            (current.x_mm, current.y_mm),
            pad,
        ) {
            return true;
        }
        previous = current;
    }
    false
}

fn layout_pad_has_supported_extent(pad: &LayoutPad) -> bool {
    let Some(size) = &pad.size else {
        return false;
    };
    if size.x_mm <= 0.0 || size.y_mm <= 0.0 || !size.x_mm.is_finite() || !size.y_mm.is_finite() {
        return false;
    }
    pad.shape.as_deref().is_some_and(|shape| {
        matches!(
            shape.trim().to_ascii_lowercase().as_str(),
            "rect" | "circle" | "oval"
        )
    })
}

fn layout_segment_touches_pad(start: (f64, f64), end: (f64, f64), pad: &LayoutPad) -> bool {
    let Some(size) = &pad.size else {
        return false;
    };
    let Some(shape) = pad.shape.as_deref().map(str::to_ascii_lowercase) else {
        return false;
    };
    let start = layout_point_to_pad_local(start.0, start.1, pad);
    let end = layout_point_to_pad_local(end.0, end.1, pad);
    match shape.trim() {
        "rect" => {
            segment_intersects_axis_aligned_rect(start, end, size.x_mm / 2.0, size.y_mm / 2.0)
        }
        "circle" => {
            let radius_mm = size.x_mm.min(size.y_mm) / 2.0;
            point_to_segment_distance_mm(0.0, 0.0, start.0, start.1, end.0, end.1)
                .is_some_and(|distance_mm| distance_mm <= radius_mm + f64::EPSILON)
        }
        "oval" => layout_segment_touches_oval_pad(start, end, size.x_mm, size.y_mm),
        _ => false,
    }
}

fn layout_pad_contains_point(pad: &LayoutPad, x_mm: f64, y_mm: f64) -> bool {
    let Some(size) = &pad.size else {
        return false;
    };
    let Some(shape) = pad.shape.as_deref().map(str::to_ascii_lowercase) else {
        return false;
    };
    let local = layout_point_to_pad_local(x_mm, y_mm, pad);
    match shape.trim() {
        "rect" => point_inside_axis_aligned_rect(local, size.x_mm / 2.0, size.y_mm / 2.0),
        "circle" => local.0.hypot(local.1) <= size.x_mm.min(size.y_mm) / 2.0 + f64::EPSILON,
        "oval" => layout_point_inside_oval_pad(local, size.x_mm, size.y_mm),
        _ => false,
    }
}

fn layout_point_to_pad_local(x_mm: f64, y_mm: f64, pad: &LayoutPad) -> (f64, f64) {
    let dx = x_mm - pad.at.x_mm;
    let dy = y_mm - pad.at.y_mm;
    let radians = -pad.rotation_deg.unwrap_or(0.0).to_radians();
    let cos = radians.cos();
    let sin = radians.sin();
    (dx * cos - dy * sin, dx * sin + dy * cos)
}

fn layout_segment_touches_oval_pad(
    start: (f64, f64),
    end: (f64, f64),
    size_x_mm: f64,
    size_y_mm: f64,
) -> bool {
    if (size_x_mm - size_y_mm).abs() <= f64::EPSILON {
        let radius_mm = size_x_mm.min(size_y_mm) / 2.0;
        return point_to_segment_distance_mm(0.0, 0.0, start.0, start.1, end.0, end.1)
            .is_some_and(|distance_mm| distance_mm <= radius_mm + f64::EPSILON);
    }
    if size_x_mm > size_y_mm {
        let radius_mm = size_y_mm / 2.0;
        let half_straight_mm = (size_x_mm - size_y_mm) / 2.0;
        segment_to_segment_distance_mm(
            start,
            end,
            (-half_straight_mm, 0.0),
            (half_straight_mm, 0.0),
        )
        .is_some_and(|distance_mm| distance_mm <= radius_mm + f64::EPSILON)
    } else {
        let radius_mm = size_x_mm / 2.0;
        let half_straight_mm = (size_y_mm - size_x_mm) / 2.0;
        segment_to_segment_distance_mm(
            start,
            end,
            (0.0, -half_straight_mm),
            (0.0, half_straight_mm),
        )
        .is_some_and(|distance_mm| distance_mm <= radius_mm + f64::EPSILON)
    }
}

fn layout_point_inside_oval_pad(point: (f64, f64), size_x_mm: f64, size_y_mm: f64) -> bool {
    if (size_x_mm - size_y_mm).abs() <= f64::EPSILON {
        return point.0.hypot(point.1) <= size_x_mm.min(size_y_mm) / 2.0 + f64::EPSILON;
    }
    if size_x_mm > size_y_mm {
        let radius_mm = size_y_mm / 2.0;
        let half_straight_mm = (size_x_mm - size_y_mm) / 2.0;
        let nearest_x_mm = point.0.clamp(-half_straight_mm, half_straight_mm);
        (point.0 - nearest_x_mm).hypot(point.1) <= radius_mm + f64::EPSILON
    } else {
        let radius_mm = size_x_mm / 2.0;
        let half_straight_mm = (size_y_mm - size_x_mm) / 2.0;
        let nearest_y_mm = point.1.clamp(-half_straight_mm, half_straight_mm);
        point.0.hypot(point.1 - nearest_y_mm) <= radius_mm + f64::EPSILON
    }
}

fn pad_contact(
    zone: &GroundZoneEvidence<'_>,
    component_id: &str,
    pad_name: &str,
    pad: &LayoutPad,
) -> SuggestedUsbGroundZoneContact {
    SuggestedUsbGroundZoneContact {
        net: zone.net_name.to_string(),
        layer: zone.zone.layer.clone(),
        contact_kind: "pad".to_string(),
        component: Some(component_id.to_string()),
        pad: Some(pad_name.to_string()),
        via_index: None,
        x_mm: pad.at.x_mm,
        y_mm: pad.at.y_mm,
    }
}

fn via_contact(
    zone: &GroundZoneEvidence<'_>,
    via_index: usize,
    via: &RouteVia,
) -> SuggestedUsbGroundZoneContact {
    SuggestedUsbGroundZoneContact {
        net: zone.net_name.to_string(),
        layer: zone.zone.layer.clone(),
        contact_kind: "via".to_string(),
        component: None,
        pad: None,
        via_index: Some(via_index),
        x_mm: via.at.x_mm,
        y_mm: via.at.y_mm,
    }
}

fn contact_key(contact: &SuggestedUsbGroundZoneContact) -> String {
    format!(
        "{}:{}:{}:{}:{}:{}",
        contact.net,
        contact.layer,
        contact.contact_kind,
        contact.component.as_deref().unwrap_or(""),
        contact.pad.as_deref().unwrap_or(""),
        contact
            .via_index
            .map(|index| index.to_string())
            .unwrap_or_default()
    )
}

fn pad_layers_include(layers: &[String], zone_layer: &str) -> bool {
    layers.iter().any(|layer| layer_matches(layer, zone_layer))
}

fn via_layers_include(layers: &[String], zone_layer: &str) -> bool {
    layers.iter().any(|layer| layer_matches(layer, zone_layer))
}

fn layer_matches(candidate: &str, zone_layer: &str) -> bool {
    candidate == zone_layer || (candidate == "*.Cu" && zone_layer.ends_with(".Cu"))
}

fn zone_outline_is_usable(zone: &CopperZone) -> bool {
    !zone.layer.trim().is_empty() && polygon_is_usable(&zone.polygon)
}

fn point_inside_zone_outline(point_x_mm: f64, point_y_mm: f64, zone: &CopperZone) -> bool {
    point_inside_polygon(point_x_mm, point_y_mm, &zone.polygon)
}

fn point_inside_any_filled_polygon(point_x_mm: f64, point_y_mm: f64, zone: &CopperZone) -> bool {
    zone.filled_polygons
        .iter()
        .filter(|polygon| polygon_is_usable(polygon))
        .any(|polygon| point_inside_polygon(point_x_mm, point_y_mm, polygon))
}

fn point_inside_same_filled_polygon(
    first_x_mm: f64,
    first_y_mm: f64,
    second_x_mm: f64,
    second_y_mm: f64,
    zone: &CopperZone,
) -> bool {
    zone.filled_polygons
        .iter()
        .filter(|polygon| polygon_is_usable(polygon))
        .any(|polygon| {
            point_inside_polygon(first_x_mm, first_y_mm, polygon)
                && point_inside_polygon(second_x_mm, second_y_mm, polygon)
        })
}

fn point_clearance_to_any_filled_polygon_edge(
    point_x_mm: f64,
    point_y_mm: f64,
    zone: &CopperZone,
) -> Option<f64> {
    zone.filled_polygons
        .iter()
        .filter(|polygon| {
            polygon_is_usable(polygon) && point_inside_polygon(point_x_mm, point_y_mm, polygon)
        })
        .filter_map(|polygon| point_clearance_to_polygon_edge(point_x_mm, point_y_mm, polygon))
        .max_by(|left, right| left.total_cmp(right))
}

fn polygon_is_usable(polygon: &[LayoutPoint]) -> bool {
    polygon.len() >= 3
        && polygon
            .iter()
            .all(|point| point.x_mm.is_finite() && point.y_mm.is_finite())
}

fn point_inside_polygon(point_x_mm: f64, point_y_mm: f64, polygon: &[LayoutPoint]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = polygon.last().expect("polygon has points");
    for current in polygon {
        if point_on_segment(
            point_x_mm,
            point_y_mm,
            previous.x_mm,
            previous.y_mm,
            current.x_mm,
            current.y_mm,
        ) {
            return true;
        }
        let y_crosses = (current.y_mm > point_y_mm) != (previous.y_mm > point_y_mm);
        if y_crosses {
            let x_intersection = (previous.x_mm - current.x_mm) * (point_y_mm - current.y_mm)
                / (previous.y_mm - current.y_mm)
                + current.x_mm;
            if point_x_mm < x_intersection {
                inside = !inside;
            }
        }
        previous = current;
    }
    inside
}

fn point_clearance_to_polygon_edge(
    point_x_mm: f64,
    point_y_mm: f64,
    polygon: &[LayoutPoint],
) -> Option<f64> {
    if polygon.len() < 3 {
        return None;
    }
    let mut previous = polygon.last().expect("polygon has points");
    let mut clearance_mm = f64::INFINITY;
    for current in polygon {
        clearance_mm = clearance_mm.min(point_to_segment_distance_mm(
            point_x_mm,
            point_y_mm,
            previous.x_mm,
            previous.y_mm,
            current.x_mm,
            current.y_mm,
        )?);
        previous = current;
    }
    clearance_mm.is_finite().then_some(clearance_mm)
}

fn point_to_segment_distance_mm(
    point_x_mm: f64,
    point_y_mm: f64,
    start_x_mm: f64,
    start_y_mm: f64,
    end_x_mm: f64,
    end_y_mm: f64,
) -> Option<f64> {
    let dx = end_x_mm - start_x_mm;
    let dy = end_y_mm - start_y_mm;
    let length_squared = dx.mul_add(dx, dy * dy);
    if length_squared <= f64::EPSILON {
        return None;
    }
    let raw_t = ((point_x_mm - start_x_mm) * dx + (point_y_mm - start_y_mm) * dy) / length_squared;
    let t = raw_t.clamp(0.0, 1.0);
    let projected_x_mm = start_x_mm + t * dx;
    let projected_y_mm = start_y_mm + t * dy;
    Some((point_x_mm - projected_x_mm).hypot(point_y_mm - projected_y_mm))
}

fn point_on_segment(
    point_x_mm: f64,
    point_y_mm: f64,
    start_x_mm: f64,
    start_y_mm: f64,
    end_x_mm: f64,
    end_y_mm: f64,
) -> bool {
    const EPSILON_MM: f64 = 1.0e-9;
    let cross = (point_y_mm - start_y_mm) * (end_x_mm - start_x_mm)
        - (point_x_mm - start_x_mm) * (end_y_mm - start_y_mm);
    if cross.abs() > EPSILON_MM {
        return false;
    }
    let dot = (point_x_mm - start_x_mm) * (end_x_mm - start_x_mm)
        + (point_y_mm - start_y_mm) * (end_y_mm - start_y_mm);
    if dot < -EPSILON_MM {
        return false;
    }
    let length_squared = (end_x_mm - start_x_mm).powi(2) + (end_y_mm - start_y_mm).powi(2);
    dot <= length_squared + EPSILON_MM
}

fn segment_length_mm(segment: &RouteSegment) -> f64 {
    let dx = segment.end.x_mm - segment.start.x_mm;
    let dy = segment.end.y_mm - segment.start.y_mm;
    dx.hypot(dy)
}
