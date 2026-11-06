use iced::widget::container;
use iced::{Border, Color, Theme};
use msui_core::{parse_hex_color, GuiConfig, MsuiConfig};

/// Check if the bar configuration currently has frosted glass enabled.
pub fn is_frosted() -> bool {
    MsuiConfig::load_or_default().bar.is_frosted()
}

/// Helper to convert a hex color string and alpha into an iced::Color, with fallback.
pub fn hex_to_rgba(hex: &str, alpha: f32, fallback: (f32, f32, f32)) -> Color {
    if let Some((r, g, b)) = parse_hex_color(hex) {
        Color::from_rgba(r, g, b, alpha)
    } else {
        Color::from_rgba(fallback.0, fallback.1, fallback.2, alpha)
    }
}

/// Universal outer modal container styling rule for all MSUI tools.
/// Loads the universal `GuiConfig` from `MsuiConfig::load_or_default()`:
/// - Background color parsed from `gui.background` with `gui.bg_opacity`
/// - Border:
///   - If `gui.border_enabled` is false or `gui.border_width == 0.0`: BORDER OFF (width 0.0, transparent)
///   - If `gui.border_enabled` is true: accent color `gui.border_color` with `gui.border_opacity`, width `gui.border_width`
/// - Corner radius from `gui.corner_radius`
pub fn modal_card_style(is_frosted: bool) -> impl Fn(&Theme) -> container::Style {
    modal_card_style_with_gui(MsuiConfig::load_or_default().gui, is_frosted)
}

/// Core styling builder given an explicit `GuiConfig`.
pub fn modal_card_style_with_gui(gui: GuiConfig, is_frosted: bool) -> impl Fn(&Theme) -> container::Style {
    let alpha = if is_frosted { gui.bg_opacity } else { (gui.bg_opacity + 0.04).min(1.0) };
    let bg_color = hex_to_rgba(&gui.background, alpha, (18.0 / 255.0, 18.0 / 255.0, 24.0 / 255.0));

    let border = if gui.border_enabled && gui.border_width > 0.0 {
        Border {
            color: hex_to_rgba(&gui.border_color, gui.border_opacity, (147.0 / 255.0, 51.0 / 255.0, 234.0 / 255.0)),
            width: gui.border_width,
            radius: gui.corner_radius.into(),
        }
    } else {
        Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: gui.corner_radius.into(),
        }
    };

    move |_theme: &Theme| container::Style {
        background: Some(bg_color.into()),
        border,
        ..Default::default()
    }
}

/// Convenience style using config from disk.
pub fn default_modal_card_style() -> impl Fn(&Theme) -> container::Style {
    modal_card_style(is_frosted())
}

/// Inner subcard / section style for cards within a modal (e.g. sliders, telemetry, settings cards).
pub fn subcard_style(is_frosted: bool) -> impl Fn(&Theme) -> container::Style {
    subcard_style_with_gui(MsuiConfig::load_or_default().gui, is_frosted)
}

/// Inner subcard style given an explicit `GuiConfig`.
pub fn subcard_style_with_gui(gui: GuiConfig, is_frosted: bool) -> impl Fn(&Theme) -> container::Style {
    let border_enabled = gui.subcard_border_enabled;
    let opacity = gui.subcard_opacity;

    move |theme: &Theme| {
        let p = theme.palette();
        let border = if border_enabled {
            Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                width: 1.0,
                radius: 12.0.into(),
            }
        } else {
            Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: 12.0.into(),
            }
        };

        if is_frosted {
            container::Style {
                background: Some(Color::from_rgba(1.0, 1.0, 1.0, opacity).into()),
                border,
                ..Default::default()
            }
        } else {
            container::Style {
                background: Some(Color::from_rgba(p.text.r, p.text.g, p.text.b, opacity.min(0.08)).into()),
                border,
                ..Default::default()
            }
        }
    }
}

/// Convenience subcard style using config from disk.
pub fn default_subcard_style() -> impl Fn(&Theme) -> container::Style {
    subcard_style(is_frosted())
}
