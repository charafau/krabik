//! Unit tests for the parts of the tree that do not touch JNI.

use super::*;
use crate::constants::{
    COLOR_TEXT_PRIMARY, DRAWABLE_DIALOG_INFO, GRAVITY_CENTER_HORIZONTAL, MATCH_PARENT, WRAP_CONTENT,
};

fn edges(insets: EdgeInsets) -> (f32, f32, f32, f32) {
    (insets.left, insets.top, insets.right, insets.bottom)
}

#[test]
fn dp_resolution_rounds_to_nearest_pixel() {
    let metrics = Metrics { density: 2.5 };
    assert_eq!(metrics.dp(4.0), 10);
    assert_eq!(metrics.dp(3.0), 8); // 7.5 rounds away from zero
}

#[test]
fn dp_resolution_rounds_negative_values_away_from_zero() {
    let metrics = Metrics { density: 2.5 };
    assert_eq!(metrics.dp(-4.0), -10);
    assert_eq!(metrics.dp(-3.0), -8);
}

#[test]
fn dp_resolution_handles_zero_density_and_zero_value() {
    assert_eq!(Metrics { density: 0.0 }.dp(42.0), 0);
    assert_eq!(Metrics { density: 3.0 }.dp(0.0), 0);
}

#[test]
fn dp_resolution_rounds_fractional_densities() {
    let metrics = Metrics { density: 2.625 };
    assert_eq!(metrics.dp(2.0), 5); // 5.25
    assert_eq!(metrics.dp(4.0), 11); // 10.5 rounds away from zero
}

#[test]
fn edge_insets_resolve_independently() {
    let metrics = Metrics { density: 2.0 };
    let insets = EdgeInsets::new(1.0, 2.0, 3.0, 4.0);
    assert_eq!(insets.resolve(metrics), (2, 4, 6, 8));
    assert_eq!(EdgeInsets::right(1.5).resolve(metrics), (0, 0, 3, 0));
}

#[test]
fn edge_insets_zero_has_no_dimensions() {
    assert_eq!(edges(EdgeInsets::ZERO), (0.0, 0.0, 0.0, 0.0));
    assert_eq!(
        EdgeInsets::ZERO.resolve(Metrics { density: 4.0 }),
        (0, 0, 0, 0)
    );
}

#[test]
fn edge_insets_constructors_set_one_side_at_a_time() {
    assert_eq!(edges(EdgeInsets::all(2.0)), (2.0, 2.0, 2.0, 2.0));
    assert_eq!(edges(EdgeInsets::top(1.0)), (0.0, 1.0, 0.0, 0.0));
    assert_eq!(edges(EdgeInsets::bottom(2.0)), (0.0, 0.0, 0.0, 2.0));
    assert_eq!(edges(EdgeInsets::right(3.0)), (0.0, 0.0, 3.0, 0.0));
}

#[test]
fn length_resolution_matches_android_constants() {
    let metrics = Metrics { density: 3.0 };
    assert_eq!(Length::MatchParent.resolve(metrics), MATCH_PARENT);
    assert_eq!(Length::WrapContent.resolve(metrics), WRAP_CONTENT);
    assert_eq!(Length::Dp(2.0).resolve(metrics), 6);
}

#[test]
fn zero_dp_length_resolves_to_zero_pixels() {
    assert_eq!(Length::Dp(0.0).resolve(Metrics { density: 5.0 }), 0);
}

#[test]
fn placed_defaults_leave_dimensions_unset() {
    let placed = Placed::new(Text::new("hello"));
    assert!(placed.width.is_none());
    assert!(placed.height.is_none());
    assert_eq!(edges(placed.margins), (0.0, 0.0, 0.0, 0.0));
    assert!(matches!(placed.element, Element::Text(_)));
}

#[test]
fn placed_builders_chain_and_last_call_wins() {
    let placed = Placed::new(Button::new("Go"))
        .width(Length::Dp(10.0))
        .height(Length::WrapContent)
        .margins(EdgeInsets::all(1.0))
        .width(Length::MatchParent);

    assert!(matches!(placed.width, Some(Length::MatchParent)));
    assert!(matches!(placed.height, Some(Length::WrapContent)));
    assert_eq!(edges(placed.margins), (1.0, 1.0, 1.0, 1.0));
}

#[test]
fn text_defaults_and_builder_overrides() {
    let text = Text::new("hello");
    assert_eq!(text.value, "hello");
    assert_eq!(text.size_sp, 14.0);
    assert_eq!(text.color, COLOR_TEXT_PRIMARY);
    assert!(text.gravity.is_none());
    assert_eq!(edges(text.padding), (0.0, 0.0, 0.0, 0.0));
    assert!(text.background.is_none());

    let text = text
        .size(18.0)
        .color(0x1122_3344)
        .gravity(GRAVITY_CENTER_HORIZONTAL)
        .padding(EdgeInsets::all(4.0))
        .background(0x5566_7788);

    assert_eq!(text.size_sp, 18.0);
    assert_eq!(text.color, 0x1122_3344);
    assert_eq!(text.gravity, Some(GRAVITY_CENTER_HORIZONTAL));
    assert_eq!(edges(text.padding), (4.0, 4.0, 4.0, 4.0));
    assert_eq!(text.background, Some(0x5566_7788));
}

#[test]
fn button_defaults_and_builder_overrides() {
    let button = Button::new("Go");
    assert_eq!(button.value, "Go");
    assert_eq!(button.size_sp, 14.0);
    assert!(!button.all_caps);
    assert!(button.enabled);
    assert!(button.action.is_none());

    let button = button.size(20.0).all_caps(true).enabled(false).action("go");

    assert_eq!(button.size_sp, 20.0);
    assert!(button.all_caps);
    assert!(!button.enabled);
    assert_eq!(button.action, Some("go"));
}

#[test]
fn image_and_image_button_default_to_no_background() {
    let image = Image::new(DRAWABLE_DIALOG_INFO);
    assert_eq!(image.drawable.to_str().as_ref(), "ic_dialog_info");
    assert!(image.background.is_none());

    let image = image.background(0x7FFF_FFFF);
    assert_eq!(image.background, Some(0x7FFF_FFFF));

    let button = ImageButton::new(DRAWABLE_DIALOG_INFO);
    assert_eq!(button.drawable.to_str().as_ref(), "ic_dialog_info");
    assert!(button.background.is_none());

    let button = button.background(0x7FFF_FFFF);
    assert_eq!(button.background, Some(0x7FFF_FFFF));
}

#[test]
fn edit_text_defaults_and_builder_overrides() {
    let edit = EditText::new("Email");
    assert_eq!(edit.hint, "Email");
    assert_eq!(edit.text, "");
    assert_eq!(edit.size_sp, 14.0);
    assert!(edit.input_type.is_none());
    assert!(edit.background.is_none());
    assert_eq!(edges(edit.padding), (0.0, 0.0, 0.0, 0.0));

    let edit = edit
        .text("user@example.com")
        .size(18.0)
        .input_type(0x21)
        .background(0x7FFF_FFFF)
        .padding(EdgeInsets::all(6.0));

    assert_eq!(edit.text, "user@example.com");
    assert_eq!(edit.size_sp, 18.0);
    assert_eq!(edit.input_type, Some(0x21));
    assert_eq!(edit.background, Some(0x7FFF_FFFF));
    assert_eq!(edges(edit.padding), (6.0, 6.0, 6.0, 6.0));
}

#[test]
fn labelled_toggles_default_to_unchecked_and_can_be_checked() {
    let checkbox = CheckBox::new("Remember me");
    assert_eq!(checkbox.label, "Remember me");
    assert!(!checkbox.checked);
    assert!(CheckBox::new("x").checked(true).checked);

    let radio = RadioButton::new("Option");
    assert_eq!(radio.label, "Option");
    assert!(!radio.checked);
    assert!(RadioButton::new("x").checked(true).checked);

    let switch = Switch::new("Enabled");
    assert_eq!(switch.label, "Enabled");
    assert!(!switch.checked);
    assert!(Switch::new("x").checked(true).checked);

    let checked = CheckedText::new("Picked");
    assert_eq!(checked.label, "Picked");
    assert!(!checked.checked);
    assert!(CheckedText::new("x").checked(true).checked);
}

#[test]
fn toggle_button_keeps_both_labels() {
    let toggle = ToggleButton::new("On", "Off");
    assert_eq!(toggle.text_on, "On");
    assert_eq!(toggle.text_off, "Off");
    assert!(!toggle.checked);
    assert!(ToggleButton::new("On", "Off").checked(true).checked);
}

#[test]
fn progress_constructors_select_kind_and_values() {
    let progress = Progress::horizontal(25, 100);
    assert!(matches!(progress.kind, ProgressKind::Horizontal));
    assert_eq!(progress.progress, 25);
    assert_eq!(progress.max, 100);

    let spinner = Progress::spinner();
    assert!(matches!(spinner.kind, ProgressKind::Spinner));
    assert_eq!(spinner.progress, 0);
    assert_eq!(spinner.max, 100);
}

#[test]
fn seek_keeps_position_and_range() {
    let seek = Seek::new(10, 20);
    assert_eq!(seek.progress, 10);
    assert_eq!(seek.max, 20);
}

#[test]
fn rating_defaults_to_half_steps() {
    let rating = Rating::new(3.0, 5);
    assert_eq!(rating.rating, 3.0);
    assert_eq!(rating.num_stars, 5);
    assert_eq!(rating.step_size, 0.5);
    assert!(!rating.indicator);

    assert_eq!(rating.step_size(1.0).step_size, 1.0);
}

#[test]
fn spinner_defaults_to_the_first_item() {
    let spinner = Spinner::new(&["One", "Two"]);
    assert_eq!(spinner.items, &["One", "Two"]);
    assert_eq!(spinner.selected, 0);
    assert_eq!(spinner.selected(2).selected, 2);
}

#[test]
fn auto_complete_defaults_to_threshold_one() {
    let field = AutoComplete::new("Fruit", &["Apple", "Pear"]);
    assert_eq!(field.hint, "Fruit");
    assert_eq!(field.items, &["Apple", "Pear"]);
    assert_eq!(field.threshold, 1);
    assert_eq!(field.threshold(3).threshold, 3);
}

#[test]
fn multi_auto_complete_defaults_to_threshold_one() {
    let field = MultiAutoComplete::new("Tags", &["rust", "android"]);
    assert_eq!(field.hint, "Tags");
    assert_eq!(field.items, &["rust", "android"]);
    assert_eq!(field.threshold, 1);
    assert_eq!(field.threshold(2).threshold, 2);
}

#[test]
fn number_picker_keeps_range_and_value() {
    let picker = NumberPicker::new(1, 10, 4);
    assert_eq!(picker.min, 1);
    assert_eq!(picker.max, 10);
    assert_eq!(picker.value, 4);
}

#[test]
fn text_clock_and_search_keep_configuration() {
    assert_eq!(TextClock::new(16.0).size_sp, 16.0);

    let search = Search::new("Search");
    assert_eq!(search.hint, "Search");
}

#[test]
fn toolbar_keeps_text_colors_and_icon() {
    let toolbar = Toolbar {
        title: "Title",
        subtitle: "Subtitle",
        background: 0x1122_3344,
        navigation_icon: DRAWABLE_DIALOG_INFO,
    };

    assert_eq!(toolbar.title, "Title");
    assert_eq!(toolbar.subtitle, "Subtitle");
    assert_eq!(toolbar.background, 0x1122_3344);
    assert_eq!(toolbar.navigation_icon.to_str().as_ref(), "ic_dialog_info");
}

#[test]
fn app_compat_toolbar_keeps_text_colors_and_icon() {
    let toolbar = AppCompatToolbar {
        title: "Title",
        subtitle: "Subtitle",
        background: 0x1122_3344,
        navigation_icon: DRAWABLE_DIALOG_INFO,
    };

    assert_eq!(toolbar.title, "Title");
    assert_eq!(toolbar.subtitle, "Subtitle");
    assert_eq!(toolbar.background, 0x1122_3344);
    assert_eq!(toolbar.navigation_icon.to_str().as_ref(), "ic_dialog_info");
}

#[test]
fn scroll_and_linear_hold_placed_children() {
    let scroll = Scroll {
        background: None,
        child: Box::new(Placed::new(Text::new("child"))),
    };
    assert!(scroll.background.is_none());
    assert!(matches!(scroll.child.element, Element::Text(_)));

    let linear = Linear {
        orientation: Orientation::Vertical,
        padding: EdgeInsets::all(8.0),
        background: Some(0x1122_3344),
        children: vec![
            Placed::new(Button::new("one")),
            Placed::new(CheckBox::new("two")),
        ],
    };
    assert!(matches!(linear.orientation, Orientation::Vertical));
    assert_eq!(edges(linear.padding), (8.0, 8.0, 8.0, 8.0));
    assert_eq!(linear.background, Some(0x1122_3344));
    assert_eq!(linear.children.len(), 2);
}

#[test]
fn element_converts_from_every_component() {
    let element: Element = Scroll {
        background: None,
        child: Box::new(Placed::new(Text::new("child"))),
    }
    .into();
    assert!(matches!(element, Element::Scroll(_)));

    let element: Element = Linear {
        orientation: Orientation::Horizontal,
        padding: EdgeInsets::ZERO,
        background: None,
        children: Vec::new(),
    }
    .into();
    assert!(matches!(element, Element::Linear(_)));

    let element: Element = RadioGroup {
        options: vec![RadioButton::new("Option")],
    }
    .into();
    assert!(matches!(element, Element::RadioGroup(_)));

    let element: Element = Text::new("text").into();
    assert!(matches!(element, Element::Text(_)));

    let element: Element = Button::new("button").into();
    assert!(matches!(element, Element::Button(_)));

    let element: Element = Image::new(DRAWABLE_DIALOG_INFO).into();
    assert!(matches!(element, Element::Image(_)));

    let element: Element = ImageButton::new(DRAWABLE_DIALOG_INFO).into();
    assert!(matches!(element, Element::ImageButton(_)));

    let element: Element = EditText::new("hint").into();
    assert!(matches!(element, Element::EditText(_)));

    let element: Element = CheckBox::new("check").into();
    assert!(matches!(element, Element::CheckBox(_)));

    let element: Element = Switch::new("switch").into();
    assert!(matches!(element, Element::Switch(_)));

    let element: Element = ToggleButton::new("On", "Off").into();
    assert!(matches!(element, Element::ToggleButton(_)));

    let element: Element = CheckedText::new("checked").into();
    assert!(matches!(element, Element::CheckedText(_)));

    let element: Element = Progress::spinner().into();
    assert!(matches!(element, Element::Progress(_)));

    let element: Element = Seek::new(0, 10).into();
    assert!(matches!(element, Element::Seek(_)));

    let element: Element = Rating::new(3.0, 5).into();
    assert!(matches!(element, Element::Rating(_)));

    let element: Element = Spinner::new(&["One"]).into();
    assert!(matches!(element, Element::Spinner(_)));

    let element: Element = AutoComplete::new("hint", &["One"]).into();
    assert!(matches!(element, Element::AutoComplete(_)));

    let element: Element = MultiAutoComplete::new("hint", &["One"]).into();
    assert!(matches!(element, Element::MultiAutoComplete(_)));

    let element: Element = NumberPicker::new(1, 2, 1).into();
    assert!(matches!(element, Element::NumberPicker(_)));

    let element: Element = TextClock::new(16.0).into();
    assert!(matches!(element, Element::TextClock(_)));

    let element: Element = Search::new("query").into();
    assert!(matches!(element, Element::Search(_)));

    let element: Element = Toolbar {
        title: "Title",
        subtitle: "Subtitle",
        background: 0x1122_3344,
        navigation_icon: DRAWABLE_DIALOG_INFO,
    }
    .into();
    assert!(matches!(element, Element::Toolbar(_)));

    let element: Element = AppCompatToolbar {
        title: "Title",
        subtitle: "Subtitle",
        background: 0x1122_3344,
        navigation_icon: DRAWABLE_DIALOG_INFO,
    }
    .into();
    assert!(matches!(element, Element::AppCompatToolbar(_)));
}

#[test]
fn recycler_defaults_to_a_vertical_list() {
    let recycler = Recycler::new(&["One", "Two"]);
    assert_eq!(recycler.items, &["One", "Two"]);
    assert!(matches!(recycler.orientation, Orientation::Vertical));
    assert!(recycler.span_count.is_none());
    assert!(!recycler.has_fixed_size);

    let recycler = recycler
        .orientation(Orientation::Horizontal)
        .grid(3)
        .has_fixed_size(true);

    assert!(matches!(recycler.orientation, Orientation::Horizontal));
    assert_eq!(recycler.span_count, Some(3));
    assert!(recycler.has_fixed_size);
}

#[test]
fn view_pager_defaults_to_the_first_horizontal_page() {
    let pager = ViewPager::new(&["One", "Two"]);
    assert_eq!(pager.items, &["One", "Two"]);
    assert!(matches!(pager.orientation, Orientation::Horizontal));
    assert!(pager.offscreen_page_limit.is_none());
    assert_eq!(pager.current_item, 0);
    assert!(pager.user_input_enabled);

    let pager = pager
        .orientation(Orientation::Vertical)
        .offscreen_page_limit(2)
        .current_item(1)
        .user_input_enabled(false);

    assert!(matches!(pager.orientation, Orientation::Vertical));
    assert_eq!(pager.offscreen_page_limit, Some(2));
    assert_eq!(pager.current_item, 1);
    assert!(!pager.user_input_enabled);
}

#[test]
fn element_converts_from_recycler_and_view_pager() {
    let element: Element = Recycler::new(&["One"]).into();
    assert!(matches!(element, Element::Recycler(_)));

    let element: Element = ViewPager::new(&["One"]).into();
    assert!(matches!(element, Element::ViewPager(_)));
}

#[test]
fn element_supports_units_and_status_without_component_conversions() {
    assert!(matches!(Element::DatePicker, Element::DatePicker));
    assert!(matches!(Element::TimePicker, Element::TimePicker));
    assert!(matches!(Element::Chronometer, Element::Chronometer));
    assert!(matches!(Element::Calendar, Element::Calendar));
    assert!(matches!(Element::AnalogClock, Element::AnalogClock));

    let status = Element::Status(Text::new("Ready"));
    assert!(matches!(status, Element::Status(text) if text.value == "Ready"));
}

#[test]
fn material_text_defaults_to_primary_text_at_body_size() {
    let text = MaterialText::new("Hello");
    assert_eq!(text.value, "Hello");
    assert_eq!(text.size_sp, 14.0);
    assert_eq!(text.color, COLOR_TEXT_PRIMARY);
    assert!(text.gravity.is_none());
    assert_eq!(edges(text.padding), (0.0, 0.0, 0.0, 0.0));
    assert!(text.background.is_none());

    let text = text
        .size(20.0)
        .color(0x1122_3344)
        .gravity(GRAVITY_CENTER_HORIZONTAL)
        .padding(EdgeInsets::all(4.0))
        .background(0x5566_7788);

    assert_eq!(text.size_sp, 20.0);
    assert_eq!(text.color, 0x1122_3344);
    assert_eq!(text.gravity, Some(GRAVITY_CENTER_HORIZONTAL));
    assert_eq!(edges(text.padding), (4.0, 4.0, 4.0, 4.0));
    assert_eq!(text.background, Some(0x5566_7788));
}

#[test]
fn material_button_defaults_to_an_enabled_unchecked_button() {
    let button = MaterialButton::new("Save");
    assert_eq!(button.label, "Save");
    assert!(!button.all_caps);
    assert!(button.enabled);
    assert!(!button.checkable);
    assert!(!button.checked);
    assert!(button.icon.is_none());
    assert!(button.action.is_none());
}

#[test]
fn material_toggle_group_requires_a_selection_by_default() {
    let group = MaterialButtonToggleGroup::new(vec![
        MaterialButton::new("Left"),
        MaterialButton::new("Right").checked(true),
    ]);
    assert!(group.single_selection);
    assert!(!group.selection_required);
    assert_eq!(group.buttons.len(), 2);
}

#[test]
fn material_text_field_defaults_to_an_outlined_box() {
    let field = MaterialTextField::new("Email");
    assert_eq!(field.hint, "Email");
    assert_eq!(field.text, "");
    assert_eq!(field.box_mode, MaterialBoxMode::Outlined);
    assert!(!field.password_toggle);
    assert_eq!(field.size_sp, 15.0);
}

#[test]
fn material_chip_group_is_multi_select_and_multi_line_by_default() {
    let group = MaterialChipGroup::new(vec![MaterialChip::new("One")]);
    assert!(!group.single_selection);
    assert!(!group.single_line);
}

#[test]
fn material_card_defaults_to_a_rounded_elevated_container() {
    let card = MaterialCard::new(vec![]);
    assert_eq!(card.radius_dp, 12.0);
    assert_eq!(card.elevation_dp, 2.0);
    assert_eq!(card.stroke_width_dp, 0.0);
    assert_eq!(edges(card.padding), (16.0, 16.0, 16.0, 16.0));

    let card = card.stroke(1.0, 0x1122_3344);
    assert_eq!(card.stroke_width_dp, 1.0);
    assert_eq!(card.stroke_color, Some(0x1122_3344));
}

#[test]
fn material_progress_constructors_set_kind_and_range() {
    let linear = MaterialProgress::linear(3, 10);
    assert_eq!(linear.kind, MaterialProgressKind::Linear);
    assert_eq!(linear.progress, 3);
    assert_eq!(linear.max, 10);
    assert!(!linear.indeterminate);

    let circular = MaterialProgress::circular(1, 5).indeterminate(true);
    assert_eq!(circular.kind, MaterialProgressKind::Circular);
    assert!(circular.indeterminate);
}

#[test]
fn material_slider_defaults_to_a_single_step() {
    let slider = MaterialSlider::new(0.5, 0.0, 1.0);
    assert_eq!(slider.value, 0.5);
    assert_eq!(slider.from, 0.0);
    assert_eq!(slider.to, 1.0);
    assert_eq!(slider.step, 0.0);
    assert!(slider.enabled);
}

#[test]
fn material_divider_defaults_to_one_dp_without_insets() {
    let divider = MaterialDivider::new();
    assert_eq!(divider.thickness_dp, 1.0);
    assert!(divider.color.is_none());
    assert_eq!(divider.inset_start_dp, 0.0);
    assert_eq!(divider.inset_end_dp, 0.0);
}

#[test]
fn material_tabs_default_to_fixed_mode_and_first_tab() {
    let tabs = MaterialTabs::new(&["One", "Two"]);
    assert_eq!(tabs.selected, 0);
    assert!(!tabs.scrollable);

    let tabs = tabs.selected(1).scrollable(true);
    assert_eq!(tabs.selected, 1);
    assert!(tabs.scrollable);
}

#[test]
fn material_carousel_defaults_to_a_horizontal_dynamic_list() {
    let carousel = MaterialCarousel::new(&["One", "Two"]);
    assert!(matches!(carousel.orientation, Orientation::Horizontal));
    assert!(!carousel.has_fixed_size);
}

#[test]
fn material_box_mode_maps_to_text_input_layout_constants() {
    assert_eq!(
        MaterialBoxMode::None.value(),
        crate::constants::MATERIAL_TEXT_INPUT_BOX_NONE
    );
    assert_eq!(
        MaterialBoxMode::Filled.value(),
        crate::constants::MATERIAL_TEXT_INPUT_BOX_FILLED
    );
    assert_eq!(
        MaterialBoxMode::Outlined.value(),
        crate::constants::MATERIAL_TEXT_INPUT_BOX_OUTLINE
    );
}

#[test]
fn element_converts_from_material_components() {
    let element: Element = MaterialText::new("Hello").into();
    assert!(matches!(element, Element::MaterialText(_)));

    let element: Element = MaterialButton::new("Save").into();
    assert!(matches!(element, Element::MaterialButton(_)));

    let element: Element = MaterialButtonToggleGroup::new(vec![
        MaterialButton::new("Left"),
        MaterialButton::new("Right"),
    ])
    .into();
    assert!(matches!(element, Element::MaterialToggleGroup(_)));

    let element: Element = MaterialCheckBox::new("Check").into();
    assert!(matches!(element, Element::MaterialCheckBox(_)));

    let element: Element = MaterialRadioGroup::new(vec![MaterialRadioButton::new("One")]).into();
    assert!(matches!(element, Element::MaterialRadioGroup(_)));

    let element: Element = MaterialSwitch::new("Switch").into();
    assert!(matches!(element, Element::MaterialSwitch(_)));

    let element: Element = MaterialTextField::new("Email").into();
    assert!(matches!(element, Element::MaterialTextField(_)));

    let element: Element = MaterialDropdown::new("Pick", &["One"]).into();
    assert!(matches!(element, Element::MaterialDropdown(_)));

    let element: Element = MaterialSearchBar::new("Search").into();
    assert!(matches!(element, Element::MaterialSearchBar(_)));

    let element: Element = MaterialSearchView::new("Search").into();
    assert!(matches!(element, Element::MaterialSearchView(_)));

    let element: Element = MaterialChip::new("Chip").into();
    assert!(matches!(element, Element::MaterialChip(_)));

    let element: Element = MaterialChipGroup::new(vec![MaterialChip::new("Chip")]).into();
    assert!(matches!(element, Element::MaterialChipGroup(_)));

    let element: Element = MaterialCard::new(vec![]).into();
    assert!(matches!(element, Element::MaterialCard(_)));

    let element: Element = MaterialToolbar::new("Title").into();
    assert!(matches!(element, Element::MaterialToolbar(_)));

    let element: Element = MaterialAppBar::new(vec![]).into();
    assert!(matches!(element, Element::MaterialAppBar(_)));

    let element: Element = MaterialCollapsingToolbar::new("Title", vec![]).into();
    assert!(matches!(element, Element::MaterialCollapsingToolbar(_)));

    let element: Element = MaterialBottomAppBar::new("Title").into();
    assert!(matches!(element, Element::MaterialBottomAppBar(_)));

    let element: Element = MaterialToolbarLayout::new(vec![]).into();
    assert!(matches!(element, Element::MaterialFloatingToolbar(_)));

    let element: Element = MaterialFab::new(DRAWABLE_DIALOG_INFO).into();
    assert!(matches!(element, Element::MaterialFab(_)));

    let element: Element = MaterialExtendedFab::new("Create").into();
    assert!(matches!(element, Element::MaterialExtendedFab(_)));

    let element: Element = MaterialProgress::linear(0, 100).into();
    assert!(matches!(element, Element::MaterialProgress(_)));

    let element: Element = MaterialLoading::new(48.0).into();
    assert!(matches!(element, Element::MaterialLoading(_)));

    let element: Element = MaterialSlider::new(0.0, 0.0, 1.0).into();
    assert!(matches!(element, Element::MaterialSlider(_)));

    let element: Element = MaterialDivider::new().into();
    assert!(matches!(element, Element::MaterialDivider(_)));

    let element: Element = MaterialShapeableImage::new(DRAWABLE_DIALOG_INFO).into();
    assert!(matches!(element, Element::MaterialShapeableImage(_)));

    let element: Element = MaterialTabs::new(&["One"]).into();
    assert!(matches!(element, Element::MaterialTabs(_)));

    let element: Element = MaterialBottomNavigation::new(&[("Home", DRAWABLE_DIALOG_INFO)]).into();
    assert!(matches!(element, Element::MaterialBottomNavigation(_)));

    let element: Element = MaterialNavigationRail::new(&[("Home", DRAWABLE_DIALOG_INFO)]).into();
    assert!(matches!(element, Element::MaterialNavigationRail(_)));

    let element: Element = MaterialNavigation::new(&["Home"]).into();
    assert!(matches!(element, Element::MaterialNavigation(_)));

    let element: Element = MaterialCarousel::new(&["One"]).into();
    assert!(matches!(element, Element::MaterialCarousel(_)));
}
