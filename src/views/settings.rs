//! Settings view for configuring refresh interval, theme preference, and alerts.

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, column, container, row, space, text, toggler};
use cosmic::Element;

use crate::app::{AppModel, Message};
use crate::config::ThemePreference;

pub fn view(app: &AppModel) -> Element<'_, Message> {
    let sp = cosmic::theme::spacing();
    let title = text::title3(crate::fl!("settings-title"));

    // Theme selector
    let theme_label = text::body(crate::fl!("settings-theme"));
    let theme_buttons = row![
        button::text(crate::fl!("theme-system"))
            .on_press(Message::SetTheme(ThemePreference::System))
            .class(if app.config.theme_pref == ThemePreference::System {
                cosmic::theme::Button::Suggested
            } else {
                cosmic::theme::Button::Text
            }),
        button::text(crate::fl!("theme-dark"))
            .on_press(Message::SetTheme(ThemePreference::Dark))
            .class(if app.config.theme_pref == ThemePreference::Dark {
                cosmic::theme::Button::Suggested
            } else {
                cosmic::theme::Button::Text
            }),
        button::text(crate::fl!("theme-light"))
            .on_press(Message::SetTheme(ThemePreference::Light))
            .class(if app.config.theme_pref == ThemePreference::Light {
                cosmic::theme::Button::Suggested
            } else {
                cosmic::theme::Button::Text
            }),
    ]
    .spacing(sp.space_xs)
    .align_y(Alignment::Center);

    let theme_section = column![theme_label, theme_buttons].spacing(sp.space_xxs);

    // Refresh rate selector
    let refresh_label = text::body(crate::fl!("settings-interval"));
    let intervals = [1, 2, 5];
    let mut interval_buttons = Vec::new();
    for sec in intervals {
        let is_sel = app.config.refresh_interval_secs == sec;
        let btn = button::text(format!("{}s", sec))
            .on_press(Message::SetInterval(sec))
            .class(if is_sel {
                cosmic::theme::Button::Suggested
            } else {
                cosmic::theme::Button::Text
            });
        interval_buttons.push(btn.into());
    }
    let interval_row = row(interval_buttons).spacing(sp.space_xs).align_y(Alignment::Center);
    let refresh_section = column![refresh_label, interval_row].spacing(sp.space_xxs);

    // Panel Alert Warning with proper spacing (label on left, toggle on right)
    let warn_label_col = column![
        text::body(crate::fl!("settings-warn-title")),
        text::caption(crate::fl!("settings-warn-desc")),
    ]
    .spacing(2);

    let warn_toggle_btn = toggler(app.config.warn_unresponsive_in_panel)
        .on_toggle(Message::ToggleWarnInPanel);

    let warn_row = row![
        warn_label_col,
        space::horizontal().width(Length::Fill),
        warn_toggle_btn,
    ]
    .align_y(Alignment::Center)
    .width(Length::Fill);

    // Back to task manager button
    let back_btn = button::text(crate::fl!("settings-done"))
        .on_press(Message::ToggleSettings)
        .class(cosmic::theme::Button::Suggested);

    let content = column![
        title,
        theme_section,
        refresh_section,
        warn_row,
        space::vertical().height(sp.space_s),
        back_btn,
    ]
    .spacing(sp.space_m)
    .padding(sp.space_m)
    .width(Length::Fill);

    container(content)
        .class(cosmic::theme::Container::Card)
        .width(Length::Fill)
        .into()
}
