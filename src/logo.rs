//! The Simple Osc logo: "SS", where the first S is an S (for Simple) and the second is the same
//! S turned on its side, which makes it a sine wave (for Osc).
//!
//! The S itself is one cycle of a sine wave standing upright, with a little extra at each end so
//! the tips curl like a letter. Laid on its side, the same curve is a sine wave.
//!
//! Pure geometry with no dependencies, so the GUI draws it in each theme's colors and the build
//! script (`build.rs`) renders the app icon from the same shape. Coordinates are in a box
//! `WIDTH` x 1 with y pointing down.

/// Width of the logo, relative to its height.
pub const WIDTH: f32 = 1.75;
/// Stroke width, relative to the logo height.
pub const STROKE: f32 = 0.13;

const POINTS: usize = 48;
/// How far past one full cycle the S runs at each end, so its tips curl.
const OVERSHOOT: f32 = 0.12;

/// The upright S, as a polyline.
pub fn letter_s() -> [(f32, f32); POINTS] {
    let mut points = [(0.0, 0.0); POINTS];
    for (i, p) in points.iter_mut().enumerate() {
        let t = i as f32 / (POINTS - 1) as f32;
        // One cycle of a sine, stood on end: starts at the top right, swings left, crosses in the
        // middle, swings right and ends at the bottom left
        let cycle = -OVERSHOOT + t * (1.0 + 2.0 * OVERSHOOT);
        let x = 0.3 - 0.24 * (std::f32::consts::TAU * cycle).sin();
        let y = 0.1 + 0.8 * t;
        *p = (x, y);
    }
    points
}

/// The S on its side: a sine wave to the right of the letter.
pub fn sideways_s() -> [(f32, f32); POINTS] {
    let mut points = [(0.0, 0.0); POINTS];
    for (i, p) in points.iter_mut().enumerate() {
        let t = i as f32 / (POINTS - 1) as f32;
        let x = 0.72 + 0.95 * t;
        let y = 0.5 - 0.24 * (std::f32::consts::TAU * t).sin();
        *p = (x, y);
    }
    points
}
