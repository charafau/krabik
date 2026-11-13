//! The `containers` widgets: properties and builder.

use crate::element::{Element, ElementKind, style_builder};
use crate::style::IntoPoints;

/// A vertical container.
#[derive(Clone, Debug, PartialEq)]
pub struct Column(pub(crate) Element);

impl Column {
    /// Creates an empty column.
    pub fn new() -> Self {
        Self(Element::container(ElementKind::Column))
    }

    /// Sets the space between children.
    pub fn spacing(mut self, value: impl IntoPoints) -> Self {
        self.0.style.gap = value.into_points();
        self
    }

    /// Appends a child.
    pub fn child(mut self, child: impl Into<Element>) -> Self {
        self.0.children.push(child.into());
        self
    }
}

impl Default for Column {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Column> for Element {
    fn from(component: Column) -> Self {
        component.0
    }
}

/// A horizontal container.
#[derive(Clone, Debug, PartialEq)]
pub struct Row(pub(crate) Element);

impl Row {
    /// Creates an empty row.
    pub fn new() -> Self {
        Self(Element::container(ElementKind::Row))
    }

    /// Sets the space between children.
    pub fn spacing(mut self, value: impl IntoPoints) -> Self {
        self.0.style.gap = value.into_points();
        self
    }

    /// Appends a child.
    pub fn child(mut self, child: impl Into<Element>) -> Self {
        self.0.children.push(child.into());
        self
    }
}

impl Default for Row {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Row> for Element {
    fn from(component: Row) -> Self {
        component.0
    }
}

/// Empty space that grows to fill the available room.
#[derive(Clone, Debug, PartialEq)]
pub struct Spacer(pub(crate) Element);

impl Spacer {
    /// Creates a spacer that grows into the remaining space.
    pub fn new() -> Self {
        let mut element = Element::leaf(ElementKind::Spacer);
        element.style.grow = 1.0;
        Self(element)
    }
}

impl Default for Spacer {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Spacer> for Element {
    fn from(component: Spacer) -> Self {
        component.0
    }
}

style_builder!(Column);
style_builder!(Row);
style_builder!(Spacer);
