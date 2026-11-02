//! Declarative descriptions of the Android widgets the renderer can build.
//!
//! Each component mirrors the configuration that is applied to one framework class. The builder
//! methods are intentionally chainable so the screen definitions read like a layout description.

use jni::strings::JNIStr;
use jni::sys::jint;

use super::{EdgeInsets, Orientation, Placed};
use crate::constants::COLOR_TEXT_PRIMARY;

/// A leaf `TextView`.
pub struct Text {
    pub value: &'static str,
    pub size_sp: f32,
    pub color: jint,
    pub gravity: Option<jint>,
    pub padding: EdgeInsets,
    pub background: Option<jint>,
}

impl Text {
    pub fn new(value: &'static str) -> Self {
        Self {
            value,
            size_sp: 14.0,
            color: COLOR_TEXT_PRIMARY,
            gravity: None,
            padding: EdgeInsets::ZERO,
            background: None,
        }
    }

    pub fn size(mut self, size_sp: f32) -> Self {
        self.size_sp = size_sp;
        self
    }

    pub fn color(mut self, color: jint) -> Self {
        self.color = color;
        self
    }

    pub fn gravity(mut self, gravity: jint) -> Self {
        self.gravity = Some(gravity);
        self
    }

    pub fn padding(mut self, padding: EdgeInsets) -> Self {
        self.padding = padding;
        self
    }

    pub fn background(mut self, color: jint) -> Self {
        self.background = Some(color);
        self
    }
}

/// A `Button`. The renderer installs the caller's click listener and writes an optional tag.
pub struct Button {
    pub value: &'static str,
    pub size_sp: f32,
    pub all_caps: bool,
    pub enabled: bool,
    /// Tag read by the application's click listener, for example to navigate.
    pub action: Option<&'static str>,
}

impl Button {
    pub fn new(value: &'static str) -> Self {
        Self {
            value,
            size_sp: 14.0,
            all_caps: false,
            enabled: true,
            action: None,
        }
    }

    pub fn size(mut self, size_sp: f32) -> Self {
        self.size_sp = size_sp;
        self
    }

    pub fn all_caps(mut self, all_caps: bool) -> Self {
        self.all_caps = all_caps;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Writes the tag that the application's click listener reads to decide what to do.
    pub fn action(mut self, tag: &'static str) -> Self {
        self.action = Some(tag);
        self
    }
}

/// An `ImageView` backed by a framework drawable resource.
///
/// `drawable` is the field name in `android.R.drawable` (for example `ic_dialog_info`), which
/// keeps the app independent of bundled image assets.
pub struct Image {
    pub drawable: &'static JNIStr,
    pub background: Option<jint>,
}

impl Image {
    pub fn new(drawable: &'static JNIStr) -> Self {
        Self {
            drawable,
            background: None,
        }
    }

    pub fn background(mut self, color: jint) -> Self {
        self.background = Some(color);
        self
    }
}

/// An `ImageButton` backed by a framework drawable resource.
pub struct ImageButton {
    pub drawable: &'static JNIStr,
    pub background: Option<jint>,
}

impl ImageButton {
    pub fn new(drawable: &'static JNIStr) -> Self {
        Self {
            drawable,
            background: None,
        }
    }

    pub fn background(mut self, color: jint) -> Self {
        self.background = Some(color);
        self
    }
}

/// An editable text field. The value itself lives in the Android view; Rust only describes the
/// initial state.
pub struct EditText {
    pub hint: &'static str,
    pub text: &'static str,
    pub size_sp: f32,
    pub input_type: Option<jint>,
    pub background: Option<jint>,
    pub padding: EdgeInsets,
}

impl EditText {
    pub fn new(hint: &'static str) -> Self {
        Self {
            hint,
            text: "",
            size_sp: 14.0,
            input_type: None,
            background: None,
            padding: EdgeInsets::ZERO,
        }
    }

    pub fn text(mut self, text: &'static str) -> Self {
        self.text = text;
        self
    }

    pub fn size(mut self, size_sp: f32) -> Self {
        self.size_sp = size_sp;
        self
    }

    pub fn input_type(mut self, input_type: jint) -> Self {
        self.input_type = Some(input_type);
        self
    }

    pub fn background(mut self, color: jint) -> Self {
        self.background = Some(color);
        self
    }

    pub fn padding(mut self, padding: EdgeInsets) -> Self {
        self.padding = padding;
        self
    }
}

/// A labelled `CheckBox`.
pub struct CheckBox {
    pub label: &'static str,
    pub checked: bool,
}

impl CheckBox {
    pub fn new(label: &'static str) -> Self {
        Self {
            label,
            checked: false,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }
}

/// A labelled `RadioButton`.
pub struct RadioButton {
    pub label: &'static str,
    pub checked: bool,
}

impl RadioButton {
    pub fn new(label: &'static str) -> Self {
        Self {
            label,
            checked: false,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }
}

/// A `RadioGroup` containing mutually exclusive `RadioButton`s.
pub struct RadioGroup {
    pub options: Vec<RadioButton>,
}

/// A labelled `Switch`.
pub struct Switch {
    pub label: &'static str,
    pub checked: bool,
}

impl Switch {
    pub fn new(label: &'static str) -> Self {
        Self {
            label,
            checked: false,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }
}

/// A `ToggleButton` with separate on/off labels.
pub struct ToggleButton {
    pub text_on: &'static str,
    pub text_off: &'static str,
    pub checked: bool,
}

impl ToggleButton {
    pub fn new(text_on: &'static str, text_off: &'static str) -> Self {
        Self {
            text_on,
            text_off,
            checked: false,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }
}

/// A `CheckedTextView` with a check mark.
pub struct CheckedText {
    pub label: &'static str,
    pub checked: bool,
}

impl CheckedText {
    pub fn new(label: &'static str) -> Self {
        Self {
            label,
            checked: false,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }
}

/// Horizontal determinate or indeterminate `ProgressBar`.
#[derive(Clone, Copy)]
pub enum ProgressKind {
    Horizontal,
    Spinner,
}

pub struct Progress {
    pub kind: ProgressKind,
    pub progress: jint,
    pub max: jint,
}

impl Progress {
    pub fn horizontal(progress: jint, max: jint) -> Self {
        Self {
            kind: ProgressKind::Horizontal,
            progress,
            max,
        }
    }

    pub fn spinner() -> Self {
        Self {
            kind: ProgressKind::Spinner,
            progress: 0,
            max: 100,
        }
    }
}

/// A `SeekBar` with an initial position.
pub struct Seek {
    pub progress: jint,
    pub max: jint,
}

impl Seek {
    pub fn new(progress: jint, max: jint) -> Self {
        Self { progress, max }
    }
}

/// A `RatingBar` with stars and an initial rating.
pub struct Rating {
    pub rating: f32,
    pub num_stars: jint,
    pub step_size: f32,
    pub indicator: bool,
}

impl Rating {
    pub fn new(rating: f32, num_stars: jint) -> Self {
        Self {
            rating,
            num_stars,
            step_size: 0.5,
            indicator: false,
        }
    }

    pub fn step_size(mut self, step_size: f32) -> Self {
        self.step_size = step_size;
        self
    }
}

/// A `Spinner` backed by an `ArrayAdapter`.
pub struct Spinner {
    pub items: &'static [&'static str],
    pub selected: jint,
}

impl Spinner {
    pub fn new(items: &'static [&'static str]) -> Self {
        Self { items, selected: 0 }
    }

    pub fn selected(mut self, selected: jint) -> Self {
        self.selected = selected;
        self
    }
}

/// An `AutoCompleteTextView` backed by an `ArrayAdapter`.
pub struct AutoComplete {
    pub hint: &'static str,
    pub items: &'static [&'static str],
    pub threshold: jint,
}

impl AutoComplete {
    pub fn new(hint: &'static str, items: &'static [&'static str]) -> Self {
        Self {
            hint,
            items,
            threshold: 1,
        }
    }

    pub fn threshold(mut self, threshold: jint) -> Self {
        self.threshold = threshold;
        self
    }
}

/// A `MultiAutoCompleteTextView` backed by an `ArrayAdapter` and comma tokenizer.
pub struct MultiAutoComplete {
    pub hint: &'static str,
    pub items: &'static [&'static str],
    pub threshold: jint,
}

impl MultiAutoComplete {
    pub fn new(hint: &'static str, items: &'static [&'static str]) -> Self {
        Self {
            hint,
            items,
            threshold: 1,
        }
    }

    pub fn threshold(mut self, threshold: jint) -> Self {
        self.threshold = threshold;
        self
    }
}

/// A `NumberPicker` with a range and initial value.
pub struct NumberPicker {
    pub min: jint,
    pub max: jint,
    pub value: jint,
}

impl NumberPicker {
    pub fn new(min: jint, max: jint, value: jint) -> Self {
        Self { min, max, value }
    }
}

/// A self-updating `TextClock`.
pub struct TextClock {
    pub size_sp: f32,
}

impl TextClock {
    pub fn new(size_sp: f32) -> Self {
        Self { size_sp }
    }
}

/// A `SearchView` with a query hint.
pub struct Search {
    pub hint: &'static str,
}

impl Search {
    pub fn new(hint: &'static str) -> Self {
        Self { hint }
    }
}

/// A native `android.widget.Toolbar` with a title, subtitle, navigation icon, and overflow menu.
pub struct Toolbar {
    pub title: &'static str,
    pub subtitle: &'static str,
    pub background: jint,
    pub navigation_icon: &'static JNIStr,
}

/// An `androidx.appcompat.widget.Toolbar` with the same configuration as [`Toolbar`].
///
/// The consuming app must depend on `androidx.appcompat:appcompat`.
pub struct AppCompatToolbar {
    pub title: &'static str,
    pub subtitle: &'static str,
    pub background: jint,
    pub navigation_icon: &'static JNIStr,
}

/// A `ScrollView` with exactly one child.
pub struct Scroll {
    pub background: Option<jint>,
    pub child: Box<Placed>,
}

/// A `LinearLayout` with any number of children.
pub struct Linear {
    pub orientation: Orientation,
    pub padding: EdgeInsets,
    pub background: Option<jint>,
    pub children: Vec<Placed>,
}

/// An `androidx.recyclerview.widget.RecyclerView` backed by the application's item adapter.
///
/// `span_count` selects a `GridLayoutManager`; `None` uses a `LinearLayoutManager`.
pub struct Recycler {
    pub items: &'static [&'static str],
    pub orientation: Orientation,
    pub span_count: Option<jint>,
    pub has_fixed_size: bool,
}

impl Recycler {
    /// Creates a vertical list.
    pub fn new(items: &'static [&'static str]) -> Self {
        Self {
            items,
            orientation: Orientation::Vertical,
            span_count: None,
            has_fixed_size: false,
        }
    }

    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    /// Uses a `GridLayoutManager` with `span_count` columns (or rows when horizontal).
    pub fn grid(mut self, span_count: jint) -> Self {
        self.span_count = Some(span_count);
        self
    }

    pub fn has_fixed_size(mut self, has_fixed_size: bool) -> Self {
        self.has_fixed_size = has_fixed_size;
        self
    }
}

/// An `androidx.viewpager2.widget.ViewPager2` backed by the application's item adapter.
pub struct ViewPager {
    pub items: &'static [&'static str],
    pub orientation: Orientation,
    /// `None` leaves ViewPager2's default offscreen-page limit in place.
    pub offscreen_page_limit: Option<jint>,
    /// Index of the page shown first.
    pub current_item: jint,
    pub user_input_enabled: bool,
}

impl ViewPager {
    /// Creates a horizontal pager showing the first item.
    pub fn new(items: &'static [&'static str]) -> Self {
        Self {
            items,
            orientation: Orientation::Horizontal,
            offscreen_page_limit: None,
            current_item: 0,
            user_input_enabled: true,
        }
    }

    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn offscreen_page_limit(mut self, limit: jint) -> Self {
        self.offscreen_page_limit = Some(limit);
        self
    }

    pub fn current_item(mut self, item: jint) -> Self {
        self.current_item = item;
        self
    }

    pub fn user_input_enabled(mut self, enabled: bool) -> Self {
        self.user_input_enabled = enabled;
        self
    }
}
