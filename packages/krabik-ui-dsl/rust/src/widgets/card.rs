//! The `card` widget: a raised surface around related content.

use std::marker::PhantomData;

use crate::element::{Element, ElementKind, RawElement, style_builder};
use crate::message::Handler;
use crate::style::{Color, EdgeInsets};
use crate::widgets::ContainerProps;

/// A rounded, slightly raised surface around related content.
///
/// It is a [`Container`](crate::ui::Container) with a look: the surface
/// color, a corner radius of 12, a padding of 16 and a small shadow. Every
/// one of them can be replaced with the usual style methods.
///
/// ```ignore
/// Card::new()
///     .on_tap(Msg::Open(note.id))
///     .child(Text::new(&note.title).bold())
///     .child(Text::new(&note.summary).color(Color::SECONDARY_TEXT))
/// ```
pub struct Card<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> Card<M> {
    /// Creates an empty card.
    pub fn new() -> Self {
        let mut element = RawElement::container(ElementKind::Container(ContainerProps::default()));
        element.style.background = Some(Color::SURFACE);
        element.style.corner_radius = 12.0;
        element.style.padding = EdgeInsets::all(16.0);
        element.style.elevation = 1.0;
        Self(element, PhantomData)
    }

    /// Appends a child. Children are laid out as in a column.
    pub fn child(mut self, child: impl Into<Element<M>>) -> Self {
        self.0.children.push(child.into().into_raw());
        self
    }

    /// Sets the space between children.
    pub fn spacing(mut self, value: impl crate::style::IntoPoints) -> Self {
        self.0.style.gap = value.into_points();
        self
    }

    /// Sends `message` when the card is tapped.
    pub fn on_tap(mut self, message: M) -> Self
    where
        M: Clone + Send + Sync + 'static,
    {
        if let ElementKind::Container(props) = &mut self.0.kind {
            props.on_tap = Handler::new(move |()| message.clone());
        }
        self
    }
}

impl<M> Default for Card<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> From<Card<M>> for Element<M> {
    fn from(component: Card<M>) -> Self {
        Element::from_raw(component.0)
    }
}

style_builder!(Card);
