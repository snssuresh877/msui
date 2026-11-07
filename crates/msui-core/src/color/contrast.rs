//! WCAG 2.1 contrast calculation and verification utilities.

use super::hct::Srgb;

/// Calculates WCAG relative luminance of an sRGB color (0.0 = black, 1.0 = white).
pub fn relative_luminance(color: Srgb) -> f64 {
    let (r_l, g_l, b_l) = color.to_linear();
    0.2126 * r_l + 0.7152 * g_l + 0.0722 * b_l
}

/// Calculates the WCAG contrast ratio between two colors (ranging from 1.0 to 21.0).
pub fn contrast_ratio(color_a: Srgb, color_b: Srgb) -> f64 {
    let lum_a = relative_luminance(color_a);
    let lum_b = relative_luminance(color_b);

    let lighter = lum_a.max(lum_b);
    let darker = lum_a.min(lum_b);

    (lighter + 0.05) / (darker + 0.05)
}

/// Checks if two colors satisfy WCAG AA body text contrast (minimum 4.5:1).
pub fn satisfies_wcag_aa(foreground: Srgb, background: Srgb) -> bool {
    contrast_ratio(foreground, background) >= 4.5
}

/// Checks if two colors satisfy WCAG AA UI elements / bold text contrast (minimum 3.0:1).
pub fn satisfies_wcag_ui(foreground: Srgb, background: Srgb) -> bool {
    contrast_ratio(foreground, background) >= 3.0
}

/// Checks if two colors satisfy WCAG AAA high contrast (minimum 7.0:1).
pub fn satisfies_wcag_aaa(foreground: Srgb, background: Srgb) -> bool {
    contrast_ratio(foreground, background) >= 7.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_black_and_white_contrast() {
        let black = Srgb::new(0, 0, 0);
        let white = Srgb::new(255, 255, 255);
        let ratio = contrast_ratio(black, white);
        assert!((ratio - 21.0).abs() < 0.1);
        assert!(satisfies_wcag_aaa(black, white));
    }

    #[test]
    fn test_identical_color_contrast() {
        let gray = Srgb::new(128, 128, 128);
        let ratio = contrast_ratio(gray, gray);
        assert!((ratio - 1.0).abs() < 0.01);
    }
}
