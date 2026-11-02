//! Layout primitives and the declarative element tree.
//!
//! Screens are plain Rust values. [`super::render::Renderer`] walks the tree once and produces the
//! matching native views. Only [`Metrics::from_context`] touches JNI; the tree itself is pure data.

pub mod components;
pub mod material;

#[cfg(test)]
mod tests;

use jni::objects::JObject;
use jni::sys::jint;
use jni::{Env, jni_sig, jni_str};

use super::constants::{MATCH_PARENT, WRAP_CONTENT};
use crate::env::EnvExt;
use crate::error::Result;
pub use components::*;
pub use material::*;

/// The screen density used to convert dp values to pixels.
#[derive(Clone, Copy, Debug)]
pub struct Metrics {
    pub density: f32,
}

impl Metrics {
    /// Reads the display density from an `Activity` or other `Context`.
    pub fn from_context(env: &mut Env<'_>, context: &JObject<'_>) -> Result<Self> {
        let resources = env.call_object(
            context,
            jni_str!("getResources"),
            jni_sig!("()Landroid/content/res/Resources;"),
            &[],
        )?;
        let display = env.call_object(
            &resources,
            jni_str!("getDisplayMetrics"),
            jni_sig!("()Landroid/util/DisplayMetrics;"),
            &[],
        )?;
        let density = env.field_float(&display, jni_str!("density"), jni_sig!("F"))?;
        Ok(Self { density })
    }

    pub fn dp(self, value: f32) -> jint {
        (value * self.density).round() as jint
    }
}

/// Orientation of a `LinearLayout`.
#[derive(Clone, Copy, Debug)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

/// A dimension in the tree. `Dp` is resolved against the screen density at render time.
#[derive(Clone, Copy, Debug)]
pub enum Length {
    MatchParent,
    WrapContent,
    Dp(f32),
}

impl Length {
    pub fn resolve(self, metrics: Metrics) -> jint {
        match self {
            Self::MatchParent => MATCH_PARENT,
            Self::WrapContent => WRAP_CONTENT,
            Self::Dp(value) => metrics.dp(value),
        }
    }
}

/// Spacing in dp.
#[derive(Clone, Copy, Debug)]
pub struct EdgeInsets {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

impl EdgeInsets {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    pub const fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn all(value: f32) -> Self {
        Self::new(value, value, value, value)
    }

    pub fn top(value: f32) -> Self {
        Self::new(0.0, value, 0.0, 0.0)
    }

    pub fn bottom(value: f32) -> Self {
        Self::new(0.0, 0.0, 0.0, value)
    }

    pub fn right(value: f32) -> Self {
        Self::new(0.0, 0.0, value, 0.0)
    }

    /// Returns `(left, top, right, bottom)` in pixels.
    pub fn resolve(self, metrics: Metrics) -> (jint, jint, jint, jint) {
        (
            metrics.dp(self.left),
            metrics.dp(self.top),
            metrics.dp(self.right),
            metrics.dp(self.bottom),
        )
    }
}

/// A node in the Rust-owned UI tree.
///
/// Layout containers and leaf widgets are variants, so a whole screen is a single value. The
/// renderer creates the matching native `View` for each variant.
pub enum Element {
    Scroll(Scroll),
    Linear(Linear),
    Recycler(Recycler),
    ViewPager(ViewPager),
    RadioGroup(RadioGroup),
    Text(Text),
    Button(Button),
    Image(Image),
    ImageButton(ImageButton),
    EditText(EditText),
    CheckBox(CheckBox),
    Switch(Switch),
    ToggleButton(ToggleButton),
    CheckedText(CheckedText),
    Progress(Progress),
    Seek(Seek),
    Rating(Rating),
    Spinner(Spinner),
    AutoComplete(AutoComplete),
    MultiAutoComplete(MultiAutoComplete),
    NumberPicker(NumberPicker),
    DatePicker,
    TimePicker,
    Chronometer,
    TextClock(TextClock),
    Calendar,
    AnalogClock,
    Search(Search),
    Toolbar(Toolbar),
    AppCompatToolbar(AppCompatToolbar),
    MaterialText(MaterialText),
    MaterialButton(MaterialButton),
    MaterialToggleGroup(MaterialButtonToggleGroup),
    MaterialCheckBox(MaterialCheckBox),
    MaterialRadioGroup(MaterialRadioGroup),
    MaterialSwitch(MaterialSwitch),
    MaterialTextField(MaterialTextField),
    MaterialDropdown(MaterialDropdown),
    MaterialSearchBar(MaterialSearchBar),
    MaterialSearchView(MaterialSearchView),
    MaterialChip(MaterialChip),
    MaterialChipGroup(MaterialChipGroup),
    MaterialCard(MaterialCard),
    MaterialToolbar(MaterialToolbar),
    MaterialAppBar(MaterialAppBar),
    MaterialCollapsingToolbar(MaterialCollapsingToolbar),
    MaterialBottomAppBar(MaterialBottomAppBar),
    MaterialFloatingToolbar(MaterialToolbarLayout),
    MaterialDockedToolbar(MaterialToolbarLayout),
    MaterialFab(MaterialFab),
    MaterialExtendedFab(MaterialExtendedFab),
    MaterialProgress(MaterialProgress),
    MaterialLoading(MaterialLoading),
    MaterialSlider(MaterialSlider),
    MaterialDivider(MaterialDivider),
    MaterialShapeableImage(MaterialShapeableImage),
    MaterialTabs(MaterialTabs),
    MaterialBottomNavigation(MaterialBottomNavigation),
    MaterialNavigationRail(MaterialNavigationRail),
    MaterialNavigation(MaterialNavigation),
    MaterialCarousel(MaterialCarousel),
    /// A `TextView` whose rendered view is retained so Rust can update it later.
    Status(Text),
}

macro_rules! element_from_components {
    ($($component:ty => $variant:ident),* $(,)?) => {
        $(
            impl From<$component> for Element {
                fn from(component: $component) -> Self {
                    Self::$variant(component)
                }
            }
        )*
    };
}

element_from_components! {
    Scroll => Scroll,
    Linear => Linear,
    Recycler => Recycler,
    ViewPager => ViewPager,
    RadioGroup => RadioGroup,
    Text => Text,
    Button => Button,
    Image => Image,
    ImageButton => ImageButton,
    EditText => EditText,
    CheckBox => CheckBox,
    Switch => Switch,
    ToggleButton => ToggleButton,
    CheckedText => CheckedText,
    Progress => Progress,
    Seek => Seek,
    Rating => Rating,
    Spinner => Spinner,
    AutoComplete => AutoComplete,
    MultiAutoComplete => MultiAutoComplete,
    NumberPicker => NumberPicker,
    TextClock => TextClock,
    Search => Search,
    Toolbar => Toolbar,
    AppCompatToolbar => AppCompatToolbar,
    MaterialText => MaterialText,
    MaterialButton => MaterialButton,
    MaterialButtonToggleGroup => MaterialToggleGroup,
    MaterialCheckBox => MaterialCheckBox,
    MaterialRadioGroup => MaterialRadioGroup,
    MaterialSwitch => MaterialSwitch,
    MaterialTextField => MaterialTextField,
    MaterialDropdown => MaterialDropdown,
    MaterialSearchBar => MaterialSearchBar,
    MaterialSearchView => MaterialSearchView,
    MaterialChip => MaterialChip,
    MaterialChipGroup => MaterialChipGroup,
    MaterialCard => MaterialCard,
    MaterialToolbar => MaterialToolbar,
    MaterialAppBar => MaterialAppBar,
    MaterialCollapsingToolbar => MaterialCollapsingToolbar,
    MaterialBottomAppBar => MaterialBottomAppBar,
    MaterialToolbarLayout => MaterialFloatingToolbar,
    MaterialFab => MaterialFab,
    MaterialExtendedFab => MaterialExtendedFab,
    MaterialProgress => MaterialProgress,
    MaterialLoading => MaterialLoading,
    MaterialSlider => MaterialSlider,
    MaterialDivider => MaterialDivider,
    MaterialShapeableImage => MaterialShapeableImage,
    MaterialTabs => MaterialTabs,
    MaterialBottomNavigation => MaterialBottomNavigation,
    MaterialNavigationRail => MaterialNavigationRail,
    MaterialNavigation => MaterialNavigation,
    MaterialCarousel => MaterialCarousel,
}

/// An [`Element`] plus the layout-parameter hints its parent needs.
///
/// Android containers size and space each child through `LayoutParams` that
/// belong to the parent-child pair rather than to the child itself. An
/// [`Element`] says *what* to render; `Placed` says *how the container should
/// lay it out*: an explicit width, an explicit height, and margins.
///
/// Containers that accept children — [`Scroll`], [`Linear`], [`MaterialCard`],
/// [`MaterialAppBar`], [`MaterialCollapsingToolbar`], [`MaterialBottomAppBar`],
/// and [`MaterialToolbarLayout`] — store them as `Vec<Placed>`, or
/// `Box<Placed>` for the single-child [`Scroll`]. The renderer converts each
/// entry into Android layout parameters when it builds the view tree.
///
/// # Defaults
///
/// Width and height are optional so that each container can supply its own
/// default:
///
/// - `width: None` uses the parent's preferred width: [`Length::MatchParent`]
///   inside a vertical [`Linear`] or a [`Scroll`], and [`Length::WrapContent`]
///   inside a horizontal [`Linear`].
/// - `height: None` always becomes [`Length::WrapContent`].
///
/// Margins are stored in dp and resolved against the screen density at render
/// time, like any other [`Length`] or [`EdgeInsets`] value.
///
/// ```text
/// Placed::new(Button::new("OK"))
///     .width(Length::MatchParent)
///     .margins(EdgeInsets::all(8.0))
/// ```
pub struct Placed {
    /// The component to render.
    pub element: Element,
    /// Explicit width, or `None` to use the parent container's default.
    pub width: Option<Length>,
    /// Explicit height, or `None` to fall back to [`Length::WrapContent`].
    pub height: Option<Length>,
    /// Space kept around this child, in dp.
    pub margins: EdgeInsets,
}

impl Placed {
    /// Creates a child that inherits its size from the parent and has no margins.
    pub fn new(element: impl Into<Element>) -> Self {
        Self {
            element: element.into(),
            width: None,
            height: None,
            margins: EdgeInsets::ZERO,
        }
    }

    /// Overrides the width the parent would otherwise choose.
    pub fn width(mut self, width: Length) -> Self {
        self.width = Some(width);
        self
    }

    /// Overrides the default [`Length::WrapContent`] height.
    pub fn height(mut self, height: Length) -> Self {
        self.height = Some(height);
        self
    }

    /// Sets the space kept around this child, in dp.
    pub fn margins(mut self, margins: EdgeInsets) -> Self {
        self.margins = margins;
        self
    }
}
