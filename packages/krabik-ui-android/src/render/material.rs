//! Material Components rendering arms for [`super::Renderer`].

use jni::objects::{JObject, JValue};
use jni::strings::JNIStr;
use jni::sys::jint;
use jni::{jni_sig, jni_str};

use super::Renderer;
use crate::constants::*;
use crate::env::EnvExt;
use crate::error::Result;
use crate::ui::{
    Length, MaterialAppBar, MaterialBottomAppBar, MaterialBottomNavigation, MaterialButton,
    MaterialButtonToggleGroup, MaterialCard, MaterialCarousel, MaterialCheckBox, MaterialChip,
    MaterialChipGroup, MaterialCollapsingToolbar, MaterialDivider, MaterialDropdown,
    MaterialExtendedFab, MaterialFab, MaterialLoading, MaterialNavigation, MaterialNavigationRail,
    MaterialProgress, MaterialProgressKind, MaterialRadioGroup, MaterialSearchBar,
    MaterialSearchView, MaterialShapeableImage, MaterialSlider, MaterialSwitch, MaterialTabs,
    MaterialText, MaterialTextField, MaterialToolbar, MaterialToolbarLayout,
};
use crate::views::{self, AdapterItemSizing, LayoutParamsKind, ViewExt};

impl<'env, 'local, 'ctx> Renderer<'env, 'local, 'ctx> {
    pub(super) fn render_material_text(
        &mut self,
        component: &MaterialText,
    ) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, MATERIAL_TEXT_VIEW_CLASS, self.activity)?;
        self.env.set_text(&view, component.value)?;
        self.env.set_text_size(&view, component.size_sp)?;
        self.env.set_text_color(&view, component.color)?;
        if let Some(gravity) = component.gravity {
            self.env.set_gravity(&view, gravity)?;
        }
        self.apply_background(&view, component.background)?;
        self.apply_padding(&view, component.padding)?;
        Ok(view)
    }

    pub(super) fn render_material_button(
        &mut self,
        component: &MaterialButton,
    ) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, MATERIAL_BUTTON_CLASS, self.activity)?;
        self.env.set_text(&view, component.label)?;
        self.env.set_text_size(&view, component.size_sp)?;
        self.env.set_all_caps(&view, component.all_caps)?;
        self.env.set_enabled(&view, component.enabled)?;

        if component.checkable {
            self.env.set_checkable(&view, true)?;
            self.env.set_checked(&view, component.checked)?;
        }
        if let Some(icon) = component.icon {
            let resource = self.env.static_int(DRAWABLE_CLASS, icon)?;
            self.env.set_icon_resource(&view, resource)?;
        }
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

    pub(super) fn render_material_toggle_group(
        &mut self,
        component: &MaterialButtonToggleGroup,
    ) -> Result<JObject<'local>> {
        let group = views::new_widget(self.env, MATERIAL_BUTTON_TOGGLE_GROUP_CLASS, self.activity)?;
        self.env
            .set_single_selection(&group, component.single_selection)?;
        self.env
            .set_selection_required(&group, component.selection_required)?;

        for button in &component.buttons {
            let view = self.render_material_button(button)?;
            let id = views::new_view_id(self.env)?;
            self.env.set_id(&view, id)?;
            // The group makes every child checkable, but setting it explicitly keeps the button
            // usable when it is inspected outside the group during debugging.
            self.env.set_checkable(&view, true)?;

            let params = views::new_layout_params(
                self.env,
                LayoutParamsKind::Linear,
                WRAP_CONTENT,
                WRAP_CONTENT,
            )?;
            views::add_child(self.env, &group, &view, &params)?;

            if button.checked {
                self.env.set_checked(&view, true)?;
            }
        }

        Ok(group)
    }

    pub(super) fn render_material_check_box(
        &mut self,
        component: &MaterialCheckBox,
    ) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, MATERIAL_CHECK_BOX_CLASS, self.activity)?;
        self.env.set_text(&view, component.label)?;
        self.env.set_checked(&view, component.checked)?;
        Ok(view)
    }

    pub(super) fn render_material_radio_group(
        &mut self,
        component: &MaterialRadioGroup,
    ) -> Result<JObject<'local>> {
        let group = views::new_widget(self.env, RADIO_GROUP_CLASS, self.activity)?;

        for option in &component.options {
            let view = views::new_widget(self.env, MATERIAL_RADIO_BUTTON_CLASS, self.activity)?;
            self.env.set_text(&view, option.label)?;
            let params = views::new_layout_params(
                self.env,
                LayoutParamsKind::Linear,
                WRAP_CONTENT,
                WRAP_CONTENT,
            )?;
            views::add_child(self.env, &group, &view, &params)?;

            if option.checked {
                self.env.set_checked(&view, true)?;
            }
        }

        Ok(group)
    }

    pub(super) fn render_material_switch(
        &mut self,
        component: &MaterialSwitch,
    ) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, MATERIAL_SWITCH_CLASS, self.activity)?;
        self.env.set_text(&view, component.label)?;
        self.env.set_checked(&view, component.checked)?;
        Ok(view)
    }

    pub(super) fn render_material_text_field(
        &mut self,
        component: &MaterialTextField,
    ) -> Result<JObject<'local>> {
        let layout = views::new_widget(self.env, MATERIAL_TEXT_INPUT_LAYOUT_CLASS, self.activity)?;
        self.env.set_hint(&layout, component.hint)?;
        self.env
            .set_box_background_mode(&layout, component.box_mode.value())?;
        if let Some(helper) = component.helper_text {
            self.env.set_helper_text(&layout, helper)?;
        }
        if let Some(error) = component.error_text {
            self.env.set_error_text(&layout, error)?;
        }
        if component.password_toggle {
            self.env
                .set_end_icon_mode(&layout, MATERIAL_END_ICON_PASSWORD_TOGGLE)?;
        }

        let edit = views::new_widget(self.env, TEXT_INPUT_EDIT_TEXT_CLASS, self.activity)?;
        self.env.set_text(&edit, component.text)?;
        self.env.set_text_size(&edit, component.size_sp)?;
        if let Some(input_type) = component.input_type {
            self.env.set_input_type(&edit, input_type)?;
        }
        let params = views::new_layout_params(
            self.env,
            LayoutParamsKind::Linear,
            MATCH_PARENT,
            WRAP_CONTENT,
        )?;
        views::add_child(self.env, &layout, &edit, &params)?;

        Ok(layout)
    }

    pub(super) fn render_material_dropdown(
        &mut self,
        component: &MaterialDropdown,
    ) -> Result<JObject<'local>> {
        let layout = views::new_widget(self.env, MATERIAL_TEXT_INPUT_LAYOUT_CLASS, self.activity)?;
        self.env.set_hint(&layout, component.hint)?;
        self.env
            .set_box_background_mode(&layout, component.box_mode.value())?;
        self.env
            .set_end_icon_mode(&layout, MATERIAL_END_ICON_DROPDOWN_MENU)?;

        let edit = views::new_widget(
            self.env,
            MATERIAL_AUTO_COMPLETE_TEXT_VIEW_CLASS,
            self.activity,
        )?;
        self.env.set_text(&edit, component.text)?;
        self.env.set_text_size(&edit, component.size_sp)?;
        let adapter =
            views::new_array_adapter(self.env, self.activity, SIMPLE_LIST_ITEM_1, component.items)?;
        self.env.call_void(
            &edit,
            jni_str!("setAdapter"),
            jni_sig!("(Landroid/widget/ListAdapter;)V"),
            &[JValue::Object(&adapter)],
        )?;

        let params = views::new_layout_params(
            self.env,
            LayoutParamsKind::Linear,
            MATCH_PARENT,
            WRAP_CONTENT,
        )?;
        views::add_child(self.env, &layout, &edit, &params)?;
        Ok(layout)
    }

    pub(super) fn render_material_search_bar(
        &mut self,
        component: &MaterialSearchBar,
    ) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, MATERIAL_SEARCH_BAR_CLASS, self.activity)?;
        self.env.set_hint(&view, component.hint)?;
        self.env.set_text(&view, component.text)?;
        if !component.title.is_empty() {
            self.env.set_title(&view, component.title)?;
        }
        let icon = self
            .env
            .static_int(DRAWABLE_CLASS, component.navigation_icon)?;
        self.env.call_void(
            &view,
            jni_str!("setNavigationIcon"),
            jni_sig!("(I)V"),
            &[JValue::Int(icon)],
        )?;
        Ok(view)
    }

    pub(super) fn render_material_search_view(
        &mut self,
        component: &MaterialSearchView,
    ) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, MATERIAL_SEARCH_VIEW_CLASS, self.activity)?;
        self.env.set_hint(&view, component.hint)?;
        self.env.set_text(&view, component.text)?;
        Ok(view)
    }

    pub(super) fn render_material_chip(
        &mut self,
        component: &MaterialChip,
    ) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, MATERIAL_CHIP_CLASS, self.activity)?;
        self.env.set_text(&view, component.label)?;
        if component.checkable {
            self.env.set_checkable(&view, true)?;
            self.env.set_checked(&view, component.checked)?;
        }
        if component.closeable {
            let close = self.env.static_int(DRAWABLE_CLASS, DRAWABLE_CLOSE)?;
            self.env.set_close_icon_resource(&view, close)?;
        }
        if let Some(icon) = component.icon {
            let resource = self.env.static_int(DRAWABLE_CLASS, icon)?;
            self.env.set_chip_icon_resource(&view, resource)?;
        }
        if let Some(action) = component.action {
            self.env.set_tag(&view, action)?;
            self.env.call_void(
                &view,
                jni_str!("setOnClickListener"),
                jni_sig!("(Landroid/view/View$OnClickListener;)V"),
                &[JValue::Object(self.listener)],
            )?;
            if component.closeable {
                self.env.call_void(
                    &view,
                    jni_str!("setOnCloseIconClickListener"),
                    jni_sig!("(Landroid/view/View$OnClickListener;)V"),
                    &[JValue::Object(self.listener)],
                )?;
            }
        }
        Ok(view)
    }

    pub(super) fn render_material_chip_group(
        &mut self,
        component: &MaterialChipGroup,
    ) -> Result<JObject<'local>> {
        let group = views::new_widget(self.env, MATERIAL_CHIP_GROUP_CLASS, self.activity)?;
        self.env
            .set_single_selection(&group, component.single_selection)?;
        self.env.set_single_line(&group, component.single_line)?;

        for chip in &component.chips {
            let view = self.render_material_chip(chip)?;
            let id = views::new_view_id(self.env)?;
            self.env.set_id(&view, id)?;
            let params = views::new_layout_params(
                self.env,
                LayoutParamsKind::Frame,
                WRAP_CONTENT,
                WRAP_CONTENT,
            )?;
            views::add_child(self.env, &group, &view, &params)?;
            if chip.checked {
                self.env.set_checked(&view, true)?;
            }
        }

        Ok(group)
    }

    pub(super) fn render_material_card(
        &mut self,
        component: &MaterialCard,
    ) -> Result<JObject<'local>> {
        let card = views::new_widget(self.env, MATERIAL_CARD_VIEW_CLASS, self.activity)?;
        if let Some(background) = component.background {
            self.env.call_void(
                &card,
                jni_str!("setCardBackgroundColor"),
                jni_sig!("(I)V"),
                &[JValue::Int(background)],
            )?;
        }
        self.env
            .set_card_radius(&card, self.metrics.dp(component.radius_dp) as f32)?;
        self.env
            .set_card_elevation(&card, self.metrics.dp(component.elevation_dp) as f32)?;
        if component.stroke_width_dp > 0.0 {
            self.env
                .set_stroke_width(&card, self.metrics.dp(component.stroke_width_dp))?;
            if let Some(color) = component.stroke_color {
                let colors = views::color_state_list(self.env, color)?;
                self.env.set_stroke_color(&card, &colors)?;
            }
        }
        self.apply_padding(&card, component.padding)?;
        if let Some(action) = component.action {
            self.env.set_tag(&card, action)?;
            self.env.call_void(
                &card,
                jni_str!("setClickable"),
                jni_sig!("(Z)V"),
                &[JValue::Bool(true)],
            )?;
            self.env.call_void(
                &card,
                jni_str!("setOnClickListener"),
                jni_sig!("(Landroid/view/View$OnClickListener;)V"),
                &[JValue::Object(self.listener)],
            )?;
        }
        self.add_children(
            &card,
            &component.children,
            LayoutParamsKind::Frame,
            Length::MatchParent,
        )?;
        Ok(card)
    }

    pub(super) fn render_material_toolbar(
        &mut self,
        component: &MaterialToolbar,
    ) -> Result<JObject<'local>> {
        self.render_toolbar_view(
            MATERIAL_TOOLBAR_CLASS,
            component.title,
            component.subtitle,
            component.background,
            component.navigation_icon,
        )
    }

    pub(super) fn render_material_bottom_app_bar(
        &mut self,
        component: &MaterialBottomAppBar,
    ) -> Result<JObject<'local>> {
        self.render_toolbar_view(
            MATERIAL_BOTTOM_APP_BAR_CLASS,
            component.title,
            component.subtitle,
            component.background,
            component.navigation_icon,
        )
    }

    pub(super) fn render_material_app_bar(
        &mut self,
        component: &MaterialAppBar,
    ) -> Result<JObject<'local>> {
        let app_bar = views::new_widget(self.env, APP_BAR_LAYOUT_CLASS, self.activity)?;
        self.apply_background(&app_bar, component.background)?;
        self.add_children(
            &app_bar,
            &component.children,
            LayoutParamsKind::AppBar,
            Length::MatchParent,
        )?;
        Ok(app_bar)
    }

    pub(super) fn render_material_collapsing_toolbar(
        &mut self,
        component: &MaterialCollapsingToolbar,
    ) -> Result<JObject<'local>> {
        let toolbar = views::new_widget(self.env, COLLAPSING_TOOLBAR_LAYOUT_CLASS, self.activity)?;
        self.env.set_title(&toolbar, component.title)?;
        if !component.subtitle.is_empty() {
            self.env.set_subtitle(&toolbar, component.subtitle)?;
        }
        self.apply_background(&toolbar, component.background)?;
        self.add_children(
            &toolbar,
            &component.children,
            LayoutParamsKind::Frame,
            Length::MatchParent,
        )?;
        Ok(toolbar)
    }

    pub(super) fn render_material_toolbar_layout(
        &mut self,
        class_name: &JNIStr,
        component: &MaterialToolbarLayout,
    ) -> Result<JObject<'local>> {
        let toolbar = views::new_widget(self.env, class_name, self.activity)?;
        self.apply_background(&toolbar, component.background)?;
        let default_width = match component.orientation {
            crate::ui::Orientation::Vertical => Length::MatchParent,
            crate::ui::Orientation::Horizontal => Length::WrapContent,
        };
        self.add_children(
            &toolbar,
            &component.children,
            LayoutParamsKind::Frame,
            default_width,
        )?;
        Ok(toolbar)
    }

    pub(super) fn render_material_fab(
        &mut self,
        component: &MaterialFab,
    ) -> Result<JObject<'local>> {
        let view = views::new_widget(
            self.env,
            MATERIAL_FLOATING_ACTION_BUTTON_CLASS,
            self.activity,
        )?;
        let icon = self.env.static_int(DRAWABLE_CLASS, component.icon)?;
        self.env.call_void(
            &view,
            jni_str!("setImageResource"),
            jni_sig!("(I)V"),
            &[JValue::Int(icon)],
        )?;
        if component.size_dp > 0.0 {
            let size = self.metrics.dp(component.size_dp);
            self.env.call_void(
                &view,
                jni_str!("setCustomSize"),
                jni_sig!("(I)V"),
                &[JValue::Int(size)],
            )?;
        }
        if let Some(background) = component.background {
            let colors = views::color_state_list(self.env, background)?;
            self.env.call_void(
                &view,
                jni_str!("setBackgroundTintList"),
                jni_sig!("(Landroid/content/res/ColorStateList;)V"),
                &[JValue::Object(&colors)],
            )?;
        }
        if let Some(action) = component.action {
            self.env.set_tag(&view, action)?;
            self.env.call_void(
                &view,
                jni_str!("setOnClickListener"),
                jni_sig!("(Landroid/view/View$OnClickListener;)V"),
                &[JValue::Object(self.listener)],
            )?;
        }
        if let Some(badge) = component.badge {
            self.pending_badges
                .push((self.env.global_ref(&view)?, badge));
        }
        Ok(view)
    }

    pub(super) fn render_material_extended_fab(
        &mut self,
        component: &MaterialExtendedFab,
    ) -> Result<JObject<'local>> {
        let view = views::new_widget(
            self.env,
            MATERIAL_EXTENDED_FLOATING_ACTION_BUTTON_CLASS,
            self.activity,
        )?;
        self.env.set_text(&view, component.label)?;
        if let Some(icon) = component.icon {
            let icon = self.env.static_int(DRAWABLE_CLASS, icon)?;
            self.env.set_icon_resource(&view, icon)?;
        }
        if let Some(background) = component.background {
            let colors = views::color_state_list(self.env, background)?;
            self.env.call_void(
                &view,
                jni_str!("setBackgroundTintList"),
                jni_sig!("(Landroid/content/res/ColorStateList;)V"),
                &[JValue::Object(&colors)],
            )?;
        }
        if let Some(action) = component.action {
            self.env.set_tag(&view, action)?;
            self.env.call_void(
                &view,
                jni_str!("setOnClickListener"),
                jni_sig!("(Landroid/view/View$OnClickListener;)V"),
                &[JValue::Object(self.listener)],
            )?;
        }
        if let Some(badge) = component.badge {
            self.pending_badges
                .push((self.env.global_ref(&view)?, badge));
        }
        Ok(view)
    }

    pub(super) fn render_material_progress(
        &mut self,
        component: &MaterialProgress,
    ) -> Result<JObject<'local>> {
        let class_name = match component.kind {
            MaterialProgressKind::Linear => MATERIAL_LINEAR_PROGRESS_INDICATOR_CLASS,
            MaterialProgressKind::Circular => MATERIAL_CIRCULAR_PROGRESS_INDICATOR_CLASS,
        };
        let view = views::new_widget(self.env, class_name, self.activity)?;
        if component.indeterminate {
            self.env.set_indeterminate(&view, true)?;
        } else {
            self.env.set_max(&view, component.max)?;
            self.env.set_progress(&view, component.progress)?;
        }
        Ok(view)
    }

    pub(super) fn render_material_loading(
        &mut self,
        component: &MaterialLoading,
    ) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, MATERIAL_LOADING_INDICATOR_CLASS, self.activity)?;
        self.env.call_void(
            &view,
            jni_str!("setIndicatorSize"),
            jni_sig!("(I)V"),
            &[JValue::Int(self.metrics.dp(component.size_dp))],
        )?;
        if let Some(color) = component.container_color {
            self.env.call_void(
                &view,
                jni_str!("setContainerColor"),
                jni_sig!("(I)V"),
                &[JValue::Int(color)],
            )?;
        }
        Ok(view)
    }

    pub(super) fn render_material_slider(
        &mut self,
        component: &MaterialSlider,
    ) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, MATERIAL_SLIDER_CLASS, self.activity)?;
        self.env.set_value_from(&view, component.from)?;
        self.env.set_value_to(&view, component.to)?;
        if component.step > 0.0 {
            self.env.set_step_size(&view, component.step)?;
        }
        self.env.set_slider_value(&view, component.value)?;
        self.env.set_enabled(&view, component.enabled)?;
        Ok(view)
    }

    pub(super) fn render_material_divider(
        &mut self,
        component: &MaterialDivider,
    ) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, MATERIAL_DIVIDER_CLASS, self.activity)?;
        self.env
            .set_divider_thickness(&view, self.metrics.dp(component.thickness_dp))?;
        if let Some(color) = component.color {
            self.env.set_divider_color(&view, color)?;
        }
        if component.inset_start_dp > 0.0 {
            self.env
                .set_divider_inset_start(&view, self.metrics.dp(component.inset_start_dp))?;
        }
        if component.inset_end_dp > 0.0 {
            self.env
                .set_divider_inset_end(&view, self.metrics.dp(component.inset_end_dp))?;
        }
        Ok(view)
    }

    pub(super) fn render_material_shapeable_image(
        &mut self,
        component: &MaterialShapeableImage,
    ) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, SHAPEABLE_IMAGE_VIEW_CLASS, self.activity)?;
        let resource = self.env.static_int(DRAWABLE_CLASS, component.drawable)?;
        self.env.call_void(
            &view,
            jni_str!("setImageResource"),
            jni_sig!("(I)V"),
            &[JValue::Int(resource)],
        )?;
        let radius = self.metrics.dp(component.corner_radius_dp) as f32;
        let model = views::rounded_shape_model(self.env, radius)?;
        self.env.set_shape_appearance_model(&view, &model)?;
        self.apply_background(&view, component.background)?;
        Ok(view)
    }

    pub(super) fn render_material_tabs(
        &mut self,
        component: &MaterialTabs,
    ) -> Result<JObject<'local>> {
        let tabs = views::new_widget(self.env, MATERIAL_TAB_LAYOUT_CLASS, self.activity)?;
        let mode = if component.scrollable {
            MATERIAL_TAB_MODE_SCROLLABLE
        } else {
            MATERIAL_TAB_MODE_FIXED
        };
        self.env.set_tab_mode(&tabs, mode)?;

        for (index, label) in component.tabs.iter().enumerate() {
            let tab = self.env.call_object(
                &tabs,
                jni_str!("newTab"),
                jni_sig!("()Lcom/google/android/material/tabs/TabLayout$Tab;"),
                &[],
            )?;
            let text = self.env.java_string(label)?;
            self.env.call_object(
                &tab,
                jni_str!("setText"),
                jni_sig!(
                    "(Ljava/lang/CharSequence;)Lcom/google/android/material/tabs/TabLayout$Tab;"
                ),
                &[JValue::Object(text.as_ref())],
            )?;
            self.env.call_void(
                &tabs,
                jni_str!("addTab"),
                jni_sig!("(Lcom/google/android/material/tabs/TabLayout$Tab;IZ)V"),
                &[
                    JValue::Object(&tab),
                    JValue::Int(index as jint),
                    JValue::Bool(index as jint == component.selected),
                ],
            )?;
        }
        Ok(tabs)
    }

    fn render_navigation_items(
        &mut self,
        view: &JObject<'_>,
        items: &[(&str, &JNIStr)],
        selected: jint,
    ) -> Result<()> {
        let menu = self.env.call_object(
            view,
            jni_str!("getMenu"),
            jni_sig!("()Landroid/view/Menu;"),
            &[],
        )?;

        let mut selected_id = None;
        for (index, (label, icon)) in items.iter().enumerate() {
            let id = views::new_view_id(self.env)?;
            crate::material::add_menu_item(self.env, &menu, id, label, Some(icon))?;
            if index as jint == selected {
                selected_id = Some(id);
            }
        }

        if let Some(id) = selected_id {
            self.env.set_selected_item_id(view, id)?;
        }
        Ok(())
    }

    pub(super) fn render_material_bottom_navigation(
        &mut self,
        component: &MaterialBottomNavigation,
    ) -> Result<JObject<'local>> {
        let view = views::new_widget(
            self.env,
            MATERIAL_BOTTOM_NAVIGATION_VIEW_CLASS,
            self.activity,
        )?;
        self.render_navigation_items(&view, component.items, component.selected)?;
        Ok(view)
    }

    pub(super) fn render_material_navigation_rail(
        &mut self,
        component: &MaterialNavigationRail,
    ) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, MATERIAL_NAVIGATION_RAIL_VIEW_CLASS, self.activity)?;
        self.render_navigation_items(&view, component.items, component.selected)?;
        Ok(view)
    }

    pub(super) fn render_material_navigation(
        &mut self,
        component: &MaterialNavigation,
    ) -> Result<JObject<'local>> {
        let view = views::new_widget(self.env, MATERIAL_NAVIGATION_VIEW_CLASS, self.activity)?;
        let menu = self.env.call_object(
            &view,
            jni_str!("getMenu"),
            jni_sig!("()Landroid/view/Menu;"),
            &[],
        )?;

        let mut selected_id = None;
        for (index, label) in component.items.iter().enumerate() {
            let id = views::new_view_id(self.env)?;
            crate::material::add_menu_item(self.env, &menu, id, label, Some(DRAWABLE_MENU_REVERT))?;
            if index as jint == component.selected {
                selected_id = Some(id);
            }
        }
        if let Some(id) = selected_id {
            self.env.call_void(
                &view,
                jni_str!("setCheckedItem"),
                jni_sig!("(I)V"),
                &[JValue::Int(id)],
            )?;
        }
        Ok(view)
    }

    pub(super) fn render_material_carousel(
        &mut self,
        component: &MaterialCarousel,
    ) -> Result<JObject<'local>> {
        let view = views::new_recycler_view(self.env, self.activity)?;
        let orientation = match component.orientation {
            crate::ui::Orientation::Vertical => LINEAR_LAYOUT_VERTICAL,
            crate::ui::Orientation::Horizontal => LINEAR_LAYOUT_HORIZONTAL,
        };
        let manager = views::new_carousel_layout_manager(self.env, orientation)?;
        self.env.set_layout_manager(&view, &manager)?;
        let adapter = self.create_item_adapter(component.items, AdapterItemSizing::Carousel)?;
        self.env.set_recycler_adapter(&view, &adapter)?;
        self.env
            .set_has_fixed_size(&view, component.has_fixed_size)?;
        Ok(view)
    }
}
