//! Recycled lists: binding rows into the holders a `RecyclerView` provides.

use crate::android::handles::{
    OBJECT, StaticMethod, VOID, class, get as jni_handles, root_signature, static_method,
};
use crate::android::host::AndroidState;
use crate::android::{KRABIK_RECYCLERS_CLASS, KRABIK_ROWS_CLASS};
use crate::host::rows::Bind;
use crate::tree::{NodeId, RecyclerProps};
use jni::errors::Result as JniResult;
use jni::objects::{JClass, JObject};
use jni::refs::Global;
use jni::signature::RuntimeMethodSignature;
use jni::sys::{jint, jlong};
use jni::{Env, JValue};
use jni::{jni_sig, jni_str};

impl AndroidState {
    /// Forgets rows and row actions whose list or item no longer exists.
    pub(crate) fn prune_row_actions(&mut self) {
        self.rows.prune(&self.runtime);
    }

    /// Builds one recycled-list row into a holder host.
    ///
    /// Returns the row height in pixels, which a variable-height recycler uses
    /// to size the holder. What to build, and what changed since the holder
    /// was last bound, is decided by the shared [`RowBinder`]; this function
    /// applies the result to the holder's views.
    ///
    /// [`RowBinder`]: crate::host::rows::RowBinder
    pub(crate) fn bind_row(
        &mut self,
        env: &mut Env<'_>,
        list: NodeId,
        index: usize,
        holder: i64,
        host: &JObject<'_>,
        width_px: jint,
    ) -> JniResult<jint> {
        let key = (list, holder);
        let width = width_px.max(1) as f32 / self.density;
        let mut pending = match self.rows.begin(&mut self.runtime, &key, list, index, width) {
            Bind::Missing => return Ok(0),
            Bind::Unchanged { height } => return Ok(self.row_height_px(height)),
            Bind::Changed(pending) => *pending,
        };

        if pending.fresh {
            // The holder was recycled without telling us, so its previous
            // widgets are unknown; the operations build the row from nothing.
            jni_handles(env)?
                .remove_all_views
                .call_void(env, host, &[])?;
        }

        // The overlay is captured before the new content is inserted, so it
        // shows exactly what this holder displayed a moment ago.
        let overlay = if pending.transition_ms > 0 && !pending.fresh && !pending.ops.is_empty() {
            let ids = jni_handles(env)?;
            let overlay = ids
                .rows
                .snapshot
                .call(env, &ids.rows.rows_class, &[JValue::Object(host)])?
                .l()?;
            if overlay.is_null() {
                None
            } else {
                Some(jcall!(env, env.new_global_ref(&overlay))?)
            }
        } else {
            None
        };

        for op in std::mem::take(&mut pending.ops) {
            self.apply_op(env, op, &mut pending.views, host, false, None)?;
        }

        if let Some(overlay) = overlay {
            let ids = jni_handles(env)?;
            ids.rows.fade.call_void(
                env,
                &ids.rows.rows_class,
                &[
                    JValue::Object(host),
                    JValue::Object(overlay.as_obj()),
                    JValue::Int(pending.transition_ms.min(i32::MAX as u32) as jint),
                ],
            )?;
        }

        let height_px = self.row_height_px(pending.height);
        self.rows.finish(key, pending);
        Ok(height_px)
    }

    fn row_height_px(&self, height: f32) -> jint {
        (height * self.density).round().max(1.0) as jint
    }

    /// Releases the widgets and actions of the row a holder currently shows.
    pub(crate) fn unbind_row(&mut self, list: NodeId, holder: i64) {
        self.rows.unbind(&mut self.runtime, &(list, holder));
    }
}

pub(crate) fn create_recycler<'local>(
    env: &mut Env<'local>,
    context: &JObject<'_>,
    root: &JObject<'_>,
    node_id: NodeId,
    props: &RecyclerProps,
    density: f32,
) -> JniResult<JObject<'local>> {
    let ids = jni_handles(env)?;
    let height = row_height_px(props, density);
    ids.rows
        .create
        .call(
            env,
            &ids.rows.recyclers_class,
            &[
                JValue::Object(context),
                JValue::Object(root),
                JValue::Long(node_id.0 as jlong),
                JValue::Int(props.item_count.min(i32::MAX as usize) as jint),
                JValue::Int(height),
                JValue::Bool(props.refresh.is_enabled()),
            ],
        )?
        .l()
}

pub(crate) fn update_recycler(
    env: &mut Env<'_>,
    view: &JObject<'_>,
    props: &RecyclerProps,
    density: f32,
) -> JniResult<()> {
    let positions: Vec<jint> = props
        .sticky
        .iter()
        .map(|index| (*index).min(i32::MAX as usize) as jint)
        .collect();
    let sticky = env.new_int_array(positions.len())?;
    sticky.set_region(env, 0, &positions)?;
    let ids = jni_handles(env)?;
    let height = row_height_px(props, density);
    ids.rows.update.call_void(
        env,
        &ids.rows.recyclers_class,
        &[
            JValue::Object(view),
            JValue::Int(props.item_count.min(i32::MAX as usize) as jint),
            JValue::Int(height),
            JValue::Long(props.revision as jlong),
            JValue::Bool(props.on_dismiss.is_set()),
            JValue::Bool(props.on_reorder.is_set()),
            JValue::Object(&sticky),
        ],
    )
}

/// The fixed row height in pixels, or `0` to let the recycler wrap content.
pub(crate) fn row_height_px(props: &RecyclerProps, density: f32) -> jint {
    props
        .item_height
        .map_or(0, |height| (height * density).round().max(1.0) as jint)
}

/// The JNI handles this module uses, resolved once; see [`crate::android::handles`].
pub(crate) struct Handles {
    pub(crate) rows_class: Global<JClass<'static>>,
    pub(crate) recyclers_class: Global<JClass<'static>>,
    pub(crate) create: StaticMethod,
    pub(crate) update: StaticMethod,
    pub(crate) snapshot: StaticMethod,
    pub(crate) fade: StaticMethod,
}

impl Handles {
    pub(crate) fn load(
        env: &mut Env<'_>,
        _widget_signature: &RuntimeMethodSignature,
    ) -> JniResult<Self> {
        let krabik_rows = class(env, KRABIK_ROWS_CLASS)?;
        let krabik_recyclers = class(env, KRABIK_RECYCLERS_CLASS)?;
        Ok(Self {
            create: StaticMethod {
                id: env.get_static_method_id(
                    &krabik_recyclers,
                    jni_str!("create"),
                    root_signature(
                        "(Landroid/content/Context;{root}JIIZ)Landroid/view/ViewGroup;",
                    )?
                    .method_signature(),
                )?,
                returns: OBJECT,
            },
            update: static_method!(
                env,
                &krabik_recyclers,
                "update",
                "(Landroid/view/ViewGroup;IIJZZ[I)V",
                VOID
            ),
            snapshot: static_method!(
                env,
                &krabik_rows,
                "snapshot",
                "(Landroid/view/ViewGroup;)Landroid/widget/ImageView;",
                OBJECT
            ),
            fade: static_method!(
                env,
                &krabik_rows,
                "fadeOut",
                "(Landroid/view/ViewGroup;Landroid/view/View;I)V",
                VOID
            ),
            rows_class: krabik_rows,
            recyclers_class: krabik_recyclers,
        })
    }
}
