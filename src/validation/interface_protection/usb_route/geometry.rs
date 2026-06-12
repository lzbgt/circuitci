use crate::board_ir::{ComponentPlacement, CopperZone, LayoutPoint, NetRoute, RouteSegment};

const EPSILON_MM: f64 = 1.0e-9;

#[derive(Debug, Clone, Copy)]
pub(super) struct PlacementPoint {
    pub(super) x_mm: f64,
    pub(super) y_mm: f64,
}

impl From<&ComponentPlacement> for PlacementPoint {
    fn from(placement: &ComponentPlacement) -> Self {
        Self {
            x_mm: placement.x_mm,
            y_mm: placement.y_mm,
        }
    }
}

impl From<&LayoutPoint> for PlacementPoint {
    fn from(point: &LayoutPoint) -> Self {
        Self {
            x_mm: point.x_mm,
            y_mm: point.y_mm,
        }
    }
}

pub(super) fn validate_route_shape(route: &NetRoute) -> Result<(), String> {
    if route.segments.is_empty() {
        return Err("USB route geometry must include at least one segment.".to_string());
    }
    for segment in &route.segments {
        let start = PlacementPoint::from(&segment.start);
        let end = PlacementPoint::from(&segment.end);
        if !point_is_finite(start) || !point_is_finite(end) {
            return Err("USB route segment endpoints must be finite.".to_string());
        }
        if segment.width_mm <= 0.0 || !segment.width_mm.is_finite() {
            return Err("USB route segment width_mm must be finite and positive.".to_string());
        }
        if segment.layer.trim().is_empty() {
            return Err("USB route segment layer must be non-empty.".to_string());
        }
        if point_distance_mm(start, end) <= EPSILON_MM {
            return Err("USB route segment length must be greater than zero.".to_string());
        }
    }
    for via in &route.vias {
        let at = PlacementPoint::from(&via.at);
        if !point_is_finite(at) {
            return Err("USB route via coordinate must be finite.".to_string());
        }
        if via.size_mm <= 0.0 || !via.size_mm.is_finite() {
            return Err("USB route via size_mm must be finite and positive.".to_string());
        }
        if via.drill_mm <= 0.0 || !via.drill_mm.is_finite() {
            return Err("USB route via drill_mm must be finite and positive.".to_string());
        }
    }
    Ok(())
}

pub(super) fn route_length_mm(route: &NetRoute) -> f64 {
    route.segments.iter().map(segment_length_mm).sum()
}

pub(super) fn segment_length_mm(segment: &RouteSegment) -> f64 {
    point_distance_mm(
        PlacementPoint::from(&segment.start),
        PlacementPoint::from(&segment.end),
    )
}

pub(super) fn segment_midpoint(segment: &RouteSegment) -> PlacementPoint {
    PlacementPoint {
        x_mm: (segment.start.x_mm + segment.end.x_mm) / 2.0,
        y_mm: (segment.start.y_mm + segment.end.y_mm) / 2.0,
    }
}

pub(super) fn validate_zone_outline(zone: &CopperZone) -> Result<(), String> {
    if zone.layer.trim().is_empty() {
        return Err("USB return-path zone layer must be non-empty.".to_string());
    }
    validate_polygon_points(
        &zone.polygon,
        "USB return-path zone polygon must include at least three points.",
        "USB return-path zone polygon points must be finite.",
    )?;
    for filled_polygon in &zone.filled_polygons {
        validate_polygon_points(
            filled_polygon,
            "USB return-path filled zone polygon must include at least three points.",
            "USB return-path filled zone polygon points must be finite.",
        )?;
    }
    Ok(())
}

pub(super) fn point_inside_zone_outline(point: PlacementPoint, zone: &CopperZone) -> bool {
    point_inside_polygon(point, &zone.polygon)
}

pub(super) fn point_inside_filled_zone(point: PlacementPoint, zone: &CopperZone) -> bool {
    zone.filled_polygons
        .iter()
        .any(|polygon| point_inside_polygon(point, polygon))
}

pub(super) fn points_inside_same_filled_zone_polygon(
    first: PlacementPoint,
    second: PlacementPoint,
    zone: &CopperZone,
) -> bool {
    zone.filled_polygons.iter().any(|polygon| {
        point_inside_polygon(first, polygon) && point_inside_polygon(second, polygon)
    })
}

pub(super) fn point_clearance_to_filled_zone_edge(
    point: PlacementPoint,
    zone: &CopperZone,
) -> Option<f64> {
    zone.filled_polygons
        .iter()
        .filter(|polygon| point_inside_polygon(point, polygon))
        .filter_map(|polygon| point_clearance_to_polygon_edge(point, polygon))
        .max_by(|left, right| left.total_cmp(right))
}

fn validate_polygon_points(
    polygon: &[LayoutPoint],
    short_message: &str,
    invalid_message: &str,
) -> Result<(), String> {
    if polygon.len() < 3 {
        return Err(short_message.to_string());
    }
    if polygon
        .iter()
        .any(|point| !point.x_mm.is_finite() || !point.y_mm.is_finite())
    {
        return Err(invalid_message.to_string());
    }
    Ok(())
}

fn point_inside_polygon(point: PlacementPoint, polygon: &[LayoutPoint]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = PlacementPoint::from(polygon.last().expect("polygon has points"));
    for current_point in polygon {
        let current = PlacementPoint::from(current_point);
        if point_on_segment(point, previous, current) {
            return true;
        }
        let y_crosses = (current.y_mm > point.y_mm) != (previous.y_mm > point.y_mm);
        if y_crosses {
            let x_intersection = (previous.x_mm - current.x_mm) * (point.y_mm - current.y_mm)
                / (previous.y_mm - current.y_mm)
                + current.x_mm;
            if point.x_mm < x_intersection {
                inside = !inside;
            }
        }
        previous = current;
    }
    inside
}

fn point_clearance_to_polygon_edge(point: PlacementPoint, polygon: &[LayoutPoint]) -> Option<f64> {
    if polygon.len() < 3 {
        return None;
    }
    let mut previous = PlacementPoint::from(polygon.last().expect("polygon has points"));
    let mut clearance_mm = f64::INFINITY;
    for current_point in polygon {
        let current = PlacementPoint::from(current_point);
        clearance_mm = clearance_mm.min(point_to_segment_distance_mm(point, previous, current)?);
        previous = current;
    }
    clearance_mm.is_finite().then_some(clearance_mm)
}

fn point_to_segment_distance_mm(
    point: PlacementPoint,
    start: PlacementPoint,
    end: PlacementPoint,
) -> Option<f64> {
    let (_, projected) = project_point_to_segment(point, start, end)?;
    Some(point_distance_mm(point, projected))
}

pub(super) fn route_distance_between_placements(
    route: &NetRoute,
    from: PlacementPoint,
    to: PlacementPoint,
    max_point_to_route_distance_mm: f64,
) -> Option<f64> {
    let from_projection = nearest_projection(route, from)?;
    let to_projection = nearest_projection(route, to)?;
    if from_projection.distance_to_point_mm > max_point_to_route_distance_mm
        || to_projection.distance_to_point_mm > max_point_to_route_distance_mm
    {
        return None;
    }
    shortest_route_distance_mm(route, &from_projection, &to_projection)
}

pub(super) fn worst_route_width_delta(
    route: &NetRoute,
    expected_width_mm: f64,
) -> Option<(usize, f64, f64)> {
    route
        .segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            let delta = (segment.width_mm - expected_width_mm).abs();
            (index, segment.width_mm, delta)
        })
        .max_by(|left, right| left.2.total_cmp(&right.2))
}

#[derive(Debug, Clone, Copy)]
pub(super) struct UsbPairGapEvidence {
    pub(super) dp_segment_index: usize,
    pub(super) dm_segment_index: usize,
    pub(super) centerline_distance_mm: f64,
    pub(super) measured_gap_mm: f64,
    pub(super) expected_gap_mm: f64,
    pub(super) gap_delta_mm: f64,
}

pub(super) fn worst_pair_gap_delta(
    dp_route: &NetRoute,
    dm_route: &NetRoute,
    expected_gap_mm: f64,
) -> Option<UsbPairGapEvidence> {
    let mut worst = None;
    for (dp_index, dp_segment) in dp_route.segments.iter().enumerate() {
        for (dm_index, dm_segment) in dm_route.segments.iter().enumerate() {
            let Some((centerline_distance_mm, measured_gap_mm)) =
                parallel_overlap_gap_mm(dp_segment, dm_segment)
            else {
                continue;
            };
            let gap_delta_mm = (measured_gap_mm - expected_gap_mm).abs();
            let evidence = UsbPairGapEvidence {
                dp_segment_index: dp_index,
                dm_segment_index: dm_index,
                centerline_distance_mm,
                measured_gap_mm,
                expected_gap_mm,
                gap_delta_mm,
            };
            if worst
                .as_ref()
                .is_none_or(|current: &UsbPairGapEvidence| gap_delta_mm > current.gap_delta_mm)
            {
                worst = Some(evidence);
            }
        }
    }
    worst
}

#[derive(Debug, Clone, Copy)]
struct Projection {
    segment_index: usize,
    t: f64,
    point: PlacementPoint,
    distance_to_point_mm: f64,
}

fn nearest_projection(route: &NetRoute, point: PlacementPoint) -> Option<Projection> {
    route
        .segments
        .iter()
        .enumerate()
        .filter_map(|(segment_index, segment)| {
            let start = PlacementPoint::from(&segment.start);
            let end = PlacementPoint::from(&segment.end);
            project_point_to_segment(point, start, end).map(|(t, projected)| Projection {
                segment_index,
                t,
                point: projected,
                distance_to_point_mm: point_distance_mm(point, projected),
            })
        })
        .min_by(|left, right| {
            left.distance_to_point_mm
                .total_cmp(&right.distance_to_point_mm)
        })
}

fn project_point_to_segment(
    point: PlacementPoint,
    start: PlacementPoint,
    end: PlacementPoint,
) -> Option<(f64, PlacementPoint)> {
    let dx = end.x_mm - start.x_mm;
    let dy = end.y_mm - start.y_mm;
    let length_squared = dx.mul_add(dx, dy * dy);
    if length_squared <= EPSILON_MM {
        return None;
    }
    let raw_t = ((point.x_mm - start.x_mm) * dx + (point.y_mm - start.y_mm) * dy) / length_squared;
    let t = raw_t.clamp(0.0, 1.0);
    Some((
        t,
        PlacementPoint {
            x_mm: start.x_mm + t * dx,
            y_mm: start.y_mm + t * dy,
        },
    ))
}

fn point_on_segment(point: PlacementPoint, start: PlacementPoint, end: PlacementPoint) -> bool {
    let cross = (point.y_mm - start.y_mm) * (end.x_mm - start.x_mm)
        - (point.x_mm - start.x_mm) * (end.y_mm - start.y_mm);
    if cross.abs() > EPSILON_MM {
        return false;
    }
    let dot = (point.x_mm - start.x_mm) * (end.x_mm - start.x_mm)
        + (point.y_mm - start.y_mm) * (end.y_mm - start.y_mm);
    if dot < -EPSILON_MM {
        return false;
    }
    let length_squared = (end.x_mm - start.x_mm).powi(2) + (end.y_mm - start.y_mm).powi(2);
    dot <= length_squared + EPSILON_MM
}

fn shortest_route_distance_mm(
    route: &NetRoute,
    from_projection: &Projection,
    to_projection: &Projection,
) -> Option<f64> {
    let mut graph = RouteGraph::default();
    for (segment_index, segment) in route.segments.iter().enumerate() {
        add_segment_to_graph(
            &mut graph,
            segment_index,
            segment,
            from_projection,
            to_projection,
        );
    }
    let from_node = graph.find_node(from_projection.point)?;
    let to_node = graph.find_node(to_projection.point)?;
    graph.shortest_distance(from_node, to_node)
}

#[derive(Default)]
struct RouteGraph {
    nodes: Vec<PlacementPoint>,
    edges: Vec<Vec<(usize, f64)>>,
}

impl RouteGraph {
    fn node_for(&mut self, point: PlacementPoint) -> usize {
        if let Some(index) = self.find_node(point) {
            return index;
        }
        let index = self.nodes.len();
        self.nodes.push(point);
        self.edges.push(Vec::new());
        index
    }

    fn find_node(&self, point: PlacementPoint) -> Option<usize> {
        self.nodes
            .iter()
            .position(|candidate| points_equal(*candidate, point))
    }

    fn connect(&mut self, a: usize, b: usize, distance_mm: f64) {
        if a == b || distance_mm <= EPSILON_MM {
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
    from_projection: &Projection,
    to_projection: &Projection,
) {
    let start = PlacementPoint::from(&segment.start);
    let end = PlacementPoint::from(&segment.end);
    let mut points = vec![(0.0, start), (1.0, end)];
    if from_projection.segment_index == segment_index {
        points.push((from_projection.t, from_projection.point));
    }
    if to_projection.segment_index == segment_index {
        points.push((to_projection.t, to_projection.point));
    }
    points.sort_by(|left, right| left.0.total_cmp(&right.0));
    points.dedup_by(|left, right| points_equal(left.1, right.1));
    for window in points.windows(2) {
        let first = graph.node_for(window[0].1);
        let second = graph.node_for(window[1].1);
        graph.connect(first, second, point_distance_mm(window[0].1, window[1].1));
    }
}

fn parallel_overlap_gap_mm(
    dp_segment: &RouteSegment,
    dm_segment: &RouteSegment,
) -> Option<(f64, f64)> {
    let dp_start = PlacementPoint::from(&dp_segment.start);
    let dp_end = PlacementPoint::from(&dp_segment.end);
    let dm_start = PlacementPoint::from(&dm_segment.start);
    let dm_end = PlacementPoint::from(&dm_segment.end);
    let dp_dx = dp_end.x_mm - dp_start.x_mm;
    let dp_dy = dp_end.y_mm - dp_start.y_mm;
    let dm_dx = dm_end.x_mm - dm_start.x_mm;
    let dm_dy = dm_end.y_mm - dm_start.y_mm;
    let dp_len = dp_dx.hypot(dp_dy);
    let dm_len = dm_dx.hypot(dm_dy);
    if dp_len <= EPSILON_MM || dm_len <= EPSILON_MM {
        return None;
    }
    let dp_unit_x = dp_dx / dp_len;
    let dp_unit_y = dp_dy / dp_len;
    let dm_unit_x = dm_dx / dm_len;
    let dm_unit_y = dm_dy / dm_len;
    let cross = (dp_unit_x * dm_unit_y - dp_unit_y * dm_unit_x).abs();
    if cross > 1.0e-6 {
        return None;
    }
    let projection_a =
        (dm_start.x_mm - dp_start.x_mm) * dp_unit_x + (dm_start.y_mm - dp_start.y_mm) * dp_unit_y;
    let projection_b =
        (dm_end.x_mm - dp_start.x_mm) * dp_unit_x + (dm_end.y_mm - dp_start.y_mm) * dp_unit_y;
    let overlap_start = projection_a.min(projection_b).max(0.0);
    let overlap_end = projection_a.max(projection_b).min(dp_len);
    if overlap_end - overlap_start <= EPSILON_MM {
        return None;
    }
    let centerline_distance_mm = ((dm_start.x_mm - dp_start.x_mm) * dp_unit_y
        - (dm_start.y_mm - dp_start.y_mm) * dp_unit_x)
        .abs();
    let measured_gap_mm =
        centerline_distance_mm - (dp_segment.width_mm + dm_segment.width_mm) / 2.0;
    Some((centerline_distance_mm, measured_gap_mm))
}

fn point_is_finite(point: PlacementPoint) -> bool {
    point.x_mm.is_finite() && point.y_mm.is_finite()
}

fn points_equal(a: PlacementPoint, b: PlacementPoint) -> bool {
    (a.x_mm - b.x_mm).abs() <= EPSILON_MM && (a.y_mm - b.y_mm).abs() <= EPSILON_MM
}

fn point_distance_mm(a: PlacementPoint, b: PlacementPoint) -> f64 {
    let dx = a.x_mm - b.x_mm;
    let dy = a.y_mm - b.y_mm;
    dx.hypot(dy)
}
