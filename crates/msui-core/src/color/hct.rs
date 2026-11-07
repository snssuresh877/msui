//! Material Design 3 HCT (Hue, Chroma, Tone) / LCH color space implementation in pure Rust.
//!
//! - **Hue ($H \in [0, 360)$)**: Perceptual hue angle on the color wheel.
//! - **Chroma ($C \ge 0$)**: Perceptual saturation / colorfulness ($0 = \text{gray}$, $48+ = \text{vibrant}$).
//! - **Tone ($T \in [0, 100]$)**: CIELAB perceptual luminance ($L^*$), guaranteeing accessible contrast.

use std::f64::consts::PI;

// D65 reference white points (normalized to Y = 100)
const X_N: f64 = 95.047;
const Y_N: f64 = 100.000;
const Z_N: f64 = 108.883;

const EPSILON: f64 = 216.0 / 24389.0; // ~0.008856
const KAPPA: f64 = 24389.0 / 27.0;     // ~903.3

/// Represents an exact color in the HCT (Hue, Chroma, Tone) color space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Hct {
    pub hue: f64,
    pub chroma: f64,
    pub tone: f64,
}

/// Standard 8-bit sRGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Srgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Srgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        let clean = hex.trim().trim_start_matches('#');
        if clean.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&clean[0..2], 16).ok()?;
        let g = u8::from_str_radix(&clean[2..4], 16).ok()?;
        let b = u8::from_str_radix(&clean[4..6], 16).ok()?;
        Some(Self::new(r, g, b))
    }

    /// Converts sRGB to CIELAB Tone ($L^*$, range 0.0..=100.0).
    pub fn to_tone(&self) -> f64 {
        let y = self.to_xyz().1;
        y_to_tone(y)
    }

    /// Converts sRGB [0..255] to normalized linear sRGB [0.0..1.0].
    pub fn to_linear(&self) -> (f64, f64, f64) {
        (
            srgb_to_linear(self.r as f64 / 255.0),
            srgb_to_linear(self.g as f64 / 255.0),
            srgb_to_linear(self.b as f64 / 255.0),
        )
    }

    /// Converts sRGB to CIE-1931 XYZ (D65 standard illuminant, Y in 0..100).
    pub fn to_xyz(&self) -> (f64, f64, f64) {
        let (r_l, g_l, b_l) = self.to_linear();
        // sRGB to XYZ (D65) matrix
        let x = 100.0 * (0.4124564 * r_l + 0.3575761 * g_l + 0.1804375 * b_l);
        let y = 100.0 * (0.2126729 * r_l + 0.7151522 * g_l + 0.0721750 * b_l);
        let z = 100.0 * (0.0193339 * r_l + 0.1191920 * g_l + 0.9503041 * b_l);
        (x, y, z)
    }

    pub fn from_linear(r_lin: f64, g_lin: f64, b_lin: f64) -> Self {
        let r = (linear_to_srgb(r_lin.clamp(0.0, 1.0)) * 255.0).round() as u8;
        let g = (linear_to_srgb(g_lin.clamp(0.0, 1.0)) * 255.0).round() as u8;
        let b = (linear_to_srgb(b_lin.clamp(0.0, 1.0)) * 255.0).round() as u8;
        Self { r, g, b }
    }

    pub fn from_xyz(x: f64, y: f64, z: f64) -> Self {
        let x_n = x / 100.0;
        let y_n = y / 100.0;
        let z_n = z / 100.0;
        // XYZ to sRGB matrix
        let r_l = 3.2404542 * x_n - 1.5371385 * y_n - 0.4985314 * z_n;
        let g_l = -0.9692660 * x_n + 1.8760108 * y_n + 0.0415560 * z_n;
        let b_l = 0.0556434 * x_n - 0.2040259 * y_n + 1.0572252 * z_n;
        Self::from_linear(r_l, g_l, b_l)
    }
}

impl Hct {
    pub fn new(hue: f64, chroma: f64, tone: f64) -> Self {
        let h = ((hue % 360.0) + 360.0) % 360.0;
        let c = chroma.max(0.0);
        let t = tone.clamp(0.0, 100.0);
        Self { hue: h, chroma: c, tone: t }
    }

    pub fn from_srgb(srgb: Srgb) -> Self {
        let (x, y, z) = srgb.to_xyz();
        let (hue, chroma, tone) = xyz_to_lch(x, y, z);
        Self { hue, chroma, tone }
    }

    pub fn from_hex(hex: &str) -> Option<Self> {
        Srgb::from_hex(hex).map(Self::from_srgb)
    }

    /// Converts this HCT color to an sRGB color.
    ///
    /// If the requested (hue, chroma, tone) point is out of the sRGB gamut,
    /// a binary search finds the maximum chroma at that hue and tone that fits within sRGB.
    pub fn to_srgb(&self) -> Srgb {
        if self.tone <= 0.0001 {
            return Srgb::new(0, 0, 0);
        }
        if self.tone >= 99.9999 {
            return Srgb::new(255, 255, 255);
        }

        // Fast check if requested chroma directly fits in gamut
        if let Some(rgb) = lch_to_srgb_bounded(self.hue, self.chroma, self.tone) {
            return rgb;
        }

        // Binary search for maximum achievable chroma that fits in sRGB
        let mut low = 0.0;
        let mut high = self.chroma;
        let mut best_rgb = lch_to_srgb_bounded(self.hue, 0.0, self.tone).unwrap_or(Srgb::new(128, 128, 128));

        for _ in 0..12 {
            let mid = (low + high) * 0.5;
            if let Some(rgb) = lch_to_srgb_bounded(self.hue, mid, self.tone) {
                best_rgb = rgb;
                low = mid;
            } else {
                high = mid;
            }
        }

        best_rgb
    }

    pub fn to_hex(&self) -> String {
        self.to_srgb().to_hex()
    }
}

// --- Gamma / Linear math ---

fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f64) -> f64 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

// --- Tone <-> Y luminance math (CIELAB) ---

pub fn tone_to_y(tone: f64) -> f64 {
    if tone > 8.0 {
        Y_N * ((tone + 16.0) / 116.0).powi(3)
    } else {
        Y_N * (tone / KAPPA)
    }
}

pub fn y_to_tone(y: f64) -> f64 {
    let y_r = y / Y_N;
    if y_r > EPSILON {
        116.0 * y_r.cbrt() - 16.0
    } else {
        KAPPA * y_r
    }
}

// --- CIELAB / LCH Conversion ---

fn lab_f(t: f64) -> f64 {
    if t > EPSILON {
        t.cbrt()
    } else {
        (KAPPA * t + 16.0) / 116.0
    }
}

fn lab_inv_f(t: f64) -> f64 {
    let t3 = t.powi(3);
    if t3 > EPSILON {
        t3
    } else {
        (116.0 * t - 16.0) / KAPPA
    }
}

fn xyz_to_lch(x: f64, y: f64, z: f64) -> (f64, f64, f64) {
    let fx = lab_f(x / X_N);
    let fy = lab_f(y / Y_N);
    let fz = lab_f(z / Z_N);

    let l = (116.0 * fy - 16.0).clamp(0.0, 100.0);
    let a = 500.0 * (fx - fy);
    let b = 200.0 * (fy - fz);

    let chroma = (a * a + b * b).sqrt();
    let mut hue = b.atan2(a) * 180.0 / PI;
    if hue < 0.0 {
        hue += 360.0;
    }

    (hue, chroma, l)
}

fn lch_to_srgb_bounded(hue: f64, chroma: f64, tone: f64) -> Option<Srgb> {
    let rad = hue * PI / 180.0;
    let a = chroma * rad.cos();
    let b = chroma * rad.sin();

    let fy = (tone + 16.0) / 116.0;
    let fx = a / 500.0 + fy;
    let fz = fy - b / 200.0;

    let x = X_N * lab_inv_f(fx);
    let y = Y_N * lab_inv_f(fy);
    let z = Z_N * lab_inv_f(fz);

    let x_n = x / 100.0;
    let y_n = y / 100.0;
    let z_n = z / 100.0;

    let r_l = 3.2404542 * x_n - 1.5371385 * y_n - 0.4985314 * z_n;
    let g_l = -0.9692660 * x_n + 1.8760108 * y_n + 0.0415560 * z_n;
    let b_l = 0.0556434 * x_n - 0.2040259 * y_n + 1.0572252 * z_n;

    let eps = 0.001;
    if r_l < -eps || r_l > 1.0 + eps || g_l < -eps || g_l > 1.0 + eps || b_l < -eps || b_l > 1.0 + eps {
        return None;
    }

    Some(Srgb::from_linear(r_l, g_l, b_l))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srgb_hex_roundtrip() {
        let hex = "#9333ea";
        let rgb = Srgb::from_hex(hex).unwrap();
        assert_eq!(rgb.to_hex(), hex);
    }

    #[test]
    fn test_tone_boundaries() {
        assert_eq!(y_to_tone(0.0), 0.0);
        let tone_100 = y_to_tone(100.0);
        assert!((tone_100 - 100.0).abs() < 0.1);

        let black = Hct::new(120.0, 50.0, 0.0).to_srgb();
        assert_eq!(black, Srgb::new(0, 0, 0));

        let white = Hct::new(120.0, 50.0, 100.0).to_srgb();
        assert_eq!(white, Srgb::new(255, 255, 255));
    }

    #[test]
    fn test_hct_tone_monotonic() {
        let c1 = Hct::new(270.0, 48.0, 20.0).to_srgb();
        let c2 = Hct::new(270.0, 48.0, 50.0).to_srgb();
        let c3 = Hct::new(270.0, 48.0, 80.0).to_srgb();

        assert!(c1.to_tone() < c2.to_tone());
        assert!(c2.to_tone() < c3.to_tone());
    }
}
