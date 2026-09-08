//! Application state and update lifecycle.

use std::time::Duration;

use cosmic::app::{Core, Task};
use cosmic::cosmic_config::{Config, CosmicConfigEntry};
use cosmic::iced::window::Id;
use cosmic::iced::Subscription;
use cosmic::{Application, Element};

use crate::config::{MiniTaskManagerConfig, ThemePreference, APP_ID, CONFIG_VERSION};
use crate::process::{
    actions, FilterTab, ProcessCollector, ProcessItem, SystemOverview,
};
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
        let Some(theme) = pref.theme() else {
            return Task::none();
        };
        cosmic::task::message(cosmic::Action::Cosmic(cosmic::app::Action::AppThemeChange(theme)))
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
            }
            Message::SearchInput(query) => {
                self.search_query = query;
            }
            Message::ToggleSettings => {
                self.show_settings = !self.show_settings;
            }
            Message::StopProcess(pid) => {
                match actions::stop_process(pid) {
                    Ok(()) => {
                        self.status_message = Some(format!("Stopped process PID {}", pid));
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Failed to stop PID {}: {}", pid, e));
                    }
                }
                self.refresh();
            }
            Message::ResumeProcess(pid) => {
                match actions::resume_process(pid) {
                    Ok(()) => {
                        self.status_message = Some(format!("Resumed process PID {}", pid));
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Failed to resume PID {}: {}", pid, e));
                    }
                }
                self.refresh();
            }
            Message::KillProcess(pid) => {
                match actions::kill_process(pid) {
                    Ok(()) => {
                        self.status_message = Some(format!("Killed process PID {}", pid));
                    }
                    Err(e) => {
                        self.status_message = Some(format!("Failed to kill PID {}: {}", pid, e));
                    }
                }
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
                self.status_message = Some(format!("Killed {} unresponsive process(es)", killed));
                self.refresh();
            }
            Message::SetInterval(sec) => {
                self.config.refresh_interval_secs = sec;
                self.save_config();
            }
            Message::SetTheme(pref) => {
                self.config.theme_pref = pref;
                self.save_config();
                return Self::apply_theme(pref);
            }
            Message::ToggleWarnInPanel(val) => {
                self.config.warn_unresponsive_in_panel = val;
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
                    return views::panel::destroy(id);
                }
            }
            Message::PopupClosed(id) => {
                if self.popup == Some(id) {
                    self.popup = None;
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
