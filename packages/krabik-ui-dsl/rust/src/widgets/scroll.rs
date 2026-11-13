//! The `scroll` widget: properties and builder.

use crate::element::{Element, ElementKind, style_builder};

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
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollView(pub(crate) Element);

impl ScrollView {
    /// Creates a vertical scroll container.
    pub fn new() -> Self {
        Self(Element::container(ElementKind::Scroll(ScrollProps {
            axis: Axis::Vertical,
        })))
    }

    /// Creates a horizontal scroll container.
    pub fn horizontal() -> Self {
        Self(Element::container(ElementKind::Scroll(ScrollProps {
            axis: Axis::Horizontal,
        })))
    }

    /// The axis the container scrolls along.
    pub fn axis(&self) -> Axis {
        match &self.0.kind {
            ElementKind::Scroll(props) => props.axis,
            _ => Axis::Vertical,
        }
    }

    /// Appends a child.
    pub fn child(mut self, child: impl Into<Element>) -> Self {
        self.0.children.push(child.into());
        self
    }
}

impl Default for ScrollView {
    fn default() -> Self {
        Self::new()
    }
}

impl From<ScrollView> for Element {
    fn from(component: ScrollView) -> Self {
        component.0
    }
}

style_builder!(ScrollView);
