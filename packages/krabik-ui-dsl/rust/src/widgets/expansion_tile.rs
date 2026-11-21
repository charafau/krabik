//! The `expansion_tile` widget: a header that shows or hides what is under it.

use std::marker::PhantomData;

use crate::element::{Element, ElementKind, RawElement, finish, style_builder};
use crate::message::Handler;
use crate::style::{Align, Color, EdgeInsets, Icon};
use crate::widgets::{Column, Container, Image, Row, Text};

/// A header row that shows or hides the content under it.
///
/// ```ignore
/// ExpansionTile::new("Advanced")
///     .expanded(self.advanced_open)
///     .on_toggle(Msg::ToggleAdvanced)
///     .child(Switch::new(self.verbose).on_toggle(Msg::Verbose))
/// ```
///
/// Whether it is open is the application's state: `on_toggle` only reports
/// the tap on the header. The content fades in when it opens.
pub struct ExpansionTile<M = ()>(
    pub(crate) RawElement,
    pub(crate) PhantomData<fn() -> M>,
    Parts,
);

struct Parts {
    title: String,
    subtitle: Option<String>,
    expanded: bool,
    on_toggle: Handler<()>,
    children: Vec<RawElement>,
}

impl<M> ExpansionTile<M> {
    /// Creates a closed tile with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self(
            RawElement::container(ElementKind::Column),
            PhantomData,
            Parts {
                title: title.into(),
                subtitle: None,
                expanded: false,
                on_toggle: Handler::none(),
                children: Vec::new(),
            },
        )
    }

    /// Adds a second, quieter line under the title.
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.2.subtitle = Some(subtitle.into());
        self
    }

    /// Shows the content while `expanded`.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.2.expanded = expanded;
        self
    }

    /// Sends `message` when the header is tapped.
    pub fn on_toggle(mut self, message: M) -> Self
    where
        M: Clone + Send + Sync + 'static,
    {
        self.2.on_toggle = Handler::new(move |()| message.clone());
        self
    }

    /// Appends a piece of the content.
    pub fn child(mut self, child: impl Into<Element<M>>) -> Self {
        self.2.children.push(child.into().into_raw());
        self
    }
}

impl<M: 'static> From<ExpansionTile<M>> for Element<M> {
    fn from(component: ExpansionTile<M>) -> Self {
        let ExpansionTile(shell, _, parts) = component;
        let mut texts = Column::<M>::new()
            .spacing(2)
            .grow(1)
            .child(Text::new(parts.title.clone()));
        if let Some(subtitle) = parts.subtitle {
            texts = texts.child(Text::new(subtitle).size(14).color(Color::SECONDARY_TEXT));
        }
        let arrow = if parts.expanded {
            Icon::chevron_up()
        } else {
            Icon::chevron_down()
        };
        let header = Container::new()
            .handler(parts.on_toggle)
            .padding_each(EdgeInsets::symmetric(16.0, 12.0))
            .min_height(56)
            .justify_content(crate::style::Justify::Center)
            .accessibility_label(parts.title)
            .key("header")
            .child(
                Row::new()
                    .spacing(16)
                    .align_items(Align::Center)
                    .child(texts)
                    .child(Image::icon(arrow).tint(Color::SECONDARY_TEXT)),
            );
        let mut column = Column::<M>::new().child(header);
        if parts.expanded {
            let mut body = Column::<M>::new()
                .spacing(8)
                .padding_each(EdgeInsets {
                    top: 0.0,
                    right: 16.0,
                    bottom: 12.0,
                    left: 16.0,
                })
                .animate_enter(150)
                .key("body");
            for child in parts.children {
                body = body.child(Element::from_raw(child));
            }
            column = column.child(body);
        }
        finish(shell, column)
    }
}

style_builder!(ExpansionTile);
