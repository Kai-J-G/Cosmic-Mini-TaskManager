//! Styling helpers using COSMIC theme tokens.

use cosmic::iced::{Border, Color};
use cosmic::theme;

const CARD_RADIUS: f32 = 10.0;
const BADGE_RADIUS: f32 = 5.0;

/// The red used for stopped, zombie, and hung processes.
const ALERT: Color = Color::from_rgba(0.93, 0.27, 0.27, 0.16);
const ALERT_BORDER: Color = Color::from_rgba(0.93, 0.27, 0.27, 0.45);

fn surface(background: Color, border: Color, radius: f32) -> theme::Container<'static> {
    let width = if border.a > 0.0 { 1.0 } else { 0.0 };
    theme::Container::Custom(Box::new(move |_theme| {
        cosmic::iced::widget::container::Style {
            background: Some(background.into()),
            border: Border {
                color: border,
                width,
                radius: radius.into(),
            },
            ..Default::default()
        }
    }))
}

/// Container for warnings and for rows holding a stopped or hung process.
pub fn alert_container() -> theme::Container<'static> {
    surface(ALERT, ALERT_BORDER, CARD_RADIUS)
}

/// A pill badge, used for process status.
pub fn badge_container(background: Color) -> theme::Container<'static> {
    surface(background, Color::TRANSPARENT, BADGE_RADIUS)
}
