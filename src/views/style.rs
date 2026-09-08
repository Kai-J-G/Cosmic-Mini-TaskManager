//! Styling helpers using COSMIC theme tokens.

use cosmic::iced::{Border, Color};
use cosmic::theme;

const CARD_RADIUS: f32 = 10.0;

fn surface(background: Color, border: Color, radius: f32) -> theme::Container<'static> {
    let width = if border.a > 0.0 { 1.0 } else { 0.0 };
    theme::Container::Custom(Box::new(move |_theme| cosmic::iced::widget::container::Style {
        background: Some(background.into()),
        border: Border {
            color: border,
            width,
            radius: radius.into(),
        },
        ..Default::default()
    }))
}

/// An alert container for warnings or stopped/hung processes.
pub fn alert_container(is_dark: bool) -> theme::Container<'static> {
    let bg = if is_dark {
        Color::from_rgba(0.93, 0.27, 0.27, 0.16)
    } else {
        Color::from_rgba(0.93, 0.27, 0.27, 0.12)
    };
    let border = Color::from_rgba(0.93, 0.27, 0.27, 0.45);
    surface(bg, border, CARD_RADIUS)
}

/// A pill badge container style.
pub fn badge_container(bg_color: Color) -> theme::Container<'static> {
    surface(bg_color, Color::TRANSPARENT, 5.0)
}
