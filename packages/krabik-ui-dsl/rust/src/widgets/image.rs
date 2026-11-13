//! The `image` widget: properties and builder.

use crate::element::{Element, ElementKind, style_builder};
use crate::engine::geometry::Size;
use crate::engine::measure::Measure;
use crate::widgets::Widget;

/// Where an image is loaded from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageSource {
    /// A file bundled in the platform application's assets.
    Asset(String),
    /// A remote image URL.
    Url(String),
}

impl ImageSource {
    /// An image bundled with the application.
    pub fn asset(path: impl Into<String>) -> Self {
        Self::Asset(path.into())
    }

    /// An image loaded from a URL.
    pub fn url(url: impl Into<String>) -> Self {
        Self::Url(url.into())
    }
}

/// How an image fills its frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImageScale {
    /// Scale the image to fit inside the frame, preserving its aspect ratio.
    #[default]
    Fit,
    /// Scale the image to fill the frame, cropping the excess.
    Fill,
}

/// Platform-independent properties of an image node.
#[derive(Clone, Debug, PartialEq)]
pub struct ImageProps {
    /// The source of the image.
    pub source: ImageSource,
    /// How the image fills its frame.
    pub scale: ImageScale,
}

impl ImageProps {
    /// Creates an image with the given source.
    pub fn new(source: ImageSource) -> Self {
        Self {
            source,
            scale: ImageScale::Fit,
        }
    }
}

/// A native image.
#[derive(Clone, Debug, PartialEq)]
pub struct Image(pub(crate) Element);

impl Image {
    /// Creates an image with the given source.
    ///
    /// Give the image an explicit width and height, or let a parent stretch it;
    /// intrinsic sizing is not available for remote images yet.
    pub fn new(source: ImageSource) -> Self {
        Self(Element::leaf(ElementKind::Widget(Widget::Image(
            ImageProps::new(source),
        ))))
    }

    /// Sets how the image fills its frame.
    pub fn scale(mut self, scale: ImageScale) -> Self {
        if let ElementKind::Widget(Widget::Image(props)) = &mut self.0.kind {
            props.scale = scale;
        }
        self
    }
}

impl From<Image> for Element {
    fn from(component: Image) -> Self {
        component.0
    }
}

style_builder!(Image);

impl ImageProps {
    /// The image's natural size, or zero while it is unknown (for example a
    /// remote image that has not loaded). The layout scales it to the
    /// node's style.
    pub(crate) fn intrinsic_size(&self, measurer: &dyn Measure) -> Size {
        match measurer.image_size(self) {
            Some(natural) if natural.width > 0.0 && natural.height > 0.0 => natural,
            _ => Size::ZERO,
        }
    }
}
