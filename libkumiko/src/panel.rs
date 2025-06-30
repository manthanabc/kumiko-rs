
use crate::config::Gutters;
use crate::utils::*;
use imageproc::rect::Rect;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Point {
    pub x: u32,
    pub y: u32,
}

#[derive(Debug, Clone)]
pub struct Panel {
    pub x: i32,
    pub y: i32,
    pub r: i32,
    pub b: i32,
    pub polygon: Vec<Point>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SerializablePanel {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Panel {
    pub fn new(x: i32, y: i32, r: i32, b: i32, polygon: Vec<Point>) -> Self {
        Self {
            x,
            y,
            r,
            b,
            polygon,
        }
    }

    pub fn from_rect(rect: Rect, polygon: Vec<Point>) -> Self {
        Self {
            x: rect.left(),
            y: rect.top(),
            r: rect.right(),
            b: rect.bottom(),
            polygon,
        }
    }

    pub fn width(&self) -> i32 {
        (self.r - self.x).max(1)
    }

    pub fn height(&self) -> i32 {
        (self.b - self.y).max(1)
    }

    pub fn to_rect(&self) -> Rect {
        Rect::at(self.x, self.y).of_size(self.width() as u32, self.height() as u32)
    }

    pub fn is_small(&self, img_w: i32, img_h: i32, ratio: f64) -> bool {
        let panel_width_f64 = self.width() as f64;
        let panel_height_f64 = self.height() as f64;
        let threshold_width = (img_w as f64) * ratio;
        let threshold_height = (img_h as f64) * ratio;

        let is_width_small = panel_width_f64 < threshold_width;
        let is_height_small = panel_height_f64 < threshold_height;

        is_width_small || is_height_small
    }

    pub fn same_row(&self, other: &Panel) -> bool {
        let (above, below) = if self.y <= other.y {
            (self, other)
        } else {
            (other, self)
        };

        if below.y > above.b {
            return false;
        }

        if below.b < above.b {
            return true;
        }

        let intersection_y = (above.b.min(below.b) - below.y) as f64;
        let min_h = above.height().min(below.height()) as f64;

        if min_h == 0.0 {
            return true;
        }

        (intersection_y / min_h) >= (1.0 / 3.0)
    }

    pub fn find_neighbour_panel<'a>(
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
    pub fn merge(&self, other: &Panel) -> Panel {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let r = (self.x + self.width()).max(other.x + other.width());
        let b = (self.y + self.height()).max(other.y + other.height());
        Panel::new(x, y, r, b, vec![]) // Polygon is not merged for simplicity
    }

    pub fn contains(&self, other: &Panel) -> bool {
        let self_rect = self.to_rect();
        let other_rect = other.to_rect();

        let wiggle_x = (other.width() as f32 * 0.3) as i32;
        let wiggle_y = (other.height() as f32 * 0.3) as i32;

        self_rect.left() <= other_rect.left() + wiggle_x
            && self_rect.right() >= other_rect.right() - wiggle_x
            && self_rect.top() <= other_rect.top() + wiggle_y
            && self_rect.bottom() >= other_rect.bottom() - wiggle_y
    }

    pub fn overlap_panel(&self, other: &Panel) -> Option<Panel> {
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

    pub fn is_close(&self, other: &Panel) -> bool {
        let c1x = self.x + self.width() / 2;
        let c1y = self.y + self.height() / 2;
        let c2x = other.x + other.width() / 2;
        let c2y = other.y + other.height() / 2;

        (c1x - c2x).abs() <= ((self.width() + other.width()) as f32 * 0.75) as i32
            && (c1y - c2y).abs() <= ((self.height() + other.height()) as f32 * 0.75) as i32
    }

    pub fn split(&self, n: u32) -> Option<Vec<Panel>> {
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
