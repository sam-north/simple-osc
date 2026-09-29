//! Procedural textures. Generated once on the GUI thread from layered noise, so themes can have
//! rich surfaces without shipping image files.

use nih_plug_egui::egui::{Color32, ColorImage};

use crate::ui::theme::{Color, RustPalette};

/// Deterministic hash, used for the textures and for placing static "random" details.
pub fn hash(x: i32, y: i32, seed: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343)
        ^ (y as u32).wrapping_mul(0xd816_3841)
        ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    h
}

/// Deterministic value in `[0, 1)`.
pub fn rand01(x: i32, y: i32, seed: u32) -> f32 {
    (hash(x, y, seed) >> 8) as f32 / (1u32 << 24) as f32
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn value_noise(x: f32, y: f32, seed: u32) -> f32 {
    let (xi, yi) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - x.floor(), y - y.floor());
    let (u, v) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let a = rand01(xi, yi, seed);
    let b = rand01(xi + 1, yi, seed);
    let c = rand01(xi, yi + 1, seed);
    let d = rand01(xi + 1, yi + 1, seed);
    let top = a + (b - a) * u;
    let bottom = c + (d - c) * u;
    top + (bottom - top) * v
}

fn fbm(x: f32, y: f32, octaves: u32, seed: u32) -> f32 {
    let (mut sum, mut amp, mut freq, mut total) = (0.0, 0.5, 1.0, 0.0);
    for i in 0..octaves {
        sum += value_noise(x * freq, y * freq, seed.wrapping_add(i * 101)) * amp;
        total += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / total
}

type Rgb = [f32; 3];

fn rgb(c: Color) -> Rgb {
    [c.0.r() as f32, c.0.g() as f32, c.0.b() as f32]
}

fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// Corroded metal: rust blotches, verdigris patches, streaks, pitting and scratches.
pub fn rusted_metal(width: usize, height: usize, palette: &RustPalette) -> ColorImage {
    let (iron, rust, bright) = (
        rgb(palette.iron),
        rgb(palette.rust),
        rgb(palette.rust_bright),
    );
    let (stain, scratch) = (rgb(palette.stain), rgb(palette.scratch));
    let (patina_dark, patina_light) = (rgb(palette.patina_dark), rgb(palette.patina_light));

    let mut pixels = vec![[0.0f32; 3]; width * height];
    let (cx, cy) = (width as f32 / 2.0, height as f32 / 2.0);

    for y in 0..height {
        for x in 0..width {
            let (fx, fy) = (x as f32, y as f32);

            // Large blotches of corrosion, with finer mottling on top
            let base = fbm(fx / 160.0, fy / 160.0, 5, 1);
            let mid = fbm(fx / 28.0, fy / 28.0, 3, 3);
            let tone = smoothstep(0.3, 0.75, base * 0.7 + mid * 0.3);
            let mut col = if tone < 0.5 {
                mix(iron, rust, tone * 2.0)
            } else {
                mix(rust, bright, (tone - 0.5) * 2.0)
            };

            // Verdigris patches where the metal has oxidized green
            let patina = smoothstep(0.56, 0.70, fbm(fx / 120.0 + 37.0, fy / 120.0 - 11.0, 4, 4));
            col = mix(col, mix(patina_dark, patina_light, mid), patina * 0.85);

            // Streaks running down the plate
            let drip = fbm(fx / 9.0, fy / 220.0, 3, 6);
            if drip > 0.62 {
                col = mix(col, stain, ((drip - 0.62) * 2.0).min(0.6));
            }

            // Pitting and grain
            let pit = value_noise(fx / 3.0, fy / 3.0, 5);
            let pit_shade = if pit > 0.8 {
                1.0 - (pit - 0.8) * 3.0
            } else {
                1.0
            };
            let grain = 0.82 + 0.3 * value_noise(fx / 1.7, fy / 1.7, 2);

            // Vignette darkens the edges
            let dx = (fx - cx) / cx;
            let dy = (fy - cy) / cy;
            let vignette = 1.0 - 0.35 * (dx * dx + dy * dy) / 2.0;

            let shade = pit_shade * grain * vignette;
            pixels[y * width + x] = [col[0] * shade, col[1] * shade, col[2] * shade];
        }
    }

    // Scratches: short bright gouges, mostly running in one direction, each with a dark edge
    for i in 0..260 {
        let sx = rand01(i, 0, 11) * width as f32;
        let sy = rand01(i, 1, 11) * height as f32;
        let angle = 0.35 + (rand01(i, 2, 11) - 0.5) * 0.9;
        let len = 10.0 + rand01(i, 3, 11) * 60.0;
        let strength = 0.15 + rand01(i, 4, 11) * 0.3;
        let (dx, dy) = (angle.cos(), angle.sin());
        for step in 0..len as i32 {
            let fade = 1.0 - (step as f32 / len - 0.5).abs() * 2.0;
            let px = (sx + dx * step as f32) as usize;
            let py = (sy + dy * step as f32) as usize;
            if px < width && py + 1 < height {
                let p = &mut pixels[py * width + px];
                *p = mix(*p, scratch, strength * fade);
                let s = &mut pixels[(py + 1) * width + px];
                *s = mix(*s, [0.0; 3], 0.35 * strength * fade);
            }
        }
    }

    let mut image = ColorImage::new([width, height], Color32::BLACK);
    for (dst, src) in image.pixels.iter_mut().zip(pixels) {
        let [r, g, b] = src.map(|c| c.clamp(0.0, 255.0) as u8);
        *dst = Color32::from_rgb(r, g, b);
    }
    image
}

/// White, anti-aliased flecks on transparency, tinted when drawn. Used to chip paint off titles.
pub fn flecks(width: usize, height: usize) -> ColorImage {
    let mut alpha = vec![0.0f32; width * height];
    let count = (width * height) as i32 / 18;
    for i in 0..count {
        let cx = rand01(i, 0, 1) * width as f32;
        let cy = rand01(i, 1, 1) * height as f32;
        let r = 0.4 + rand01(i, 2, 1).powi(3) * 1.8;
        let (x0, x1) = (
            (cx - r - 1.0).max(0.0) as usize,
            ((cx + r + 1.0) as usize).min(width - 1),
        );
        let (y0, y1) = (
            (cy - r - 1.0).max(0.0) as usize,
            ((cy + r + 1.0) as usize).min(height - 1),
        );
        for y in y0..=y1 {
            for x in x0..=x1 {
                let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
                let a = &mut alpha[y * width + x];
                *a = a.max((r + 0.5 - d).clamp(0.0, 1.0));
            }
        }
    }
    let mut image = ColorImage::new([width, height], Color32::TRANSPARENT);
    for (dst, a) in image.pixels.iter_mut().zip(alpha) {
        *dst = Color32::from_white_alpha((a * 255.0) as u8);
    }
    image
}

/// Amp-style tolex: a fine pebbled vinyl grain with soft highlights on the raised bumps.
pub fn tolex(
    width: usize,
    height: usize,
    base: Color,
    grain: Color,
    highlight: Color,
) -> ColorImage {
    let (base, grain, highlight) = (rgb(base), rgb(grain), rgb(highlight));
    let (cx, cy) = (width as f32 / 2.0, height as f32 / 2.0);
    let mut image = ColorImage::new([width, height], Color32::BLACK);
    for y in 0..height {
        for x in 0..width {
            let (fx, fy) = (x as f32, y as f32);
            // Small pebbles, a coarser mottle, and highlights on the tops of the bumps
            let pebble = value_noise(fx / 2.2, fy / 2.2, 31);
            let mottle = fbm(fx / 40.0, fy / 40.0, 3, 32);
            let mut col = mix(base, grain, (pebble * 0.7 + mottle * 0.3).clamp(0.0, 1.0));
            let bump = smoothstep(0.72, 0.95, value_noise(fx / 2.2 + 0.5, fy / 2.2 - 0.5, 31));
            col = mix(col, highlight, bump * 0.6);

            let dx = (fx - cx) / cx;
            let dy = (fy - cy) / cy;
            let vignette = 1.0 - 0.3 * (dx * dx + dy * dy) / 2.0;
            let [r, g, b] = col.map(|c| (c * vignette).clamp(0.0, 255.0) as u8);
            image.pixels[y * width + x] = Color32::from_rgb(r, g, b);
        }
    }
    image
}
