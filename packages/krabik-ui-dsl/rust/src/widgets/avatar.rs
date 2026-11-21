//! The `avatar` widget: a round picture or a pair of initials.

use std::marker::PhantomData;

use crate::element::{Element, ElementKind, RawElement, finish, style_builder};
use crate::style::{Align, Color, IntoPoints, Justify, Length};
use crate::widgets::{Container, Image, ImageScale, ImageSource, Text};

/// A circle that shows a person: their picture, or their initials on a
/// color.
///
/// ```ignore
/// Avatar::image(ImageSource::url(&user.photo))
/// Avatar::initials("RW").radius(16)
/// ```
///
/// It is 40 points across unless [`radius`](Self::radius) says otherwise.
pub struct Avatar<M = ()>(
    pub(crate) RawElement,
    pub(crate) PhantomData<fn() -> M>,
    Parts,
);

enum Parts {
    Image(ImageSource),
    Initials { text: String, color: Color },
}

impl<M> Avatar<M> {
    fn with(parts: Parts) -> Self {
        let mut shell = RawElement::container(ElementKind::Column);
        shell.style.background = Some(Color::OUTLINE);
        shell.style.align_items = Align::Center;
        shell.style.justify_content = Justify::Center;
        Self(shell, PhantomData, parts).radius(20)
    }

    /// Creates an avatar that shows a picture, cropped to the circle.
    pub fn image(source: ImageSource) -> Self {
        Self::with(Parts::Image(source))
    }

    /// Creates an avatar that shows a short text, usually two letters.
    pub fn initials(text: impl Into<String>) -> Self {
        Self::with(Parts::Initials {
            text: text.into(),
            color: Color::TEXT,
        })
    }

    /// Sets the radius of the circle; the avatar is twice as wide.
    pub fn radius(mut self, radius: impl IntoPoints) -> Self {
        let radius = radius.into_points();
        self.0.style.width = Length::Points(radius * 2.0);
        self.0.style.height = Length::Points(radius * 2.0);
        self.0.style.corner_radius = radius;
        self
    }

    /// Sets the color of the initials.
    pub fn color(mut self, value: impl Into<Color>) -> Self {
        if let Parts::Initials { color, .. } = &mut self.2 {
            *color = value.into();
        }
        self
    }
}

impl<M: 'static> From<Avatar<M>> for Element<M> {
    fn from(component: Avatar<M>) -> Self {
        let Avatar(shell, _, parts) = component;
        match parts {
            // The image is the circle: a view clips its own content to its
            // rounded corners.
            Parts::Image(source) => finish(shell, Image::new(source).scale(ImageScale::Fill)),
            Parts::Initials { text, color } => {
                let diameter = match shell.style.width {
                    Length::Points(diameter) => diameter,
                    _ => 40.0,
                };
                finish(
                    shell,
                    Container::new()
                        .child(Text::new(text).size(diameter * 0.4).color(color).bold()),
                )
            }
        }
    }
}

style_builder!(Avatar);
