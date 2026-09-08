//! UI layout and views built with `libcosmic`.
//!
//! Submodules:
//! - [`panel`]: The panel button in the COSMIC dock/bar and Wayland popup surface management.
//! - [`header`]: System CPU/RAM meters, total process counters, and action buttons.
//! - [`filter_bar`]: Search text input and category filter tabs.
//! - [`alert_banner`]: Warning banner for hung or stopped tasks with "Kill All" action.
//! - [`process_row`]: Tabular row display for individual processes with "Stop", "Resume", and "Kill" buttons.
//! - [`settings`]: Settings panel for theme preference, refresh rate, and panel alert toggle.
//! - [`style`]: Custom container and badge styling helper functions.

pub mod alert_banner;
pub mod filter_bar;
pub mod header;
pub mod panel;
pub mod process_row;
pub mod settings;
pub mod style;


use cosmic::iced::Length;
use cosmic::widget::{column, container, scrollable, text};
use cosmic::Element;

use crate::app::{AppModel, Message};
use crate::process::filter_and_sort_processes;

/// Renders the complete task manager popup view.
pub fn view_popup<'a>(app: &'a AppModel) -> Element<'a, Message> {
    let sp = cosmic::theme::spacing();
    let is_dark = app.is_dark();

    if app.show_settings {
        return column![
            header::view(app),
            settings::view(app),
        ]
        .spacing(sp.space_s)
        .padding(sp.space_s)
        .into();
    }

    let header_el = header::view(app);
    let alert_el = alert_banner::view(app);
    let filter_el = filter_bar::view(app);

    // Filter and sort items according to current tab and query
    let filtered_procs = filter_and_sort_processes(&app.processes, app.active_tab, &app.search_query);

    let mut rows = Vec::new();
    if filtered_procs.is_empty() {
        rows.push(
            container(
                text::body(crate::fl!("no-processes"))
            )
            .padding(sp.space_l)
            .center(Length::Fill)
            .into()
        );
    } else {
        // Limit rendering to top 80 to keep scrolling silky smooth
        for proc_item in filtered_procs.iter().take(80) {
            rows.push(process_row::view(proc_item, is_dark));
        }
    }

    let list_column = column(rows).spacing(sp.space_xxs).width(Length::Fill);
    let scroll_list = scrollable(list_column)
        .height(Length::Fixed(360.0))
        .width(Length::Fill);

    let mut main_col = column![header_el];

    if let Some(alert) = alert_el {
        main_col = main_col.push(alert);
    }

    main_col = main_col.push(filter_el).push(scroll_list);

    if let Some(msg) = &app.status_message {
        let status_bar = text::caption(msg);
        main_col = main_col.push(status_bar);
    }

    main_col
        .spacing(sp.space_xs)
        .padding(sp.space_s)
        .into()
}
