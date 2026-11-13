//! The `containers` widgets: properties and builder.

use std::marker::PhantomData;

use crate::element::{Element, ElementKind, RawElement, style_builder};
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

style_builder!(Column);
style_builder!(Row);
style_builder!(Spacer);
