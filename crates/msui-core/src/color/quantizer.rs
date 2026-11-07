//! Fast image color quantization and dominant accent extraction.

use super::hct::{Hct, Srgb};
use std::collections::HashMap;

/// A candidate dominant color identified by quantization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuantizedColor {
    pub color: Srgb,
    pub hct: Hct,
    pub count: usize,
    pub score: f64,
}

pub struct Quantizer;

impl Quantizer {
    /// Extracts the best dominant accent color from a list of sRGB pixels.
    pub fn find_dominant_accent(pixels: &[Srgb]) -> Hct {
        let candidates = Self::quantize(pixels);
        if let Some(best) = candidates.first() {
            best.hct
        } else {
            // Default to Material Violet if image has no usable colors
            Hct::new(270.0, 48.0, 50.0)
        }
    }

    /// Quantizes pixel samples into scored color clusters.
    pub fn quantize(pixels: &[Srgb]) -> Vec<QuantizedColor> {
        if pixels.is_empty() {
            return Vec::new();
        }

        // Subsample if image sample is too large (> 8192 pixels)
        let step = (pixels.len() / 4096).max(1);
        let sampled = pixels.iter().step_by(step);

        // Quantize colors into 4-bit per channel bins (16 * 16 * 16 = 4096 bins)
        let mut bins: HashMap<u16, (usize, u64, u64, u64)> = HashMap::new();

        for &px in sampled {
            let r_bin = (px.r >> 4) as u16;
            let g_bin = (px.g >> 4) as u16;
            let b_bin = (px.b >> 4) as u16;
            let key = (r_bin << 8) | (g_bin << 4) | b_bin;

            let entry = bins.entry(key).or_insert((0, 0, 0, 0));
            entry.0 += 1;
            entry.1 += px.r as u64;
            entry.2 += px.g as u64;
            entry.3 += px.b as u64;
        }

        let mut clusters: Vec<QuantizedColor> = Vec::new();

        for (_key, (count, r_sum, g_sum, b_sum)) in bins {
            if count == 0 {
                continue;
            }
            let avg_r = (r_sum / count as u64) as u8;
            let avg_g = (g_sum / count as u64) as u8;
            let avg_b = (b_sum / count as u64) as u8;

            let srgb = Srgb::new(avg_r, avg_g, avg_b);
            let hct = Hct::from_srgb(srgb);

            // Filter out near-pure-black, near-pure-white, and muddy grays
            let is_too_dark = hct.tone < 12.0;
            let is_too_bright = hct.tone > 92.0;
            let is_gray = hct.chroma < 12.0;

            if is_too_dark || is_too_bright || is_gray {
                continue;
            }

            // Material ranking formula: Score = sqrt(Count) * (Chroma + 12.0)
            let score = (count as f64).sqrt() * (hct.chroma + 12.0);

            clusters.push(QuantizedColor {
                color: srgb,
                hct,
                count,
                score,
            });
        }

        // If all pixels were filtered out (e.g. grayscale/monochrome wallpaper)
        if clusters.is_empty() {
            // Fallback: take average color regardless of chroma
            let mut r_tot: u64 = 0;
            let mut g_tot: u64 = 0;
            let mut b_tot: u64 = 0;
            let n = pixels.len().max(1) as u64;
            for p in pixels {
                r_tot += p.r as u64;
                g_tot += p.g as u64;
                b_tot += p.b as u64;
            }
            let avg_srgb = Srgb::new((r_tot / n) as u8, (g_tot / n) as u8, (b_tot / n) as u8);
            let hct = Hct::from_srgb(avg_srgb);
            return vec![QuantizedColor {
                color: avg_srgb,
                hct,
                count: pixels.len(),
                score: 1.0,
            }];
        }

        // Sort descending by score
        clusters.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        clusters
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantizer_picks_vibrant_accent() {
        // Sample with a large dark background and a vibrant cyan square
        let mut pixels = vec![Srgb::new(10, 10, 15); 1000];
        let cyan = Srgb::new(0, 200, 255);
        pixels.extend(vec![cyan; 200]);

        let best = Quantizer::find_dominant_accent(&pixels);
        // The cyan hue is approximately 190-210 deg
        assert!(best.chroma > 20.0);
    }
}
