//! The `scroll` widget: properties and builder.

use std::marker::PhantomData;

use crate::element::{Element, ElementKind, RawElement, style_builder};

/// The axis a scroll container scrolls along.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Axis {
    /// Scroll vertically.
    #[default]
    Vertical,
    /// Scroll horizontally.
    Horizontal,
}

/// Platform-independent properties of a scroll container.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScrollProps {
    /// The axis the container scrolls along.
    pub axis: Axis,
}

/// A native scroll container.
///
/// Give it a bounded size (`.grow(1)`, a fixed height, or a parent-provided
/// constraint) so its content can actually overflow.
pub struct ScrollView<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> ScrollView<M> {
    /// Creates a vertical scroll container.
    pub fn new() -> Self {
        Self(
            RawElement::container(ElementKind::Scroll(ScrollProps {
                axis: Axis::Vertical,
            })),
            PhantomData,
        )
    }

    /// Creates a horizontal scroll container.
    pub fn horizontal() -> Self {
        Self(
            RawElement::container(ElementKind::Scroll(ScrollProps {
                axis: Axis::Horizontal,
            })),
            PhantomData,
        )
    }

    /// The axis the container scrolls along.
    pub fn axis(&self) -> Axis {
        match &self.0.kind {
            ElementKind::Scroll(props) => props.axis,
            _ => Axis::Vertical,
        }
    }

    /// Appends a child.
    pub fn child(mut self, child: impl Into<Element<M>>) -> Self {
        self.0.children.push(child.into().into_raw());
        self
    }
}

impl<M> Default for ScrollView<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> From<ScrollView<M>> for Element<M> {
    fn from(component: ScrollView<M>) -> Self {
        Element::from_raw(component.0)
    }
}

style_builder!(ScrollView);
