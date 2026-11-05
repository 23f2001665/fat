//! Minimal software chart renderer.
//!
//! Charts are rasterised into a Slint image instead of being drawn with a
//! declarative canvas. Only actual observations are plotted: line charts place
//! points at their recorded dates, and bar charts place one bar per recorded
//! category/week. No missing dates or samples are synthesised.

use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

use crate::models::analytics::SeriesPoint;

const WIDTH: u32 = 760;
const HEIGHT: u32 = 340;

const BG: [u8; 4] = [23, 28, 36, 255];
const GRID: [u8; 4] = [43, 52, 66, 255];
const AXIS: [u8; 4] = [70, 84, 104, 255];
const ACCENT: [u8; 4] = [94, 234, 212, 255];
const ACCENT_DARK: [u8; 4] = [30, 130, 118, 255];

struct Canvas {
    width: u32,
    height: u32,
    bytes: Vec<u8>,
}

impl Canvas {
    fn new() -> Self {
        let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(WIDTH, HEIGHT);
        let bytes = buffer.make_mut_bytes().to_vec();
        let mut canvas = Self {
            width: WIDTH,
            height: HEIGHT,
            bytes,
        };
        canvas.fill_rect(0, 0, WIDTH as i32, HEIGHT as i32, BG);
        canvas
    }

    fn set(&mut self, x: i32, y: i32, color: [u8; 4]) {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return;
        }
        let index = ((y as u32 * self.width + x as u32) * 4) as usize;
        self.bytes[index..index + 4].copy_from_slice(&color);
    }

    fn fill_rect(&mut self, x: i32, y: i32, width: i32, height: i32, color: [u8; 4]) {
        for py in y..y + height {
            for px in x..x + width {
                self.set(px, py, color);
            }
        }
    }

    fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: [u8; 4], thickness: i32) {
        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let steps = dx.max(dy).max(1);
        for step in 0..=steps {
            let t = step as f64 / steps as f64;
            let x = (x0 as f64 + (x1 - x0) as f64 * t).round() as i32;
            let y = (y0 as f64 + (y1 - y0) as f64 * t).round() as i32;
            let offset = thickness / 2;
            self.fill_rect(x - offset, y - offset, thickness, thickness, color);
        }
    }

    fn circle(&mut self, cx: i32, cy: i32, radius: i32, color: [u8; 4]) {
        for y in -radius..=radius {
            for x in -radius..=radius {
                if x * x + y * y <= radius * radius {
                    self.set(cx + x, cy + y, color);
                }
            }
        }
    }

    fn finish(self) -> Image {
        let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(self.width, self.height);
        buffer.make_mut_bytes().copy_from_slice(&self.bytes);
        Image::from_rgba8(buffer)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub min: f64,
    pub max: f64,
}

impl Bounds {
    pub fn span(self) -> f64 {
        (self.max - self.min).max(f64::EPSILON)
    }
}

/// Bounds for a line series with 10% headroom. A flat series still gets a
/// non-zero range so it can be drawn instead of collapsing onto the axis.
pub fn series_bounds(points: &[SeriesPoint]) -> Option<Bounds> {
    let mut values = points.iter().map(|point| point.value);
    let first = values.next()?;
    let mut min = first;
    let mut max = first;
    for value in values {
        min = min.min(value);
        max = max.max(value);
    }
    Some(padded(min, max))
}

fn padded(min: f64, max: f64) -> Bounds {
    if (max - min).abs() < f64::EPSILON {
        let padding = (min.abs() * 0.1).max(1.0);
        Bounds {
            min: min - padding,
            max: max + padding,
        }
    } else {
        let padding = (max - min) * 0.1;
        Bounds {
            min: min - padding,
            max: max + padding,
        }
    }
}

pub fn render_line(points: &[SeriesPoint]) -> Option<Image> {
    if points.is_empty() {
        return None;
    }
    let mut canvas = Canvas::new();
    let bounds = series_bounds(points)?;

    let left = 18;
    let right = WIDTH as i32 - 18;
    let top = 18;
    let bottom = HEIGHT as i32 - 22;
    let plot_width = (right - left) as f64;
    let plot_height = (bottom - top) as f64;

    for step in 0..=4 {
        let y = top + (plot_height * step as f64 / 4.0).round() as i32;
        canvas.line(left, y, right, y, GRID, 1);
    }
    canvas.line(left, bottom, right, bottom, AXIS, 1);

    let start = points.first().map(|point| point.date);
    let end = points.last().map(|point| point.date);
    let (Some(start), Some(end)) = (start, end) else {
        return None;
    };
    let span_days = (end - start).num_days();

    let coordinates: Vec<(i32, i32)> = points
        .iter()
        .map(|point| {
            let x = if span_days <= 0 {
                (left as f64 + plot_width / 2.0).round() as i32
            } else {
                let fraction = (point.date - start).num_days() as f64 / span_days as f64;
                (left as f64 + plot_width * fraction).round() as i32
            };
            let normalised = (point.value - bounds.min) / bounds.span();
            let y = (bottom as f64 - plot_height * normalised).round() as i32;
            (x, y)
        })
        .collect();

    for pair in coordinates.windows(2) {
        canvas.line(pair[0].0, pair[0].1, pair[1].0, pair[1].1, ACCENT_DARK, 5);
        canvas.line(pair[0].0, pair[0].1, pair[1].0, pair[1].1, ACCENT, 3);
    }
    for (x, y) in &coordinates {
        canvas.circle(*x, *y, 5, BG);
        canvas.circle(*x, *y, 4, ACCENT);
    }

    Some(canvas.finish())
}

pub fn render_bars(values: &[f64]) -> Option<Image> {
    if values.is_empty() {
        return None;
    }
    let mut canvas = Canvas::new();

    let left = 18;
    let right = WIDTH as i32 - 18;
    let top = 18;
    let bottom = HEIGHT as i32 - 22;
    let plot_width = (right - left) as f64;
    let plot_height = (bottom - top) as f64;

    for step in 0..=4 {
        let y = top + (plot_height * step as f64 / 4.0).round() as i32;
        canvas.line(left, y, right, y, GRID, 1);
    }
    canvas.line(left, bottom, right, bottom, AXIS, 1);

    let max = values.iter().copied().fold(0.0_f64, f64::max).max(1e-9);
    let slot = plot_width / values.len() as f64;
    let bar_width = (slot * 0.62).max(2.0);

    for (index, value) in values.iter().enumerate() {
        let height = ((value / max) * plot_height)
            .round()
            .clamp(0.0, plot_height) as i32;
        if height <= 0 {
            continue;
        }
        let center = left as f64 + slot * (index as f64 + 0.5);
        let x = (center - bar_width / 2.0).round() as i32;
        let width = bar_width.round() as i32;
        canvas.fill_rect(x, bottom - height, width, height, ACCENT);
        canvas.fill_rect(x, bottom - height, width, 3, [180, 255, 245, 255]);
    }

    Some(canvas.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn point(day: u32, value: f64) -> SeriesPoint {
        SeriesPoint {
            date: NaiveDate::from_ymd_opt(2026, 1, day).unwrap(),
            value,
        }
    }

    #[test]
    fn bounds_include_every_real_value() {
        let series = vec![point(1, 10.0), point(2, 30.0), point(3, 20.0)];
        let bounds = series_bounds(&series).unwrap();
        assert!(bounds.min <= 10.0);
        assert!(bounds.max >= 30.0);
    }

    #[test]
    fn flat_series_still_has_a_drawable_range() {
        let series = vec![point(1, 50.0), point(2, 50.0)];
        let bounds = series_bounds(&series).unwrap();
        assert!(bounds.max > bounds.min);
    }

    #[test]
    fn line_and_bar_images_have_expected_size() {
        let image = render_line(&[point(1, 10.0), point(5, 20.0)]).unwrap();
        assert_eq!(image.size().width, WIDTH);
        assert_eq!(image.size().height, HEIGHT);

        let image = render_bars(&[1.0, 2.0, 3.0]).unwrap();
        assert_eq!(image.size().width, WIDTH);
    }

    #[test]
    fn empty_input_renders_nothing() {
        assert!(render_line(&[]).is_none());
        assert!(render_bars(&[]).is_none());
    }
}
