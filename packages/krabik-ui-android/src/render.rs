//! Translates an [`Element`] tree into native Android views.

mod material;

use jni::objects::{JObject, JValue};
use jni::refs::Global;
use jni::strings::JNIStr;
use jni::sys::jint;
use jni::{Env, jni_sig, jni_str};

use super::constants::*;
use super::env::EnvExt;
use super::error::Result;
use super::ui::{EdgeInsets, Element, Length, Metrics, Orientation, Placed, components::*};
use super::views::{
    self, AdapterItemSizing, ItemAdapterFactory, LayoutParamsKind, ViewExt, ViewGroupKind,
};

pub struct Renderer<'env, 'local, 'ctx> {
    env: &'env mut Env<'local>,
    activity: &'env JObject<'ctx>,
    listener: &'env JObject<'ctx>,
    metrics: Metrics,
    status: Option<Global<JObject<'static>>>,
    item_adapter_factory: Option<&'env dyn ItemAdapterFactory>,
    /// Badges can only be attached once their anchor has a parent, so Material components queue
    /// them here and the renderer flushes the queue after each subtree is attached.
    pending_badges: Vec<(Global<JObject<'static>>, jint)>,
}

impl<'env, 'local, 'ctx> Renderer<'env, 'local, 'ctx> {
    pub fn new(
        env: &'env mut Env<'local>,
        activity: &'env JObject<'ctx>,
        listener: &'env JObject<'ctx>,
        metrics: Metrics,
    ) -> Self {
        Self {
            env,
            activity,
            listener,
            metrics,
            status: None,
            item_adapter_factory: None,
            pending_badges: Vec::new(),
        }
    }

    /// Registers the factory that supplies `RecyclerView.Adapter` instances for
    /// [`Element::Recycler`] and [`Element::ViewPager`].
    pub fn with_item_adapter_factory(mut self, factory: &'env dyn ItemAdapterFactory) -> Self {
        self.item_adapter_factory = Some(factory);
        self
    }

    /// Returns the view rendered for [`Element::Status`], if the tree contained one.
    pub fn take_status(&mut self) -> Option<Global<JObject<'static>>> {
        self.status.take()
    }

    pub fn render(&mut self, element: &Element) -> Result<JObject<'local>> {
        let view = match element {
            Element::Scroll(component) => self.render_scroll(component),
            Element::Linear(component) => self.render_linear(component),
            Element::Recycler(component) => self.render_recycler(component),
            Element::ViewPager(component) => self.render_view_pager(component),
            Element::RadioGroup(component) => self.render_radio_group(component),
            Element::Text(component) => self.render_text(component),
            Element::Button(component) => self.render_button(component),
            Element::Image(component) => self.render_image(component),
            Element::ImageButton(component) => self.render_image_button(component),
            Element::EditText(component) => self.render_edit_text(component),
            Element::CheckBox(component) => self.render_check_box(component),
            Element::Switch(component) => self.render_switch(component),
            Element::ToggleButton(component) => self.render_toggle_button(component),
            Element::CheckedText(component) => self.render_checked_text(component),
            Element::Progress(component) => self.render_progress(component),
            Element::Seek(component) => self.render_seek(component),
            Element::Rating(component) => self.render_rating(component),
            Element::Spinner(component) => self.render_spinner(component),
            Element::AutoComplete(component) => self.render_auto_complete(component),
            Element::MultiAutoComplete(component) => self.render_multi_auto_complete(component),
            Element::NumberPicker(component) => self.render_number_picker(component),
            Element::DatePicker => views::new_widget(self.env, DATE_PICKER_CLASS, self.activity),
            Element::TimePicker => views::new_widget(self.env, TIME_PICKER_CLASS, self.activity),
            Element::Chronometer => self.render_chronometer(),
            Element::TextClock(component) => self.render_text_clock(component),
            Element::Calendar => views::new_widget(self.env, CALENDAR_VIEW_CLASS, self.activity),
            Element::AnalogClock => views::new_widget(self.env, ANALOG_CLOCK_CLASS, self.activity),
            Element::Search(component) => self.render_search(component),
            Element::Toolbar(component) => self.render_toolbar(component),
            Element::AppCompatToolbar(component) => self.render_app_compat_toolbar(component),
            Element::MaterialText(component) => self.render_material_text(component),
            Element::MaterialButton(component) => self.render_material_button(component),
            Element::MaterialToggleGroup(component) => self.render_material_toggle_group(component),
            Element::MaterialCheckBox(component) => self.render_material_check_box(component),
            Element::MaterialRadioGroup(component) => self.render_material_radio_group(component),
            Element::MaterialSwitch(component) => self.render_material_switch(component),
            Element::MaterialTextField(component) => self.render_material_text_field(component),
            Element::MaterialDropdown(component) => self.render_material_dropdown(component),
            Element::MaterialSearchBar(component) => self.render_material_search_bar(component),
            Element::MaterialSearchView(component) => self.render_material_search_view(component),
            Element::MaterialChip(component) => self.render_material_chip(component),
            Element::MaterialChipGroup(component) => self.render_material_chip_group(component),
            Element::MaterialCard(component) => self.render_material_card(component),
            Element::MaterialToolbar(component) => self.render_material_toolbar(component),
            Element::MaterialAppBar(component) => self.render_material_app_bar(component),
            Element::MaterialCollapsingToolbar(component) => {
                self.render_material_collapsing_toolbar(component)
            }
            Element::MaterialBottomAppBar(component) => {
                self.render_material_bottom_app_bar(component)
            }
            Element::MaterialFloatingToolbar(component) => self
                .render_material_toolbar_layout(MATERIAL_FLOATING_TOOLBAR_LAYOUT_CLASS, component),
            Element::MaterialDockedToolbar(component) => {
                self.render_material_toolbar_layout(MATERIAL_DOCKED_TOOLBAR_LAYOUT_CLASS, component)
            }
            Element::MaterialFab(component) => self.render_material_fab(component),
            Element::MaterialExtendedFab(component) => self.render_material_extended_fab(component),
            Element::MaterialProgress(component) => self.render_material_progress(component),
            Element::MaterialLoading(component) => self.render_material_loading(component),
            Element::MaterialSlider(component) => self.render_material_slider(component),
            Element::MaterialDivider(component) => self.render_material_divider(component),
            Element::MaterialShapeableImage(component) => {
                self.render_material_shapeable_image(component)
            }
            Element::MaterialTabs(component) => self.render_material_tabs(component),
            Element::MaterialBottomNavigation(component) => {
                self.render_material_bottom_navigation(component)
            }
            Element::MaterialNavigationRail(component) => {
                self.render_material_navigation_rail(component)
            }
            Element::MaterialNavigation(component) => self.render_material_navigation(component),
            Element::MaterialCarousel(component) => self.render_material_carousel(component),
            Element::Status(component) => self.render_status(component),
        }?;

        self.flush_pending_badges()?;
        Ok(view)
    }

    /// Attaches queued badges whose anchor has been added to the live hierarchy.
    ///
    /// `BadgeUtils.attachBadgeDrawable` reads the anchor's parent, so it cannot run until the
    /// renderer has added the anchor with `ViewGroup.addView`. Badges whose anchor is still
    /// unattached stay queued for a later pass.
    fn flush_pending_badges(&mut self) -> Result<()> {
        let mut index = 0;
        while index < self.pending_badges.len() {
            let has_parent = {
                let (anchor, _) = &self.pending_badges[index];
                let parent = self.env.call_object(
                    anchor.as_obj(),
                    jni_str!("getParent"),
                    jni_sig!("()Landroid/view/ViewParent;"),
                    &[],
                )?;
                !parent.is_null()
            };

            if has_parent {
                let (anchor, number) = self.pending_badges.remove(index);
                crate::material::attach_badge(self.env, self.activity, anchor.as_obj(), number)?;
            } else {
                index += 1;
            }
        }

        Ok(())
    }

    fn resolve(&self, explicit: Option<Length>, default: Length) -> jint {
        explicit.unwrap_or(default).resolve(self.metrics)
    }

    fn apply_padding(&mut self, view: &JObject<'_>, padding: EdgeInsets) -> Result<()> {
        self.env.set_padding(view, padding.resolve(self.metrics))
    }

    fn apply_background(&mut self, view: &JObject<'_>, background: Option<jint>) -> Result<()> {
        match background {
            Some(color) => self.env.set_background_color(view, color),
            None => Ok(()),
        }
    }

    /// Renders and attaches `children`, using `default_width` for children that do not set one.
    fn add_children(
        &mut self,
        parent: &JObject<'_>,
        children: &[Placed],
        params_kind: LayoutParamsKind,
        default_width: Length,
    ) -> Result<()> {
        for child in children {
            let view = self.render(&child.element)?;
            let width = self.resolve(child.width, default_width);
            let height = self.resolve(child.height, Length::WrapContent);
            let params = views::new_layout_params(self.env, params_kind, width, height)?;
            views::set_margins(self.env, &params, child.margins.resolve(self.metrics))?;
            views::add_child(self.env, parent, &view, &params)?;
        }

        Ok(())
    }

    fn render_scroll(&mut self, component: &Scroll) -> Result<JObject<'local>> {
        let scroll = views::new_view_group(self.env, ViewGroupKind::Scroll, self.activity)?;
        self.apply_background(&scroll, component.background)?;
        self.add_children(
            &scroll,
            std::slice::from_ref(&component.child),
            LayoutParamsKind::Frame,
            Length::MatchParent,
        )?;
        Ok(scroll)
    }

    fn render_linear(&mut self, component: &Linear) -> Result<JObject<'local>> {
        let (kind, default_width) = match component.orientation {
            Orientation::Vertical => (ViewGroupKind::LinearVertical, Length::MatchParent),
            Orientation::Horizontal => (ViewGroupKind::LinearHorizontal, Length::WrapContent),
        };

        let layout = views::new_view_group(self.env, kind, self.activity)?;
        self.apply_background(&layout, component.background)?;
        self.apply_padding(&layout, component.padding)?;
        self.add_children(
            &layout,
            &component.children,
            LayoutParamsKind::Linear,
            default_width,
        )?;
        Ok(layout)
    }

    fn create_item_adapter(
        &mut self,
        items: &[&str],
        sizing: AdapterItemSizing,
    ) -> Result<JObject<'local>> {
        let factory = self.item_adapter_factory.ok_or_else(|| {
            super::error::Error::message(
                "Recycler/ViewPager rendering requires Renderer::with_item_adapter_factory",
            )
        })?;
        factory.create_adapter(self.env, self.activity, items, sizing)
    }

    fn render_recycler(&mut self, component: &Recycler) -> Result<JObject<'local>> {
        let view = views::new_recycler_view(self.env, self.activity)?;
        let orientation = match component.orientation {
            Orientation::Vertical => LINEAR_LAYOUT_VERTICAL,
            Orientation::Horizontal => LINEAR_LAYOUT_HORIZONTAL,
        };

        let manager = match component.span_count {
            Some(span_count) => {
                views::new_grid_layout_manager(self.env, self.activity, span_count, orientation)?
            }
            None => views::new_linear_layout_manager(self.env, self.activity, orientation)?,
        };

        self.env.set_layout_manager(&view, &manager)?;
        let adapter = self.create_item_adapter(component.items, AdapterItemSizing::WrapContent)?;
        self.env.set_recycler_adapter(&view, &adapter)?;
        self.env
            .set_has_fixed_size(&view, component.has_fixed_size)?;
        Ok(view)
    }

    fn render_view_pager(&mut self, component: &ViewPager) -> Result<JObject<'local>> {
        let view = views::new_view_pager(self.env, self.activity)?;
        let orientation = match component.orientation {
            Orientation::Vertical => LINEAR_LAYOUT_VERTICAL,
            Orientation::Horizontal => LINEAR_LAYOUT_HORIZONTAL,
        };

        self.env.set_orientation(&view, orientation)?;
        let adapter = self.create_item_adapter(component.items, AdapterItemSizing::FillParent)?;
        self.env.set_recycler_adapter(&view, &adapter)?;

        if let Some(limit) = component.offscreen_page_limit {
            self.env.set_offscreen_page_limit(&view, limit)?;
        }
        if component.current_item != 0 {
            self.env.set_current_item(&view, component.current_item)?;
        }
        self.env
            .set_user_input_enabled(&view, component.user_input_enabled)?;
        Ok(view)
    }

    fn render_radio_group(&mut self, component: &RadioGroup) -> Result<JObject<'local>> {
        let group = views::new_widget(self.env, RADIO_GROUP_CLASS, self.activity)?;

        for option in &component.options {
            let view = self.render_radio_button(option)?;
            let params = views::new_layout_params(
                self.env,
                LayoutParamsKind::Linear,
                WRAP_CONTENT,
                WRAP_CONTENT,
            )?;
            views::add_child(self.env, &group, &view, &params)?;

            // Set the checked state after the button is attached so RadioGroup's hierarchy
            // listener can track the checked id and keep the options mutually exclusive.
            if option.checked {
                self.env.set_checked(&view, true)?;
            }
        }

        Ok(group)
    }

    fn render_text(&mut self, component: &Text) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, TEXT_VIEW_CLASS, self.activity)?;
        self.configure_text(&view, component)?;
        Ok(view)
    }

    fn configure_text(&mut self, view: &JObject<'_>, component: &Text) -> Result<()> {
        self.env.set_text(view, component.value)?;
        self.env.set_text_size(view, component.size_sp)?;
        self.env.set_text_color(view, component.color)?;

        if let Some(gravity) = component.gravity {
            self.env.set_gravity(view, gravity)?;
        }
        if let Some(background) = component.background {
            self.env.set_background_color(view, background)?;
        }

        self.apply_padding(view, component.padding)
    }

    fn render_button(&mut self, component: &Button) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, BUTTON_CLASS, self.activity)?;
        self.env.set_text(&view, component.value)?;
        self.env.set_text_size(&view, component.size_sp)?;
        self.env.set_all_caps(&view, component.all_caps)?;
        self.env.set_enabled(&view, component.enabled)?;

        if let Some(action) = component.action {
            self.env.set_tag(&view, action)?;
        }

        self.env.call_void(
            &view,
            jni_str!("setOnClickListener"),
            jni_sig!("(Landroid/view/View$OnClickListener;)V"),
            &[JValue::Object(self.listener)],
        )?;

        Ok(view)
    }

    fn render_image_like(
        &mut self,
        class_name: &JNIStr,
        drawable: &JNIStr,
        background: Option<jint>,
    ) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, class_name, self.activity)?;
        let resource = self.env.static_int(DRAWABLE_CLASS, drawable)?;
        self.env.call_void(
            &view,
            jni_str!("setImageResource"),
            jni_sig!("(I)V"),
            &[JValue::Int(resource)],
        )?;
        self.apply_background(&view, background)?;
        Ok(view)
    }

    fn render_image(&mut self, component: &Image) -> Result<JObject<'local>> {
        self.render_image_like(IMAGE_VIEW_CLASS, component.drawable, component.background)
    }

    fn render_image_button(&mut self, component: &ImageButton) -> Result<JObject<'local>> {
        self.render_image_like(IMAGE_BUTTON_CLASS, component.drawable, component.background)
    }

    fn render_edit_text(&mut self, component: &EditText) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, EDIT_TEXT_CLASS, self.activity)?;
        self.env.set_hint(&view, component.hint)?;
        self.env.set_text(&view, component.text)?;
        self.env.set_text_size(&view, component.size_sp)?;

        if let Some(input_type) = component.input_type {
            self.env.set_input_type(&view, input_type)?;
        }

        self.apply_background(&view, component.background)?;
        self.apply_padding(&view, component.padding)?;
        Ok(view)
    }

    fn render_check_box(&mut self, component: &CheckBox) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, CHECK_BOX_CLASS, self.activity)?;
        self.env.set_text(&view, component.label)?;
        self.env.set_checked(&view, component.checked)?;
        Ok(view)
    }

    fn render_radio_button(&mut self, component: &RadioButton) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, RADIO_BUTTON_CLASS, self.activity)?;
        self.env.set_text(&view, component.label)?;
        Ok(view)
    }

    fn render_switch(&mut self, component: &Switch) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, SWITCH_CLASS, self.activity)?;
        self.env.set_text(&view, component.label)?;
        self.env.set_checked(&view, component.checked)?;
        Ok(view)
    }

    fn render_toggle_button(&mut self, component: &ToggleButton) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, TOGGLE_BUTTON_CLASS, self.activity)?;
        self.env.set_text_on(&view, component.text_on)?;
        self.env.set_text_off(&view, component.text_off)?;
        self.env.set_text_size(&view, 14.0)?;
        self.env.set_checked(&view, component.checked)?;
        Ok(view)
    }

    fn render_checked_text(&mut self, component: &CheckedText) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, CHECKED_TEXT_VIEW_CLASS, self.activity)?;
        self.env.set_text(&view, component.label)?;
        self.env.set_text_size(&view, 14.0)?;

        let resource = self.env.static_int(DRAWABLE_CLASS, DRAWABLE_STAR_ON)?;
        self.env.call_void(
            &view,
            jni_str!("setCheckMarkDrawable"),
            jni_sig!("(I)V"),
            &[JValue::Int(resource)],
        )?;

        self.env.set_checked(&view, component.checked)?;
        Ok(view)
    }

    fn render_progress(&mut self, component: &Progress) -> Result<JObject<'local>> {
        let view = match component.kind {
            ProgressKind::Horizontal => views::new_progress_bar(self.env, self.activity)?,
            ProgressKind::Spinner => {
                views::new_widget(self.env, PROGRESS_BAR_CLASS, self.activity)?
            }
        };

        if matches!(component.kind, ProgressKind::Horizontal) {
            self.env.set_max(&view, component.max)?;
            self.env.set_progress(&view, component.progress)?;
        }

        Ok(view)
    }

    fn render_seek(&mut self, component: &Seek) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, SEEK_BAR_CLASS, self.activity)?;
        self.env.set_max(&view, component.max)?;
        self.env.set_progress(&view, component.progress)?;
        Ok(view)
    }

    fn render_rating(&mut self, component: &Rating) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, RATING_BAR_CLASS, self.activity)?;
        self.env.set_num_stars(&view, component.num_stars)?;
        self.env.set_rating(&view, component.rating)?;
        self.env.set_step_size(&view, component.step_size)?;
        self.env.set_is_indicator(&view, component.indicator)?;
        Ok(view)
    }

    fn render_spinner(&mut self, component: &Spinner) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, SPINNER_CLASS, self.activity)?;
        let adapter = views::new_array_adapter(
            self.env,
            self.activity,
            SIMPLE_SPINNER_ITEM,
            component.items,
        )?;

        let dropdown = self
            .env
            .static_int(LAYOUT_CLASS, SIMPLE_SPINNER_DROPDOWN_ITEM)?;
        self.env.call_void(
            &adapter,
            jni_str!("setDropDownViewResource"),
            jni_sig!("(I)V"),
            &[JValue::Int(dropdown)],
        )?;

        self.env.call_void(
            &view,
            jni_str!("setAdapter"),
            jni_sig!("(Landroid/widget/SpinnerAdapter;)V"),
            &[JValue::Object(&adapter)],
        )?;
        self.env.call_void(
            &view,
            jni_str!("setSelection"),
            jni_sig!("(I)V"),
            &[JValue::Int(component.selected)],
        )?;
        Ok(view)
    }

    fn configure_auto_complete(
        &mut self,
        view: &JObject<'_>,
        hint: &str,
        items: &[&str],
        threshold: jint,
    ) -> Result<()> {
        self.env.set_hint(view, hint)?;
        self.env.set_text_size(view, 15.0)?;
        self.env.set_threshold(view, threshold)?;

        let adapter = views::new_array_adapter(self.env, self.activity, SIMPLE_LIST_ITEM_1, items)?;
        self.env.call_void(
            view,
            jni_str!("setAdapter"),
            jni_sig!("(Landroid/widget/ListAdapter;)V"),
            &[JValue::Object(&adapter)],
        )
    }

    fn render_auto_complete(&mut self, component: &AutoComplete) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, AUTO_COMPLETE_TEXT_VIEW_CLASS, self.activity)?;
        self.configure_auto_complete(&view, component.hint, component.items, component.threshold)?;
        Ok(view)
    }

    fn render_multi_auto_complete(
        &mut self,
        component: &MultiAutoComplete,
    ) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, MULTI_AUTO_COMPLETE_TEXT_VIEW_CLASS, self.activity)?;
        self.configure_auto_complete(&view, component.hint, component.items, component.threshold)?;

        let tokenizer = views::new_comma_tokenizer(self.env)?;
        self.env.call_void(
            &view,
            jni_str!("setTokenizer"),
            jni_sig!("(Landroid/widget/MultiAutoCompleteTextView$Tokenizer;)V"),
            &[JValue::Object(&tokenizer)],
        )?;

        Ok(view)
    }

    fn render_number_picker(&mut self, component: &NumberPicker) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, NUMBER_PICKER_CLASS, self.activity)?;
        self.env.set_min_value(&view, component.min)?;
        self.env.set_max_value(&view, component.max)?;
        self.env.set_value(&view, component.value)?;
        Ok(view)
    }

    fn render_chronometer(&mut self) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, CHRONOMETER_CLASS, self.activity)?;
        let base = self.env.call_static_long(
            SYSTEM_CLOCK_CLASS,
            jni_str!("elapsedRealtime"),
            jni_sig!("()J"),
            &[],
        )?;

        self.env.call_void(
            &view,
            jni_str!("setBase"),
            jni_sig!("(J)V"),
            &[JValue::Long(base)],
        )?;
        self.env
            .call_void(&view, jni_str!("start"), jni_sig!("()V"), &[])?;
        Ok(view)
    }

    fn render_text_clock(&mut self, component: &TextClock) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, TEXT_CLOCK_CLASS, self.activity)?;
        self.env.set_text_size(&view, component.size_sp)?;
        Ok(view)
    }

    fn render_search(&mut self, component: &Search) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, SEARCH_VIEW_CLASS, self.activity)?;
        self.env.call_void(
            &view,
            jni_str!("setIconifiedByDefault"),
            jni_sig!("(Z)V"),
            &[JValue::Bool(false)],
        )?;
        self.env.set_query_hint(&view, component.hint)?;
        self.env.call_void(
            &view,
            jni_str!("setSubmitButtonEnabled"),
            jni_sig!("(Z)V"),
            &[JValue::Bool(true)],
        )?;
        Ok(view)
    }

    fn render_toolbar(&mut self, component: &Toolbar) -> Result<JObject<'local>> {
        self.render_toolbar_view(
            TOOLBAR_CLASS,
            component.title,
            component.subtitle,
            component.background,
            component.navigation_icon,
        )
    }

    fn render_app_compat_toolbar(
        &mut self,
        component: &AppCompatToolbar,
    ) -> Result<JObject<'local>> {
        self.render_toolbar_view(
            APP_COMPAT_TOOLBAR_CLASS,
            component.title,
            component.subtitle,
            component.background,
            component.navigation_icon,
        )
    }

    fn render_toolbar_view(
        &mut self,
        class_name: &JNIStr,
        title: &str,
        subtitle: &str,
        background: jint,
        navigation_icon: &JNIStr,
    ) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, class_name, self.activity)?;
        self.env.set_title(&view, title)?;
        self.env.set_subtitle(&view, subtitle)?;

        self.env.call_void(
            &view,
            jni_str!("setTitleTextColor"),
            jni_sig!("(I)V"),
            &[JValue::Int(COLOR_TEXT_PRIMARY)],
        )?;
        self.env.call_void(
            &view,
            jni_str!("setSubtitleTextColor"),
            jni_sig!("(I)V"),
            &[JValue::Int(COLOR_TEXT_SECONDARY)],
        )?;

        self.apply_background(&view, Some(background))?;

        let icon = self.env.static_int(DRAWABLE_CLASS, navigation_icon)?;
        self.env.call_void(
            &view,
            jni_str!("setNavigationIcon"),
            jni_sig!("(I)V"),
            &[JValue::Int(icon)],
        )?;

        let description = self.env.java_string("Navigation")?;
        self.env.call_void(
            &view,
            jni_str!("setNavigationContentDescription"),
            jni_sig!("(Ljava/lang/CharSequence;)V"),
            &[JValue::Object(description.as_ref())],
        )?;

        // Add real MenuItems so the toolbar shows its overflow button. The items are display-only
        // for this demo; navigation remains tag-driven.
        let menu = self.env.call_object(
            &view,
            jni_str!("getMenu"),
            jni_sig!("()Landroid/view/Menu;"),
            &[],
        )?;

        for label in ["Refresh", "Settings"] {
            let label = self.env.java_string(label)?;
            self.env.call_object(
                &menu,
                jni_str!("add"),
                jni_sig!("(Ljava/lang/CharSequence;)Landroid/view/MenuItem;"),
                &[JValue::Object(label.as_ref())],
            )?;
        }

        Ok(view)
    }

    fn render_status(&mut self, component: &Text) -> Result<JObject<'local>> {
        let view = self.render_text(component)?;
        self.status = Some(self.env.global_ref(&view)?);
        Ok(view)
    }
}
