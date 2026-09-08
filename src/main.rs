//! Cosmic Mini Task Manager: a process monitor applet for the COSMIC desktop.
//!
//! - [`app`]: the Elm-style update loop, subscriptions, and popup lifecycle.
//! - [`config`]: settings persisted through `cosmic-config`.
//! - [`process`]: `sysinfo` polling, `.desktop` matching, and signal dispatch.
//! - [`views`]: the panel button and the widgets inside the popup.
//! - [`localize`]: Fluent catalogs embedded at build time.

mod app;
mod config;
pub mod localize;
mod process;
mod views;

fn main() -> cosmic::iced::Result {
    localize::init();
    cosmic::applet::run::<app::AppModel>(())
}
