use iced::widget::container;
use iced::{Border, Color, Theme};
use msui_core::MsuiConfig;

/// Check if the bar configuration currently has frosted glass enabled.
pub fn is_frosted() -> bool {
    MsuiConfig::load_or_default().bar.is_frosted()
}

/// Mac-style smoked acrylic glass outer modal container.
/// When `is_frosted` is true:
/// - Smoked dark acrylic background: `rgba(16, 18, 26, 0.72)`
/// - Hairline white border highlight: `rgba(255, 255, 255, 0.14)`
/// - Generous rounded corners: `radius: 18.0`
/// When `is_frosted` is false (solid):
/// - Solid theme background with subtle border.
pub fn modal_card_style(is_frosted: bool) -> impl Fn(&Theme) -> container::Style {
    move |theme: &Theme| {
        let p = theme.palette();
        if is_frosted {
            container::Style {
                background: Some(Color::from_rgba(16.0 / 255.0, 18.0 / 255.0, 26.0 / 255.0, 0.72).into()),
                border: Border {
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.14),
                    width: 1.0,
                    radius: 18.0.into(),
                },
                ..Default::default()
            }
        } else {
            container::Style {
                background: Some(p.background.into()),
                border: Border {
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.12),
                    width: 1.0,
                    radius: 18.0.into(),
                },
                ..Default::default()
            }
        }
    }
}

/// Convenience style using config from disk.
pub fn default_modal_card_style() -> impl Fn(&Theme) -> container::Style {
    modal_card_style(is_frosted())
}

/// Inner subcard / section style for cards within a modal (e.g. sliders, telemetry, settings cards).
/// Floating with translucent pill styling inside the glass container.
pub fn subcard_style(is_frosted: bool) -> impl Fn(&Theme) -> container::Style {
    move |theme: &Theme| {
        let p = theme.palette();
        if is_frosted {
            container::Style {
                background: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.05).into()),
                border: Border {
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                    width: 1.0,
                    radius: 12.0.into(),
                },
                ..Default::default()
            }
        } else {
            container::Style {
                background: Some(Color::from_rgba(p.text.r, p.text.g, p.text.b, 0.04).into()),
                border: Border {
                    color: Color::from_rgba(p.text.r, p.text.g, p.text.b, 0.10),
                    width: 1.0,
                    radius: 12.0.into(),
                },
                ..Default::default()
            }
        }
    }
}

/// Convenience subcard style using config from disk.
pub fn default_subcard_style() -> impl Fn(&Theme) -> container::Style {
    subcard_style(is_frosted())
}
