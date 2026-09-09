//! The panel button and the popup surface it opens.

use std::sync::LazyLock;

use cosmic::Element;
use cosmic::app::Task;
use cosmic::applet::cosmic_panel_config::PanelAnchor;
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::window::Id;
use cosmic::iced::{Alignment, Border, Color, Length, Limits, Rectangle, Shadow};
use cosmic::surface::action::{LiveSettings, app_popup, destroy_popup};
use cosmic::widget::{autosize, button, column, container, icon, row, text};

use crate::app::{AppModel, Message};

static PANEL_ID: LazyLock<cosmic::widget::Id> =
    LazyLock::new(|| cosmic::widget::Id::new("autosize-main"));
static POPUP_ID: LazyLock<cosmic::widget::Id> =
    LazyLock::new(|| cosmic::widget::Id::new("cosmic-ext-mini-taskmanager-popup"));

const POPUP_WIDTH: f32 = 640.0;
const MIN_HEIGHT: f32 = 350.0;
const MAX_HEIGHT: f32 = 850.0;

/// What the panel icon is currently reporting.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pulse {
    /// Something is stopped or hung.
    Alert,
    /// The machine is busy.
    Busy,
    Idle {
        dark: bool,
    },
}

/// CPU load above which the icon turns amber.
const BUSY_CPU_PERCENT: f32 = 75.0;

impl Pulse {
    /// The four icons are built once. Re-formatting and re-parsing the SVG on
    /// every frame showed up as steady allocation churn in the panel.
    fn handle(self) -> icon::Handle {
        static ALERT: LazyLock<icon::Handle> = LazyLock::new(|| svg("#ef4444", true));
        static BUSY: LazyLock<icon::Handle> = LazyLock::new(|| svg("#f59e0b", false));
        static DARK: LazyLock<icon::Handle> = LazyLock::new(|| svg("#38bdf8", false));
        static LIGHT: LazyLock<icon::Handle> = LazyLock::new(|| svg("#0284c7", false));

        match self {
            Self::Alert => ALERT.clone(),
            Self::Busy => BUSY.clone(),
            Self::Idle { dark: true } => DARK.clone(),
            Self::Idle { dark: false } => LIGHT.clone(),
        }
    }
}

/// An activity-monitor glyph, optionally with a warning badge in the corner.
fn svg(stroke: &str, badge: bool) -> icon::Handle {
    let trace = if badge {
        // Shortened so the trace does not run under the badge.
        r#"M 2 8.2 L 4.2 8.2 L 5.4 5.2 L 6.8 11.2 L 7.8 7.2 L 8.6 9.4 L 9.4 8.2 L 10.5 8.2"#
    } else {
        r#"M 2 8.2 L 4.4 8.2 L 5.7 4.5 L 7.4 12.2 L 8.7 6.2 L 9.8 9.6 L 10.8 8.2 L 14 8.2"#
    };

    let marker = if badge {
        format!(
            r##"<circle cx="12.5" cy="4" r="2.8" fill="{stroke}" />
  <path d="M 12.5 2.5 L 12.5 4.3 M 12.5 5.5 L 12.5 5.6" stroke="#ffffff" stroke-width="0.9" stroke-linecap="round" />"##
        )
    } else {
        format!(r##"<circle cx="10.8" cy="8.2" r="1.1" fill="{stroke}" />"##)
    };

    let markup = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" width="32" height="32" fill="none">
  <rect x="0.8" y="1.8" width="14.4" height="12.4" rx="2.8" stroke="{stroke}" stroke-width="1.3" />
  <path d="{trace}" stroke="{stroke}" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" />
  {marker}
</svg>"##
    );

    icon::from_svg_bytes(markup.into_bytes())
}

/// The panel button: activity glyph plus CPU percentage, or a count of
/// unresponsive processes when there are any.
pub fn view(app: &AppModel) -> Element<'_, Message> {
    let sp = cosmic::theme::spacing();
    let size = app.core.applet.suggested_size(true);

    let has_alert =
        app.overview.unresponsive_or_stopped_count > 0 && app.config.warn_unresponsive_in_panel;

    let pulse = if has_alert {
        Pulse::Alert
    } else if app.overview.total_cpu_percent > BUSY_CPU_PERCENT {
        Pulse::Busy
    } else {
        Pulse::Idle {
            dark: app.is_dark(),
        }
    };

    let icon_el = icon::icon(pulse.handle()).size(size.0);

    let label = if has_alert {
        format!("!{}", app.overview.unresponsive_or_stopped_count)
    } else {
        format!("{:.0}%", app.overview.total_cpu_percent)
    };
    let label_el = text::body(label).size(12);

    let horizontal = app.core.applet.is_horizontal();
    let content: Element<'_, Message> = if horizontal {
        row![icon_el, label_el]
            .spacing(sp.space_xxs)
            .align_y(Alignment::Center)
            .into()
    } else {
        column![icon_el, label_el]
            .spacing(2)
            .align_x(Alignment::Center)
            .into()
    };

    let padding = app.core.applet.suggested_padding(true).0;
    let is_open = app.popup.is_some();

    let button = button::custom(content)
        .padding(if horizontal {
            [0, padding]
        } else {
            [padding, 0]
        })
        .class(cosmic::theme::Button::AppletIcon)
        .on_press_with_rectangle(move |offset, bounds| {
            if is_open {
                Message::ClosePopup
            } else {
                Message::Surface(open_popup(offset, bounds))
            }
        });

    autosize::autosize(button, PANEL_ID.clone()).into()
}

fn open_popup(offset: cosmic::iced::Vector, bounds: Rectangle) -> cosmic::surface::Action {
    app_popup::<AppModel>(
        |_| LiveSettings {
            blur: Some(true),
            ..Default::default()
        },
        move |app: &mut AppModel| {
            let id = Id::unique();
            app.popup = Some(id);

            let mut settings = app.core.applet.get_popup_settings(
                app.core
                    .main_window_id()
                    .expect("applet always has a main window"),
                id,
                Some((POPUP_WIDTH as u32, 600)),
                None,
                None,
            );

            settings.positioner.size_limits = popup_limits();
            settings.positioner.anchor_rect = Rectangle {
                x: (bounds.x - offset.x) as i32,
                y: (bounds.y - offset.y) as i32,
                width: bounds.width as i32,
                height: bounds.height as i32,
            };
            settings.positioner.offset = centering_offset(app, settings.positioner.offset, bounds);
            settings
        },
        None,
    )
}

fn centering_offset(app: &AppModel, offset: (i32, i32), bounds: Rectangle) -> (i32, i32) {
    let (icon_width, icon_height) = app.core.applet.suggested_size(true);
    let (_, padding_across) = app.core.applet.suggested_padding(true);

    let (slot, button) = if app.core.applet.is_horizontal() {
        (f32::from(icon_height + 2 * padding_across), bounds.height)
    } else {
        (f32::from(icon_width + 2 * padding_across), bounds.width)
    };

    let shortfall = ((slot - button) / 2.0).max(0.0).round() as i32;
    let (x, y) = offset;

    (x + x.signum() * shortfall, y + y.signum() * shortfall)
}

/// Wraps the popup content in the shell's rounded, themed frosted glass popup surface.
pub fn popup_container<'a>(
    app: &AppModel,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    let (align_y, align_x) = match app.core.applet.anchor {
        PanelAnchor::Left => (Vertical::Center, Horizontal::Left),
        PanelAnchor::Right => (Vertical::Center, Horizontal::Right),
        PanelAnchor::Top => (Vertical::Top, Horizontal::Center),
        PanelAnchor::Bottom => (Vertical::Bottom, Horizontal::Center),
    };

    let surface = container(content).style(|theme| {
        let cosmic = theme.cosmic();
        let background = cosmic.background(true);
        cosmic::iced::widget::container::Style {
            text_color: Some(background.on.into()),
            background: Some(Color::from(background.base).into()),
            border: Border {
                radius: cosmic.corner_radii.radius_m.into(),
                width: 1.0,
                color: background.divider.into(),
            },
            shadow: Shadow::default(),
            icon_color: Some(background.on.into()),
            snap: true,
        }
    });

    autosize::autosize(
        container(surface)
            .height(Length::Shrink)
            .align_x(align_x)
            .align_y(align_y),
        POPUP_ID.clone(),
    )
    .limits(popup_limits())
    .into()
}

/// The popup is a fixed width and grows only vertically, within bounds.
fn popup_limits() -> Limits {
    Limits::NONE
        .min_height(MIN_HEIGHT)
        .min_width(POPUP_WIDTH)
        .max_width(POPUP_WIDTH)
        .max_height(MAX_HEIGHT)
}

pub fn destroy(id: Id) -> Task<Message> {
    cosmic::task::message(cosmic::Action::Cosmic(cosmic::app::Action::Surface(
        destroy_popup(id),
    )))
}
