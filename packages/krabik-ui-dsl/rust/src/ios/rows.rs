//! Recycled lists: the collection view data source and binding rows into its cells.

use crate::geometry::Size;
use crate::host::NodeEvent;
use crate::host::rows::Bind;
use crate::ios::cast_view;
use crate::ios::events::dispatch;
use crate::ios::host::{Content, IosState, ViewHandle};
use crate::ios::motion::{animation_is_enabled, run_animation};
use crate::ios::widgets::Site;
use crate::ios::{guard, remove_subviews, schedule, with_state};
use crate::layout::LayoutResult;
use crate::render::RenderNode;
use crate::tree;
use crate::tree::{NodeId, RecyclerProps, UiNode};
use crate::ui::{AnimationCurve, RawElement};
use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::runtime::{AnyObject, Bool};
use objc2::runtime::{NSObject, NSObjectProtocol};
use objc2::{ClassType, DefinedClass};
use objc2::{MainThreadMarker, MainThreadOnly};
use objc2_core_foundation::CGAffineTransform;
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{NSArray, NSIndexPath, NSString};
use objc2_ui_kit::{NSIndexPathUIKitAdditions, UICollectionViewFlowLayout, UIEdgeInsets};
use objc2_ui_kit::{
    UICollectionView, UICollectionViewCell, UICollectionViewDataSource, UICollectionViewDelegate,
    UICollectionViewDelegateFlowLayout, UICollectionViewLayout, UIGestureRecognizer,
    UIGestureRecognizerDelegate, UIGestureRecognizerState, UILongPressGestureRecognizer,
    UIPanGestureRecognizer, UIScrollView, UIScrollViewDelegate, UIView, UIViewAnimatingPosition,
    UIViewAnimationOptions,
};
use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// The header that stays at the top of a list while its section scrolls: a
/// cell of the list's own kind that the collection view does not manage.
pub(crate) struct Pinned {
    cell: Retained<UICollectionViewCell>,
    /// The item, the revision and the width the cell was last bound for.
    bound: Cell<Option<(usize, u64, u64)>>,
    /// The item the cell should show now.
    wanted: Cell<usize>,
}

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
    /// The application listens for a row being swiped away.
    pub(crate) dismissible: AtomicBool,
    /// The application listens for a row being dragged elsewhere.
    pub(crate) reorderable: AtomicBool,
    /// The items that are pinned headers, ascending.
    pub(crate) sticky: RefCell<Vec<usize>>,
    /// The pinned header, once one was needed.
    pub(crate) pinned: RefCell<Option<Pinned>>,
    /// The row under a horizontal pan, and its item.
    pub(crate) swiped: RefCell<Option<(Retained<UICollectionViewCell>, usize)>>,
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

    unsafe impl UIScrollViewDelegate for RecyclerDataSource {
        #[unsafe(method(scrollViewDidScroll:))]
        fn scroll_view_did_scroll(&self, scroll_view: &UIScrollView) {
            let any: &AnyObject = scroll_view.as_ref();
            if let Some(collection) = any.downcast_ref::<UICollectionView>() {
                self.place_header(collection);
            }
        }
    }

    unsafe impl UIGestureRecognizerDelegate for RecyclerDataSource {
        #[unsafe(method(gestureRecognizerShouldBegin:))]
        fn should_begin(&self, recognizer: &UIGestureRecognizer) -> bool {
            let ivars = self.ivars();
            let any: &AnyObject = recognizer.as_ref();
            if let Some(pan) = any.downcast_ref::<UIPanGestureRecognizer>() {
                // Only a clearly sideways pan is a swipe; anything else is
                // the list scrolling.
                let velocity = pan.velocityInView(pan.view().as_deref());
                ivars.dismissible.load(Ordering::Relaxed)
                    && velocity.x.abs() > velocity.y.abs() * 1.5
            } else if any.downcast_ref::<UILongPressGestureRecognizer>().is_some() {
                ivars.reorderable.load(Ordering::Relaxed)
            } else {
                true
            }
        }
    }

    impl RecyclerDataSource {
        /// A long press picks a row up; the collection view moves it with
        /// the finger and tells the data source where it was dropped.
        #[unsafe(method(reorder:))]
        fn reorder(&self, recognizer: &UILongPressGestureRecognizer) {
            let Some(view) = recognizer.view() else {
                return;
            };
            let any: &AnyObject = view.as_ref();
            let Some(collection) = any.downcast_ref::<UICollectionView>() else {
                return;
            };
            let point = recognizer.locationInView(Some(&view));
            let state = recognizer.state();
            if state == UIGestureRecognizerState::Began {
                if let Some(path) = collection.indexPathForItemAtPoint(point) {
                    collection.beginInteractiveMovementForItemAtIndexPath(&path);
                }
            } else if state == UIGestureRecognizerState::Changed {
                collection.updateInteractiveMovementTargetPosition(point);
            } else if state == UIGestureRecognizerState::Ended {
                collection.endInteractiveMovement();
            } else {
                collection.cancelInteractiveMovement();
            }
        }

        /// A sideways pan slides a row; far or fast enough, it leaves.
        #[unsafe(method(swipe:))]
        fn swipe(&self, recognizer: &UIPanGestureRecognizer) {
            let Some(view) = recognizer.view() else {
                return;
            };
            let any: &AnyObject = view.as_ref();
            let Some(collection) = any.downcast_ref::<UICollectionView>() else {
                return;
            };
            let ivars = self.ivars();
            let state = recognizer.state();
            let offset = recognizer.translationInView(Some(&view)).x;
            if state == UIGestureRecognizerState::Began {
                let point = recognizer.locationInView(Some(&view));
                let row = collection.indexPathForItemAtPoint(point).and_then(|path| {
                    let index = path.item().max(0) as usize;
                    collection.cellForItemAtIndexPath(&path).map(|cell| (cell, index))
                });
                *ivars.swiped.borrow_mut() = row;
            } else if state == UIGestureRecognizerState::Changed {
                if let Some((cell, _)) = ivars.swiped.borrow().as_ref() {
                    cell.contentView().setTransform(translation(offset));
                }
            } else {
                let Some((cell, index)) = ivars.swiped.borrow_mut().take() else {
                    return;
                };
                let width = collection.bounds().size.width;
                let speed = recognizer.velocityInView(Some(&view)).x;
                let leaves = state == UIGestureRecognizerState::Ended
                    && (offset.abs() > width * 0.4 || speed.abs() > 900.0);
                let (handle, host, id) = (ivars.handle, ivars.host, ivars.id);
                finish_swipe(cell, leaves.then_some(offset.signum() * width), self.mtm(), move || {
                    dispatch(handle, host, id, NodeEvent::Dismiss(index));
                });
            }
        }
    }

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

        #[unsafe(method(collectionView:canMoveItemAtIndexPath:))]
        fn can_move_item(&self, _collection_view: &UICollectionView, _path: &NSIndexPath) -> bool {
            self.ivars().reorderable.load(Ordering::Relaxed)
        }

        /// The row is already shown at its new place. The application hears
        /// where it came from and went to; the rows are bound again from
        /// the state after the render that follows.
        #[unsafe(method(collectionView:moveItemAtIndexPath:toIndexPath:))]
        fn move_item(
            &self,
            _collection_view: &UICollectionView,
            from: &NSIndexPath,
            to: &NSIndexPath,
        ) {
            let ivars = self.ivars();
            let (handle, host, id) = (ivars.handle, ivars.host, ivars.id);
            let (from, to) = (from.item().max(0) as usize, to.item().max(0) as usize);
            if from != to {
                schedule(move || dispatch(handle, host, id, NodeEvent::Move(from, to)));
            }
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
            dismissible: AtomicBool::new(props.on_dismiss.is_set()),
            reorderable: AtomicBool::new(props.on_reorder.is_set()),
            sticky: RefCell::new(props.sticky.clone()),
            pinned: RefCell::new(None),
            swiped: RefCell::new(None),
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
        ivars
            .dismissible
            .store(props.on_dismiss.is_set(), Ordering::Relaxed);
        ivars
            .reorderable
            .store(props.on_reorder.is_set(), Ordering::Relaxed);
        if *ivars.sticky.borrow() != props.sticky {
            *ivars.sticky.borrow_mut() = props.sticky.clone();
        }
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

    /// Puts the current section's header at the top of the visible part of
    /// the list, pushed up by the next header as it arrives, or hides it.
    ///
    /// This runs while the list scrolls and may run while the state is
    /// locked, so it only asks the collection view. What the header shows
    /// is bound a turn later by [`bind_pinned`].
    pub(crate) fn place_header(&self, collection: &UICollectionView) {
        let ivars = self.ivars();
        let hide = || {
            if let Some(pinned) = ivars.pinned.borrow().as_ref() {
                pinned.cell.setHidden(true);
            }
        };
        let sticky = ivars.sticky.borrow();
        if sticky.is_empty() {
            return hide();
        }
        let top = collection.contentOffset().y;
        let frame_of = |index: usize| {
            let path = NSIndexPath::indexPathForItem_inSection(index as isize, 0);
            collection
                .layoutAttributesForItemAtIndexPath(&path)
                .map(|attributes| attributes.frame())
        };
        let Some(first) = collection
            .indexPathForItemAtPoint(CGPoint::new(1.0, top + 1.0))
            .map(|path| path.item().max(0) as usize)
        else {
            return hide();
        };
        let Some(header) = sticky.iter().rev().find(|index| **index <= first).copied() else {
            return hide();
        };
        let Some(own) = frame_of(header) else {
            return hide();
        };
        // The header row itself is still fully in place: nothing to pin.
        if own.origin.y >= top {
            return hide();
        }
        let height = own.size.height;
        let next_top = sticky
            .iter()
            .find(|index| **index > first)
            .and_then(|index| frame_of(*index))
            .map(|frame| frame.origin.y);
        let y = next_top.map_or(top, |next| top.min(next - height));

        let mut slot = ivars.pinned.borrow_mut();
        let pinned = slot.get_or_insert_with(|| {
            let cell = UICollectionViewCell::initWithFrame(
                UICollectionViewCell::alloc(self.mtm()),
                CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(0.0, 0.0)),
            );
            // A picture of the row, as on Android; and above the rows.
            cell.setUserInteractionEnabled(false);
            cell.layer().setZPosition(1000.0);
            collection.addSubview(&cell);
            Pinned {
                cell,
                bound: Cell::new(None),
                wanted: Cell::new(header),
            }
        });
        let width = collection.bounds().size.width;
        pinned.cell.setFrame(CGRect::new(
            CGPoint::new(0.0, y),
            CGSize::new(width, height),
        ));
        pinned.cell.setHidden(false);
        pinned.wanted.set(header);
        let wanted = Some((
            header,
            ivars.revision.load(Ordering::Relaxed),
            width.to_bits(),
        ));
        if pinned.bound.get() != wanted {
            let (handle, host, id) = (ivars.handle, ivars.host, ivars.id);
            schedule(move || bind_pinned(handle, host, id));
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
        let tree = self
            .runtime
            .build_subtree(&element, tree::row_root(list, index));
        let layout = self.layout_row(&tree, width);
        Some((tree, layout))
    }

    fn layout_row(&self, tree: &UiNode, width: f32) -> LayoutResult {
        self.runtime
            .layout_tree(tree, Size::new(width.max(1.0), 100_000.0))
    }

    /// The height a variable-height row needs at the given width.
    pub(crate) fn row_height(&self, list: NodeId, index: usize, width: f32) -> f32 {
        let Some((_, layout)) = self.row_layout(list, index, width) else {
            return 0.0;
        };
        root_height(&layout, list, index)
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
        let mut resized = false;
        for path in paths.iter() {
            let index = path.item().max(0) as usize;
            if let Some(cell) = collection.cellForItemAtIndexPath(&path) {
                resized |= self.bind_row(id, index, &cell, width);
            }
        }
        // Only a row whose measured height changed needs the layout to ask
        // for sizes again; that query lays every visible row out once more.
        if resized {
            collection.collectionViewLayout().invalidateLayout();
        }
    }

    /// Builds or updates one recycled row inside a collection-view cell.
    ///
    /// Returns whether the row's height differs from what the cell showed.
    /// What to build, and what changed since the cell was last bound, is
    /// decided by the shared [`RowBinder`]; this function applies the result
    /// to the cell's views.
    ///
    /// [`RowBinder`]: crate::host::rows::RowBinder
    pub(crate) fn bind_row(
        &mut self,
        list: NodeId,
        index: usize,
        cell: &UICollectionViewCell,
        width: f64,
    ) -> bool {
        // Cells are reused by UIKit, so a row is keyed by its cell's identity.
        let key = cell as *const UICollectionViewCell as usize;
        let mut pending = match self
            .rows
            .begin(&mut self.runtime, &key, list, index, width as f32)
        {
            Bind::Missing | Bind::Unchanged { .. } => return false,
            Bind::Changed(pending) => *pending,
        };
        let content = cell.contentView();

        if pending.fresh {
            // The cell was recycled behind our back; start from nothing.
            remove_subviews(&content);
        }

        // The snapshot shows the old content while the new widgets are laid
        // out underneath. The cell stays available for reuse immediately.
        let transition_ms = pending.transition_ms;
        let snapshot =
            if animation_is_enabled(transition_ms) && !pending.fresh && !pending.ops.is_empty() {
                content.snapshotViewAfterScreenUpdates(false)
            } else {
                None
            };

        for op in std::mem::take(&mut pending.ops) {
            self.apply_to(op, &mut pending.views, &content, false);
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

        let rebinding = pending.rebinding;
        let resized = pending.resized;
        self.rows.finish(key, pending);

        if !rebinding
            && let Some(focus) = self.runtime.row_focus()
            && focus.list == list
            && focus.index == index
        {
            self.try_focus_field(focus);
        }
        resized
    }

    /// Releases the widgets and actions of the row a cell currently shows.
    pub(crate) fn unbind_row(&mut self, key: usize) {
        if let Some(row) = self.rows.unbind(&mut self.runtime, &key) {
            for handle in row.views.values() {
                if let Some(link) = &handle.frame_link {
                    link.invalidate();
                }
            }
        }
    }

    /// Whether a row field is still in a window, and so still really focused.
    pub(crate) fn field_attached(&self, id: NodeId) -> bool {
        self.rows.rows().any(|row| {
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
        for row in self.rows.rows() {
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
        self.rows.prune(&self.runtime);
    }
}

/// The measured height of a laid-out row.
fn root_height(layout: &LayoutResult, list: NodeId, index: usize) -> f32 {
    layout
        .frames
        .get(&tree::row_root(list, index))
        .map_or(0.0, |frame| frame.height.max(0.0))
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
        data_source.place_header(&collection);
    });
}

/// Binds the pinned header to the item it should show now.
///
/// Runs on a main-queue turn of its own, because binding a row needs the
/// state, which may be locked while the list scrolls.
pub(crate) fn bind_pinned(handle: u64, host: u64, id: NodeId) {
    guard("pinned header", || {
        let Some((collection, data_source)) =
            with_state(handle, host, |state| state.recycler_for(id)).flatten()
        else {
            return;
        };
        let ivars = data_source.ivars();
        let slot = ivars.pinned.borrow();
        let Some(pinned) = slot.as_ref() else {
            return;
        };
        let width = collection.bounds().size.width;
        let wanted = (
            pinned.wanted.get(),
            ivars.revision.load(Ordering::Relaxed),
            width.to_bits(),
        );
        if pinned.bound.get() == Some(wanted) {
            return;
        }
        pinned.bound.set(Some(wanted));
        with_state(handle, host, |state| {
            state.bind_row(id, wanted.0, &pinned.cell, width);
        });
    });
}

fn translation(x: f64) -> CGAffineTransform {
    CGAffineTransform {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        tx: x,
        ty: 0.0,
    }
}

/// Ends a swipe: the row slides off to `leave_to` and `dismissed` runs, or
/// it slides back when it was not swiped far enough.
fn finish_swipe(
    cell: Retained<UICollectionViewCell>,
    leave_to: Option<f64>,
    mtm: MainThreadMarker,
    dismissed: impl Fn() + Send + Clone + 'static,
) {
    let content = cell.contentView();
    let sliding = content.clone();
    let slide = RcBlock::new(move || sliding.setTransform(translation(leave_to.unwrap_or(0.0))));
    let leaves = leave_to.is_some();
    let done = RcBlock::new(move |_: Bool| {
        if !leaves {
            return;
        }
        // The row stays out of sight until the list is bound again from the
        // state, which shows this cell's next content, or the same row when
        // the application kept it.
        content.setAlpha(0.0);
        content.setTransform(translation(0.0));
        schedule(dismissed.clone());
        let returning = content.clone();
        let back = RcBlock::new(move || returning.setAlpha(1.0));
        UIView::animateWithDuration_delay_options_animations_completion(
            0.1,
            0.25,
            UIViewAnimationOptions::empty(),
            &back,
            None,
            mtm,
        );
    });
    UIView::animateWithDuration_delay_options_animations_completion(
        0.2,
        0.0,
        UIViewAnimationOptions::empty(),
        &slide,
        Some(&done),
        mtm,
    );
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

/// Creates the native view of a recycled list.
pub(crate) fn create_recycler(props: &RecyclerProps, node: &RenderNode, site: &Site) -> ViewHandle {
    let layout = UICollectionViewFlowLayout::new(site.mtm);
    layout.setMinimumLineSpacing(0.0);
    layout.setMinimumInteritemSpacing(0.0);
    layout.setSectionInset(UIEdgeInsets {
        top: 0.0,
        left: 0.0,
        bottom: 0.0,
        right: 0.0,
    });
    let collection = UICollectionView::initWithFrame_collectionViewLayout(
        UICollectionView::alloc(site.mtm),
        CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(0.0, 0.0)),
        &layout,
    );
    collection.setShowsVerticalScrollIndicator(true);
    collection.setAlwaysBounceVertical(true);
    let identifier = NSString::from_str(ROW_CELL_ID);
    // SAFETY: the cell class is a `UICollectionViewCell`, which is
    // what the dequeue call below expects.
    unsafe {
        collection.registerClass_forCellWithReuseIdentifier(
            Some(UICollectionViewCell::class()),
            &identifier,
        );
    }
    let data_source = RecyclerDataSource::new(site.handle, site.host, site.node, props, site.mtm);
    collection.setDataSource(Some(ProtocolObject::from_ref(&*data_source)));
    // SAFETY: the data source is kept alive by the view handle and
    // only reads the collection view's size and index paths.
    unsafe {
        collection.setDelegate(Some(ProtocolObject::from_ref(&*data_source)));
    }
    // The gestures on a row. Their delegate lets each begin only while the
    // application listens for it.
    // SAFETY: `reorder:` and `swipe:` are methods of the data source that
    // take their recognizer; the handle keeps the data source alive.
    unsafe {
        let hold: Retained<UILongPressGestureRecognizer> = objc2::msg_send![
            UILongPressGestureRecognizer::alloc(site.mtm),
            initWithTarget: &*data_source,
            action: objc2::sel!(reorder:),
        ];
        hold.setDelegate(Some(ProtocolObject::from_ref(&*data_source)));
        collection.addGestureRecognizer(&hold);
        let pan: Retained<UIPanGestureRecognizer> = objc2::msg_send![
            UIPanGestureRecognizer::alloc(site.mtm),
            initWithTarget: &*data_source,
            action: objc2::sel!(swipe:),
        ];
        pan.setDelegate(Some(ProtocolObject::from_ref(&*data_source)));
        collection.addGestureRecognizer(&pan);
    }
    crate::ios::widgets::refresh::attach(&collection, &props.refresh, site);
    let view = cast_view(collection.clone());
    ViewHandle::new(
        view,
        Content::Recycler {
            collection,
            data_source,
        },
        node,
    )
}
