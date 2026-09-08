//! Search input and filter tabs for process navigation.

use cosmic::Element;
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, column, row, text_input};

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

    let unresponsive = app.overview.unresponsive_or_stopped_count;

    let tab_buttons: Vec<_> = tabs
        .into_iter()
        .map(|tab| {
            let mut label = tab.label();
            // The alert banner already says what happened; the tab only needs
            // to carry the count.
            if tab == FilterTab::Unresponsive && unresponsive > 0 {
                label = format!("{label} ({unresponsive})");
            }

            button::text(label)
                .on_press(Message::SelectTab(tab))
                .class(if app.active_tab == tab {
                    cosmic::theme::Button::Suggested
                } else {
                    cosmic::theme::Button::Text
                })
                .padding([4, 10])
                .into()
        })
        .collect();

    let tabs_row = row(tab_buttons)
        .spacing(sp.space_xxs)
        .align_y(Alignment::Center);

    column![search_box, tabs_row]
        .spacing(sp.space_xs)
        .width(Length::Fill)
        .into()
}
