//! Search input and filter tabs for process navigation.

use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, column, row, text_input};
use cosmic::Element;

use crate::app::{AppModel, Message};
use crate::process::FilterTab;

pub fn view(app: &AppModel) -> Element<'_, Message> {
    let sp = cosmic::theme::spacing();

    let search_box = text_input(crate::fl!("search-placeholder"), &app.search_query)
        .on_input(Message::SearchInput)
        .width(Length::Fill);

    let tabs = [
        FilterTab::All,
        FilterTab::Apps,
        FilterTab::TopCpu,
        FilterTab::TopMemory,
        FilterTab::Unresponsive,
    ];

    let mut tab_buttons = Vec::new();
    for tab in tabs {
        let is_selected = app.active_tab == tab;
        let mut label_str = tab.label().to_string();

        if tab == FilterTab::Unresponsive && app.overview.unresponsive_or_stopped_count > 0 {
            label_str = format!("⚠️ {} ({})", tab.label(), app.overview.unresponsive_or_stopped_count);
        }

        let btn = button::text(label_str)
            .on_press(Message::SelectTab(tab))
            .class(if is_selected {
                cosmic::theme::Button::Suggested
            } else {
                cosmic::theme::Button::Text
            })
            .padding([4, 10]);

        tab_buttons.push(btn.into());
    }

    let tabs_row = row(tab_buttons)
        .spacing(sp.space_xxs)
        .align_y(Alignment::Center);

    let filter_container = column![
        search_box,
        tabs_row,
    ]
    .spacing(sp.space_xs)
    .width(Length::Fill);

    filter_container.into()
}
