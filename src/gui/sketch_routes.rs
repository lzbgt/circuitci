use eframe::egui;

pub(super) fn orthogonal_points(start: egui::Pos2, end: egui::Pos2) -> Vec<egui::Pos2> {
    if (start.x - end.x).abs() <= 0.5 || (start.y - end.y).abs() <= 0.5 {
        return vec![start, end];
    }
    let mid_x = (start.x + end.x) / 2.0;
    vec![
        start,
        egui::pos2(mid_x, start.y),
        egui::pos2(mid_x, end.y),
        end,
    ]
}

pub(super) fn wire_points(
    start: egui::Pos2,
    route: &[egui::Pos2],
    end: egui::Pos2,
) -> Vec<egui::Pos2> {
    if route.is_empty() {
        return orthogonal_points(start, end);
    }
    let mut points = Vec::with_capacity(route.len() * 3 + 4);
    let mut previous = start;
    push_point(&mut points, previous);
    for waypoint in route.iter().copied().chain(std::iter::once(end)) {
        for point in orthogonal_points(previous, waypoint).into_iter().skip(1) {
            push_point(&mut points, point);
        }
        previous = waypoint;
    }
    points
}

pub(super) fn closest_point_on_polyline(position: egui::Pos2, points: &[egui::Pos2]) -> egui::Pos2 {
    points
        .windows(2)
        .map(|segment| closest_point_on_segment(position, segment[0], segment[1]))
        .min_by(|left, right| {
            left.distance_sq(position)
                .partial_cmp(&right.distance_sq(position))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or_else(|| points.last().copied().unwrap_or(position))
}

pub(super) fn route_insert_index(
    start: egui::Pos2,
    route: &[egui::Pos2],
    end: egui::Pos2,
    position: egui::Pos2,
) -> usize {
    let mut best_index = 0;
    let mut best_distance = f32::INFINITY;
    let mut previous = start;
    for (index, waypoint) in route
        .iter()
        .copied()
        .chain(std::iter::once(end))
        .enumerate()
    {
        for segment in orthogonal_points(previous, waypoint).windows(2) {
            let closest = closest_point_on_segment(position, segment[0], segment[1]);
            let distance = closest.distance_sq(position);
            if distance < best_distance {
                best_distance = distance;
                best_index = index;
            }
        }
        previous = waypoint;
    }
    best_index.min(route.len())
}

fn closest_point_on_segment(
    position: egui::Pos2,
    start: egui::Pos2,
    end: egui::Pos2,
) -> egui::Pos2 {
    let segment = end - start;
    let length_sq = segment.length_sq();
    if length_sq <= f32::EPSILON {
        return start;
    }
    let t = ((position - start).dot(segment) / length_sq).clamp(0.0, 1.0);
    start + segment * t
}

fn push_point(points: &mut Vec<egui::Pos2>, point: egui::Pos2) {
    if points
        .last()
        .is_none_or(|last| last.distance_sq(point) > 0.25)
    {
        points.push(point);
    }
}
