//! The screens the app can show.

mod demos;
mod main;

use krabik_ui_android::constants::*;
use krabik_ui_android::ui::{
    Button, EdgeInsets, Element, Length, Linear, Orientation, Placed, Text,
};

use super::Screen;
use super::widget::Action;

/// A heading inside a demo's preview card.
pub(super) fn heading(value: &'static str) -> Placed {
    Placed::new(Text::new(value).size(22.0).color(COLOR_TEXT_PRIMARY))
        .margins(EdgeInsets::bottom(6.0))
}

/// The explanatory paragraph under a demo heading.
pub(super) fn description(value: &'static str) -> Placed {
    Placed::new(Text::new(value).size(14.0).color(COLOR_TEXT_PRIMARY))
        .margins(EdgeInsets::bottom(12.0))
}

/// A small footnote under a demo's widgets.
pub(super) fn note(value: &'static str) -> Placed {
    Placed::new(Text::new(value).size(12.0).color(COLOR_TEXT_SECONDARY))
        .margins(EdgeInsets::top(4.0))
}

/// The Back button and title shown at the top of a demo screen.
pub(super) fn header(title: &'static str) -> Placed {
    Placed::new(Linear {
        orientation: Orientation::Horizontal,
        padding: EdgeInsets::ZERO,
        background: None,
        children: vec![
            Placed::new(
                Button::new("Back")
                    .size(14.0)
                    .all_caps(false)
                    .action(Action::Back.tag()),
            )
            .height(Length::Dp(44.0))
            .margins(EdgeInsets::right(12.0)),
            Placed::new(
                Text::new(title)
                    .size(20.0)
                    .color(COLOR_TEXT_PRIMARY)
                    .gravity(GRAVITY_CENTER_VERTICAL),
            )
            .margins(EdgeInsets::top(2.0)),
        ],
    })
    .margins(EdgeInsets::bottom(12.0))
}

/// Builds the tree for `screen`.
pub fn element(screen: Screen) -> Element {
    match screen {
        Screen::Main => main::build(),
        Screen::Demo(kind) => demos::build(kind),
    }
}
