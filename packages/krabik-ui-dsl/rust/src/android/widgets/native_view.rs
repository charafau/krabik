//! The `native_view` widget on Android.

use crate::android::NATIVE_VIEWS_CLASS;
use crate::android::handles::{
    OBJECT, StaticMethod, VOID, class, get as jni_handles, static_method,
};
use crate::android::widgets::Site;
use crate::android::widgets::container::string_array;
use crate::ui::NativeViewProps;
use jni::errors::Result as JniResult;
use jni::objects::{JClass, JObject};
use jni::refs::Global;
use jni::signature::RuntimeMethodSignature;
use jni::{Env, JValue};
use jni::{jni_sig, jni_str};

/// Tells the host its name and the properties of the description; the host
/// asks the application's factory for a view when the name is new.
pub(crate) fn update_native_view(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &NativeViewProps,
) -> JniResult<()> {
    let name = jcall!(env, env.new_string(&props.name))?;
    let keys = string_array(
        env,
        props.props.iter().map(|(key, _)| key.as_str()).collect(),
    )?;
    let values = string_array(
        env,
        props
            .props
            .iter()
            .map(|(_, value)| value.as_str())
            .collect(),
    )?;
    let ids = jni_handles(env)?;
    ids.native_view.update.call_void(
        env,
        &ids.native_view.class,
        &[
            JValue::Object(view),
            JValue::Object(name.as_ref()),
            JValue::Object(&keys),
            JValue::Object(&values),
        ],
    )
}

/// Creates the host of a native view: an empty frame until the first update.
pub(crate) fn create<'local>(env: &mut Env<'local>, site: &Site<'_>) -> JniResult<JObject<'local>> {
    let ids = jni_handles(env)?;
    ids.native_view
        .create
        .call(
            env,
            &ids.native_view.class,
            &[
                JValue::Object(site.context),
                JValue::Object(site.root),
                JValue::Long(site.node.0 as jni::sys::jlong),
            ],
        )?
        .l()
}

/// The JNI handles this module uses, resolved once; see [`crate::android::handles`].
pub(crate) struct Handles {
    pub(crate) class: Global<JClass<'static>>,
    pub(crate) create: StaticMethod,
    pub(crate) update: StaticMethod,
}

impl Handles {
    pub(crate) fn load(
        env: &mut Env<'_>,
        _widget_signature: &RuntimeMethodSignature,
    ) -> JniResult<Self> {
        let views = class(env, NATIVE_VIEWS_CLASS)?;
        Ok(Self {
            create: static_method!(
                env,
                &views,
                "create",
                "(Landroid/content/Context;Ldev/krabik/ui/RustRootView;J)Landroid/view/View;",
                OBJECT
            ),
            update: static_method!(
                env,
                &views,
                "update",
                "(Landroid/view/View;Ljava/lang/String;[Ljava/lang/String;[Ljava/lang/String;)V",
                VOID
            ),
            class: views,
        })
    }
}
