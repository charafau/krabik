//! The public, platform-independent UI description.
//!
//! Applications build a tree of [`Element`]s with the components in this module.
//! The tree is plain data: it holds no platform handles and no callbacks.

/// A length in logical points (dp on Android, points on iOS).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Length {
    /// The size is derived from the content.
    #[default]
    Auto,
    /// An absolute size in logical points.
    Points(f32),
    /// A fraction of the parent's size, in the range `0.0..=1.0`.
    Percent(f32),
}

/// A value that can be used as a distance in logical points.
///
/// Implementing this for `f32` and `i32` lets both `.padding(16.0)` and
/// `.padding(16)` compile.
pub trait IntoPoints {
    /// Converts the value into logical points.
    fn into_points(self) -> f32;
}

impl IntoPoints for f32 {
    fn into_points(self) -> f32 {
        self
    }
}

impl IntoPoints for i32 {
    fn into_points(self) -> f32 {
        self as f32
    }
}

/// Padding or margin around a node.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EdgeInsets {
    /// Space above the node.
    pub top: f32,
    /// Space to the right of the node.
    pub right: f32,
    /// Space below the node.
    pub bottom: f32,
    /// Space to the left of the node.
    pub left: f32,
}

impl EdgeInsets {
    /// No space on any side.
    pub const ZERO: Self = Self {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    };

    /// The same space on every side.
    pub fn all(value: impl IntoPoints) -> Self {
        let value = value.into_points();
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    /// The same space on the horizontal and vertical axes.
    pub fn symmetric(horizontal: impl IntoPoints, vertical: impl IntoPoints) -> Self {
        let horizontal = horizontal.into_points();
        let vertical = vertical.into_points();
        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    /// Space on each side, clockwise from the top.
    pub const fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Whether every side is zero.
    pub fn is_zero(&self) -> bool {
        *self == Self::ZERO
    }
}

/// Cross-axis alignment of a container's children.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    /// Pack children at the start of the cross axis.
    Start,
    /// Center children on the cross axis.
    Center,
    /// Pack children at the end of the cross axis.
    End,
    /// Stretch children across the cross axis.
    #[default]
    Stretch,
}

/// Main-axis distribution of a container's children.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Justify {
    /// Pack children at the start of the main axis.
    #[default]
    Start,
    /// Center children on the main axis.
    Center,
    /// Pack children at the end of the main axis.
    End,
    /// Spread children evenly, with no space at the edges.
    SpaceBetween,
}

/// Font weight of a text-bearing node.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FontWeight {
    /// Regular text.
    #[default]
    Normal,
    /// Emphasized text.
    Bold,
}

/// Platform-independent properties of a text node.
#[derive(Clone, Debug, PartialEq)]
pub struct TextProps {
    /// The text to display.
    pub content: String,
    /// Font size in logical points.
    pub size: f32,
    /// ARGB text color.
    pub color: u32,
    /// Font weight.
    pub weight: FontWeight,
}

impl TextProps {
    /// Creates text with the default size and color.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            size: 16.0,
            color: 0xFF00_0000,
            weight: FontWeight::Normal,
        }
    }
}

/// Platform-independent properties of a button node.
#[derive(Clone, Debug, PartialEq)]
pub struct ButtonProps {
    /// The label shown on the button.
    pub label: String,
    /// The action dispatched when the button is pressed.
    pub action: &'static str,
    /// Font size in logical points.
    pub size: f32,
    /// ARGB text color.
    pub color: u32,
    /// Font weight.
    pub weight: FontWeight,
}

impl ButtonProps {
    /// Creates a button with the given label and no action.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            action: "",
            size: 16.0,
            color: 0xFF00_0000,
            weight: FontWeight::Normal,
        }
    }
}

/// A border drawn around a node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Border {
    /// The stroke width in logical points.
    pub width: f32,
    /// The ARGB stroke color.
    pub color: u32,
}

impl Border {
    /// Creates a border with the given width and color.
    pub const fn new(width: f32, color: u32) -> Self {
        Self { width, color }
    }
}

/// Platform-independent properties of a text input node.
#[derive(Clone, Debug, PartialEq)]
pub struct TextFieldProps {
    /// The current text of the input.
    pub value: String,
    /// Hint shown while the input is empty.
    pub placeholder: String,
    /// The action dispatched whenever the text changes.
    pub action: &'static str,
    /// Font size in logical points.
    pub size: f32,
    /// ARGB text color.
    pub color: u32,
    /// Font weight.
    pub weight: FontWeight,
}

impl TextFieldProps {
    /// Creates an empty input with the default size and color.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            placeholder: String::new(),
            action: "",
            size: 16.0,
            color: 0xFF00_0000,
            weight: FontWeight::Normal,
        }
    }
}

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

/// The layout and decoration properties shared by every component.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    /// Space between the node's border and its children.
    pub padding: EdgeInsets,
    /// Space around the node.
    pub margin: EdgeInsets,
    /// Width of the node.
    pub width: Length,
    /// Height of the node.
    pub height: Length,
    /// Space between children.
    pub gap: f32,
    /// Share of the remaining main-axis space the node claims.
    pub grow: f32,
    /// Cross-axis alignment of the node's children.
    pub align_items: Align,
    /// Main-axis distribution of the node's children.
    pub justify_content: Justify,
    /// ARGB background color, if any.
    pub background: Option<u32>,
    /// Radius of the node's rounded corners in logical points.
    pub corner_radius: f32,
    /// Border drawn around the node, if any.
    pub border: Option<Border>,
    /// Opacity of the node and its children, in the range `0.0..=1.0`.
    pub opacity: Option<f32>,
}

impl Style {
    /// Creates the default style.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the node needs a native view to draw its decoration.
    ///
    /// Native widgets always have a view of their own; this only matters for
    /// layout nodes, which stay virtual unless they actually paint something.
    pub fn needs_view(&self) -> bool {
        self.background.is_some()
            || self.corner_radius > 0.0
            || self.border.is_some()
            || self.opacity.is_some_and(|opacity| opacity < 1.0)
    }
}

/// The component-specific part of an [`Element`].
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ElementKind {
    Column,
    Row,
    Spacer,
    Text(TextProps),
    Button(ButtonProps),
    TextField(TextFieldProps),
    Image(ImageProps),
    Scroll(ScrollProps),
}

impl ElementKind {
    /// A stable tag used when hashing node identifiers.
    ///
    /// The scroll axis is part of the tag: switching it rebuilds the subtree,
    /// because the native scroll container changes class.
    pub(crate) fn tag(&self) -> u64 {
        match self {
            Self::Column => 1,
            Self::Row => 2,
            Self::Spacer => 3,
            Self::Text(_) => 4,
            Self::Button(_) => 5,
            Self::TextField(_) => 6,
            Self::Image(_) => 7,
            Self::Scroll(props) => match props.axis {
                Axis::Vertical => 8,
                Axis::Horizontal => 9,
            },
        }
    }
}

/// A node of the declarative UI description.
///
/// Components such as [`Column`] and [`Text`] convert into an `Element` and can
/// therefore be nested with [`Column::child`] and [`Row::child`].
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub(crate) kind: ElementKind,
    pub(crate) style: Style,
    pub(crate) key: Option<&'static str>,
    pub(crate) children: Vec<Element>,
}

impl Element {
    /// Creates a container element.
    pub(crate) fn container(kind: ElementKind) -> Self {
        Self {
            kind,
            style: Style::default(),
            key: None,
            children: Vec::new(),
        }
    }

    /// Creates a leaf element.
    pub(crate) fn leaf(kind: ElementKind) -> Self {
        Self {
            kind,
            style: Style::default(),
            key: None,
            children: Vec::new(),
        }
    }

    /// The children of the element.
    pub fn children(&self) -> &[Element] {
        &self.children
    }

    /// The key that identifies the element among its siblings.
    pub fn key(&self) -> Option<&'static str> {
        self.key
    }
}

/// Adds the shared style builder methods to a component.
macro_rules! style_builder {
    ($component:ty) => {
        impl $component {
            /// Sets uniform padding.
            pub fn padding(mut self, value: impl IntoPoints) -> Self {
                self.0.style.padding = EdgeInsets::all(value.into_points());
                self
            }

            /// Sets padding on each side.
            pub fn padding_each(mut self, value: EdgeInsets) -> Self {
                self.0.style.padding = value;
                self
            }

            /// Sets uniform margin.
            pub fn margin(mut self, value: impl IntoPoints) -> Self {
                self.0.style.margin = EdgeInsets::all(value.into_points());
                self
            }

            /// Sets margin on each side.
            pub fn margin_each(mut self, value: EdgeInsets) -> Self {
                self.0.style.margin = value;
                self
            }

            /// Sets a fixed width in logical points.
            pub fn width(mut self, value: impl IntoPoints) -> Self {
                self.0.style.width = Length::Points(value.into_points());
                self
            }

            /// Sets the width as a fraction of the parent's width.
            pub fn width_percent(mut self, value: impl IntoPoints) -> Self {
                self.0.style.width = Length::Percent(value.into_points());
                self
            }

            /// Sets a fixed height in logical points.
            pub fn height(mut self, value: impl IntoPoints) -> Self {
                self.0.style.height = Length::Points(value.into_points());
                self
            }

            /// Sets the height as a fraction of the parent's height.
            pub fn height_percent(mut self, value: impl IntoPoints) -> Self {
                self.0.style.height = Length::Percent(value.into_points());
                self
            }

            /// Lets the node grow into the remaining main-axis space.
            pub fn grow(mut self, value: impl IntoPoints) -> Self {
                self.0.style.grow = value.into_points();
                self
            }

            /// Sets the cross-axis alignment of the node's children.
            pub fn align_items(mut self, value: Align) -> Self {
                self.0.style.align_items = value;
                self
            }

            /// Sets the main-axis distribution of the node's children.
            pub fn justify_content(mut self, value: Justify) -> Self {
                self.0.style.justify_content = value;
                self
            }

            /// Sets the background color as ARGB.
            pub fn background(mut self, color: u32) -> Self {
                self.0.style.background = Some(color);
                self
            }

            /// Rounds the corners of the node's background and border.
            pub fn corner_radius(mut self, radius: impl IntoPoints) -> Self {
                self.0.style.corner_radius = radius.into_points();
                self
            }

            /// Draws a border around the node.
            pub fn border(mut self, width: impl IntoPoints, color: u32) -> Self {
                self.0.style.border = Some(Border::new(width.into_points(), color));
                self
            }

            /// Sets the opacity of the node and its children.
            pub fn opacity(mut self, opacity: f32) -> Self {
                self.0.style.opacity = Some(opacity.clamp(0.0, 1.0));
                self
            }

            /// Identifies the node among its siblings across rebuilds.
            pub fn key(mut self, key: &'static str) -> Self {
                self.0.key = Some(key);
                self
            }
        }
    };
}

/// Adds the shared text styling methods to a text-bearing component.
macro_rules! text_builder {
    ($component:ty) => {
        impl $component {
            /// Sets the font size in logical points.
            pub fn size(mut self, size: impl IntoPoints) -> Self {
                self.0.set_text_size(size.into_points());
                self
            }

            /// Sets the ARGB text color.
            pub fn color(mut self, color: u32) -> Self {
                self.0.set_text_color(color);
                self
            }

            /// Renders the text in bold.
            pub fn bold(mut self) -> Self {
                self.0.set_text_weight(FontWeight::Bold);
                self
            }
        }
    };
}

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

/// A run of text.
#[derive(Clone, Debug, PartialEq)]
pub struct Text(pub(crate) Element);

impl Text {
    /// Creates a text node.
    pub fn new(content: impl Into<String>) -> Self {
        Self(Element::leaf(ElementKind::Text(TextProps::new(content))))
    }
}

impl From<Text> for Element {
    fn from(component: Text) -> Self {
        component.0
    }
}

/// A native button.
#[derive(Clone, Debug, PartialEq)]
pub struct Button(pub(crate) Element);

impl Button {
    /// Creates a button.
    pub fn new(label: impl Into<String>) -> Self {
        Self(Element::leaf(ElementKind::Button(ButtonProps::new(label))))
    }

    /// Sets the action dispatched when the button is pressed.
    pub fn action(mut self, action: &'static str) -> Self {
        if let ElementKind::Button(props) = &mut self.0.kind {
            props.action = action;
        }
        self
    }
}

impl From<Button> for Element {
    fn from(component: Button) -> Self {
        component.0
    }
}

/// A native single-line text input.
#[derive(Clone, Debug, PartialEq)]
pub struct TextField(pub(crate) Element);

impl TextField {
    /// Creates an input with the given initial value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(Element::leaf(ElementKind::TextField(TextFieldProps::new(
            value,
        ))))
    }

    /// Sets the hint shown while the input is empty.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        if let ElementKind::TextField(props) = &mut self.0.kind {
            props.placeholder = text.into();
        }
        self
    }

    /// Sets the action dispatched whenever the text changes.
    pub fn action(mut self, action: &'static str) -> Self {
        if let ElementKind::TextField(props) = &mut self.0.kind {
            props.action = action;
        }
        self
    }

    /// The current text of the input.
    pub fn value(&self) -> Option<&str> {
        match &self.0.kind {
            ElementKind::TextField(props) => Some(&props.value),
            _ => None,
        }
    }
}

impl From<TextField> for Element {
    fn from(component: TextField) -> Self {
        component.0
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
        Self(Element::leaf(ElementKind::Image(ImageProps::new(source))))
    }

    /// Sets how the image fills its frame.
    pub fn scale(mut self, scale: ImageScale) -> Self {
        if let ElementKind::Image(props) = &mut self.0.kind {
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

impl Element {
    fn set_text_size(&mut self, size: f32) {
        match &mut self.kind {
            ElementKind::Text(props) => props.size = size,
            ElementKind::Button(props) => props.size = size,
            ElementKind::TextField(props) => props.size = size,
            _ => {}
        }
    }

    fn set_text_color(&mut self, color: u32) {
        match &mut self.kind {
            ElementKind::Text(props) => props.color = color,
            ElementKind::Button(props) => props.color = color,
            ElementKind::TextField(props) => props.color = color,
            _ => {}
        }
    }

    fn set_text_weight(&mut self, weight: FontWeight) {
        match &mut self.kind {
            ElementKind::Text(props) => props.weight = weight,
            ElementKind::Button(props) => props.weight = weight,
            ElementKind::TextField(props) => props.weight = weight,
            _ => {}
        }
    }
}

style_builder!(Column);
style_builder!(Row);
style_builder!(Spacer);
style_builder!(Text);
style_builder!(Button);
style_builder!(TextField);
style_builder!(Image);
style_builder!(ScrollView);
text_builder!(Text);
text_builder!(Button);
text_builder!(TextField);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn components_convert_into_elements() {
        let element: Element = Column::new()
            .spacing(8)
            .padding(16)
            .child(Text::new("Hello").bold())
            .child(Button::new("Click").action("click"))
            .into();

        assert_eq!(element.children().len(), 2);
        assert_eq!(element.style.gap, 8.0);
        assert_eq!(element.style.padding, EdgeInsets::all(16.0));

        let ElementKind::Text(text) = &element.children()[0].kind else {
            panic!("expected text");
        };
        assert_eq!(text.content, "Hello");
        assert_eq!(text.weight, FontWeight::Bold);

        let ElementKind::Button(button) = &element.children()[1].kind else {
            panic!("expected button");
        };
        assert_eq!(button.label, "Click");
        assert_eq!(button.action, "click");
    }

    #[test]
    fn spacer_grows_by_default() {
        let element: Element = Spacer::new().into();
        assert_eq!(element.style.grow, 1.0);
        assert!(element.children().is_empty());
    }

    #[test]
    fn keys_are_kept_on_the_element() {
        let element: Element = Text::new("a").key("first").into();
        assert_eq!(element.key(), Some("first"));
    }
}
