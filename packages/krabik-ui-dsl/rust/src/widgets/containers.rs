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

/// A row that continues on a new line when its children do not fit.
///
/// ```ignore
/// Wrap::new()
///     .spacing(8)
///     .run_spacing(8)
///     .child(Chip::new("Rust"))
///     .child(Chip::new("Kotlin"))
///     .child(Chip::new("Swift"))
/// ```
///
/// It needs a width to wrap at, which it has wherever a column would stretch
/// it. Each line is as tall as its tallest child.
pub struct Wrap<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> Wrap<M> {
    /// Creates an empty wrapping row.
    pub fn new() -> Self {
        let mut element = RawElement::container(ElementKind::Row);
        element.style.wrap = true;
        // A child keeps its own height in a line that is taller.
        element.style.align_items = crate::style::Align::Start;
        Self(element, PhantomData)
    }

    /// Sets the space between children on a line, and between lines unless
    /// [`run_spacing`](Self::run_spacing) says otherwise.
    pub fn spacing(mut self, value: impl IntoPoints) -> Self {
        self.0.style.gap = value.into_points();
        self
    }

    /// Sets the space between lines.
    pub fn run_spacing(mut self, value: impl IntoPoints) -> Self {
        self.0.style.run_gap = Some(value.into_points());
        self
    }

    /// Appends a child.
    pub fn child(mut self, child: impl Into<Element<M>>) -> Self {
        self.0.children.push(child.into().into_raw());
        self
    }
}

impl<M> Default for Wrap<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> From<Wrap<M>> for Element<M> {
    fn from(component: Wrap<M>) -> Self {
        Element::from_raw(component.0)
    }
}

/// A container whose children lie on top of each other, the first at the
/// back.
///
/// The stack is as large as its largest child, unless it is given a size.
/// Every child fills it by default; `.align_items(Align::Center)` (or
/// `Start`, `End`) keeps the children at their own size and places them
/// there on both axes. A child with
/// [`.positioned(Inset)`](Container::positioned) is held at distances from the
/// stack's edges instead, and does not count for the stack's size. Another
/// way to put a child in a corner is to let it fill the stack and arrange
/// its content with rows, columns and spacers:
///
/// ```ignore
/// Stack::new()
///     .child(Image::new(ImageSource::asset("cover.webp")))
///     .child(
///         Column::new()
///             .padding(12)
///             .child(Spacer::new())
///             .child(Text::new("Caption").color(Color::fixed(0xFFFF_FFFF))),
///     )
/// ```
pub struct Stack<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> Stack<M> {
    /// Creates an empty stack.
    pub fn new() -> Self {
        Self(RawElement::container(ElementKind::Stack), PhantomData)
    }

    /// Adds a child on top of the ones added before.
    pub fn child(mut self, child: impl Into<Element<M>>) -> Self {
        self.0.children.push(child.into().into_raw());
        self
    }
}

impl<M> Default for Stack<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> From<Stack<M>> for Element<M> {
    fn from(component: Stack<M>) -> Self {
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
    /// Sends its message when the container is pressed and held.
    pub on_long_press: Handler<()>,
    /// The menu the container opens, on a tap or on a long press.
    pub menu: Option<crate::widgets::MenuProps>,
}

impl ContainerProps {
    pub(crate) fn map_messages(&mut self, wrap: &crate::message::MessageMap) {
        self.on_tap.map(wrap);
        self.on_long_press.map(wrap);
        if let Some(menu) = &mut self.menu {
            menu.map_messages(wrap);
        }
    }
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

    /// Sets the erased tap handler; for widgets that are built from a
    /// container.
    pub(crate) fn handler(mut self, on_tap: Handler<()>) -> Self {
        if let ElementKind::Container(props) = &mut self.0.kind {
            props.on_tap = on_tap;
        }
        self
    }

    /// Sends `message` when the container is pressed and held.
    pub fn on_long_press(mut self, message: M) -> Self
    where
        M: Clone + Send + Sync + 'static,
    {
        if let ElementKind::Container(props) = &mut self.0.kind {
            props.on_long_press = Handler::new(move |()| message.clone());
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
style_builder!(Stack);
style_builder!(Column);
style_builder!(Row);
style_builder!(Wrap);
style_builder!(Spacer);
