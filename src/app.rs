//! Application state and update lifecycle.

use std::time::{Duration, Instant};

use cosmic::app::{Core, Task};
use cosmic::cosmic_config::{Config, CosmicConfigEntry};
use cosmic::iced::Subscription;
use cosmic::iced::window::Id;
use cosmic::{Application, Element};

use crate::config::{APP_ID, CONFIG_VERSION, MiniTaskManagerConfig, ThemePreference};
use crate::process::{
    FilterTab, ProcessCollector, ProcessItem, SystemOverview, actions, tree_pids,
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
    /// Result of the last signal we sent, shown at the foot of the popup.
    pub status_message: Option<String>,
    /// When the last poll happened, to avoid sampling faster than CPU usage
    /// can actually be measured.
    pub last_refresh: Instant,
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
        // CPU usage is a delta between two samples. Sampling again within
        // sysinfo's minimum interval yields 0% for every process, so a quick
        // double-click on Refresh would blank the whole list.
        if self.last_refresh.elapsed() < sysinfo::MINIMUM_CPU_UPDATE_INTERVAL {
            return;
        }

        let (overview, processes) = self.collector.collect();
        self.overview = overview;
        self.processes = processes;
        self.last_refresh = Instant::now();
    }

    /// The process plus every descendant, which is what "kill" has to mean if
    /// it isn't going to leave orphans running.
    ///
    /// Our own PID is dropped from the sweep unless it is the process the user
    /// actually clicked: dying midway would leave the rest of the tree alive.
    fn kill_targets(&self, root: u32) -> Vec<u32> {
        let own_pid = std::process::id();
        tree_pids(root, &self.processes)
            .into_iter()
            .filter(|&pid| pid == root || pid != own_pid)
            .collect()
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
            last_refresh: Instant::now(),
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
                let pids = self.kill_targets(pid);
                let children = pids.len().saturating_sub(1);

                self.status_message = Some(match actions::kill_process(pid) {
                    Ok(()) => {
                        // The root died; sweep whatever it left behind.
                        let swept = actions::kill_all(&pids[1..]);
                        if children == 0 {
                            crate::fl!("msg-killed", pid = pid)
                        } else {
                            crate::fl!("msg-killed-tree", pid = pid, count = swept)
                        }
                    }
                    Err(error) => crate::fl!("msg-kill-failed", pid = pid, error = error),
                });
                self.refresh();
            }
            Message::KillAllUnresponsive => {
                let roots: Vec<u32> = self
                    .processes
                    .iter()
                    .filter(|p| p.is_unresponsive_or_stopped())
                    .map(|p| p.pid)
                    .collect();

                // Trees overlap when a hung parent and its hung child are both
                // listed, so de-duplicate before signalling anything twice.
                let mut pids = Vec::new();
                for root in roots {
                    for pid in self.kill_targets(root) {
                        if !pids.contains(&pid) {
                            pids.push(pid);
                        }
                    }
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::ProcessState;

    fn model() -> AppModel {
        let mut collector = ProcessCollector::new();
        let (overview, processes) = collector.collect();
        AppModel {
            core: Core::default(),
            popup: None,
            config: MiniTaskManagerConfig::default(),
            collector,
            overview,
            processes,
            active_tab: FilterTab::All,
            search_query: String::new(),
            show_settings: false,
            status_message: None,
            last_refresh: Instant::now(),
        }
    }

    /// Process CPU usage is a delta between two samples. Polling again inside
    /// sysinfo's minimum interval reports 0% for everything, so a fast double
    /// click on Refresh would blank every row.
    #[test]
    fn refresh_is_ignored_when_it_would_report_zero() {
        let mut app = model();

        // Give the collector a real sampling window so the numbers mean
        // something, then record them.
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL * 3);
        app.refresh();
        let settled = app.overview.total_cpu_percent;
        let busiest = app
            .processes
            .iter()
            .map(|p| p.cpu_usage)
            .fold(0.0f32, f32::max);
        assert!(
            busiest > 0.0,
            "no process reported any CPU; test is not measuring anything"
        );

        // An immediate second poll must be dropped rather than served a
        // window too short to measure.
        app.refresh();
        assert_eq!(app.overview.total_cpu_percent, settled);
        assert_eq!(
            app.processes
                .iter()
                .map(|p| p.cpu_usage)
                .fold(0.0f32, f32::max),
            busiest
        );

        // Once the window has passed it polls again.
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL * 3);
        app.refresh();
        assert!(app.last_refresh.elapsed() < sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
    }

    /// The sweep must cover descendants, and must not drop the clicked process
    /// even when that process is the applet itself.
    #[test]
    fn kill_targets_covers_descendants_and_keeps_the_clicked_pid() {
        let mut app = model();
        app.processes = vec![
            ProcessItem {
                pid: 500,
                parent: Some(1),
                name: "parent".into(),
                cmd: String::new(),
                cpu_usage: 0.0,
                memory_bytes: 0,
                status: ProcessState::Running,
                is_gui_app: false,
                icon_name: String::new(),
            },
            ProcessItem {
                pid: std::process::id(),
                parent: Some(500),
                name: "us".into(),
                cmd: String::new(),
                cpu_usage: 0.0,
                memory_bytes: 0,
                status: ProcessState::Running,
                is_gui_app: false,
                icon_name: String::new(),
            },
        ];

        // Our own PID is a descendant here, and is skipped so the sweep can
        // finish instead of dying halfway through.
        assert_eq!(app.kill_targets(500), [500]);

        // Clicked directly, it is still the target.
        assert_eq!(app.kill_targets(std::process::id()), [std::process::id()]);
    }
}
