//! The main screen: one button per supported widget.

use krabik_ui_android::constants::*;
use krabik_ui_android::ui::{
    Button, EdgeInsets, Element, Length, Linear, Orientation, Placed, Scroll, Text,
};

use super::super::widget::{Action, WidgetKind};

pub(super) fn build() -> Element {
    let mut children = vec![
        Placed::new(
            Text::new("Android widgets")
                .size(30.0)
                .color(COLOR_TEXT_PRIMARY)
                .gravity(GRAVITY_CENTER_HORIZONTAL),
        )
        .margins(EdgeInsets::bottom(4.0)),
        Placed::new(
            Text::new("Every widget below is a native Android view created from Rust through JNI.")
                .size(14.0)
                .color(COLOR_TEXT_SECONDARY)
                .gravity(GRAVITY_CENTER_HORIZONTAL),
        )
        .margins(EdgeInsets::bottom(20.0)),
        Placed::new(
            Text::new("Choose a widget")
                .size(12.0)
                .color(COLOR_TEXT_SECONDARY),
        )
        .margins(EdgeInsets::bottom(8.0)),
    ];

    children.extend(WidgetKind::ALL.iter().map(|&kind| {
        Placed::new(
            Button::new(kind.title())
                .size(16.0)
                .all_caps(false)
                .action(Action::Open(kind).tag()),
        )
        .width(Length::MatchParent)
        .height(Length::Dp(52.0))
        .margins(EdgeInsets::bottom(8.0))
    }));

    children.push(
        Placed::new(
            Text::new("Built with direct JNI. No XML layouts, Compose, or WebView.")
                .size(12.0)
                .color(COLOR_TEXT_SECONDARY)
                .gravity(GRAVITY_CENTER_HORIZONTAL),
        )
        .margins(EdgeInsets::top(12.0)),
    );

    Element::Scroll(Scroll {
        background: Some(COLOR_SCREEN_BACKGROUND),
        child: Box::new(
            Placed::new(Linear {
                orientation: Orientation::Vertical,
                padding: EdgeInsets::new(20.0, 56.0, 20.0, 32.0),
                background: None,
                children,
            })
            .width(Length::MatchParent),
        ),
    })
}
