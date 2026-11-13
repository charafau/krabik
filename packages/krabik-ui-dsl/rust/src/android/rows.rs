//! Recycled lists: binding rows into the holders a `RecyclerView` provides.

use crate::android::handles::get as jni_handles;
use crate::android::host::{AndroidState, RowState};
use crate::geometry::Size;
use crate::host::Actions;
use crate::layout::LayoutResult;
use crate::tree::{NodeId, RecyclerProps, UiNode};
use crate::{render, tree};
use jni::errors::Result as JniResult;
use jni::objects::JObject;
use jni::sys::{jint, jlong};
use jni::{Env, JValue};
use std::collections::HashMap;

impl AndroidState {
    /// Forgets row actions of bindings whose index no longer exists.
    pub(crate) fn prune_row_actions(&mut self) {
        let stale: Vec<((NodeId, usize), Vec<NodeId>)> = self
            .row_action_ids
            .iter()
            .filter(|((list, index), _)| {
                self.runtime
                    .recycled_item_count(*list)
                    .is_none_or(|count| *index >= count)
            })
            .map(|(key, ids)| (*key, ids.clone()))
            .collect();

        for (key, ids) in stale {
            self.row_action_ids.remove(&key);
            for id in ids {
                self.row_actions.remove(&id);
            }
        }

        // A screen switch removes the recycler from the main tree; its cached
        // holders must not keep their widgets alive.
        let detached: Vec<(NodeId, i64)> = self
            .row_states
            .keys()
            .filter(|(list, _)| self.runtime.recycled_item_count(*list).is_none())
            .copied()
            .collect();
        for key in detached {
            if let Some(state) = self.row_states.remove(&key) {
                self.unregister_row_nodes(&state.tree);
            }
        }
    }

    /// Drops the actions and widgets recorded for one row binding.
    pub(crate) fn clear_row_actions(&mut self, list: NodeId, index: usize) {
        if let Some(ids) = self.row_action_ids.remove(&(list, index)) {
            for id in ids {
                self.row_actions.remove(&id);
            }
        }
    }

    /// Builds one recycled-list row into a holder host.
    ///
    /// Returns the row height in pixels, which a variable-height recycler uses
    /// to size the holder. A rebind of the same item diffs against the cached
    /// tree instead of recreating every widget.
    pub(crate) fn bind_row(
        &mut self,
        env: &mut Env<'_>,
        list: NodeId,
        index: usize,
        holder: i64,
        host: &JObject<'_>,
        width_px: jint,
    ) -> JniResult<jint> {
        let (transition_ms, element) = {
            let Some(source) = self.runtime.row_source(list) else {
                return Ok(0);
            };
            if index >= source.item_count {
                return Ok(0);
            }
            (source.row_transition_ms, (source.rows)(index))
        };

        let root = tree::row_root(list, index);
        let width = width_px.max(1) as f32 / self.density;
        let (tree, layout) =
            self.runtime
                .layout_subtree(&element, root, Size::new(width, 100_000.0));

        let previous = self.row_states.remove(&(list, holder));
        let had_previous = previous.is_some();
        let (old_tree, old_layout, mut views) = if let Some(mut state) = previous {
            self.clear_row_actions(list, state.index);
            if state.index != index
                && let Some(focus) = self.runtime.row_focus()
                && tree::find(&state.tree, focus.field).is_some()
            {
                // The holder is about to show another item; keep the logical
                // focus so it can be restored when the row comes back.
                self.runtime.park_row_focus(focus.field);
            }
            self.unregister_row_nodes(&state.tree);
            let views = std::mem::take(&mut state.views);
            (state.tree, state.layout, views)
        } else {
            // The holder was recycled without telling us, so its previous
            // widgets are unknown. Diffing against an empty tree creates the
            // whole row and its frames.
            jni_handles(env)?
                .remove_all_views
                .call_void(env, host, &[])?;
            (UiNode::empty(), LayoutResult::default(), HashMap::new())
        };

        let mut ops = render::diff(&old_tree, &tree);
        ops.extend(render::diff_layout(&old_tree, &tree, &old_layout, &layout));

        // The overlay is captured before the new content is inserted, so it
        // shows exactly what this holder displayed a moment ago.
        let overlay = if transition_ms > 0 && had_previous && !ops.is_empty() {
            let ids = jni_handles(env)?;
            let overlay = ids
                .snapshot_row
                .call(env, &ids.krabik_rows, &[JValue::Object(host)])?
                .l()?;
            if overlay.is_null() {
                None
            } else {
                Some(jcall!(env, env.new_global_ref(&overlay))?)
            }
        } else {
            None
        };

        for op in ops {
            self.apply_op(env, op, &mut views, host, false, None)?;
        }

        if let Some(overlay) = overlay {
            let ids = jni_handles(env)?;
            ids.fade_row.call_void(
                env,
                &ids.krabik_rows,
                &[
                    JValue::Object(host),
                    JValue::Object(overlay.as_obj()),
                    JValue::Int(transition_ms.min(i32::MAX as u32) as jint),
                ],
            )?;
        }

        self.record_row_actions(list, index, &tree);
        self.register_row_nodes(list, index, &tree);
        let height = layout
            .frames
            .get(&root)
            .map_or(0.0, |frame| frame.height)
            .max(0.0);
        self.row_states.insert(
            (list, holder),
            RowState {
                index,
                tree,
                layout,
                views,
            },
        );

        Ok((height * self.density).round().max(1.0) as jint)
    }

    /// Releases the widgets and actions of the row a holder currently shows.
    pub(crate) fn unbind_row(&mut self, list: NodeId, holder: i64) {
        if let Some(state) = self.row_states.remove(&(list, holder)) {
            if let Some(focus) = self.runtime.row_focus()
                && tree::find(&state.tree, focus.field).is_some()
            {
                self.runtime.park_row_focus(focus.field);
            }
            self.unregister_row_nodes(&state.tree);
            self.clear_row_actions(list, state.index);
        }
    }

    /// Maps every node of a bound row back to its item.
    pub(crate) fn register_row_nodes(&mut self, list: NodeId, index: usize, tree: &UiNode) {
        for node in tree::preorder(tree) {
            self.row_nodes.insert(node.id, (list, index));
        }
    }

    /// Forgets the nodes of a row that is no longer bound.
    pub(crate) fn unregister_row_nodes(&mut self, tree: &UiNode) {
        for node in tree::preorder(tree) {
            self.row_nodes.remove(&node.id);
        }
    }

    pub(crate) fn record_row_actions(&mut self, list: NodeId, index: usize, tree: &UiNode) {
        let mut ids = Vec::new();
        for node in tree::preorder(tree) {
            if let Some(actions) = Actions::of(node) {
                self.row_actions.insert(node.id, actions);
                ids.push(node.id);
            }
        }
        self.row_action_ids.insert((list, index), ids);
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
    ids.create_recycler
        .call(
            env,
            &ids.krabik_recyclers,
            &[
                JValue::Object(context),
                JValue::Object(root),
                JValue::Long(node_id.0 as jlong),
                JValue::Int(props.item_count.min(i32::MAX as usize) as jint),
                JValue::Int(height),
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
    let ids = jni_handles(env)?;
    let height = row_height_px(props, density);
    ids.update_recycler.call_void(
        env,
        &ids.krabik_recyclers,
        &[
            JValue::Object(view),
            JValue::Int(props.item_count.min(i32::MAX as usize) as jint),
            JValue::Int(height),
            JValue::Long(props.revision as jlong),
        ],
    )
}

/// The fixed row height in pixels, or `0` to let the recycler wrap content.
pub(crate) fn row_height_px(props: &RecyclerProps, density: f32) -> jint {
    props
        .item_height
        .map_or(0, |height| (height * density).round().max(1.0) as jint)
}
