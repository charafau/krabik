//! The `container` widget on Android.

use crate::android::KRABIK_VIEWS_CLASS;
use crate::android::handles::{
    Method, StaticMethod, VOID, class, get as jni_handles, method, static_method,
};
use crate::android::widgets::Site;
use crate::ui::ContainerProps;
use jni::errors::Result as JniResult;
use jni::objects::{JClass, JObject};
use jni::refs::Global;
use jni::signature::RuntimeMethodSignature;
use jni::sys::jint;
use jni::{Env, JValue};
use jni::{jni_sig, jni_str};

/// Creates the native view of a container: a frame its children are placed
/// in.
///
/// Like a stock button it has no node of its own to report to, so the root
/// view is its click listener and finds the node through the view's tag.
/// Whether a tap is accepted at all is decided by [`update_container`].
pub(crate) fn create<'local>(env: &mut Env<'local>, site: &Site<'_>) -> JniResult<JObject<'local>> {
    let ids = jni_handles(env)?;
    let view = site.stock(env, &ids.frame_layout, &ids.frame_layout_ctor)?;
    ids.set_on_click_listener
        .call_void(env, &view, &[JValue::Object(site.root)])?;
    ids.container
        .set_on_long_click_listener
        .call_void(env, &view, &[JValue::Object(site.root)])?;
    let_children_overflow(env, &view)?;
    Ok(view)
}

/// Lets the content of a container's children show outside their bounds;
/// see `KrabikViews.letChildrenOverflow`.
pub(crate) fn let_children_overflow(env: &mut Env<'_>, view: &JObject<'_>) -> JniResult<()> {
    let ids = jni_handles(env)?;
    ids.container
        .let_children_overflow
        .call_void(env, &ids.krabik_views, &[JValue::Object(view)])
}

/// Makes the container react to taps and long presses while it has a
/// handler for them.
pub(crate) fn update_container(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &ContainerProps,
) -> JniResult<()> {
    // A menu makes the container react to the gesture that opens it.
    let opens_on = |long_press: bool| {
        props
            .menu
            .as_ref()
            .is_some_and(|menu| menu.on_long_press == long_press)
    };
    let ids = jni_handles(env)?;
    ids.container.set_tappable.call_void(
        env,
        &ids.krabik_views,
        &[
            JValue::Object(view),
            JValue::Bool(props.on_tap.is_set() || opens_on(false)),
            JValue::Bool(props.on_long_press.is_set() || opens_on(true)),
        ],
    )?;
    let Some(menu) = &props.menu else {
        return ids.container.clear_menu.call_void(
            env,
            &ids.container.menus,
            &[JValue::Object(view)],
        );
    };
    let labels = string_array(
        env,
        menu.items.iter().map(|item| item.label.as_str()).collect(),
    )?;
    let icons = string_array(
        env,
        menu.items
            .iter()
            .map(|item| {
                item.icon
                    .as_ref()
                    .and_then(|icon| icon.android_name())
                    .unwrap_or_default()
            })
            .collect(),
    )?;
    let flags: Vec<bool> = menu.items.iter().map(|item| item.destructive).collect();
    let destructive = env.new_boolean_array(flags.len())?;
    destructive.set_region(env, 0, &flags)?;
    ids.container.set_menu.call_void(
        env,
        &ids.container.menus,
        &[
            JValue::Object(view),
            JValue::Object(&labels),
            JValue::Object(&icons),
            JValue::Object(&destructive),
            JValue::Int(menu.selected.map_or(-1, |index| index as jint)),
            JValue::Bool(menu.on_long_press),
        ],
    )
}

/// A Java `String[]` of the given values.
pub(crate) fn string_array<'local>(
    env: &mut Env<'local>,
    values: Vec<&str>,
) -> JniResult<jni::objects::JObjectArray<'local>> {
    let array = env.new_object_array(
        values.len() as jint,
        jni_str!("java/lang/String"),
        JObject::null(),
    )?;
    for (index, value) in values.into_iter().enumerate() {
        let value = jcall!(env, env.new_string(value))?;
        let object: &JObject<'_> = value.as_ref();
        array.set_element(env, index, object)?;
    }
    Ok(array)
}

/// The JNI handles this module uses, resolved once; see [`crate::android::handles`].
pub(crate) struct Handles {
    pub(crate) set_tappable: StaticMethod,
    pub(crate) set_text_is_selectable: Method,
    pub(crate) set_tooltip: StaticMethod,
    pub(crate) menus: Global<JClass<'static>>,
    pub(crate) set_menu: StaticMethod,
    pub(crate) clear_menu: StaticMethod,
    pub(crate) let_children_overflow: StaticMethod,
    pub(crate) set_on_long_click_listener: Method,
}

impl Handles {
    pub(crate) fn load(
        env: &mut Env<'_>,
        _widget_signature: &RuntimeMethodSignature,
    ) -> JniResult<Self> {
        let krabik_views = class(env, KRABIK_VIEWS_CLASS)?;
        let view = class(env, crate::android::VIEW_CLASS)?;
        let menus = class(env, crate::android::KRABIK_MENUS_CLASS)?;
        Ok(Self {
            set_tappable: static_method!(
                env,
                &krabik_views,
                "setTappable",
                "(Landroid/view/View;ZZ)V",
                VOID
            ),
            set_menu: static_method!(
                env,
                &menus,
                "set",
                "(Landroid/view/View;[Ljava/lang/String;[Ljava/lang/String;[ZIZ)V",
                VOID
            ),
            clear_menu: static_method!(env, &menus, "clear", "(Landroid/view/View;)V", VOID),
            menus,
            set_text_is_selectable: {
                let text_view = class(env, crate::android::TEXT_VIEW_CLASS)?;
                method!(env, &text_view, "setTextIsSelectable", "(Z)V", VOID)
            },
            set_tooltip: static_method!(
                env,
                &krabik_views,
                "setTooltip",
                "(Landroid/view/View;Ljava/lang/String;)V",
                VOID
            ),
            let_children_overflow: static_method!(
                env,
                &krabik_views,
                "letChildrenOverflow",
                "(Landroid/view/View;)V",
                VOID
            ),
            set_on_long_click_listener: method!(
                env,
                &view,
                "setOnLongClickListener",
                "(Landroid/view/View$OnLongClickListener;)V",
                VOID
            ),
        })
    }
}
