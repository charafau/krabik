//! The widgets the app can demonstrate and the actions a button can trigger.

/// Every widget that has a demo screen. The main list is generated from [`WidgetKind::ALL`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WidgetKind {
    TextView,
    Button,
    EditText,
    ImageView,
    ImageButton,
    CheckBox,
    RadioButton,
    Switch,
    ToggleButton,
    CheckedTextView,
    ProgressBar,
    SeekBar,
    RatingBar,
    Spinner,
    AutoCompleteTextView,
    MultiAutoCompleteTextView,
    NumberPicker,
    DatePicker,
    TimePicker,
    Chronometer,
    TextClock,
    CalendarView,
    AnalogClock,
    SearchView,
    Toolbar,
    AppCompatToolbar,
    LinearLayout,
    RecyclerView,
    ViewPager2,
    MaterialButton,
    MaterialButtonToggleGroup,
    MaterialCheckBox,
    MaterialRadioButton,
    MaterialSwitch,
    MaterialText,
    MaterialTextField,
    MaterialDropdown,
    MaterialSearchBar,
    MaterialSearchView,
    MaterialChip,
    MaterialChipGroup,
    MaterialCard,
    MaterialToolbar,
    MaterialAppBar,
    MaterialCollapsingToolbar,
    MaterialBottomAppBar,
    MaterialFloatingToolbar,
    MaterialDockedToolbar,
    MaterialFab,
    MaterialExtendedFab,
    MaterialProgress,
    MaterialLoading,
    MaterialSlider,
    MaterialDivider,
    MaterialShapeableImage,
    MaterialTabs,
    MaterialBottomNavigation,
    MaterialNavigationRail,
    MaterialNavigation,
    MaterialCarousel,
    Snackbar,
    AlertDialog,
    MaterialDatePicker,
    MaterialTimePicker,
}

/// The labels and documentation that describe a [`WidgetKind`].
#[derive(Clone, Copy, Debug)]
pub struct WidgetInfo {
    /// Stable tag written to the button. It doubles as the navigation key.
    pub key: &'static str,
    pub title: &'static str,
    pub description: &'static str,
}

impl WidgetKind {
    pub const ALL: &'static [Self] = &[
        Self::TextView,
        Self::Button,
        Self::EditText,
        Self::ImageView,
        Self::ImageButton,
        Self::CheckBox,
        Self::RadioButton,
        Self::Switch,
        Self::ToggleButton,
        Self::CheckedTextView,
        Self::ProgressBar,
        Self::SeekBar,
        Self::RatingBar,
        Self::Spinner,
        Self::AutoCompleteTextView,
        Self::MultiAutoCompleteTextView,
        Self::NumberPicker,
        Self::DatePicker,
        Self::TimePicker,
        Self::Chronometer,
        Self::TextClock,
        Self::CalendarView,
        Self::AnalogClock,
        Self::SearchView,
        Self::Toolbar,
        Self::AppCompatToolbar,
        Self::LinearLayout,
        Self::RecyclerView,
        Self::ViewPager2,
        Self::MaterialButton,
        Self::MaterialButtonToggleGroup,
        Self::MaterialCheckBox,
        Self::MaterialRadioButton,
        Self::MaterialSwitch,
        Self::MaterialText,
        Self::MaterialTextField,
        Self::MaterialDropdown,
        Self::MaterialSearchBar,
        Self::MaterialSearchView,
        Self::MaterialChip,
        Self::MaterialChipGroup,
        Self::MaterialCard,
        Self::MaterialToolbar,
        Self::MaterialAppBar,
        Self::MaterialCollapsingToolbar,
        Self::MaterialBottomAppBar,
        Self::MaterialFloatingToolbar,
        Self::MaterialDockedToolbar,
        Self::MaterialFab,
        Self::MaterialExtendedFab,
        Self::MaterialProgress,
        Self::MaterialLoading,
        Self::MaterialSlider,
        Self::MaterialDivider,
        Self::MaterialShapeableImage,
        Self::MaterialTabs,
        Self::MaterialBottomNavigation,
        Self::MaterialNavigationRail,
        Self::MaterialNavigation,
        Self::MaterialCarousel,
        Self::Snackbar,
        Self::AlertDialog,
        Self::MaterialDatePicker,
        Self::MaterialTimePicker,
    ];

    pub fn info(self) -> WidgetInfo {
        use WidgetKind::*;

        let (key, title, description) = match self {
            TextView => (
                "textview",
                "TextView",
                "Read-only text with configurable size, colour, gravity, padding, and background.",
            ),
            Button => (
                "button",
                "Button",
                "A push button. The first one is wired to a Rust-owned tap counter.",
            ),
            EditText => (
                "edittext",
                "EditText",
                "Editable text fields with hints, initial values, padding, and input types.",
            ),
            ImageView => (
                "imageview",
                "ImageView",
                "Images from the framework's drawable resources.",
            ),
            ImageButton => (
                "imagebutton",
                "ImageButton",
                "A clickable button that displays an image.",
            ),
            CheckBox => ("checkbox", "CheckBox", "Two-state check boxes with labels."),
            RadioButton => (
                "radiobutton",
                "RadioButton",
                "Mutually exclusive options inside a RadioGroup.",
            ),
            Switch => ("switch", "Switch", "An on/off switch with a label."),
            ToggleButton => (
                "togglebutton",
                "ToggleButton",
                "A button that keeps a checked state and swaps its label.",
            ),
            CheckedTextView => (
                "checkedtextview",
                "CheckedTextView",
                "A text row with a check mark and checked state.",
            ),
            ProgressBar => (
                "progressbar",
                "ProgressBar",
                "Determinate and indeterminate progress indicators.",
            ),
            SeekBar => ("seekbar", "SeekBar", "A draggable progress slider."),
            RatingBar => ("ratingbar", "RatingBar", "A star rating control."),
            Spinner => (
                "spinner",
                "Spinner",
                "A drop-down selector backed by an ArrayAdapter built in Rust.",
            ),
            AutoCompleteTextView => (
                "autocomplete",
                "AutoCompleteTextView",
                "An editable field with completion suggestions.",
            ),
            MultiAutoCompleteTextView => (
                "multiautocomplete",
                "MultiAutoCompleteTextView",
                "Completion suggestions for multiple comma-separated values.",
            ),
            NumberPicker => (
                "numberpicker",
                "NumberPicker",
                "A scrollable number selector.",
            ),
            DatePicker => ("datepicker", "DatePicker", "A native date picker."),
            TimePicker => ("timepicker", "TimePicker", "A native time picker."),
            Chronometer => (
                "chronometer",
                "Chronometer",
                "A running elapsed-time clock.",
            ),
            TextClock => ("textclock", "TextClock", "A self-updating text clock."),
            CalendarView => (
                "calendarview",
                "CalendarView",
                "A scrollable month calendar.",
            ),
            AnalogClock => ("analogclock", "AnalogClock", "An analog clock face."),
            SearchView => (
                "searchview",
                "SearchView",
                "A search field with a query hint and submit button.",
            ),
            Toolbar => (
                "toolbar",
                "Toolbar",
                "An app bar with a title, subtitle, navigation icon, and overflow menu.",
            ),
            AppCompatToolbar => (
                "appcompattoolbar",
                "AppCompat Toolbar",
                "An androidx.appcompat.widget.Toolbar with title, subtitle, icon, and menu.",
            ),
            LinearLayout => (
                "linearlayout",
                "LinearLayout",
                "A real LinearLayout arranging children horizontally and vertically.",
            ),
            RecyclerView => (
                "recyclerview",
                "RecyclerView",
                "A scrolling list and grid backed by an AndroidX RecyclerView.Adapter.",
            ),
            ViewPager2 => (
                "viewpager2",
                "ViewPager2",
                "A horizontally paged list backed by AndroidX ViewPager2.",
            ),
            MaterialButton => (
                "materialbutton",
                "Material Button",
                "MaterialButton from com.google.android.material with filled, outlined, text, and icon variants.",
            ),
            MaterialButtonToggleGroup => (
                "materialtogglegroup",
                "Material Toggle Group",
                "MaterialButtonToggleGroup keeps its checkable MaterialButtons mutually exclusive.",
            ),
            MaterialCheckBox => (
                "materialcheckbox",
                "Material CheckBox",
                "A MaterialCheckBox, the Material-themed replacement for android.widget.CheckBox.",
            ),
            MaterialRadioButton => (
                "materialradiobutton",
                "Material Radio Group",
                "MaterialRadioButtons inside a RadioGroup with Material theming.",
            ),
            MaterialSwitch => (
                "materialswitch",
                "Material Switch",
                "MaterialSwitch uses the Material 3 switch track, thumb, and icon treatment.",
            ),
            MaterialText => (
                "materialtext",
                "Material TextView",
                "A MaterialTextView with Material typography defaults and explicit styling.",
            ),
            MaterialTextField => (
                "materialtextfield",
                "Material Text Field",
                "TextInputLayout wrapping a TextInputEditText with hints, helper text, errors, and a password toggle.",
            ),
            MaterialDropdown => (
                "materialdropdown",
                "Material Dropdown",
                "A TextInputLayout with an exposed MaterialAutoCompleteTextView dropdown.",
            ),
            MaterialSearchBar => (
                "materialsearchbar",
                "Material Search Bar",
                "A Material SearchBar with a hint, title, navigation icon, and text.",
            ),
            MaterialSearchView => (
                "materialsearchview",
                "Material Search View",
                "A Material SearchView container with a search hint.",
            ),
            MaterialChip => (
                "materialchip",
                "Material Chips",
                "Chip variants: assist, filter (checkable), input (closeable), and icon chips.",
            ),
            MaterialChipGroup => (
                "materialchipgroup",
                "Material Chip Group",
                "A single-selection ChipGroup that lays out its chips and enforces exclusivity.",
            ),
            MaterialCard => (
                "materialcard",
                "Material Card",
                "MaterialCardView with rounded corners, elevation, a stroke, padding, and children.",
            ),
            MaterialToolbar => (
                "materialtoolbar",
                "Material Toolbar",
                "MaterialToolbar with a title, subtitle, navigation icon, and overflow menu.",
            ),
            MaterialAppBar => (
                "materialappbar",
                "Material App Bar",
                "AppBarLayout with a MaterialToolbar child using app-bar scroll flags.",
            ),
            MaterialCollapsingToolbar => (
                "materialcollapsingtoolbar",
                "Collapsing Toolbar",
                "CollapsingToolbarLayout with a title, subtitle, and a child MaterialToolbar.",
            ),
            MaterialBottomAppBar => (
                "materialbottomappbar",
                "Bottom App Bar",
                "BottomAppBar with a title, subtitle, navigation icon, and menu items.",
            ),
            MaterialFloatingToolbar => (
                "materialfloatingtoolbar",
                "Floating Toolbar",
                "FloatingToolbarLayout with MaterialButtons arranged horizontally.",
            ),
            MaterialDockedToolbar => (
                "materialdockettoolbar",
                "Docked Toolbar",
                "DockedToolbarLayout with MaterialButtons arranged horizontally.",
            ),
            MaterialFab => (
                "materialfab",
                "Material FAB",
                "FloatingActionButton with a custom size, icon, background tint, and a badge.",
            ),
            MaterialExtendedFab => (
                "materialextendedfab",
                "Extended FAB",
                "ExtendedFloatingActionButton with an icon, label, background tint, and badge.",
            ),
            MaterialProgress => (
                "materialprogress",
                "Material Progress",
                "Linear and circular Material progress indicators, determinate and indeterminate.",
            ),
            MaterialLoading => (
                "materialloading",
                "Loading Indicator",
                "A Material 3 LoadingIndicator with a custom indicator size and container colour.",
            ),
            MaterialSlider => (
                "materialslider",
                "Material Slider",
                "A Material Slider with a value range, step size, and initial value.",
            ),
            MaterialDivider => (
                "materialdivider",
                "Material Divider",
                "MaterialDivider with configurable thickness, colour, and start/end insets.",
            ),
            MaterialShapeableImage => (
                "materialshapeimage",
                "Shapeable Image",
                "ShapeableImageView with a ShapeAppearanceModel that rounds every corner.",
            ),
            MaterialTabs => (
                "materialtabs",
                "Material Tabs",
                "TabLayout with text tabs in fixed or scrollable mode.",
            ),
            MaterialBottomNavigation => (
                "materialbottomnav",
                "Bottom Navigation",
                "BottomNavigationView with menu items created from Rust.",
            ),
            MaterialNavigationRail => (
                "materialnavigationrail",
                "Navigation Rail",
                "NavigationRailView with menu items created from Rust.",
            ),
            MaterialNavigation => (
                "materialnavigation",
                "Navigation Drawer",
                "NavigationView with menu items added through NavigationView.getMenu().",
            ),
            MaterialCarousel => (
                "materialcarousel",
                "Material Carousel",
                "A Material carousel rendered with RecyclerView and CarouselLayoutManager.",
            ),
            Snackbar => (
                "snackbar",
                "Snackbar",
                "Snackbar.make() shows a transient message anchored to the tapped button.",
            ),
            AlertDialog => (
                "alertdialog",
                "Alert Dialog",
                "MaterialAlertDialogBuilder shows a themed title, message, and OK button.",
            ),
            MaterialDatePicker => (
                "materialdatepicker",
                "Material Date Picker",
                "MaterialDatePicker opens in a DialogFragment; the activity must be an AppCompatActivity.",
            ),
            MaterialTimePicker => (
                "materialtimepicker",
                "Material Time Picker",
                "MaterialTimePicker opens in a DialogFragment; the activity must be an AppCompatActivity.",
            ),
        };

        WidgetInfo {
            key,
            title,
            description,
        }
    }

    pub fn key(self) -> &'static str {
        self.info().key
    }

    pub fn title(self) -> &'static str {
        self.info().title
    }

    pub fn description(self) -> &'static str {
        self.info().description
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|kind| kind.key() == key)
    }
}

/// What a button does when it is tapped. The variant is stored as the view's tag.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Tap,
    Back,
    Open(WidgetKind),
    ShowSnackbar,
    ShowAlertDialog,
    ShowDatePicker,
    ShowTimePicker,
}

impl Action {
    pub fn tag(self) -> &'static str {
        match self {
            Self::Tap => "tap",
            Self::Back => "back",
            Self::Open(kind) => kind.key(),
            Self::ShowSnackbar => "show-snackbar",
            Self::ShowAlertDialog => "show-alert-dialog",
            Self::ShowDatePicker => "show-date-picker",
            Self::ShowTimePicker => "show-time-picker",
        }
    }

    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "tap" => Some(Self::Tap),
            "back" => Some(Self::Back),
            "show-snackbar" => Some(Self::ShowSnackbar),
            "show-alert-dialog" => Some(Self::ShowAlertDialog),
            "show-date-picker" => Some(Self::ShowDatePicker),
            "show-time-picker" => Some(Self::ShowTimePicker),
            _ => WidgetKind::from_key(tag).map(Self::Open),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_keys_round_trip() {
        for &kind in WidgetKind::ALL {
            assert_eq!(WidgetKind::from_key(kind.key()), Some(kind));
            assert_eq!(Action::from_tag(kind.key()), Some(Action::Open(kind)));
            assert!(!kind.title().is_empty());
            assert!(!kind.description().is_empty());
        }
    }

    #[test]
    fn button_actions_round_trip() {
        for action in [
            Action::Tap,
            Action::Back,
            Action::ShowSnackbar,
            Action::ShowAlertDialog,
            Action::ShowDatePicker,
            Action::ShowTimePicker,
        ] {
            assert_eq!(Action::from_tag(action.tag()), Some(action));
        }
        assert_eq!(Action::from_tag("unknown"), None);
    }

    #[test]
    fn widget_keys_are_unique() {
        let mut keys: Vec<_> = WidgetKind::ALL.iter().map(|kind| kind.key()).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), WidgetKind::ALL.len());
    }
}
