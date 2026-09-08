//! High-visibility alert banner shown when stopped or unresponsive processes are detected.

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, container, icon, row, space, text};
use cosmic::Element;

use crate::app::{AppModel, Message};
use crate::process::FilterTab;
use crate::views::style::alert_container;

pub fn view(app: &AppModel) -> Option<Element<'_, Message>> {
    let count = app.overview.unresponsive_or_stopped_count;
    if count == 0 {
        return None;
    }

    let sp = cosmic::theme::spacing();
    let is_dark = app.is_dark();

    let warn_icon = icon::from_name("dialog-warning-symbolic")
        .size(20)
        .symbolic(true)
        .icon();

    let msg_text = text::body(crate::fl!("alert-unresponsive", count = count));

    let view_btn = button::text(crate::fl!("btn-inspect"))
        .on_press(Message::SelectTab(FilterTab::Unresponsive))
        .class(cosmic::theme::Button::Text)
        .padding([5, 12]);

    let kill_all_content = row![
        icon::from_name("process-stop-symbolic").size(14).symbolic(true),
        text::body(crate::fl!("btn-kill-all")),
    ]
    .spacing(sp.space_xxs)
    .align_y(Alignment::Center);

    let kill_all_btn = button::custom(kill_all_content)
        .on_press(Message::KillAllUnresponsive)
        .class(cosmic::theme::Button::Destructive)
        .padding([5, 14]);

    let banner = container(
        row![
            warn_icon,
            msg_text,
            space::horizontal().width(Length::Fill),
            view_btn,
            kill_all_btn,
        ]
        .spacing(sp.space_xs)
        .align_y(Alignment::Center)
        .padding([sp.space_xs, sp.space_s]),
    )
    .class(alert_container(is_dark))
    .width(Length::Fill);

    Some(banner.into())
}
