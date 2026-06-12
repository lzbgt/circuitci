use crate::board_ir::{ComponentPlacement, LayoutPoint, NetRoute, RouteSegment, Scenario};
use crate::library::{BoundBoard, UsbConnector};
use crate::reports::Finding;
use serde_json::json;

use super::{
    UsbConnectorSignal, placement_is_finite, required_scenario_numeric_parameter,
    valid_component_placement, valid_protection_clamps_for_net,
};
use crate::validation::USB_ROUTE_GEOMETRY_VALID;
use crate::validation::common::validation_input_missing;

const EPSILON_MM: f64 = 1.0e-9;

pub(super) fn validate_usb_route_geometry(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(max_route_length_mm) =
        required_positive_parameter(scenario, "max_data_line_route_length_mm", findings)
    else {
        return;
    };
    let Some(max_via_count) =
        required_integer_parameter(scenario, "max_data_line_via_count", findings)
    else {
        return;
    };
    let Some(max_protection_route_distance_mm) = required_positive_parameter(
        scenario,
        "max_connector_to_protection_route_distance_mm",
        findings,
    ) else {
        return;
    };
    let Some(max_component_to_route_distance_mm) =
        required_positive_parameter(scenario, "max_component_to_route_distance_mm", findings)
    else {
        return;
    };

    let Some(target) = &scenario.target else {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection target.component is required for USB_ROUTE_GEOMETRY_VALID.",
        );
        return;
    };
    let Some(component) = bound.project.board.components.get(&target.component) else {
        findings.push(usb_route_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB route target component {} is not declared.",
                target.component
            ),
            "component",
            &target.component,
        ));
        return;
    };
    let Some(model) = bound.library.get(&component.model) else {
        findings.push(usb_route_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB route target component {} model {} is not loaded.",
                target.component, component.model
            ),
            "model",
            &component.model,
        ));
        return;
    };
    let Some(connector) = &model.usb_connector else {
        findings.push(usb_route_metadata_finding(
            scenario,
            &target.component,
            format!(
                "Component {} model {} has no usb_connector metadata.",
                target.component, component.model
            ),
            "usb_connector",
            &component.model,
        ));
        return;
    };
    let Some(connector_placement) =
        valid_component_placement(bound, scenario, &target.component, findings)
    else {
        return;
    };

    for signal in [UsbConnectorSignal::Dp, UsbConnectorSignal::Dm] {
        validate_usb_route_for_signal(
            bound,
            scenario,
            UsbRouteSignalCheck {
                connector_id: &target.component,
                component,
                connector,
                connector_placement,
                signal,
                max_route_length_mm,
                max_via_count,
                max_protection_route_distance_mm,
                max_component_to_route_distance_mm,
            },
            findings,
        );
    }
}

struct UsbRouteSignalCheck<'a> {
    connector_id: &'a str,
    component: &'a crate::board_ir::ComponentSpec,
    connector: &'a UsbConnector,
    connector_placement: &'a ComponentPlacement,
    signal: UsbConnectorSignal,
    max_route_length_mm: f64,
    max_via_count: usize,
    max_protection_route_distance_mm: f64,
    max_component_to_route_distance_mm: f64,
}

fn validate_usb_route_for_signal(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    check: UsbRouteSignalCheck<'_>,
    findings: &mut Vec<Finding>,
) {
    let pin = check.signal.pin(check.connector);
    let Some(net_name) = check.component.pins.get(pin) else {
        findings.push(usb_route_metadata_finding(
            scenario,
            check.connector_id,
            format!(
                "USB connector {} {} pin {pin} is not connected, so route geometry cannot be checked.",
                check.connector_id,
                check.signal.label()
            ),
            "missing_pin",
            pin,
        ));
        return;
    };
    if !bound.project.board.nets.contains_key(net_name) {
        findings.push(usb_route_metadata_finding(
            scenario,
            check.connector_id,
            format!(
                "USB connector {} {} net {net_name} is not declared, so route geometry cannot be checked.",
                check.connector_id,
                check.signal.label()
            ),
            "missing_net",
            net_name,
        ));
        return;
    }
    let Some(route) = bound.project.board.layout.routes.get(net_name) else {
        findings.push(usb_route_metadata_finding(
            scenario,
            check.connector_id,
            format!(
                "USB connector {} {} net {net_name} has no board.layout.routes entry.",
                check.connector_id,
                check.signal.label()
            ),
            "missing_route",
            net_name,
        ));
        return;
    };
    if let Err(message) = validate_route_shape(route) {
        findings.push(usb_route_metadata_finding(
            scenario,
            check.connector_id,
            message,
            "route_geometry",
            net_name,
        ));
        return;
    }

    let route_length_mm = route_length_mm(route);
    if route_length_mm > check.max_route_length_mm {
        findings.push(usb_route_length_finding(
            scenario,
            check.connector_id,
            check.signal,
            net_name,
            route_length_mm,
            check.max_route_length_mm,
        ));
    }
    let via_count = route.vias.len();
    if via_count > check.max_via_count {
        findings.push(usb_route_via_count_finding(
            scenario,
            check.connector_id,
            check.signal,
            net_name,
            via_count,
            check.max_via_count,
        ));
    }
    validate_protection_route_distance(bound, scenario, &check, net_name, route, findings);
}

fn validate_protection_route_distance(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    check: &UsbRouteSignalCheck<'_>,
    net_name: &str,
    route: &NetRoute,
    findings: &mut Vec<Finding>,
) {
    let protections = valid_protection_clamps_for_net(bound, check.connector_id, net_name);
    if protections.is_empty() {
        findings.push(usb_route_metadata_finding(
            scenario,
            check.connector_id,
            format!(
                "USB connector {} {} net {net_name} has no valid protection clamp for route-order validation.",
                check.connector_id,
                check.signal.label()
            ),
            "required_protection_clamp",
            net_name,
        ));
        return;
    }
    let connector_point = PlacementPoint::from(check.connector_placement);
    let mut nearest = None;
    let mut missing_placements = Vec::new();
    let mut off_route = Vec::new();
    for protection in &protections {
        let Some(protection_placement) = bound
            .project
            .board
            .layout
            .placements
            .get(protection.component_id)
        else {
            missing_placements.push(protection.component_id.to_string());
            continue;
        };
        if !placement_is_finite(protection_placement) {
            missing_placements.push(protection.component_id.to_string());
            continue;
        }
        let protection_point = PlacementPoint::from(protection_placement);
        let Some(route_distance) = route_distance_between_placements(
            route,
            connector_point,
            protection_point,
            check.max_component_to_route_distance_mm,
        ) else {
            off_route.push(protection.component_id.to_string());
            continue;
        };
        if nearest
            .as_ref()
            .is_none_or(|(_, distance): &(&str, f64)| route_distance < *distance)
        {
            nearest = Some((protection.component_id, route_distance));
        }
    }
    let Some((protection_component, route_distance_mm)) = nearest else {
        findings.push(usb_route_no_protection_path_finding(
            scenario,
            check.connector_id,
            check.signal,
            net_name,
            &missing_placements,
            &off_route,
            check.max_component_to_route_distance_mm,
        ));
        return;
    };
    if route_distance_mm > check.max_protection_route_distance_mm {
        findings.push(usb_route_protection_distance_finding(
            scenario,
            check.connector_id,
            check.signal,
            net_name,
            protection_component,
            route_distance_mm,
            check.max_protection_route_distance_mm,
        ));
    }
}

#[derive(Debug, Clone, Copy)]
struct PlacementPoint {
    x_mm: f64,
    y_mm: f64,
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

fn validate_route_shape(route: &NetRoute) -> Result<(), String> {
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

fn route_length_mm(route: &NetRoute) -> f64 {
    route
        .segments
        .iter()
        .map(|segment| {
            point_distance_mm(
                PlacementPoint::from(&segment.start),
                PlacementPoint::from(&segment.end),
            )
        })
        .sum()
}

fn route_distance_between_placements(
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

fn required_positive_parameter(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<f64> {
    let value = required_scenario_numeric_parameter(scenario, name, findings)?;
    if value <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            format!("interface_protection parameters.{name} must be greater than zero."),
        );
        return None;
    }
    Some(value)
}

fn required_integer_parameter(
    scenario: &Scenario,
    name: &str,
    findings: &mut Vec<Finding>,
) -> Option<usize> {
    let value = required_scenario_numeric_parameter(scenario, name, findings)?;
    if value.fract() != 0.0 || value > usize::MAX as f64 {
        validation_input_missing(
            findings,
            scenario,
            format!("interface_protection parameters.{name} must be a non-negative integer."),
        );
        return None;
    }
    Some(value as usize)
}

fn usb_route_metadata_finding(
    scenario: &Scenario,
    component_id: &str,
    message: String,
    field: &str,
    value: &str,
) -> Finding {
    let mut finding = Finding::critical(USB_ROUTE_GEOMETRY_VALID, &scenario.name, message);
    finding.component = Some(component_id.to_string());
    finding.limit.insert(field.to_string(), json!(value));
    finding.suggested_fixes = vec![
        "Import PCB route geometry with import-kicad-pcb before declaring USB_ROUTE_GEOMETRY_VALID.".to_string(),
        "Declare route limits from the board USB/layout rule instead of inferring them from coordinates.".to_string(),
    ];
    finding
}

fn usb_route_length_finding(
    scenario: &Scenario,
    connector_id: &str,
    signal: UsbConnectorSignal,
    net: &str,
    route_length_mm: f64,
    max_route_length_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_ROUTE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} {} net {net} route length {:.3} mm exceeds limit {:.3} mm.",
            signal.label(),
            route_length_mm,
            max_route_length_mm
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!(signal.label()));
    finding
        .measured
        .insert("route_length_mm".to_string(), json!(route_length_mm));
    finding.limit.insert(
        "max_data_line_route_length_mm".to_string(),
        json!(max_route_length_mm),
    );
    finding.suggested_fixes = vec![
        "Shorten the USB data-line route or move the connector/protected device closer together."
            .to_string(),
        "Use a board-specific USB layout rule for max_data_line_route_length_mm.".to_string(),
    ];
    finding
}

fn usb_route_via_count_finding(
    scenario: &Scenario,
    connector_id: &str,
    signal: UsbConnectorSignal,
    net: &str,
    via_count: usize,
    max_via_count: usize,
) -> Finding {
    let mut finding = Finding::critical(
        USB_ROUTE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} {} net {net} has {via_count} vias, above limit {max_via_count}.",
            signal.label()
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!(signal.label()));
    finding
        .measured
        .insert("via_count".to_string(), json!(via_count));
    finding
        .limit
        .insert("max_data_line_via_count".to_string(), json!(max_via_count));
    finding.suggested_fixes = vec![
        "Reduce USB data-line layer changes or relax max_data_line_via_count only with layout/SI justification.".to_string(),
        "Keep D+ and D- via usage symmetric when the board route must change layers.".to_string(),
    ];
    finding
}

fn usb_route_no_protection_path_finding(
    scenario: &Scenario,
    connector_id: &str,
    signal: UsbConnectorSignal,
    net: &str,
    missing_placements: &[String],
    off_route_components: &[String],
    max_component_to_route_distance_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_ROUTE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} {} net {net} has no protection component with usable route-distance evidence.",
            signal.label()
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!(signal.label()));
    finding.measured.insert(
        "protection_components_without_placement".to_string(),
        json!(missing_placements),
    );
    finding.measured.insert(
        "protection_components_off_route".to_string(),
        json!(off_route_components),
    );
    finding.limit.insert(
        "max_component_to_route_distance_mm".to_string(),
        json!(max_component_to_route_distance_mm),
    );
    finding.suggested_fixes = vec![
        "Place the USB ESD component on the routed USB net near the connector and import updated PCB route geometry.".to_string(),
        "Check that component placement coordinates and route coordinates share the same PCB coordinate system.".to_string(),
    ];
    finding
}

fn usb_route_protection_distance_finding(
    scenario: &Scenario,
    connector_id: &str,
    signal: UsbConnectorSignal,
    net: &str,
    protection_component: &str,
    route_distance_mm: f64,
    max_route_distance_mm: f64,
) -> Finding {
    let mut finding = Finding::critical(
        USB_ROUTE_GEOMETRY_VALID,
        &scenario.name,
        format!(
            "USB connector {connector_id} {} net {net} reaches protection component {protection_component} after {:.3} mm of route, exceeding limit {:.3} mm.",
            signal.label(),
            route_distance_mm,
            max_route_distance_mm
        ),
    );
    finding.component = Some(connector_id.to_string());
    finding.net = Some(net.to_string());
    finding
        .measured
        .insert("connector_signal".to_string(), json!(signal.label()));
    finding.measured.insert(
        "protection_component".to_string(),
        json!(protection_component),
    );
    finding.measured.insert(
        "connector_to_protection_route_distance_mm".to_string(),
        json!(route_distance_mm),
    );
    finding.limit.insert(
        "max_connector_to_protection_route_distance_mm".to_string(),
        json!(max_route_distance_mm),
    );
    finding.suggested_fixes = vec![
        "Move the ESD component closer to the USB connector along the routed data line.".to_string(),
        "Route connector pins through the protection device before continuing to the USB transceiver.".to_string(),
    ];
    finding
}
