//! UI layout and views built with `libcosmic`.

pub mod alert_banner;
pub mod filter_bar;
pub mod header;
pub mod panel;
pub mod process_row;
pub mod settings;
pub mod style;

use cosmic::Element;
use cosmic::iced::Length;
use cosmic::widget::{column, container, scrollable, text};

use crate::app::{AppModel, Message};
use crate::process::filter_and_sort_processes;

const LIST_HEIGHT: f32 = 360.0;

/// `iced` builds every widget in the tree each frame, so a machine with a few
/// hundred processes would pay for rows nobody scrolls to. The tabs and the
/// search box are how you reach the rest.
const MAX_ROWS: usize = 80;

/// Renders the task manager popup.
pub fn view_popup(app: &AppModel) -> Element<'_, Message> {
    let sp = cosmic::theme::spacing();

    if app.show_settings {
        return column![header::view(app), settings::view(app)]
            .spacing(sp.space_s)
            .padding(sp.space_s)
            .into();
    }

    let processes = filter_and_sort_processes(&app.processes, app.active_tab, &app.search_query);

    let rows: Vec<Element<'_, Message>> = if processes.is_empty() {
        vec![
            container(text::body(crate::fl!("no-processes")))
                .padding(sp.space_l)
                .center(Length::Fill)
                .into(),
        ]
    } else {
        processes
            .iter()
            .take(MAX_ROWS)
            .map(|item| process_row::view(item))
            .collect()
    };

    let list = scrollable(column(rows).spacing(sp.space_xxs).width(Length::Fill))
        .height(Length::Fixed(LIST_HEIGHT))
        .width(Length::Fill);

    let mut content = column![header::view(app)];

    if let Some(banner) = alert_banner::view(app) {
        content = content.push(banner);
    }

    content = content.push(filter_bar::view(app)).push(list);

    if let Some(message) = &app.status_message {
        content = content.push(text::caption(message));
    }

    content.spacing(sp.space_xs).padding(sp.space_s).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{MiniTaskManagerConfig, ThemePreference};
    use crate::process::{FilterTab, ProcessCollector, ProcessItem, ProcessState, SystemOverview};

    /// Names and command lines that have historically broken rendering:
    /// multi-byte characters around the elision point, empty strings,
    /// and out-of-range metrics.
    fn awkward_processes() -> Vec<ProcessItem> {
        let states = [
            ProcessState::Running,
            ProcessState::Sleeping,
            ProcessState::Stopped,
            ProcessState::Zombie,
            ProcessState::DiskSleep,
            ProcessState::Other,
        ];

        // Slide a multi-byte character across the elision point so at least
        // one case lands mid-codepoint wherever that point happens to be.
        let mut commands = vec![
            String::new(),
            "x".repeat(200),
            "日本語のとても長いプログラム名前です --オプション".to_string(),
            "/usr/bin/sleep".to_string(),
        ];
        // A run of two-byte characters shifted one byte at a time, so a
        // character straddles the elision point no matter where it falls.
        for pad in 0..64 {
            commands.push(format!("{}{}", "x".repeat(pad), "é".repeat(40)));
        }

        let mut items = Vec::new();
        for (i, status) in states.into_iter().enumerate() {
            for (j, cmd) in commands.iter().cloned().enumerate() {
                items.push(ProcessItem {
                    pid: (i * 10 + j) as u32,
                    name: if cmd.is_empty() {
                        String::new()
                    } else {
                        cmd.clone()
                    },
                    cmd,
                    cpu_usage: [0.0, f32::NAN, 1234.5, -1.0][j % 4],
                    memory_bytes: [0, u64::MAX, 1024, 5 << 30][j % 4],
                    status,
                    is_gui_app: i % 2 == 0,
                    icon_name: String::new(),
                });
            }
        }
        items
    }

    fn model(processes: Vec<ProcessItem>, tab: FilterTab, show_settings: bool) -> AppModel {
        let unresponsive = processes
            .iter()
            .filter(|p| p.is_unresponsive_or_stopped())
            .count();

        AppModel {
            core: cosmic::app::Core::default(),
            popup: None,
            config: MiniTaskManagerConfig {
                theme_pref: ThemePreference::Dark,
                ..Default::default()
            },
            collector: ProcessCollector::new(),
            overview: SystemOverview {
                total_cpu_percent: 42.5,
                used_memory_bytes: 8 << 30,
                total_memory_bytes: 16 << 30,
                memory_percent: 50.0,
                total_processes: processes.len(),
                unresponsive_or_stopped_count: unresponsive,
            },
            processes,
            active_tab: tab,
            search_query: String::new(),
            show_settings,
            status_message: Some("Killed process PID 1234".to_string()),
        }
    }

    const TABS: [FilterTab; 5] = [
        FilterTab::All,
        FilterTab::Apps,
        FilterTab::TopCpu,
        FilterTab::TopMemory,
        FilterTab::Unresponsive,
    ];

    /// Building the widget tree is where a bad slice or a missing message id
    /// shows up, so do it for every tab against hostile data.
    #[test]
    fn every_tab_builds_a_widget_tree() {
        for tab in TABS {
            let app = model(awkward_processes(), tab, false);
            let _ = view_popup(&app);
            let _ = panel::view(&app);
        }
    }

    #[test]
    fn settings_and_empty_states_build() {
        let _ = view_popup(&model(awkward_processes(), FilterTab::All, true));
        // No processes at all, and a search that matches nothing.
        let _ = view_popup(&model(Vec::new(), FilterTab::All, false));

        let mut app = model(awkward_processes(), FilterTab::All, false);
        app.search_query = "no-such-process".to_string();
        app.status_message = None;
        let _ = view_popup(&app);
    }

    /// The panel button has to render in each of its four states.
    #[test]
    fn panel_button_builds_in_every_state() {
        for (cpu, unresponsive, warn) in [
            (10.0, 0, true),
            (99.0, 0, true),
            (10.0, 3, true),
            (10.0, 3, false),
        ] {
            let mut app = model(awkward_processes(), FilterTab::All, false);
            app.overview.total_cpu_percent = cpu;
            app.overview.unresponsive_or_stopped_count = unresponsive;
            app.config.warn_unresponsive_in_panel = warn;
            let _ = panel::view(&app);
        }
    }
}
