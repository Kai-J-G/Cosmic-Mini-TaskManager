//! Header view with system resource gauges (CPU & RAM meters) and quick actions.

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, column, container, icon, progress_bar, row, space, text};
use cosmic::Element;

use crate::app::{AppModel, Message};
use crate::process::format_bytes;

pub fn view(app: &AppModel) -> Element<'_, Message> {
    let sp = cosmic::theme::spacing();
    
    let overview = &app.overview;

    // Header Title & Controls Row
    let title_icon = icon::from_name("utilities-system-monitor-symbolic")
        .size(20)
        .symbolic(true)
        .icon();
    let title_text = text::title3(crate::fl!("app-title"));
    let proc_badge = text::caption(crate::fl!("process-count", count = overview.total_processes));

    let title_row = row![title_icon, title_text, proc_badge]
        .spacing(sp.space_xs)
        .align_y(Alignment::Center);

    let refresh_btn = button::icon(
        icon::from_name("view-refresh-symbolic")
            .size(16)
            .symbolic(true),
    )
    .on_press(Message::Tick)
    .class(cosmic::theme::Button::Text);

    let settings_icon = if app.show_settings {
        "go-previous-symbolic"
    } else {
        "emblem-system-symbolic"
    };

    let settings_btn = button::icon(
        icon::from_name(settings_icon)
            .size(16)
            .symbolic(true),
    )
    .on_press(Message::ToggleSettings)
    .class(cosmic::theme::Button::Text);

    let top_bar = row![
        title_row,
        space::horizontal().width(Length::Fill),
        refresh_btn,
        settings_btn,
    ]
    .align_y(Alignment::Center);

    // Resource Meters: CPU & RAM
    let cpu_prog = (overview.total_cpu_percent / 100.0).clamp(0.0, 1.0);
    let cpu_percent_str = format!("{:.1}", overview.total_cpu_percent);
    let cpu_label = text::caption(crate::fl!("cpu-label", percent = cpu_percent_str));
    let cpu_bar = progress_bar::determinate_linear(cpu_prog)
        .width(Length::Fill);

    let cpu_card = column![cpu_label, cpu_bar].spacing(sp.space_xxs);

    let ram_prog = (overview.memory_percent / 100.0).clamp(0.0, 1.0);
    let ram_used = format_bytes(overview.used_memory_bytes);
    let ram_total = format_bytes(overview.total_memory_bytes);
    let ram_percent_str = format!("{:.1}", overview.memory_percent);
    let ram_label = text::caption(crate::fl!(
        "ram-label",
        used = ram_used,
        total = ram_total,
        percent = ram_percent_str
    ));
    let ram_bar = progress_bar::determinate_linear(ram_prog)
        .width(Length::Fill);

    let ram_card = column![ram_label, ram_bar].spacing(sp.space_xxs);

    let gauges_row = row![
        container(cpu_card).width(Length::FillPortion(1)),
        space::horizontal().width(sp.space_s),
        container(ram_card).width(Length::FillPortion(1)),
    ]
    .align_y(Alignment::Center);

    let header_card = container(
        column![top_bar, gauges_row]
            .spacing(sp.space_s)
            .padding(sp.space_s),
    )
    .class(cosmic::theme::Container::Card)
    .width(Length::Fill);

    header_card.into()
}
