//! JNI names and Android constants used by the bridge.

use jni::jni_str;
use jni::strings::JNIStr;
use jni::sys::jint;

/// Logcat tag for messages written by [`super::env::log_error`].
pub const LOG_TAG: &str = "RustUi";

// Framework classes passed to `JNIEnv::FindClass`.
pub const ACTIVITY_CLASS: &JNIStr = jni_str!("android/app/Activity");
pub const ON_CLICK_LISTENER_CLASS: &JNIStr = jni_str!("android/view/View$OnClickListener");

pub const LINEAR_LAYOUT_CLASS: &JNIStr = jni_str!("android/widget/LinearLayout");
pub const LINEAR_LAYOUT_PARAMS_CLASS: &JNIStr =
    jni_str!("android/widget/LinearLayout$LayoutParams");
pub const FRAME_LAYOUT_PARAMS_CLASS: &JNIStr = jni_str!("android/widget/FrameLayout$LayoutParams");
pub const SCROLL_VIEW_CLASS: &JNIStr = jni_str!("android/widget/ScrollView");

pub const TEXT_VIEW_CLASS: &JNIStr = jni_str!("android/widget/TextView");
pub const BUTTON_CLASS: &JNIStr = jni_str!("android/widget/Button");
pub const IMAGE_VIEW_CLASS: &JNIStr = jni_str!("android/widget/ImageView");
pub const IMAGE_BUTTON_CLASS: &JNIStr = jni_str!("android/widget/ImageButton");
pub const EDIT_TEXT_CLASS: &JNIStr = jni_str!("android/widget/EditText");
pub const CHECK_BOX_CLASS: &JNIStr = jni_str!("android/widget/CheckBox");
pub const RADIO_BUTTON_CLASS: &JNIStr = jni_str!("android/widget/RadioButton");
pub const RADIO_GROUP_CLASS: &JNIStr = jni_str!("android/widget/RadioGroup");
pub const SWITCH_CLASS: &JNIStr = jni_str!("android/widget/Switch");
pub const TOGGLE_BUTTON_CLASS: &JNIStr = jni_str!("android/widget/ToggleButton");
pub const CHECKED_TEXT_VIEW_CLASS: &JNIStr = jni_str!("android/widget/CheckedTextView");
pub const PROGRESS_BAR_CLASS: &JNIStr = jni_str!("android/widget/ProgressBar");
pub const SEEK_BAR_CLASS: &JNIStr = jni_str!("android/widget/SeekBar");
pub const RATING_BAR_CLASS: &JNIStr = jni_str!("android/widget/RatingBar");
pub const SPINNER_CLASS: &JNIStr = jni_str!("android/widget/Spinner");
pub const ARRAY_ADAPTER_CLASS: &JNIStr = jni_str!("android/widget/ArrayAdapter");
pub const AUTO_COMPLETE_TEXT_VIEW_CLASS: &JNIStr = jni_str!("android/widget/AutoCompleteTextView");
pub const MULTI_AUTO_COMPLETE_TEXT_VIEW_CLASS: &JNIStr =
    jni_str!("android/widget/MultiAutoCompleteTextView");
pub const COMMA_TOKENIZER_CLASS: &JNIStr =
    jni_str!("android/widget/MultiAutoCompleteTextView$CommaTokenizer");
pub const NUMBER_PICKER_CLASS: &JNIStr = jni_str!("android/widget/NumberPicker");
pub const DATE_PICKER_CLASS: &JNIStr = jni_str!("android/widget/DatePicker");
pub const TIME_PICKER_CLASS: &JNIStr = jni_str!("android/widget/TimePicker");
pub const CHRONOMETER_CLASS: &JNIStr = jni_str!("android/widget/Chronometer");
pub const TEXT_CLOCK_CLASS: &JNIStr = jni_str!("android/widget/TextClock");
pub const CALENDAR_VIEW_CLASS: &JNIStr = jni_str!("android/widget/CalendarView");
pub const ANALOG_CLOCK_CLASS: &JNIStr = jni_str!("android/widget/AnalogClock");
pub const SEARCH_VIEW_CLASS: &JNIStr = jni_str!("android/widget/SearchView");
pub const TOOLBAR_CLASS: &JNIStr = jni_str!("android/widget/Toolbar");
pub const SYSTEM_CLOCK_CLASS: &JNIStr = jni_str!("android/os/SystemClock");

// AndroidX classes. The consuming app must depend on `androidx.recyclerview:recyclerview`,
// `androidx.viewpager2:viewpager2`, and `androidx.appcompat:appcompat`.
pub const RECYCLER_VIEW_CLASS: &JNIStr = jni_str!("androidx/recyclerview/widget/RecyclerView");
pub const RECYCLER_VIEW_ADAPTER_CLASS: &JNIStr =
    jni_str!("androidx/recyclerview/widget/RecyclerView$Adapter");
pub const RECYCLER_VIEW_LAYOUT_MANAGER_CLASS: &JNIStr =
    jni_str!("androidx/recyclerview/widget/RecyclerView$LayoutManager");
pub const LINEAR_LAYOUT_MANAGER_CLASS: &JNIStr =
    jni_str!("androidx/recyclerview/widget/LinearLayoutManager");
pub const GRID_LAYOUT_MANAGER_CLASS: &JNIStr =
    jni_str!("androidx/recyclerview/widget/GridLayoutManager");
pub const VIEW_PAGER_2_CLASS: &JNIStr = jni_str!("androidx/viewpager2/widget/ViewPager2");
pub const APP_COMPAT_TOOLBAR_CLASS: &JNIStr = jni_str!("androidx/appcompat/widget/Toolbar");

// Material Components classes. The consuming app must depend on
// `com.google.android.material:material` and use a Material Components theme.
pub const MATERIAL_BUTTON_CLASS: &JNIStr =
    jni_str!("com/google/android/material/button/MaterialButton");
pub const MATERIAL_BUTTON_TOGGLE_GROUP_CLASS: &JNIStr =
    jni_str!("com/google/android/material/button/MaterialButtonToggleGroup");
pub const MATERIAL_CHECK_BOX_CLASS: &JNIStr =
    jni_str!("com/google/android/material/checkbox/MaterialCheckBox");
pub const MATERIAL_RADIO_BUTTON_CLASS: &JNIStr =
    jni_str!("com/google/android/material/radiobutton/MaterialRadioButton");
pub const MATERIAL_SWITCH_CLASS: &JNIStr =
    jni_str!("com/google/android/material/materialswitch/MaterialSwitch");
pub const MATERIAL_TEXT_VIEW_CLASS: &JNIStr =
    jni_str!("com/google/android/material/textview/MaterialTextView");
pub const MATERIAL_TEXT_INPUT_LAYOUT_CLASS: &JNIStr =
    jni_str!("com/google/android/material/textfield/TextInputLayout");
pub const TEXT_INPUT_EDIT_TEXT_CLASS: &JNIStr =
    jni_str!("com/google/android/material/textfield/TextInputEditText");
pub const MATERIAL_AUTO_COMPLETE_TEXT_VIEW_CLASS: &JNIStr =
    jni_str!("com/google/android/material/textfield/MaterialAutoCompleteTextView");
pub const MATERIAL_SEARCH_BAR_CLASS: &JNIStr =
    jni_str!("com/google/android/material/search/SearchBar");
pub const MATERIAL_SEARCH_VIEW_CLASS: &JNIStr =
    jni_str!("com/google/android/material/search/SearchView");
pub const MATERIAL_CHIP_CLASS: &JNIStr = jni_str!("com/google/android/material/chip/Chip");
pub const MATERIAL_CHIP_GROUP_CLASS: &JNIStr =
    jni_str!("com/google/android/material/chip/ChipGroup");
pub const MATERIAL_CARD_VIEW_CLASS: &JNIStr =
    jni_str!("com/google/android/material/card/MaterialCardView");
pub const MATERIAL_TOOLBAR_CLASS: &JNIStr =
    jni_str!("com/google/android/material/appbar/MaterialToolbar");
pub const APP_BAR_LAYOUT_CLASS: &JNIStr =
    jni_str!("com/google/android/material/appbar/AppBarLayout");
pub const APP_BAR_LAYOUT_PARAMS_CLASS: &JNIStr =
    jni_str!("com/google/android/material/appbar/AppBarLayout$LayoutParams");
pub const COLLAPSING_TOOLBAR_LAYOUT_CLASS: &JNIStr =
    jni_str!("com/google/android/material/appbar/CollapsingToolbarLayout");
pub const MATERIAL_BOTTOM_APP_BAR_CLASS: &JNIStr =
    jni_str!("com/google/android/material/bottomappbar/BottomAppBar");
pub const MATERIAL_FLOATING_TOOLBAR_LAYOUT_CLASS: &JNIStr =
    jni_str!("com/google/android/material/floatingtoolbar/FloatingToolbarLayout");
pub const MATERIAL_DOCKED_TOOLBAR_LAYOUT_CLASS: &JNIStr =
    jni_str!("com/google/android/material/dockedtoolbar/DockedToolbarLayout");
pub const MATERIAL_FLOATING_ACTION_BUTTON_CLASS: &JNIStr =
    jni_str!("com/google/android/material/floatingactionbutton/FloatingActionButton");
pub const MATERIAL_EXTENDED_FLOATING_ACTION_BUTTON_CLASS: &JNIStr =
    jni_str!("com/google/android/material/floatingactionbutton/ExtendedFloatingActionButton");
pub const MATERIAL_LINEAR_PROGRESS_INDICATOR_CLASS: &JNIStr =
    jni_str!("com/google/android/material/progressindicator/LinearProgressIndicator");
pub const MATERIAL_CIRCULAR_PROGRESS_INDICATOR_CLASS: &JNIStr =
    jni_str!("com/google/android/material/progressindicator/CircularProgressIndicator");
pub const MATERIAL_LOADING_INDICATOR_CLASS: &JNIStr =
    jni_str!("com/google/android/material/loadingindicator/LoadingIndicator");
pub const MATERIAL_SLIDER_CLASS: &JNIStr = jni_str!("com/google/android/material/slider/Slider");
pub const MATERIAL_DIVIDER_CLASS: &JNIStr =
    jni_str!("com/google/android/material/divider/MaterialDivider");
pub const SHAPEABLE_IMAGE_VIEW_CLASS: &JNIStr =
    jni_str!("com/google/android/material/imageview/ShapeableImageView");
pub const MATERIAL_TAB_LAYOUT_CLASS: &JNIStr =
    jni_str!("com/google/android/material/tabs/TabLayout");
pub const MATERIAL_BOTTOM_NAVIGATION_VIEW_CLASS: &JNIStr =
    jni_str!("com/google/android/material/bottomnavigation/BottomNavigationView");
pub const MATERIAL_NAVIGATION_RAIL_VIEW_CLASS: &JNIStr =
    jni_str!("com/google/android/material/navigationrail/NavigationRailView");
pub const MATERIAL_NAVIGATION_VIEW_CLASS: &JNIStr =
    jni_str!("com/google/android/material/navigation/NavigationView");
pub const CAROUSEL_LAYOUT_MANAGER_CLASS: &JNIStr =
    jni_str!("com/google/android/material/carousel/CarouselLayoutManager");
pub const MATERIAL_ALERT_DIALOG_BUILDER_CLASS: &JNIStr =
    jni_str!("com/google/android/material/dialog/MaterialAlertDialogBuilder");
pub const MATERIAL_DATE_PICKER_CLASS: &JNIStr =
    jni_str!("com/google/android/material/datepicker/MaterialDatePicker");
pub const MATERIAL_DATE_PICKER_BUILDER_CLASS: &JNIStr =
    jni_str!("com/google/android/material/datepicker/MaterialDatePicker$Builder");
pub const MATERIAL_TIME_PICKER_CLASS: &JNIStr =
    jni_str!("com/google/android/material/timepicker/MaterialTimePicker");
pub const MATERIAL_TIME_PICKER_BUILDER_CLASS: &JNIStr =
    jni_str!("com/google/android/material/timepicker/MaterialTimePicker$Builder");
pub const MATERIAL_SNACKBAR_CLASS: &JNIStr =
    jni_str!("com/google/android/material/snackbar/Snackbar");
pub const MATERIAL_BADGE_DRAWABLE_CLASS: &JNIStr =
    jni_str!("com/google/android/material/badge/BadgeDrawable");
pub const MATERIAL_BADGE_UTILS_CLASS: &JNIStr =
    jni_str!("com/google/android/material/badge/BadgeUtils");
pub const MATERIAL_SHAPE_APPEARANCE_MODEL_CLASS: &JNIStr =
    jni_str!("com/google/android/material/shape/ShapeAppearanceModel");
pub const MATERIAL_DYNAMIC_COLORS_CLASS: &JNIStr =
    jni_str!("com/google/android/material/color/DynamicColors");
pub const MATERIAL_TOOLTIP_COMPAT_CLASS: &JNIStr =
    jni_str!("com/google/android/material/tooltip/TooltipCompat");
pub const VIEW_CLASS: &JNIStr = jni_str!("android/view/View");
pub const COLOR_STATE_LIST_CLASS: &JNIStr = jni_str!("android/content/res/ColorStateList");
pub const FRAGMENT_ACTIVITY_CLASS: &JNIStr = jni_str!("androidx/fragment/app/FragmentActivity");

/// Framework resource tables read through static fields.
pub const DRAWABLE_CLASS: &JNIStr = jni_str!("android/R$drawable");
pub const ATTR_CLASS: &JNIStr = jni_str!("android/R$attr");
pub const LAYOUT_CLASS: &JNIStr = jni_str!("android/R$layout");

// Names of the framework resources and layouts the demo screens use.
pub const DRAWABLE_DIALOG_INFO: &JNIStr = jni_str!("ic_dialog_info");
pub const DRAWABLE_MENU_CAMERA: &JNIStr = jni_str!("ic_menu_camera");
pub const DRAWABLE_MENU_REVERT: &JNIStr = jni_str!("ic_menu_revert");
pub const DRAWABLE_STAR_ON: &JNIStr = jni_str!("btn_star_big_on");
pub const DRAWABLE_CLOSE: &JNIStr = jni_str!("ic_menu_close_clear_cancel");

pub const SIMPLE_SPINNER_ITEM: &JNIStr = jni_str!("simple_spinner_item");
pub const SIMPLE_SPINNER_DROPDOWN_ITEM: &JNIStr = jni_str!("simple_spinner_dropdown_item");
pub const SIMPLE_LIST_ITEM_1: &JNIStr = jni_str!("simple_list_item_1");

// `ViewGroup.LayoutParams` dimension constants.
pub const MATCH_PARENT: jint = -1;
pub const WRAP_CONTENT: jint = -2;

// `LinearLayout` orientation values.
pub const LINEAR_LAYOUT_HORIZONTAL: jint = 0;
pub const LINEAR_LAYOUT_VERTICAL: jint = 1;

// `android.view.Gravity` flags.
//
// `Gravity.START` is available since API 17; the app's minSdk is 24.
pub const GRAVITY_CENTER_HORIZONTAL: jint = 0x01;
pub const GRAVITY_CENTER_VERTICAL: jint = 0x10;
pub const GRAVITY_TOP: jint = 0x30;
pub const GRAVITY_START: jint = 0x0080_0003;
pub const GRAVITY_TOP_START: jint = GRAVITY_TOP | GRAVITY_START;

/// `InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_PASSWORD`.
pub const INPUT_TYPE_TEXT_PASSWORD: jint = 0x0000_0081;

/// `R.attr.progressBarStyleHorizontal`, used to build a determinate progress bar.
pub const PROGRESS_BAR_STYLE_HORIZONTAL: &JNIStr = jni_str!("progressBarStyleHorizontal");

// Material `TextInputLayout` box and end-icon modes.
pub const MATERIAL_TEXT_INPUT_BOX_NONE: jint = 0;
pub const MATERIAL_TEXT_INPUT_BOX_FILLED: jint = 1;
pub const MATERIAL_TEXT_INPUT_BOX_OUTLINE: jint = 2;
pub const MATERIAL_END_ICON_NONE: jint = 0;
pub const MATERIAL_END_ICON_PASSWORD_TOGGLE: jint = 1;
pub const MATERIAL_END_ICON_CLEAR_TEXT: jint = 2;
pub const MATERIAL_END_ICON_DROPDOWN_MENU: jint = 3;

// `TabLayout` modes.
pub const MATERIAL_TAB_MODE_SCROLLABLE: jint = 0;
pub const MATERIAL_TAB_MODE_FIXED: jint = 1;
pub const MATERIAL_TAB_MODE_AUTO: jint = 2;

// `AppBarLayout.LayoutParams` scroll flags.
pub const APP_BAR_SCROLL_FLAG_SCROLL: jint = 0x1;
pub const APP_BAR_SCROLL_FLAG_ENTER_ALWAYS: jint = 0x4;

// `BaseTransientBottomBar` snackbar durations.
pub const MATERIAL_SNACKBAR_LENGTH_SHORT: jint = -1;
pub const MATERIAL_SNACKBAR_LENGTH_LONG: jint = 0;

/// `CornerFamily.ROUNDED`.
pub const MATERIAL_CORNER_FAMILY_ROUNDED: jint = 0;

// ARGB colours. Android's `setTextColor`/`setBackgroundColor` take an `int`.
pub const COLOR_SCREEN_BACKGROUND: jint = 0xFFF8_FAFCu32 as jint;
pub const COLOR_TEXT_PRIMARY: jint = 0xFF11_1827u32 as jint;
pub const COLOR_TEXT_SECONDARY: jint = 0xFF6B_7280u32 as jint;
pub const COLOR_BLUE: jint = 0xFF1A_73E8u32 as jint;
pub const COLOR_CHIP_BACKGROUND: jint = 0xFFFE_F7E0u32 as jint;
pub const COLOR_WIDGETS_BACKGROUND: jint = 0xFFEF_F6FFu32 as jint;
pub const COLOR_ICON_BACKGROUND: jint = 0xFFDC_E7F5u32 as jint;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_dimensions_match_the_framework_values() {
        assert_eq!(MATCH_PARENT, -1);
        assert_eq!(WRAP_CONTENT, -2);
    }

    #[test]
    fn linear_layout_orientations_match_the_framework_values() {
        assert_eq!(LINEAR_LAYOUT_HORIZONTAL, 0);
        assert_eq!(LINEAR_LAYOUT_VERTICAL, 1);
    }

    #[test]
    fn gravity_top_start_combines_both_flags() {
        assert_eq!(GRAVITY_TOP_START, GRAVITY_TOP | GRAVITY_START);
        assert_eq!(GRAVITY_TOP_START & GRAVITY_TOP, GRAVITY_TOP);
        assert_eq!(GRAVITY_TOP_START & GRAVITY_START, GRAVITY_START);
    }

    #[test]
    fn password_input_type_combines_text_class_and_variation() {
        assert_eq!(INPUT_TYPE_TEXT_PASSWORD, 0x0000_0081);
    }

    #[test]
    fn all_colours_are_opaque_argb_values() {
        let colours = [
            COLOR_SCREEN_BACKGROUND,
            COLOR_TEXT_PRIMARY,
            COLOR_TEXT_SECONDARY,
            COLOR_BLUE,
            COLOR_CHIP_BACKGROUND,
            COLOR_WIDGETS_BACKGROUND,
            COLOR_ICON_BACKGROUND,
        ];

        for colour in colours {
            assert_eq!((colour as u32) >> 24, 0xFF);
        }
    }

    #[test]
    fn material_classes_use_their_library_class_names() {
        assert_eq!(
            MATERIAL_BUTTON_CLASS.to_str().as_ref(),
            "com/google/android/material/button/MaterialButton"
        );
        assert_eq!(
            MATERIAL_TEXT_INPUT_LAYOUT_CLASS.to_str().as_ref(),
            "com/google/android/material/textfield/TextInputLayout"
        );
        assert_eq!(
            MATERIAL_DATE_PICKER_BUILDER_CLASS.to_str().as_ref(),
            "com/google/android/material/datepicker/MaterialDatePicker$Builder"
        );
        assert_eq!(
            MATERIAL_TIME_PICKER_BUILDER_CLASS.to_str().as_ref(),
            "com/google/android/material/timepicker/MaterialTimePicker$Builder"
        );
        assert_eq!(
            CAROUSEL_LAYOUT_MANAGER_CLASS.to_str().as_ref(),
            "com/google/android/material/carousel/CarouselLayoutManager"
        );
    }

    #[test]
    fn material_modes_match_the_library_values() {
        assert_eq!(MATERIAL_TEXT_INPUT_BOX_NONE, 0);
        assert_eq!(MATERIAL_TEXT_INPUT_BOX_FILLED, 1);
        assert_eq!(MATERIAL_TEXT_INPUT_BOX_OUTLINE, 2);
        assert_eq!(MATERIAL_END_ICON_NONE, 0);
        assert_eq!(MATERIAL_END_ICON_PASSWORD_TOGGLE, 1);
        assert_eq!(MATERIAL_END_ICON_CLEAR_TEXT, 2);
        assert_eq!(MATERIAL_END_ICON_DROPDOWN_MENU, 3);
        assert_eq!(MATERIAL_TAB_MODE_SCROLLABLE, 0);
        assert_eq!(MATERIAL_TAB_MODE_FIXED, 1);
        assert_eq!(MATERIAL_TAB_MODE_AUTO, 2);
        assert_eq!(MATERIAL_CORNER_FAMILY_ROUNDED, 0);
        assert_eq!(MATERIAL_SNACKBAR_LENGTH_SHORT, -1);
        assert_eq!(MATERIAL_SNACKBAR_LENGTH_LONG, 0);
    }

    #[test]
    fn app_bar_scroll_flags_are_distinct_bits() {
        assert_eq!(APP_BAR_SCROLL_FLAG_SCROLL, 0x1);
        assert_eq!(APP_BAR_SCROLL_FLAG_ENTER_ALWAYS, 0x4);
        assert_eq!(
            APP_BAR_SCROLL_FLAG_SCROLL & APP_BAR_SCROLL_FLAG_ENTER_ALWAYS,
            0
        );
    }

    #[test]
    fn log_tag_is_stable() {
        assert_eq!(LOG_TAG, "RustUi");
    }

    #[test]
    fn androidx_widgets_use_their_library_class_names() {
        assert_eq!(
            RECYCLER_VIEW_CLASS.to_str().as_ref(),
            "androidx/recyclerview/widget/RecyclerView"
        );
        assert_eq!(
            LINEAR_LAYOUT_MANAGER_CLASS.to_str().as_ref(),
            "androidx/recyclerview/widget/LinearLayoutManager"
        );
        assert_eq!(
            GRID_LAYOUT_MANAGER_CLASS.to_str().as_ref(),
            "androidx/recyclerview/widget/GridLayoutManager"
        );
        assert_eq!(
            VIEW_PAGER_2_CLASS.to_str().as_ref(),
            "androidx/viewpager2/widget/ViewPager2"
        );
        assert_eq!(
            APP_COMPAT_TOOLBAR_CLASS.to_str().as_ref(),
            "androidx/appcompat/widget/Toolbar"
        );
    }
}
