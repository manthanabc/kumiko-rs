
use crate::config::{Gutters, KumikoConfig, ReadingDirection};
use crate::panel::{Panel, Point, SerializablePanel};
use crate::utils::*;
use image::{GrayImage, Luma};
use imageproc::contours::{find_contours, Contour};

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

pub fn find_panels(
    img_path: &std::path::Path,
    config: &KumikoConfig,
) -> Result<((u32, u32), Vec<SerializablePanel>), Box<dyn std::error::Error>> {
    let img = image::open(img_path)?;
    let (img_w, img_h) = (img.width(), img.height());
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

    let contours: Vec<Contour<u32>> = find_contours(&binary_img);

    let mut panels: Vec<Panel> = contours
        .iter()
        .map(|c| {
            let points: Vec<Point> = c.points.iter().map(|p| Point { x: p.x, y: p.y }).collect();
            let arclength = calculate_polygon_perimeter(&points); // Calculate perimeter
            let approximated_points =
                approximate_polygon(&points, config.rdp_epsilon * arclength); // Use arclength for epsilon
            Panel::from_rect(
                bounding_rect_from_points(&approximated_points),
                approximated_points,
            )
        })
        .filter(|p| !p.is_small(img_w as i32, img_h as i32, config.small_panel_ratio)) // Filter based on ratio
        .collect();

    let mut i = 0;
    let mut panels_to_add = Vec::new();

    // Merge small panels
    while i < panels.len() {
        let p1 = &panels[i];

        if !p1.is_small(img_w as i32, img_h as i32, config.small_panel_ratio) {
            i += 1;
            continue;
        }

        let mut big_panel = p1.clone();
        let mut grouped_indices = vec![i];

        for j in (i + 1)..panels.len() {
            let p2 = &panels[j];

            if j == i || !p2.is_small(img_w as i32, img_h as i32, config.small_panel_ratio) {
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
            if !big_panel.is_small(img_w as i32, img_h as i32, config.small_panel_ratio) {
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
    panels.retain(|p| !p.is_small(img_w as i32, img_h as i32, config.small_panel_ratio));

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

    panels.retain(|p| !p.is_small(img_w as i32, img_h as i32, config.small_panel_ratio));
    expand_panels(&mut panels, &config.gutters);

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

    panels.sort_by(|a, b| {
        if (a.y - b.y).abs() < (a.height().min(b.height()) / 2) {
            // Panels are on the same row, sort by x based on reading direction
            match config.reading_direction {
                ReadingDirection::Ltr => a.x.cmp(&b.x),
                ReadingDirection::Rtl => b.x.cmp(&a.x),
            }
        } else {
            // Panels are on different rows, sort by y
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

    Ok(((img_w, img_h), serializable_panels))
}
