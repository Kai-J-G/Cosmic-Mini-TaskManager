//! Cosmic Mini Task Manager
//!
//! A native task manager and process monitor applet for the COSMIC Desktop Environment (Kashi OS).
//!
//! # Architecture
//!
//! - [`app`]: Defines [`app::AppModel`] implementing [`cosmic::Application`], handling
//!   the Elm-style update loop, subscriptions (periodic tick and cosmic-config watch),
//!   and popup surface lifecycle.
//! - [`config`]: Persistent settings (`cosmic-config`) for theme preference, refresh
//!   intervals, sorting defaults, and alert badge toggling.
//! - [`process`]: System information collector ([`process::ProcessCollector`]) powered by
//!   `sysinfo`, desktop `.desktop` file metadata matching, and POSIX process signal dispatching
//!   ([`process::actions`]).
//! - [`views`]: Native `libcosmic` UI components, including the panel button, frosted-glass
//!   Wayland popup surface, resource gauges, filter tabs, alert banners, and process rows.

mod app;
mod config;
mod process;
mod views;

fn main() -> cosmic::iced::Result {
    cosmic::applet::run::<app::AppModel>(())
}

