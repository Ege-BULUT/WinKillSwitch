//! Small data visuals drawn with Rune shapes: a ring gauge with segments and a sparkline.

use rune::{Color, El, LineCap, Shape, canvas};

/// Points along a circle arc, clockwise from 12 o'clock, `from`..`to` as fractions of a turn.
fn arc(cx: f32, cy: f32, r: f32, from: f32, to: f32) -> Vec<[f32; 2]> {
    let sweep = (to - from).max(0.0);
    let n = ((sweep * 160.0) as usize).max(2);
    (0..=n)
        .map(|i| {
            let t = from + sweep * i as f32 / n as f32;
            let a = t * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
            [cx + r * a.cos(), cy + r * a.sin()]
        })
        .collect()
}

/// A ring of `size` points with a track and coloured segments (`from`, `to`, colour), in turns.
pub fn ring(size: f32, width: f32, track: Color, segments: &[(f32, f32, Color)]) -> El {
    let (c, r) = (size / 2.0, size / 2.0 - width / 2.0 - 1.0);
    let mut sh = Shape::new().stroke_with(arc(c, c, r, 0.0, 1.0), true, width, LineCap::Butt, track);
    for &(from, to, color) in segments {
        if to - from > 0.002 {
            sh = sh.stroke_with(arc(c, c, r, from, to.min(1.0)), false, width, LineCap::Round, color);
        }
    }
    canvas(sh).size(size, size)
}

/// A filled sparkline of values in 0..=1, oldest first.
pub fn sparkline(values: &[f32], w: f32, h: f32, line: Color, fill: Color) -> El {
    if values.len() < 2 {
        return rune::el().size(w, h);
    }
    let step = w / (values.len() - 1) as f32;
    // Scale to the data's own range (at least ten points wide) so small swings stay visible.
    let lo = values.iter().copied().fold(f32::MAX, f32::min);
    let hi = values.iter().copied().fold(f32::MIN, f32::max);
    let span = (hi - lo).max(0.10);
    let base = (lo + hi) / 2.0 - span / 2.0;
    let pts: Vec<[f32; 2]> = values.iter().enumerate().map(|(i, v)| [i as f32 * step, 3.0 + (h - 6.0) * (1.0 - ((v - base) / span).clamp(0.0, 1.0))]).collect();
    let mut area = pts.clone();
    area.push([w, h]);
    area.push([0.0, h]);
    canvas(Shape::new().fill(area, fill).stroke(pts, 2.0, line)).size(w, h)
}

/// Ease-out cubic, for count-ups.
pub fn ease(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}
