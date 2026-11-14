//! Binding the rows of a recycled list: the half both platforms share.
//!
//! A recycled list is a native recycler (`RecyclerView`, `UICollectionView`).
//! It owns a handful of reusable holders and asks for a row to be put into
//! one whenever it scrolls into view, and again after every render. The rows
//! are not part of the host's tree: each is built on demand as a small tree
//! of its own.
//!
//! [`RowBinder`] keeps, per holder, the tree and layout the row was last
//! shown with. Binding is then the same diff the main tree gets: compare
//! with what the holder showed, and hand the platform the operations to
//! apply. The platform only applies them; it decides nothing.

use std::collections::HashMap;
use std::hash::Hash;

use crate::engine::geometry::Size;
use crate::engine::layout::LayoutResult;
use crate::engine::render::{self, Op};
use crate::engine::runtime::Runtime;
use crate::engine::tree::{self, NodeId, UiNode};

use super::Actions;

/// What one holder shows.
pub(crate) struct Row<V> {
    /// The recycled list the row belongs to.
    pub(crate) list: NodeId,
    /// The item the holder shows.
    pub(crate) index: usize,
    /// The width the row was laid out for, in logical points.
    width: f32,
    /// The measured height of the row, in logical points.
    pub(crate) height: f32,
    /// The row's own tree, rooted at [`tree::row_root`].
    pub(crate) tree: UiNode,
    layout: LayoutResult,
    /// The row's native views by node id.
    pub(crate) views: HashMap<NodeId, V>,
}

/// The outcome of asking for a row to be bound.
pub(crate) enum Bind<V> {
    /// The list or the item no longer exists.
    Missing,
    /// The holder already shows exactly this row. Nothing to apply; the
    /// handlers of the latest render were taken over.
    Unchanged {
        /// The row's height in logical points.
        height: f32,
    },
    /// The platform has operations to apply, then calls
    /// [`RowBinder::finish`].
    Changed(Box<Pending<V>>),
}

/// A row whose operations the platform is about to apply.
pub(crate) struct Pending<V> {
    /// What to do to the holder's views, in order.
    pub(crate) ops: Vec<Op>,
    /// The holder's views, to be updated by applying `ops`.
    pub(crate) views: HashMap<NodeId, V>,
    /// The holder was not known: whatever native views it still contains
    /// must be removed first, and `ops` build the row from nothing.
    pub(crate) fresh: bool,
    /// The holder showed this same item before.
    pub(crate) rebinding: bool,
    /// The same item now has another height.
    pub(crate) resized: bool,
    /// The row's height in logical points.
    pub(crate) height: f32,
    /// How long the list cross-fades a row whose content changed.
    pub(crate) transition_ms: u32,
    list: NodeId,
    index: usize,
    width: f32,
    tree: UiNode,
    layout: LayoutResult,
}

/// The rows a host's recyclers currently show, by holder.
///
/// `K` identifies a holder (a `RecyclerView` holder id, a collection-view
/// cell). `V` is the platform's handle to one native view.
pub(crate) struct RowBinder<K, V> {
    rows: HashMap<K, Row<V>>,
    /// The handlers of every widget in a bound row. Row widgets are not in
    /// the host's tree, so an event finds its message here.
    actions: HashMap<NodeId, Actions>,
    /// Which widgets recorded actions for an item, to remove them again.
    action_ids: HashMap<(NodeId, usize), Vec<NodeId>>,
    /// Every node of a bound row, mapped back to its list and item.
    nodes: HashMap<NodeId, (NodeId, usize)>,
}

impl<K, V> Default for RowBinder<K, V> {
    fn default() -> Self {
        Self {
            rows: HashMap::new(),
            actions: HashMap::new(),
            action_ids: HashMap::new(),
            nodes: HashMap::new(),
        }
    }
}

impl<K: Eq + Hash + Clone, V> RowBinder<K, V> {
    /// Builds item `index` of `list` for the holder `key` at `width` logical
    /// points and compares it with what the holder shows.
    pub(crate) fn begin(
        &mut self,
        runtime: &mut Runtime,
        key: &K,
        list: NodeId,
        index: usize,
        width: f32,
    ) -> Bind<V> {
        let (transition_ms, element) = {
            let Some(source) = runtime.row_source(list) else {
                return Bind::Missing;
            };
            if index >= source.item_count {
                return Bind::Missing;
            }
            (source.row_transition_ms, (source.rows)(index))
        };
        let root = tree::row_root(list, index);
        let tree = runtime.build_subtree(&element, root);

        // A control in this row the user just changed is updated whatever
        // the comparison says; see `Runtime::reapply`.
        let reapply = runtime.take_row_reapply(&tree);

        // A render rebinds every attached row. Most of them did not change:
        // keep the widgets and frames, and only take over the new handlers.
        if reapply.is_none()
            && let Some(row) = self.rows.get_mut(key)
            && row.list == list
            && row.index == index
            && row.width == width
            && row.tree == tree
        {
            let height = row.height;
            row.tree = tree;
            self.clear_actions(list, index);
            let actions = actions_of(&self.rows[key].tree);
            self.store_actions(list, index, actions);
            return Bind::Unchanged { height };
        }

        let layout = runtime.layout_tree(&tree, Size::new(width.max(1.0), 100_000.0));
        let height = layout
            .frames
            .get(&root)
            .map_or(0.0, |frame| frame.height.max(0.0));

        let previous = self.rows.remove(key);
        let fresh = previous.is_none();
        let rebinding = previous
            .as_ref()
            .is_some_and(|row| row.list == list && row.index == index);
        let resized = rebinding
            && previous
                .as_ref()
                .is_some_and(|row| (row.height - height).abs() > 0.5);

        let (old_tree, old_layout, views) = match previous {
            Some(row) => {
                self.clear_actions(row.list, row.index);
                if !rebinding {
                    // The holder is about to show another item; keep the
                    // logical focus so it can be restored when the row
                    // comes back.
                    park_focus(runtime, &row.tree);
                }
                self.unregister(&row.tree);
                (row.tree, row.layout, row.views)
            }
            // The holder was recycled without telling us, so what it shows
            // is unknown. Diffing against an empty tree creates the whole
            // row and its frames.
            None => (UiNode::empty(), LayoutResult::default(), HashMap::new()),
        };

        let mut ops = render::diff_reapplying(&old_tree, &tree, reapply);
        ops.extend(render::diff_layout(&old_tree, &tree, &old_layout, &layout));

        Bind::Changed(Box::new(Pending {
            ops,
            views,
            fresh,
            rebinding,
            resized,
            height,
            transition_ms,
            list,
            index,
            width,
            tree,
            layout,
        }))
    }

    /// Records the row as shown, with the views the operations left.
    pub(crate) fn finish(&mut self, key: K, pending: Pending<V>) {
        let Pending {
            views,
            height,
            list,
            index,
            width,
            tree,
            layout,
            ..
        } = pending;
        let actions = actions_of(&tree);
        self.store_actions(list, index, actions);
        for node in tree::preorder(&tree) {
            self.nodes.insert(node.id, (list, index));
        }
        self.rows.insert(
            key,
            Row {
                list,
                index,
                width,
                height,
                tree,
                layout,
                views,
            },
        );
    }

    /// Forgets what a holder shows and returns it, so the platform can
    /// release the views. The logical focus of a field in it is kept.
    pub(crate) fn unbind(&mut self, runtime: &mut Runtime, key: &K) -> Option<Row<V>> {
        let row = self.rows.remove(key)?;
        park_focus(runtime, &row.tree);
        self.unregister(&row.tree);
        self.clear_actions(row.list, row.index);
        Some(row)
    }

    /// Forgets actions and rows whose list or item no longer exists, for
    /// example after a screen swap removed the recycler from the tree.
    pub(crate) fn prune(&mut self, runtime: &Runtime) {
        let stale: Vec<(NodeId, usize)> = self
            .action_ids
            .keys()
            .filter(|(list, index)| {
                runtime
                    .recycled_item_count(*list)
                    .is_none_or(|count| *index >= count)
            })
            .copied()
            .collect();
        for (list, index) in stale {
            self.clear_actions(list, index);
        }

        let detached: Vec<K> = self
            .rows
            .iter()
            .filter(|(_, row)| runtime.recycled_item_count(row.list).is_none())
            .map(|(key, _)| key.clone())
            .collect();
        for key in detached {
            if let Some(row) = self.rows.remove(&key) {
                self.unregister(&row.tree);
            }
        }
    }

    /// The handlers of a widget inside a bound row.
    pub(crate) fn actions(&self, id: NodeId) -> Option<&Actions> {
        self.actions.get(&id)
    }

    /// The list and item a row node belongs to.
    pub(crate) fn item_of(&self, id: NodeId) -> Option<(NodeId, usize)> {
        self.nodes.get(&id).copied()
    }

    /// What a holder shows.
    pub(crate) fn row(&self, key: &K) -> Option<&Row<V>> {
        self.rows.get(key)
    }

    /// Every bound row.
    pub(crate) fn rows(&self) -> impl Iterator<Item = &Row<V>> {
        self.rows.values()
    }

    fn unregister(&mut self, tree: &UiNode) {
        for node in tree::preorder(tree) {
            self.nodes.remove(&node.id);
        }
    }

    fn clear_actions(&mut self, list: NodeId, index: usize) {
        if let Some(ids) = self.action_ids.remove(&(list, index)) {
            for id in ids {
                self.actions.remove(&id);
            }
        }
    }

    fn store_actions(&mut self, list: NodeId, index: usize, actions: Vec<(NodeId, Actions)>) {
        let ids = actions.iter().map(|(id, _)| *id).collect();
        self.actions.extend(actions);
        self.action_ids.insert((list, index), ids);
    }
}

/// The handlers of every widget in a row.
fn actions_of(tree: &UiNode) -> Vec<(NodeId, Actions)> {
    tree::preorder(tree)
        .into_iter()
        .filter_map(|node| Actions::of(node).map(|actions| (node.id, actions)))
        .collect()
}

/// Keeps the logical focus of a field in `tree`, whose views are going away.
fn park_focus(runtime: &mut Runtime, tree: &UiNode) {
    if let Some(focus) = runtime.row_focus()
        && tree::find(tree, focus.field).is_some()
    {
        runtime.park_row_focus(focus.field);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::element::Element;
    use crate::engine::measure::ApproximateMeasure;
    use crate::engine::tree::NodeKind;
    use crate::host::NodeEvent;
    use crate::ui::{Column, List, Row as RowWidget, Switch, Text, TextField};
    use crate::widgets::Widget;

    #[derive(Clone, Debug, PartialEq)]
    enum Msg {
        Toggled(usize, bool),
        Typed(usize, String),
    }

    /// What the list shows: one label and one switch state per item.
    type Items = Arc<Mutex<Vec<(String, bool)>>>;

    fn items(labels: &[&str]) -> Items {
        Arc::new(Mutex::new(
            labels
                .iter()
                .map(|label| (label.to_string(), false))
                .collect(),
        ))
    }

    /// A screen with one recycled list over `items`.
    fn screen(items: &Items) -> Element<Msg> {
        let snapshot = Arc::new(items.lock().unwrap().clone());
        let indices: Arc<Vec<usize>> = Arc::new((0..snapshot.len()).collect());
        Column::new()
            .child(
                List::new(indices)
                    .item_height(40.0)
                    .recycled()
                    .key(|index| index.to_string())
                    .row(move |index| {
                        let index = *index;
                        let (label, on) = snapshot[index].clone();
                        RowWidget::new()
                            .child(Text::new(label.clone()))
                            .child(Switch::new(on).on_toggle(move |on| Msg::Toggled(index, on)))
                            .child(
                                TextField::new(label)
                                    .on_change(move |text| Msg::Typed(index, text)),
                            )
                            .into()
                    }),
            )
            .into()
    }

    /// A runtime that has rendered `items`, and the id of its list.
    fn rendered(items: &Items) -> (Runtime, NodeId) {
        tree::recycle_in_tests(true);
        let mut runtime = Runtime::new(Box::new(ApproximateMeasure));
        runtime.update(&screen(items), Size::new(320.0, 640.0));
        let list = tree::preorder(runtime.tree())
            .into_iter()
            .find(|node| matches!(node.kind, NodeKind::Recycler(_)))
            .expect("a recycled list is a recycler node")
            .id;
        (runtime, list)
    }

    type Binder = RowBinder<u32, ()>;

    /// Binds and plays the platform: one unit "view" per created node.
    fn bind(
        binder: &mut Binder,
        runtime: &mut Runtime,
        holder: u32,
        list: NodeId,
        index: usize,
    ) -> Vec<Op> {
        match binder.begin(runtime, &holder, list, index, 320.0) {
            Bind::Changed(pending) => {
                let mut pending = *pending;
                let ops = pending.ops.clone();
                for op in &ops {
                    match op {
                        Op::Create(node) => {
                            pending.views.insert(node.id, ());
                        }
                        Op::Remove { id, .. } => {
                            pending.views.remove(id);
                        }
                        _ => {}
                    }
                }
                binder.finish(holder, pending);
                ops
            }
            Bind::Unchanged { .. } => Vec::new(),
            Bind::Missing => panic!("the item exists"),
        }
    }

    fn widget_of(binder: &Binder, holder: u32, matches: impl Fn(&Widget) -> bool) -> NodeId {
        let row = binder.row(&holder).expect("the holder is bound");
        tree::preorder(&row.tree)
            .into_iter()
            .find(|node| matches!(&node.kind, NodeKind::Widget(widget) if matches(widget)))
            .expect("the row has the widget")
            .id
    }

    fn updates(ops: &[Op]) -> Vec<NodeId> {
        ops.iter()
            .filter_map(|op| match op {
                Op::Update(node) => Some(node.id),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_first_bind_creates_the_row_and_records_its_handlers() {
        let items = items(&["a", "b"]);
        let (mut runtime, list) = rendered(&items);
        let mut binder = Binder::default();

        let Bind::Changed(pending) = binder.begin(&mut runtime, &1, list, 0, 320.0) else {
            panic!("a new holder has something to create");
        };
        assert!(pending.fresh);
        assert!(!pending.rebinding);
        assert!(pending.height > 0.0);
        assert!(pending.ops.iter().any(|op| matches!(op, Op::Create(_))));
        binder.finish(1, *pending);

        // The switch's handler is found by its id, and the id leads back to
        // the item.
        let switch = widget_of(&binder, 1, |widget| matches!(widget, Widget::Switch(_)));
        let message = binder
            .actions(switch)
            .and_then(|actions| actions.message(&NodeEvent::Toggle(true)))
            .and_then(|message| message.downcast::<Msg>().ok());
        assert_eq!(message.as_deref(), Some(&Msg::Toggled(0, true)));
        assert_eq!(binder.item_of(switch), Some((list, 0)));
    }

    #[test]
    fn neighbouring_rows_do_not_share_ids() {
        let items = items(&["a", "b"]);
        let (mut runtime, list) = rendered(&items);
        let mut binder = Binder::default();
        bind(&mut binder, &mut runtime, 1, list, 0);
        bind(&mut binder, &mut runtime, 2, list, 1);

        let first = widget_of(&binder, 1, |widget| matches!(widget, Widget::TextField(_)));
        let second = widget_of(&binder, 2, |widget| matches!(widget, Widget::TextField(_)));
        assert_ne!(first, second);

        // Typing in the second row is reported for the second item.
        let message = binder
            .actions(second)
            .and_then(|actions| actions.message(&NodeEvent::Text("x".into())))
            .and_then(|message| message.downcast::<Msg>().ok());
        assert_eq!(message.as_deref(), Some(&Msg::Typed(1, "x".into())));
    }

    #[test]
    fn rebinding_an_unchanged_row_applies_nothing() {
        let items = items(&["a", "b"]);
        let (mut runtime, list) = rendered(&items);
        let mut binder = Binder::default();
        bind(&mut binder, &mut runtime, 1, list, 0);

        // Another render, same state: the adapter rebinds every row.
        runtime.update(&screen(&items), Size::new(320.0, 640.0));
        assert!(matches!(
            binder.begin(&mut runtime, &1, list, 0, 320.0),
            Bind::Unchanged { .. }
        ));
        // The handlers are still there after the shortcut.
        let switch = widget_of(&binder, 1, |widget| matches!(widget, Widget::Switch(_)));
        assert!(binder.actions(switch).is_some());
    }

    #[test]
    fn a_changed_row_is_updated_in_place() {
        let items = items(&["a", "b"]);
        let (mut runtime, list) = rendered(&items);
        let mut binder = Binder::default();
        bind(&mut binder, &mut runtime, 1, list, 0);
        let switch = widget_of(&binder, 1, |widget| matches!(widget, Widget::Switch(_)));

        items.lock().unwrap()[0].1 = true;
        runtime.update(&screen(&items), Size::new(320.0, 640.0));
        let ops = bind(&mut binder, &mut runtime, 1, list, 0);

        assert_eq!(updates(&ops), [switch]);
        assert!(
            !ops.iter()
                .any(|op| matches!(op, Op::Create(_) | Op::Remove { .. }))
        );
    }

    #[test]
    fn a_holder_given_another_item_swaps_the_rows_views() {
        let items = items(&["a", "b"]);
        let (mut runtime, list) = rendered(&items);
        let mut binder = Binder::default();
        bind(&mut binder, &mut runtime, 1, list, 0);
        let old = widget_of(&binder, 1, |widget| matches!(widget, Widget::Switch(_)));

        let Bind::Changed(pending) = binder.begin(&mut runtime, &1, list, 1, 320.0) else {
            panic!("another item is a change");
        };
        assert!(!pending.fresh);
        assert!(!pending.rebinding);
        // The first item's widgets go, the second item's are created.
        assert!(
            pending
                .ops
                .iter()
                .any(|op| matches!(op, Op::Remove { id, .. } if *id == old))
        );
        assert!(pending.ops.iter().any(|op| matches!(op, Op::Create(_))));
        binder.finish(1, *pending);

        // The first item's handlers went with it.
        assert!(binder.actions(old).is_none());
        assert_eq!(binder.row(&1).unwrap().index, 1);
    }

    #[test]
    fn a_control_the_application_overruled_is_updated_although_nothing_changed() {
        let items = items(&["a", "b"]);
        let (mut runtime, list) = rendered(&items);
        let mut binder = Binder::default();
        bind(&mut binder, &mut runtime, 1, list, 0);
        bind(&mut binder, &mut runtime, 2, list, 1);
        let switch = widget_of(&binder, 2, |widget| matches!(widget, Widget::Switch(_)));

        // The user flipped the switch in the second row; the application
        // left its state alone, so the row's description is the same.
        runtime.reapply(switch);
        runtime.update(&screen(&items), Size::new(320.0, 640.0));

        // Only the row that holds the control is touched.
        assert!(bind(&mut binder, &mut runtime, 1, list, 0).is_empty());
        assert_eq!(
            updates(&bind(&mut binder, &mut runtime, 2, list, 1)),
            [switch]
        );
        // And only once.
        assert!(bind(&mut binder, &mut runtime, 2, list, 1).is_empty());
    }

    #[test]
    fn an_item_that_is_gone_is_reported_missing() {
        let items = items(&["a", "b"]);
        let (mut runtime, list) = rendered(&items);
        let mut binder = Binder::default();
        bind(&mut binder, &mut runtime, 1, list, 1);
        let switch = widget_of(&binder, 1, |widget| matches!(widget, Widget::Switch(_)));

        items.lock().unwrap().pop();
        runtime.update(&screen(&items), Size::new(320.0, 640.0));
        assert!(matches!(
            binder.begin(&mut runtime, &1, list, 1, 320.0),
            Bind::Missing
        ));

        // Its handlers are dropped with the item.
        binder.prune(&runtime);
        assert!(binder.actions(switch).is_none());
    }

    #[test]
    fn unbinding_releases_the_row_and_keeps_a_focused_field_in_mind() {
        let items = items(&["a", "b"]);
        let (mut runtime, list) = rendered(&items);
        let mut binder = Binder::default();
        bind(&mut binder, &mut runtime, 1, list, 0);
        let field = widget_of(&binder, 1, |widget| matches!(widget, Widget::TextField(_)));
        runtime.set_row_focus(list, 0, field, true);

        let row = binder
            .unbind(&mut runtime, &1)
            .expect("the holder was bound");
        assert!(row.views.contains_key(&field));
        assert!(binder.row(&1).is_none());
        assert!(binder.actions(field).is_none());
        assert_eq!(binder.item_of(field), None);
        // The row left the screen; that is not the user leaving the field.
        assert!(runtime.row_focus().is_some_and(|focus| focus.parked));
    }

    #[test]
    fn rows_of_a_list_that_left_the_screen_are_forgotten() {
        let items = items(&["a", "b"]);
        let (mut runtime, list) = rendered(&items);
        let mut binder = Binder::default();
        bind(&mut binder, &mut runtime, 1, list, 0);
        assert_eq!(binder.rows().count(), 1);

        let other: Element<Msg> = Column::new().child(Text::new("elsewhere")).into();
        runtime.update(&other, Size::new(320.0, 640.0));
        binder.prune(&runtime);
        assert_eq!(binder.rows().count(), 0);
    }
}
