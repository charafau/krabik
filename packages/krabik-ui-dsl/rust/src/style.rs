//! Layout and decoration values shared by every widget.

/// Whether the device is showing its light or its dark appearance.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorScheme {
    /// Dark content on a light background.
    #[default]
    Light,
    /// Light content on a dark background.
    Dark,
}

/// A colour, with a value for the light appearance and one for the dark.
///
/// A plain ARGB number such as `0xFF21_96F3` converts into a colour that is
/// the same in both appearances, so `.background(0xFF21_96F3)` keeps working.
/// Use [`Color::adaptive`] or one of the named colours to follow the device:
///
/// ```
/// use krabik_ui_dsl::ui::Color;
///
/// const CARD: Color = Color::adaptive(0xFFF4_F4F4, 0xFF1E_1E1E);
/// let title = Color::TEXT;
/// # let _ = (CARD, title);
/// ```
///
/// The engine picks the value for the current appearance when it builds the
/// tree, so switching between light and dark is an ordinary update.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    /// The ARGB value in the light appearance.
    pub light: u32,
    /// The ARGB value in the dark appearance.
    pub dark: u32,
}

impl Color {
    /// The default colour of text.
    pub const TEXT: Self = Self::adaptive(0xFF00_0000, 0xFFE6_E1E5);
    /// Text that should stand back: captions, hints, secondary labels.
    pub const SECONDARY_TEXT: Self = Self::adaptive(0xFF66_6666, 0xFFA8_A8A8);
    /// The page background.
    pub const BACKGROUND: Self = Self::adaptive(0xFFFF_FFFF, 0xFF12_1212);
    /// A card or field that sits on the page background.
    pub const SURFACE: Self = Self::adaptive(0xFFF4_F4F4, 0xFF1E_1E1E);
    /// Borders and dividers.
    pub const OUTLINE: Self = Self::adaptive(0xFFD0_D0D0, 0xFF3A_3A3A);
    /// The default accent of sliders, progress indicators and spinners.
    pub const ACCENT: Self = Self::adaptive(0xFF21_96F3, 0xFF64_B5F6);

    /// The same ARGB value in both appearances.
    pub const fn fixed(argb: u32) -> Self {
        Self {
            light: argb,
            dark: argb,
        }
    }

    /// One ARGB value for the light appearance and one for the dark.
    pub const fn adaptive(light: u32, dark: u32) -> Self {
        Self { light, dark }
    }

    /// The colour with the value for `scheme` in both appearances.
    pub const fn resolve(self, scheme: ColorScheme) -> Self {
        match scheme {
            ColorScheme::Light => Self::fixed(self.light),
            ColorScheme::Dark => Self::fixed(self.dark),
        }
    }

    /// The ARGB value of a colour the engine has resolved.
    ///
    /// The renderers read colours from the tree, where both values are
    /// already the one for the current appearance.
    pub const fn argb(self) -> u32 {
        self.light
    }
}

impl From<u32> for Color {
    fn from(argb: u32) -> Self {
        Self::fixed(argb)
    }
}

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

/// Horizontal alignment of a text node inside its frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextAlign {
    /// Align the text to the leading edge.
    #[default]
    Start,
    /// Center the text horizontally.
    Center,
    /// Align the text to the trailing edge.
    End,
}

/// A border drawn around a node.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Border {
    /// The stroke width in logical points.
    pub width: f32,
    /// The stroke color.
    pub color: Color,
}

impl Border {
    /// Creates a border with the given width and color.
    pub fn new(width: f32, color: impl Into<Color>) -> Self {
        Self {
            width,
            color: color.into(),
        }
    }
}

/// The alpha multiplier applied to DSL-drawn decoration while a node is
/// disabled.
///
/// Native text and controls dim themselves; this only affects fills, gradients,
/// borders and shadows that the DSL paints.
pub const DISABLED_ALPHA: f32 = 0.4;

/// Dims a color when its node is disabled, preserving the RGB channels.
///
/// The explicit opacity of a node is applied separately, so a disabled node
/// keeps its own alpha and is dimmed on top of it.
pub fn dim_color(color: Color, enabled: bool) -> Color {
    if enabled {
        return color;
    }
    let dim = |argb: u32| {
        let alpha = (argb >> 24) & 0xFF;
        let alpha = (alpha as f32 * DISABLED_ALPHA).round() as u32;
        (argb & 0x00FF_FFFF) | (alpha << 24)
    };
    Color::adaptive(dim(color.light), dim(color.dark))
}

/// Per-corner radii of a node's rounded corners.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CornerRadii {
    /// Radius of the top-left corner.
    pub top_left: f32,
    /// Radius of the top-right corner.
    pub top_right: f32,
    /// Radius of the bottom-right corner.
    pub bottom_right: f32,
    /// Radius of the bottom-left corner.
    pub bottom_left: f32,
}

impl CornerRadii {
    /// Creates radii for each corner, clockwise from the top left.
    pub const fn new(top_left: f32, top_right: f32, bottom_right: f32, bottom_left: f32) -> Self {
        Self {
            top_left,
            top_right,
            bottom_right,
            bottom_left,
        }
    }

    /// The same radius on every corner.
    pub const fn all(radius: f32) -> Self {
        Self::new(radius, radius, radius, radius)
    }
}

/// The direction a background gradient runs in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GradientDirection {
    /// From the top edge to the bottom edge.
    #[default]
    Vertical,
    /// From the left edge to the right edge.
    Horizontal,
}

/// A two-color linear background gradient.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gradient {
    /// The color at the start edge.
    pub start: Color,
    /// The color at the end edge.
    pub end: Color,
    /// The axis the gradient runs along.
    pub direction: GradientDirection,
}

impl Gradient {
    /// Creates a vertical gradient between two colors.
    pub fn vertical(start: impl Into<Color>, end: impl Into<Color>) -> Self {
        Self {
            start: start.into(),
            end: end.into(),
            direction: GradientDirection::Vertical,
        }
    }

    /// Creates a horizontal gradient between two colors.
    pub fn horizontal(start: impl Into<Color>, end: impl Into<Color>) -> Self {
        Self {
            start: start.into(),
            end: end.into(),
            direction: GradientDirection::Horizontal,
        }
    }
}

/// A timing curve for implicit animations.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum AnimationCurve {
    /// Constant speed.
    Linear,
    /// Starts slowly and speeds up.
    EaseIn,
    /// Starts quickly and slows down.
    EaseOut,
    /// Starts and ends slowly.
    #[default]
    EaseInOut,
    /// A custom cubic Bezier curve, with control points in `0.0..=1.0`.
    CubicBezier {
        /// X coordinate of the first control point.
        x1: f32,
        /// Y coordinate of the first control point.
        y1: f32,
        /// X coordinate of the second control point.
        x2: f32,
        /// Y coordinate of the second control point.
        y2: f32,
    },
    /// A damped spring.
    Spring {
        /// Damping ratio. `1.0` is critically damped; less overshoots.
        damping: f32,
        /// Natural response time in seconds.
        response: f32,
    },
}

impl AnimationCurve {
    /// Creates a custom cubic Bezier curve.
    pub const fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self::CubicBezier { x1, y1, x2, y2 }
    }

    /// Creates a damped spring curve.
    pub const fn spring(damping: f32, response: f32) -> Self {
        Self::Spring { damping, response }
    }

    /// Packs the curve into the fixed-width integers shared with the Android renderer.
    ///
    /// The first value is the kind: `0` standard, `1` cubic Bezier, `2` spring.
    /// Fractional parameters use four decimal places.
    pub(crate) fn packed(&self) -> [i32; 5] {
        fn fixed(value: f32) -> i32 {
            (value * 10_000.0).round() as i32
        }

        match self {
            Self::EaseInOut => [0, 0, 0, 0, 0],
            Self::Linear => [0, 1, 0, 0, 0],
            Self::EaseIn => [0, 2, 0, 0, 0],
            Self::EaseOut => [0, 3, 0, 0, 0],
            Self::CubicBezier { x1, y1, x2, y2 } => {
                [1, fixed(*x1), fixed(*y1), fixed(*x2), fixed(*y2)]
            }
            Self::Spring { damping, response } => [2, fixed(*damping), fixed(*response), 0, 0],
        }
    }
}

/// An implicit animation applied when a node's frame or opacity changes.
#[derive(Clone, Debug, PartialEq)]
pub struct Animation {
    /// Duration in milliseconds. Zero disables the animation.
    pub duration_ms: u32,
    /// The timing curve.
    pub curve: AnimationCurve,
}

impl Animation {
    /// The duration used when only a curve is chosen.
    pub const DEFAULT_DURATION_MS: u32 = 250;

    /// Creates an animation with the given duration and an ease-in-out curve.
    pub const fn new(duration_ms: u32) -> Self {
        Self {
            duration_ms,
            curve: AnimationCurve::EaseInOut,
        }
    }

    /// Replaces the timing curve.
    pub fn curve(mut self, curve: AnimationCurve) -> Self {
        self.curve = curve;
        self
    }
}

impl Default for Animation {
    fn default() -> Self {
        Self::new(Self::DEFAULT_DURATION_MS)
    }
}

/// A fade-and-scale transition used when a node enters or leaves the tree.
#[derive(Clone, Debug, PartialEq)]
pub struct Transition {
    /// Duration in milliseconds. Zero disables the transition.
    pub duration_ms: u32,
    /// The timing curve.
    pub curve: AnimationCurve,
}

impl Transition {
    /// Creates a transition with the given duration and an ease-in-out curve.
    pub const fn new(duration_ms: u32) -> Self {
        Self {
            duration_ms,
            curve: AnimationCurve::EaseInOut,
        }
    }

    /// Replaces the timing curve.
    pub fn curve(mut self, curve: AnimationCurve) -> Self {
        self.curve = curve;
        self
    }
}

impl Default for Transition {
    fn default() -> Self {
        Self::new(Animation::DEFAULT_DURATION_MS)
    }
}

/// A platform icon reference.
///
/// Android names resolve to drawable resources; iOS names resolve to an SF
/// Symbol or an asset-catalog image. A shared name is used when the
/// platform-specific name is absent.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Icon {
    /// Fallback name used on both platforms.
    pub shared: Option<String>,
    /// Android drawable resource name.
    pub android: Option<String>,
    /// iOS SF Symbol or asset-catalog name.
    pub ios: Option<String>,
}

impl Icon {
    /// Creates an empty icon.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a fallback name used on both platforms.
    pub fn shared(mut self, name: impl Into<String>) -> Self {
        self.shared = Some(name.into());
        self
    }

    /// Sets the Android drawable resource name.
    pub fn drawable(mut self, name: impl Into<String>) -> Self {
        self.android = Some(name.into());
        self
    }

    /// Sets the iOS SF Symbol or asset-catalog name.
    pub fn sf(mut self, name: impl Into<String>) -> Self {
        self.ios = Some(name.into());
        self
    }

    /// The name Android should resolve.
    pub fn android_name(&self) -> Option<&str> {
        self.android.as_deref().or(self.shared.as_deref())
    }

    /// The name iOS should resolve.
    pub fn ios_name(&self) -> Option<&str> {
        self.ios.as_deref().or(self.shared.as_deref())
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
    /// Width-to-height ratio, when the node derives one dimension from the other.
    pub aspect_ratio: Option<f32>,
    /// Space between children.
    pub gap: f32,
    /// Share of the remaining main-axis space the node claims.
    pub grow: f32,
    /// Cross-axis alignment of the node's children.
    pub align_items: Align,
    /// Main-axis distribution of the node's children.
    pub justify_content: Justify,
    /// Background color, if any.
    pub background: Option<Color>,
    /// Background gradient, if any. Takes precedence over `background`.
    pub gradient: Option<Gradient>,
    /// Radius of the node's rounded corners in logical points.
    pub corner_radius: f32,
    /// Per-corner radii, overriding `corner_radius` when set.
    pub corner_radii: Option<CornerRadii>,
    /// Border drawn around the node, if any.
    pub border: Option<Border>,
    /// Opacity of the node and its children, in the range `0.0..=1.0`.
    pub opacity: Option<f32>,
    /// Shadow depth in logical points, or zero for no elevation.
    pub elevation: f32,
    /// Whether the node accepts input. `None` leaves the platform default.
    pub enabled: Option<bool>,
    /// Implicit animation for frame and decoration changes, if any.
    pub animation: Option<Animation>,
    /// Fade-and-scale transition when the node is created, if any.
    pub enter: Option<Transition>,
    /// Fade-and-scale transition applied before the node is removed, if any.
    pub exit: Option<Transition>,
    /// Accessibility label announced by screen readers, if any.
    pub accessibility_label: Option<String>,
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
            || self.gradient.is_some()
            || self.corner_radius > 0.0
            || self.corner_radii.is_some()
            || self.border.is_some()
            || self.opacity.is_some_and(|opacity| opacity < 1.0)
            || self.elevation > 0.0
    }

    /// The corner radii the node draws with.
    pub fn corner_radii(&self) -> CornerRadii {
        self.corner_radii
            .unwrap_or_else(|| CornerRadii::all(self.corner_radius))
    }
}

impl Style {
    /// Picks the value for `scheme` in every colour of the style.
    pub(crate) fn resolve_colors(&mut self, scheme: ColorScheme) {
        if let Some(background) = &mut self.background {
            *background = background.resolve(scheme);
        }
        if let Some(gradient) = &mut self.gradient {
            gradient.start = gradient.start.resolve(scheme);
            gradient.end = gradient.end.resolve(scheme);
        }
        if let Some(border) = &mut self.border {
            border.color = border.color.resolve(scheme);
        }
    }
}
