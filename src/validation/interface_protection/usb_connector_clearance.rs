use crate::board_ir::{LayoutPoint, Scenario};
use crate::library::BoundBoard;
use crate::reports::Finding;

use super::super::common::validation_input_missing;
use super::usb_connector::{
    footprint_arc_points, footprint_circle_points, mechanical_footprint_kind, placement_is_finite,
    point_is_finite, point_to_segment_distance_mm, rectangle_corners,
    required_usb_connector_nonnegative_parameter, segment_length_mm,
    segment_to_segment_distance_mm,
};
use super::usb_connector_findings::{
    usb_component_clearance_finding, usb_component_clearance_metadata_finding,
};

pub(super) fn validate_usb_connector_component_clearance(
    bound: &BoundBoard<'_>,
    scenario: &Scenario,
    findings: &mut Vec<Finding>,
) {
    let rule = &bound.project.board.layout.constraints.usb_connector;
    let Some(min_clearance_mm) = required_usb_connector_nonnegative_parameter(
        scenario,
        rule,
        "min_connector_to_component_clearance_mm",
        rule.min_connector_to_component_clearance_mm,
        findings,
    ) else {
        return;
    };
    if min_clearance_mm < 0.0 {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection parameters.min_connector_to_component_clearance_mm must be zero or greater.",
        );
        return;
    }
    let Some(target) = &scenario.target else {
        validation_input_missing(
            findings,
            scenario,
            "interface_protection target.component is required for USB_CONNECTOR_COMPONENT_CLEARANCE_VALID.",
        );
        return;
    };
    let Some(component) = bound.project.board.components.get(&target.component) else {
        findings.push(usb_component_clearance_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector component-clearance target component {} is not declared.",
                target.component
            ),
            "component",
            &target.component,
        ));
        return;
    };
    let Some(model) = bound.library.get(&component.model) else {
        findings.push(usb_component_clearance_metadata_finding(
            scenario,
            &target.component,
            format!(
                "USB connector component-clearance target component {} model {} is not loaded.",
                target.component, component.model
            ),
            "model",
            &component.model,
        ));
        return;
    };
    if model.usb_connector.is_none() {
        findings.push(usb_component_clearance_metadata_finding(
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
    }
    let connector_primitives = mechanical_clearance_primitives(bound, &target.component, false);
    if connector_primitives.is_empty() {
        findings.push(usb_component_clearance_metadata_finding(
            scenario,
            &target.component,
            "USB connector component clearance requires imported fabrication/courtyard footprint graphics for the target connector.".to_string(),
            "connector_footprint",
            "missing",
        ));
        return;
    }

    for other_component_id in bound.project.board.components.keys() {
        if other_component_id == &target.component {
            continue;
        }
        let other_primitives = mechanical_clearance_primitives(bound, other_component_id, true);
        if other_primitives.is_empty() {
            continue;
        }
        let Some((clearance_mm, connector_reference, other_reference)) =
            nearest_clearance_between(&connector_primitives, &other_primitives)
        else {
            continue;
        };
        if clearance_mm < min_clearance_mm {
            findings.push(usb_component_clearance_finding(
                UsbComponentClearanceEvidence {
                    scenario,
                    connector_id: &target.component,
                    other_component_id,
                    clearance_mm,
                    min_clearance_mm,
                    connector_reference,
                    other_reference,
                },
            ));
        }
    }
}

pub(super) struct UsbComponentClearanceEvidence<'a> {
    pub(super) scenario: &'a Scenario,
    pub(super) connector_id: &'a str,
    pub(super) other_component_id: &'a str,
    pub(super) clearance_mm: f64,
    pub(super) min_clearance_mm: f64,
    pub(super) connector_reference: UsbComponentClearanceReference<'a>,
    pub(super) other_reference: UsbComponentClearanceReference<'a>,
}

#[derive(Clone, Copy)]
pub(super) enum UsbComponentClearanceReference<'a> {
    PlacementCenter,
    FootprintSegment { layer: &'a str, kind: &'a str },
    FootprintRectangle { layer: &'a str, kind: &'a str },
    FootprintPolygon { layer: &'a str, kind: &'a str },
    FootprintCircle { layer: &'a str, kind: &'a str },
    FootprintArc { layer: &'a str, kind: &'a str },
}

impl UsbComponentClearanceReference<'_> {
    pub(super) fn label(&self) -> &'static str {
        match self {
            UsbComponentClearanceReference::PlacementCenter => "placement_center",
            UsbComponentClearanceReference::FootprintSegment { .. } => "footprint_segment",
            UsbComponentClearanceReference::FootprintRectangle { .. } => "footprint_rectangle",
            UsbComponentClearanceReference::FootprintPolygon { .. } => "footprint_polygon",
            UsbComponentClearanceReference::FootprintCircle { .. } => "footprint_circle",
            UsbComponentClearanceReference::FootprintArc { .. } => "footprint_arc",
        }
    }

    pub(super) fn footprint_layer(&self) -> Option<&str> {
        match self {
            UsbComponentClearanceReference::PlacementCenter => None,
            UsbComponentClearanceReference::FootprintSegment { layer, .. }
            | UsbComponentClearanceReference::FootprintRectangle { layer, .. }
            | UsbComponentClearanceReference::FootprintPolygon { layer, .. }
            | UsbComponentClearanceReference::FootprintCircle { layer, .. }
            | UsbComponentClearanceReference::FootprintArc { layer, .. } => Some(layer),
        }
    }

    pub(super) fn footprint_kind(&self) -> Option<&str> {
        match self {
            UsbComponentClearanceReference::PlacementCenter => None,
            UsbComponentClearanceReference::FootprintSegment { kind, .. }
            | UsbComponentClearanceReference::FootprintRectangle { kind, .. }
            | UsbComponentClearanceReference::FootprintPolygon { kind, .. }
            | UsbComponentClearanceReference::FootprintCircle { kind, .. }
            | UsbComponentClearanceReference::FootprintArc { kind, .. } => Some(kind),
        }
    }
}

#[derive(Clone)]
enum ClearancePrimitive<'a> {
    Point {
        point: LayoutPoint,
        reference: UsbComponentClearanceReference<'a>,
    },
    Segment {
        start: LayoutPoint,
        end: LayoutPoint,
        reference: UsbComponentClearanceReference<'a>,
    },
}

impl<'a> ClearancePrimitive<'a> {
    fn reference(&self) -> UsbComponentClearanceReference<'a> {
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
                || segment_length_mm(&segment.start, &segment.end) <= f64::EPSILON
            {
                continue;
            }
            primitives.push(ClearancePrimitive::Segment {
                start: segment.start.clone(),
                end: segment.end.clone(),
                reference: UsbComponentClearanceReference::FootprintSegment {
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
                UsbComponentClearanceReference::FootprintRectangle {
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
                UsbComponentClearanceReference::FootprintPolygon {
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
                UsbComponentClearanceReference::FootprintCircle {
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
                UsbComponentClearanceReference::FootprintArc {
                    layer: &arc.layer,
                    kind: &arc.kind,
                },
            );
        }
    }
    if primitives.is_empty()
        && include_placement_fallback
        && let Some(placement) = bound.project.board.layout.placements.get(component_id)
        && placement_is_finite(placement)
    {
        primitives.push(ClearancePrimitive::Point {
            point: LayoutPoint {
                x_mm: placement.x_mm,
                y_mm: placement.y_mm,
            },
            reference: UsbComponentClearanceReference::PlacementCenter,
        });
    }
    primitives
}

fn push_closed_clearance_polyline<'a>(
    primitives: &mut Vec<ClearancePrimitive<'a>>,
    points: &[LayoutPoint],
    reference: UsbComponentClearanceReference<'a>,
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
    reference: UsbComponentClearanceReference<'a>,
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
    UsbComponentClearanceReference<'a>,
    UsbComponentClearanceReference<'a>,
)> {
    let mut nearest: Option<(
        f64,
        UsbComponentClearanceReference<'a>,
        UsbComponentClearanceReference<'a>,
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
        ) => segment_length_mm(a, b),
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
