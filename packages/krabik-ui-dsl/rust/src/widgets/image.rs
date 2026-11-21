//! The `image` widget: properties and builder.

use krabik_ui_dsl_macros::Widget;

use crate::engine::geometry::Size;
use crate::engine::measure::Measure;
use crate::style::{Color, Icon, IntoPoints, Length};
use crate::widgets::IntrinsicSize;

/// Where an image is loaded from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageSource {
    /// A file bundled in the platform application's assets.
    Asset(String),
    /// A remote image URL.
    Url(String),
    /// An icon the platform has by name: an Android drawable resource, an
    /// SF Symbol or an image of the iOS asset catalog.
    Icon(Icon),
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
#[derive(Clone, Debug, PartialEq, Widget)]
#[widget(
    name = Image,
    about = "A native image.",
    new = "Creates an image with the given source.\n\nGive the image an explicit width and height, or let a parent stretch it; intrinsic sizing is not available for remote images yet."
)]
pub struct ImageProps {
    /// The source of the image.
    #[widget(arg)]
    pub source: ImageSource,
    /// How the image fills its frame.
    #[widget(set)]
    pub scale: ImageScale,
    /// The one color an icon is drawn in. Other images ignore it.
    #[widget(set(some, into))]
    pub tint: Option<Color>,
}

/// The side of an icon that was given no size, in logical points.
pub(crate) const ICON_SIZE: f32 = 24.0;

impl<M> Image<M> {
    /// Creates an icon: a small image the platform has by name, drawn in one
    /// color.
    ///
    /// ```ignore
    /// Image::icon(Icon::new().drawable("ic_add").sf("plus"))
    ///     .size(20)
    ///     .tint(Color::ACCENT)
    /// ```
    ///
    /// It is 24 points square and in the text color unless
    /// [`size`](Self::size) and [`tint`](Self::tint) say otherwise. A name
    /// the platform does not have shows nothing. An icon that stands alone
    /// should say what it means with `.accessibility_label(..)`.
    pub fn icon(icon: Icon) -> Self {
        Self::new(ImageSource::Icon(icon)).tint(Color::TEXT)
    }

    /// Sets the width and the height to the same value.
    pub fn size(mut self, value: impl IntoPoints) -> Self {
        let value = Length::Points(value.into_points());
        self.0.style.width = value;
        self.0.style.height = value;
        self
    }
}

impl IntrinsicSize for ImageProps {
    /// The image's natural size, or zero while it is unknown (for example a
    /// remote image that has not loaded). The layout scales it to the
    /// node's style.
    fn intrinsic_size(&self, _max_width: Option<f32>, measurer: &dyn Measure) -> Size {
        if matches!(self.source, ImageSource::Icon(_)) {
            return Size::new(ICON_SIZE, ICON_SIZE);
        }
        match measurer.image_size(self) {
            Some(natural) if natural.width > 0.0 && natural.height > 0.0 => natural,
            _ => Size::ZERO,
        }
    }
}
