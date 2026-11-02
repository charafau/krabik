//! One demo screen per Android widget.
//!
//! The content inside a demo's preview card is pure data: platform-specific behaviour (clicks,
//! checked state, scrolling) is provided by the real Android widget.

use krabik_ui_android::constants::*;
use krabik_ui_android::ui::{
    AppCompatToolbar, AutoComplete, Button, CheckBox, CheckedText, EdgeInsets, EditText, Element,
    Image, ImageButton, Length, Linear, MaterialAppBar, MaterialBottomAppBar,
    MaterialBottomNavigation, MaterialBoxMode, MaterialButton, MaterialButtonToggleGroup,
    MaterialCard, MaterialCarousel, MaterialCheckBox, MaterialChip, MaterialChipGroup,
    MaterialCollapsingToolbar, MaterialDivider, MaterialDropdown, MaterialExtendedFab, MaterialFab,
    MaterialLoading, MaterialNavigation, MaterialNavigationRail, MaterialProgress,
    MaterialRadioButton, MaterialRadioGroup, MaterialSearchBar, MaterialSearchView,
    MaterialShapeableImage, MaterialSlider, MaterialSwitch, MaterialTabs, MaterialText,
    MaterialTextField, MaterialToolbar, MaterialToolbarLayout, MultiAutoComplete, NumberPicker,
    Orientation, Placed, Progress, RadioButton, RadioGroup, Rating, Recycler, Scroll, Search, Seek,
    Spinner, Switch, Text, TextClock, ToggleButton, Toolbar, ViewPager,
};

use super::{description, header, heading, note};
use crate::android::widget::{Action, WidgetKind};

const SPINNER_COLORS: &[&str] = &["Red", "Green", "Blue", "Purple"];
const FRUITS: &[&str] = &["Apple", "Apricot", "Banana", "Blueberry", "Cherry", "Grape"];
const NAMES: &[&str] = &["Ada", "Alan", "Grace", "Linus", "Margaret"];
const LIST_ITEMS: &[&str] = &[
    "RecyclerView item 1",
    "RecyclerView item 2",
    "RecyclerView item 3",
    "RecyclerView item 4",
    "RecyclerView item 5",
    "RecyclerView item 6",
];
const PAGER_ITEMS: &[&str] = &["Page 1", "Page 2", "Page 3", "Page 4"];

pub(super) fn build(kind: WidgetKind) -> Element {
    let content = Linear {
        orientation: Orientation::Vertical,
        padding: EdgeInsets::new(20.0, 56.0, 20.0, 32.0),
        background: None,
        children: vec![
            header(kind.title()),
            Placed::new(
                Text::new(kind.description())
                    .size(14.0)
                    .color(COLOR_TEXT_SECONDARY),
            )
            .margins(EdgeInsets::bottom(16.0)),
            Placed::new(Linear {
                orientation: Orientation::Vertical,
                padding: EdgeInsets::all(16.0),
                background: Some(COLOR_WIDGETS_BACKGROUND),
                children: content(kind),
            }),
        ],
    };

    Element::Scroll(Scroll {
        background: Some(COLOR_SCREEN_BACKGROUND),
        child: Box::new(Placed::new(content).width(Length::MatchParent)),
    })
}

/// The content shown inside a widget demo's preview card.
fn content(kind: WidgetKind) -> Vec<Placed> {
    match kind {
        WidgetKind::TextView => vec![
            heading("TextView"),
            description(
                "A TextView displays read-only text. Size, colour, gravity, padding, and background are all set from Rust.",
            ),
            Placed::new(
                Text::new("Centered and blue")
                    .size(18.0)
                    .color(COLOR_BLUE)
                    .gravity(GRAVITY_CENTER_HORIZONTAL)
                    .background(COLOR_CHIP_BACKGROUND)
                    .padding(EdgeInsets::all(12.0)),
            ),
        ],
        WidgetKind::Button => vec![
            heading("Button"),
            description(
                "Buttons are native android.widget.Button instances. The first one is wired to a Rust-owned tap counter.",
            ),
            Placed::new(Element::Status(
                Text::new("Tapped 0 times").size(18.0).color(COLOR_BLUE),
            ))
            .margins(EdgeInsets::bottom(8.0)),
            Placed::new(Button::new("Tap me").size(16.0).action(Action::Tap.tag()))
                .width(Length::MatchParent)
                .height(Length::Dp(48.0))
                .margins(EdgeInsets::bottom(8.0)),
            Placed::new(Button::new("All caps").size(14.0).all_caps(true))
                .width(Length::MatchParent)
                .height(Length::Dp(48.0))
                .margins(EdgeInsets::bottom(8.0)),
            Placed::new(Button::new("Disabled").size(14.0).enabled(false))
                .width(Length::MatchParent)
                .height(Length::Dp(48.0)),
            note("The disabled button does not receive clicks."),
        ],
        WidgetKind::EditText => vec![
            heading("EditText"),
            description(
                "Editable text fields with hints, initial values, padding, and input types.",
            ),
            Placed::new(
                EditText::new("Type something...")
                    .size(15.0)
                    .background(COLOR_ICON_BACKGROUND)
                    .padding(EdgeInsets::new(12.0, 10.0, 12.0, 10.0)),
            )
            .width(Length::MatchParent)
            .height(Length::Dp(48.0))
            .margins(EdgeInsets::bottom(8.0)),
            Placed::new(
                EditText::new("Prefilled")
                    .text("Hello from Rust")
                    .size(15.0)
                    .background(COLOR_ICON_BACKGROUND)
                    .padding(EdgeInsets::new(12.0, 10.0, 12.0, 10.0)),
            )
            .width(Length::MatchParent)
            .height(Length::Dp(48.0))
            .margins(EdgeInsets::bottom(8.0)),
            Placed::new(
                EditText::new("Password")
                    .input_type(INPUT_TYPE_TEXT_PASSWORD)
                    .size(15.0)
                    .background(COLOR_ICON_BACKGROUND)
                    .padding(EdgeInsets::new(12.0, 10.0, 12.0, 10.0)),
            )
            .width(Length::MatchParent)
            .height(Length::Dp(48.0)),
            note("The last field uses InputType.TYPE_CLASS_TEXT | TYPE_TEXT_VARIATION_PASSWORD."),
        ],
        WidgetKind::ImageView => vec![
            heading("ImageView"),
            description("ImageViews backed by framework drawable resources."),
            Placed::new(Linear {
                orientation: Orientation::Horizontal,
                padding: EdgeInsets::ZERO,
                background: None,
                children: vec![
                    Placed::new(Image::new(DRAWABLE_DIALOG_INFO).background(COLOR_ICON_BACKGROUND))
                        .width(Length::Dp(72.0))
                        .height(Length::Dp(72.0))
                        .margins(EdgeInsets::right(12.0)),
                    Placed::new(Image::new(DRAWABLE_MENU_CAMERA).background(COLOR_ICON_BACKGROUND))
                        .width(Length::Dp(72.0))
                        .height(Length::Dp(72.0)),
                ],
            })
            .margins(EdgeInsets::bottom(8.0)),
            note("ic_dialog_info and ic_menu_camera from android.R.drawable."),
        ],
        WidgetKind::ImageButton => vec![
            heading("ImageButton"),
            description("An ImageButton combines a clickable button with an image."),
            Placed::new(ImageButton::new(DRAWABLE_MENU_CAMERA).background(COLOR_ICON_BACKGROUND))
                .width(Length::Dp(72.0))
                .height(Length::Dp(72.0))
                .margins(EdgeInsets::bottom(8.0)),
            note("Rendered with android.widget.ImageButton."),
        ],
        WidgetKind::CheckBox => vec![
            heading("CheckBox"),
            description("CheckBox extends CompoundButton and keeps its own checked state."),
            Placed::new(CheckBox::new("Checked option").checked(true))
                .height(Length::Dp(48.0))
                .margins(EdgeInsets::bottom(8.0)),
            Placed::new(CheckBox::new("Unchecked option")).height(Length::Dp(48.0)),
        ],
        WidgetKind::RadioButton => vec![
            heading("RadioButton"),
            description("RadioButtons inside a RadioGroup are mutually exclusive."),
            Placed::new(RadioGroup {
                options: vec![
                    RadioButton::new("Option A").checked(true),
                    RadioButton::new("Option B"),
                    RadioButton::new("Option C"),
                ],
            })
            .margins(EdgeInsets::bottom(8.0)),
            note("The RadioGroup is created and populated from Rust."),
        ],
        WidgetKind::Switch => vec![
            heading("Switch"),
            description("Switch is a two-state toggle with a label."),
            Placed::new(Switch::new("Wi-Fi").checked(true))
                .height(Length::Dp(48.0))
                .margins(EdgeInsets::bottom(8.0)),
            Placed::new(Switch::new("Bluetooth")).height(Length::Dp(48.0)),
        ],
        WidgetKind::ToggleButton => vec![
            heading("ToggleButton"),
            description("A ToggleButton keeps a checked state and swaps between two labels."),
            Placed::new(ToggleButton::new("ON", "OFF").checked(true))
                .width(Length::MatchParent)
                .height(Length::Dp(48.0))
                .margins(EdgeInsets::bottom(8.0)),
            Placed::new(ToggleButton::new("ENABLED", "DISABLED"))
                .width(Length::MatchParent)
                .height(Length::Dp(48.0)),
            note("Tap a toggle to switch its label and state."),
        ],
        WidgetKind::CheckedTextView => vec![
            heading("CheckedTextView"),
            description("CheckedTextView adds a check mark and checked state to a text row."),
            Placed::new(CheckedText::new("Checked item").checked(true))
                .width(Length::MatchParent)
                .height(Length::Dp(48.0))
                .margins(EdgeInsets::bottom(8.0)),
            Placed::new(CheckedText::new("Unchecked item"))
                .width(Length::MatchParent)
                .height(Length::Dp(48.0)),
        ],
        WidgetKind::ProgressBar => vec![
            heading("ProgressBar"),
            description("ProgressBar supports determinate and indeterminate modes."),
            Placed::new(
                Text::new("Determinate: 65%")
                    .size(13.0)
                    .color(COLOR_TEXT_SECONDARY),
            )
            .margins(EdgeInsets::bottom(4.0)),
            Placed::new(Progress::horizontal(65, 100))
                .width(Length::MatchParent)
                .height(Length::Dp(24.0))
                .margins(EdgeInsets::bottom(12.0)),
            Placed::new(
                Text::new("Indeterminate")
                    .size(13.0)
                    .color(COLOR_TEXT_SECONDARY),
            )
            .margins(EdgeInsets::bottom(4.0)),
            Placed::new(Progress::spinner())
                .width(Length::Dp(48.0))
                .height(Length::Dp(48.0)),
        ],
        WidgetKind::SeekBar => vec![
            heading("SeekBar"),
            description("SeekBar is a draggable progress slider."),
            Placed::new(
                Text::new("Initial progress: 35%")
                    .size(13.0)
                    .color(COLOR_TEXT_SECONDARY),
            )
            .margins(EdgeInsets::bottom(4.0)),
            Placed::new(Seek::new(35, 100))
                .width(Length::MatchParent)
                .height(Length::Dp(48.0)),
            note("Drag the thumb to change the native progress value."),
        ],
        WidgetKind::RatingBar => vec![
            heading("RatingBar"),
            description(
                "RatingBar is a star rating control with configurable stars and step size.",
            ),
            Placed::new(Rating::new(3.5, 5).step_size(0.5))
                .width(Length::MatchParent)
                .height(Length::Dp(48.0))
                .margins(EdgeInsets::bottom(8.0)),
            note("Five stars, a 0.5 step, and an initial rating of 3.5."),
        ],
        WidgetKind::Spinner => vec![
            heading("Spinner"),
            description("Spinner presents a drop-down list. Its ArrayAdapter is built from Rust."),
            Placed::new(Spinner::new(SPINNER_COLORS).selected(1))
                .width(Length::MatchParent)
                .height(Length::Dp(48.0))
                .margins(EdgeInsets::bottom(8.0)),
            note("Items: Red, Green, Blue, Purple. Green is selected initially."),
        ],
        WidgetKind::AutoCompleteTextView => vec![
            heading("AutoCompleteTextView"),
            description("An editable text field that offers completion suggestions."),
            Placed::new(AutoComplete::new("Type a fruit", FRUITS).threshold(1))
                .width(Length::MatchParent)
                .height(Length::Dp(48.0))
                .margins(EdgeInsets::bottom(8.0)),
            note("Start typing to see suggestions from the ArrayAdapter."),
        ],
        WidgetKind::MultiAutoCompleteTextView => vec![
            heading("MultiAutoCompleteTextView"),
            description("Completion suggestions for multiple comma-separated values."),
            Placed::new(
                MultiAutoComplete::new("Type names, separated by commas", NAMES).threshold(1),
            )
            .width(Length::MatchParent)
            .height(Length::Dp(48.0))
            .margins(EdgeInsets::bottom(8.0)),
            note("Uses MultiAutoCompleteTextView.CommaTokenizer."),
        ],
        WidgetKind::NumberPicker => vec![
            heading("NumberPicker"),
            description("A scrollable number selector."),
            Placed::new(NumberPicker::new(0, 10, 4))
                .width(Length::Dp(120.0))
                .height(Length::Dp(160.0))
                .margins(EdgeInsets::bottom(8.0)),
            note("Range 0-10 with 4 selected initially."),
        ],
        WidgetKind::DatePicker => vec![
            heading("DatePicker"),
            description("The native Android date picker."),
            Placed::new(Element::DatePicker).margins(EdgeInsets::bottom(8.0)),
            note("Initialised with the current date."),
        ],
        WidgetKind::TimePicker => vec![
            heading("TimePicker"),
            description("The native Android time picker."),
            Placed::new(Element::TimePicker).margins(EdgeInsets::bottom(8.0)),
            note("Initialised with the current time."),
        ],
        WidgetKind::Chronometer => vec![
            heading("Chronometer"),
            description("A running elapsed-time clock."),
            Placed::new(Element::Chronometer)
                .height(Length::Dp(48.0))
                .margins(EdgeInsets::bottom(8.0)),
            note("Started with SystemClock.elapsedRealtime() as its base."),
        ],
        WidgetKind::TextClock => vec![
            heading("TextClock"),
            description("TextClock is a TextView that updates itself with the current time."),
            Placed::new(TextClock::new(32.0))
                .height(Length::Dp(56.0))
                .margins(EdgeInsets::bottom(8.0)),
            note("The clock follows the device's 12/24-hour setting."),
        ],
        WidgetKind::CalendarView => vec![
            heading("CalendarView"),
            description("A scrollable month calendar."),
            Placed::new(Element::Calendar)
                .width(Length::MatchParent)
                .height(Length::Dp(300.0))
                .margins(EdgeInsets::bottom(8.0)),
            note("The selected date starts at today."),
        ],
        WidgetKind::AnalogClock => vec![
            heading("AnalogClock"),
            description("An analog clock face."),
            Placed::new(Element::AnalogClock)
                .width(Length::Dp(160.0))
                .height(Length::Dp(160.0))
                .margins(EdgeInsets::bottom(8.0)),
            note("The hands follow the device time."),
        ],
        WidgetKind::SearchView => vec![
            heading("SearchView"),
            description("A search field with a query hint and submit button."),
            Placed::new(Search::new("Search widgets..."))
                .width(Length::MatchParent)
                .height(Length::Dp(56.0))
                .margins(EdgeInsets::bottom(8.0)),
            note("Created with iconifiedByDefault(false)."),
        ],
        WidgetKind::Toolbar => vec![
            heading("Toolbar"),
            description(
                "A native android.widget.Toolbar with a title, subtitle, navigation icon, and overflow menu.",
            ),
            Placed::new(Toolbar {
                title: "Rust Toolbar",
                subtitle: "Built with JNI",
                background: COLOR_WIDGETS_BACKGROUND,
                navigation_icon: DRAWABLE_MENU_REVERT,
            })
            .width(Length::MatchParent)
            .height(Length::Dp(72.0))
            .margins(EdgeInsets::bottom(8.0)),
            note("Menu items are added through Toolbar.getMenu() from Rust."),
        ],
        WidgetKind::AppCompatToolbar => vec![
            heading("AppCompat Toolbar"),
            description(
                "An androidx.appcompat.widget.Toolbar with a title, subtitle, navigation icon, and overflow menu.",
            ),
            Placed::new(AppCompatToolbar {
                title: "AppCompat Toolbar",
                subtitle: "Built from Rust",
                background: COLOR_WIDGETS_BACKGROUND,
                navigation_icon: DRAWABLE_MENU_REVERT,
            })
            .width(Length::MatchParent)
            .height(Length::Dp(72.0))
            .margins(EdgeInsets::bottom(8.0)),
            note("Provided by androidx.appcompat:appcompat; menu items are added from Rust."),
        ],
        WidgetKind::LinearLayout => vec![
            heading("LinearLayout"),
            description(
                "LinearLayout arranges its children in a single direction. The layouts below are real Android views nested inside the demo card.",
            ),
            Placed::new(
                Text::new("Horizontal orientation")
                    .size(13.0)
                    .color(COLOR_TEXT_SECONDARY),
            )
            .margins(EdgeInsets::bottom(4.0)),
            Placed::new(Linear {
                orientation: Orientation::Horizontal,
                padding: EdgeInsets::all(8.0),
                background: Some(COLOR_ICON_BACKGROUND),
                children: vec![
                    Placed::new(
                        Text::new("One")
                            .size(14.0)
                            .color(COLOR_BLUE)
                            .gravity(GRAVITY_CENTER_HORIZONTAL)
                            .background(COLOR_CHIP_BACKGROUND)
                            .padding(EdgeInsets::all(8.0)),
                    )
                    .width(Length::Dp(64.0))
                    .margins(EdgeInsets::right(8.0)),
                    Placed::new(
                        Text::new("Two")
                            .size(14.0)
                            .color(COLOR_BLUE)
                            .gravity(GRAVITY_CENTER_HORIZONTAL)
                            .background(COLOR_CHIP_BACKGROUND)
                            .padding(EdgeInsets::all(8.0)),
                    )
                    .width(Length::Dp(64.0))
                    .margins(EdgeInsets::right(8.0)),
                    Placed::new(
                        Text::new("Three")
                            .size(14.0)
                            .color(COLOR_BLUE)
                            .gravity(GRAVITY_CENTER_HORIZONTAL)
                            .background(COLOR_CHIP_BACKGROUND)
                            .padding(EdgeInsets::all(8.0)),
                    )
                    .width(Length::Dp(64.0)),
                ],
            })
            .width(Length::MatchParent)
            .margins(EdgeInsets::bottom(12.0)),
            Placed::new(
                Text::new("Vertical orientation")
                    .size(13.0)
                    .color(COLOR_TEXT_SECONDARY),
            )
            .margins(EdgeInsets::bottom(4.0)),
            Placed::new(Linear {
                orientation: Orientation::Vertical,
                padding: EdgeInsets::all(8.0),
                background: Some(COLOR_ICON_BACKGROUND),
                children: vec![
                    Placed::new(
                        Text::new("Row A")
                            .size(14.0)
                            .color(COLOR_TEXT_PRIMARY)
                            .padding(EdgeInsets::all(6.0)),
                    )
                    .width(Length::MatchParent)
                    .margins(EdgeInsets::bottom(6.0)),
                    Placed::new(
                        Text::new("Row B")
                            .size(14.0)
                            .color(COLOR_TEXT_PRIMARY)
                            .padding(EdgeInsets::all(6.0)),
                    )
                    .width(Length::MatchParent),
                ],
            })
            .width(Length::MatchParent)
            .margins(EdgeInsets::bottom(8.0)),
            note(
                "Orientation, padding, background, margins, and child sizes are all set from Rust.",
            ),
        ],
        WidgetKind::RecyclerView => vec![
            heading("RecyclerView"),
            description(
                "An AndroidX RecyclerView with a LinearLayoutManager and an adapter supplied by the app.",
            ),
            Placed::new(Recycler::new(LIST_ITEMS).has_fixed_size(true))
                .width(Length::MatchParent)
                .height(Length::Dp(280.0))
                .margins(EdgeInsets::bottom(12.0)),
            heading("Grid layout"),
            Placed::new(Recycler::new(LIST_ITEMS).grid(2))
                .width(Length::MatchParent)
                .height(Length::Dp(200.0)),
            note("The first list uses LinearLayoutManager; the second uses GridLayoutManager(2)."),
        ],
        WidgetKind::ViewPager2 => vec![
            heading("ViewPager2"),
            description("An AndroidX ViewPager2 whose pages come from the same item adapter."),
            Placed::new(ViewPager::new(PAGER_ITEMS))
                .width(Length::MatchParent)
                .height(Length::Dp(220.0))
                .margins(EdgeInsets::bottom(8.0)),
            note("Swipe horizontally to move between pages."),
        ],
        WidgetKind::MaterialButton => vec![
            heading("Material Button"),
            description(
                "MaterialButton is the Material-themed button. Buttons can be filled, outlined, text-only, or icon-bearing.",
            ),
            Placed::new(MaterialButton::new("Filled button").action(Action::Tap.tag()))
                .width(Length::MatchParent)
                .height(Length::Dp(48.0))
                .margins(EdgeInsets::bottom(8.0)),
            Placed::new(MaterialButton::new("Upper case label").all_caps(true))
                .width(Length::MatchParent)
                .height(Length::Dp(48.0))
                .margins(EdgeInsets::bottom(8.0)),
            Placed::new(
                MaterialButton::new("With icon")
                    .icon(DRAWABLE_MENU_CAMERA)
                    .size(15.0),
            )
            .width(Length::MatchParent)
            .height(Length::Dp(48.0))
            .margins(EdgeInsets::bottom(8.0)),
            Placed::new(MaterialButton::new("Disabled").enabled(false))
                .width(Length::MatchParent)
                .height(Length::Dp(48.0)),
            note("Tap the first button; the tap counter is owned by Rust."),
        ],
        WidgetKind::MaterialButtonToggleGroup => vec![
            heading("Material Toggle Group"),
            description(
                "MaterialButtonToggleGroup makes its checkable MaterialButtons behave like a segmented control.",
            ),
            Placed::new(MaterialButtonToggleGroup::new(vec![
                MaterialButton::new("Day").checked(true),
                MaterialButton::new("Week"),
                MaterialButton::new("Month"),
            ]))
            .width(Length::MatchParent)
            .margins(EdgeInsets::bottom(8.0)),
            note("Single selection is enabled by default; exactly one option stays checked."),
        ],
        WidgetKind::MaterialCheckBox => vec![
            heading("Material CheckBox"),
            description(
                "MaterialCheckBox is the Material themed CompoundButton with a 48dp touch target.",
            ),
            Placed::new(MaterialCheckBox::new("Checked option").checked(true))
                .height(Length::Dp(48.0))
                .margins(EdgeInsets::bottom(8.0)),
            Placed::new(MaterialCheckBox::new("Unchecked option")).height(Length::Dp(48.0)),
        ],
        WidgetKind::MaterialRadioButton => vec![
            heading("Material Radio Group"),
            description("MaterialRadioButtons inside a RadioGroup remain mutually exclusive."),
            Placed::new(MaterialRadioGroup::new(vec![
                MaterialRadioButton::new("Small").checked(true),
                MaterialRadioButton::new("Medium"),
                MaterialRadioButton::new("Large"),
            ]))
            .margins(EdgeInsets::bottom(8.0)),
            note("Selecting one radio button clears the others."),
        ],
        WidgetKind::MaterialSwitch => vec![
            heading("Material Switch"),
            description(
                "MaterialSwitch uses the Material 3 switch track, thumb, and optional icon treatment.",
            ),
            Placed::new(MaterialSwitch::new("Wi-Fi").checked(true))
                .height(Length::Dp(48.0))
                .margins(EdgeInsets::bottom(8.0)),
            Placed::new(MaterialSwitch::new("Bluetooth")).height(Length::Dp(48.0)),
        ],
        WidgetKind::MaterialText => vec![
            heading("Material TextView"),
            description(
                "MaterialTextView is a TextView with Material typography defaults. Size and colour are still set explicitly from Rust.",
            ),
            Placed::new(
                MaterialText::new("Headline in Material style")
                    .size(22.0)
                    .color(COLOR_BLUE)
                    .gravity(GRAVITY_CENTER_HORIZONTAL)
                    .padding(EdgeInsets::all(12.0))
                    .background(COLOR_CHIP_BACKGROUND),
            )
            .width(Length::MatchParent)
            .margins(EdgeInsets::bottom(8.0)),
            Placed::new(
                MaterialText::new("Supporting body copy with a smaller size.")
                    .size(14.0)
                    .color(COLOR_TEXT_SECONDARY),
            ),
        ],
        WidgetKind::MaterialTextField => vec![
            heading("Material Text Field"),
            description(
                "TextInputLayout adds a floating label, helper text, error text, and end icons to a TextInputEditText.",
            ),
            Placed::new(
                MaterialTextField::new("Email address")
                    .text("someone@example.com")
                    .helper_text("We never share your address."),
            )
            .width(Length::MatchParent)
            .margins(EdgeInsets::bottom(12.0)),
            Placed::new(
                MaterialTextField::new("Password")
                    .password_toggle(true)
                    .input_type(INPUT_TYPE_TEXT_PASSWORD),
            )
            .width(Length::MatchParent)
            .margins(EdgeInsets::bottom(12.0)),
            Placed::new(
                MaterialTextField::new("Username")
                    .error_text("This name is already taken")
                    .box_mode(MaterialBoxMode::Filled),
            )
            .width(Length::MatchParent),
            note("The password field shows the Material end-icon toggle."),
        ],
        WidgetKind::MaterialDropdown => vec![
            heading("Material Dropdown"),
            description(
                "A TextInputLayout with an exposed MaterialAutoCompleteTextView shows a Material dropdown menu.",
            ),
            Placed::new(MaterialDropdown::new("Favourite fruit", FRUITS))
                .width(Length::MatchParent)
                .margins(EdgeInsets::bottom(12.0)),
            note("Tap the field or the dropdown icon to choose a suggestion."),
        ],
        WidgetKind::MaterialSearchBar => vec![
            heading("Material Search Bar"),
            description(
                "SearchBar is a toolbar-styled search field. It can show a title, hint, and navigation icon.",
            ),
            Placed::new(
                MaterialSearchBar::new("Search the library")
                    .title("Library")
                    .text("Material"),
            )
            .width(Length::MatchParent)
            .height(Length::Dp(64.0))
            .margins(EdgeInsets::bottom(8.0)),
            note("SearchBar extends androidx.appcompat.widget.Toolbar."),
        ],
        WidgetKind::MaterialSearchView => vec![
            heading("Material Search View"),
            description("SearchView is the full-screen or inline Material search experience."),
            Placed::new(MaterialSearchView::new("Search messages").text("inbox"))
                .width(Length::MatchParent)
                .height(Length::Dp(64.0))
                .margins(EdgeInsets::bottom(8.0)),
            note("The hint and initial text are set from Rust."),
        ],
        WidgetKind::MaterialChip => vec![
            heading("Material Chips"),
            description(
                "Chips represent compact actions, filters, and input. Checkable chips toggle; closeable chips show a close icon.",
            ),
            Placed::new(Linear {
                orientation: Orientation::Horizontal,
                padding: EdgeInsets::ZERO,
                background: None,
                children: vec![
                    Placed::new(MaterialChip::new("Assist").icon(DRAWABLE_DIALOG_INFO))
                        .margins(EdgeInsets::right(8.0)),
                    Placed::new(MaterialChip::new("Filter").checkable(true).checked(true))
                        .margins(EdgeInsets::right(8.0)),
                    Placed::new(MaterialChip::new("Input").closeable(true)),
                ],
            })
            .margins(EdgeInsets::bottom(8.0)),
            note("The filter chip starts checked; the input chip is closeable."),
        ],
        WidgetKind::MaterialChipGroup => vec![
            heading("Material Chip Group"),
            description("ChipGroup lays out chips and can enforce single selection across them."),
            Placed::new(
                MaterialChipGroup::new(vec![
                    MaterialChip::new("Small").checkable(true),
                    MaterialChip::new("Medium").checkable(true).checked(true),
                    MaterialChip::new("Large").checkable(true),
                ])
                .single_selection(true),
            )
            .width(Length::MatchParent)
            .margins(EdgeInsets::bottom(8.0)),
            note("Single selection keeps exactly one chip checked at a time."),
        ],
        WidgetKind::MaterialCard => vec![
            heading("Material Card"),
            description(
                "MaterialCardView draws a rounded, elevated surface with an optional stroke and arbitrary children.",
            ),
            Placed::new(
                MaterialCard::new(vec![
                    Placed::new(
                        MaterialText::new("Card title")
                            .size(18.0)
                            .color(COLOR_TEXT_PRIMARY),
                    )
                    .margins(EdgeInsets::bottom(6.0)),
                    Placed::new(
                        MaterialText::new("Cards group related content on their own surface.")
                            .size(14.0)
                            .color(COLOR_TEXT_SECONDARY),
                    ),
                ])
                .radius(16.0)
                .elevation(4.0)
                .stroke(1.0, COLOR_ICON_BACKGROUND)
                .action(Action::Tap.tag()),
            )
            .width(Length::MatchParent)
            .margins(EdgeInsets::bottom(12.0)),
            note("The card itself is clickable and reports through the Rust listener."),
        ],
        WidgetKind::MaterialToolbar => vec![
            heading("Material Toolbar"),
            description(
                "MaterialToolbar is the Material themed app bar with a title, subtitle, navigation icon, and overflow menu.",
            ),
            Placed::new(MaterialToolbar {
                title: "Material Toolbar",
                subtitle: "Material 3",
                background: COLOR_WIDGETS_BACKGROUND,
                navigation_icon: DRAWABLE_MENU_REVERT,
            })
            .width(Length::MatchParent)
            .height(Length::Dp(72.0))
            .margins(EdgeInsets::bottom(8.0)),
            note("Menu items are added from Rust through MaterialToolbar.getMenu()."),
        ],
        WidgetKind::MaterialAppBar => vec![
            heading("Material App Bar"),
            description(
                "AppBarLayout hosts a MaterialToolbar and applies app-bar scroll flags to its children.",
            ),
            Placed::new(MaterialAppBar::new(vec![Placed::new(MaterialToolbar {
                title: "App Bar Layout",
                subtitle: "Scroll flags applied",
                background: COLOR_WIDGETS_BACKGROUND,
                navigation_icon: DRAWABLE_MENU_REVERT,
            })]))
            .width(Length::MatchParent)
            .height(Length::Dp(80.0))
            .margins(EdgeInsets::bottom(8.0)),
            note("The toolbar child receives AppBarLayout.LayoutParams with scroll flags."),
        ],
        WidgetKind::MaterialCollapsingToolbar => vec![
            heading("Collapsing Toolbar"),
            description(
                "CollapsingToolbarLayout coordinates a title with a toolbar and collapses as its parent app bar scrolls.",
            ),
            Placed::new(MaterialCollapsingToolbar::new(
                "Collapsing",
                vec![Placed::new(MaterialToolbar {
                    title: "Collapsing Toolbar",
                    subtitle: "Scrolling header",
                    background: COLOR_WIDGETS_BACKGROUND,
                    navigation_icon: DRAWABLE_MENU_REVERT,
                })],
            ))
            .width(Length::MatchParent)
            .height(Length::Dp(96.0))
            .margins(EdgeInsets::bottom(8.0)),
            note("Title, subtitle, background, and children are all configured from Rust."),
        ],
        WidgetKind::MaterialBottomAppBar => vec![
            heading("Bottom App Bar"),
            description(
                "BottomAppBar is a Material app bar designed to sit at the bottom of the screen next to a FAB.",
            ),
            Placed::new(MaterialBottomAppBar {
                title: "Bottom App Bar",
                subtitle: "Menu and navigation icon",
                background: COLOR_WIDGETS_BACKGROUND,
                navigation_icon: DRAWABLE_MENU_REVERT,
            })
            .width(Length::MatchParent)
            .height(Length::Dp(72.0))
            .margins(EdgeInsets::bottom(8.0)),
            note("The overflow menu and navigation icon are created through JNI."),
        ],
        WidgetKind::MaterialFloatingToolbar => vec![
            heading("Floating Toolbar"),
            description("FloatingToolbarLayout wraps its children in a floating Material surface."),
            Placed::new(MaterialToolbarLayout::new(vec![
                Placed::new(MaterialButton::new("Edit").size(13.0)).margins(EdgeInsets::right(8.0)),
                Placed::new(MaterialButton::new("Share").size(13.0))
                    .margins(EdgeInsets::right(8.0)),
                Placed::new(MaterialButton::new("Delete").size(13.0)),
            ]))
            .width(Length::MatchParent)
            .height(Length::Dp(72.0))
            .margins(EdgeInsets::bottom(8.0)),
            note("Children are laid out horizontally inside the floating container."),
        ],
        WidgetKind::MaterialDockedToolbar => vec![
            heading("Docked Toolbar"),
            description(
                "DockedToolbarLayout is a compact Material toolbar designed to dock at the edge of a screen.",
            ),
            Placed::new(Element::MaterialDockedToolbar(MaterialToolbarLayout::new(
                vec![
                    Placed::new(MaterialButton::new("Cut").size(13.0))
                        .margins(EdgeInsets::right(8.0)),
                    Placed::new(MaterialButton::new("Copy").size(13.0))
                        .margins(EdgeInsets::right(8.0)),
                    Placed::new(MaterialButton::new("Paste").size(13.0)),
                ],
            )))
            .width(Length::MatchParent)
            .height(Length::Dp(64.0))
            .margins(EdgeInsets::bottom(8.0)),
            note("DockedToolbarLayout shares the same Rust component as the floating toolbar."),
        ],
        WidgetKind::MaterialFab => vec![
            heading("Material FAB"),
            description(
                "FloatingActionButton is the primary circular action. It supports a custom size and a badge.",
            ),
            Placed::new(
                MaterialFab::new(DRAWABLE_STAR_ON)
                    .size(56.0)
                    .badge(3)
                    .action(Action::Tap.tag()),
            )
            .margins(EdgeInsets::bottom(8.0)),
            note("The badge is a BadgeDrawable attached by the helper module."),
        ],
        WidgetKind::MaterialExtendedFab => vec![
            heading("Extended FAB"),
            description(
                "ExtendedFloatingActionButton adds a text label next to the icon for a clearer primary action.",
            ),
            Placed::new(
                MaterialExtendedFab::new("Create")
                    .icon(DRAWABLE_MENU_CAMERA)
                    .badge(1)
                    .action(Action::Tap.tag()),
            )
            .margins(EdgeInsets::bottom(8.0)),
            note("The label is set with setText(); the icon uses setIconResource()."),
        ],
        WidgetKind::MaterialProgress => vec![
            heading("Material Progress"),
            description(
                "Material progress indicators come in linear and circular shapes, with determinate and indeterminate modes.",
            ),
            Placed::new(
                MaterialText::new("Linear, determinate")
                    .size(13.0)
                    .color(COLOR_TEXT_SECONDARY),
            )
            .margins(EdgeInsets::bottom(4.0)),
            Placed::new(MaterialProgress::linear(65, 100))
                .width(Length::MatchParent)
                .height(Length::Dp(12.0))
                .margins(EdgeInsets::bottom(12.0)),
            Placed::new(
                MaterialText::new("Linear, indeterminate")
                    .size(13.0)
                    .color(COLOR_TEXT_SECONDARY),
            )
            .margins(EdgeInsets::bottom(4.0)),
            Placed::new(MaterialProgress::linear(0, 100).indeterminate(true))
                .width(Length::MatchParent)
                .height(Length::Dp(12.0))
                .margins(EdgeInsets::bottom(12.0)),
            Placed::new(
                MaterialText::new("Circular, determinate")
                    .size(13.0)
                    .color(COLOR_TEXT_SECONDARY),
            )
            .margins(EdgeInsets::bottom(4.0)),
            Placed::new(MaterialProgress::circular(40, 100))
                .width(Length::Dp(48.0))
                .height(Length::Dp(48.0)),
        ],
        WidgetKind::MaterialLoading => vec![
            heading("Loading Indicator"),
            description("LoadingIndicator is the Material 3 expressive loading indicator."),
            Placed::new(MaterialLoading::new(48.0).container_color(COLOR_CHIP_BACKGROUND))
                .width(Length::Dp(72.0))
                .height(Length::Dp(72.0))
                .margins(EdgeInsets::bottom(8.0)),
            note("Indicator size and container colour are set from Rust."),
        ],
        WidgetKind::MaterialSlider => vec![
            heading("Material Slider"),
            description("Slider selects a value from a continuous or stepped range."),
            Placed::new(MaterialSlider::new(0.4, 0.0, 1.0).step(0.1))
                .width(Length::MatchParent)
                .height(Length::Dp(48.0))
                .margins(EdgeInsets::bottom(8.0)),
            note("Range 0.0-1.0 with a 0.1 step and an initial value of 0.4."),
        ],
        WidgetKind::MaterialDivider => vec![
            heading("Material Divider"),
            description("MaterialDivider separates content with a configurable line."),
            Placed::new(MaterialText::new("Section one").size(15.0))
                .margins(EdgeInsets::bottom(8.0)),
            Placed::new(MaterialDivider::new().insets(16.0, 16.0))
                .width(Length::MatchParent)
                .margins(EdgeInsets::bottom(8.0)),
            Placed::new(MaterialText::new("Section two").size(15.0))
                .margins(EdgeInsets::bottom(8.0)),
            Placed::new(MaterialDivider::new().thickness(3.0).color(COLOR_BLUE))
                .width(Length::MatchParent),
            note("The second divider is thicker and tinted blue."),
        ],
        WidgetKind::MaterialShapeableImage => vec![
            heading("Shapeable Image"),
            description(
                "ShapeableImageView applies a ShapeAppearanceModel to an image. This demo rounds all four corners.",
            ),
            Placed::new(
                MaterialShapeableImage::new(DRAWABLE_MENU_CAMERA)
                    .corner_radius(24.0)
                    .background(COLOR_ICON_BACKGROUND),
            )
            .width(Length::Dp(96.0))
            .height(Length::Dp(96.0))
            .margins(EdgeInsets::bottom(8.0)),
            note("The corner radius is converted from dp to px before building the shape model."),
        ],
        WidgetKind::MaterialTabs => vec![
            heading("Material Tabs"),
            description("TabLayout presents top-level navigation as text tabs."),
            Placed::new(MaterialTabs::new(&["Home", "Explore", "Library"]).selected(1))
                .width(Length::MatchParent)
                .height(Length::Dp(56.0))
                .margins(EdgeInsets::bottom(8.0)),
            Placed::new(
                MaterialTabs::new(&["Inbox", "Starred", "Sent", "Drafts", "Archive", "Spam"])
                    .scrollable(true),
            )
            .width(Length::MatchParent)
            .height(Length::Dp(56.0)),
            note("The first tab layout is fixed; the second is scrollable."),
        ],
        WidgetKind::MaterialBottomNavigation => vec![
            heading("Bottom Navigation"),
            description("BottomNavigationView hosts three to five destinations as menu items."),
            Placed::new(MaterialBottomNavigation::new(&[
                ("Home", DRAWABLE_DIALOG_INFO),
                ("Search", DRAWABLE_MENU_CAMERA),
                ("Profile", DRAWABLE_STAR_ON),
            ]))
            .width(Length::MatchParent)
            .height(Length::Dp(80.0))
            .margins(EdgeInsets::bottom(8.0)),
            note("Menu items are created with Menu.add() from Rust."),
        ],
        WidgetKind::MaterialNavigationRail => vec![
            heading("Navigation Rail"),
            description(
                "NavigationRailView is a compact side navigation for tablets and foldables.",
            ),
            Placed::new(MaterialNavigationRail::new(&[
                ("Home", DRAWABLE_DIALOG_INFO),
                ("Search", DRAWABLE_MENU_CAMERA),
                ("Profile", DRAWABLE_STAR_ON),
            ]))
            .height(Length::Dp(240.0))
            .margins(EdgeInsets::bottom(8.0)),
            note("The rail keeps its menu items vertical and always visible."),
        ],
        WidgetKind::MaterialNavigation => vec![
            heading("Navigation Drawer"),
            description(
                "NavigationView is the Material navigation drawer surface with menu items.",
            ),
            Placed::new(MaterialNavigation::new(&[
                "Inbox", "Starred", "Sent", "Drafts",
            ]))
            .width(Length::MatchParent)
            .height(Length::Dp(280.0))
            .margins(EdgeInsets::bottom(8.0)),
            note("Items and the checked item id are configured through NavigationView.getMenu()."),
        ],
        WidgetKind::MaterialCarousel => vec![
            heading("Material Carousel"),
            description(
                "Material carousels are built from RecyclerView plus CarouselLayoutManager. The strategy determines the visible item sizes.",
            ),
            Placed::new(MaterialCarousel::new(LIST_ITEMS))
                .width(Length::MatchParent)
                .height(Length::Dp(220.0))
                .margins(EdgeInsets::bottom(12.0)),
            Placed::new(MaterialCarousel::new(LIST_ITEMS).has_fixed_size(true))
                .width(Length::MatchParent)
                .height(Length::Dp(220.0)),
            note("The second carousel uses fixed-size items for stable layout."),
        ],
        WidgetKind::Snackbar => vec![
            heading("Snackbar"),
            description(
                "Snackbar shows a brief message at the bottom of the screen and can host an action.",
            ),
            Placed::new(MaterialButton::new("Show snackbar").action(Action::ShowSnackbar.tag()))
                .width(Length::MatchParent)
                .height(Length::Dp(48.0))
                .margins(EdgeInsets::bottom(8.0)),
            note("Snackbar.make() is called from Rust with the tapped button as its anchor."),
        ],
        WidgetKind::AlertDialog => vec![
            heading("Alert Dialog"),
            description(
                "MaterialAlertDialogBuilder shows a Material themed dialog with a title, message, and buttons.",
            ),
            Placed::new(MaterialButton::new("Show dialog").action(Action::ShowAlertDialog.tag()))
                .width(Length::MatchParent)
                .height(Length::Dp(48.0))
                .margins(EdgeInsets::bottom(8.0)),
            note("The dialog is created and shown entirely through JNI."),
        ],
        WidgetKind::MaterialDatePicker => vec![
            heading("Material Date Picker"),
            description(
                "MaterialDatePicker opens in a DialogFragment and returns the chosen date to the host activity.",
            ),
            Placed::new(MaterialButton::new("Pick a date").action(Action::ShowDatePicker.tag()))
                .width(Length::MatchParent)
                .height(Length::Dp(48.0))
                .margins(EdgeInsets::bottom(8.0)),
            note("The host activity is an AppCompatActivity so it can supply a FragmentManager."),
        ],
        WidgetKind::MaterialTimePicker => vec![
            heading("Material Time Picker"),
            description(
                "MaterialTimePicker opens a Material clock dial or keyboard input in a DialogFragment.",
            ),
            Placed::new(MaterialButton::new("Pick a time").action(Action::ShowTimePicker.tag()))
                .width(Length::MatchParent)
                .height(Length::Dp(48.0))
                .margins(EdgeInsets::bottom(8.0)),
            note("The picker is shown through the activity's support FragmentManager."),
        ],
    }
}
