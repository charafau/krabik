//! The `containers` widgets: properties and builder.

use std::marker::PhantomData;

use crate::element::{Element, ElementKind, RawElement, style_builder};
use crate::message::Handler;
use crate::style::IntoPoints;

/// A vertical container.
pub struct Column<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> Column<M> {
    /// Creates an empty column.
    pub fn new() -> Self {
        Self(RawElement::container(ElementKind::Column), PhantomData)
    }

    /// Sets the space between children.
    pub fn spacing(mut self, value: impl IntoPoints) -> Self {
        self.0.style.gap = value.into_points();
        self
    }

    /// Appends a child.
    pub fn child(mut self, child: impl Into<Element<M>>) -> Self {
        self.0.children.push(child.into().into_raw());
        self
    }
}

impl<M> Default for Column<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> From<Column<M>> for Element<M> {
    fn from(component: Column<M>) -> Self {
        Element::from_raw(component.0)
    }
}

/// A horizontal container.
pub struct Row<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> Row<M> {
    /// Creates an empty row.
    pub fn new() -> Self {
        Self(RawElement::container(ElementKind::Row), PhantomData)
    }

    /// Sets the space between children.
    pub fn spacing(mut self, value: impl IntoPoints) -> Self {
        self.0.style.gap = value.into_points();
        self
    }

    /// Appends a child.
    pub fn child(mut self, child: impl Into<Element<M>>) -> Self {
        self.0.children.push(child.into().into_raw());
        self
    }
}

impl<M> Default for Row<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> From<Row<M>> for Element<M> {
    fn from(component: Row<M>) -> Self {
        Element::from_raw(component.0)
    }
}

/// Empty space that grows to fill the available room.
pub struct Spacer<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> Spacer<M> {
    /// Creates a spacer that grows into the remaining space.
    pub fn new() -> Self {
        let mut element = RawElement::leaf(ElementKind::Spacer);
        element.style.grow = 1.0;
        Self(element, PhantomData)
    }
}

impl<M> Default for Spacer<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> From<Spacer<M>> for Element<M> {
    fn from(component: Spacer<M>) -> Self {
        Element::from_raw(component.0)
    }
}

/// Platform-independent properties of a container node.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ContainerProps {
    /// Sends its message when the container is tapped.
    pub on_tap: Handler<()>,
}

/// A box around its content that can be tapped.
///
/// A container always has a native view, so it is the widget to reach for
/// when a whole card or a whole row of a list should react to a tap:
///
/// ```ignore
/// Container::new()
///     .padding(16)
///     .background(Color::SURFACE)
///     .corner_radius(12)
///     .on_tap(Msg::Open(note.id))
///     .child(Row::new().child(Text::new(&note.title)).child(Spacer::new()))
/// ```
///
/// It usually has one child. More than one are laid out as in a
/// [`Column`]. A control inside it, such as a [`Button`](crate::ui::Button),
/// keeps its own taps; the container receives the ones that land elsewhere.
pub struct Container<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> Container<M> {
    /// Creates an empty container.
    pub fn new() -> Self {
        Self(
            RawElement::container(ElementKind::Container(ContainerProps::default())),
            PhantomData,
        )
    }

    /// Appends a child.
    pub fn child(mut self, child: impl Into<Element<M>>) -> Self {
        self.0.children.push(child.into().into_raw());
        self
    }

    /// Sends `message` when the container is tapped.
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

impl<M> Default for Container<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> From<Container<M>> for Element<M> {
    fn from(component: Container<M>) -> Self {
        Element::from_raw(component.0)
    }
}

style_builder!(Container);
style_builder!(Column);
style_builder!(Row);
style_builder!(Spacer);
