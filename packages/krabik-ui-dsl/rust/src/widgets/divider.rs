//! The `divider` widget: a thin line between pieces of content.

use std::marker::PhantomData;

use crate::element::{Element, ElementKind, RawElement, style_builder};
use crate::style::{Color, IntoPoints, Length};

/// A thin line that separates content.
///
/// In a column it is horizontal and as wide as the column. In a row use
/// [`vertical`](Self::vertical).
///
/// ```ignore
/// Column::new()
///     .child(Text::new("Inbox"))
///     .child(Divider::new().indent(16))
///     .child(Text::new("Archive"))
/// ```
pub struct Divider<M = ()>(pub(crate) RawElement, pub(crate) PhantomData<fn() -> M>);

impl<M> Divider<M> {
    /// Creates a horizontal line, one point thick, in [`Color::OUTLINE`].
    pub fn new() -> Self {
        let mut element = RawElement::leaf(ElementKind::Column);
        element.style.height = Length::Points(1.0);
        element.style.background = Some(Color::OUTLINE);
        Self(element, PhantomData)
    }

    /// Makes the line vertical, for a row. The row stretches it to its
    /// height.
    pub fn vertical(mut self) -> Self {
        let thickness = match self.0.style.height {
            Length::Points(thickness) => thickness,
            _ => 1.0,
        };
        self.0.style.height = Length::Auto;
        self.0.style.width = Length::Points(thickness);
        self
    }

    /// Sets how thick the line is.
    pub fn thickness(mut self, value: impl IntoPoints) -> Self {
        let value = Length::Points(value.into_points());
        if matches!(self.0.style.width, Length::Points(_)) {
            self.0.style.width = value;
        } else {
            self.0.style.height = value;
        }
        self
    }

    /// Leaves this much room before the line starts.
    pub fn indent(mut self, value: impl IntoPoints) -> Self {
        let value = value.into_points();
        if matches!(self.0.style.width, Length::Points(_)) {
            self.0.style.margin.top = value;
        } else {
            self.0.style.margin.left = value;
        }
        self
    }

    /// Sets the color of the line.
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.0.style.background = Some(color.into());
        self
    }
}

impl<M> Default for Divider<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M> From<Divider<M>> for Element<M> {
    fn from(component: Divider<M>) -> Self {
        Element::from_raw(component.0)
    }
}

style_builder!(Divider);
