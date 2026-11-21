//! The `list_tile` widget: one row of a list or a settings screen.

use std::marker::PhantomData;

use crate::element::{Element, ElementKind, RawElement, finish, style_builder};
use crate::message::Handler;
use crate::style::{Align, Color, EdgeInsets, Justify};
use crate::widgets::{Column, Container, Row, Text};

/// A row with a title, an optional second line, and room for something
/// before and after: an icon, an avatar, a switch.
///
/// ```ignore
/// ListTile::new("Wi-Fi")
///     .subtitle("Home network")
///     .leading(Image::icon(Icon::new().drawable("ic_wifi").sf("wifi")))
///     .trailing(Switch::new(self.wifi).on_toggle(Msg::Wifi))
///     .on_tap(Msg::OpenWifi)
/// ```
///
/// It is at least 56 points tall. A control in `leading` or `trailing`
/// keeps its own taps.
pub struct ListTile<M = ()>(
    pub(crate) RawElement,
    pub(crate) PhantomData<fn() -> M>,
    Parts,
);

struct Parts {
    title: String,
    subtitle: Option<String>,
    leading: Option<RawElement>,
    trailing: Option<RawElement>,
    on_tap: Handler<()>,
}

impl<M> ListTile<M> {
    /// Creates a tile with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        let mut shell = RawElement::container(ElementKind::Column);
        shell.style.padding = EdgeInsets::symmetric(16.0, 8.0);
        shell.style.min_height = Some(56.0);
        shell.style.justify_content = Justify::Center;
        Self(
            shell,
            PhantomData,
            Parts {
                title: title.into(),
                subtitle: None,
                leading: None,
                trailing: None,
                on_tap: Handler::none(),
            },
        )
    }

    /// Adds a second, quieter line under the title.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.2.subtitle = Some(subtitle.into());
        self
    }

    /// Puts an element before the title.
    pub fn leading(mut self, element: impl Into<Element<M>>) -> Self {
        self.2.leading = Some(element.into().into_raw());
        self
    }

    /// Puts an element after the title, at the end of the row.
    pub fn trailing(mut self, element: impl Into<Element<M>>) -> Self {
        self.2.trailing = Some(element.into().into_raw());
        self
    }

    /// Sends `message` when the tile is tapped.
    pub fn on_tap(mut self, message: M) -> Self
    where
        M: Clone + Send + Sync + 'static,
    {
        self.2.on_tap = Handler::new(move |()| message.clone());
        self
    }
}

impl<M: 'static> From<ListTile<M>> for Element<M> {
    fn from(component: ListTile<M>) -> Self {
        let ListTile(shell, _, parts) = component;
        let mut texts = Column::<M>::new()
            .spacing(2)
            .grow(1)
            .child(Text::new(parts.title));
        if let Some(subtitle) = parts.subtitle {
            texts = texts.child(Text::new(subtitle).size(14).color(Color::SECONDARY_TEXT));
        }
        let mut row = Row::<M>::new().spacing(16).align_items(Align::Center);
        if let Some(leading) = parts.leading {
            row = row.child(Element::from_raw(leading));
        }
        row = row.child(texts);
        if let Some(trailing) = parts.trailing {
            row = row.child(Element::from_raw(trailing));
        }
        finish(shell, Container::new().handler(parts.on_tap).child(row))
    }
}

style_builder!(ListTile);
