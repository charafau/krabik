//! The `list` widget: properties and builder.

use std::marker::PhantomData;

use crate::element::{Element, ElementKind, RawElement};
use crate::message::{Handler, Wrap};
use crate::style::{IntoPoints, Length};
use crate::widgets::Refresh;
use std::fmt;
use std::sync::Arc;

/// How a [`List`] materializes its rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ListMode {
    /// Window the content inside a native scroll container (default).
    #[default]
    Windowed,
    /// Let a native recycler own scrolling and view recycling.
    Recycled,
}

/// The item source of a virtualized [`List`].
///
/// Keys and rows are closures so the runtime can materialize only the items in
/// the window. They must be `Send + Sync` because the app may render on any
/// thread.
#[derive(Clone)]
pub(crate) struct ListProps {
    pub(crate) item_count: usize,
    pub(crate) mode: ListMode,
    pub(crate) item_height: Option<f32>,
    pub(crate) estimated_height: f32,
    pub(crate) buffer: usize,
    /// The item revision, when the application supplies one.
    pub(crate) revision: Option<u64>,
    /// Cross-fade duration for a holder rebind, in milliseconds.
    pub(crate) row_transition_ms: u32,
    /// Pull-to-refresh.
    pub(crate) refresh: Refresh,
    pub(crate) keys: Arc<dyn Fn(usize) -> String + Send + Sync>,
    pub(crate) rows: Arc<dyn Fn(usize) -> RawElement + Send + Sync>,
}

impl fmt::Debug for ListProps {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ListProps")
            .field("mode", &self.mode)
            .field("item_count", &self.item_count)
            .field("item_height", &self.item_height)
            .field("buffer", &self.buffer)
            .field("revision", &self.revision)
            .finish_non_exhaustive()
    }
}

impl PartialEq for ListProps {
    /// Row sources are opaque; two lists are equal when their visible
    /// configuration matches and they share the same item source.
    fn eq(&self, other: &Self) -> bool {
        self.mode == other.mode
            && self.item_count == other.item_count
            && self.item_height == other.item_height
            && self.buffer == other.buffer
            && self.revision == other.revision
            && self.row_transition_ms == other.row_transition_ms
            && self.refresh == other.refresh
            && Arc::ptr_eq(&self.keys, &other.keys)
    }
}

impl ListProps {
    /// Re-wraps the messages of every row the list builds.
    pub(crate) fn map_messages(&mut self, wrap: &Wrap) {
        self.refresh.on_refresh.map(wrap);
        let rows = Arc::clone(&self.rows);
        let wrap = Arc::clone(wrap);
        self.rows = Arc::new(move |index| {
            let mut row = rows(index);
            row.map_messages(&wrap);
            row
        });
    }
}

/// Derives the key of one list item.
type KeyBuilder<T> = Box<dyn Fn(&T) -> String + Send + Sync>;

/// Builds the element of one list item.
type RowBuilder<T> = Box<dyn Fn(&T) -> RawElement + Send + Sync>;

/// A virtualized vertical list.
///
/// Only the rows around the viewport are turned into elements and native
/// views; the rest of the content is empty space, so the scroll extent and the
/// scrollbar still reflect every item. Give the list a bounded area, normally
/// by placing it in a sized or growing container.
///
/// Keys must be unique among the items; they keep a row's native view alive
/// while it scrolls through the window.
pub struct List<T, M = ()> {
    message: PhantomData<fn() -> M>,
    items: Arc<Vec<T>>,
    mode: ListMode,
    item_height: Option<f32>,
    estimated_height: f32,
    buffer: usize,
    revision: Option<u64>,
    row_transition_ms: u32,
    refresh: Refresh,
    key: Option<KeyBuilder<T>>,
    row: Option<RowBuilder<T>>,
}

impl<T: Send + Sync + 'static, M> List<T, M> {
    /// Creates a list over the given items.
    ///
    /// Pass an `Arc<Vec<_>>` to avoid copying the data on every rebuild.
    pub fn new(items: impl Into<Arc<Vec<T>>>) -> Self {
        Self {
            message: PhantomData,
            items: items.into(),
            mode: ListMode::Windowed,
            item_height: None,
            estimated_height: 56.0,
            buffer: 3,
            revision: None,
            row_transition_ms: 0,
            refresh: Refresh::default(),
            key: None,
            row: None,
        }
    }

    /// Fixes every row to the same height.
    ///
    /// Without it the list measures each row and caches the result, which
    /// tolerates mixed row heights at the cost of a measurement pass.
    pub fn item_height(mut self, height: impl IntoPoints) -> Self {
        self.item_height = Some(height.into_points().max(1.0));
        self
    }

    /// Uses a native recycler for scrolling and view recycling.
    ///
    /// On Android this swaps the windowed scroll container for a `RecyclerView`
    /// and on iOS for a `UICollectionView`. `item_height` is optional: with it
    /// the holder has a fixed height, without it Rust measures each row and the
    /// holder wraps the content.
    ///
    /// Recycled rows are rebuilt by diffing the previous tree of their holder,
    /// so they may contain any widget. A new render revision rebinds the
    /// attached rows, which is how application state changes reach a row. A
    /// focused field keeps its logical focus across recycling and regains it
    /// when its row is bound again.
    pub fn recycled(mut self) -> Self {
        self.mode = ListMode::Recycled;
        self
    }

    /// Cross-fades a recycled row when a holder is rebound to new content.
    ///
    /// The fade is visual only: the holder is updated immediately and stays
    /// available for reuse. `0` (the default) disables it. Windowed lists
    /// rebuild their rows in place and ignore this.
    pub fn row_transition(mut self, duration_ms: u32) -> Self {
        self.row_transition_ms = duration_ms;
        self
    }

    /// Sets the height assumed for rows that have not been measured yet.
    pub fn estimated_height(mut self, height: impl IntoPoints) -> Self {
        self.estimated_height = height.into_points().max(1.0);
        self
    }

    /// Materializes this many extra rows above and below the viewport.
    pub fn buffer(mut self, rows: usize) -> Self {
        self.buffer = rows;
        self
    }

    /// Declares a revision of the item sequence.
    ///
    /// Only a list without [`item_height`](Self::item_height) uses it: a
    /// fixed-height list needs no measurements to cache, and a recycled list
    /// is rebound by the runtime on every render.
    ///
    /// Change the value whenever the items, their order, or the strings their
    /// keys derive from change. The runtime caches row heights and prefix sums
    /// per revision, so an unchanged revision makes a rebuild cost only the
    /// visible rows instead of re-keying the whole list. It must also change
    /// when a different list takes over the same position in the tree (for
    /// example after a screen swap).
    ///
    /// Without it the list stays correct but is fully re-keyed on every build,
    /// which caps it at hundreds of rows.
    pub fn revision(mut self, revision: u64) -> Self {
        self.revision = Some(revision);
        self
    }

    /// Shows the refresh indicator while `refreshing` is true; see
    /// [`on_refresh`](Self::on_refresh).
    pub fn refreshing(mut self, refreshing: bool) -> Self {
        self.refresh.refreshing = refreshing;
        self
    }

    /// Derives the stable, unique key of an item.
    pub fn key(mut self, key: impl Fn(&T) -> String + Send + Sync + 'static) -> Self {
        self.key = Some(Box::new(key));
        self
    }

    /// Builds the element of one item. Called only for visible items.
    pub fn row(mut self, row: impl Fn(&T) -> Element<M> + Send + Sync + 'static) -> Self {
        self.row = Some(Box::new(move |item| row(item).into_raw()));
        self
    }
}

impl<T: Send + Sync + 'static, M: Clone + Send + Sync + 'static> List<T, M> {
    /// Sends `message` when the user pulls the list down past its top: the
    /// platform's pull-to-refresh, for windowed and recycled lists alike.
    ///
    /// Show the indicator for as long as the refresh takes with
    /// [`refreshing`](Self::refreshing).
    pub fn on_refresh(mut self, message: M) -> Self {
        self.refresh.on_refresh = Handler::new(move |()| message.clone());
        self
    }
}

impl<T: Send + Sync + 'static, M> From<List<T, M>> for Element<M> {
    fn from(list: List<T, M>) -> Self {
        let key = list.key.expect("List::key must be set");
        let row = list.row.expect("List::row must be set");
        let items = list.items;

        let keys = {
            let items = Arc::clone(&items);
            Arc::new(move |index: usize| key(&items[index]))
                as Arc<dyn Fn(usize) -> String + Send + Sync>
        };
        let rows = {
            let items = Arc::clone(&items);
            Arc::new(move |index: usize| row(&items[index]))
                as Arc<dyn Fn(usize) -> RawElement + Send + Sync>
        };

        let mut element = RawElement::container(ElementKind::List(ListProps {
            item_count: items.len(),
            mode: list.mode,
            item_height: list.item_height,
            estimated_height: list.estimated_height,
            buffer: list.buffer,
            revision: list.revision,
            row_transition_ms: list.row_transition_ms,
            refresh: list.refresh,
            keys,
            rows,
        }));
        // A list is a scroll region: it grows into the space its flex parent
        // leaves and spans the cross axis.
        element.style.width = Length::Percent(1.0);
        element.style.grow = 1.0;
        Element::from_raw(element)
    }
}

impl<T: Send + Sync + 'static> From<List<T, ()>> for RawElement {
    fn from(list: List<T, ()>) -> Self {
        Element::from(list).into_raw()
    }
}
