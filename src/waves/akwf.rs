//! A curated set of single-cycle waveforms from the Adventure Kid Waveforms collection (AKWF) by
//! Kristoffer Ekstrand, <https://github.com/KristofferKarlAxelEkstrand/AKWF-FREE>.
//!
//! AKWF is released under CC0 1.0 (public domain); see `assets/waves/akwf/LICENSE.md`. No
//! attribution is required, but it's credited here and in docs/third-party.md because it's deserved.
//!
//! To add a wave: drop the `.wav` into `assets/waves/akwf/` and add a line below. Only append,
//! since hosts save the waveform by its position in the catalog.

/// (stable id, display name, 16-bit mono WAV file)
pub const WAVES: &[(&str, &str, &[u8])] = &[
    (
        "akwf_altosax",
        "Alto Sax",
        include_bytes!("../../assets/waves/akwf/AKWF_altosax_0001.wav"),
    ),
    (
        "akwf_bitreduced",
        "Bit Reduced",
        include_bytes!("../../assets/waves/akwf/AKWF_bitreduced_0001.wav"),
    ),
    (
        "akwf_cello",
        "Cello",
        include_bytes!("../../assets/waves/akwf/AKWF_cello_0001.wav"),
    ),
    (
        "akwf_clarinet",
        "Clarinet",
        include_bytes!("../../assets/waves/akwf/AKWF_clarinett_0001.wav"),
    ),
    (
        "akwf_oboe",
        "Oboe",
        include_bytes!("../../assets/waves/akwf/AKWF_oboe_0001.wav"),
    ),
    (
        "akwf_distorted",
        "Distorted",
        include_bytes!("../../assets/waves/akwf/AKWF_distorted_0001.wav"),
    ),
    (
        "akwf_ebass",
        "E-Bass",
        include_bytes!("../../assets/waves/akwf/AKWF_ebass_0001.wav"),
    ),
    (
        "akwf_dbass",
        "Double Bass",
        include_bytes!("../../assets/waves/akwf/AKWF_dbass_0001.wav"),
    ),
    (
        "akwf_eguitar",
        "Lead Guitar",
        include_bytes!("../../assets/waves/akwf/AKWF_eguitar_0001.wav"),
    ),
    (
        "akwf_flute",
        "Flute",
        include_bytes!("../../assets/waves/akwf/AKWF_flute_0001.wav"),
    ),
    (
        "akwf_fmsynth",
        "FM Synth",
        include_bytes!("../../assets/waves/akwf/AKWF_fmsynth_0001.wav"),
    ),
    (
        "akwf_voice",
        "Voice",
        include_bytes!("../../assets/waves/akwf/AKWF_hvoice_0001.wav"),
    ),
    (
        "akwf_chip",
        "Chip",
        include_bytes!("../../assets/waves/akwf/AKWF_oscchip_0001.wav"),
    ),
    (
        "akwf_raw",
        "Raw Synth",
        include_bytes!("../../assets/waves/akwf/AKWF_raw_0001.wav"),
    ),
    (
        "akwf_tannerin",
        "Tannerin",
        include_bytes!("../../assets/waves/akwf/AKWF_tannerin_0001.wav"),
    ),
    (
        "akwf_violin",
        "Violin",
        include_bytes!("../../assets/waves/akwf/AKWF_violin_0001.wav"),
    ),
];

/// Reads the samples of a 16-bit PCM WAV file (mono, or the first channel otherwise).
pub fn decode_wav(bytes: &[u8]) -> Result<Vec<f32>, String> {
    let u16_at = |i: usize| {
        bytes
            .get(i..i + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
    };
    let u32_at = |i: usize| {
        bytes
            .get(i..i + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    if bytes.get(0..4) != Some(b"RIFF") || bytes.get(8..12) != Some(b"WAVE") {
        return Err("not a WAV file".into());
    }

    let (mut channels, mut bits) = (0u16, 0u16);
    let mut pos = 12;
    while let (Some(id), Some(size)) = (bytes.get(pos..pos + 4), u32_at(pos + 4)) {
        let body = pos + 8;
        let size = size as usize;
        match id {
            b"fmt " => {
                if u16_at(body) != Some(1) {
                    return Err("only uncompressed PCM is supported".into());
                }
                channels = u16_at(body + 2).unwrap_or(0);
                bits = u16_at(body + 14).unwrap_or(0);
            }
            b"data" => {
                if bits != 16 || channels == 0 {
                    return Err(format!(
                        "expected 16-bit PCM, got {bits}-bit, {channels} ch"
                    ));
                }
                let data = bytes.get(body..body + size).ok_or("truncated data chunk")?;
                let frame = 2 * channels as usize;
                return Ok(data
                    .chunks_exact(frame)
                    .map(|f| i16::from_le_bytes([f[0], f[1]]) as f32 / 32768.0)
                    .collect());
            }
            _ => {}
        }
        // Chunks are padded to an even size
        pos = body + size + (size & 1);
    }
    Err("no data chunk".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_waves_decode_to_one_cycle() {
        for (id, _, bytes) in WAVES {
            let samples = decode_wav(bytes).unwrap_or_else(|e| panic!("{id}: {e}"));
            assert_eq!(samples.len(), 600, "{id}");
            assert!(samples.iter().any(|x| x.abs() > 0.1), "{id} is silent");
        }
    }
}
