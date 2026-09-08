//! The panel button and the popup surface it opens.

use std::sync::LazyLock;

use cosmic::app::Task;
use cosmic::applet::cosmic_panel_config::PanelAnchor;
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::window::Id;
use cosmic::iced::{Alignment, Border, Color, Length, Limits, Rectangle, Shadow};
use cosmic::surface::action::{app_popup, destroy_popup, LiveSettings};
use cosmic::widget::{autosize, button, column, container, icon, row, text};
use cosmic::Element;

use crate::app::{AppModel, Message};

static PANEL_ID: LazyLock<cosmic::widget::Id> =
    LazyLock::new(|| cosmic::widget::Id::new("autosize-main"));
static POPUP_ID: LazyLock<cosmic::widget::Id> =
    LazyLock::new(|| cosmic::widget::Id::new("cosmic-mini-taskmanager-popup"));

const POPUP_WIDTH: f32 = 640.0;
const MAX_HEIGHT: f32 = 850.0;

/// Generates a crisp, theme-tinted activity monitor SVG glyph for the panel button.
fn pulse_icon_svg(stroke_color: &str, alert_color: Option<&str>) -> String {
    if let Some(alert) = alert_color {
        format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" width="32" height="32" fill="none">
  <rect x="0.8" y="1.8" width="14.4" height="12.4" rx="2.8" stroke="{stroke_color}" stroke-width="1.3" />
  <path d="M 2 8.2 L 4.2 8.2 L 5.4 5.2 L 6.8 11.2 L 7.8 7.2 L 8.6 9.4 L 9.4 8.2 L 10.5 8.2" 
        stroke="{stroke_color}" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" />
  <circle cx="12.5" cy="4" r="2.8" fill="{alert}" />
  <path d="M 12.5 2.5 L 12.5 4.3 M 12.5 5.5 L 12.5 5.6" stroke="#ffffff" stroke-width="0.9" stroke-linecap="round" />
</svg>"##
        )
    } else {
        format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" width="32" height="32" fill="none">
  <rect x="0.8" y="1.8" width="14.4" height="12.4" rx="2.8" stroke="{stroke_color}" stroke-width="1.3" />
  <path d="M 2 8.2 L 4.4 8.2 L 5.7 4.5 L 7.4 12.2 L 8.7 6.2 L 9.8 9.6 L 10.8 8.2 L 14 8.2" 
        stroke="{stroke_color}" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round" />
  <circle cx="10.8" cy="8.2" r="1.1" fill="{stroke_color}" />
</svg>"##
        )
    }
}

/// Applet's panel button view: sleek pulse icon + CPU % + alert indicator if stopped/hung.
pub fn view(app: &AppModel) -> Element<'_, Message> {
    let sp = cosmic::theme::spacing();
    let size = app.core.applet.suggested_size(true);
    let is_dark = app.is_dark();

    let has_alert = app.overview.unresponsive_or_stopped_count > 0 && app.config.warn_unresponsive_in_panel;
    let horizontal = app.core.applet.is_horizontal();

    let stroke_color = if has_alert {
        "#ef4444"
    } else if app.overview.total_cpu_percent > 75.0 {
        "#f59e0b"
    } else if is_dark {
        "#38bdf8"
    } else {
        "#0284c7"
    };

    let alert_color = if has_alert { Some("#ef4444") } else { None };
    let svg_code = pulse_icon_svg(stroke_color, alert_color);

    let icon_el = icon::icon(icon::from_svg_bytes(svg_code.into_bytes()))
        .size(size.0);

    let label_str = if has_alert {
        format!("!{}", app.overview.unresponsive_or_stopped_count)
    } else {
        format!("{:.0}%", app.overview.total_cpu_percent)
    };

    let label_el = text::body(label_str).size(12);

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
        .padding(if horizontal { [0, padding] } else { [padding, 0] })
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
                app.core.main_window_id().expect("applet always has a main window"),
                id,
                Some((POPUP_WIDTH as u32, 600)),
                None,
                None,
            );

            settings.positioner.size_limits = Limits::NONE
                .min_height(350.0)
                .min_width(POPUP_WIDTH)
                .max_width(POPUP_WIDTH)
                .max_height(MAX_HEIGHT);
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
    .limits(
        Limits::NONE
            .min_height(350.0)
            .min_width(POPUP_WIDTH)
            .max_width(POPUP_WIDTH)
            .max_height(MAX_HEIGHT),
    )
    .into()
}

pub fn destroy(id: Id) -> Task<Message> {
    cosmic::task::message(cosmic::Action::Cosmic(cosmic::app::Action::Surface(
        destroy_popup(id),
    )))
}
