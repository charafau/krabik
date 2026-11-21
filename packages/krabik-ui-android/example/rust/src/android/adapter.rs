//! Item adapters used by the RecyclerView, ViewPager2, and Material carousel demos.
//!
//! AndroidX's `RecyclerView.Adapter` is an abstract Java class, so it cannot be instantiated
//! through JNI alone. The app owns the small concrete adapters (`TextRecyclerAdapter.java` and
//! `CarouselTextAdapter.java`); this factory constructs them from Rust and hands them to the
//! reusable renderer.

use jni::objects::{JObject, JValue};
use jni::strings::JNIStr;
use jni::{Env, jni_sig, jni_str};

use krabik_ui_android::env::EnvExt;
use krabik_ui_android::error::Result;
use krabik_ui_android::views::{self, AdapterItemSizing, ItemAdapterFactory};

const TEXT_RECYCLER_ADAPTER_CLASS: &JNIStr =
    jni_str!("com/example/krabik_ui_example/TextRecyclerAdapter");
const CAROUSEL_TEXT_ADAPTER_CLASS: &JNIStr =
    jni_str!("com/example/krabik_ui_example/CarouselTextAdapter");

pub struct TextAdapterFactory;

pub static TEXT_ADAPTER_FACTORY: TextAdapterFactory = TextAdapterFactory;

impl ItemAdapterFactory for TextAdapterFactory {
    fn create_adapter<'local>(
        &self,
        env: &mut Env<'local>,
        context: &JObject<'_>,
        items: &[&str],
        sizing: AdapterItemSizing,
    ) -> Result<JObject<'local>> {
        let items = views::new_string_array(env, items)?;

        if matches!(sizing, AdapterItemSizing::Carousel) {
            return env.construct(
                CAROUSEL_TEXT_ADAPTER_CLASS,
                jni_sig!("(Landroid/content/Context;[Ljava/lang/String;)V"),
                &[JValue::Object(context), JValue::Object(&items)],
            );
        }

        env.construct(
            TEXT_RECYCLER_ADAPTER_CLASS,
            jni_sig!("(Landroid/content/Context;[Ljava/lang/String;Z)V"),
            &[
                JValue::Object(context),
                JValue::Object(&items),
                JValue::Bool(matches!(sizing, AdapterItemSizing::FillParent)),
            ],
        )
    }
}
