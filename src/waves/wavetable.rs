//! Band-limited playback of single-cycle waveforms.
//!
//! A raw single-cycle wave played at a high note contains harmonics above the Nyquist frequency,
//! which fold back as harsh, inharmonic aliasing. So each wave is analyzed once (a DFT of its
//! cycle) and rebuilt as a set of tables, each keeping only half the harmonics of the one
//! before. Playback picks the richest table whose harmonics all fit below Nyquist.

use std::f64::consts::TAU;

/// Length of the richest table. Each level with half the harmonics gets half the length (down
/// to [`MIN_TABLE_LEN`]), keeping the same number of samples per cycle of its top harmonic, so
/// every level interpolates equally well while using far less memory.
const TABLE_LEN: usize = 2048;
const MIN_TABLE_LEN: usize = 64;
/// Keep the top harmonic a little below Nyquist, leaving room for interpolation error.
const MAX_HARMONIC_FRACTION: f32 = 0.45;

pub struct Wavetable {
    /// `levels[i]` holds at most `harmonics[i]` harmonics, plus one guard sample at the end for
    /// interpolation.
    levels: Vec<Box<[f32]>>,
    harmonics: Vec<usize>,
}

impl Wavetable {
    /// Builds the tables from one cycle of samples. Runs once, off the audio thread.
    pub fn from_cycle(cycle: &[f32]) -> Self {
        let n = cycle.len().max(1);
        let max_harmonic = (n / 2).min(TABLE_LEN / 2 - 1).max(1);

        // Fourier coefficients of the cycle, using a precomputed table of sines and cosines.
        // Harmonic 0 (DC offset) is dropped.
        let twiddle: Vec<(f64, f64)> = (0..n)
            .map(|i| {
                let a = TAU * i as f64 / n as f64;
                (a.cos(), a.sin())
            })
            .collect();
        let coefficients: Vec<(f64, f64)> = (1..=max_harmonic)
            .map(|h| {
                cycle
                    .iter()
                    .enumerate()
                    .fold((0.0, 0.0), |(re, im), (i, &x)| {
                        let (c, s) = twiddle[(h * i) % n];
                        (re + x as f64 * c, im + x as f64 * s)
                    })
            })
            .collect();

        let mut harmonics = Vec::new();
        let mut h = max_harmonic;
        while h >= 1 {
            harmonics.push(h);
            h /= 2;
        }

        let mut levels: Vec<Box<[f32]>> = harmonics
            .iter()
            .map(|&limit| {
                let len = (TABLE_LEN * limit / max_harmonic)
                    .next_power_of_two()
                    .clamp(MIN_TABLE_LEN, TABLE_LEN);
                let wave: Vec<(f64, f64)> = (0..len)
                    .map(|i| {
                        let a = TAU * i as f64 / len as f64;
                        (a.cos(), a.sin())
                    })
                    .collect();
                let mut table: Vec<f32> = (0..len)
                    .map(|i| {
                        coefficients[..limit]
                            .iter()
                            .enumerate()
                            .map(|(k, (re, im))| {
                                let (c, s) = wave[((k + 1) * i) % len];
                                re * c + im * s
                            })
                            .sum::<f64>() as f32
                    })
                    .collect();
                table.push(table[0]);
                table.into_boxed_slice()
            })
            .collect();

        // Normalize every level by the fullest one's peak so switching levels keeps the volume
        let peak = levels[0]
            .iter()
            .fold(0.0f32, |m, x| m.max(x.abs()))
            .max(1e-9);
        for table in &mut levels {
            for x in table.iter_mut() {
                *x /= peak;
            }
        }

        Self { levels, harmonics }
    }

    /// Memory used by the tables, in bytes.
    #[cfg(test)]
    pub fn size_bytes(&self) -> usize {
        self.levels
            .iter()
            .map(|t| t.len() * std::mem::size_of::<f32>())
            .sum()
    }

    /// One sample at phase `t` (0..1) for a phase increment of `dt` per sample.
    #[inline]
    pub fn sample(&self, t: f32, dt: f32) -> f32 {
        // The richest level whose top harmonic stays below Nyquist
        let allowed = MAX_HARMONIC_FRACTION / dt.max(1e-9);
        let level = self
            .harmonics
            .iter()
            .position(|&h| h as f32 <= allowed)
            .unwrap_or(self.harmonics.len() - 1);
        let table = &self.levels[level];
        let len = table.len() - 1;

        let x = t * len as f32;
        let i = (x as usize).min(len - 1);
        let frac = x - i as f32;
        table[i] + (table[i + 1] - table[i]) * frac
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebuilds_a_sine_and_drops_harmonics_for_high_notes() {
        // A cycle with harmonics 1 and 40
        let cycle: Vec<f32> = (0..600)
            .map(|i| {
                let t = i as f32 / 600.0;
                (std::f32::consts::TAU * t).sin() + (std::f32::consts::TAU * 40.0 * t).sin()
            })
            .collect();
        let table = Wavetable::from_cycle(&cycle);

        // Low note: both harmonics survive, so the shape has fast wiggles
        let low: Vec<f32> = (0..64)
            .map(|i| table.sample(i as f32 / 64.0, 1.0 / 4800.0))
            .collect();
        // High note (harmonic 40 would be above Nyquist): only the fundamental remains
        let high: Vec<f32> = (0..64)
            .map(|i| table.sample(i as f32 / 64.0, 1.0 / 40.0))
            .collect();
        let roughness = |v: &[f32]| v.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f32>();
        assert!(roughness(&low) > 2.0 * roughness(&high));
        assert!(high.iter().all(|x| x.abs() <= 1.01));
        // Levels shrink with their harmonics: well under the 9 x 2048 floats a fixed size would use
        assert!(
            table.size_bytes() < 5000 * 4,
            "{} bytes",
            table.size_bytes()
        );
    }
}
