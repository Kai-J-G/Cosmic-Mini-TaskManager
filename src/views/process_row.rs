//! A single process row: identity, metrics, status badge, and actions.

use cosmic::Element;
use cosmic::iced::alignment::Horizontal;
use cosmic::iced::{Alignment, Color, Length};
use cosmic::widget::{button, column, container, icon, row, text};

use crate::app::Message;
use crate::process::{ProcessItem, ProcessState};
use crate::views::style::{alert_container, badge_container};

/// Widths that keep the columns lined up down the list, since each row is an
/// independent container rather than part of a real table.
const METRICS_WIDTH: f32 = 90.0;
const STATUS_WIDTH: f32 = 60.0;
const ACTIONS_WIDTH: f32 = 125.0;

/// How much of the command line to show before eliding it.
const CMD_CHARS: usize = 36;

pub fn view(item: &ProcessItem) -> Element<'_, Message> {
    let sp = cosmic::theme::spacing();

    let proc_icon = icon::from_name(&*item.icon_name)
        .size(24)
        .symbolic(false)
        .icon();

    let title_line = row![
        text::body(&item.name),
        text::caption(format!("({})", item.pid))
    ]
    .spacing(sp.space_xxs)
    .align_y(Alignment::Center);

    let cmd = if item.cmd.is_empty() {
        &item.name
    } else {
        &item.cmd
    };

    let info_col = column![title_line, text::caption(elide(cmd, CMD_CHARS))]
        .spacing(2)
        .width(Length::Fill);

    let metrics_col = column![
        text::caption(format!("CPU {:.1}%", item.cpu_usage)),
        text::caption(item.memory_formatted()),
    ]
    .spacing(2)
    .width(Length::Fixed(METRICS_WIDTH));

    let status_badge = container(status_badge(item.status))
        .width(Length::Fixed(STATUS_WIDTH))
        .center_x(Length::Fixed(STATUS_WIDTH));

    let actions = container(action_buttons(item))
        .width(Length::Fixed(ACTIONS_WIDTH))
        .align_x(Horizontal::Right);

    let row_content = row![proc_icon, info_col, metrics_col, status_badge, actions]
        .spacing(sp.space_xs)
        .align_y(Alignment::Center)
        .padding([sp.space_xxs, sp.space_xs]);

    let class = if item.is_unresponsive_or_stopped() {
        alert_container()
    } else {
        cosmic::theme::Container::Card
    };

    container(row_content)
        .class(class)
        .width(Length::Fill)
        .into()
}

/// Which actions make sense depends on the state: a stopped process can be
/// resumed, and a zombie or hung one cannot usefully be stopped again.
fn action_buttons(item: &ProcessItem) -> Element<'static, Message> {
    let sp = cosmic::theme::spacing();

    let kill = button::text(crate::fl!("btn-kill"))
        .on_press(Message::KillProcess(item.pid))
        .class(cosmic::theme::Button::Destructive)
        .padding([3, 8]);

    match item.status {
        ProcessState::Stopped => {
            let resume = button::text(crate::fl!("btn-resume"))
                .on_press(Message::ResumeProcess(item.pid))
                .class(cosmic::theme::Button::Suggested)
                .padding([3, 8]);
            row![resume, kill]
                .spacing(sp.space_xxs)
                .align_y(Alignment::Center)
                .into()
        }
        ProcessState::Zombie | ProcessState::DiskSleep => row![kill.padding([3, 10])]
            .align_y(Alignment::Center)
            .into(),
        _ => {
            let stop = button::text(crate::fl!("btn-stop"))
                .on_press(Message::StopProcess(item.pid))
                .class(cosmic::theme::Button::Text)
                .padding([3, 8]);
            row![stop, kill]
                .spacing(sp.space_xxs)
                .align_y(Alignment::Center)
                .into()
        }
    }
}

fn status_badge(status: ProcessState) -> Element<'static, Message> {
    let background = match status {
        ProcessState::Running => Color::from_rgba(0.06, 0.72, 0.51, 0.18),
        ProcessState::Stopped => Color::from_rgba(0.96, 0.62, 0.04, 0.28),
        ProcessState::Zombie => Color::from_rgba(0.66, 0.33, 0.98, 0.28),
        ProcessState::DiskSleep => Color::from_rgba(0.93, 0.27, 0.27, 0.28),
        ProcessState::Sleeping | ProcessState::Other => Color::from_rgba(0.5, 0.5, 0.5, 0.12),
    };

    container(text::caption(status.label()).size(10))
        .padding([2, 6])
        .class(badge_container(background))
        .into()
}

/// Shortens `text` to `max` characters, counting characters rather than bytes:
/// slicing on a byte offset would panic mid-codepoint on a non-ASCII path.
fn elide(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((byte, _)) => format!("{}…", &text[..byte]),
        None => text.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_text_is_untouched() {
        assert_eq!(elide("/usr/bin/sleep", CMD_CHARS), "/usr/bin/sleep");
        assert_eq!(elide("", CMD_CHARS), "");
    }

    #[test]
    fn long_text_is_elided_at_a_character_boundary() {
        assert_eq!(elide("abcdef", 3), "abc…");
    }

    /// A command line with a multi-byte character across the cut-off used to
    /// panic and take the whole applet down with it.
    #[test]
    fn multibyte_characters_do_not_panic() {
        for pad in 0..8 {
            let cmd = format!("{}/Téléchargements/é-app --flag", "x".repeat(20 + pad));
            let elided = elide(&cmd, CMD_CHARS);
            assert_eq!(elided.chars().count(), CMD_CHARS + 1);
        }

        // Also correct when every character is multi-byte.
        assert_eq!(elide("日本語のプログラム", 4), "日本語の…");
    }
}
