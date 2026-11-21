//! The `grid` widget: a scrolling list with several items on a line.

use std::marker::PhantomData;
use std::sync::Arc;

use crate::element::{Element, ElementKind, RawElement};
use crate::style::{IntoPoints, Length};
use crate::widgets::{Column, List, Row};

/// Derives the key of one item.
type KeyBuilder<T> = Arc<dyn Fn(&T) -> String + Send + Sync>;

/// Builds the element of one item.
type CellBuilder<T> = Arc<dyn Fn(&T) -> RawElement + Send + Sync>;

/// A scrolling grid: a [`List`] with a fixed number of equally wide items
/// on every line.
///
/// ```ignore
/// Grid::new(self.photos.clone())
///     .columns(3)
///     .spacing(4)
///     .key(|photo| photo.id.to_string())
///     .cell(|photo| Image::new(ImageSource::url(&photo.thumb)).aspect_ratio(1.0).into())
/// ```
///
/// Each line of the grid is one row of the list, so everything a list does
/// holds: only the lines near the viewport exist, [`recycled`](Self::recycled)
/// uses the platform's recycler, and a line is as tall as its tallest cell
/// unless [`cell_height`](Self::cell_height) fixes it. Like a list it needs
/// a bounded area.
pub struct Grid<T, M = ()> {
    shell: RawElement,
    message: PhantomData<fn() -> M>,
    items: Arc<Vec<T>>,
    columns: usize,
    spacing: f32,
    cell_height: Option<f32>,
    recycled: bool,
    revision: Option<u64>,
    key: Option<KeyBuilder<T>>,
    cell: Option<CellBuilder<T>>,
}

impl<T: Send + Sync + 'static, M> Grid<T, M> {
    /// Creates a grid over the given items, two to a line.
    ///
    /// Pass an `Arc<Vec<_>>` to avoid copying the data on every rebuild.
    pub fn new(items: impl Into<Arc<Vec<T>>>) -> Self {
        let mut shell = RawElement::container(ElementKind::Column);
        shell.style.width = Length::Percent(1.0);
        shell.style.grow = 1.0;
        Self {
            shell,
            message: PhantomData,
            items: items.into(),
            columns: 2,
            spacing: 0.0,
            cell_height: None,
            recycled: false,
            revision: None,
            key: None,
            cell: None,
        }
    }

    /// Sets how many items are on a line; at least one.
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns.max(1);
        self
    }

    /// Sets the space between items, in both directions.
    pub fn spacing(mut self, value: impl IntoPoints) -> Self {
        self.spacing = value.into_points().max(0.0);
        self
    }

    /// Fixes the height of every cell; see [`List::item_height`].
    pub fn cell_height(mut self, height: impl IntoPoints) -> Self {
        self.cell_height = Some(height.into_points().max(1.0));
        self
    }

    /// Uses the platform's recycler; see [`List::recycled`].
    pub fn recycled(mut self) -> Self {
        self.recycled = true;
        self
    }

    /// Declares a revision of the items; see [`List::revision`].
    pub fn revision(mut self, revision: u64) -> Self {
        self.revision = Some(revision);
        self
    }

    /// Derives the stable, unique key of an item.
    pub fn key(mut self, key: impl Fn(&T) -> String + Send + Sync + 'static) -> Self {
        self.key = Some(Arc::new(key));
        self
    }

    /// Builds the element of one item. Called only for visible items.
    pub fn cell(mut self, cell: impl Fn(&T) -> Element<M> + Send + Sync + 'static) -> Self {
        self.cell = Some(Arc::new(move |item| cell(item).into_raw()));
        self
    }

    /// Sets a fixed width in logical points.
    pub fn width(mut self, value: impl IntoPoints) -> Self {
        self.shell.style.width = Length::Points(value.into_points());
        self
    }

    /// Sets a fixed height in logical points.
    pub fn height(mut self, value: impl IntoPoints) -> Self {
        self.shell.style.height = Length::Points(value.into_points());
        self.shell.style.grow = 0.0;
        self
    }

    /// Sets uniform padding around the whole grid.
    pub fn padding(mut self, value: impl IntoPoints) -> Self {
        self.shell.style.padding = crate::style::EdgeInsets::all(value.into_points());
        self
    }
}

impl<T: Send + Sync + 'static, M: 'static> From<Grid<T, M>> for Element<M> {
    fn from(grid: Grid<T, M>) -> Self {
        let key = grid.key.expect("Grid::key must be set");
        let cell = grid.cell.expect("Grid::cell must be set");
        let (items, columns, spacing) = (grid.items, grid.columns, grid.spacing);
        let lines: Vec<usize> = (0..items.len().div_ceil(columns)).collect();
        let last = lines.len().saturating_sub(1);

        let mut list = List::<usize, M>::new(lines)
            .key({
                let items = Arc::clone(&items);
                move |line| {
                    let start = line * columns;
                    let end = (start + columns).min(items.len());
                    let keys: Vec<String> =
                        items[start..end].iter().map(|item| key(item)).collect();
                    keys.join("\u{1f}")
                }
            })
            .row(move |line| {
                let start = line * columns;
                // The space under a line belongs to the line, so every line
                // of the list has it except the last.
                let mut row =
                    Row::<M>::new()
                        .spacing(spacing)
                        .padding_each(crate::style::EdgeInsets {
                            top: 0.0,
                            right: 0.0,
                            bottom: if *line < last { spacing } else { 0.0 },
                            left: 0.0,
                        });
                for column in 0..columns {
                    // Every cell asks for the same share and shrinks by the
                    // same amount to make room for the gaps, so the cells
                    // are equally wide whatever they hold. A line that is
                    // not full keeps its cells at that width.
                    let mut slot = Column::<M>::new()
                        .width_percent(1.0 / columns as f32)
                        .min_width(0)
                        .key(format!("cell-{column}"));
                    if let Some(item) = items.get(start + column) {
                        slot = slot.child(Element::from_raw(cell(item)));
                    }
                    row = row.child(slot);
                }
                row.into()
            });
        if let Some(height) = grid.cell_height {
            list = list.item_height(height + spacing);
        }
        if grid.recycled {
            list = list.recycled();
        }
        if let Some(revision) = grid.revision {
            list = list.revision(revision);
        }
        let mut raw = Element::from(list).into_raw();
        raw.style = grid.shell.style;
        Element::from_raw(raw)
    }
}

impl<T: Send + Sync + 'static> From<Grid<T, ()>> for RawElement {
    fn from(grid: Grid<T, ()>) -> Self {
        Element::from(grid).into_raw()
    }
}
