use crate::board_ir::{BoardLayout, CopperZone, LayoutPad, LayoutPoint, NetRoute, RouteVia};

use super::{
    GerberCopper, GerberCopperFeature, GerberCopperRegion, GerberCopperSegment, GerberPoint,
    POINT_EPSILON_MM, point_distance_mm, point_inside_polygon, polygon_signed_area_mm2,
};

#[derive(Debug, Clone)]
struct CopperNetOwner {
    net: String,
    island_id: Option<String>,
    owner_kind: Option<&'static str>,
    component: Option<String>,
    pin: Option<String>,
    via_index: Option<usize>,
}

impl CopperNetOwner {
    fn net_only(net: &str) -> Self {
        Self {
            net: net.to_string(),
            island_id: None,
            owner_kind: None,
            component: None,
            pin: None,
            via_index: None,
        }
    }

    fn pad(net: &str, component: &str, pin: &str) -> Self {
        Self {
            net: net.to_string(),
            island_id: None,
            owner_kind: Some("pad"),
            component: Some(component.to_string()),
            pin: Some(pin.to_string()),
            via_index: None,
        }
    }

    fn via(net: &str, via_index: usize) -> Self {
        Self {
            net: net.to_string(),
            island_id: None,
            owner_kind: Some("via"),
            component: None,
            pin: None,
            via_index: Some(via_index),
        }
    }
}

#[derive(Debug, Clone)]
struct CopperOwnerPad {
    at: GerberPoint,
    layers: Vec<String>,
    net: String,
    component: String,
    pin: String,
    shape: Option<String>,
    size_x_mm: Option<f64>,
    size_y_mm: Option<f64>,
}

#[derive(Debug, Clone)]
struct CopperOwnerVia {
    at: GerberPoint,
    layers: Vec<String>,
    net: String,
    via_index: usize,
    size_mm: f64,
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
    island_id: String,
    polygons: Vec<Vec<GerberPoint>>,
}

#[derive(Debug, Clone, Default)]
struct CopperOwnershipIndex {
    pads: Vec<CopperOwnerPad>,
    vias: Vec<CopperOwnerVia>,
    route_segments: Vec<CopperOwnerRouteSegment>,
    zones: Vec<CopperOwnerZone>,
}

pub(super) fn associate_copper_nets(copper: &mut GerberCopper, layout: &BoardLayout) {
    let ownership = CopperOwnershipIndex::from_layout(layout);
    for feature in &mut copper.features {
        if let Some(owner) = ownership.owner_for_feature(feature, &copper.layer) {
            feature.net = Some(owner.net);
            feature.island_id = owner.island_id;
            feature.owner_kind = owner.owner_kind.map(str::to_string);
            feature.component = owner.component;
            feature.pin = owner.pin;
            feature.via_index = owner.via_index;
        }
    }
    for segment in &mut copper.segments {
        if let Some(owner) = ownership.owner_for_segment(segment, &copper.layer) {
            segment.net = Some(owner.net);
            segment.island_id = owner.island_id;
        }
    }
    for region in &mut copper.regions {
        if let Some(owner) = ownership.owner_for_region(region, &copper.layer) {
            region.net = Some(owner.net);
            region.island_id = owner.island_id;
        }
    }
}

pub(super) fn associate_solder_mask_opening_owners(mask: &mut GerberCopper, layout: &BoardLayout) {
    let ownership = CopperOwnershipIndex::from_layout(layout);
    let Some(copper_layer) = copper_layer_for_solder_mask_layer(&mask.layer) else {
        return;
    };
    for feature in &mut mask.features {
        if let Some(owner) = ownership.owner_for_opening_feature(feature, copper_layer, true) {
            feature.net = Some(owner.net);
            feature.owner_kind = owner.owner_kind.map(str::to_string);
            feature.component = owner.component;
            feature.pin = owner.pin;
            feature.via_index = owner.via_index;
        }
    }
}

pub(super) fn associate_solder_paste_opening_owners(
    paste: &mut GerberCopper,
    layout: &BoardLayout,
) {
    let ownership = CopperOwnershipIndex::from_layout(layout);
    let Some(copper_layer) = copper_layer_for_solder_paste_layer(&paste.layer) else {
        return;
    };
    for feature in &mut paste.features {
        if let Some(owner) = ownership.owner_for_opening_feature(feature, copper_layer, false) {
            feature.net = Some(owner.net);
            feature.owner_kind = owner.owner_kind.map(str::to_string);
            feature.component = owner.component;
            feature.pin = owner.pin;
            feature.via_index = owner.via_index;
        }
    }
}

impl CopperOwnershipIndex {
    fn from_layout(layout: &BoardLayout) -> Self {
        let pads = layout
            .pads
            .iter()
            .flat_map(|(component, component_pads)| {
                component_pads
                    .iter()
                    .filter_map(|(pin, pad)| owner_pad_from_layout_pad(component, pin, pad))
            })
            .collect();
        let vias = layout
            .routes
            .iter()
            .flat_map(|(net, route)| {
                route
                    .vias
                    .iter()
                    .enumerate()
                    .filter_map(|(via_index, via)| owner_via_from_route_via(net, via_index, via))
            })
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
            vias,
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
                candidates.push(CopperNetOwner::pad(&pad.net, &pad.component, &pad.pin));
            }
        }
        for via in self.vias.iter().filter(|via| via_on_layer(via, layer)) {
            if point_inside_feature_via(feature, via) {
                candidates.push(CopperNetOwner::via(&via.net, via.via_index));
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
                candidates.push(CopperNetOwner::net_only(&route.net));
            }
        }
        for zone in self.zones.iter().filter(|zone| zone.layer == layer) {
            if zone
                .polygons
                .iter()
                .any(|polygon| point_inside_polygon(feature.at, polygon))
            {
                candidates.push(CopperNetOwner {
                    net: zone.net.clone(),
                    island_id: Some(zone.island_id.clone()),
                    owner_kind: None,
                    component: None,
                    pin: None,
                    via_index: None,
                });
            }
        }
        unique_owner(candidates)
    }

    fn owner_for_opening_feature(
        &self,
        feature: &GerberCopperFeature,
        copper_layer: &str,
        include_vias: bool,
    ) -> Option<CopperNetOwner> {
        let mut candidates = Vec::new();
        for pad in self
            .pads
            .iter()
            .filter(|pad| pad_on_layer(pad, copper_layer))
        {
            if point_inside_feature_pad(feature.at, pad) {
                candidates.push(CopperNetOwner::pad(&pad.net, &pad.component, &pad.pin));
            }
        }
        if include_vias {
            for via in self
                .vias
                .iter()
                .filter(|via| via_on_layer(via, copper_layer))
            {
                if point_inside_feature_via(feature, via) {
                    candidates.push(CopperNetOwner::via(&via.net, via.via_index));
                }
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
                candidates.push(CopperNetOwner::net_only(&route.net));
            }
        }
        for pad in self.pads.iter().filter(|pad| pad_on_layer(pad, layer)) {
            if point_to_segment_distance_mm(pad.at, segment.start, segment.end)
                <= segment.aperture.x_mm / 2.0 + POINT_EPSILON_MM
            {
                candidates.push(CopperNetOwner::net_only(&pad.net));
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
                candidates.push(CopperNetOwner {
                    net: zone.net.clone(),
                    island_id: Some(zone.island_id.clone()),
                    owner_kind: None,
                    component: None,
                    pin: None,
                    via_index: None,
                });
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
                candidates.push(CopperNetOwner {
                    net: zone.net.clone(),
                    island_id: Some(zone.island_id.clone()),
                    owner_kind: None,
                    component: None,
                    pin: None,
                    via_index: None,
                });
            }
        }
        for pad in self.pads.iter().filter(|pad| pad_on_layer(pad, layer)) {
            if point_inside_polygon(pad.at, &region.points) {
                candidates.push(CopperNetOwner::net_only(&pad.net));
            }
        }
        unique_owner(candidates)
    }
}

fn copper_layer_for_solder_mask_layer(layer: &str) -> Option<&'static str> {
    match layer {
        "F.Mask" => Some("F.Cu"),
        "B.Mask" => Some("B.Cu"),
        _ => None,
    }
}

fn copper_layer_for_solder_paste_layer(layer: &str) -> Option<&'static str> {
    match layer {
        "F.Paste" => Some("F.Cu"),
        "B.Paste" => Some("B.Cu"),
        _ => None,
    }
}

fn owner_pad_from_layout_pad(
    component: &str,
    pin: &str,
    pad: &LayoutPad,
) -> Option<CopperOwnerPad> {
    if !finite_gerber_point(point_from_layout(&pad.at)) || pad.net.trim().is_empty() {
        return None;
    }
    Some(CopperOwnerPad {
        at: point_from_layout(&pad.at),
        layers: pad.layers.clone(),
        net: pad.net.clone(),
        component: component.to_string(),
        pin: pin.to_string(),
        shape: pad.shape.clone(),
        size_x_mm: pad.size.as_ref().map(|size| size.x_mm),
        size_y_mm: pad.size.as_ref().map(|size| size.y_mm),
    })
}

fn owner_via_from_route_via(net: &str, via_index: usize, via: &RouteVia) -> Option<CopperOwnerVia> {
    if net.trim().is_empty() || !finite_gerber_point(point_from_layout(&via.at)) {
        return None;
    }
    if !via.size_mm.is_finite() || via.size_mm <= 0.0 {
        return None;
    }
    Some(CopperOwnerVia {
        at: point_from_layout(&via.at),
        layers: via.layers.clone(),
        net: net.to_string(),
        via_index,
        size_mm: via.size_mm,
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
    for (zone_index, zone) in zones.iter().enumerate() {
        if net.trim().is_empty() {
            continue;
        }
        let source_polygons = if zone.filled_polygons.is_empty() {
            vec![&zone.polygon]
        } else {
            zone.filled_polygons.iter().collect()
        };
        for (polygon_index, polygon) in source_polygons.into_iter().enumerate() {
            let polygon = points_from_layout(polygon);
            if polygon.len() < 3 || polygon_signed_area_mm2(&polygon).abs() <= f64::EPSILON {
                continue;
            }
            owner_zones.push(CopperOwnerZone {
                layer: zone.layer.clone(),
                net: net.to_string(),
                island_id: zone_owner_island_id(net, zone, zone_index, polygon_index),
                polygons: vec![polygon],
            });
        }
    }
    owner_zones
}

fn zone_owner_island_id(
    net: &str,
    zone: &CopperZone,
    zone_index: usize,
    polygon_index: usize,
) -> String {
    if let Some(island_id) = zone
        .island_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        if zone.filled_polygons.len() <= 1 {
            return island_id.to_string();
        }
        return format!("{island_id}_polygon_{polygon_index}");
    }
    format!(
        "{}_{}_zone_{}_polygon_{}",
        sanitize_island_id_part(&zone.layer),
        sanitize_island_id_part(net),
        zone_index,
        polygon_index
    )
}

fn sanitize_island_id_part(value: &str) -> String {
    let mut sanitized = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }
    let sanitized = sanitized.trim_matches('_');
    if sanitized.is_empty() {
        "unnamed".to_string()
    } else {
        sanitized.to_string()
    }
}

fn unique_owner(candidates: Vec<CopperNetOwner>) -> Option<CopperNetOwner> {
    let mut nets = candidates
        .iter()
        .filter(|candidate| !candidate.net.trim().is_empty())
        .map(|candidate| candidate.net.clone())
        .collect::<Vec<_>>();
    nets.sort();
    nets.dedup();
    if nets.len() != 1 {
        return None;
    }
    let mut island_ids = candidates
        .iter()
        .filter(|candidate| candidate.net == nets[0])
        .filter_map(|candidate| candidate.island_id.clone())
        .filter(|island_id| !island_id.trim().is_empty())
        .collect::<Vec<_>>();
    island_ids.sort();
    island_ids.dedup();
    let mut rich_owners = candidates
        .iter()
        .filter(|candidate| candidate.net == nets[0])
        .filter(|candidate| candidate.owner_kind.is_some())
        .map(|candidate| {
            (
                candidate.owner_kind,
                candidate.component.clone(),
                candidate.pin.clone(),
                candidate.via_index,
            )
        })
        .collect::<Vec<_>>();
    rich_owners.sort();
    rich_owners.dedup();
    let rich_owner = (rich_owners.len() == 1)
        .then(|| rich_owners.pop().expect("rich_owners contains one element"));
    let (owner_kind, component, pin, via_index) = rich_owner.unwrap_or((None, None, None, None));
    Some(CopperNetOwner {
        net: nets.pop().expect("nets contains one element"),
        island_id: (island_ids.len() == 1)
            .then(|| island_ids.pop().expect("island_ids contains one element")),
        owner_kind,
        component,
        pin,
        via_index,
    })
}

fn pad_on_layer(pad: &CopperOwnerPad, layer: &str) -> bool {
    pad.layers.is_empty() || pad.layers.iter().any(|candidate| candidate == layer)
}

fn via_on_layer(via: &CopperOwnerVia, layer: &str) -> bool {
    via.layers.is_empty() || via.layers.iter().any(|candidate| candidate == layer)
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

fn point_inside_feature_via(feature: &GerberCopperFeature, via: &CopperOwnerVia) -> bool {
    if feature.aperture.x_mm <= 0.0
        || feature.aperture.y_mm <= 0.0
        || !feature.aperture.x_mm.is_finite()
        || !feature.aperture.y_mm.is_finite()
    {
        return false;
    }
    point_distance_mm(feature.at, via.at)
        <= feature.aperture.x_mm.min(feature.aperture.y_mm) / 2.0
            + via.size_mm / 2.0
            + POINT_EPSILON_MM
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
