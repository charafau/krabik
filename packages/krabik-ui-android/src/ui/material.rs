//! Declarative descriptions of Material Components widgets.
//!
//! These components require `com.google.android.material:material` at runtime and a Material
//! Components theme (for example `Theme.Material3.DayNight.NoActionBar`) in the consuming app.
//! The Rust crate itself has no Gradle dependency: every Material class is reached by name.

use jni::strings::JNIStr;
use jni::sys::jint;

use super::{EdgeInsets, Orientation, Placed};
use crate::constants::COLOR_TEXT_PRIMARY;

/// The box treatment used by `TextInputLayout`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialBoxMode {
    None,
    Filled,
    Outlined,
}

impl MaterialBoxMode {
    pub fn value(self) -> jint {
        match self {
            Self::None => crate::constants::MATERIAL_TEXT_INPUT_BOX_NONE,
            Self::Filled => crate::constants::MATERIAL_TEXT_INPUT_BOX_FILLED,
            Self::Outlined => crate::constants::MATERIAL_TEXT_INPUT_BOX_OUTLINE,
        }
    }
}

/// A `MaterialTextView`.
pub struct MaterialText {
    pub value: &'static str,
    pub size_sp: f32,
    pub color: jint,
    pub gravity: Option<jint>,
    pub padding: EdgeInsets,
    pub background: Option<jint>,
}

impl MaterialText {
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

/// A `MaterialButton`.
pub struct MaterialButton {
    pub label: &'static str,
    pub size_sp: f32,
    pub all_caps: bool,
    pub enabled: bool,
    pub checkable: bool,
    pub checked: bool,
    pub icon: Option<&'static JNIStr>,
    pub action: Option<&'static str>,
}

impl MaterialButton {
    pub fn new(label: &'static str) -> Self {
        Self {
            label,
            size_sp: 14.0,
            all_caps: false,
            enabled: true,
            checkable: false,
            checked: false,
            icon: None,
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

    pub fn checkable(mut self, checkable: bool) -> Self {
        self.checkable = checkable;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn icon(mut self, icon: &'static JNIStr) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn action(mut self, tag: &'static str) -> Self {
        self.action = Some(tag);
        self
    }
}

/// A `MaterialButtonToggleGroup` containing checkable [`MaterialButton`]s.
pub struct MaterialButtonToggleGroup {
    pub buttons: Vec<MaterialButton>,
    pub single_selection: bool,
    pub selection_required: bool,
}

impl MaterialButtonToggleGroup {
    pub fn new(buttons: Vec<MaterialButton>) -> Self {
        Self {
            buttons,
            single_selection: true,
            selection_required: false,
        }
    }

    pub fn single_selection(mut self, single_selection: bool) -> Self {
        self.single_selection = single_selection;
        self
    }

    pub fn selection_required(mut self, selection_required: bool) -> Self {
        self.selection_required = selection_required;
        self
    }
}

/// A `MaterialCheckBox`.
pub struct MaterialCheckBox {
    pub label: &'static str,
    pub checked: bool,
}

impl MaterialCheckBox {
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

/// A `MaterialRadioButton`.
pub struct MaterialRadioButton {
    pub label: &'static str,
    pub checked: bool,
}

impl MaterialRadioButton {
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

/// A `RadioGroup` populated with [`MaterialRadioButton`]s.
pub struct MaterialRadioGroup {
    pub options: Vec<MaterialRadioButton>,
}

impl MaterialRadioGroup {
    pub fn new(options: Vec<MaterialRadioButton>) -> Self {
        Self { options }
    }
}

/// A `MaterialSwitch`.
pub struct MaterialSwitch {
    pub label: &'static str,
    pub checked: bool,
}

impl MaterialSwitch {
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

/// A `TextInputLayout` wrapping a `TextInputEditText`.
pub struct MaterialTextField {
    pub hint: &'static str,
    pub text: &'static str,
    pub helper_text: Option<&'static str>,
    pub error_text: Option<&'static str>,
    pub input_type: Option<jint>,
    pub box_mode: MaterialBoxMode,
    pub password_toggle: bool,
    pub size_sp: f32,
}

impl MaterialTextField {
    pub fn new(hint: &'static str) -> Self {
        Self {
            hint,
            text: "",
            helper_text: None,
            error_text: None,
            input_type: None,
            box_mode: MaterialBoxMode::Outlined,
            password_toggle: false,
            size_sp: 15.0,
        }
    }

    pub fn text(mut self, text: &'static str) -> Self {
        self.text = text;
        self
    }

    pub fn helper_text(mut self, helper_text: &'static str) -> Self {
        self.helper_text = Some(helper_text);
        self
    }

    pub fn error_text(mut self, error_text: &'static str) -> Self {
        self.error_text = Some(error_text);
        self
    }

    pub fn input_type(mut self, input_type: jint) -> Self {
        self.input_type = Some(input_type);
        self
    }

    pub fn box_mode(mut self, box_mode: MaterialBoxMode) -> Self {
        self.box_mode = box_mode;
        self
    }

    pub fn password_toggle(mut self, password_toggle: bool) -> Self {
        self.password_toggle = password_toggle;
        self
    }

    pub fn size(mut self, size_sp: f32) -> Self {
        self.size_sp = size_sp;
        self
    }
}

/// A `TextInputLayout` with a `MaterialAutoCompleteTextView` exposed-dropdown field.
pub struct MaterialDropdown {
    pub hint: &'static str,
    pub text: &'static str,
    pub items: &'static [&'static str],
    pub box_mode: MaterialBoxMode,
    pub size_sp: f32,
}

impl MaterialDropdown {
    pub fn new(hint: &'static str, items: &'static [&'static str]) -> Self {
        Self {
            hint,
            text: "",
            items,
            box_mode: MaterialBoxMode::Outlined,
            size_sp: 15.0,
        }
    }

    pub fn text(mut self, text: &'static str) -> Self {
        self.text = text;
        self
    }

    pub fn box_mode(mut self, box_mode: MaterialBoxMode) -> Self {
        self.box_mode = box_mode;
        self
    }

    pub fn size(mut self, size_sp: f32) -> Self {
        self.size_sp = size_sp;
        self
    }
}

/// A Material `SearchBar`.
pub struct MaterialSearchBar {
    pub hint: &'static str,
    pub text: &'static str,
    pub title: &'static str,
    pub navigation_icon: &'static JNIStr,
}

impl MaterialSearchBar {
    pub fn new(hint: &'static str) -> Self {
        Self {
            hint,
            text: "",
            title: "",
            navigation_icon: crate::constants::DRAWABLE_MENU_REVERT,
        }
    }

    pub fn text(mut self, text: &'static str) -> Self {
        self.text = text;
        self
    }

    pub fn title(mut self, title: &'static str) -> Self {
        self.title = title;
        self
    }

    pub fn navigation_icon(mut self, icon: &'static JNIStr) -> Self {
        self.navigation_icon = icon;
        self
    }
}

/// A Material `SearchView` container.
pub struct MaterialSearchView {
    pub hint: &'static str,
    pub text: &'static str,
}

impl MaterialSearchView {
    pub fn new(hint: &'static str) -> Self {
        Self { hint, text: "" }
    }

    pub fn text(mut self, text: &'static str) -> Self {
        self.text = text;
        self
    }
}

/// A `Chip`.
pub struct MaterialChip {
    pub label: &'static str,
    pub checkable: bool,
    pub checked: bool,
    pub closeable: bool,
    pub icon: Option<&'static JNIStr>,
    pub action: Option<&'static str>,
}

impl MaterialChip {
    pub fn new(label: &'static str) -> Self {
        Self {
            label,
            checkable: false,
            checked: false,
            closeable: false,
            icon: None,
            action: None,
        }
    }

    pub fn checkable(mut self, checkable: bool) -> Self {
        self.checkable = checkable;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn closeable(mut self, closeable: bool) -> Self {
        self.closeable = closeable;
        self
    }

    pub fn icon(mut self, icon: &'static JNIStr) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn action(mut self, tag: &'static str) -> Self {
        self.action = Some(tag);
        self
    }
}

/// A `ChipGroup` containing [`MaterialChip`]s.
pub struct MaterialChipGroup {
    pub chips: Vec<MaterialChip>,
    pub single_selection: bool,
    pub single_line: bool,
}

impl MaterialChipGroup {
    pub fn new(chips: Vec<MaterialChip>) -> Self {
        Self {
            chips,
            single_selection: false,
            single_line: false,
        }
    }

    pub fn single_selection(mut self, single_selection: bool) -> Self {
        self.single_selection = single_selection;
        self
    }

    pub fn single_line(mut self, single_line: bool) -> Self {
        self.single_line = single_line;
        self
    }
}

/// A `MaterialCardView` container.
pub struct MaterialCard {
    pub background: Option<jint>,
    pub radius_dp: f32,
    pub elevation_dp: f32,
    pub stroke_width_dp: f32,
    pub stroke_color: Option<jint>,
    pub padding: EdgeInsets,
    pub action: Option<&'static str>,
    pub children: Vec<Placed>,
}

impl MaterialCard {
    pub fn new(children: Vec<Placed>) -> Self {
        Self {
            background: None,
            radius_dp: 12.0,
            elevation_dp: 2.0,
            stroke_width_dp: 0.0,
            stroke_color: None,
            padding: EdgeInsets::all(16.0),
            action: None,
            children,
        }
    }

    pub fn background(mut self, color: jint) -> Self {
        self.background = Some(color);
        self
    }

    pub fn radius(mut self, radius_dp: f32) -> Self {
        self.radius_dp = radius_dp;
        self
    }

    pub fn elevation(mut self, elevation_dp: f32) -> Self {
        self.elevation_dp = elevation_dp;
        self
    }

    pub fn stroke(mut self, width_dp: f32, color: jint) -> Self {
        self.stroke_width_dp = width_dp;
        self.stroke_color = Some(color);
        self
    }

    pub fn padding(mut self, padding: EdgeInsets) -> Self {
        self.padding = padding;
        self
    }

    pub fn action(mut self, tag: &'static str) -> Self {
        self.action = Some(tag);
        self
    }
}

/// A `MaterialToolbar`.
pub struct MaterialToolbar {
    pub title: &'static str,
    pub subtitle: &'static str,
    pub background: jint,
    pub navigation_icon: &'static JNIStr,
}

impl MaterialToolbar {
    pub fn new(title: &'static str) -> Self {
        Self {
            title,
            subtitle: "",
            background: crate::constants::COLOR_WIDGETS_BACKGROUND,
            navigation_icon: crate::constants::DRAWABLE_MENU_REVERT,
        }
    }

    pub fn subtitle(mut self, subtitle: &'static str) -> Self {
        self.subtitle = subtitle;
        self
    }

    pub fn background(mut self, color: jint) -> Self {
        self.background = color;
        self
    }

    pub fn navigation_icon(mut self, icon: &'static JNIStr) -> Self {
        self.navigation_icon = icon;
        self
    }
}

/// An `AppBarLayout` container.
pub struct MaterialAppBar {
    pub background: Option<jint>,
    pub children: Vec<Placed>,
}

impl MaterialAppBar {
    pub fn new(children: Vec<Placed>) -> Self {
        Self {
            background: None,
            children,
        }
    }

    pub fn background(mut self, color: jint) -> Self {
        self.background = Some(color);
        self
    }
}

/// A `CollapsingToolbarLayout` container.
pub struct MaterialCollapsingToolbar {
    pub title: &'static str,
    pub subtitle: &'static str,
    pub background: Option<jint>,
    pub children: Vec<Placed>,
}

impl MaterialCollapsingToolbar {
    pub fn new(title: &'static str, children: Vec<Placed>) -> Self {
        Self {
            title,
            subtitle: "",
            background: None,
            children,
        }
    }

    pub fn subtitle(mut self, subtitle: &'static str) -> Self {
        self.subtitle = subtitle;
        self
    }

    pub fn background(mut self, color: jint) -> Self {
        self.background = Some(color);
        self
    }
}

/// A `BottomAppBar`.
pub struct MaterialBottomAppBar {
    pub title: &'static str,
    pub subtitle: &'static str,
    pub background: jint,
    pub navigation_icon: &'static JNIStr,
}

impl MaterialBottomAppBar {
    pub fn new(title: &'static str) -> Self {
        Self {
            title,
            subtitle: "",
            background: crate::constants::COLOR_WIDGETS_BACKGROUND,
            navigation_icon: crate::constants::DRAWABLE_MENU_REVERT,
        }
    }

    pub fn subtitle(mut self, subtitle: &'static str) -> Self {
        self.subtitle = subtitle;
        self
    }

    pub fn background(mut self, color: jint) -> Self {
        self.background = color;
        self
    }

    pub fn navigation_icon(mut self, icon: &'static JNIStr) -> Self {
        self.navigation_icon = icon;
        self
    }
}

/// A frame container for a `FloatingToolbarLayout` or `DockedToolbarLayout`.
pub struct MaterialToolbarLayout {
    pub orientation: Orientation,
    pub background: Option<jint>,
    pub children: Vec<Placed>,
}

impl MaterialToolbarLayout {
    pub fn new(children: Vec<Placed>) -> Self {
        Self {
            orientation: Orientation::Horizontal,
            background: None,
            children,
        }
    }

    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn background(mut self, color: jint) -> Self {
        self.background = Some(color);
        self
    }
}

/// A `FloatingActionButton`.
pub struct MaterialFab {
    pub icon: &'static JNIStr,
    pub size_dp: f32,
    pub background: Option<jint>,
    pub action: Option<&'static str>,
    pub badge: Option<jint>,
}

impl MaterialFab {
    pub fn new(icon: &'static JNIStr) -> Self {
        Self {
            icon,
            size_dp: 56.0,
            background: None,
            action: None,
            badge: None,
        }
    }

    pub fn size(mut self, size_dp: f32) -> Self {
        self.size_dp = size_dp;
        self
    }

    pub fn background(mut self, color: jint) -> Self {
        self.background = Some(color);
        self
    }

    pub fn action(mut self, tag: &'static str) -> Self {
        self.action = Some(tag);
        self
    }

    pub fn badge(mut self, number: jint) -> Self {
        self.badge = Some(number);
        self
    }
}

/// An `ExtendedFloatingActionButton`.
pub struct MaterialExtendedFab {
    pub label: &'static str,
    pub icon: Option<&'static JNIStr>,
    pub background: Option<jint>,
    pub action: Option<&'static str>,
    pub badge: Option<jint>,
}

impl MaterialExtendedFab {
    pub fn new(label: &'static str) -> Self {
        Self {
            label,
            icon: None,
            background: None,
            action: None,
            badge: None,
        }
    }

    pub fn icon(mut self, icon: &'static JNIStr) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn background(mut self, color: jint) -> Self {
        self.background = Some(color);
        self
    }

    pub fn action(mut self, tag: &'static str) -> Self {
        self.action = Some(tag);
        self
    }

    pub fn badge(mut self, number: jint) -> Self {
        self.badge = Some(number);
        self
    }
}

/// The kind of Material progress indicator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MaterialProgressKind {
    Linear,
    Circular,
}

/// A linear or circular Material progress indicator.
pub struct MaterialProgress {
    pub kind: MaterialProgressKind,
    pub progress: jint,
    pub max: jint,
    pub indeterminate: bool,
}

impl MaterialProgress {
    pub fn linear(progress: jint, max: jint) -> Self {
        Self {
            kind: MaterialProgressKind::Linear,
            progress,
            max,
            indeterminate: false,
        }
    }

    pub fn circular(progress: jint, max: jint) -> Self {
        Self {
            kind: MaterialProgressKind::Circular,
            progress,
            max,
            indeterminate: false,
        }
    }

    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }
}

/// A `LoadingIndicator`.
pub struct MaterialLoading {
    pub size_dp: f32,
    pub container_color: Option<jint>,
}

impl MaterialLoading {
    pub fn new(size_dp: f32) -> Self {
        Self {
            size_dp,
            container_color: None,
        }
    }

    pub fn container_color(mut self, color: jint) -> Self {
        self.container_color = Some(color);
        self
    }
}

/// A Material `Slider`.
pub struct MaterialSlider {
    pub value: f32,
    pub from: f32,
    pub to: f32,
    pub step: f32,
    pub enabled: bool,
}

impl MaterialSlider {
    pub fn new(value: f32, from: f32, to: f32) -> Self {
        Self {
            value,
            from,
            to,
            step: 0.0,
            enabled: true,
        }
    }

    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// A `MaterialDivider`.
pub struct MaterialDivider {
    pub thickness_dp: f32,
    pub color: Option<jint>,
    pub inset_start_dp: f32,
    pub inset_end_dp: f32,
}

impl MaterialDivider {
    pub fn new() -> Self {
        Self {
            thickness_dp: 1.0,
            color: None,
            inset_start_dp: 0.0,
            inset_end_dp: 0.0,
        }
    }

    pub fn thickness(mut self, thickness_dp: f32) -> Self {
        self.thickness_dp = thickness_dp;
        self
    }

    pub fn color(mut self, color: jint) -> Self {
        self.color = Some(color);
        self
    }

    pub fn insets(mut self, start_dp: f32, end_dp: f32) -> Self {
        self.inset_start_dp = start_dp;
        self.inset_end_dp = end_dp;
        self
    }
}

impl Default for MaterialDivider {
    fn default() -> Self {
        Self::new()
    }
}

/// A `ShapeableImageView`.
pub struct MaterialShapeableImage {
    pub drawable: &'static JNIStr,
    pub corner_radius_dp: f32,
    pub background: Option<jint>,
}

impl MaterialShapeableImage {
    pub fn new(drawable: &'static JNIStr) -> Self {
        Self {
            drawable,
            corner_radius_dp: 16.0,
            background: None,
        }
    }

    pub fn corner_radius(mut self, corner_radius_dp: f32) -> Self {
        self.corner_radius_dp = corner_radius_dp;
        self
    }

    pub fn background(mut self, color: jint) -> Self {
        self.background = Some(color);
        self
    }
}

/// A `TabLayout` with text tabs.
pub struct MaterialTabs {
    pub tabs: &'static [&'static str],
    pub selected: jint,
    pub scrollable: bool,
}

impl MaterialTabs {
    pub fn new(tabs: &'static [&'static str]) -> Self {
        Self {
            tabs,
            selected: 0,
            scrollable: false,
        }
    }

    pub fn selected(mut self, selected: jint) -> Self {
        self.selected = selected;
        self
    }

    pub fn scrollable(mut self, scrollable: bool) -> Self {
        self.scrollable = scrollable;
        self
    }
}

/// An item in a menu-backed navigation bar.
pub type MaterialNavItem = (&'static str, &'static JNIStr);

/// A `BottomNavigationView`.
pub struct MaterialBottomNavigation {
    pub items: &'static [MaterialNavItem],
    pub selected: jint,
}

impl MaterialBottomNavigation {
    pub fn new(items: &'static [MaterialNavItem]) -> Self {
        Self { items, selected: 0 }
    }

    pub fn selected(mut self, selected: jint) -> Self {
        self.selected = selected;
        self
    }
}

/// A `NavigationRailView`.
pub struct MaterialNavigationRail {
    pub items: &'static [MaterialNavItem],
    pub selected: jint,
}

impl MaterialNavigationRail {
    pub fn new(items: &'static [MaterialNavItem]) -> Self {
        Self { items, selected: 0 }
    }

    pub fn selected(mut self, selected: jint) -> Self {
        self.selected = selected;
        self
    }
}

/// A `NavigationView`.
pub struct MaterialNavigation {
    pub items: &'static [&'static str],
    pub selected: jint,
}

impl MaterialNavigation {
    pub fn new(items: &'static [&'static str]) -> Self {
        Self { items, selected: 0 }
    }

    pub fn selected(mut self, selected: jint) -> Self {
        self.selected = selected;
        self
    }
}

/// A Material `Carousel`, rendered with a `RecyclerView` and `CarouselLayoutManager`.
pub struct MaterialCarousel {
    pub items: &'static [&'static str],
    pub orientation: Orientation,
    pub has_fixed_size: bool,
}

impl MaterialCarousel {
    pub fn new(items: &'static [&'static str]) -> Self {
        Self {
            items,
            orientation: Orientation::Horizontal,
            has_fixed_size: false,
        }
    }

    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn has_fixed_size(mut self, has_fixed_size: bool) -> Self {
        self.has_fixed_size = has_fixed_size;
        self
    }
}
