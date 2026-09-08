//! Single process row widget with process info, status badge, and "Stop" / "Kill" buttons.

use cosmic::iced::{Alignment, Color, Length};
use cosmic::widget::{button, column, container, icon, row, text};
use cosmic::Element;

use crate::app::Message;
use crate::process::{ProcessItem, ProcessState};
use crate::views::style::{alert_container, badge_container};

pub fn view<'a>(item: &'a ProcessItem, is_dark: bool) -> Element<'a, Message> {
    let sp = cosmic::theme::spacing();

    // Process icon
    let proc_icon = icon::from_name(&*item.icon_name)
        .size(24)
        .symbolic(false)
        .icon();

    // Title: Name + PID
    let name_text = text::body(&item.name);
    let pid_text = text::caption(format!("({})", item.pid));

    let title_line = row![name_text, pid_text]
        .spacing(sp.space_xxs)
        .align_y(Alignment::Center);

    // Subtitle: Command line truncated
    let cmd_clean = if item.cmd.is_empty() {
        &item.name
    } else {
        &item.cmd
    };
    let truncated_cmd = if cmd_clean.len() > 36 {
        format!("{}…", &cmd_clean[..36])
    } else {
        cmd_clean.to_string()
    };
    let cmd_line = text::caption(truncated_cmd);

    let info_col = column![title_line, cmd_line]
        .spacing(2)
        .width(Length::Fill);

    // Metrics: CPU & Memory (fixed width so it never overlaps with process title)
    let cpu_str = if item.cpu_usage > 0.05 {
        format!("{:.1}%", item.cpu_usage)
    } else {
        "0.0%".to_string()
    };
    let cpu_text = text::caption(format!("CPU {}", cpu_str));
    let mem_text = text::caption(item.memory_formatted());

    let metrics_col = column![cpu_text, mem_text]
        .spacing(2)
        .width(Length::Fixed(90.0));

    // Status Badge (fixed width for vertical column alignment)
    let status_badge = container(make_status_badge(item.status))
        .width(Length::Fixed(60.0))
        .center_x(Length::Fixed(60.0));

    // "Stop or Kill" Action Buttons
    let action_buttons = match item.status {
        ProcessState::Stopped => {
            let resume_btn = button::text(crate::fl!("btn-resume"))
                .on_press(Message::ResumeProcess(item.pid))
                .class(cosmic::theme::Button::Suggested)
                .padding([3, 8]);

            let kill_btn = button::text(crate::fl!("btn-kill"))
                .on_press(Message::KillProcess(item.pid))
                .class(cosmic::theme::Button::Destructive)
                .padding([3, 8]);

            row![resume_btn, kill_btn].spacing(sp.space_xxs)
        }
        ProcessState::Zombie | ProcessState::DiskSleep => {
            let kill_btn = button::text(crate::fl!("btn-kill"))
                .on_press(Message::KillProcess(item.pid))
                .class(cosmic::theme::Button::Destructive)
                .padding([3, 10]);

            row![kill_btn]
        }
        _ => {
            let stop_btn = button::text(crate::fl!("btn-stop"))
                .on_press(Message::StopProcess(item.pid))
                .class(cosmic::theme::Button::Text)
                .padding([3, 8]);

            let kill_btn = button::text(crate::fl!("btn-kill"))
                .on_press(Message::KillProcess(item.pid))
                .class(cosmic::theme::Button::Destructive)
                .padding([3, 8]);

            row![stop_btn, kill_btn].spacing(sp.space_xxs)
        }
    }
    .align_y(Alignment::Center);

    let actions = container(action_buttons)
        .width(Length::Fixed(125.0))
        .align_x(cosmic::iced::alignment::Horizontal::Right);

    let row_content = row![
        proc_icon,
        info_col,
        metrics_col,
        status_badge,
        actions,
    ]
    .spacing(sp.space_xs)
    .align_y(Alignment::Center);

    if item.is_unresponsive_or_stopped() {
        container(row_content.padding([sp.space_xxs, sp.space_xs]))
            .class(alert_container(is_dark))
            .width(Length::Fill)
            .into()
    } else {
        container(row_content.padding([sp.space_xxs, sp.space_xs]))
            .class(cosmic::theme::Container::Card)
            .width(Length::Fill)
            .into()
    }
}

fn make_status_badge(status: ProcessState) -> Element<'static, Message> {
    let bg_color = match status {
        ProcessState::Running => Color::from_rgba(0.06, 0.72, 0.51, 0.18),
        ProcessState::Sleeping => Color::from_rgba(0.5, 0.5, 0.5, 0.12),
        ProcessState::Stopped => Color::from_rgba(0.96, 0.62, 0.04, 0.28),
        ProcessState::Zombie => Color::from_rgba(0.66, 0.33, 0.98, 0.28),
        ProcessState::DiskSleep => Color::from_rgba(0.93, 0.27, 0.27, 0.28),
        ProcessState::Other => Color::from_rgba(0.5, 0.5, 0.5, 0.12),
    };

    container(text::caption(status.label()).size(10))
        .padding([2, 6])
        .class(badge_container(bg_color))
        .into()
}
