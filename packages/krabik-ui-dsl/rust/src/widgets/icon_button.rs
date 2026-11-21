//! The `icon_button` widget: a button that is an icon.

use std::marker::PhantomData;

use crate::element::{Element, ElementKind, RawElement, finish, style_builder};
use crate::message::Handler;
use crate::style::{Align, Color, EdgeInsets, Icon, IntoPoints, Justify};
use crate::widgets::{Container, Image, Row, Text};

/// A button that shows an icon, alone or before a short text.
///
/// ```ignore
/// IconButton::new(Icon::new().drawable("ic_share").sf("square.and.arrow.up"), "Share")
///     .on_tap(Msg::Share)
/// ```
///
/// The second argument says what the button does. A screen reader announces
/// it, and UI tests find the button by it; it is not shown unless
/// [`text`](Self::text) asks for a visible label. The button is at least 44
/// points in each direction, whatever the size of the icon.
pub struct IconButton<M = ()>(
    pub(crate) RawElement,
    pub(crate) PhantomData<fn() -> M>,
    Parts,
);

struct Parts {
    icon: Icon,
    icon_size: f32,
    tint: Color,
    text: Option<String>,
    on_tap: Handler<()>,
}

impl<M> IconButton<M> {
    /// Creates a button with the given icon and what it means.
    pub fn new(icon: Icon, label: impl Into<String>) -> Self {
        let mut shell = RawElement::container(ElementKind::Column);
        shell.style.min_width = Some(44.0);
        shell.style.min_height = Some(44.0);
        shell.style.corner_radius = 22.0;
        shell.style.align_items = Align::Center;
        shell.style.justify_content = Justify::Center;
        shell.style.accessibility_label = Some(label.into());
        Self(
            shell,
            PhantomData,
            Parts {
                icon,
                icon_size: 24.0,
                tint: Color::TEXT,
                text: None,
                on_tap: Handler::none(),
            },
        )
    }

    /// Sets the side of the icon; 24 points unless set.
    pub fn icon_size(mut self, size: impl IntoPoints) -> Self {
        self.2.icon_size = size.into_points();
        self
    }

    /// Sets the color of the icon and of the text.
    pub fn tint(mut self, color: impl Into<Color>) -> Self {
        self.2.tint = color.into();
        self
    }

    /// Shows a text after the icon.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.2.text = Some(text.into());
        self.0.style.padding = EdgeInsets::symmetric(12.0, 0.0);
        self
    }

    /// Sends `message` when the button is tapped.
    pub fn on_tap(mut self, message: M) -> Self
    where
        M: Clone + Send + Sync + 'static,
    {
        self.2.on_tap = Handler::new(move |()| message.clone());
        self
    }

    /// Sets the erased handler; for widgets that are built from this one.
    pub(crate) fn handler(mut self, on_tap: Handler<()>) -> Self {
        self.2.on_tap = on_tap;
        self
    }
}

impl<M: 'static> From<IconButton<M>> for Element<M> {
    fn from(component: IconButton<M>) -> Self {
        let IconButton(shell, _, parts) = component;
        let icon = Image::<M>::icon(parts.icon)
            .size(parts.icon_size)
            .tint(parts.tint);
        let container = Container::new().handler(parts.on_tap);
        finish(
            shell,
            match parts.text {
                Some(text) => container.child(
                    Row::new()
                        .spacing(8)
                        .align_items(Align::Center)
                        .child(icon)
                        .child(Text::new(text).color(parts.tint)),
                ),
                None => container.child(icon),
            },
        )
    }
}

style_builder!(IconButton);
