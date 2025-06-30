use crate::panel::Point;
use imageproc::rect::Rect;

pub fn calculate_polygon_area(points: &[Point]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    let n = points.len();
    for i in 0..n {
        let p1 = &points[i];
        let p2 = &points[(i + 1) % n];
        area += (p1.x as f64 * p2.y as f64) - (p2.x as f64 * p1.y as f64);
    }
    area.abs() / 2.0
}

pub fn bounding_rect_from_points(points: &[Point]) -> Rect {
    let mut min_x = u32::MAX;
    let mut min_y = u32::MAX;
    let mut max_x = 0;
    let mut max_y = 0;

    for point in points {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    let width = if max_x > min_x { max_x - min_x } else { 1 };
    let height = if max_y > min_y { max_y - min_y } else { 1 };

    Rect::at(min_x as i32, min_y as i32).of_size(width, height)
}

// Ramer-Douglas-Peucker algorithm implementation
pub fn approximate_polygon(points: &[Point], epsilon: f64) -> Vec<Point> {
    if points.len() <= 2 {
        return points.to_vec();
    }

    let mut dmax = 0.0;
    let mut index = 0;
    let end = points.len() - 1;

    for i in 1..end {
        let d = perpendicular_distance(&points[i], &points[0], &points[end]);
        if d > dmax {
            dmax = d;
            index = i;
        }
    }

    if dmax > epsilon {
        let mut results = Vec::new();
        let rec_results1 = approximate_polygon(&points[0..=index], epsilon);
        let rec_results2 = approximate_polygon(&points[index..=end], epsilon);

        results.extend_from_slice(&rec_results1[0..rec_results1.len() - 1]);
        results.extend_from_slice(&rec_results2[0..]);
        results
    } else {
        vec![points[0].clone(), points[end].clone()]
    }
}

fn perpendicular_distance(pt: &Point, line_start: &Point, line_end: &Point) -> f64 {
    let dx = line_end.x as f64 - line_start.x as f64;
    let dy = line_end.y as f64 - line_start.y as f64;

    let mag_sq = dx * dx + dy * dy;
    if mag_sq == 0.0 {
        return distance(pt, line_start);
    }

    let u = ((pt.x as f64 - line_start.x as f64) * dx + (pt.y as f64 - line_start.y as f64) * dy)
        / mag_sq;

    let intersection_x = line_start.x as f64 + u * dx;
    let intersection_y = line_start.y as f64 + u * dy;

    distance(
        pt,
        &Point {
            x: intersection_x as u32,
            y: intersection_y as u32,
        },
    )
}

pub fn distance(p1: &Point, p2: &Point) -> f64 {
    let dx = p1.x as f64 - p2.x as f64;
    let dy = p1.y as f64 - p2.y as f64;
    (dx * dx + dy * dy).sqrt()
}

pub fn calculate_polygon_perimeter(points: &[Point]) -> f64 {
    if points.len() < 2 {
        return 0.0;
    }
    let mut perimeter = 0.0;
    for i in 0..points.len() {
        let p1 = &points[i];
        let p2 = &points[(i + 1) % points.len()]; // Wrap around for the last segment
        perimeter += distance(p1, p2);
    }
    perimeter
}
