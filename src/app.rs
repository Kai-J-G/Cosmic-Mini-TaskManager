//! Application state and update lifecycle.

use std::time::Duration;

use cosmic::app::{Core, Task};
use cosmic::cosmic_config::{Config, CosmicConfigEntry};
use cosmic::iced::Subscription;
use cosmic::iced::window::Id;
use cosmic::{Application, Element};

use crate::config::{APP_ID, CONFIG_VERSION, MiniTaskManagerConfig, ThemePreference};
use crate::process::{FilterTab, ProcessCollector, ProcessItem, SystemOverview, actions};
use crate::views;

pub struct AppModel {
    pub core: Core,
    pub popup: Option<Id>,
    pub config: MiniTaskManagerConfig,
    pub collector: ProcessCollector,
    pub overview: SystemOverview,
    pub processes: Vec<ProcessItem>,
    pub active_tab: FilterTab,
    pub search_query: String,
    pub show_settings: bool,
    /// Result of the last signal we sent, shown at the foot of the popup.
    pub status_message: Option<String>,
}

#[derive(Clone, Debug)]
pub enum Message {
    Tick,
    SelectTab(FilterTab),
    SearchInput(String),
    ToggleSettings,
    StopProcess(u32),
    ResumeProcess(u32),
    KillProcess(u32),
    KillAllUnresponsive,
    SetTheme(ThemePreference),
    SetInterval(u64),
    ToggleWarnInPanel(bool),
    ClosePopup,
    PopupClosed(Id),
    Surface(cosmic::surface::Action),
    ConfigChanged(MiniTaskManagerConfig),
}

impl AppModel {
    pub fn is_dark(&self) -> bool {
        match self.config.theme_pref {
            ThemePreference::Dark => true,
            ThemePreference::Light => false,
            ThemePreference::System => cosmic::theme::is_dark(),
        }
    }

    fn refresh(&mut self) {
        let (overview, processes) = self.collector.collect();
        self.overview = overview;
        self.processes = processes;
    }

    fn save_config(&self) {
        if let Ok(handler) = Config::new(APP_ID, CONFIG_VERSION) {
            let _ = self.config.write_entry(&handler);
        }
    }

    fn apply_theme(pref: ThemePreference) -> Task<Message> {
        cosmic::task::message(cosmic::Action::Cosmic(cosmic::app::Action::AppThemeChange(
            pref.theme(),
        )))
    }
}

impl Application for AppModel {
    type Executor = cosmic::iced::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let config = Config::new(APP_ID, CONFIG_VERSION)
            .ok()
            .and_then(|handler| MiniTaskManagerConfig::get_entry(&handler).ok())
            .unwrap_or_default();

        let mut collector = ProcessCollector::new();
        let (overview, processes) = collector.collect();

        let app = Self {
            core,
            popup: None,
            config,
            collector,
            overview,
            processes,
            active_tab: FilterTab::All,
            search_query: String::new(),
            show_settings: false,
            status_message: None,
        };

        let theme_task = Self::apply_theme(app.config.theme_pref);
        (app, theme_task)
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::Tick => {
                self.refresh();
            }
            Message::SelectTab(tab) => {
                self.active_tab = tab;
                self.status_message = None;
            }
            Message::SearchInput(query) => {
                self.search_query = query;
                self.status_message = None;
            }
            Message::ToggleSettings => {
                self.show_settings = !self.show_settings;
                self.status_message = None;
            }
            // Each signal is followed by a refresh so the row's status badge
            // updates immediately instead of at the next tick.
            Message::StopProcess(pid) => {
                self.status_message = Some(match actions::stop_process(pid) {
                    Ok(()) => crate::fl!("msg-stopped", pid = pid),
                    Err(error) => crate::fl!("msg-stop-failed", pid = pid, error = error),
                });
                self.refresh();
            }
            Message::ResumeProcess(pid) => {
                self.status_message = Some(match actions::resume_process(pid) {
                    Ok(()) => crate::fl!("msg-resumed", pid = pid),
                    Err(error) => crate::fl!("msg-resume-failed", pid = pid, error = error),
                });
                self.refresh();
            }
            Message::KillProcess(pid) => {
                self.status_message = Some(match actions::kill_process(pid) {
                    Ok(()) => crate::fl!("msg-killed", pid = pid),
                    Err(error) => crate::fl!("msg-kill-failed", pid = pid, error = error),
                });
                self.refresh();
            }
            Message::KillAllUnresponsive => {
                let pids: Vec<u32> = self
                    .processes
                    .iter()
                    .filter(|p| p.is_unresponsive_or_stopped())
                    .map(|p| p.pid)
                    .collect();
                let killed = actions::kill_all(&pids);
                self.status_message = Some(crate::fl!("msg-killed-all", count = killed));
                self.refresh();
            }
            Message::SetInterval(secs) => {
                self.config.refresh_interval_secs = secs;
                self.save_config();
            }
            Message::SetTheme(pref) => {
                self.config.theme_pref = pref;
                self.save_config();
                return Self::apply_theme(pref);
            }
            Message::ToggleWarnInPanel(enabled) => {
                self.config.warn_unresponsive_in_panel = enabled;
                self.save_config();
            }
            Message::ConfigChanged(config) => {
                let theme_changed = config.theme_pref != self.config.theme_pref;
                self.config = config;
                if theme_changed {
                    return Self::apply_theme(self.config.theme_pref);
                }
            }
            Message::ClosePopup => {
                if let Some(id) = self.popup.take() {
                    self.status_message = None;
                    return views::panel::destroy(id);
                }
            }
            Message::PopupClosed(id) => {
                if self.popup == Some(id) {
                    self.popup = None;
                    self.status_message = None;
                }
            }
            Message::Surface(action) => {
                return cosmic::task::message(cosmic::Action::Cosmic(
                    cosmic::app::Action::Surface(action),
                ));
            }
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        views::panel::view(self)
    }

    fn view_window(&self, _id: Id) -> Element<'_, Self::Message> {
        views::panel::popup_container(self, views::view_popup(self))
    }

    fn style(&self) -> Option<cosmic::iced::theme::Style> {
        Some(cosmic::applet::style())
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        let interval = Duration::from_secs(self.config.refresh_interval_secs.max(1));

        Subscription::batch([
            cosmic::iced::time::every(interval).map(|_| Message::Tick),
            self.core()
                .watch_config::<MiniTaskManagerConfig>(APP_ID)
                .map(|update| Message::ConfigChanged(update.config)),
        ])
    }

    fn on_close_requested(&self, id: Id) -> Option<Self::Message> {
        Some(Message::PopupClosed(id))
    }
}
