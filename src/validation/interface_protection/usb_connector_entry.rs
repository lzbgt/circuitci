use crate::board_ir::{
    ComponentPlacement, LayoutFootprintPolygon, LayoutFootprintSegment, LayoutPoint, Scenario,
};
use crate::library::{BoundBoard, UsbConnector};
use crate::reports::Finding;
use serde_json::json;

use super::super::USB_CONNECTOR_ENTRY_CLEARANCE_VALID;
use super::super::common::validation_input_missing;
use super::{
    required_scenario_numeric_parameter, scenario_numeric_parameter,
    usb_connector::{
        footprint_arc_points, footprint_circle_points, mechanical_footprint_kind,
        placement_is_finite, point_is_finite, rectangle_corners, segment_length_mm,
        segment_to_segment_distance_mm,
    },
};

pub(super) fn validate_usb_connector_entry_clearance(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let Some(depth_mm) = required_scenario_numeric_parameter(
        scenario,
        "min_cable_entry_clearance_depth_mm",
        findings,
    ) else {
        return;
    };
    if depth_mm <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection parameters.min_cable_entry_clearance_depth_mm must be greater than zero.",
        );
        return;
    }
    let Some(width_mm) =
        required_scenario_numeric_parameter(scenario, "cable_entry_clearance_width_mm", findings)
    else {
        return;
    };
    if width_mm <= 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection parameters.cable_entry_clearance_width_mm must be greater than zero.",
        );
        return;
    }
    let Some(target) = &scenario.target else {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection target.component is required for USB_CONNECTOR_ENTRY_CLEARANCE_VALID.",
        );
        return;
    };
    let Some(component) = bound.project.board.components.get(&target.component) else {
        findings.push(entry_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector entry-clearance target component {} is not declared.",
                target.component
            ),
            "component",
            &target.component,
        ));
        return;
    };
    let Some(model) = bound.library.get(&component.model) else {
        findings.push(entry_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector entry-clearance target component {} model {} is not loaded.",
                target.component, component.model
            ),
            "model",
            &component.model,
        ));
        return;
    };
    let Some(connector) = model.usb_connector.as_ref() else {
        findings.push(entry_metadata_finding(
            scenario,
            &target.component,
            format!(
                "Component {} model {} has no usb_connector metadata.",
                target.component, component.model
            ),
            "usb_connector",
            "missing",
        ));
        return;
    };
    let Some(placement) = bound.project.board.layout.placements.get(&target.component) else {
        findings.push(entry_metadata_finding(
            scenario,
            &target.component,
            format!(
                "Component {} has no board.layout.placements entry.",
                target.component
            ),
            "placement",
            &target.component,
        ));
        return;
    };
    if !placement_is_finite(placement) {
        findings.push(entry_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector {} placement must have finite x_mm and y_mm.",
                target.component
            ),
            "placement",
            &target.component,
        ));
        return;
    }
    let entry_direction = match scenario_numeric_parameter(
        scenario,
        "entry_direction_deg",
        findings,
    ) {
        Some(direction) => EntryDirectionEvidence {
            deg: direction,
            source: "scenario_parameter",
            offset_deg: None,
        },
        None => {
            let Some(entry_direction) = model_entry_direction(placement, connector) else {
                findings.push(entry_metadata_finding(
                    scenario,
                    &target.component,
                    format!(
                        "USB connector {} entry clearance requires placement rotation_deg evidence or parameters.entry_direction_deg.",
                        target.component
                    ),
                    "rotation_deg",
                    "missing",
                ));
                return;
            };
            entry_direction
        }
    };
    let entry_direction_deg = entry_direction.deg;
    if !entry_direction_deg.is_finite() {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection parameters.entry_direction_deg must be finite when declared.",
        );
        return;
    }
    let Some(connector_front) =
        connector_front_projection(bound, &target.component, entry_direction_deg)
    else {
        findings.push(entry_metadata_finding(
            scenario,
            &target.component,
            "USB connector entry clearance requires imported fabrication/courtyard footprint graphics for the target connector.".to_string(),
            "connector_footprint",
            "missing",
        ));
        return;
    };
    let corridor = EntryCorridor::new(
        entry_direction_deg,
        connector_front,
        placement_lateral_projection(placement, entry_direction_deg),
        depth_mm,
        width_mm,
    );
    if let Some(obstruction) = nearest_entry_obstruction(bound, &target.component, &corridor) {
        findings.push(entry_clearance_finding(EntryClearanceEvidence {
            scenario,
            connector_id: &target.component,
            obstruction,
            entry_direction,
            depth_mm,
            width_mm,
        }));
    }
}

fn model_entry_direction(
    placement: &ComponentPlacement,
    connector: &UsbConnector,
) -> Option<EntryDirectionEvidence> {
    let rotation_deg = placement.rotation_deg?;
    let offset_deg = connector.entry_direction_offset_deg;
    let entry_direction_deg = (rotation_deg + offset_deg.unwrap_or(0.0)).rem_euclid(360.0);
    entry_direction_deg
        .is_finite()
        .then_some(EntryDirectionEvidence {
            deg: entry_direction_deg,
            source: if offset_deg.is_some() {
                "component_model_offset"
            } else {
                "placement_rotation"
            },
            offset_deg,
        })
}

#[derive(Clone, Copy)]
struct EntryDirectionEvidence {
    deg: f64,
    source: &'static str,
    offset_deg: Option<f64>,
}

struct EntryClearanceEvidence<'a> {
    scenario: &'a Scenario,
    connector_id: &'a str,
    obstruction: EntryObstruction<'a>,
    entry_direction: EntryDirectionEvidence,
    depth_mm: f64,
    width_mm: f64,
}

#[derive(Clone)]
struct EntryObstruction<'a> {
    component_id: &'a str,
    depth_mm: f64,
    lateral_offset_mm: f64,
    reference: EntryObstacleReference<'a>,
}

#[derive(Clone, Copy)]
enum EntryObstacleReference<'a> {
    PlacementCenter,
    FootprintSegment { layer: &'a str, kind: &'a str },
    FootprintRectangle { layer: &'a str, kind: &'a str },
    FootprintPolygon { layer: &'a str, kind: &'a str },
    FootprintCircle { layer: &'a str, kind: &'a str },
    FootprintArc { layer: &'a str, kind: &'a str },
}

impl EntryObstacleReference<'_> {
    fn label(&self) -> &'static str {
        match self {
            EntryObstacleReference::PlacementCenter => "placement_center",
            EntryObstacleReference::FootprintSegment { .. } => "footprint_segment",
            EntryObstacleReference::FootprintRectangle { .. } => "footprint_rectangle",
            EntryObstacleReference::FootprintPolygon { .. } => "footprint_polygon",
            EntryObstacleReference::FootprintCircle { .. } => "footprint_circle",
            EntryObstacleReference::FootprintArc { .. } => "footprint_arc",
        }
    }

    fn footprint_layer(&self) -> Option<&str> {
        match self {
            EntryObstacleReference::PlacementCenter => None,
            EntryObstacleReference::FootprintSegment { layer, .. }
            | EntryObstacleReference::FootprintRectangle { layer, .. }
            | EntryObstacleReference::FootprintPolygon { layer, .. }
            | EntryObstacleReference::FootprintCircle { layer, .. }
            | EntryObstacleReference::FootprintArc { layer, .. } => Some(layer),
        }
    }

    fn footprint_kind(&self) -> Option<&str> {
        match self {
            EntryObstacleReference::PlacementCenter => None,
            EntryObstacleReference::FootprintSegment { kind, .. }
            | EntryObstacleReference::FootprintRectangle { kind, .. }
            | EntryObstacleReference::FootprintPolygon { kind, .. }
            | EntryObstacleReference::FootprintCircle { kind, .. }
            | EntryObstacleReference::FootprintArc { kind, .. } => Some(kind),
        }
    }
}

#[derive(Clone)]
enum ObstaclePrimitive<'a> {
    Point {
        point: LayoutPoint,
        reference: EntryObstacleReference<'a>,
    },
    Segment {
        start: LayoutPoint,
        end: LayoutPoint,
        reference: EntryObstacleReference<'a>,
    },
}

struct EntryCorridor {
    direction_x: f64,
    direction_y: f64,
    normal_x: f64,
    normal_y: f64,
    front_projection: f64,
    center_lateral_projection: f64,
    depth_mm: f64,
    half_width_mm: f64,
    edges: [LayoutPoint; 4],
}

impl EntryCorridor {
    fn new(
        entry_direction_deg: f64,
        front_projection: f64,
        center_lateral_projection: f64,
        depth_mm: f64,
        width_mm: f64,
    ) -> Self {
        let radians = entry_direction_deg.to_radians();
        let direction_x = radians.cos();
        let direction_y = radians.sin();
        let normal_x = -direction_y;
        let normal_y = direction_x;
        let half_width_mm = width_mm / 2.0;
        let p0 = point_from_projection(
            direction_x,
            direction_y,
            normal_x,
            normal_y,
            front_projection,
            center_lateral_projection - half_width_mm,
        );
        let p1 = point_from_projection(
            direction_x,
            direction_y,
            normal_x,
            normal_y,
            front_projection + depth_mm,
            center_lateral_projection - half_width_mm,
        );
        let p2 = point_from_projection(
            direction_x,
            direction_y,
            normal_x,
            normal_y,
            front_projection + depth_mm,
            center_lateral_projection + half_width_mm,
        );
        let p3 = point_from_projection(
            direction_x,
            direction_y,
            normal_x,
            normal_y,
            front_projection,
            center_lateral_projection + half_width_mm,
        );
        Self {
            direction_x,
            direction_y,
            normal_x,
            normal_y,
            front_projection,
            center_lateral_projection,
            depth_mm,
            half_width_mm,
            edges: [p0, p1, p2, p3],
        }
    }

    fn point_projection(&self, point: &LayoutPoint) -> (f64, f64) {
        (
            point.x_mm * self.direction_x + point.y_mm * self.direction_y,
            point.x_mm * self.normal_x + point.y_mm * self.normal_y,
        )
    }

    fn point_inside(&self, point: &LayoutPoint) -> bool {
        let (forward, lateral) = self.point_projection(point);
        forward >= self.front_projection - f64::EPSILON
            && forward <= self.front_projection + self.depth_mm + f64::EPSILON
            && (lateral - self.center_lateral_projection).abs() <= self.half_width_mm + f64::EPSILON
    }

    fn point_depth_and_lateral(&self, point: &LayoutPoint) -> (f64, f64) {
        let (forward, lateral) = self.point_projection(point);
        (
            (forward - self.front_projection).max(0.0),
            lateral - self.center_lateral_projection,
        )
    }

    fn segment_intersects(&self, start: &LayoutPoint, end: &LayoutPoint) -> bool {
        if self.point_inside(start) || self.point_inside(end) {
            return true;
        }
        for index in 0..self.edges.len() {
            let next_index = (index + 1) % self.edges.len();
            if segment_to_segment_distance_mm(
                start,
                end,
                &self.edges[index],
                &self.edges[next_index],
            ) <= f64::EPSILON
            {
                return true;
            }
        }
        false
    }

    fn primitive_obstruction<'a>(
        &self,
        primitive: &ObstaclePrimitive<'a>,
    ) -> Option<(f64, f64, EntryObstacleReference<'a>)> {
        match primitive {
            ObstaclePrimitive::Point { point, reference } => self.point_inside(point).then(|| {
                let (depth, lateral) = self.point_depth_and_lateral(point);
                (depth, lateral, *reference)
            }),
            ObstaclePrimitive::Segment {
                start,
                end,
                reference,
            } => {
                if !self.segment_intersects(start, end) {
                    return None;
                }
                let candidates = [start, end]
                    .into_iter()
                    .filter(|point| self.point_inside(point))
                    .map(|point| self.point_depth_and_lateral(point))
                    .collect::<Vec<_>>();
                let (depth, lateral) = candidates
                    .into_iter()
                    .min_by(|left, right| left.0.total_cmp(&right.0))
                    .unwrap_or((0.0, 0.0));
                Some((depth, lateral, *reference))
            }
        }
    }
}

fn connector_front_projection(
    bound: &BoundBoard<'_>,
    connector_id: &str,
    entry_direction_deg: f64,
) -> Option<f64> {
    mechanical_footprint_points(bound, connector_id, false, entry_direction_deg)
        .into_iter()
        .map(|point| direction_projection(&point, entry_direction_deg))
        .max_by(|left, right| left.total_cmp(right))
}

fn placement_lateral_projection(placement: &ComponentPlacement, entry_direction_deg: f64) -> f64 {
    let radians = entry_direction_deg.to_radians();
    let normal_x = -radians.sin();
    let normal_y = radians.cos();
    placement.x_mm * normal_x + placement.y_mm * normal_y
}

fn direction_projection(point: &LayoutPoint, entry_direction_deg: f64) -> f64 {
    let radians = entry_direction_deg.to_radians();
    point.x_mm * radians.cos() + point.y_mm * radians.sin()
}

fn point_from_projection(
    direction_x: f64,
    direction_y: f64,
    normal_x: f64,
    normal_y: f64,
    forward: f64,
    lateral: f64,
) -> LayoutPoint {
    LayoutPoint {
        x_mm: direction_x * forward + normal_x * lateral,
        y_mm: direction_y * forward + normal_y * lateral,
    }
}

fn nearest_entry_obstruction<'a>(
    bound: &'a BoundBoard<'_>,
    connector_id: &'a str,
    corridor: &EntryCorridor,
) -> Option<EntryObstruction<'a>> {
    let mut nearest = None;
    for component_id in bound.project.board.components.keys() {
        if component_id == connector_id {
            continue;
        }
        for primitive in obstacle_primitives(bound, component_id, true) {
            let Some((depth_mm, lateral_offset_mm, reference)) =
                corridor.primitive_obstruction(&primitive)
            else {
                continue;
            };
            let obstruction = EntryObstruction {
                component_id,
                depth_mm,
                lateral_offset_mm,
                reference,
            };
            if nearest
                .as_ref()
                .is_none_or(|current: &EntryObstruction<'_>| depth_mm < current.depth_mm)
            {
                nearest = Some(obstruction);
            }
        }
    }
    nearest
}

fn mechanical_footprint_points(
    bound: &BoundBoard<'_>,
    component_id: &str,
    include_placement_fallback: bool,
    entry_direction_deg: f64,
) -> Vec<LayoutPoint> {
    let mut points = Vec::new();
    if let Some(footprint) = bound.project.board.layout.footprints.get(component_id) {
        for segment in &footprint.segments {
            if valid_segment(segment) && mechanical_footprint_kind(&segment.kind) {
                points.push(segment.start.clone());
                points.push(segment.end.clone());
            }
        }
        for rectangle in &footprint.rectangles {
            if mechanical_footprint_kind(&rectangle.kind)
                && let Some(corners) = rectangle_corners(rectangle)
            {
                points.extend(corners);
            }
        }
        for polygon in &footprint.polygons {
            if valid_polygon(polygon) && mechanical_footprint_kind(&polygon.kind) {
                points.extend(polygon.points.clone());
            }
        }
        for circle in &footprint.circles {
            if mechanical_footprint_kind(&circle.kind)
                && let Some(circle_points) = footprint_circle_points(circle)
            {
                points.extend(circle_points);
            }
        }
        for arc in &footprint.arcs {
            if mechanical_footprint_kind(&arc.kind)
                && let Some(arc_points) = footprint_arc_points(arc)
            {
                points.extend(arc_points);
            }
        }
    }
    if points.is_empty()
        && include_placement_fallback
        && let Some(placement) = bound.project.board.layout.placements.get(component_id)
        && placement_is_finite(placement)
    {
        points.push(LayoutPoint {
            x_mm: placement.x_mm,
            y_mm: placement.y_mm,
        });
    }
    points.sort_by(|left, right| {
        direction_projection(right, entry_direction_deg)
            .total_cmp(&direction_projection(left, entry_direction_deg))
    });
    points
}

fn obstacle_primitives<'a>(
    bound: &'a BoundBoard<'_>,
    component_id: &'a str,
    include_placement_fallback: bool,
) -> Vec<ObstaclePrimitive<'a>> {
    let mut primitives = Vec::new();
    if let Some(footprint) = bound.project.board.layout.footprints.get(component_id) {
        for segment in &footprint.segments {
            if valid_segment(segment) && mechanical_footprint_kind(&segment.kind) {
                primitives.push(ObstaclePrimitive::Segment {
                    start: segment.start.clone(),
                    end: segment.end.clone(),
                    reference: EntryObstacleReference::FootprintSegment {
                        layer: &segment.layer,
                        kind: &segment.kind,
                    },
                });
            }
        }
        for rectangle in &footprint.rectangles {
            if mechanical_footprint_kind(&rectangle.kind)
                && let Some(corners) = rectangle_corners(rectangle)
            {
                push_closed_polyline(
                    &mut primitives,
                    &corners,
                    EntryObstacleReference::FootprintRectangle {
                        layer: &rectangle.layer,
                        kind: &rectangle.kind,
                    },
                );
            }
        }
        for polygon in &footprint.polygons {
            if valid_polygon(polygon) && mechanical_footprint_kind(&polygon.kind) {
                push_closed_polyline(
                    &mut primitives,
                    &polygon.points,
                    EntryObstacleReference::FootprintPolygon {
                        layer: &polygon.layer,
                        kind: &polygon.kind,
                    },
                );
            }
        }
        for circle in &footprint.circles {
            if mechanical_footprint_kind(&circle.kind)
                && let Some(points) = footprint_circle_points(circle)
            {
                push_closed_polyline(
                    &mut primitives,
                    &points,
                    EntryObstacleReference::FootprintCircle {
                        layer: &circle.layer,
                        kind: &circle.kind,
                    },
                );
            }
        }
        for arc in &footprint.arcs {
            if mechanical_footprint_kind(&arc.kind)
                && let Some(points) = footprint_arc_points(arc)
            {
                push_open_polyline(
                    &mut primitives,
                    &points,
                    EntryObstacleReference::FootprintArc {
                        layer: &arc.layer,
                        kind: &arc.kind,
                    },
                );
            }
        }
    }
    if primitives.is_empty()
        && include_placement_fallback
        && let Some(placement) = bound.project.board.layout.placements.get(component_id)
        && placement_is_finite(placement)
    {
        primitives.push(ObstaclePrimitive::Point {
            point: LayoutPoint {
                x_mm: placement.x_mm,
                y_mm: placement.y_mm,
            },
            reference: EntryObstacleReference::PlacementCenter,
        });
    }
    primitives
}

fn valid_segment(segment: &LayoutFootprintSegment) -> bool {
    point_is_finite(&segment.start)
        && point_is_finite(&segment.end)
        && segment_length_mm(&segment.start, &segment.end) > f64::EPSILON
}

fn valid_polygon(polygon: &LayoutFootprintPolygon) -> bool {
    polygon.points.len() >= 3 && polygon.points.iter().all(point_is_finite)
}

fn push_closed_polyline<'a>(
    primitives: &mut Vec<ObstaclePrimitive<'a>>,
    points: &[LayoutPoint],
    reference: EntryObstacleReference<'a>,
) {
    for index in 0..points.len() {
        let next_index = (index + 1) % points.len();
        primitives.push(ObstaclePrimitive::Segment {
            start: points[index].clone(),
            end: points[next_index].clone(),
            reference,
        });
    }
}

fn push_open_polyline<'a>(
    primitives: &mut Vec<ObstaclePrimitive<'a>>,
    points: &[LayoutPoint],
    reference: EntryObstacleReference<'a>,
) {
    for window in points.windows(2) {
        primitives.push(ObstaclePrimitive::Segment {
            start: window[0].clone(),
            end: window[1].clone(),
            reference,
        });
    }
}

fn entry_metadata_finding(
    scenario: &Scenario,
    component_id: &str,
    message: String,
    field: &str,
    value: &str,
) -> Finding {
    let mut finding =
        Finding::critical(USB_CONNECTOR_ENTRY_CLEARANCE_VALID, &scenario.name, message);
    finding.component = Some(component_id.to_string());
    finding.limit.insert(field.to_string(), json!(value));
    finding.suggested_fixes = vec![
        "Import connector placement and fabrication/courtyard footprint evidence with import-kicad-pcb before declaring USB_CONNECTOR_ENTRY_CLEARANCE_VALID.".to_string(),
        "Set cable-entry depth and width from connector, cable plug, panel, and enclosure mechanical drawings.".to_string(),
    ];
    finding
}

fn entry_clearance_finding(evidence: EntryClearanceEvidence<'_>) -> Finding {
    let mut finding = Finding::critical(
        USB_CONNECTOR_ENTRY_CLEARANCE_VALID,
        &evidence.scenario.name,
        format!(
            "USB connector {} cable-entry corridor is obstructed by component {} at {:.3} mm into the required {:.3} mm depth.",
            evidence.connector_id,
            evidence.obstruction.component_id,
            evidence.obstruction.depth_mm,
            evidence.depth_mm
        ),
    );
    finding.component = Some(evidence.connector_id.to_string());
    finding.measured.insert(
        "obstructing_component".to_string(),
        json!(evidence.obstruction.component_id),
    );
    finding.measured.insert(
        "entry_obstruction_depth_mm".to_string(),
        json!(evidence.obstruction.depth_mm),
    );
    finding.measured.insert(
        "entry_obstruction_lateral_offset_mm".to_string(),
        json!(evidence.obstruction.lateral_offset_mm),
    );
    finding.measured.insert(
        "entry_direction_deg".to_string(),
        json!(evidence.entry_direction.deg),
    );
    finding.measured.insert(
        "entry_direction_source".to_string(),
        json!(evidence.entry_direction.source),
    );
    if let Some(offset_deg) = evidence.entry_direction.offset_deg {
        finding
            .measured
            .insert("entry_direction_offset_deg".to_string(), json!(offset_deg));
    }
    finding.measured.insert(
        "obstruction_reference".to_string(),
        json!(evidence.obstruction.reference.label()),
    );
    if let Some(layer) = evidence.obstruction.reference.footprint_layer() {
        finding.measured.insert(
            "obstruction_footprint_graphic_layer".to_string(),
            json!(layer),
        );
    }
    if let Some(kind) = evidence.obstruction.reference.footprint_kind() {
        finding.measured.insert(
            "obstruction_footprint_graphic_kind".to_string(),
            json!(kind),
        );
    }
    finding.limit.insert(
        "min_cable_entry_clearance_depth_mm".to_string(),
        json!(evidence.depth_mm),
    );
    finding.limit.insert(
        "cable_entry_clearance_width_mm".to_string(),
        json!(evidence.width_mm),
    );
    finding.suggested_fixes = vec![
        format!(
            "Move component {} outside USB connector {} cable-entry corridor or reduce the corridor only from mechanical drawing evidence.",
            evidence.obstruction.component_id, evidence.connector_id
        ),
        "Use a 3D mechanical/enclosure review for connector shell, panel, plug, cable bend radius, and assembly stack-up before treating this 2D corridor check as full clearance sign-off.".to_string(),
    ];
    finding
}
