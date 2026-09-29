//! Renders the app icon from the logo geometry in `src/logo.rs` and embeds it into the standalone
//! `.exe` on Windows. No image files: the icon is drawn here at build time.

use std::path::PathBuf;

#[path = "src/logo.rs"]
#[allow(dead_code)]
mod logo;

/// Icon sizes, in pixels. Uncompressed, so the set is kept small (~17 KB).
const SIZES: [u32; 4] = [16, 24, 32, 48];

const BACKGROUND: [f32; 3] = [28.0, 26.0, 24.0];
const LETTER: [f32; 3] = [233.0, 228.0, 216.0];
const WAVE: [f32; 3] = [255.0, 179.0, 71.0];

fn main() {
    println!("cargo:rerun-if-changed=src/logo.rs");
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let out = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is set by cargo"));
    let icon_path = out.join("simple_osc.ico");
    std::fs::write(&icon_path, ico(&SIZES)).expect("write icon");

    let rc_path = out.join("simple_osc.rc");
    let icon = icon_path.display().to_string().replace('\\', "\\\\");
    std::fs::write(&rc_path, format!("1 ICON \"{icon}\"\n")).expect("write resource script");

    match embed_resource::compile_for(&rc_path, ["simple_osc_standalone"], embed_resource::NONE) {
        embed_resource::CompilationResult::Failed(e) => {
            panic!("embedding the app icon failed: {e}")
        }
        embed_resource::CompilationResult::NotAttempted(why) => {
            println!("cargo:warning=app icon not embedded: {why}")
        }
        _ => {}
    }
}

/// Distance from `p` to the segment `a`-`b`.
fn segment_distance(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (qx, qy) = (a.0 + t * dx - p.0, a.1 + t * dy - p.1);
    (qx * qx + qy * qy).sqrt()
}

fn polyline_distance(p: (f32, f32), points: &[(f32, f32)]) -> f32 {
    points
        .windows(2)
        .map(|w| segment_distance(p, w[0], w[1]))
        .fold(f32::MAX, f32::min)
}

/// One anti-aliased icon image, as BGRA rows from top to bottom.
fn render(size: u32) -> Vec<[u8; 4]> {
    let s = size as f32;
    // Fit the logo into the square with a margin, centered
    let scale = s * 0.8 / logo::WIDTH;
    let offset = ((s - logo::WIDTH * scale) / 2.0, (s - scale) / 2.0);
    // Thin strokes vanish at 16 px, so keep them at least 1.6 px wide
    let half_stroke = (logo::STROKE * scale * 1.2).max(1.6) / 2.0;
    let (letter, wave) = (logo::letter_s(), logo::sideways_s());
    let corner = s * 0.2;

    let mut pixels = Vec::with_capacity((size * size) as usize);
    for y in 0..size {
        for x in 0..size {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            // Rounded-square background
            let qx = (px - s / 2.0).abs() - (s / 2.0 - corner);
            let qy = (py - s / 2.0).abs() - (s / 2.0 - corner);
            let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0);
            let bg_alpha = (0.5 - (outside - corner)).clamp(0.0, 1.0);

            let local = ((px - offset.0) / scale, (py - offset.1) / scale);
            let cover = |points: &[(f32, f32)]| {
                (half_stroke + 0.5 - polyline_distance(local, points) * scale).clamp(0.0, 1.0)
            };
            let (a_letter, a_wave) = (cover(&letter), cover(&wave));

            let mut rgb = BACKGROUND;
            for c in 0..3 {
                rgb[c] += (LETTER[c] - rgb[c]) * a_letter;
                rgb[c] += (WAVE[c] - rgb[c]) * a_wave;
            }
            pixels.push([
                rgb[2] as u8,
                rgb[1] as u8,
                rgb[0] as u8,
                (bg_alpha * 255.0) as u8,
            ]);
        }
    }
    pixels
}

/// A Windows `.ico` file with one 32-bit image per size.
fn ico(sizes: &[u32]) -> Vec<u8> {
    let images: Vec<Vec<u8>> = sizes.iter().map(|&size| dib(size, &render(size))).collect();

    let mut file = Vec::new();
    file.extend_from_slice(&[0, 0, 1, 0]);
    file.extend_from_slice(&(sizes.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * sizes.len() as u32;
    for (&size, image) in sizes.iter().zip(&images) {
        let dim = if size >= 256 { 0 } else { size as u8 };
        file.extend_from_slice(&[dim, dim, 0, 0]);
        file.extend_from_slice(&1u16.to_le_bytes()); // color planes
        file.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        file.extend_from_slice(&(image.len() as u32).to_le_bytes());
        file.extend_from_slice(&offset.to_le_bytes());
        offset += image.len() as u32;
    }
    for image in images {
        file.extend_from_slice(&image);
    }
    file
}

/// An icon image in the bitmap format `.ico` files use: a header, bottom-up BGRA rows and an
/// (unused, all-zero) transparency mask.
fn dib(size: u32, pixels: &[[u8; 4]]) -> Vec<u8> {
    let mask_row = size.div_ceil(32) * 4;
    let mut out = Vec::new();
    for field in [40u32, size, size * 2] {
        out.extend_from_slice(&field.to_le_bytes());
    }
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&[0u8; 24]); // compression, image size, resolution, palette: unused
    for row in (0..size).rev() {
        for px in &pixels[(row * size) as usize..((row + 1) * size) as usize] {
            out.extend_from_slice(px);
        }
    }
    out.extend(std::iter::repeat_n(0u8, (mask_row * size) as usize));
    out
}
