//! Construction and configuration of the Android views behind the declarative tree.
//!
//! Every widget is a real framework class instantiated through its `(Context)` constructor. The
//! helpers here keep the JNI plumbing out of [`super::render`] and the screen definitions.

use jni::objects::{JObject, JObjectArray, JString, JValue};
use jni::strings::JNIStr;
use jni::sys::{jint, jsize};
use jni::{Env, jni_sig, jni_str};

use super::constants::*;
use super::env::EnvExt;
use super::error::Result;

/// The `ViewGroup` classes the bridge instantiates directly.
#[derive(Clone, Copy, Debug)]
pub enum ViewGroupKind {
    LinearVertical,
    LinearHorizontal,
    Scroll,
}

impl ViewGroupKind {
    fn class_name(self) -> &'static JNIStr {
        match self {
            Self::LinearVertical | Self::LinearHorizontal => LINEAR_LAYOUT_CLASS,
            Self::Scroll => SCROLL_VIEW_CLASS,
        }
    }

    fn configure(self, env: &mut Env<'_>, group: &JObject<'_>) -> Result<()> {
        match self {
            Self::LinearVertical => env.call_void(
                group,
                jni_str!("setOrientation"),
                jni_sig!("(I)V"),
                &[JValue::Int(LINEAR_LAYOUT_VERTICAL)],
            ),
            Self::LinearHorizontal => env.call_void(
                group,
                jni_str!("setOrientation"),
                jni_sig!("(I)V"),
                &[JValue::Int(LINEAR_LAYOUT_HORIZONTAL)],
            ),
            Self::Scroll => env.call_void(
                group,
                jni_str!("setFillViewport"),
                jni_sig!("(Z)V"),
                &[JValue::Bool(true)],
            ),
        }
    }
}

/// The layout-parameter class a parent expects from its children.
#[derive(Clone, Copy, Debug)]
pub enum LayoutParamsKind {
    Linear,
    Frame,
    AppBar,
}

/// Creates a `View` subclass through its `(Context)` constructor.
pub fn new_widget<'local>(
    env: &mut Env<'local>,
    class_name: &JNIStr,
    context: &JObject<'_>,
) -> Result<JObject<'local>> {
    env.construct(
        class_name,
        jni_sig!("(Landroid/content/Context;)V"),
        &[JValue::Object(context)],
    )
}

/// Creates a `ViewGroup` and applies the configuration that defines its kind.
pub fn new_view_group<'local>(
    env: &mut Env<'local>,
    kind: ViewGroupKind,
    context: &JObject<'_>,
) -> Result<JObject<'local>> {
    let group = new_widget(env, kind.class_name(), context)?;
    kind.configure(env, &group)?;
    Ok(group)
}

/// Creates an empty `androidx.recyclerview.widget.RecyclerView`.
///
/// A layout manager and an adapter are applied by the renderer.
pub fn new_recycler_view<'local>(
    env: &mut Env<'local>,
    context: &JObject<'_>,
) -> Result<JObject<'local>> {
    new_widget(env, RECYCLER_VIEW_CLASS, context)
}

/// Creates an empty `androidx.viewpager2.widget.ViewPager2`.
///
/// A `RecyclerView.Adapter` is applied by the renderer.
pub fn new_view_pager<'local>(
    env: &mut Env<'local>,
    context: &JObject<'_>,
) -> Result<JObject<'local>> {
    new_widget(env, VIEW_PAGER_2_CLASS, context)
}

/// Creates an empty `androidx.appcompat.widget.Toolbar`.
pub fn new_app_compat_toolbar<'local>(
    env: &mut Env<'local>,
    context: &JObject<'_>,
) -> Result<JObject<'local>> {
    new_widget(env, APP_COMPAT_TOOLBAR_CLASS, context)
}

/// Creates a `CarouselLayoutManager` with the requested orientation.
pub fn new_carousel_layout_manager<'local>(
    env: &mut Env<'local>,
    orientation: jint,
) -> Result<JObject<'local>> {
    let manager = env.construct(CAROUSEL_LAYOUT_MANAGER_CLASS, jni_sig!("()V"), &[])?;
    env.call_void(
        &manager,
        jni_str!("setOrientation"),
        jni_sig!("(I)V"),
        &[JValue::Int(orientation)],
    )?;
    Ok(manager)
}

/// Generates a positive view id that is unique within the process.
pub fn new_view_id(env: &mut Env<'_>) -> Result<jint> {
    env.call_static_int(VIEW_CLASS, jni_str!("generateViewId"), jni_sig!("()I"), &[])
}

/// Wraps a colour in an `android.content.res.ColorStateList`.
pub fn color_state_list<'local>(env: &mut Env<'local>, color: jint) -> Result<JObject<'local>> {
    env.call_static_object(
        COLOR_STATE_LIST_CLASS,
        jni_str!("valueOf"),
        jni_sig!("(I)Landroid/content/res/ColorStateList;"),
        &[JValue::Int(color)],
    )
}

/// Builds a `ShapeAppearanceModel` with uniformly rounded corners.
pub fn rounded_shape_model<'local>(env: &mut Env<'local>, radius: f32) -> Result<JObject<'local>> {
    let builder = env.call_static_object(
        MATERIAL_SHAPE_APPEARANCE_MODEL_CLASS,
        jni_str!("builder"),
        jni_sig!("()Lcom/google/android/material/shape/ShapeAppearanceModel$Builder;"),
        &[],
    )?;
    let builder = env.call_object(
        &builder,
        jni_str!("setAllCorners"),
        jni_sig!("(IF)Lcom/google/android/material/shape/ShapeAppearanceModel$Builder;"),
        &[
            JValue::Int(MATERIAL_CORNER_FAMILY_ROUNDED),
            JValue::Float(radius),
        ],
    )?;
    env.call_object(
        &builder,
        jni_str!("build"),
        jni_sig!("()Lcom/google/android/material/shape/ShapeAppearanceModel;"),
        &[],
    )
}

/// Adds a titled item to a `Menu` and returns the new `MenuItem`.
pub fn add_menu_item<'local>(
    env: &mut Env<'local>,
    menu: &JObject<'_>,
    item_id: jint,
    title: &str,
) -> Result<JObject<'local>> {
    let title = env.java_string(title)?;
    env.call_object(
        menu,
        jni_str!("add"),
        jni_sig!("(IIILjava/lang/CharSequence;)Landroid/view/MenuItem;"),
        &[
            JValue::Int(0),
            JValue::Int(item_id),
            JValue::Int(0),
            JValue::Object(title.as_ref()),
        ],
    )
}

/// Creates a `LinearLayoutManager` with the requested orientation.
pub fn new_linear_layout_manager<'local>(
    env: &mut Env<'local>,
    context: &JObject<'_>,
    orientation: jint,
) -> Result<JObject<'local>> {
    let manager = env.construct(
        LINEAR_LAYOUT_MANAGER_CLASS,
        jni_sig!("(Landroid/content/Context;)V"),
        &[JValue::Object(context)],
    )?;
    env.call_void(
        &manager,
        jni_str!("setOrientation"),
        jni_sig!("(I)V"),
        &[JValue::Int(orientation)],
    )?;
    Ok(manager)
}

/// Creates a `GridLayoutManager` with the requested span count and orientation.
pub fn new_grid_layout_manager<'local>(
    env: &mut Env<'local>,
    context: &JObject<'_>,
    span_count: jint,
    orientation: jint,
) -> Result<JObject<'local>> {
    let manager = env.construct(
        GRID_LAYOUT_MANAGER_CLASS,
        jni_sig!("(Landroid/content/Context;I)V"),
        &[JValue::Object(context), JValue::Int(span_count)],
    )?;
    env.call_void(
        &manager,
        jni_str!("setOrientation"),
        jni_sig!("(I)V"),
        &[JValue::Int(orientation)],
    )?;
    Ok(manager)
}

/// Creates a `java.lang.String[]` populated with `items`.
///
/// Item-adapter factories supplied through [`ItemAdapterFactory`] can pass the result to an
/// adapter constructor without repeating the JNI array plumbing.
pub fn new_string_array<'local>(
    env: &mut Env<'local>,
    items: &[&str],
) -> Result<JObjectArray<'local>> {
    let array = env
        .new_object_array(
            items.len() as jsize,
            jni_str!("java/lang/String"),
            JObject::null(),
        )
        .map_err(|error| super::error::Error::context("creating a String array", error))?;

    for (index, item) in items.iter().enumerate() {
        let item = env.java_string(item)?;
        array
            .set_element(env, index, &item)
            .map_err(|error| super::error::Error::context("filling a String array", error))?;
    }

    env.check_exception("creating a String array")?;
    Ok(array)
}

/// How item views should be sized by the application's adapter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdapterItemSizing {
    /// List rows: match the parent's width and wrap the content height.
    WrapContent,
    /// Pager pages: match the parent in both dimensions, as `ViewPager2` requires.
    FillParent,
    /// Carousel items: match the parent in both dimensions and use a
    /// `com.google.android.material.carousel.MaskableFrameLayout` root, as
    /// `CarouselLayoutManager` requires.
    Carousel,
}

/// Creates the concrete `RecyclerView.Adapter` used by [`super::ui::Recycler`] and
/// [`super::ui::ViewPager`].
///
/// `RecyclerView`, `ViewPager2`, and the Material carousel need an adapter subclass, which cannot be
/// expressed as a framework class from Rust. The application supplies one small adapter class and
/// registers a factory with [`super::Renderer::with_item_adapter_factory`]. The adapter is expected
/// to fill itself from the supplied item strings, sizing each item according to
/// [`AdapterItemSizing`].
pub trait ItemAdapterFactory {
    fn create_adapter<'local>(
        &self,
        env: &mut Env<'local>,
        context: &JObject<'_>,
        items: &[&str],
        sizing: AdapterItemSizing,
    ) -> Result<JObject<'local>>;
}

/// Creates a horizontal, determinate `ProgressBar`.
///
/// `ProgressBar(Context)` defaults to an indeterminate spinner, so the constructor that accepts a
/// `defStyleAttr` is used with `android.R.attr.progressBarStyleHorizontal`.
pub fn new_progress_bar<'local>(
    env: &mut Env<'local>,
    context: &JObject<'_>,
) -> Result<JObject<'local>> {
    let style = env.static_int(ATTR_CLASS, PROGRESS_BAR_STYLE_HORIZONTAL)?;
    let null = JObject::null();
    env.construct(
        PROGRESS_BAR_CLASS,
        jni_sig!("(Landroid/content/Context;Landroid/util/AttributeSet;I)V"),
        &[
            JValue::Object(context),
            JValue::Object(&null),
            JValue::Int(style),
        ],
    )
}

/// Creates an `ArrayAdapter` populated with `items`.
pub fn new_array_adapter<'local>(
    env: &mut Env<'local>,
    context: &JObject<'_>,
    resource: &JNIStr,
    items: &[&str],
) -> Result<JObject<'local>> {
    let resource = env.static_int(LAYOUT_CLASS, resource)?;
    let adapter = env.construct(
        ARRAY_ADAPTER_CLASS,
        jni_sig!("(Landroid/content/Context;I)V"),
        &[JValue::Object(context), JValue::Int(resource)],
    )?;

    for item in items {
        let text = env.java_string(item)?;
        env.call_void(
            &adapter,
            jni_str!("add"),
            jni_sig!("(Ljava/lang/Object;)V"),
            &[JValue::Object(text.as_ref())],
        )?;
    }

    Ok(adapter)
}

/// Creates the `CommaTokenizer` used by `MultiAutoCompleteTextView`.
pub fn new_comma_tokenizer<'local>(env: &mut Env<'local>) -> Result<JObject<'local>> {
    env.construct(COMMA_TOKENIZER_CLASS, jni_sig!("()V"), &[])
}

/// Creates layout parameters for `parent_kind`.
pub fn new_layout_params<'local>(
    env: &mut Env<'local>,
    parent_kind: LayoutParamsKind,
    width: jint,
    height: jint,
) -> Result<JObject<'local>> {
    let params = match parent_kind {
        LayoutParamsKind::Linear => env.construct(
            LINEAR_LAYOUT_PARAMS_CLASS,
            jni_sig!("(II)V"),
            &[JValue::Int(width), JValue::Int(height)],
        )?,
        // `FrameLayout.LayoutParams` exposes gravity through its `(int, int, int)` constructor
        // rather than a `setGravity` method.
        LayoutParamsKind::Frame => env.construct(
            FRAME_LAYOUT_PARAMS_CLASS,
            jni_sig!("(III)V"),
            &[
                JValue::Int(width),
                JValue::Int(height),
                JValue::Int(GRAVITY_TOP_START),
            ],
        )?,
        LayoutParamsKind::AppBar => {
            let params = env.construct(
                APP_BAR_LAYOUT_PARAMS_CLASS,
                jni_sig!("(II)V"),
                &[JValue::Int(width), JValue::Int(height)],
            )?;
            env.call_void(
                &params,
                jni_str!("setScrollFlags"),
                jni_sig!("(I)V"),
                &[JValue::Int(
                    APP_BAR_SCROLL_FLAG_SCROLL | APP_BAR_SCROLL_FLAG_ENTER_ALWAYS,
                )],
            )?;
            params
        }
    };

    Ok(params)
}

/// Applies margins in pixels to a `MarginLayoutParams`.
pub fn set_margins(
    env: &mut Env<'_>,
    params: &JObject<'_>,
    (left, top, right, bottom): (jint, jint, jint, jint),
) -> Result<()> {
    env.call_void(
        params,
        jni_str!("setMargins"),
        jni_sig!("(IIII)V"),
        &[
            JValue::Int(left),
            JValue::Int(top),
            JValue::Int(right),
            JValue::Int(bottom),
        ],
    )
}

/// Adds `child` to `parent` with the supplied layout parameters.
pub fn add_child(
    env: &mut Env<'_>,
    parent: &JObject<'_>,
    child: &JObject<'_>,
    params: &JObject<'_>,
) -> Result<()> {
    env.call_void(
        parent,
        jni_str!("addView"),
        jni_sig!("(Landroid/view/View;Landroid/view/ViewGroup$LayoutParams;)V"),
        &[JValue::Object(child), JValue::Object(params)],
    )
}

/// Reads the tag written by [`ViewExt::set_tag`].
///
/// The bridge only ever writes string tags, so a non-null tag is read as a `String`.
pub fn view_tag(env: &mut Env<'_>, view: &JObject<'_>) -> Result<Option<String>> {
    let tag = env.call_object(
        view,
        jni_str!("getTag"),
        jni_sig!("()Ljava/lang/Object;"),
        &[],
    )?;

    if tag.is_null() {
        return Ok(None);
    }

    let tag = unsafe { JString::from_raw(env, tag.into_raw()) };
    Ok(Some(tag.to_string()))
}

/// Setters for the widget properties the declarative components can express.
///
/// These are implemented on [`Env`] so rendering code reads as a sequence of view updates.
pub trait ViewExt<'local> {
    fn set_text(&mut self, view: &JObject<'_>, text: &str) -> Result<()>;
    fn set_hint(&mut self, view: &JObject<'_>, hint: &str) -> Result<()>;
    fn set_text_on(&mut self, view: &JObject<'_>, text: &str) -> Result<()>;
    fn set_text_off(&mut self, view: &JObject<'_>, text: &str) -> Result<()>;
    fn set_query_hint(&mut self, view: &JObject<'_>, hint: &str) -> Result<()>;
    fn set_title(&mut self, view: &JObject<'_>, title: &str) -> Result<()>;
    fn set_subtitle(&mut self, view: &JObject<'_>, subtitle: &str) -> Result<()>;
    fn set_text_size(&mut self, view: &JObject<'_>, size_sp: f32) -> Result<()>;
    fn set_text_color(&mut self, view: &JObject<'_>, color: jint) -> Result<()>;
    fn set_gravity(&mut self, view: &JObject<'_>, gravity: jint) -> Result<()>;
    fn set_padding(&mut self, view: &JObject<'_>, padding: (jint, jint, jint, jint)) -> Result<()>;
    fn set_background_color(&mut self, view: &JObject<'_>, color: jint) -> Result<()>;
    fn set_tag(&mut self, view: &JObject<'_>, tag: &str) -> Result<()>;
    fn set_all_caps(&mut self, view: &JObject<'_>, all_caps: bool) -> Result<()>;
    fn set_enabled(&mut self, view: &JObject<'_>, enabled: bool) -> Result<()>;
    fn set_orientation(&mut self, view: &JObject<'_>, orientation: jint) -> Result<()>;
    fn set_layout_manager(&mut self, view: &JObject<'_>, manager: &JObject<'_>) -> Result<()>;
    fn set_recycler_adapter(&mut self, view: &JObject<'_>, adapter: &JObject<'_>) -> Result<()>;
    fn set_has_fixed_size(&mut self, view: &JObject<'_>, has_fixed_size: bool) -> Result<()>;
    fn set_offscreen_page_limit(&mut self, view: &JObject<'_>, limit: jint) -> Result<()>;
    fn set_current_item(&mut self, view: &JObject<'_>, item: jint) -> Result<()>;
    fn set_user_input_enabled(&mut self, view: &JObject<'_>, enabled: bool) -> Result<()>;
    fn set_input_type(&mut self, view: &JObject<'_>, input_type: jint) -> Result<()>;
    fn set_checked(&mut self, view: &JObject<'_>, checked: bool) -> Result<()>;
    fn set_max(&mut self, view: &JObject<'_>, max: jint) -> Result<()>;
    fn set_progress(&mut self, view: &JObject<'_>, progress: jint) -> Result<()>;
    fn set_num_stars(&mut self, view: &JObject<'_>, num_stars: jint) -> Result<()>;
    fn set_rating(&mut self, view: &JObject<'_>, rating: f32) -> Result<()>;
    fn set_step_size(&mut self, view: &JObject<'_>, step_size: f32) -> Result<()>;
    fn set_is_indicator(&mut self, view: &JObject<'_>, indicator: bool) -> Result<()>;
    fn set_threshold(&mut self, view: &JObject<'_>, threshold: jint) -> Result<()>;
    fn set_min_value(&mut self, view: &JObject<'_>, min: jint) -> Result<()>;
    fn set_max_value(&mut self, view: &JObject<'_>, max: jint) -> Result<()>;
    fn set_value(&mut self, view: &JObject<'_>, value: jint) -> Result<()>;
    fn set_id(&mut self, view: &JObject<'_>, id: jint) -> Result<()>;
    fn set_checkable(&mut self, view: &JObject<'_>, checkable: bool) -> Result<()>;
    fn set_close_icon_resource(&mut self, view: &JObject<'_>, resource: jint) -> Result<()>;
    fn set_chip_icon_resource(&mut self, view: &JObject<'_>, resource: jint) -> Result<()>;
    fn set_icon_resource(&mut self, view: &JObject<'_>, resource: jint) -> Result<()>;
    fn set_box_background_mode(&mut self, view: &JObject<'_>, mode: jint) -> Result<()>;
    fn set_helper_text(&mut self, view: &JObject<'_>, text: &str) -> Result<()>;
    fn set_error_text(&mut self, view: &JObject<'_>, text: &str) -> Result<()>;
    fn set_end_icon_mode(&mut self, view: &JObject<'_>, mode: jint) -> Result<()>;
    fn set_single_selection(&mut self, view: &JObject<'_>, single: bool) -> Result<()>;
    fn set_selection_required(&mut self, view: &JObject<'_>, required: bool) -> Result<()>;
    fn set_single_line(&mut self, view: &JObject<'_>, single_line: bool) -> Result<()>;
    fn set_slider_value(&mut self, view: &JObject<'_>, value: f32) -> Result<()>;
    fn set_value_from(&mut self, view: &JObject<'_>, value: f32) -> Result<()>;
    fn set_value_to(&mut self, view: &JObject<'_>, value: f32) -> Result<()>;
    fn set_indeterminate(&mut self, view: &JObject<'_>, indeterminate: bool) -> Result<()>;
    fn set_divider_thickness(&mut self, view: &JObject<'_>, thickness: jint) -> Result<()>;
    fn set_divider_color(&mut self, view: &JObject<'_>, color: jint) -> Result<()>;
    fn set_divider_inset_start(&mut self, view: &JObject<'_>, inset: jint) -> Result<()>;
    fn set_divider_inset_end(&mut self, view: &JObject<'_>, inset: jint) -> Result<()>;
    fn set_card_radius(&mut self, view: &JObject<'_>, radius: f32) -> Result<()>;
    fn set_card_elevation(&mut self, view: &JObject<'_>, elevation: f32) -> Result<()>;
    fn set_stroke_width(&mut self, view: &JObject<'_>, width: jint) -> Result<()>;
    fn set_stroke_color(&mut self, view: &JObject<'_>, colors: &JObject<'_>) -> Result<()>;
    fn set_shape_appearance_model(&mut self, view: &JObject<'_>, model: &JObject<'_>)
    -> Result<()>;
    fn set_tab_mode(&mut self, view: &JObject<'_>, mode: jint) -> Result<()>;
    fn set_selected_item_id(&mut self, view: &JObject<'_>, id: jint) -> Result<()>;
    fn set_menu_item_icon(&mut self, item: &JObject<'_>, resource: jint) -> Result<()>;
    fn set_tooltip_text(&mut self, view: &JObject<'_>, text: &str) -> Result<()>;
}

impl<'local> ViewExt<'local> for Env<'local> {
    fn set_text(&mut self, view: &JObject<'_>, text: &str) -> Result<()> {
        let text = self.java_string(text)?;
        self.call_void(
            view,
            jni_str!("setText"),
            jni_sig!("(Ljava/lang/CharSequence;)V"),
            &[JValue::Object(text.as_ref())],
        )
    }

    fn set_hint(&mut self, view: &JObject<'_>, hint: &str) -> Result<()> {
        let hint = self.java_string(hint)?;
        self.call_void(
            view,
            jni_str!("setHint"),
            jni_sig!("(Ljava/lang/CharSequence;)V"),
            &[JValue::Object(hint.as_ref())],
        )
    }

    fn set_text_on(&mut self, view: &JObject<'_>, text: &str) -> Result<()> {
        let text = self.java_string(text)?;
        self.call_void(
            view,
            jni_str!("setTextOn"),
            jni_sig!("(Ljava/lang/CharSequence;)V"),
            &[JValue::Object(text.as_ref())],
        )
    }

    fn set_text_off(&mut self, view: &JObject<'_>, text: &str) -> Result<()> {
        let text = self.java_string(text)?;
        self.call_void(
            view,
            jni_str!("setTextOff"),
            jni_sig!("(Ljava/lang/CharSequence;)V"),
            &[JValue::Object(text.as_ref())],
        )
    }

    fn set_query_hint(&mut self, view: &JObject<'_>, hint: &str) -> Result<()> {
        let hint = self.java_string(hint)?;
        self.call_void(
            view,
            jni_str!("setQueryHint"),
            jni_sig!("(Ljava/lang/CharSequence;)V"),
            &[JValue::Object(hint.as_ref())],
        )
    }

    fn set_title(&mut self, view: &JObject<'_>, title: &str) -> Result<()> {
        let title = self.java_string(title)?;
        self.call_void(
            view,
            jni_str!("setTitle"),
            jni_sig!("(Ljava/lang/CharSequence;)V"),
            &[JValue::Object(title.as_ref())],
        )
    }

    fn set_subtitle(&mut self, view: &JObject<'_>, subtitle: &str) -> Result<()> {
        let subtitle = self.java_string(subtitle)?;
        self.call_void(
            view,
            jni_str!("setSubtitle"),
            jni_sig!("(Ljava/lang/CharSequence;)V"),
            &[JValue::Object(subtitle.as_ref())],
        )
    }

    fn set_text_size(&mut self, view: &JObject<'_>, size_sp: f32) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setTextSize"),
            jni_sig!("(F)V"),
            &[JValue::Float(size_sp)],
        )
    }

    fn set_text_color(&mut self, view: &JObject<'_>, color: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setTextColor"),
            jni_sig!("(I)V"),
            &[JValue::Int(color)],
        )
    }

    fn set_gravity(&mut self, view: &JObject<'_>, gravity: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setGravity"),
            jni_sig!("(I)V"),
            &[JValue::Int(gravity)],
        )
    }

    fn set_padding(
        &mut self,
        view: &JObject<'_>,
        (left, top, right, bottom): (jint, jint, jint, jint),
    ) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setPadding"),
            jni_sig!("(IIII)V"),
            &[
                JValue::Int(left),
                JValue::Int(top),
                JValue::Int(right),
                JValue::Int(bottom),
            ],
        )
    }

    fn set_background_color(&mut self, view: &JObject<'_>, color: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setBackgroundColor"),
            jni_sig!("(I)V"),
            &[JValue::Int(color)],
        )
    }

    fn set_tag(&mut self, view: &JObject<'_>, tag: &str) -> Result<()> {
        let tag = self.java_string(tag)?;
        self.call_void(
            view,
            jni_str!("setTag"),
            jni_sig!("(Ljava/lang/Object;)V"),
            &[JValue::Object(tag.as_ref())],
        )
    }

    fn set_all_caps(&mut self, view: &JObject<'_>, all_caps: bool) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setAllCaps"),
            jni_sig!("(Z)V"),
            &[JValue::Bool(all_caps)],
        )
    }

    fn set_enabled(&mut self, view: &JObject<'_>, enabled: bool) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setEnabled"),
            jni_sig!("(Z)V"),
            &[JValue::Bool(enabled)],
        )
    }

    fn set_orientation(&mut self, view: &JObject<'_>, orientation: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setOrientation"),
            jni_sig!("(I)V"),
            &[JValue::Int(orientation)],
        )
    }

    fn set_layout_manager(&mut self, view: &JObject<'_>, manager: &JObject<'_>) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setLayoutManager"),
            jni_sig!("(Landroidx/recyclerview/widget/RecyclerView$LayoutManager;)V"),
            &[JValue::Object(manager)],
        )
    }

    fn set_recycler_adapter(&mut self, view: &JObject<'_>, adapter: &JObject<'_>) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setAdapter"),
            jni_sig!("(Landroidx/recyclerview/widget/RecyclerView$Adapter;)V"),
            &[JValue::Object(adapter)],
        )
    }

    fn set_has_fixed_size(&mut self, view: &JObject<'_>, has_fixed_size: bool) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setHasFixedSize"),
            jni_sig!("(Z)V"),
            &[JValue::Bool(has_fixed_size)],
        )
    }

    fn set_offscreen_page_limit(&mut self, view: &JObject<'_>, limit: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setOffscreenPageLimit"),
            jni_sig!("(I)V"),
            &[JValue::Int(limit)],
        )
    }

    fn set_current_item(&mut self, view: &JObject<'_>, item: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setCurrentItem"),
            jni_sig!("(I)V"),
            &[JValue::Int(item)],
        )
    }

    fn set_user_input_enabled(&mut self, view: &JObject<'_>, enabled: bool) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setUserInputEnabled"),
            jni_sig!("(Z)V"),
            &[JValue::Bool(enabled)],
        )
    }

    fn set_input_type(&mut self, view: &JObject<'_>, input_type: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setInputType"),
            jni_sig!("(I)V"),
            &[JValue::Int(input_type)],
        )
    }

    fn set_checked(&mut self, view: &JObject<'_>, checked: bool) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setChecked"),
            jni_sig!("(Z)V"),
            &[JValue::Bool(checked)],
        )
    }

    fn set_max(&mut self, view: &JObject<'_>, max: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setMax"),
            jni_sig!("(I)V"),
            &[JValue::Int(max)],
        )
    }

    fn set_progress(&mut self, view: &JObject<'_>, progress: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setProgress"),
            jni_sig!("(I)V"),
            &[JValue::Int(progress)],
        )
    }

    fn set_num_stars(&mut self, view: &JObject<'_>, num_stars: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setNumStars"),
            jni_sig!("(I)V"),
            &[JValue::Int(num_stars)],
        )
    }

    fn set_rating(&mut self, view: &JObject<'_>, rating: f32) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setRating"),
            jni_sig!("(F)V"),
            &[JValue::Float(rating)],
        )
    }

    fn set_step_size(&mut self, view: &JObject<'_>, step_size: f32) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setStepSize"),
            jni_sig!("(F)V"),
            &[JValue::Float(step_size)],
        )
    }

    fn set_is_indicator(&mut self, view: &JObject<'_>, indicator: bool) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setIsIndicator"),
            jni_sig!("(Z)V"),
            &[JValue::Bool(indicator)],
        )
    }

    fn set_threshold(&mut self, view: &JObject<'_>, threshold: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setThreshold"),
            jni_sig!("(I)V"),
            &[JValue::Int(threshold)],
        )
    }

    fn set_min_value(&mut self, view: &JObject<'_>, min: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setMinValue"),
            jni_sig!("(I)V"),
            &[JValue::Int(min)],
        )
    }

    fn set_max_value(&mut self, view: &JObject<'_>, max: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setMaxValue"),
            jni_sig!("(I)V"),
            &[JValue::Int(max)],
        )
    }

    fn set_value(&mut self, view: &JObject<'_>, value: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setValue"),
            jni_sig!("(I)V"),
            &[JValue::Int(value)],
        )
    }

    fn set_id(&mut self, view: &JObject<'_>, id: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setId"),
            jni_sig!("(I)V"),
            &[JValue::Int(id)],
        )
    }

    fn set_checkable(&mut self, view: &JObject<'_>, checkable: bool) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setCheckable"),
            jni_sig!("(Z)V"),
            &[JValue::Bool(checkable)],
        )
    }

    fn set_close_icon_resource(&mut self, view: &JObject<'_>, resource: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setCloseIconResource"),
            jni_sig!("(I)V"),
            &[JValue::Int(resource)],
        )
    }

    fn set_chip_icon_resource(&mut self, view: &JObject<'_>, resource: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setChipIconResource"),
            jni_sig!("(I)V"),
            &[JValue::Int(resource)],
        )
    }

    fn set_icon_resource(&mut self, view: &JObject<'_>, resource: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setIconResource"),
            jni_sig!("(I)V"),
            &[JValue::Int(resource)],
        )
    }

    fn set_box_background_mode(&mut self, view: &JObject<'_>, mode: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setBoxBackgroundMode"),
            jni_sig!("(I)V"),
            &[JValue::Int(mode)],
        )
    }

    fn set_helper_text(&mut self, view: &JObject<'_>, text: &str) -> Result<()> {
        let text = self.java_string(text)?;
        self.call_void(
            view,
            jni_str!("setHelperText"),
            jni_sig!("(Ljava/lang/CharSequence;)V"),
            &[JValue::Object(text.as_ref())],
        )
    }

    fn set_error_text(&mut self, view: &JObject<'_>, text: &str) -> Result<()> {
        let text = self.java_string(text)?;
        self.call_void(
            view,
            jni_str!("setError"),
            jni_sig!("(Ljava/lang/CharSequence;)V"),
            &[JValue::Object(text.as_ref())],
        )
    }

    fn set_end_icon_mode(&mut self, view: &JObject<'_>, mode: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setEndIconMode"),
            jni_sig!("(I)V"),
            &[JValue::Int(mode)],
        )
    }

    fn set_single_selection(&mut self, view: &JObject<'_>, single: bool) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setSingleSelection"),
            jni_sig!("(Z)V"),
            &[JValue::Bool(single)],
        )
    }

    fn set_selection_required(&mut self, view: &JObject<'_>, required: bool) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setSelectionRequired"),
            jni_sig!("(Z)V"),
            &[JValue::Bool(required)],
        )
    }

    fn set_single_line(&mut self, view: &JObject<'_>, single_line: bool) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setSingleLine"),
            jni_sig!("(Z)V"),
            &[JValue::Bool(single_line)],
        )
    }

    fn set_slider_value(&mut self, view: &JObject<'_>, value: f32) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setValue"),
            jni_sig!("(F)V"),
            &[JValue::Float(value)],
        )
    }

    fn set_value_from(&mut self, view: &JObject<'_>, value: f32) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setValueFrom"),
            jni_sig!("(F)V"),
            &[JValue::Float(value)],
        )
    }

    fn set_value_to(&mut self, view: &JObject<'_>, value: f32) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setValueTo"),
            jni_sig!("(F)V"),
            &[JValue::Float(value)],
        )
    }

    fn set_indeterminate(&mut self, view: &JObject<'_>, indeterminate: bool) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setIndeterminate"),
            jni_sig!("(Z)V"),
            &[JValue::Bool(indeterminate)],
        )
    }

    fn set_divider_thickness(&mut self, view: &JObject<'_>, thickness: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setDividerThickness"),
            jni_sig!("(I)V"),
            &[JValue::Int(thickness)],
        )
    }

    fn set_divider_color(&mut self, view: &JObject<'_>, color: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setDividerColor"),
            jni_sig!("(I)V"),
            &[JValue::Int(color)],
        )
    }

    fn set_divider_inset_start(&mut self, view: &JObject<'_>, inset: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setDividerInsetStart"),
            jni_sig!("(I)V"),
            &[JValue::Int(inset)],
        )
    }

    fn set_divider_inset_end(&mut self, view: &JObject<'_>, inset: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setDividerInsetEnd"),
            jni_sig!("(I)V"),
            &[JValue::Int(inset)],
        )
    }

    fn set_card_radius(&mut self, view: &JObject<'_>, radius: f32) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setRadius"),
            jni_sig!("(F)V"),
            &[JValue::Float(radius)],
        )
    }

    fn set_card_elevation(&mut self, view: &JObject<'_>, elevation: f32) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setCardElevation"),
            jni_sig!("(F)V"),
            &[JValue::Float(elevation)],
        )
    }

    fn set_stroke_width(&mut self, view: &JObject<'_>, width: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setStrokeWidth"),
            jni_sig!("(I)V"),
            &[JValue::Int(width)],
        )
    }

    fn set_stroke_color(&mut self, view: &JObject<'_>, colors: &JObject<'_>) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setStrokeColor"),
            jni_sig!("(Landroid/content/res/ColorStateList;)V"),
            &[JValue::Object(colors)],
        )
    }

    fn set_shape_appearance_model(
        &mut self,
        view: &JObject<'_>,
        model: &JObject<'_>,
    ) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setShapeAppearanceModel"),
            jni_sig!("(Lcom/google/android/material/shape/ShapeAppearanceModel;)V"),
            &[JValue::Object(model)],
        )
    }

    fn set_tab_mode(&mut self, view: &JObject<'_>, mode: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setTabMode"),
            jni_sig!("(I)V"),
            &[JValue::Int(mode)],
        )
    }

    fn set_selected_item_id(&mut self, view: &JObject<'_>, id: jint) -> Result<()> {
        self.call_void(
            view,
            jni_str!("setSelectedItemId"),
            jni_sig!("(I)V"),
            &[JValue::Int(id)],
        )
    }

    fn set_menu_item_icon(&mut self, item: &JObject<'_>, resource: jint) -> Result<()> {
        self.call_object(
            item,
            jni_str!("setIcon"),
            jni_sig!("(I)Landroid/view/MenuItem;"),
            &[JValue::Int(resource)],
        )?;
        Ok(())
    }

    fn set_tooltip_text(&mut self, view: &JObject<'_>, text: &str) -> Result<()> {
        let text = self.java_string(text)?;
        self.call_static_void(
            MATERIAL_TOOLTIP_COMPAT_CLASS,
            jni_str!("setTooltipText"),
            jni_sig!("(Landroid/view/View;Ljava/lang/CharSequence;)V"),
            &[JValue::Object(view), JValue::Object(text.as_ref())],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_kinds_use_the_linear_layout_class() {
        assert_eq!(
            ViewGroupKind::LinearVertical.class_name().to_str().as_ref(),
            "android/widget/LinearLayout"
        );
        assert_eq!(
            ViewGroupKind::LinearHorizontal
                .class_name()
                .to_str()
                .as_ref(),
            "android/widget/LinearLayout"
        );
    }

    #[test]
    fn scroll_kind_uses_the_scroll_view_class() {
        assert_eq!(
            ViewGroupKind::Scroll.class_name().to_str().as_ref(),
            "android/widget/ScrollView"
        );
    }
}
