//! Recycled lists: the collection view data source and binding rows into its cells.

use crate::geometry::Size;
use crate::host::Actions;
use crate::ios::host::{Content, IosState, ViewHandle};
use crate::ios::motion::{animation_is_enabled, run_animation};
use crate::ios::{guard, remove_subviews, schedule, with_state};
use crate::layout::LayoutResult;
use crate::tree::{NodeId, RecyclerProps, UiNode};
use crate::ui::{AnimationCurve, RawElement};
use crate::{render, tree};
use block2::RcBlock;
use objc2::DefinedClass;
use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol};
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_core_foundation::CGSize;
use objc2_foundation::{NSArray, NSIndexPath, NSString};
use objc2_ui_kit::NSIndexPathUIKitAdditions;
use objc2_ui_kit::{
    UICollectionView, UICollectionViewCell, UICollectionViewDataSource, UICollectionViewDelegate,
    UICollectionViewDelegateFlowLayout, UICollectionViewLayout, UIScrollViewDelegate, UIView,
    UIViewAnimatingPosition,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// The reuse identifier of a recycled-list row cell.
pub(crate) const ROW_CELL_ID: &str = "KrabikRow";

/// What a recycled-list data source remembers about its node.
///
/// Only the main thread touches the collection view, but the callbacks take
/// `&self`, so the mutable parts are atomics.
pub(crate) struct RecyclerIvars {
    pub(crate) handle: u64,
    pub(crate) host: u64,
    pub(crate) id: NodeId,
    /// The item count the collection view currently shows.
    pub(crate) count: AtomicU64,
    /// The latest item count requested by the application.
    pub(crate) target_count: AtomicU64,
    /// Fixed row height in points encoded as `f64` bits; `0` means automatic.
    pub(crate) item_height: AtomicU64,
    pub(crate) revision: AtomicU64,
    /// A sync block is already scheduled.
    pub(crate) pending: AtomicBool,
    /// The count changed and needs one insert/delete batch.
    pub(crate) needs_batch: AtomicBool,
    /// A row-height change needs a layout invalidation.
    pub(crate) needs_layout: AtomicBool,
    /// A revision change needs the visible cells rebound.
    pub(crate) needs_refresh: AtomicBool,
}

/// The changes one sync block should apply to a collection view.
pub(crate) struct RecyclerChanges {
    pub(crate) from: usize,
    pub(crate) to: usize,
    pub(crate) batch: bool,
    pub(crate) layout: bool,
    pub(crate) refresh: bool,
}

objc2::define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "KrabikRecyclerDataSource"]
    #[ivars = RecyclerIvars]
    pub(crate) struct RecyclerDataSource;

    unsafe impl NSObjectProtocol for RecyclerDataSource {}

    unsafe impl UIScrollViewDelegate for RecyclerDataSource {}

    unsafe impl UICollectionViewDataSource for RecyclerDataSource {
        #[unsafe(method(collectionView:numberOfItemsInSection:))]
        fn number_of_items_in_section(
            &self,
            _collection_view: &UICollectionView,
            _section: isize,
        ) -> isize {
            self.ivars()
                .count
                .load(Ordering::Relaxed)
                .min(isize::MAX as u64) as isize
        }

        #[unsafe(method_id(collectionView:cellForItemAtIndexPath:))]
        fn cell_for_item_at_index_path(
            &self,
            collection_view: &UICollectionView,
            index_path: &NSIndexPath,
        ) -> Retained<UICollectionViewCell> {
            let identifier = NSString::from_str(ROW_CELL_ID);
            let cell = collection_view
                .dequeueReusableCellWithReuseIdentifier_forIndexPath(&identifier, index_path);
            let ivars = self.ivars();
            let index = index_path.item().max(0) as usize;
            let width = collection_view.bounds().size.width;
            let _ = with_state(ivars.handle, ivars.host, |state| {
                state.bind_row(ivars.id, index, &cell, width);
            });
            cell
        }
    }

    unsafe impl UICollectionViewDelegate for RecyclerDataSource {
        #[unsafe(method(collectionView:didEndDisplayingCell:forItemAtIndexPath:))]
        fn did_end_displaying_cell(
            &self,
            _collection_view: &UICollectionView,
            cell: &UICollectionViewCell,
            _index_path: &NSIndexPath,
        ) {
            let key = cell as *const UICollectionViewCell as usize;
            let ivars = self.ivars();
            let _ = with_state(ivars.handle, ivars.host, |state| state.unbind_row(key));
        }
    }

    unsafe impl UICollectionViewDelegateFlowLayout for RecyclerDataSource {
        #[unsafe(method(collectionView:layout:sizeForItemAtIndexPath:))]
        fn size_for_item_at_index_path(
            &self,
            collection_view: &UICollectionView,
            _collection_view_layout: &UICollectionViewLayout,
            index_path: &NSIndexPath,
        ) -> CGSize {
            let ivars = self.ivars();
            let width = collection_view.bounds().size.width.max(1.0);
            let fixed = f64::from_bits(ivars.item_height.load(Ordering::Relaxed));
            if fixed > 0.0 {
                return CGSize::new(width, fixed);
            }

            let index = index_path.item().max(0) as usize;
            let height = with_state(ivars.handle, ivars.host, |state| {
                state.row_height(ivars.id, index, width as f32)
            })
            .unwrap_or(0.0);
            CGSize::new(width, f64::from(height).max(1.0))
        }
    }
);

impl RecyclerDataSource {
    pub(crate) fn new(
        handle: u64,
        host: u64,
        id: NodeId,
        props: &RecyclerProps,
        mtm: MainThreadMarker,
    ) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(RecyclerIvars {
            handle,
            host,
            id,
            count: AtomicU64::new(props.item_count as u64),
            target_count: AtomicU64::new(props.item_count as u64),
            item_height: AtomicU64::new(props.item_height.map_or(0.0, f64::from).to_bits()),
            revision: AtomicU64::new(props.revision),
            pending: AtomicBool::new(false),
            needs_batch: AtomicBool::new(false),
            needs_layout: AtomicBool::new(false),
            needs_refresh: AtomicBool::new(false),
        });
        // SAFETY: `NSObject`'s initializer returns an initialized instance of
        // this class with the ivars already set.
        unsafe { objc2::msg_send![super(this), init] }
    }

    /// Records new list properties and schedules one coalesced sync.
    ///
    /// This runs while the state lock is held, so it only touches atomics.
    /// The batch update itself is deferred: UIKit may ask the data source for
    /// cells synchronously, and those callbacks acquire the state lock.
    pub(crate) fn update(&self, props: &RecyclerProps) {
        let count = props.item_count as u64;
        let height = props.item_height.map_or(0.0, f64::from).to_bits();
        let ivars = self.ivars();
        let count_changed = ivars.target_count.swap(count, Ordering::Relaxed) != count;
        let height_changed = ivars.item_height.swap(height, Ordering::Relaxed) != height;
        let revision_changed =
            ivars.revision.swap(props.revision, Ordering::Relaxed) != props.revision;
        if count_changed {
            ivars.needs_batch.store(true, Ordering::Release);
        }
        if height_changed {
            ivars.needs_layout.store(true, Ordering::Release);
        }
        if revision_changed {
            ivars.needs_refresh.store(true, Ordering::Release);
        }
        if (count_changed || height_changed || revision_changed)
            && !ivars.pending.swap(true, Ordering::AcqRel)
        {
            let handle = ivars.handle;
            let host = ivars.host;
            let id = ivars.id;
            schedule(move || sync_recycler(handle, host, id));
        }
    }

    /// Atomically takes the pending work for one sync block.
    pub(crate) fn take_changes(&self) -> Option<RecyclerChanges> {
        let ivars = self.ivars();
        // Clear the scheduled flag first: if an update lands while the batch is
        // running, it schedules another sync instead of being dropped.
        ivars.pending.store(false, Ordering::Release);

        let batch_flag = ivars.needs_batch.swap(false, Ordering::AcqRel);
        let layout = ivars.needs_layout.swap(false, Ordering::AcqRel);
        let refresh = ivars.needs_refresh.swap(false, Ordering::AcqRel);

        let from = ivars.count.load(Ordering::Acquire) as usize;
        let target = ivars.target_count.load(Ordering::Acquire) as usize;
        let batch = batch_flag && from != target;
        if batch {
            // The data source must report the new count while the batch runs.
            ivars.count.store(target as u64, Ordering::Release);
        }

        if !batch && !layout && !refresh {
            return None;
        }
        Some(RecyclerChanges {
            from,
            to: target,
            batch,
            layout,
            refresh,
        })
    }
}

/// One recycled row currently bound to a collection-view cell.
///
/// Cells are reused by UIKit, so the state is keyed by the cell's identity and
/// diffed when that cell shows another item or new content.
pub(crate) struct IosRowState {
    pub(crate) list: NodeId,
    pub(crate) index: usize,
    pub(crate) tree: UiNode,
    pub(crate) layout: LayoutResult,
    pub(crate) views: HashMap<NodeId, ViewHandle>,
}

impl IosState {
    /// The element of one recycled row, or `None` when the item vanished.
    pub(crate) fn row_element(&self, list: NodeId, index: usize) -> Option<RawElement> {
        let source = self.runtime.row_source(list)?;
        (index < source.item_count).then(|| (source.rows)(index))
    }

    /// Builds and lays out one recycled row with its own root identifier.
    pub(crate) fn row_layout(
        &self,
        list: NodeId,
        index: usize,
        width: f32,
    ) -> Option<(UiNode, LayoutResult)> {
        let element = self.row_element(list, index)?;
        let root = tree::row_root(list, index);
        let (tree, layout) =
            self.runtime
                .layout_subtree(&element, root, Size::new(width.max(1.0), 100_000.0));
        Some((tree, layout))
    }

    /// The height a variable-height row needs at the given width.
    pub(crate) fn row_height(&self, list: NodeId, index: usize, width: f32) -> f32 {
        let Some((_, layout)) = self.row_layout(list, index, width) else {
            return 0.0;
        };
        layout
            .frames
            .get(&tree::row_root(list, index))
            .map_or(0.0, |frame| frame.height.max(0.0))
    }

    /// The collection view and data source of a recycled list.
    pub(crate) fn recycler_for(
        &self,
        id: NodeId,
    ) -> Option<(Retained<UICollectionView>, Retained<RecyclerDataSource>)> {
        match &self.views.get(&id)?.content {
            Content::Recycler {
                collection,
                data_source,
            } => Some((collection.clone(), data_source.clone())),
            _ => None,
        }
    }

    /// The collection view of a recycled list.
    pub(crate) fn collection_for(&self, id: NodeId) -> Option<Retained<UICollectionView>> {
        self.recycler_for(id).map(|(collection, _)| collection)
    }

    /// Diffs every visible cell in place after an application state change.
    pub(crate) fn rebind_visible_rows(&mut self, id: NodeId) {
        let Some(collection) = self.collection_for(id) else {
            return;
        };
        let width = collection.bounds().size.width;
        let paths = collection.indexPathsForVisibleItems();
        for path in paths.iter() {
            let index = path.item().max(0) as usize;
            if let Some(cell) = collection.cellForItemAtIndexPath(&path) {
                self.bind_row(id, index, &cell, width);
            }
        }
        // Content changes can alter a row's measured height.
        collection.collectionViewLayout().invalidateLayout();
    }

    /// Builds or updates one recycled row inside a collection-view cell.
    pub(crate) fn bind_row(
        &mut self,
        list: NodeId,
        index: usize,
        cell: &UICollectionViewCell,
        width: f64,
    ) {
        let Some((tree, layout)) = self.row_layout(list, index, width as f32) else {
            return;
        };
        let transition_ms = self
            .runtime
            .row_source(list)
            .map_or(0, |props| props.row_transition_ms);
        let content = cell.contentView();
        let key = cell as *const UICollectionViewCell as usize;

        let previous = self.row_states.remove(&key);
        let had_previous = previous.is_some();
        let rebinding = previous
            .as_ref()
            .is_some_and(|state| state.list == list && state.index == index);
        let (old_tree, old_layout, mut views) = if let Some(state) = previous {
            self.clear_row_actions(state.list, state.index);
            if !rebinding
                && let Some(focus) = self.runtime.row_focus()
                && tree::find(&state.tree, focus.field).is_some()
            {
                // The cell is about to show another item; keep the logical
                // focus for when the row comes back.
                self.runtime.park_row_focus(focus.field);
            }
            self.unregister_row_nodes(&state.tree);
            (state.tree, state.layout, state.views)
        } else {
            // The cell was recycled behind our back; start from an empty tree.
            remove_subviews(&content);
            (UiNode::empty(), LayoutResult::default(), HashMap::new())
        };

        let mut ops = render::diff(&old_tree, &tree);
        ops.extend(render::diff_layout(&old_tree, &tree, &old_layout, &layout));

        // The snapshot shows the old content while the new widgets are laid
        // out underneath. The cell stays available for reuse immediately.
        let snapshot = if animation_is_enabled(transition_ms) && had_previous && !ops.is_empty() {
            content.snapshotViewAfterScreenUpdates(false)
        } else {
            None
        };

        for op in ops {
            self.apply_to(op, &mut views, &content, false);
        }

        if let Some(snapshot) = snapshot
            && let Some(mtm) = MainThreadMarker::new()
        {
            content.addSubview(&snapshot);
            let fading = snapshot.clone();
            let block = RcBlock::new(move || fading.setAlpha(0.0));
            let removing = snapshot;
            let completion = RcBlock::new(move |_: UIViewAnimatingPosition| {
                removing.removeFromSuperview();
            });
            run_animation(
                transition_ms,
                &AnimationCurve::EaseOut,
                block,
                Some(completion),
                mtm,
            );
        }

        self.record_row_actions(list, index, &tree);
        self.register_row_nodes(list, index, &tree);
        self.row_states.insert(
            key,
            IosRowState {
                list,
                index,
                tree,
                layout,
                views,
            },
        );

        if !rebinding
            && let Some(focus) = self.runtime.row_focus()
            && focus.list == list
            && focus.index == index
        {
            self.try_focus_field(focus);
        }
    }

    /// Releases the widgets and actions of the row a cell currently shows.
    pub(crate) fn unbind_row(&mut self, key: usize) {
        if let Some(state) = self.row_states.remove(&key) {
            for handle in state.views.values() {
                if let Some(link) = &handle.frame_link {
                    link.invalidate();
                }
            }
            if let Some(focus) = self.runtime.row_focus()
                && tree::find(&state.tree, focus.field).is_some()
            {
                self.runtime.park_row_focus(focus.field);
            }
            self.unregister_row_nodes(&state.tree);
            self.clear_row_actions(state.list, state.index);
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

    /// Whether a row field is still in a window, and so still really focused.
    pub(crate) fn field_attached(&self, id: NodeId) -> bool {
        self.row_states.values().any(|row| {
            row.views
                .get(&id)
                .is_some_and(|handle| match &handle.content {
                    Content::Field(field) => field.window().is_some(),
                    _ => handle.view.window().is_some(),
                })
        })
    }

    /// Makes a row field the first responder, if it can be.
    pub(crate) fn focus_field(&self, id: NodeId) -> bool {
        for row in self.row_states.values() {
            if let Some(handle) = row.views.get(&id)
                && let Content::Field(field) = &handle.content
            {
                return field.becomeFirstResponder();
            }
        }
        false
    }

    /// Restores the logical focus of a row that was just (re)bound.
    pub(crate) fn try_focus_field(&mut self, focus: crate::runtime::RowFocus) {
        if self.focus_field(focus.field) {
            self.runtime
                .set_row_focus(focus.list, focus.index, focus.field, true);
        } else {
            // The cell is not in the window yet; try again next turn.
            let handle = self.handle;
            let host = self.host;
            let field = focus.field;
            schedule(move || focus_row_async(handle, host, field));
        }
    }

    /// Forgets actions and rows whose list or item no longer exists.
    pub(crate) fn prune_rows(&mut self) {
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

        let detached: Vec<usize> = self
            .row_states
            .iter()
            .filter(|(_, row)| self.runtime.recycled_item_count(row.list).is_none())
            .map(|(key, _)| *key)
            .collect();
        for key in detached {
            if let Some(state) = self.row_states.remove(&key) {
                self.unregister_row_nodes(&state.tree);
            }
        }
    }

    /// Drops the actions recorded for one row binding.
    pub(crate) fn clear_row_actions(&mut self, list: NodeId, index: usize) {
        if let Some(ids) = self.row_action_ids.remove(&(list, index)) {
            for id in ids {
                self.row_actions.remove(&id);
            }
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

/// Applies pending count, height and content changes to a recycled list.
///
/// UIKit may ask the data source for cells synchronously during a batch
/// update; those callbacks acquire the state lock themselves, so the sync runs
/// after the lock that produced it has been released.
pub(crate) fn sync_recycler(handle: u64, host: u64, id: NodeId) {
    guard("recycler", || {
        let Some((collection, data_source)) =
            with_state(handle, host, |state| state.recycler_for(id)).flatten()
        else {
            return;
        };
        let Some(changes) = data_source.take_changes() else {
            return;
        };

        if changes.batch {
            let paths: Vec<Retained<NSIndexPath>> = if changes.to > changes.from {
                (changes.from..changes.to)
                    .map(|index| NSIndexPath::indexPathForItem_inSection(index as isize, 0))
                    .collect()
            } else {
                (changes.to..changes.from)
                    .map(|index| NSIndexPath::indexPathForItem_inSection(index as isize, 0))
                    .collect()
            };
            let array = NSArray::from_retained_slice(&paths);
            let insert = changes.to > changes.from;
            let updates_collection = collection.clone();
            let updates = RcBlock::new(move || {
                if insert {
                    updates_collection.insertItemsAtIndexPaths(&array);
                } else {
                    updates_collection.deleteItemsAtIndexPaths(&array);
                }
            });
            let batch_collection = collection.clone();
            let batch = RcBlock::new(move || {
                batch_collection.performBatchUpdates_completion(Some(&*updates), None);
            });
            // SAFETY: the sync block runs on the main queue.
            let mtm = unsafe { MainThreadMarker::new_unchecked() };
            UIView::performWithoutAnimation(&batch, mtm);
        }

        if changes.layout {
            collection.collectionViewLayout().invalidateLayout();
        }
        if changes.refresh {
            with_state(handle, host, |state| state.rebind_visible_rows(id));
        }
    });
}

pub(crate) fn focus_row_async(handle: u64, host: u64, field: NodeId) {
    guard("focus", || {
        with_state(handle, host, |state| {
            let Some(focus) = state.runtime.row_focus() else {
                return;
            };
            if focus.field != field {
                return;
            }
            if state.focus_field(field) {
                state
                    .runtime
                    .set_row_focus(focus.list, focus.index, field, true);
            }
        });
    });
}
