//! The public, platform-independent UI description.
//!
//! Applications build a tree of [`Element`]s with the components in this module.
//! The tree is plain data: it holds no platform handles and no callbacks.

use std::borrow::Cow;
use std::fmt;
use std::sync::Arc;

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
    /// Horizontal alignment inside the node's frame.
    pub align: TextAlign,
    /// Maximum number of lines to draw, or unlimited when `None`.
    pub max_lines: Option<u32>,
    /// Whether the last visible line is truncated with an ellipsis.
    pub ellipsis: bool,
}

impl TextProps {
    /// Creates text with the default size and color.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            size: 16.0,
            color: 0xFF00_0000,
            weight: FontWeight::Normal,
            align: TextAlign::Start,
            max_lines: None,
            ellipsis: false,
        }
    }
}

/// Platform-independent properties of a button node.
#[derive(Clone, Debug, PartialEq)]
pub struct ButtonProps {
    /// The label shown on the button.
    pub label: String,
    /// The action dispatched when the button is pressed.
    pub action: Cow<'static, str>,
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
            action: Cow::Borrowed(""),
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

/// Which on-screen keyboard an input requests.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyboardType {
    /// Free-form text.
    #[default]
    Text,
    /// An email address.
    Email,
    /// A number.
    Number,
    /// A phone number.
    Phone,
}

/// The action key shown on the keyboard of an input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImeAction {
    /// Let the platform choose the action.
    #[default]
    Default,
    /// Show a done key.
    Done,
    /// Show a next key.
    Next,
    /// Show a search key.
    Search,
    /// Show a send key.
    Send,
}

/// Platform-independent properties of a text input node.
#[derive(Clone, Debug, PartialEq)]
pub struct TextFieldProps {
    /// The current text of the input.
    pub value: String,
    /// Hint shown while the input is empty.
    pub placeholder: String,
    /// The action dispatched whenever the text changes.
    pub action: Cow<'static, str>,
    /// The action dispatched when the user submits the input.
    pub submit_action: Cow<'static, str>,
    /// The action dispatched when the input gains or loses focus.
    pub focus_action: Cow<'static, str>,
    /// Whether the input accepts multiple lines of text.
    pub multiline: bool,
    /// Whether the text is obscured while typing.
    pub secure: bool,
    /// The keyboard the input requests.
    pub keyboard: KeyboardType,
    /// The action key shown on the keyboard.
    pub ime_action: ImeAction,
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
            action: Cow::Borrowed(""),
            submit_action: Cow::Borrowed(""),
            focus_action: Cow::Borrowed(""),
            multiline: false,
            secure: false,
            keyboard: KeyboardType::Text,
            ime_action: ImeAction::Default,
            size: 16.0,
            color: 0xFF00_0000,
            weight: FontWeight::Normal,
        }
    }
}

/// Platform-independent properties of a switch node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwitchProps {
    /// Whether the switch is on.
    pub checked: bool,
    /// The action dispatched whenever the switch is toggled.
    pub action: Cow<'static, str>,
}

impl SwitchProps {
    /// Creates a switch in the given state with no action.
    pub const fn new(checked: bool) -> Self {
        Self {
            checked,
            action: Cow::Borrowed(""),
        }
    }
}

/// Platform-independent properties of a checkbox or radio button.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckProps {
    /// The label shown next to the control.
    pub label: String,
    /// Whether the control is selected.
    pub checked: bool,
    /// The action dispatched whenever the control is toggled.
    pub action: Cow<'static, str>,
    /// Font size of the label in logical points.
    pub size: f32,
    /// ARGB color of the label.
    pub color: u32,
    /// Font weight of the label.
    pub weight: FontWeight,
}

impl CheckProps {
    /// Creates a control with the given label and state.
    pub fn new(label: impl Into<String>, checked: bool) -> Self {
        Self {
            label: label.into(),
            checked,
            action: Cow::Borrowed(""),
            size: 16.0,
            color: 0xFF00_0000,
            weight: FontWeight::Normal,
        }
    }
}

/// A calendar date in the device's local calendar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DateValue {
    /// Full year, for example `2026`.
    pub year: i32,
    /// Month in the range `1..=12`.
    pub month: u8,
    /// Day in the range `1..=31`.
    pub day: u8,
}

impl DateValue {
    /// Creates a date, clamping the month and day to their valid ranges.
    pub const fn new(year: i32, month: u8, day: u8) -> Self {
        Self {
            year,
            month: if month < 1 {
                1
            } else if month > 12 {
                12
            } else {
                month
            },
            day: if day < 1 {
                1
            } else if day > 31 {
                31
            } else {
                day
            },
        }
    }
}

impl fmt::Display for DateValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02}",
            self.year, self.month, self.day
        )
    }
}

/// A time of day in the device's local timezone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TimeValue {
    /// Hour in the range `0..=23`.
    pub hour: u8,
    /// Minute in the range `0..=59`.
    pub minute: u8,
}

impl TimeValue {
    /// Creates a time, clamping the hour and minute to their valid ranges.
    pub const fn new(hour: u8, minute: u8) -> Self {
        Self {
            hour: if hour > 23 { 23 } else { hour },
            minute: if minute > 59 { 59 } else { minute },
        }
    }
}

impl fmt::Display for TimeValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:02}:{:02}", self.hour, self.minute)
    }
}

/// The value of a [`DatePicker`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatePickerValue {
    /// A calendar date.
    Date(DateValue),
    /// A time of day.
    Time(TimeValue),
    /// A calendar date and a time of day.
    DateTime {
        /// The selected date.
        date: DateValue,
        /// The selected time.
        time: TimeValue,
    },
}

impl DatePickerValue {
    /// Clamps this value to the optional date and time ranges.
    pub fn clamped(
        self,
        date_range: Option<(DateValue, DateValue)>,
        time_range: Option<(TimeValue, TimeValue)>,
    ) -> Self {
        // A range whose bounds are reversed is treated as unset.
        let date_range = date_range.filter(|(min, max)| max >= min);
        let time_range = time_range.filter(|(min, max)| max >= min);
        let clamp_date = |date: DateValue| match date_range {
            Some((min, max)) => date.max(min).min(max),
            None => date,
        };
        let clamp_time = |time: TimeValue| match time_range {
            Some((min, max)) => time.max(min).min(max),
            None => time,
        };
        match self {
            Self::Date(date) => Self::Date(clamp_date(date)),
            Self::Time(time) => Self::Time(clamp_time(time)),
            Self::DateTime { date, time } => {
                let date = clamp_date(date);
                let mut time = clamp_time(time);
                if let (Some((min, _)), Some((min_time, _))) = (date_range, time_range)
                    && date == min
                    && time < min_time
                {
                    time = min_time;
                }
                if let (Some((_, max)), Some((_, max_time))) = (date_range, time_range)
                    && date == max
                    && time > max_time
                {
                    time = max_time;
                }
                Self::DateTime { date, time }
            }
        }
    }
}

/// How a [`DatePicker`] presents its picker.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DatePickerMode {
    /// A compact field that opens the platform's picker.
    #[default]
    Compact,
    /// An inline picker that is always visible.
    Inline,
}

/// The default accent color of sliders and progress indicators.
pub const CONTROL_ACCENT: u32 = 0xFF21_96F3;

/// The alpha multiplier applied to DSL-drawn decoration while a node is
/// disabled.
///
/// Native text and controls dim themselves; this only affects fills, gradients,
/// borders and shadows that the DSL paints.
pub const DISABLED_ALPHA: f32 = 0.4;

/// Dims an ARGB color when its node is disabled, preserving the RGB channels.
///
/// The explicit opacity of a node is applied separately, so a disabled node
/// keeps its own alpha and is dimmed on top of it.
pub fn dim_color(argb: u32, enabled: bool) -> u32 {
    if enabled {
        return argb;
    }
    let alpha = (argb >> 24) & 0xFF;
    let alpha = (alpha as f32 * DISABLED_ALPHA).round() as u32;
    (argb & 0x00FF_FFFF) | (alpha << 24)
}

/// Platform-independent properties of a slider.
#[derive(Clone, Debug, PartialEq)]
pub struct SliderProps {
    /// The current value.
    pub value: f32,
    /// The lowest selectable value.
    pub min: f32,
    /// The highest selectable value.
    pub max: f32,
    /// The interval values snap to, or `None` for a continuous slider.
    pub step: Option<f32>,
    /// The action dispatched whenever the value changes.
    pub action: Cow<'static, str>,
    /// ARGB color of the active track and thumb.
    pub color: u32,
}

impl SliderProps {
    /// Creates a slider with the given value and the default range `0..=1`.
    pub fn new(value: f32) -> Self {
        Self {
            value,
            min: 0.0,
            max: 1.0,
            step: None,
            action: Cow::Borrowed(""),
            color: CONTROL_ACCENT,
        }
    }
}

/// Snaps a slider value to its range and step.
#[cfg_attr(not(any(target_os = "android", target_os = "ios")), allow(dead_code))]
pub(crate) fn snap_slider_value(value: f32, min: f32, max: f32, step: Option<f32>) -> f32 {
    let low = min.min(max);
    let high = max.max(min);
    let clamped = value.clamp(low, high);
    match step {
        Some(step) if step > 0.0 => {
            let snapped = low + ((clamped - low) / step).round() * step;
            snapped.clamp(low, high)
        }
        _ => clamped,
    }
}

/// Platform-independent properties of a progress indicator.
#[derive(Clone, Debug, PartialEq)]
pub struct ProgressProps {
    /// The progress in the range `0.0..=1.0`.
    pub value: f32,
    /// ARGB color of the filled part.
    pub color: u32,
}

impl ProgressProps {
    /// Creates a progress indicator with the given fraction.
    pub fn new(value: f32) -> Self {
        Self {
            value,
            color: CONTROL_ACCENT,
        }
    }
}

/// Platform-independent properties of an indeterminate progress spinner.
#[derive(Clone, Debug, PartialEq)]
pub struct SpinnerProps {
    /// ARGB color of the spinner.
    pub color: u32,
}

impl SpinnerProps {
    /// Creates a spinner with the default accent color.
    pub fn new() -> Self {
        Self {
            color: CONTROL_ACCENT,
        }
    }
}

impl Default for SpinnerProps {
    fn default() -> Self {
        Self::new()
    }
}

/// Platform-independent properties of a group of mutually exclusive radio buttons.
#[derive(Clone, Debug, PartialEq)]
pub struct RadioGroupProps {
    /// The labels of the options, in display order.
    pub options: Vec<String>,
    /// The index of the selected option, or `None` when nothing is selected.
    pub selected: Option<usize>,
    /// The action dispatched whenever the selection changes.
    pub action: Cow<'static, str>,
    /// Font size of the labels in logical points.
    pub size: f32,
    /// ARGB color of the labels and indicators.
    pub color: u32,
    /// Font weight of the labels.
    pub weight: FontWeight,
    /// Space between two options in logical points.
    pub spacing: f32,
}

impl RadioGroupProps {
    /// Creates a radio group with the given option labels.
    pub fn new(options: Vec<String>) -> Self {
        Self {
            options,
            selected: None,
            action: Cow::Borrowed(""),
            size: 16.0,
            color: 0xFF00_0000,
            weight: FontWeight::Normal,
            spacing: 8.0,
        }
    }
}

/// Platform-independent properties of a native tab bar.
#[derive(Clone, Debug, PartialEq)]
pub struct TabBarProps {
    /// The labels of the tabs, in display order.
    pub tabs: Vec<String>,
    /// The index of the selected tab.
    pub selected: usize,
    /// The action dispatched whenever the selection changes.
    pub action: Cow<'static, str>,
    /// Font size of the labels in logical points.
    pub size: f32,
    /// ARGB color of the labels.
    pub color: u32,
    /// Font weight of the labels.
    pub weight: FontWeight,
}

impl TabBarProps {
    /// Creates a tab bar with the given labels.
    pub fn new(tabs: Vec<String>) -> Self {
        Self {
            tabs,
            selected: 0,
            action: Cow::Borrowed(""),
            size: 14.0,
            color: 0xFF00_0000,
            weight: FontWeight::Normal,
        }
    }
}

/// Platform-independent properties of a date, time or date-and-time picker.
#[derive(Clone, Debug, PartialEq)]
pub struct DatePickerProps {
    /// The selected date, time or both.
    pub value: DatePickerValue,
    /// How the picker is presented.
    pub mode: DatePickerMode,
    /// The optional inclusive date range.
    pub date_range: Option<(DateValue, DateValue)>,
    /// The optional inclusive time range.
    pub time_range: Option<(TimeValue, TimeValue)>,
    /// The action dispatched whenever the value changes.
    pub action: Cow<'static, str>,
    /// Font size of the compact field in logical points.
    pub size: f32,
    /// ARGB color of the compact field's text.
    pub color: u32,
    /// Font weight of the compact field's text.
    pub weight: FontWeight,
}

impl DatePickerProps {
    /// Creates a compact date picker with the given value.
    pub fn new(value: DatePickerValue) -> Self {
        Self {
            value,
            mode: DatePickerMode::Compact,
            date_range: None,
            time_range: None,
            action: Cow::Borrowed(""),
            size: 16.0,
            color: 0xFF00_0000,
            weight: FontWeight::Normal,
        }
    }
}

/// Platform-independent properties of a swipeable page container.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PagerProps {
    /// The number of pages.
    pub pages: usize,
    /// The index of the selected page.
    pub selected: usize,
    /// The action dispatched when the user swipes to another page.
    pub action: Cow<'static, str>,
}

/// A native horizontally paged container.
#[derive(Clone, Debug, PartialEq)]
pub struct Pager(pub(crate) Element);

impl Pager {
    /// Creates an empty pager that fills the available width.
    pub fn new() -> Self {
        let mut element = Element::container(ElementKind::Pager(PagerProps::default()));
        element.style.width = Length::Percent(1.0);
        Self(element)
    }

    /// Sets the selected page index. Changing it scrolls the native pager
    /// to that page in place; the pages keep their state.
    pub fn selected(mut self, index: usize) -> Self {
        if let ElementKind::Pager(props) = &mut self.0.kind {
            props.selected = index;
        }
        self
    }

    /// Sets the action dispatched when the page changes.
    pub fn action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let ElementKind::Pager(props) = &mut self.0.kind {
            props.action = action.into();
        }
        self
    }

    /// Appends one page.
    pub fn child(mut self, child: impl Into<Element>) -> Self {
        self.0.children.push(child.into());
        self
    }
}

impl Default for Pager {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Pager> for Element {
    fn from(component: Pager) -> Self {
        component.0
    }
}

/// A tab strip above a swipeable pager.
#[derive(Clone, Debug, PartialEq)]
pub struct TabsView(pub(crate) Element);

impl TabsView {
    /// Creates a tabs view with the given labels and no pages.
    pub fn new(tabs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let column = Column::new()
            .width_percent(1.0)
            .child(TabBar::new(tabs))
            .child(Pager::new().grow(1.0));
        Self(column.0)
    }

    /// Sets the selected tab and page.
    pub fn selected(mut self, index: usize) -> Self {
        if let Some(bar) = self.0.children.first_mut()
            && let ElementKind::TabBar(props) = &mut bar.kind
        {
            props.selected = index;
        }
        if let Some(pager) = self.0.children.get_mut(1)
            && let ElementKind::Pager(props) = &mut pager.kind
        {
            props.selected = index;
        }
        self
    }

    /// Sets the action dispatched for tab taps and page swipes.
    pub fn action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        let action = action.into();
        if let Some(bar) = self.0.children.first_mut()
            && let ElementKind::TabBar(props) = &mut bar.kind
        {
            props.action = action.clone();
        }
        if let Some(pager) = self.0.children.get_mut(1)
            && let ElementKind::Pager(props) = &mut pager.kind
        {
            props.action = action;
        }
        self
    }

    /// Appends one page.
    pub fn child(mut self, child: impl Into<Element>) -> Self {
        if let Some(pager) = self.0.children.get_mut(1) {
            pager.children.push(child.into());
        }
        self
    }
}

impl From<TabsView> for Element {
    fn from(component: TabsView) -> Self {
        component.0
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

/// How a [`List`] materializes its rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ListMode {
    /// Window the content inside a native scroll container (default).
    #[default]
    Windowed,
    /// Let a native recycler own scrolling and view recycling.
    Recycled,
}

/// The item source of a virtualized [`List`].
///
/// Keys and rows are closures so the runtime can materialize only the items in
/// the window. They must be `Send + Sync` because the app may render on any
/// thread.
#[derive(Clone)]
pub(crate) struct ListProps {
    pub(crate) item_count: usize,
    pub(crate) mode: ListMode,
    pub(crate) item_height: Option<f32>,
    pub(crate) estimated_height: f32,
    pub(crate) buffer: usize,
    /// The item revision, when the application supplies one.
    pub(crate) revision: Option<u64>,
    /// Cross-fade duration for a holder rebind, in milliseconds.
    pub(crate) row_transition_ms: u32,
    pub(crate) keys: Arc<dyn Fn(usize) -> String + Send + Sync>,
    pub(crate) rows: Arc<dyn Fn(usize) -> Element + Send + Sync>,
}

impl fmt::Debug for ListProps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ListProps")
            .field("mode", &self.mode)
            .field("item_count", &self.item_count)
            .field("item_height", &self.item_height)
            .field("buffer", &self.buffer)
            .field("revision", &self.revision)
            .finish_non_exhaustive()
    }
}

impl PartialEq for ListProps {
    /// Row sources are opaque; two lists are equal when their visible
    /// configuration matches and they share the same item source.
    fn eq(&self, other: &Self) -> bool {
        self.mode == other.mode
            && self.item_count == other.item_count
            && self.item_height == other.item_height
            && self.buffer == other.buffer
            && self.revision == other.revision
            && self.row_transition_ms == other.row_transition_ms
            && Arc::ptr_eq(&self.keys, &other.keys)
    }
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
    /// ARGB color at the start edge.
    pub start: u32,
    /// ARGB color at the end edge.
    pub end: u32,
    /// The axis the gradient runs along.
    pub direction: GradientDirection,
}

impl Gradient {
    /// Creates a vertical gradient between two colors.
    pub const fn vertical(start: u32, end: u32) -> Self {
        Self {
            start,
            end,
            direction: GradientDirection::Vertical,
        }
    }

    /// Creates a horizontal gradient between two colors.
    pub const fn horizontal(start: u32, end: u32) -> Self {
        Self {
            start,
            end,
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
    /// ARGB background color, if any.
    pub background: Option<u32>,
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

/// The component-specific part of an [`Element`].
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ElementKind {
    Column,
    Row,
    Spacer,
    Text(TextProps),
    Button(ButtonProps),
    TextField(TextFieldProps),
    Switch(SwitchProps),
    Checkbox(CheckProps),
    RadioButton(CheckProps),
    RadioGroup(RadioGroupProps),
    TabBar(TabBarProps),
    Pager(PagerProps),
    DatePicker(DatePickerProps),
    Slider(SliderProps),
    Progress(ProgressProps),
    Spinner(SpinnerProps),
    Image(ImageProps),
    Scroll(ScrollProps),
    List(ListProps),
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
            Self::Switch(_) => 10,
            Self::Checkbox(_) => 12,
            Self::RadioButton(_) => 13,
            Self::RadioGroup(_) => 16,
            Self::TabBar(_) => 19,
            Self::Pager(_) => 20,
            Self::DatePicker(_) => 18,
            Self::Slider(_) => 14,
            Self::Progress(_) => 15,
            Self::Spinner(_) => 17,
            Self::Image(_) => 7,
            Self::Scroll(props) => match props.axis {
                Axis::Vertical => 8,
                Axis::Horizontal => 9,
            },
            Self::List(_) => 11,
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
    pub(crate) key: Option<Cow<'static, str>>,
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
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
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

            /// Sets the width-to-height ratio.
            pub fn aspect_ratio(mut self, ratio: f32) -> Self {
                self.0.style.aspect_ratio = Some(ratio);
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

            /// Sets a two-color background gradient.
            pub fn gradient(mut self, gradient: Gradient) -> Self {
                self.0.style.gradient = Some(gradient);
                self
            }

            /// Rounds the corners of the node's background and border.
            pub fn corner_radius(mut self, radius: impl IntoPoints) -> Self {
                self.0.style.corner_radius = radius.into_points();
                self
            }

            /// Rounds each corner of the node independently.
            pub fn corner_radii(mut self, radii: CornerRadii) -> Self {
                self.0.style.corner_radii = Some(radii);
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

            /// Raises the node above its siblings with a native shadow.
            pub fn elevation(mut self, elevation: impl IntoPoints) -> Self {
                self.0.style.elevation = elevation.into_points().max(0.0);
                self
            }

            /// Animates this node's frame and opacity changes.
            ///
            /// A duration of zero makes subsequent changes snap.
            pub fn animate(mut self, duration_ms: u32) -> Self {
                self.0.style.animation = Some(Animation::new(duration_ms));
                self
            }

            /// Sets the full implicit animation.
            pub fn animation(mut self, animation: Animation) -> Self {
                self.0.style.animation = Some(animation);
                self
            }

            /// Replaces the timing curve, keeping the current duration.
            pub fn animation_curve(mut self, curve: AnimationCurve) -> Self {
                let animation = self.0.style.animation.unwrap_or_default().curve(curve);
                self.0.style.animation = Some(animation);
                self
            }

            /// Fades and scales the node in when it is created.
            pub fn animate_enter(mut self, duration_ms: u32) -> Self {
                self.0.style.enter = Some(Transition::new(duration_ms));
                self
            }

            /// Fades and scales the node out before it is removed.
            pub fn animate_exit(mut self, duration_ms: u32) -> Self {
                self.0.style.exit = Some(Transition::new(duration_ms));
                self
            }

            /// Replaces the enter transition's timing curve.
            pub fn enter_curve(mut self, curve: AnimationCurve) -> Self {
                let transition = self.0.style.enter.take().unwrap_or_default().curve(curve);
                self.0.style.enter = Some(transition);
                self
            }

            /// Replaces the exit transition's timing curve.
            pub fn exit_curve(mut self, curve: AnimationCurve) -> Self {
                let transition = self.0.style.exit.take().unwrap_or_default().curve(curve);
                self.0.style.exit = Some(transition);
                self
            }

            /// Enables or disables interaction with the node.
            pub fn enabled(mut self, enabled: bool) -> Self {
                self.0.style.enabled = Some(enabled);
                self
            }

            /// Sets the label announced by accessibility services.
            pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
                self.0.style.accessibility_label = Some(label.into());
                self
            }

            /// Identifies the node among its siblings across rebuilds.
            ///
            /// Keys must be unique among siblings and stable across rebuilds.
            /// Both static strings and owned `String`s are accepted, which lets
            /// list rows derive their key from the item they represent.
            pub fn key(mut self, key: impl Into<Cow<'static, str>>) -> Self {
                self.0.key = Some(key.into());
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

    /// Aligns the text horizontally inside its frame.
    pub fn text_align(mut self, align: TextAlign) -> Self {
        if let ElementKind::Text(props) = &mut self.0.kind {
            props.align = align;
        }
        self
    }

    /// Limits the number of lines the text may occupy.
    pub fn max_lines(mut self, lines: u32) -> Self {
        if let ElementKind::Text(props) = &mut self.0.kind {
            props.max_lines = Some(lines.max(1));
        }
        self
    }

    /// Truncates the text with an ellipsis when it does not fit.
    pub fn ellipsis(mut self) -> Self {
        if let ElementKind::Text(props) = &mut self.0.kind {
            props.ellipsis = true;
        }
        self
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
    ///
    /// Accepts a `&'static str` or an owned `String` for actions built at
    /// runtime, such as `format!("note:delete:{index}")`.
    pub fn action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let ElementKind::Button(props) = &mut self.0.kind {
            props.action = action.into();
        }
        self
    }
}

impl From<Button> for Element {
    fn from(component: Button) -> Self {
        component.0
    }
}

/// A native text input.
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
    ///
    /// Accepts a `&'static str` or an owned `String` for actions built at
    /// runtime, such as `format!("field:{index}")`.
    pub fn action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let ElementKind::TextField(props) = &mut self.0.kind {
            props.action = action.into();
        }
        self
    }

    /// Sets the action dispatched when the user submits the input.
    ///
    /// Accepts a `&'static str` or an owned `String` for actions built at
    /// runtime.
    pub fn submit_action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let ElementKind::TextField(props) = &mut self.0.kind {
            props.submit_action = action.into();
        }
        self
    }

    /// Sets the action dispatched when the input gains or loses focus.
    ///
    /// Accepts a `&'static str` or an owned `String` for actions built at
    /// runtime.
    pub fn focus_action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let ElementKind::TextField(props) = &mut self.0.kind {
            props.focus_action = action.into();
        }
        self
    }

    /// Lets the input accept multiple lines of text.
    pub fn multiline(mut self, multiline: bool) -> Self {
        if let ElementKind::TextField(props) = &mut self.0.kind {
            props.multiline = multiline;
        }
        self
    }

    /// Obscures the text while typing.
    pub fn secure(mut self, secure: bool) -> Self {
        if let ElementKind::TextField(props) = &mut self.0.kind {
            props.secure = secure;
        }
        self
    }

    /// Requests a specific keyboard layout.
    pub fn keyboard(mut self, keyboard: KeyboardType) -> Self {
        if let ElementKind::TextField(props) = &mut self.0.kind {
            props.keyboard = keyboard;
        }
        self
    }

    /// Sets the action key shown on the keyboard.
    pub fn ime_action(mut self, action: ImeAction) -> Self {
        if let ElementKind::TextField(props) = &mut self.0.kind {
            props.ime_action = action;
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

/// A native on/off switch.
#[derive(Clone, Debug, PartialEq)]
pub struct Switch(pub(crate) Element);

impl Switch {
    /// Creates a switch in the given state.
    pub fn new(checked: bool) -> Self {
        Self(Element::leaf(ElementKind::Switch(SwitchProps::new(
            checked,
        ))))
    }

    /// Sets whether the switch is on.
    pub fn checked(mut self, checked: bool) -> Self {
        if let ElementKind::Switch(props) = &mut self.0.kind {
            props.checked = checked;
        }
        self
    }

    /// Sets the action dispatched whenever the switch is toggled.
    ///
    /// Accepts a `&'static str` or an owned `String` for actions built at
    /// runtime.
    pub fn action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let ElementKind::Switch(props) = &mut self.0.kind {
            props.action = action.into();
        }
        self
    }
}

impl From<Switch> for Element {
    fn from(component: Switch) -> Self {
        component.0
    }
}

/// A native checkbox with a text label.
#[derive(Clone, Debug, PartialEq)]
pub struct Checkbox(pub(crate) Element);

impl Checkbox {
    /// Creates a checkbox with the given label and state.
    pub fn new(label: impl Into<String>, checked: bool) -> Self {
        let label = label.into();
        let mut element = Element::leaf(ElementKind::Checkbox(CheckProps::new(
            label.clone(),
            checked,
        )));
        element.style.accessibility_label = Some(label);
        Self(element)
    }

    /// Sets whether the checkbox is selected.
    pub fn checked(mut self, checked: bool) -> Self {
        if let ElementKind::Checkbox(props) = &mut self.0.kind {
            props.checked = checked;
        }
        self
    }

    /// Sets the action dispatched whenever the checkbox is toggled.
    pub fn action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let ElementKind::Checkbox(props) = &mut self.0.kind {
            props.action = action.into();
        }
        self
    }
}

impl From<Checkbox> for Element {
    fn from(component: Checkbox) -> Self {
        component.0
    }
}

/// A native radio button with a text label.
///
/// Radio buttons are independent controls; mutual exclusion is application
/// state, usually a selected index rendered as one checked button.
#[derive(Clone, Debug, PartialEq)]
pub struct RadioButton(pub(crate) Element);

impl RadioButton {
    /// Creates a radio button with the given label and state.
    pub fn new(label: impl Into<String>, checked: bool) -> Self {
        let label = label.into();
        let mut element = Element::leaf(ElementKind::RadioButton(CheckProps::new(
            label.clone(),
            checked,
        )));
        element.style.accessibility_label = Some(label);
        Self(element)
    }

    /// Sets whether the radio button is selected.
    pub fn checked(mut self, checked: bool) -> Self {
        if let ElementKind::RadioButton(props) = &mut self.0.kind {
            props.checked = checked;
        }
        self
    }

    /// Sets the action dispatched whenever the radio button is selected.
    pub fn action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let ElementKind::RadioButton(props) = &mut self.0.kind {
            props.action = action.into();
        }
        self
    }
}

impl From<RadioButton> for Element {
    fn from(component: RadioButton) -> Self {
        component.0
    }
}

/// A native group of mutually exclusive radio buttons.
///
/// Unlike individual [`RadioButton`]s, the group owns the selection: tapping an
/// option reports the chosen index through [`Event::Selected`](crate::app::Event::Selected).
#[derive(Clone, Debug, PartialEq)]
pub struct RadioGroup(pub(crate) Element);

impl RadioGroup {
    /// Creates a radio group with the given option labels.
    pub fn new(options: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let options: Vec<String> = options.into_iter().map(Into::into).collect();
        Self(Element::leaf(ElementKind::RadioGroup(
            RadioGroupProps::new(options),
        )))
    }

    /// Sets the index of the selected option.
    pub fn selected(mut self, index: usize) -> Self {
        if let ElementKind::RadioGroup(props) = &mut self.0.kind {
            props.selected = Some(index);
        }
        self
    }

    /// Sets the action dispatched whenever the selection changes.
    pub fn action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let ElementKind::RadioGroup(props) = &mut self.0.kind {
            props.action = action.into();
        }
        self
    }

    /// Sets the space between two options in logical points.
    pub fn spacing(mut self, spacing: impl IntoPoints) -> Self {
        if let ElementKind::RadioGroup(props) = &mut self.0.kind {
            props.spacing = spacing.into_points();
        }
        self
    }
}

impl From<RadioGroup> for Element {
    fn from(component: RadioGroup) -> Self {
        component.0
    }
}

/// A native tab bar that reports the tapped tab through [`Event::Selected`].
///
/// The application owns the selection: it stores the index and passes it back
/// with [`TabBar::selected`]. Use [`Tabs`] to pair the bar with a content view.
///
/// [`Event::Selected`]: crate::app::Event::Selected
#[derive(Clone, Debug, PartialEq)]
pub struct TabBar(pub(crate) Element);

impl TabBar {
    /// Creates a tab bar with the given labels and the first tab selected.
    pub fn new(tabs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let tabs: Vec<String> = tabs.into_iter().map(Into::into).collect();
        Self(Element::leaf(ElementKind::TabBar(TabBarProps::new(tabs))))
    }

    /// Sets the index of the selected tab.
    pub fn selected(mut self, index: usize) -> Self {
        if let ElementKind::TabBar(props) = &mut self.0.kind {
            props.selected = index;
        }
        self
    }

    /// Sets the action dispatched whenever the selection changes.
    pub fn action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let ElementKind::TabBar(props) = &mut self.0.kind {
            props.action = action.into();
        }
        self
    }
}

impl From<TabBar> for Element {
    fn from(component: TabBar) -> Self {
        component.0
    }
}

/// A tab bar stacked above application-owned content.
///
/// This is a plain composition of [`TabBar`] and one child; it does not create
/// a native tab controller. The application chooses which content to build for
/// the current index and handles [`Event::Selected`](crate::app::Event::Selected).
#[derive(Clone, Debug, PartialEq)]
pub struct Tabs(pub(crate) Element);

impl Tabs {
    /// Creates a tab bar with the given labels above empty content.
    pub fn new(tabs: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let mut column = Column::new();
        column.0.children.push(TabBar::new(tabs).into());
        Self(column.0)
    }

    /// Sets the index of the selected tab.
    pub fn selected(mut self, index: usize) -> Self {
        if let Some(tab_bar) = self.0.children.first_mut()
            && let ElementKind::TabBar(props) = &mut tab_bar.kind
        {
            props.selected = index;
        }
        self
    }

    /// Sets the action dispatched whenever the selection changes.
    pub fn action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let Some(tab_bar) = self.0.children.first_mut()
            && let ElementKind::TabBar(props) = &mut tab_bar.kind
        {
            props.action = action.into();
        }
        self
    }

    /// Appends the content shown below the tab bar.
    pub fn child(mut self, child: impl Into<Element>) -> Self {
        self.0.children.push(child.into());
        self
    }
}

impl From<Tabs> for Element {
    fn from(component: Tabs) -> Self {
        component.0
    }
}

/// A native date, time or date-and-time picker.
///
/// The value kind is chosen by the constructor: [`DatePicker::new`] for a
/// date, [`DatePicker::time`] for a time, and [`DatePicker::date_time`] for
/// both. The reported event matches the kind.
#[derive(Clone, Debug, PartialEq)]
pub struct DatePicker(pub(crate) Element);

impl DatePicker {
    /// Creates a compact date picker with the given date.
    pub fn new(value: DateValue) -> Self {
        Self(Element::leaf(ElementKind::DatePicker(
            DatePickerProps::new(DatePickerValue::Date(value)),
        )))
    }

    /// Creates a compact time picker with the given time.
    pub fn time(value: TimeValue) -> Self {
        Self(Element::leaf(ElementKind::DatePicker(
            DatePickerProps::new(DatePickerValue::Time(value)),
        )))
    }

    /// Creates a compact date-and-time picker.
    pub fn date_time(date: DateValue, time: TimeValue) -> Self {
        Self(Element::leaf(ElementKind::DatePicker(
            DatePickerProps::new(DatePickerValue::DateTime { date, time }),
        )))
    }

    /// Chooses between the compact field and the inline picker.
    pub fn mode(mut self, mode: DatePickerMode) -> Self {
        if let ElementKind::DatePicker(props) = &mut self.0.kind {
            props.mode = mode;
        }
        self
    }

    /// Sets inclusive date bounds.
    pub fn range(mut self, min: DateValue, max: DateValue) -> Self {
        if let ElementKind::DatePicker(props) = &mut self.0.kind {
            props.date_range = Some((min, max));
        }
        self
    }

    /// Sets inclusive time bounds.
    pub fn time_range(mut self, min: TimeValue, max: TimeValue) -> Self {
        if let ElementKind::DatePicker(props) = &mut self.0.kind {
            props.time_range = Some((min, max));
        }
        self
    }

    /// Sets the action dispatched whenever the date changes.
    pub fn action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let ElementKind::DatePicker(props) = &mut self.0.kind {
            props.action = action.into();
        }
        self
    }
}

impl From<DatePicker> for Element {
    fn from(component: DatePicker) -> Self {
        component.0
    }
}

/// A native slider.
#[derive(Clone, Debug, PartialEq)]
pub struct Slider(pub(crate) Element);

impl Slider {
    /// Creates a slider with the given value and the default range `0..=1`.
    pub fn new(value: f32) -> Self {
        Self(Element::leaf(ElementKind::Slider(SliderProps::new(value))))
    }

    /// Sets the lowest selectable value.
    pub fn min(mut self, min: f32) -> Self {
        if let ElementKind::Slider(props) = &mut self.0.kind {
            props.min = min;
        }
        self
    }

    /// Sets the highest selectable value.
    pub fn max(mut self, max: f32) -> Self {
        if let ElementKind::Slider(props) = &mut self.0.kind {
            props.max = max;
        }
        self
    }

    /// Quantizes the slider to the given interval.
    ///
    /// A non-positive step restores continuous values.
    pub fn step(mut self, step: f32) -> Self {
        if let ElementKind::Slider(props) = &mut self.0.kind {
            props.step = (step > 0.0).then_some(step);
        }
        self
    }

    /// Sets the action dispatched whenever the value changes.
    pub fn action(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        if let ElementKind::Slider(props) = &mut self.0.kind {
            props.action = action.into();
        }
        self
    }

    /// Sets the ARGB color of the active track and thumb.
    pub fn color(mut self, color: u32) -> Self {
        if let ElementKind::Slider(props) = &mut self.0.kind {
            props.color = color;
        }
        self
    }
}

impl From<Slider> for Element {
    fn from(component: Slider) -> Self {
        component.0
    }
}

/// A native determinate progress indicator.
#[derive(Clone, Debug, PartialEq)]
pub struct ProgressIndicator(pub(crate) Element);

impl ProgressIndicator {
    /// Creates a determinate progress indicator with a fraction in `0.0..=1.0`.
    pub fn new(value: f32) -> Self {
        Self(Element::leaf(ElementKind::Progress(ProgressProps::new(
            value.clamp(0.0, 1.0),
        ))))
    }

    /// Creates an indeterminate progress spinner.
    pub fn indeterminate() -> Self {
        Self(Element::leaf(ElementKind::Spinner(SpinnerProps::new())))
    }

    /// Sets the ARGB color of the filled part or spinner.
    pub fn color(mut self, color: u32) -> Self {
        match &mut self.0.kind {
            ElementKind::Progress(props) => props.color = color,
            ElementKind::Spinner(props) => props.color = color,
            _ => {}
        }
        self
    }
}

impl From<ProgressIndicator> for Element {
    fn from(component: ProgressIndicator) -> Self {
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

/// Derives the key of one list item.
type KeyBuilder<T> = Box<dyn Fn(&T) -> String + Send + Sync>;

/// Builds the element of one list item.
type RowBuilder<T> = Box<dyn Fn(&T) -> Element + Send + Sync>;

/// A virtualized vertical list.
///
/// Only the rows around the viewport are turned into elements and native
/// views; the rest of the content is empty space, so the scroll extent and the
/// scrollbar still reflect every item. Give the list a bounded area, normally
/// by placing it in a sized or growing container.
///
/// Keys must be unique among the items; they keep a row's native view alive
/// while it scrolls through the window.
pub struct List<T> {
    items: Arc<Vec<T>>,
    mode: ListMode,
    item_height: Option<f32>,
    estimated_height: f32,
    buffer: usize,
    revision: Option<u64>,
    row_transition_ms: u32,
    key: Option<KeyBuilder<T>>,
    row: Option<RowBuilder<T>>,
}

impl<T: Send + Sync + 'static> List<T> {
    /// Creates a list over the given items.
    ///
    /// Pass an `Arc<Vec<_>>` to avoid copying the data on every rebuild.
    pub fn new(items: impl Into<Arc<Vec<T>>>) -> Self {
        Self {
            items: items.into(),
            mode: ListMode::Windowed,
            item_height: None,
            estimated_height: 56.0,
            buffer: 3,
            revision: None,
            row_transition_ms: 0,
            key: None,
            row: None,
        }
    }

    /// Fixes every row to the same height.
    ///
    /// Without it the list measures each row and caches the result, which
    /// tolerates mixed row heights at the cost of a measurement pass.
    pub fn item_height(mut self, height: impl IntoPoints) -> Self {
        self.item_height = Some(height.into_points().max(1.0));
        self
    }

    /// Uses a native recycler for scrolling and view recycling.
    ///
    /// On Android this swaps the windowed scroll container for a `RecyclerView`
    /// and on iOS for a `UICollectionView`. `item_height` is optional: with it
    /// the holder has a fixed height, without it Rust measures each row and the
    /// holder wraps the content.
    ///
    /// Recycled rows are rebuilt by diffing the previous tree of their holder,
    /// so they may contain any widget. A new render revision rebinds the
    /// attached rows, which is how application state changes reach a row. A
    /// focused field keeps its logical focus across recycling and regains it
    /// when its row is bound again.
    pub fn recycled(mut self) -> Self {
        self.mode = ListMode::Recycled;
        self
    }

    /// Cross-fades a recycled row when a holder is rebound to new content.
    ///
    /// The fade is visual only: the holder is updated immediately and stays
    /// available for reuse. `0` (the default) disables it. Windowed lists
    /// rebuild their rows in place and ignore this.
    pub fn row_transition(mut self, duration_ms: u32) -> Self {
        self.row_transition_ms = duration_ms;
        self
    }

    /// Sets the height assumed for rows that have not been measured yet.
    pub fn estimated_height(mut self, height: impl IntoPoints) -> Self {
        self.estimated_height = height.into_points().max(1.0);
        self
    }

    /// Materializes this many extra rows above and below the viewport.
    pub fn buffer(mut self, rows: usize) -> Self {
        self.buffer = rows;
        self
    }

    /// Declares a revision of the item sequence.
    ///
    /// Change the value whenever the items, their order, or the strings their
    /// keys derive from change. The runtime caches row heights and prefix sums
    /// per revision, so an unchanged revision makes a rebuild cost only the
    /// visible rows instead of re-keying the whole list. It must also change
    /// when a different list takes over the same position in the tree (for
    /// example after a screen swap).
    ///
    /// Without it the list stays correct but is fully re-keyed on every build,
    /// which caps it at hundreds of rows.
    pub fn revision(mut self, revision: u64) -> Self {
        self.revision = Some(revision);
        self
    }

    /// Derives the stable, unique key of an item.
    pub fn key(mut self, key: impl Fn(&T) -> String + Send + Sync + 'static) -> Self {
        self.key = Some(Box::new(key));
        self
    }

    /// Builds the element of one item. Called only for visible items.
    pub fn row(mut self, row: impl Fn(&T) -> Element + Send + Sync + 'static) -> Self {
        self.row = Some(Box::new(row));
        self
    }
}

impl<T: Send + Sync + 'static> From<List<T>> for Element {
    fn from(list: List<T>) -> Self {
        let key = list.key.expect("List::key must be set");
        let row = list.row.expect("List::row must be set");
        let items = list.items;

        let keys = {
            let items = Arc::clone(&items);
            Arc::new(move |index: usize| key(&items[index]))
                as Arc<dyn Fn(usize) -> String + Send + Sync>
        };
        let rows = {
            let items = Arc::clone(&items);
            Arc::new(move |index: usize| row(&items[index]))
                as Arc<dyn Fn(usize) -> Element + Send + Sync>
        };

        let mut element = Element::container(ElementKind::List(ListProps {
            item_count: items.len(),
            mode: list.mode,
            item_height: list.item_height,
            estimated_height: list.estimated_height,
            buffer: list.buffer,
            revision: list.revision,
            row_transition_ms: list.row_transition_ms,
            keys,
            rows,
        }));
        // A list is a scroll region: it grows into the space its flex parent
        // leaves and spans the cross axis.
        element.style.width = Length::Percent(1.0);
        element.style.grow = 1.0;
        element
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
            ElementKind::Checkbox(props) | ElementKind::RadioButton(props) => props.size = size,
            ElementKind::RadioGroup(props) => props.size = size,
            ElementKind::TabBar(props) => props.size = size,
            ElementKind::DatePicker(props) => props.size = size,
            _ => {}
        }
    }

    fn set_text_color(&mut self, color: u32) {
        match &mut self.kind {
            ElementKind::Text(props) => props.color = color,
            ElementKind::Button(props) => props.color = color,
            ElementKind::TextField(props) => props.color = color,
            ElementKind::Checkbox(props) | ElementKind::RadioButton(props) => props.color = color,
            ElementKind::RadioGroup(props) => props.color = color,
            ElementKind::TabBar(props) => props.color = color,
            ElementKind::DatePicker(props) => props.color = color,
            _ => {}
        }
    }

    fn set_text_weight(&mut self, weight: FontWeight) {
        match &mut self.kind {
            ElementKind::Text(props) => props.weight = weight,
            ElementKind::Button(props) => props.weight = weight,
            ElementKind::TextField(props) => props.weight = weight,
            ElementKind::Checkbox(props) | ElementKind::RadioButton(props) => props.weight = weight,
            ElementKind::RadioGroup(props) => props.weight = weight,
            ElementKind::TabBar(props) => props.weight = weight,
            ElementKind::DatePicker(props) => props.weight = weight,
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
style_builder!(Switch);
style_builder!(Checkbox);
style_builder!(RadioButton);
style_builder!(RadioGroup);
style_builder!(TabBar);
style_builder!(Pager);
style_builder!(TabsView);
style_builder!(DatePicker);
style_builder!(Slider);
style_builder!(ProgressIndicator);
style_builder!(Image);
style_builder!(ScrollView);
text_builder!(Text);
text_builder!(Button);
text_builder!(TextField);
text_builder!(Checkbox);
text_builder!(RadioButton);
text_builder!(RadioGroup);
text_builder!(DatePicker);

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

    #[test]
    fn switch_keeps_its_state_and_action() {
        let element: Element = Switch::new(true).action("toggle").into();

        let ElementKind::Switch(props) = element.kind else {
            panic!("expected switch");
        };
        assert!(props.checked);
        assert_eq!(props.action, "toggle");
    }

    #[test]
    fn radio_group_keeps_options_and_selection() {
        let element: Element = RadioGroup::new(["Small", "Medium", "Large"])
            .selected(1)
            .action("size")
            .spacing(12)
            .into();

        let ElementKind::RadioGroup(props) = element.kind else {
            panic!("expected radio group");
        };
        assert_eq!(props.options, ["Small", "Medium", "Large"]);
        assert_eq!(props.selected, Some(1));
        assert_eq!(props.action, "size");
        assert_eq!(props.spacing, 12.0);
    }

    #[test]
    fn slider_step_is_optional() {
        let element: Element = Slider::new(0.5).step(0.25).into();
        let ElementKind::Slider(props) = element.kind else {
            panic!("expected slider");
        };
        assert_eq!(props.step, Some(0.25));

        let element: Element = Slider::new(0.5).step(0.0).into();
        let ElementKind::Slider(props) = element.kind else {
            panic!("expected slider");
        };
        assert_eq!(props.step, None);
    }

    #[test]
    fn slider_values_snap_to_the_step() {
        assert_eq!(snap_slider_value(37.0, 0.0, 100.0, Some(10.0)), 40.0);
        assert_eq!(snap_slider_value(34.0, 0.0, 100.0, Some(10.0)), 30.0);
        assert_eq!(snap_slider_value(9.0, 1.0, 5.0, Some(1.0)), 5.0);
        assert_eq!(snap_slider_value(0.5, 1.0, 5.0, Some(1.0)), 1.0);
        assert_eq!(snap_slider_value(37.0, 0.0, 100.0, None), 37.0);
        assert_eq!(snap_slider_value(-4.0, 0.0, 100.0, None), 0.0);
        assert_eq!(snap_slider_value(140.0, 0.0, 100.0, None), 100.0);
    }

    #[test]
    fn indeterminate_progress_is_a_spinner() {
        let element: Element = ProgressIndicator::indeterminate().color(0xFF00_897B).into();
        let ElementKind::Spinner(props) = element.kind else {
            panic!("expected spinner");
        };
        assert_eq!(props.color, 0xFF00_897B);
    }

    #[test]
    fn dates_format_and_clamp() {
        assert_eq!(DateValue::new(2026, 11, 12).to_string(), "2026-11-12");
        assert_eq!(DateValue::new(2026, 0, 0), DateValue::new(2026, 1, 1));
        assert_eq!(DateValue::new(2026, 13, 40), DateValue::new(2026, 12, 31));
    }

    #[test]
    fn times_format_and_clamp() {
        assert_eq!(TimeValue::new(9, 5).to_string(), "09:05");
        assert_eq!(TimeValue::new(30, 90), TimeValue::new(23, 59));
    }

    #[test]
    fn picker_values_clamp_to_their_ranges() {
        let date_range = Some((DateValue::new(2026, 1, 10), DateValue::new(2026, 1, 20)));
        let time_range = Some((TimeValue::new(9, 0), TimeValue::new(17, 0)));

        assert_eq!(
            DatePickerValue::Date(DateValue::new(2025, 12, 31)).clamped(date_range, time_range),
            DatePickerValue::Date(DateValue::new(2026, 1, 10))
        );
        assert_eq!(
            DatePickerValue::Time(TimeValue::new(18, 30)).clamped(date_range, time_range),
            DatePickerValue::Time(TimeValue::new(17, 0))
        );
        assert_eq!(
            DatePickerValue::DateTime {
                date: DateValue::new(2026, 1, 20),
                time: TimeValue::new(18, 30),
            }
            .clamped(date_range, time_range),
            DatePickerValue::DateTime {
                date: DateValue::new(2026, 1, 20),
                time: TimeValue::new(17, 0),
            }
        );
        // A range whose bounds are reversed is ignored.
        assert_eq!(
            DatePickerValue::Time(TimeValue::new(12, 0))
                .clamped(None, Some((TimeValue::new(17, 0), TimeValue::new(9, 0)))),
            DatePickerValue::Time(TimeValue::new(12, 0))
        );
    }

    #[test]
    fn time_and_datetime_pickers_keep_their_ranges() {
        let element: Element = DatePicker::time(TimeValue::new(10, 30))
            .time_range(TimeValue::new(9, 0), TimeValue::new(17, 0))
            .action("time")
            .into();
        let ElementKind::DatePicker(props) = element.kind else {
            panic!("expected date picker");
        };
        assert_eq!(props.value, DatePickerValue::Time(TimeValue::new(10, 30)));
        assert_eq!(
            props.time_range,
            Some((TimeValue::new(9, 0), TimeValue::new(17, 0)))
        );

        let element: Element =
            DatePicker::date_time(DateValue::new(2026, 11, 12), TimeValue::new(10, 30))
                .range(DateValue::new(2026, 11, 1), DateValue::new(2026, 11, 30))
                .into();
        let ElementKind::DatePicker(props) = element.kind else {
            panic!("expected date picker");
        };
        assert_eq!(
            props.value,
            DatePickerValue::DateTime {
                date: DateValue::new(2026, 11, 12),
                time: TimeValue::new(10, 30),
            }
        );
        assert_eq!(
            props.date_range,
            Some((DateValue::new(2026, 11, 1), DateValue::new(2026, 11, 30)))
        );
    }

    #[test]
    fn tab_bar_keeps_tabs_selection_and_action() {
        let element: Element = TabBar::new(["Day", "Week", "Month"])
            .selected(2)
            .action("period")
            .into();
        let ElementKind::TabBar(props) = element.kind else {
            panic!("expected tab bar");
        };
        assert_eq!(props.tabs, ["Day", "Week", "Month"]);
        assert_eq!(props.selected, 2);
        assert_eq!(props.action, "period");
    }

    #[test]
    fn tabs_compose_a_bar_above_their_content() {
        let element: Element = Tabs::new(["One", "Two"])
            .selected(1)
            .action("tab")
            .child(Text::new("content"))
            .into();

        assert_eq!(element.children.len(), 2);
        let ElementKind::TabBar(props) = &element.children[0].kind else {
            panic!("expected tab bar");
        };
        assert_eq!(props.tabs, ["One", "Two"]);
        assert_eq!(props.selected, 1);
        assert_eq!(props.action, "tab");
        assert!(matches!(element.children[1].kind, ElementKind::Text(_)));
    }

    #[test]
    fn lists_keep_their_row_transition() {
        let list: Element = List::new(vec!["a", "b"])
            .key(|value| value.to_string())
            .row(|value| Text::new(*value).into())
            .recycled()
            .row_transition(180)
            .into();
        let ElementKind::List(props) = &list.kind else {
            panic!("expected list");
        };
        assert_eq!(props.row_transition_ms, 180);

        let plain: Element = List::new(vec!["a"])
            .key(|value| value.to_string())
            .row(|value| Text::new(*value).into())
            .into();
        let ElementKind::List(props) = &plain.kind else {
            panic!("expected list");
        };
        assert_eq!(props.row_transition_ms, 0);
    }

    #[test]
    fn disabled_decoration_is_dimmed_without_changing_rgb() {
        assert_eq!(dim_color(0xFF21_96F3, true), 0xFF21_96F3);
        assert_eq!(dim_color(0xFF21_96F3, false), 0x6621_96F3);
        // A translucent color keeps its own alpha as the base.
        assert_eq!(dim_color(0x8021_96F3, false), 0x3321_96F3);
    }

    #[test]
    fn style_builders_store_animations() {
        let element: Element = Button::new("Animate")
            .animate(320)
            .animation_curve(AnimationCurve::EaseOut)
            .into();
        assert_eq!(
            element.style.animation,
            Some(Animation::new(320).curve(AnimationCurve::EaseOut))
        );

        let element: Element = Text::new("Fade")
            .animation_curve(AnimationCurve::Linear)
            .into();
        assert_eq!(
            element.style.animation,
            Some(Animation::new(Animation::DEFAULT_DURATION_MS).curve(AnimationCurve::Linear))
        );
    }

    #[test]
    fn custom_curves_and_enter_exit_transitions_are_stored() {
        let element: Element = Button::new("Motion")
            .animation(Animation::new(400).curve(AnimationCurve::cubic_bezier(0.2, 0.9, 0.3, 1.0)))
            .animate_enter(200)
            .enter_curve(AnimationCurve::spring(0.7, 0.35))
            .animate_exit(160)
            .exit_curve(AnimationCurve::EaseIn)
            .into();

        let animation = element.style.animation.as_ref().unwrap();
        assert_eq!(
            animation.curve,
            AnimationCurve::cubic_bezier(0.2, 0.9, 0.3, 1.0)
        );
        let enter = element.style.enter.as_ref().unwrap();
        assert_eq!(enter.duration_ms, 200);
        assert_eq!(enter.curve, AnimationCurve::spring(0.7, 0.35));
        let exit = element.style.exit.as_ref().unwrap();
        assert_eq!(exit.duration_ms, 160);
        assert_eq!(exit.curve, AnimationCurve::EaseIn);
    }

    #[test]
    fn pager_and_tabs_view_keep_pages_and_selection() {
        let pager: Element = Pager::new()
            .selected(1)
            .action("page")
            .child(Text::new("one"))
            .child(Text::new("two"))
            .into();
        let ElementKind::Pager(props) = &pager.kind else {
            panic!("expected pager");
        };
        assert_eq!(props.selected, 1);
        assert_eq!(props.action, "page");
        assert_eq!(pager.children.len(), 2);

        let tabs: Element = TabsView::new(["One", "Two"])
            .selected(1)
            .action("tab")
            .child(Text::new("one"))
            .child(Text::new("two"))
            .into();
        assert_eq!(tabs.children.len(), 2);
        let ElementKind::TabBar(bar) = &tabs.children[0].kind else {
            panic!("expected tab bar");
        };
        assert_eq!(bar.selected, 1);
        let ElementKind::Pager(pager) = &tabs.children[1].kind else {
            panic!("expected pager");
        };
        assert_eq!(pager.selected, 1);
        assert_eq!(tabs.children[1].children.len(), 2);
    }

    #[test]
    fn icons_prefer_platform_names_over_the_shared_name() {
        let icon = Icon::new().shared("home").drawable("ic_home");
        assert_eq!(icon.android_name(), Some("ic_home"));
        assert_eq!(icon.ios_name(), Some("home"));

        let icon = Icon::new().shared("home").sf("house.fill");
        assert_eq!(icon.android_name(), Some("home"));
        assert_eq!(icon.ios_name(), Some("house.fill"));
    }

    #[test]
    fn date_picker_keeps_its_mode_and_action() {
        let element: Element = DatePicker::new(DateValue::new(2026, 11, 12))
            .mode(DatePickerMode::Inline)
            .action("pick")
            .into();

        let ElementKind::DatePicker(props) = element.kind else {
            panic!("expected date picker");
        };
        assert_eq!(
            props.value,
            DatePickerValue::Date(DateValue::new(2026, 11, 12))
        );
        assert_eq!(props.mode, DatePickerMode::Inline);
        assert_eq!(props.action, "pick");
    }
}
