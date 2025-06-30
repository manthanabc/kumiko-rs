use image::{GrayImage, Luma};
use imageproc::contours::{find_contours, Contour};
use imageproc::rect::Rect;
use serde::Serialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq)]
struct Point {
    x: u32,
    y: u32,
}

#[derive(Debug, Clone)]
struct Panel {
    x: i32,
    y: i32,
    r: i32,
    b: i32,
    polygon: Vec<Point>,
}

#[derive(Debug, Clone, Serialize)]
struct SerializablePanel {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Debug, Clone, Copy)]
struct Gutters {
    x: i32,
    y: i32,
    r: i32,
    b: i32,
}

impl Panel {
    fn new(x: i32, y: i32, r: i32, b: i32, polygon: Vec<Point>) -> Self {
        Self {
            x,
            y,
            r,
            b,
            polygon,
        }
    }

    fn from_rect(rect: Rect, polygon: Vec<Point>) -> Self {
        Self {
            x: rect.left(),
            y: rect.top(),
            r: rect.right(),
            b: rect.bottom(),
            polygon,
        }
    }

    fn width(&self) -> i32 {
        (self.r - self.x).max(1)
    }

    fn height(&self) -> i32 {
        (self.b - self.y).max(1)
    }

    fn to_rect(&self) -> Rect {
        Rect::at(self.x, self.y).of_size(self.width() as u32, self.height() as u32)
    }

    fn is_small(&self, img_w: i32, img_h: i32, ratio: f64) -> bool {
        let panel_width_f64 = self.width() as f64;
        let panel_height_f64 = self.height() as f64;
        let threshold_width = (img_w as f64) * ratio;
        let threshold_height = (img_h as f64) * ratio;

        let is_width_small = panel_width_f64 < threshold_width;
        let is_height_small = panel_height_f64 < threshold_height;

        is_width_small || is_height_small
    }

    fn same_row(&self, other: &Panel) -> bool {
        // Sort by y coordinate
        let (above, below) = if self.y <= other.y {
            (self, other)
        } else {
            (other, self)
        };

        // strictly above
        if below.y > above.b {
            return false;
        }

        // contained vertically
        if below.b < above.b {
            return true;
        }

        // intersect
        let intersection_y = (above.b.min(below.b) - below.y) as f64;
        let min_h = above.height().min(below.height()) as f64;

        if min_h == 0.0 {
            return true;
        }

        (intersection_y / min_h) >= (1.0 / 3.0)
    }

    fn find_neighbour_panel<'a>(
        &self,
        direction: &str,
        panels: &'a [Panel],
        _gutters: &Gutters, // gutters are unused here, but can be applied in expand_panels
    ) -> Option<&'a Panel> {
        match direction {
            "x" => panels
                .iter()
                .filter(|p| p.r <= self.x) // to the left
                .filter(|p| p.same_row(self))
                .max_by_key(|p| p.r), // closest on left
            "r" => panels
                .iter()
                .filter(|p| p.x >= self.r) // to the right
                .filter(|p| p.same_row(self))
                .min_by_key(|p| p.x), // closest on right
            "y" => panels
                .iter()
                .filter(|p| p.b <= self.y) // above
                .filter(|p| p.x <= self.r && p.r >= self.x) // horizontally overlapping
                .max_by_key(|p| p.b), // closest above
            "b" => panels
                .iter()
                .filter(|p| p.y >= self.b) // below
                .filter(|p| p.x <= self.r && p.r >= self.x) // horizontally overlapping
                .min_by_key(|p| p.y), // closest below
            _ => None,
        }
    }
    fn merge(&self, other: &Panel) -> Panel {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let r = (self.x + self.width()).max(other.x + other.width());
        let b = (self.y + self.height()).max(other.y + other.height());
        Panel::new(x, y, r, b, vec![]) // Polygon is not merged for simplicity
    }

    fn contains(&self, other: &Panel) -> bool {
        let self_rect = self.to_rect();
        let other_rect = other.to_rect();

        let wiggle_x = (other.width() as f32 * 0.3) as i32;
        let wiggle_y = (other.height() as f32 * 0.3) as i32;

        self_rect.left() <= other_rect.left() + wiggle_x
            && self_rect.right() >= other_rect.right() - wiggle_x
            && self_rect.top() <= other_rect.top() + wiggle_y
            && self_rect.bottom() >= other_rect.bottom() - wiggle_y
    }

    fn overlap_panel(&self, other: &Panel) -> Option<Panel> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let r = self.r.min(other.r);
        let b = self.b.min(other.b);

        if x < r && y < b {
            Some(Panel::new(x, y, r, b, vec![]))
        } else {
            None
        }
    }

    fn is_close(&self, other: &Panel) -> bool {
        let c1x = self.x + self.width() / 2;
        let c1y = self.y + self.height() / 2;
        let c2x = other.x + other.width() / 2;
        let c2y = other.y + other.height() / 2;

        (c1x - c2x).abs() <= ((self.width() + other.width()) as f32 * 0.75) as i32
            && (c1y - c2y).abs() <= ((self.height() + other.height()) as f32 * 0.75) as i32
    }

    fn split(&self, n: u32) -> Option<Vec<Panel>> {
        if n == 2 {
            return None;
        }
        if self.polygon.is_empty() {
            return None;
        }

        let close_dots = self._find_close_dots();
        if close_dots.is_empty() {
            return None;
        }

        let cuts = self._sort_cuts_by_distance(close_dots);

        for cut in cuts {
            if !self._is_valid_cut(&cut) {
                continue;
            }

            let (poly1, poly2) = self._split_polygon(&cut);

            let panel1 = Panel::from_rect(bounding_rect_from_points(&poly1), poly1);
            let panel2 = Panel::from_rect(bounding_rect_from_points(&poly2), poly2);

            if !self._valid_subpanels(&panel1, &panel2) {
                continue;
            }

            let mut subpanels = Vec::new();
            if let Some(mut s) = panel1.split(n + 1) {
                subpanels.append(&mut s);
            } else {
                subpanels.push(panel1);
            }
            if let Some(mut s) = panel2.split(n + 1) {
                subpanels.append(&mut s);
            } else {
                subpanels.push(panel2);
            }
            return Some(subpanels);
        }
        None
    }

    fn _find_close_dots(&self) -> Vec<(usize, usize)> {
        let mut close_dots = Vec::new();
        let ratio = 0.25;
        let max_dist_w = (self.width() as f64 * ratio) as i32;
        let max_dist_h = (self.height() as f64 * ratio) as i32;
        let max_dist = max_dist_w.min(max_dist_h);

        for i in 0..self.polygon.len() {
            for j in (i + 1)..self.polygon.len() {
                let dot1 = &self.polygon[i];
                let dot2 = &self.polygon[j];

                if (dot1.x as i32 - dot2.x as i32).abs() < max_dist
                    && (dot1.y as i32 - dot2.y as i32).abs() < max_dist
                {
                    close_dots.push((i, j));
                }
            }
        }
        close_dots
    }

    fn _sort_cuts_by_distance(&self, cuts: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
        let mut sorted_cuts = cuts;
        sorted_cuts.sort_by_key(|(i, j)| {
            let dot1 = &self.polygon[*i];
            let dot2 = &self.polygon[*j];
            ((dot1.x as i32 - dot2.x as i32).abs() + (dot1.y as i32 - dot2.y as i32).abs()) as u32
        });
        sorted_cuts
    }

    fn _is_valid_cut(&self, cut: &(usize, usize)) -> bool {
        let poly1_len = self.polygon.len() - cut.1 + cut.0;
        let poly2_len = cut.1 - cut.0;
        poly1_len > 2 && poly2_len > 2
    }

    fn _split_polygon(&self, cut: &(usize, usize)) -> (Vec<Point>, Vec<Point>) {
        let mut poly1 = Vec::new();
        let mut poly2 = Vec::new();

        for (idx, point) in self.polygon.iter().enumerate() {
            if idx <= cut.0 || idx >= cut.1 {
                poly1.push(point.clone());
            } else {
                poly2.push(point.clone());
            }
        }
        (poly1, poly2)
    }

    fn _valid_subpanels(&self, panel1: &Panel, panel2: &Panel) -> bool {
        if (panel1.height() as f64 / self.height() as f64) < 0.1
            || (panel1.width() as f64 / self.width() as f64) < 0.1
            || (panel2.height() as f64 / self.height() as f64) < 0.1
            || (panel2.width() as f64 / self.width() as f64) < 0.1
        {
            return false;
        }

        let area1 = calculate_polygon_area(&panel1.polygon);
        let area2 = calculate_polygon_area(&panel2.polygon);

        if area1 == 0.0 || area2 == 0.0 {
            return false;
        }

        (area1.min(area2) / area1.max(area2)) >= 0.1
    }
}

fn calculate_polygon_area(points: &[Point]) -> f64 {
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

fn bounding_rect_from_points(points: &[Point]) -> Rect {
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

fn get_background_color(img: &GrayImage) -> &str {
    let (width, height) = img.dimensions();
    let mut white_pixels = 0;
    let mut black_pixels = 0;

    for y in 0..height {
        for x in 0..width {
            let pixel = img.get_pixel(x, y)[0];
            if pixel > 200 {
                // Assuming >200 is white
                white_pixels += 1;
            } else if pixel < 50 {
                // Assuming <50 is black
                black_pixels += 1;
            }
        }
    }

    if white_pixels > black_pixels {
        "white"
    } else {
        "black"
    }
}

// Ramer-Douglas-Peucker algorithm implementation
fn approximate_polygon(points: &[Point], epsilon: f64) -> Vec<Point> {
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

fn distance(p1: &Point, p2: &Point) -> f64 {
    let dx = p1.x as f64 - p2.x as f64;
    let dy = p1.y as f64 - p2.y as f64;
    (dx * dx + dy * dy).sqrt()
}

fn calculate_polygon_perimeter(points: &[Point]) -> f64 {
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

// Add this new function to your code.
fn resolve_overlaps(panels: &mut Vec<Panel>) {
    // We loop enough times to ensure all overlaps have been resolved.
    // For most comic layouts, a few passes are more than sufficient.
    for _ in 0..1 {
        let mut overlaps_found = false;
        for i in 0..panels.len() {
            for j in (i + 1)..panels.len() {
                // We need to re-split the mutable borrow for each pair.
                let (panels_left, panels_right) = panels.split_at_mut(j);
                let p1 = &mut panels_left[i];
                let p2 = &mut panels_right[0];

                if let Some(overlap) = p1.overlap_panel(p2) {
                    overlaps_found = true;
                    // Decide if the overlap is primarily horizontal or vertical.
                    if overlap.width() < overlap.height() {
                        // Vertical overlap: Adjust the panels' y and b coordinates.
                        let (top_panel, bottom_panel) =
                            if p1.y < p2.y { (p1, p2) } else { (p2, p1) };
                        let midpoint = top_panel.b + overlap.height() / 2;

                        top_panel.b = midpoint;
                        bottom_panel.y = midpoint;
                    } else {
                        // Horizontal overlap: Adjust the panels' x and r coordinates.
                        let (left_panel, right_panel) =
                            if p1.x < p2.x { (p1, p2) } else { (p2, p1) };
                        let midpoint = left_panel.r + overlap.width() / 2;

                        left_panel.r = midpoint;
                        right_panel.x = midpoint;
                    }
                }
            }
        }
        // If a full pass found no overlaps, we can stop early.
        if !overlaps_found {
            break;
        }
    }
}

fn expand_panels(panels: &mut Vec<Panel>, gutters: &Gutters) {
    let directions = ["x", "y", "r", "b"];
    // 1. Create a single, stable clone for reference. This prevents basing
    //    calculations on partially updated data within the same pass.
    let original_panels = panels.clone();

    // 2. Iterate through the indices to get mutable access to the panels one by one.
    for i in 0..panels.len() {
        for &d in &directions {
            // Find the neighbor based on the panel's original state.
            let neighbour = original_panels[i].find_neighbour_panel(d, &original_panels, gutters);

            let new_coord = if let Some(n) = neighbour {
                // 3. CORRECTED GUTTER LOGIC:
                //    Expand the panel's edge towards the neighbor's edge,
                //    leaving the specified gutter space.
                match d {
                    "x" => n.r + gutters.x, // For left edge, move towards neighbor's right edge
                    "r" => n.x - gutters.r, // For right edge, move towards neighbor's left edge
                    "y" => n.b + gutters.y, // For top edge, move towards neighbor's bottom edge
                    "b" => n.y - gutters.b, // For bottom edge, move towards neighbor's top edge
                    _ => continue,
                }
            } else {
                // No neighbor: expand to the outermost boundary of all panels.
                // (This logic can be replaced with expanding to image boundary: 0, 0, img_w, img_h if needed)
                match d {
                    "x" => original_panels
                        .iter()
                        .map(|p| p.x)
                        .min()
                        .unwrap_or(panels[i].x),
                    "y" => original_panels
                        .iter()
                        .map(|p| p.y)
                        .min()
                        .unwrap_or(panels[i].y),
                    "r" => original_panels
                        .iter()
                        .map(|p| p.r)
                        .max()
                        .unwrap_or(panels[i].r),
                    "b" => original_panels
                        .iter()
                        .map(|p| p.b)
                        .max()
                        .unwrap_or(panels[i].b),
                    _ => continue,
                }
            };

            // 4. Get a mutable reference to the panel we are actually updating.
            let p = &mut panels[i];

            // 5. Apply the new coordinate only if it's a valid expansion.
            match d {
                "x" if new_coord < p.x => p.x = new_coord,
                "y" if new_coord < p.y => p.y = new_coord,
                "r" if new_coord > p.r => p.r = new_coord,
                "b" if new_coord > p.b => p.b = new_coord,
                _ => {} // Do nothing if it's not an expansion
            }
        }
    }
}

fn process_image(img_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // println!("Processing image: {:?}", img_path);

    let img = image::open(img_path)?;
    let (img_w, img_h) = (img.width() as i32, img.height() as i32);
    let gray_img = img.to_luma8();

    let bg_color = get_background_color(&gray_img);
    let (threshold_val, invert) = match bg_color {
        "white" => (220u8, true),
        "black" => (50u8, false),
        _ => (100u8, false), // Default, should not happen with current logic
    };

    let mut binary_img = GrayImage::new(img.width(), img.height());
    for (x, y, pixel) in gray_img.enumerate_pixels() {
        let val = pixel[0];
        if invert {
            if val > threshold_val {
                binary_img.put_pixel(x, y, Luma([0u8])); // Pixels > threshold become black
            } else {
                binary_img.put_pixel(x, y, Luma([255u8])); // Pixels <= threshold become white
            }
        } else {
            if val > threshold_val {
                binary_img.put_pixel(x, y, Luma([255u8])); // Pixels > threshold become white
            } else {
                binary_img.put_pixel(x, y, Luma([0u8])); // Pixels <= threshold become black
            }
        }
    }
    // binary_img.save("output_panels/binary.png")?;
    let contours: Vec<Contour<u32>> = find_contours(&binary_img);

    let mut panels: Vec<Panel> = contours
        .iter()
        .map(|c| {
            let points: Vec<Point> = c.points.iter().map(|p| Point { x: p.x, y: p.y }).collect();
            let arclength = calculate_polygon_perimeter(&points); // Calculate perimeter
            let approximated_points = approximate_polygon(&points, 0.01 * arclength); // Use arclength for epsilon
            Panel::from_rect(
                bounding_rect_from_points(&approximated_points),
                approximated_points,
            )
        })
        .filter(|p| !p.is_small(img_w, img_h, 1.0 / 15.0)) // Filter based on ratio
        .collect();

    // Save contours image
    let mut debug_img = img.to_rgb8();
    for contour in &contours {
        for pt in &contour.points {
            if pt.x < debug_img.width() && pt.y < debug_img.height() {
                debug_img.put_pixel(pt.x, pt.y, image::Rgb([255, 0, 0]));
            }
        }
    }
    // debug_img.save("output_panels/contours.png")?;

    let mut debug_img = img.to_rgb8();
    for contour in &panels {
        for pt in &contour.polygon {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let nx = pt.x as i32 + dx;
                    let ny = pt.y as i32 + dy;
                    if nx >= 0
                        && ny >= 0
                        && nx < debug_img.width() as i32
                        && ny < debug_img.height() as i32
                    {
                        debug_img.put_pixel(nx as u32, ny as u32, image::Rgb([255, 0, 0]));
                    }
                }
            }
        }
    }
    // debug_img.save("output_panels/contourss.png")?;

    // println!("Before grouping: {} panels", panels.len());
    let mut i = 0;
    let mut panels_to_add = Vec::new();
    let rr = 1.0 / 15.0;

    while i < panels.len() {
        let p1 = &panels[i];

        if !p1.is_small(img_w, img_h, rr) {
            i += 1;
            continue;
        }

        let mut big_panel = p1.clone();
        let mut grouped_indices = vec![i];

        for j in (i + 1)..panels.len() {
            let p2 = &panels[j];

            if j == i || !p2.is_small(img_w, img_h, rr) {
                continue;
            }

            if p2.is_close(&big_panel) {
                grouped_indices.push(j);
                big_panel = big_panel.merge(p2);
            }
        }

        if grouped_indices.len() <= 1 {
            panels.remove(i);
            continue; // ← match Python: re-evaluate same index after shifting
        } else {
            if !big_panel.is_small(img_w, img_h, rr) {
                panels_to_add.push(big_panel);
            }

            // Remove all grouped panels in reverse order
            grouped_indices.sort_unstable_by(|a, b| b.cmp(a));
            for k in grouped_indices {
                panels.remove(k);
            }
        }

        i += 1;
    }

    // Add new merged panels at the end
    panels.append(&mut panels_to_add);

    // println!("After grouping: {} panels", panels.len());

    // println!("Before splitting: {} panels", panels.len());
    // Split panels
    let mut panels_to_split = Vec::new();
    for p in panels.drain(..) {
        if let Some(mut new_panels) = p.split(0) {
            panels_to_split.append(&mut new_panels);
        } else {
            panels_to_split.push(p);
        }
    }
    panels = panels_to_split;

    // Re-filter out small panels after splitting
    panels.retain(|p| !p.is_small(img_w, img_h, 1.0 / 15.0));
    // println!("After splitting: {} panels", panels.len());

    // println!("Before merging contained: {} panels", panels.len());
    // Merge contained panels
    let mut merged = true;
    while merged {
        merged = false;
        let mut i = 0;
        while i < panels.len() {
            let mut j = 0;
            while j < panels.len() {
                if i == j {
                    j += 1;
                    continue;
                }
                if panels[i].contains(&panels[j]) {
                    panels[i] = panels[i].merge(&panels[j]);
                    panels.remove(j);
                    merged = true;
                    if j < i {
                        i -= 1;
                    }
                } else {
                    j += 1;
                }
            }
            i += 1;
        }
    }
    // println!("After merging contained: {} panels", panels.len());

    let gutters = Gutters {
        x: -2,
        y: -2,
        r: 2,
        b: 2,
    }; // example gutter of ±2px
    println!("Before de-overlapping: {} panels", panels.len());
    // De-overlap panels
    for i in 0..panels.len() {
        for j in 0..panels.len() {
            if i == j {
                continue;
            }
            if let Some(overlap) = panels[i].overlap_panel(&panels[j]) {
                if overlap.width() < overlap.height() && panels[i].r == overlap.r {
                    // Vertical overlap, right aligned
                    panels[i].r = overlap.x;
                    panels[j].x = overlap.r;
                    continue;
                }

                if overlap.width() > overlap.height() && panels[i].b == overlap.b {
                    // Horizontal overlap, bottom aligned
                    panels[i].b = overlap.y;
                    panels[j].y = overlap.b;
                    continue;
                }
            }
        }
    }
    println!("After de-overlapping: {} panels", panels.len());
    panels.retain(|p| !p.is_small(img_w, img_h, 1.0 / 15.0));
    expand_panels(&mut panels, &gutters);

    for i in 0..panels.len() {
        for j in i + 1..panels.len() {
            if let Some(overlap) = panels[i].overlap_panel(&panels[j]) {
                if overlap.width() < overlap.height() && panels[i].r == overlap.r {
                    // Vertical overlap, right aligned
                    panels[i].r = overlap.x;
                    panels[j].x = overlap.r;
                    continue;
                }

                if overlap.width() > overlap.height() && panels[i].b == overlap.b {
                    // Horizontal overlap, bottom aligned
                    panels[i].b = overlap.y;
                    panels[j].y = overlap.b;
                    continue;
                }
            }
        }
    }
    // resolve_overlaps(&mut panels);

    panels.sort_by(|a, b| {
        if (a.y - b.y).abs() < (a.height().min(b.height()) / 2) {
            a.x.cmp(&b.x)
        } else {
            a.y.cmp(&b.y)
        }
    });

    let serializable_panels: Vec<SerializablePanel> = panels
        .into_iter()
        .map(|p| SerializablePanel {
            x: p.x,
            y: p.y,
            width: p.width(),
            height: p.height(),
        })
        .collect();
    let output_dir = Path::new("output_panels");
    fs::create_dir_all(&output_dir)?;
    let output_file_name = img_path.file_stem().unwrap().to_str().unwrap().to_owned() + ".json";
    let output_path = output_dir.join(output_file_name);

    let json_output = serde_json::to_string_pretty(&serializable_panels)?;
    fs::write(&output_path, json_output)?;

    // println!(
    // "✅ Successfully processed image and saved panels to {:?}",
    // output_path
    // );

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        panic!("Usage: kumiko_rs <path_to_image_or_directory>");
    }
    let input_path = PathBuf::from(&args[1]);

    if input_path.is_dir() {
        // println!("Processing directory: {:?}", input_path);
        for entry in fs::read_dir(&input_path)? {
            let entry = entry?;
            let path = entry.path();
            // println!("  Found entry: {:?}", path);
            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if let Some(ext_str) = extension.to_str().map(|s| s.to_lowercase()) {
                        match ext_str.as_str() {
                            "jpg" | "jpeg" | "png" => {
                                // println!("    Processing image file: {:?}", path);
                                process_image(&path)?;
                            }
                            _ => {
                                // println!("    Skipping non-image file: {:?}", path);
                            }
                        }
                    }
                } else {
                    // println!("    Skipping file with no extension: {:?}", path);
                }
            } else if path.is_dir() {
                // Process files in immediate subdirectories
                // println!("    Entering subdirectory: {:?}", path);
                for sub_entry in fs::read_dir(&path)? {
                    let sub_entry = sub_entry?;
                    let sub_path = sub_entry.path();
                    if sub_path.is_file() {
                        if let Some(extension) = sub_path.extension() {
                            if let Some(ext_str) = extension.to_str().map(|s| s.to_lowercase()) {
                                match ext_str.as_str() {
                                    "jpg" | "jpeg" | "png" => {
                                        // println!("      Processing image file: {:?}", sub_path);
                                        process_image(&sub_path)?;
                                    }
                                    _ => {
                                        // println!("      Skipping non-image file: {:?}", sub_path);
                                    }
                                }
                            }
                        }
                    } else {
                        // println!("      Skipping non-file entry: {:?}", sub_path);
                    }
                }
            } else {
                // println!("    Skipping non-file/non-directory entry: {:?}", path);
            }
        }
    } else if input_path.is_file() {
        process_image(&input_path)?;
    } else {
        panic!("Invalid input path: {:?}", input_path);
    }

    Ok(())
}
